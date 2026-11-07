//! Which Physis edition is installed on this machine, and where.
//!
//! `physis-core` is the open engine; `physis-pro` is a separate, proprietary
//! product with its own licence gate. Core therefore cannot depend on Pro — the
//! link would drag a proprietary EULA into an Apache-2.0 crate. The two are
//! joined at runtime instead: core looks for the Pro executable on disk and
//! hands work over to it by process exec.
//!
//! Detection is deliberately cheap and free of side effects. Both the `physis`
//! front-door banner and the studio's `/api/edition` handler call it, the latter
//! on every page load, so it never runs a subprocess just to answer "is Pro
//! here?" — it only resolves paths. Whether a *licence* is valid stays Pro's
//! business: core does not hold the verifying key and must not pretend to.

use std::path::{Path, PathBuf};

/// Executable that carries the Pro CLI.
pub const PRO_CLI: &str = "physis-pro";
/// Executable that serves the Pro dashboards (Console, Demo, Study).
pub const PRO_WEB: &str = "physis-pro-web";
/// Executable that carries the Core CLI.
pub const CORE_CLI: &str = "physis-core";

/// Where the upgrade path is documented, shown when Pro is absent.
pub const UPGRADE_URL: &str = "https://github.com/ilPez00/physis-pro";

/// What is installed alongside this build.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Edition {
    /// `physis-pro`, when it can be found.
    pub pro_cli: Option<PathBuf>,
    /// `physis-pro-web`, when it can be found.
    pub pro_web: Option<PathBuf>,
    /// `physis-core`, which the front door delegates core subcommands to.
    pub core_cli: Option<PathBuf>,
}

impl Edition {
    /// Resolve the installed executables.
    pub fn detect() -> Self {
        Self {
            pro_cli: find_executable(PRO_CLI),
            pro_web: find_executable(PRO_WEB),
            core_cli: find_executable(CORE_CLI),
        }
    }

    /// Whether any part of Pro is present.
    pub fn has_pro(&self) -> bool {
        self.pro_cli.is_some() || self.pro_web.is_some()
    }

    /// Short name for banners and the studio header.
    pub fn name(&self) -> &'static str {
        if self.has_pro() {
            "Physis Core + Pro"
        } else {
            "Physis Core"
        }
    }
}

/// Locate `name` as an executable, preferring a build that sits next to the
/// running one.
///
/// The sibling check matters for two common cases the `PATH` alone misses: a
/// `cargo build` tree where nothing is installed yet, and an install directory
/// that a shell has not rehashed. Falls back to scanning `PATH`.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Some(hit) = executable_at(&dir.join(with_exe_suffix(name))) {
                return Some(hit);
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| executable_at(&dir.join(with_exe_suffix(name))))
}

/// `name.exe` on Windows, `name` everywhere else.
fn with_exe_suffix(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// `Some(path)` when the path is a regular file we could plausibly run.
fn executable_at(path: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Without this a same-named directory entry or a non-executable file
        // would be reported as an installed edition.
        if meta.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    Some(path.to_path_buf())
}

/// The one-paragraph pitch shown wherever Pro is absent. Kept here so the CLI
/// banner and the studio's upgrade panel cannot drift apart.
pub const PRO_SUMMARY: &[(&str, &str)] = &[
    (
        "Operations Console",
        "live plant dashboards, OEE, shift reports, incident timelines",
    ),
    (
        "Machine connectors",
        "MQTT, Modbus TCP/RTU, serial and OPC-UA adapters",
    ),
    (
        "Multi-tenant serving",
        "per-tenant state, tokens, and an authenticated HTTP API",
    ),
    (
        "LLM cascade",
        "multi-provider routing with a coherence harness over the answers",
    ),
    (
        "Durable storage",
        "embedded database plus an optional graph mirror",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_is_not_an_executable() {
        let dir = std::env::temp_dir();
        assert!(
            executable_at(&dir).is_none(),
            "a directory must never be reported as an installed binary",
        );
    }

    #[test]
    fn a_missing_path_is_not_an_executable() {
        assert!(executable_at(Path::new("/nonexistent/physis-pro")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_non_executable_file_is_not_an_executable() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "not a program").unwrap();
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert!(
            executable_at(f.path()).is_none(),
            "a readable-but-not-executable file is not an install",
        );
    }

    #[test]
    fn edition_name_reflects_what_was_found() {
        let core_only = Edition {
            pro_cli: None,
            pro_web: None,
            core_cli: Some("/usr/bin/physis-core".into()),
        };
        assert!(!core_only.has_pro());
        assert_eq!(core_only.name(), "Physis Core");

        // Either half of Pro is enough to call the install "Core + Pro": the web
        // server alone is a working Pro deployment.
        let with_web = Edition {
            pro_cli: None,
            pro_web: Some("/usr/bin/physis-pro-web".into()),
            core_cli: None,
        };
        assert!(with_web.has_pro());
        assert_eq!(with_web.name(), "Physis Core + Pro");
    }

    #[test]
    fn the_running_executable_is_discoverable_by_its_own_name() {
        // Guards the sibling lookup, which is what makes `physis` work from a
        // plain `cargo build` tree before anything is installed.
        let exe = std::env::current_exe().unwrap();
        let name = exe.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(find_executable(&name).as_deref(), Some(exe.as_path()));
    }
}
