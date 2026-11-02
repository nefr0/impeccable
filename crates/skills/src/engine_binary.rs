//! After a skill directory is written, make it self-contained: when its
//! `scripts/VERSION` exists and no `scripts/bin/<os>-<arch>/impeccable[.exe]`
//! is present, copy the engine binary from `IMPECCABLE_BIN` or the running
//! executable and set the executable bit. No network. Not part of the JS.

use impeccable_common::Io;

use crate::providers::Sys;
use crate::util::{self, jsp};

/// The `<os>-<arch>` tag the launcher computes (`darwin|linux|windows` x
/// `arm64|x64`); `None` on a platform without a release asset.
pub fn platform_tag() -> Option<(&'static str, &'static str)> {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        return None;
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return None;
    };
    Some((os, arch))
}

/// Where the binary for `(os, arch)` lives inside a skill directory.
pub fn binary_path(skill_dir: &str, os: &str, arch: &str) -> String {
    let name = if os == "windows" { "impeccable.exe" } else { "impeccable" };
    jsp::join(&[skill_dir, "scripts", "bin", &format!("{os}-{arch}"), name])
}

/// Belt and suspenders for the executable bit: chmod +x the launcher
/// (`scripts/impeccable`) and every staged engine binary
/// (`scripts/bin/**/impeccable*`) inside a skill dir. Zip extraction and the
/// dir copy preserve modes themselves, but any channel that loses them (a
/// zip with no unix modes, a mode-dropping copier) would otherwise turn every
/// hook and Setup call into "Permission denied". No-op off unix and for
/// missing paths.
pub fn ensure_executable_scripts(skill_dir: &str) {
    let launcher = jsp::join(&[skill_dir, "scripts", "impeccable"]);
    if util::exists(&launcher) {
        let _ = util::set_executable(&launcher);
    }
    let bin_root = jsp::join(&[skill_dir, "scripts", "bin"]);
    for tag in util::read_dir_names(&bin_root).unwrap_or_default() {
        let dir = jsp::join(&[&bin_root, &tag]);
        if !util::is_dir(&dir) {
            continue;
        }
        for name in util::read_dir_names(&dir).unwrap_or_default() {
            if name.starts_with("impeccable") {
                let _ = util::set_executable(&jsp::join(&[&dir, &name]));
            }
        }
    }
}

/// The engine binary to copy: `IMPECCABLE_BIN` when set, else this running
/// executable. No network is involved.
fn engine_source(sys: &Sys) -> Result<String, String> {
    if let Some(bin) = sys.env.get("IMPECCABLE_BIN").filter(|v| !v.is_empty()) {
        let resolved = jsp::resolve(&sys.cwd, &[bin]);
        if !util::exists(&resolved) {
            return Err(format!("IMPECCABLE_BIN not found: {resolved}"));
        }
        return Ok(resolved);
    }
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("cannot locate the running executable: {e}"))
}

/// Install the engine binary into every skill dir in `skill_dirs` that needs
/// one (a `scripts/VERSION` and no `scripts/bin/<os>-<arch>/impeccable[.exe]`),
/// copied from `IMPECCABLE_BIN` or the running executable. Prints one line per
/// binary installed; failures are reported on stderr and do not fail the
/// install. Also repairs the executable bit on the launcher and any binaries
/// already present in those dirs.
pub fn install_engine_binaries(sys: &Sys, io: &mut Io, skill_dirs: &[String]) {
    for skill_dir in skill_dirs {
        ensure_executable_scripts(skill_dir);
    }
    let Some((os, arch)) = platform_tag() else { return };
    let mut source: Option<Result<String, String>> = None;
    for skill_dir in skill_dirs {
        let version_file = jsp::join(&[skill_dir, "scripts", "VERSION"]);
        let Ok(raw) = util::read_text(&version_file) else { continue };
        let version = raw.trim();
        let dest = binary_path(skill_dir, os, arch);
        if version.is_empty() || util::exists(&dest) {
            continue;
        }
        let src = match source.get_or_insert_with(|| engine_source(sys)) {
            Ok(src) => src.clone(),
            Err(e) => {
                io.err(&format!("Could not install the impeccable engine v{version} for {os}-{arch}: {e}\n"));
                continue;
            }
        };
        let written = util::mkdir_p(&jsp::dirname(&dest))
            .and_then(|_| std::fs::copy(&src, &dest).map(|_| ()).map_err(|e| e.to_string()))
            .and_then(|_| util::set_executable(&dest));
        match written {
            Ok(()) => io.out(&format!(
                "Installed impeccable engine v{version} ({os}-{arch}) into: {}\n",
                sys.format_path_for_display(&dest)
            )),
            Err(e) => io.err(&format!(
                "Could not write the impeccable engine v{version} ({os}-{arch}) into {}: {e}\n",
                sys.format_path_for_display(&dest)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_path_is_platform_keyed() {
        assert_eq!(
            binary_path("/s", "linux", "x64"),
            jsp::join(&["/s", "scripts", "bin", "linux-x64", "impeccable"])
        );
        assert_eq!(
            binary_path("/s", "windows", "arm64"),
            jsp::join(&["/s", "scripts", "bin", "windows-arm64", "impeccable.exe"])
        );
    }

    #[test]
    fn install_copies_the_engine_from_impeccable_bin() {
        let Some((os, arch)) = platform_tag() else { return };
        let dir = std::env::temp_dir().join(format!("impeccable-local-engine-{}", std::process::id()));
        let skill_dir = dir.join("skill").to_string_lossy().into_owned();
        std::fs::create_dir_all(dir.join("skill/scripts")).unwrap();
        std::fs::write(dir.join("skill/scripts/VERSION"), "9.9.9\n").unwrap();
        std::fs::write(dir.join("engine"), "#!/bin/sh\nexit 0\n").unwrap();
        let env: util::Env = [("IMPECCABLE_BIN".to_string(), dir.join("engine").to_string_lossy().into_owned())].into();
        let (mut io, _captured) = Io::captured("", dir.clone(), env.clone());
        install_engine_binaries(&Sys::new(env, skill_dir.clone()), &mut io, &[skill_dir.clone()]);
        assert_eq!(std::fs::read_to_string(binary_path(&skill_dir, os, arch)).unwrap(), "#!/bin/sh\nexit 0\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
