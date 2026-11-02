//! The roll API's client side, shared by `concept-seed` (the roll and the
//! agent-run choice ping) and `serve-question` (the picker's own ping).
//!
//! Three pieces live here:
//! - the request plumbing: API base, timeout, opt-outs, and the
//!   `X-Impeccable-Client` header every `/api/roll` and `/api/chosen` request
//!   carries;
//! - the seed reference and the seed ledger. The reference,
//!   `seed:<16 hex of sha256("impeccable-seed:" + key)>`, is what committed
//!   files (the surface brief's FORM line) record instead of the key. The
//!   ledger, `.impeccable/questions/seeds/<those 16 hex>.json`, is the local,
//!   self-ignored record that resolves a reference back to its key on this
//!   machine (`{"key"}`, written for every roll), plus, only while telemetry
//!   is on, one entry per API-dealt round (`poolRevision`, the dealt catalog
//!   ids) and the marker that a round's choice was already sent. Unlike
//!   `roll.json`, which a decision page removes when it takes the roll, the
//!   ledger outlives the page, so the ping the agent sends after the round
//!   still finds its round;
//! - the choice ping body and its send.

use crate::jsp;
use crate::util::{json_compact, safe_read, Env};
use serde_json::{json, Map, Value};
use std::time::Duration;


/// The kinds a decision-page option card declares. `grounded` is one of the
/// agent's grounded directions that is neither the assigned card nor the
/// pick (a safer round's lineup), any number of them. `dealt` is a surface
/// round's dealt structure other than the lead; it has no ping kind, so its
/// ping carries `option` alone. Canon is the `canonCard`, never an option.
pub const CARD_KINDS: [&str; 5] = ["assigned", "pick", "challenger", "grounded", "dealt"];





/// `impeccable/<version>[ <channel>]`, matching
/// `^impeccable/[0-9A-Za-z.+-]{1,32}( [a-z]{1,16})?$`. The version is the
/// installed skill's (SKILL.md frontmatter, what `/api/version` compares)
/// when the engine can read it, else the engine's own as `<engine>+engine`.
/// The channel is `IMPECCABLE_TELEMETRY_CHANNEL` when it is 1 to 16 lowercase
/// letters (`test` in this repo's tests and CI), else absent.
/// A roll response's `poolRevision` when it is the 12 lowercase hex the API
/// sends; anything else is not recorded or forwarded.
pub fn valid_pool_revision(v: &str) -> bool {
    v.len() == 12 && v.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A roll response's `handId` (32 lowercase hex, the server's opaque name
/// for the exact hand) is kept and forwarded only in that shape; older
/// servers omit it and nothing is sent.
pub fn valid_hand_id(v: &str) -> bool {
    v.len() == 32 && v.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// ---- seed ledger -------------------------------------------------------------

/// One API-dealt round as concept-seed recorded it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeedRound {
    pub scope: String,
    pub reroll: usize,
    pub mode: Option<String>,
    pub register: Option<String>,
    pub pool_revision: Option<String>,
    pub hand_id: Option<String>,
    /// The catalog ids the round printed as challengers, in print order.
    pub dealt: Vec<String>,
    /// The round's choice state: `pending` while the page's send is in
    /// flight, `picker` or `agent` once a send was confirmed.
    pub recorded: Option<String>,
    /// When the pending send began (epoch ms), while `recorded` is `pending`.
    pub pending_at: Option<f64>,
    /// 32 random hex naming this round's choice report, the same for the
    /// page's and the agent's send, so the server can drop a duplicate.
    pub ping_id: Option<String>,
}

impl SeedRound {
    /// A confirmed send: the agent's rerun for this round is a no-op.
    pub fn confirmed(&self) -> bool {
        matches!(self.recorded.as_deref(), Some("picker") | Some("agent"))
    }
}

/// The salt seed references are computed under (`impeccable_common::seed`).
/// `None` when no valid salt can be persisted on this machine: then there
/// is no seed reference at all, never an unsalted one.
pub fn seed_salt(env: &Env, cwd: &str) -> Option<String> {
    impeccable_common::seed::salt(&crate::util::homedir(env), Some(cwd), env.get("IMPECCABLE_SEED_SALT").map(String::as_str))
}

/// Every valid salt this machine keeps (home and project): a reference made
/// under any of them resolves.
pub fn seed_salts(env: &Env, cwd: &str) -> Vec<String> {
    impeccable_common::seed::existing_salts(&crate::util::homedir(env), Some(cwd), env.get("IMPECCABLE_SEED_SALT").map(String::as_str))
}

/// The seed ledger of one project directory: `.impeccable/questions/seeds/
/// <reference hex>.json`, `{"key", "rounds"?: {"<reroll>": {...}}}`. The key
/// entry is what lets a reference stand for its key on this machine and is
/// never pruned; rounds exist only while telemetry is on and are pruned after
/// 30 days. Every update is a read-modify-write under a per-file lock, so the
/// page's send and a concurrent roll of the next round never lose each
/// other's writes.
pub struct Ledger {
    cwd: String,
    salt: Option<String>,
    /// Every salt references may have been made under, `salt` first.
    salts: Vec<String>,
    /// A key this run generated itself: it cannot have a ledger, so the
    /// locator skips its search (`for_fresh_key`).
    fresh: Option<String>,
}

/// Ledger rounds older than this are pruned when a ledger is written; the
/// key behind the reference stays.
const SEED_TTL_MS: f64 = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;

/// A lock older than this belongs to a process that died holding it.
const LOCK_STALE_MS: u128 = 10_000;

/// How long a ledger update waits for the lock before giving up (best
/// effort: the feature never fails on its ledger).
const LOCK_WAIT_MS: u128 = 3_000;

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{}-{}-{}", std::process::id(), n, t)
}

