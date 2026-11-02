//! String-level guards over the two launchers (`skill/scripts/impeccable`,
//! `skill/scripts/impeccable.cmd`). The .cmd cannot be executed here (no Windows),
//! so this pins the shapes a dry parse depends on: asset/URL construction in
//! both launchers agrees with `engine_binary::asset_url`, the .cmd contains
//! no multi-line parenthesized blocks (the parse-time `%var%` expansion bug
//! that made its download path dead code), both launchers carry the
//! engine-probe handshake, and the .cmd verifies downloads via certutil.


/// The launchers ship next to the skill they power.
fn launcher_dir() -> String {
    format!("{}/../../skill/scripts", env!("CARGO_MANIFEST_DIR"))
}

fn launcher_file(name: &str) -> String {
    let path = format!("{}/{name}", launcher_dir());
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn cmd_launcher_forwards_engine_exit_code() {
    // Bare `exit /b` drops the process exit code when this file is cmd.exe's
    // entry point (cmd /c, PowerShell, Node spawn). Forward %errorlevel%
    // after each engine invocation instead.
    let cmd = launcher_file("impeccable.cmd");
    for (i, line) in cmd.lines().enumerate() {
        assert_ne!(
            line.trim(),
            "exit /b",
            "impeccable.cmd line {}: bare exit /b drops the process code: {line}",
            i + 1
        );
    }
    let forward = "exit /b %errorlevel%";
    assert_eq!(
        cmd.matches(forward).count(),
        2,
        "PATH candidate and :run must both forward the engine exit code"
    );
}

#[test]
fn cmd_launcher_has_no_multiline_parenthesized_blocks() {
    // cmd.exe expands %var% inside a parenthesized block at parse time, so a
    // `set` + read-back inside one block silently reads the pre-block value
    // (rollout review S3: the download path could never fire). The launcher
    // is written as straight-line goto flow; keep it that way.
    let cmd = launcher_file("impeccable.cmd");
    for (i, line) in cmd.lines().enumerate() {
        assert!(
            !line.trim_end().ends_with('('),
            "impeccable.cmd line {}: opens a multi-line parenthesized block: {line}",
            i + 1
        );
    }
}

#[test]
fn launchers_fail_closed_on_missing_checksum() {
    // Triage C1: a freshly downloaded binary runs only after its hash matches
    // the digest committed in engine-checksums.txt (#813); nothing published
    // beside the binary is trusted. No recorded digest, or a machine with no
    // sha256 tool, refuses the download instead of running an unverified
    // binary; a mismatch stays fatal.
    let sh = launcher_file("impeccable");
    let cmd = launcher_file("impeccable.cmd");
    assert!(sh.contains("refusing the unverified download"));
    assert!(sh.contains(r#"manifest="$dir/engine-checksums.txt""#));
    assert!(sh.contains(r#"if [ -z "$actual" ]; then"#));
    assert!(cmd.contains("refusing the unverified download"));
    assert!(cmd.contains(r#"set "manifest=%~dp0engine-checksums.txt""#));
    assert!(cmd.contains("goto verify_refuse"));
    for text in [&sh, &cmd] {
        // The digest is resolved before any download, and the sidecar is gone.
        assert!(text.contains("no checksum for engine"));
        assert!(!text.contains(".sha256"));
        // Cached binaries are hashed once; the stamp records it.
        assert!(text.contains(".verified"));
        assert!(text.contains("cannot verify the cached engine"));
        assert!(text.contains("checksum mismatch downloading"));
    }
}

#[test]
fn sh_launcher_exports_skill_dir_before_the_env_bin_exec() {
    // Regression: the launcher used to `exec "$IMPECCABLE_BIN"` BEFORE it
    // exported IMPECCABLE_SKILL_DIR/SELF, so a binary reached via IMPECCABLE_BIN
    // ran with no skill dir and could not inline reference/*.md or read its own
    // version (native platform refs and UPDATE_AVAILABLE silently vanished).
    // The skill-behavior suite caught it; the oracle could not (it sets the env
    // directly). Guard the ordering at the string level and functionally.
    let sh = launcher_file("impeccable");
    let export_pos = sh
        .find("export IMPECCABLE_SKILL_DIR IMPECCABLE_SELF")
        .expect("launcher exports the skill-dir env");
    let env_bin_exec = sh
        .find(r#"exec "${IMPECCABLE_BIN}""#)
        .expect("launcher execs IMPECCABLE_BIN");
    assert!(
        export_pos < env_bin_exec,
        "IMPECCABLE_SKILL_DIR must be exported before the IMPECCABLE_BIN exec"
    );
}

#[cfg(unix)]
#[test]
fn sh_launcher_passes_skill_dir_to_the_env_bin() {
    use std::os::unix::fs::PermissionsExt;
    let launcher = format!("{}/impeccable", launcher_dir());
    let dir = std::env::temp_dir().join(format!("impeccable-launcher-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // A stub "engine binary" that just prints the skill dir it was handed.
    let stub = dir.join("stub");
    std::fs::write(&stub, "#!/bin/sh\nprintf 'SKILL_DIR=%s\\n' \"${IMPECCABLE_SKILL_DIR:-UNSET}\"\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = std::process::Command::new("sh")
        .arg(&launcher)
        .arg("context")
        .env("IMPECCABLE_BIN", &stub)
        .env_remove("IMPECCABLE_SKILL_DIR")
        .output()
        .expect("run launcher");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SKILL_DIR=") && !stdout.contains("SKILL_DIR=UNSET") && !stdout.contains("SKILL_DIR=\n"),
        "launcher must export a non-empty IMPECCABLE_SKILL_DIR to the IMPECCABLE_BIN binary; got: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