struct LockGuard(String);

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn lock(path: &str) -> Option<LockGuard> {
    let lock = format!("{}.lock", path);
    let start = std::time::Instant::now();
    loop {
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&lock) {
            Ok(_) => return Some(LockGuard(lock)),
            // Windows answers a create racing a delete of the same name with
            // access denied rather than exists: both mean another holder.
            Err(e) if matches!(e.kind(), std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied) => {
                // Every pass checks the overall deadline first, the stale
                // branch included: a lock that cannot be removed (Windows
                // keeps a deleted file delete-pending; a read-only folder)
                // must not spin forever. Giving up never blocks a flow.
                if start.elapsed().as_millis() > LOCK_WAIT_MS {
                    return None;
                }
                let stale = std::fs::metadata(&lock).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_millis() > LOCK_STALE_MS);
                if stale && std::fs::remove_file(&lock).is_ok() {
                    continue;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            Err(_) => return None,
        }
    }
}

fn round_from(v: &Value, reroll: usize) -> SeedRound {
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    SeedRound {
        scope: s("scope").unwrap_or_default(),
        reroll,
        mode: s("mode"),
        register: s("register"),
        pool_revision: s("poolRevision").filter(|p| valid_pool_revision(p)),
        hand_id: s("handId").filter(|h| valid_hand_id(h)),
        dealt: v.get("dealt").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default(),
        recorded: s("recorded"),
        pending_at: v.get("pendingAt").and_then(Value::as_f64),
        ping_id: s("pingId").filter(|p| valid_hand_id(p)),
    }
}

impl Ledger {
    pub fn new(env: &Env, cwd: &str) -> Ledger {
        let salt = seed_salt(env, cwd);
        let mut salts: Vec<String> = salt.iter().cloned().collect();
        for s in seed_salts(env, cwd) {
            if !salts.contains(&s) {
                salts.push(s);
            }
        }
        Ledger { cwd: cwd.to_string(), salt, salts, fresh: None }
    }

    fn dir(&self) -> String {
        jsp::join(&[&self.cwd, ".impeccable", "questions", "seeds"])
    }

    /// The key's ledger, located once by the shared rule
    /// (`impeccable_common::seed::locate`): every operation, the printed
    /// reference and resolution all derive from this one answer.
    fn locate(&self, key: &str) -> impeccable_common::seed::Located {
        if self.fresh.as_deref() == Some(key) {
            return impeccable_common::seed::locate_fresh(&self.cwd, self.salt.as_deref(), key);
        }
        impeccable_common::seed::locate(&self.cwd, self.salt.as_deref(), &self.salts, key)
    }

    /// Marks `key` as one this run just generated (no `--from`, no
    /// `IMPECCABLE_CONCEPT_SEED`): 128 random bits have no ledger, so no
    /// name check or folder scan is spent on it.
    pub fn for_fresh_key(mut self, key: &str, fresh: bool) -> Ledger {
        if fresh {
            self.fresh = Some(key.to_string());
        }
        self
    }

    fn file(&self, key: &str) -> String {
        self.locate(key).path.to_string_lossy().into_owned()
    }

    /// The seed reference committed files record instead of `key`: the
    /// reference of the key's existing ledger, under whichever salt it was
    /// made, or for a new key the primary salt's. `None` when this machine
    /// has no salt and the key has no salted ledger.
    pub fn reference(&self, key: &str) -> Option<String> {
        self.locate(key).reference
    }

    fn read_file(path: &str) -> Option<Map<String, Value>> {
        safe_read(path).and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.as_object().cloned())
    }

    fn write_file(&self, path: &str, ledger: &Map<String, Value>) {
        let temp = format!("{}.tmp-{}", path, unique_suffix());
        if std::fs::write(&temp, json_compact(&Value::Object(ledger.clone()))).is_err() {
            let _ = std::fs::remove_file(&temp);
            return;
        }
        // A rename over a file an unlocked reader has open can fail for a
        // moment on Windows; retry briefly before giving up.
        for attempt in 0..20 {
            if std::fs::rename(&temp, path).is_ok() {
                return;
            }
            if attempt < 19 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let _ = std::fs::remove_file(&temp);
    }

    /// Read, change, write `key`'s ledger under its lock. The questions
    /// directory's ignore file goes in before anything is written.
    fn update<R>(&self, key: &str, f: impl FnOnce(&mut Map<String, Value>) -> R) -> Option<R> {
        if std::fs::create_dir_all(self.dir()).is_err() {
            return None;
        }
        crate::serve_question::ignore_questions_dir(&jsp::join(&[&self.cwd, ".impeccable", "questions"]));
        let located = self.locate(key);
        let path = located.path.to_string_lossy().into_owned();
        let _guard = lock(&path)?;
        let mut ledger = Self::read_file(&path).unwrap_or_default();
        // A ledger made with no salt says so, so it never yields a reference.
        if !located.exists && located.reference.is_none() {
            ledger.insert("unsalted".into(), Value::Bool(true));
        }
        let before = ledger.clone();
        let out = f(&mut ledger);
        if ledger != before {
            self.write_file(&path, &ledger);
        }
        Some(out)
    }

    /// A seed value as given to `--from`: a key stands as itself; a
    /// reference resolves to the key this machine recorded for it. `Err`
    /// names a reference this machine has no record of.
    pub fn resolve(&self, v: &str) -> Result<String, String> {
        if impeccable_common::seed::parse_reference(v).is_none() {
            return Ok(v.to_string());
        }
        // The ledger is named by the reference itself, whatever salt made it.
        impeccable_common::seed::resolve(&self.cwd, v).ok_or_else(|| v.to_string())
    }

    /// Records the key behind a reference, for every roll whatever the
    /// telemetry setting: it is what makes the reference usable.
    pub fn record_key(&self, key: &str) {
        self.update(key, |l| {
            l.insert("key".into(), Value::String(key.to_string()));
        });
        self.prune();
    }

    /// Records an API-dealt round, replacing an earlier record of the same
    /// round (a reproduced roll is a new presentation, so it gets a fresh
    /// `pingId` and no sent state). Telemetry only.
    pub fn record_round(&self, key: &str, round: &SeedRound) {
        let ping_id = impeccable_common::seed::random_hex32();
        self.update(key, |l| {
            l.insert("key".into(), Value::String(key.to_string()));
            let rounds = l.entry("rounds").or_insert_with(|| Value::Object(Map::new()));
            if !rounds.is_object() {
                *rounds = Value::Object(Map::new());
            }
            let mut r = Map::new();
            r.insert("scope".into(), Value::String(round.scope.clone()));
            if let Some(m) = &round.mode {
                r.insert("mode".into(), Value::String(m.clone()));
            }
            if let Some(reg) = &round.register {
                r.insert("register".into(), Value::String(reg.clone()));
            }
            if let Some(p) = round.pool_revision.as_ref().filter(|p| valid_pool_revision(p)) {
                r.insert("poolRevision".into(), Value::String(p.clone()));
            }
            if let Some(h) = round.hand_id.as_ref().filter(|h| valid_hand_id(h)) {
                r.insert("handId".into(), Value::String(h.clone()));
            }
            r.insert("dealt".into(), json!(round.dealt));
            if let Some(p) = &ping_id {
                r.insert("pingId".into(), Value::String(p.clone()));
            }
            rounds.as_object_mut().unwrap().insert(round.reroll.to_string(), Value::Object(r));
        });
        self.prune();
    }

    /// The recorded round `reroll` for `key`, or the latest recorded round
    /// when `reroll` is `None` (a choice resolves on the round last dealt).
    pub fn round(&self, key: &str, reroll: Option<usize>) -> Option<SeedRound> {
        let ledger = Self::read_file(&self.file(key))?;
        let rounds = ledger.get("rounds")?.as_object()?;
        let n = match reroll {
            Some(n) => n,
            None => rounds.keys().filter_map(|k| k.parse::<usize>().ok()).max()?,
        };
        rounds.get(&n.to_string()).map(|v| round_from(v, n))
    }

    fn with_round<R>(&self, key: &str, reroll: usize, f: impl FnOnce(&mut Map<String, Value>) -> R) -> Option<R> {
        self.update(key, |l| {
            let r = l.get_mut("rounds").and_then(Value::as_object_mut).and_then(|rs| rs.get_mut(&reroll.to_string())).and_then(Value::as_object_mut)?;
            Some(f(r))
        })
        .flatten()
    }

    /// The page's claim on a round before it sends: a round with no choice
    /// state becomes `pending`. Returns the round (with its `pingId`) when
    /// the claim took; `None` when the round is unknown or already claimed.
    pub fn claim(&self, key: &str, reroll: usize) -> Option<SeedRound> {
        self.with_round(key, reroll, |r| {
            if r.contains_key("recorded") {
                return None;
            }
            r.insert("recorded".into(), Value::String("pending".into()));
            r.insert("pendingAt".into(), Value::from(crate::util::now_ms() as u64));
            Some(round_from(&Value::Object(r.clone()), reroll))
        })
        .flatten()
    }

    /// Whether a completion for the report `ping_id` still belongs to this
    /// round: a reproduced round gets a new `pingId`, so a send for the old
    /// one that finishes late touches nothing.
    fn same_report(r: &Map<String, Value>, ping_id: Option<&str>) -> bool {
        r.get("pingId").and_then(Value::as_str) == ping_id
    }

    /// A confirmed send of the report `ping_id` by `by`: the round's later
    /// reruns are no-ops. Ignored when the round now names another report.
    pub fn confirm(&self, key: &str, reroll: usize, by: &str, ping_id: Option<&str>) {
        self.with_round(key, reroll, |r| {
            if Self::same_report(r, ping_id) {
                r.insert("recorded".into(), Value::String(by.to_string()));
                r.remove("pendingAt");
            }
        });
    }

    /// A send of the report `ping_id` that failed: a still-pending round
    /// goes back to no state, so the agent's rerun sends it. Ignored when
    /// the round now names another report.
    pub fn release(&self, key: &str, reroll: usize, ping_id: Option<&str>) {
        self.with_round(key, reroll, |r| {
            if Self::same_report(r, ping_id) && r.get("recorded").and_then(Value::as_str) == Some("pending") {
                r.remove("recorded");
                r.remove("pendingAt");
            }
        });
    }

    /// Drops the rounds of ledgers untouched for `SEED_TTL_MS`, keeping the
    /// key so their references keep resolving.
    fn prune(&self) {
        let Ok(entries) = std::fs::read_dir(self.dir()) else { return };
        for e in entries.flatten() {
            let path = e.path().to_string_lossy().into_owned();
            if !path.ends_with(".json") {
                continue;
            }
            let old = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_millis() as f64 > SEED_TTL_MS);
            if !old || !Self::read_file(&path).is_some_and(|l| l.contains_key("rounds")) {
                continue;
            }
            if let Some(_guard) = lock(&path) {
                // Re-checked under the lock: another process may have written
                // a round, or claimed one, since the unlocked look above.
                let still_old = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_millis() as f64 > SEED_TTL_MS);
                if let Some(mut l) = Self::read_file(&path).filter(|_| still_old) {
                    let pending = l.get("rounds").and_then(Value::as_object).is_some_and(|rs| rs.values().any(|r| r.get("recorded").and_then(Value::as_str) == Some("pending")));
                    if !pending && l.remove("rounds").is_some() {
                        self.write_file(&path, &l);
                    }
                }
            }
        }
    }
}

// ---- `impeccable telemetry off|on|status` -------------------------------------


/// `impeccable telemetry [off|on|status]`: telemetry is removed in this
/// fork, so every form prints the same line and changes nothing.
pub fn run(_args: &[String], io: &mut impeccable_common::Io) -> i32 {
    io.out("telemetry disabled in this fork\n");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("impeccable-telemetry-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().into_owned()
    }

    fn env(pairs: &[(&str, &str)]) -> Env {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    pub(crate) fn test_ledger(cwd: &str) -> Ledger {
        Ledger::new(&env(&[("IMPECCABLE_SEED_SALT", "test-salt")]), cwd)
    }

    fn round(n: usize, dealt: &[&str]) -> SeedRound {
        SeedRound {
            scope: "direction".into(),
            reroll: n,
            mode: Some("persuade".into()),
            pool_revision: Some("0123456789ab".into()),
            hand_id: Some("00112233445566778899aabbccddeeff".into()),
            dealt: dealt.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn ledger_rounds_carry_a_ping_id_and_move_through_pending_to_confirmed() {
        let cwd = temp_dir("ledger");
        let l = test_ledger(&cwd);
        assert_eq!(l.round("k1", None), None);
        l.record_round("k1", &round(0, &["a", "b"]));
        l.record_round("k1", &round(1, &["c"]));
        assert_eq!(l.round("k1", None).unwrap().dealt, vec!["c"]);
        let r0 = l.round("k1", Some(0)).unwrap();
        assert_eq!((r0.dealt.clone(), r0.recorded.clone()), (vec!["a".to_string(), "b".to_string()], None));
        let ping_id = r0.ping_id.clone().unwrap();
        assert!(valid_hand_id(&ping_id), "{ping_id}");
        assert_ne!(l.round("k1", Some(1)).unwrap().ping_id.unwrap(), ping_id, "one id per round");
        assert_eq!(l.round("k1", Some(5)), None);
        // The page claims, the claim is exclusive, a failure releases it.
        let claimed = l.claim("k1", 0).unwrap();
        assert_eq!(claimed.ping_id.as_deref(), Some(ping_id.as_str()));
        assert_eq!(l.claim("k1", 0), None);
        let pending = l.round("k1", Some(0)).unwrap();
        assert!(pending.recorded.as_deref() == Some("pending") && pending.pending_at.is_some() && !pending.confirmed());
        let id = Some(ping_id.as_str());
        l.release("k1", 0, id);
        assert_eq!(l.round("k1", Some(0)).unwrap().recorded, None);
        // A confirmed send is final; release never undoes it.
        l.claim("k1", 0).unwrap();
        l.confirm("k1", 0, "picker", id);
        l.release("k1", 0, id);
        let done = l.round("k1", Some(0)).unwrap();
        assert!(done.confirmed() && done.pending_at.is_none(), "{done:?}");
        assert_eq!(l.round("k1", Some(0)).unwrap().ping_id.as_deref(), Some(ping_id.as_str()));
        // Malformed values are never stored.
        let mut r = round(2, &[]);
        r.pool_revision = Some("NOT-HEX".into());
        r.hand_id = Some("00112233445566778899AABBCCDDEEFF".into());
        l.record_round("k1", &r);
        let back = l.round("k1", Some(2)).unwrap();
        assert_eq!((back.pool_revision, back.hand_id), (None, None));
        // Files are named by the reference's hex and hold no time but a pending send's.
        l.record_round("../Evil Key", &round(0, &["x"]));
        for e in std::fs::read_dir(format!("{}/.impeccable/questions/seeds", cwd)).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            assert!(name.len() == 21 && name.ends_with(".json") && !name.contains("k1") && !name.contains("Evil"), "{name}");
            assert!(!std::fs::read_to_string(e.path()).unwrap().contains("\"at\""));
        }
        assert_eq!(l.resolve(&l.reference("../Evil Key").unwrap()).unwrap(), "../Evil Key");
        assert_eq!(std::fs::read_to_string(format!("{}/.impeccable/questions/.gitignore", cwd)).unwrap(), format!("{}\n*\n", impeccable_common::git::RUNTIME_IGNORE_HEADER));
    }

    #[test]
    fn a_late_completion_for_a_replaced_round_changes_nothing() {
        let cwd = temp_dir("ledger-late");
        let l = test_ledger(&cwd);
        l.record_round("k1", &round(0, &["a"]));
        let old = l.claim("k1", 0).unwrap().ping_id;
        // The round is reproduced (a new presentation, a new report) while the
        // old send is still in flight.
        l.record_round("k1", &round(0, &["a"]));
        let fresh = l.round("k1", Some(0)).unwrap();
        assert_ne!(fresh.ping_id, old);
        // The old send finishing, either way, leaves the new round alone.
        l.confirm("k1", 0, "picker", old.as_deref());
        assert_eq!(l.round("k1", Some(0)).unwrap().recorded, None, "a late success does not record the new round");
        let new_claim = l.claim("k1", 0).unwrap();
        l.release("k1", 0, old.as_deref());
        assert_eq!(l.round("k1", Some(0)).unwrap().recorded.as_deref(), Some("pending"), "a late failure does not clear the new claim");
        l.confirm("k1", 0, "picker", new_claim.ping_id.as_deref());
        assert!(l.round("k1", Some(0)).unwrap().confirmed());
    }

    #[test]
    fn no_salt_means_no_reference_never_an_unsalted_one() {
        let cwd = temp_dir("no-salt");
        let home = format!("{}/home", cwd);
        std::fs::create_dir_all(format!("{}/.impeccable", home)).unwrap();
        std::fs::write(format!("{}/.impeccable/seed-salt", home), "corrupt").unwrap();
        let l = Ledger::new(&env(&[("HOME", home.as_str()), ("USERPROFILE", home.as_str())]), &cwd);
        assert_eq!(l.reference("k1"), None);
        assert_eq!(l.resolve(&impeccable_common::seed::reference("", "k1")), Err(impeccable_common::seed::reference("", "k1")), "an unsalted hash resolves to nothing");
        // Telemetry rounds still work by key.
        l.record_round("k1", &round(0, &["a"]));
        assert!(l.round("k1", Some(0)).is_some());
    }

    #[test]
    fn concurrent_ledger_updates_lose_nothing() {
        let cwd = temp_dir("ledger-race");
        let l = test_ledger(&cwd);
        for n in 0..8 {
            l.record_round("k1", &round(n, &["a"]));
        }
        // Eight threads, each claiming and confirming its own round of the
        // same file at once: every write survives the others.
        let handles: Vec<_> = (0..8)
            .map(|n| {
                let cwd = cwd.clone();
                std::thread::spawn(move || {
                    let l = test_ledger(&cwd);
                    let claimed = l.claim("k1", n).unwrap();
                    l.confirm("k1", n, "picker", claimed.ping_id.as_deref());
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        for n in 0..8 {
            assert!(l.round("k1", Some(n)).unwrap().confirmed(), "round {n}");
        }
        let leftovers: Vec<String> = std::fs::read_dir(format!("{}/.impeccable/questions/seeds", cwd)).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.contains(".tmp-") || n.ends_with(".lock")).collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn a_stale_lock_is_taken_over() {
        let cwd = temp_dir("ledger-stale");
        let l = test_ledger(&cwd);
        l.record_key("k1");
        let lock_path = format!("{}.lock", l.file("k1"));
        std::fs::write(&lock_path, "").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(60);
        std::fs::File::options().write(true).open(&lock_path).unwrap().set_modified(old).unwrap();
        l.record_round("k1", &round(0, &[]));
        assert!(l.round("k1", Some(0)).is_some(), "a lock left by a dead process does not block");
    }

    #[test]
    fn pruning_keeps_the_key_behind_a_reference() {
        let cwd = temp_dir("ledger-prune");
        let l = test_ledger(&cwd);
        l.record_round("k1", &round(0, &["a"]));
        let path = l.file("k1");
        let old = std::time::SystemTime::now() - Duration::from_secs(40 * 24 * 3600);
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(old).unwrap();
        // A pending send keeps its rounds even on an old file.
        l.claim("k1", 0).unwrap();
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(old).unwrap();
        l.record_key("other");
        assert!(l.round("k1", Some(0)).is_some(), "a pending round is never pruned");
        let id = l.round("k1", Some(0)).unwrap().ping_id;
        l.release("k1", 0, id.as_deref());
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(old).unwrap();
        l.record_key("other");
        assert_eq!(l.round("k1", None), None, "old rounds go");
        assert_eq!(l.resolve(&l.reference("k1").unwrap()).unwrap(), "k1", "the reference still resolves");
    }

    /// A page from older skill text: no card declares a kind.
    /// (kind, chosenId, option, kindSource) of a ping.
    #[test]
    fn seed_references_are_salted_hashes_that_resolve_only_where_recorded() {
        let cwd = temp_dir("reference");
        let l = test_ledger(&cwd);
        let r = l.reference("3f9a1c2e").unwrap();
        assert!(impeccable_common::seed::parse_reference(&r).is_some() && r.len() == 21 && !r.contains("3f9a1c2e"), "{r}");
        assert_eq!(l.reference("3f9a1c2e").unwrap(), r, "stable");
        assert_ne!(Ledger::new(&env(&[("IMPECCABLE_SEED_SALT", "another")]), &cwd).reference("3f9a1c2e").unwrap(), r, "salted");
        // A key resolves to itself; a reference only once this machine recorded its key.
        assert_eq!(l.resolve("k1").unwrap(), "k1");
        assert_eq!(l.resolve(&r), Err(r.clone()));
        l.record_key("3f9a1c2e");
        assert_eq!(l.resolve(&r).unwrap(), "3f9a1c2e");
        // The key record carries no telemetry round.
        assert_eq!(l.round("3f9a1c2e", None), None);
    }

    #[test]
    fn references_from_either_salt_resolve() {
        let cwd = temp_dir("two-salts");
        let home = format!("{}/home", cwd);
        let e = env(&[("HOME", home.as_str()), ("USERPROFILE", home.as_str())]);
        // A sandboxed run (no home) makes a project salt and a reference under it.
        // A home that cannot hold a directory (it is a file) stands in for a read-only one.
        let blocked = format!("{}/blocked-home", cwd);
        std::fs::write(&blocked, "").unwrap();
        let sandboxed = Ledger::new(&env(&[("HOME", blocked.as_str()), ("USERPROFILE", blocked.as_str())]), &cwd);
        let old = sandboxed.reference("k1").unwrap();
        sandboxed.record_key("k1");
        // A stray home salt appears later (another tool, a restored backup).
        std::fs::create_dir_all(format!("{}/.impeccable", home)).unwrap();
        std::fs::write(format!("{}/.impeccable/seed-salt", home), "0123456789abcdef0123456789abcdef\n").unwrap();
        let l = Ledger::new(&e, &cwd);
        let new = l.reference("k2").unwrap();
        assert_ne!(impeccable_common::seed::parse_reference(&new), None);
        l.record_key("k2");
        assert_eq!(l.resolve(&old).unwrap(), "k1", "the project salt's reference still resolves");
        assert_eq!(l.resolve(&new).unwrap(), "k2");
    }

    #[cfg(unix)]
    #[test]
    fn a_stale_lock_that_cannot_be_removed_gives_up_within_the_deadline() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::path::PathBuf::from(temp_dir("lock-stuck"));
        let path = dir.join("ledger.json").to_string_lossy().into_owned();
        let lock_path = format!("{}.lock", path);
        std::fs::write(&lock_path, "").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(60);
        std::fs::File::options().write(true).open(&lock_path).unwrap().set_modified(old).unwrap();
        // A read-only folder: the stale lock can be neither removed nor replaced.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        if std::fs::remove_file(&lock_path).is_ok() {
            return; // running as root: permissions are not enforced
        }
        let start = std::time::Instant::now();
        assert!(lock(&path).is_none());
        assert!(start.elapsed().as_millis() < LOCK_WAIT_MS + 1_000, "{:?}", start.elapsed());
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));
    }
}

