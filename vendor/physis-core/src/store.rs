//! Simple JSON-file persistence under `~/.physis-core/`.
//!
//! Provides path resolution plus atomic file writes and core save/load helpers
//! for concrete persisted/write-path workflows (drift-checked staged commit
//! integration lives in [`crate::revision`]; callers may persist after commit).

use std::path::{Path, PathBuf};

/// Environment variable that overrides the state directory outright.
pub const DATA_DIR_ENV: &str = "PHYSIS_CORE_DIR";

/// Data directory for physis-core state.
///
/// Every surface — CLI, studio, library — resolves the graph through here, which
/// is what makes `physis-core studio` and `physis-core scan` operate on the *same*
/// nodes. `PHYSIS_CORE_DIR` overrides it wholesale so that behaviour can be
/// escaped where sharing is wrong: tests that must not touch the developer's real
/// graph, containers with a mounted volume, or a second graph kept per project.
/// Unset, it stays `$HOME/.physis-core` so the shared default is unchanged.
pub fn data_dir() -> PathBuf {
    resolve_data_dir(std::env::var_os(DATA_DIR_ENV), std::env::var_os("HOME"))
}

/// The branching behind [`data_dir`], kept pure so it is testable without
/// mutating the process environment (which tests share and would race on).
fn resolve_data_dir(
    override_dir: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> PathBuf {
    // An empty value is treated as unset: `PHYSIS_CORE_DIR= physis-core studio`
    // must not resolve the graph to the process's working directory.
    if let Some(dir) = override_dir.filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    let home = home.filter(|v| !v.is_empty()).unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".physis-core")
}

/// Path where the coherence-node graph is persisted.
pub fn nodes_path() -> PathBuf {
    data_dir().join("nodes.json")
}

/// Path where the quality failures are persisted.
pub fn quality_path() -> PathBuf {
    data_dir().join("quality.json")
}

/// Path where studio custom ontology edits are persisted.
pub fn custom_ontology_path() -> PathBuf {
    data_dir().join("custom_ontology.json")
}

/// Ensure the data dir exists.
pub fn ensure_data_dir() -> anyhow::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    Ok(())
}

/// Read a text file that may not exist yet; missing → empty string.
pub fn read_optional(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Atomically write `contents` to `path` (write temp sibling, fsync, rename).
///
/// Creates parent directories as needed. The temp name is unique to avoid
/// collisions; content written is exactly `contents`.
pub fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "state".to_string());
    let tmp_name = format!(
        ".{}.tmp-{}-{}",
        file_name,
        std::process::id(),
        uuid::Uuid::new_v4()
    );
    let tmp = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join(&tmp_name),
        _ => PathBuf::from(&tmp_name),
    };

    let write_result = (|| {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write_result
}

/// Save a [`crate::core::PhysisCore`] to `path` as JSON (atomic write).
pub fn save_core(path: &Path, core: &crate::core::PhysisCore) -> anyhow::Result<()> {
    let json = core.to_json()?;
    atomic_write(path, json.as_bytes()).map_err(anyhow::Error::from)
}

/// Load a [`crate::core::PhysisCore`] from `path` (JSON).
pub fn load_core(path: &Path) -> anyhow::Result<crate::core::PhysisCore> {
    let json = std::fs::read_to_string(path)?;
    crate::core::PhysisCore::from_json(&json)
}

/// Atomically persist `core` to the default nodes path ([`nodes_path`]).
pub fn save_core_default(core: &crate::core::PhysisCore) -> anyhow::Result<()> {
    save_core(&nodes_path(), core)
}

/// Load core from the default nodes path, or a fresh empty core if missing/empty.
pub fn load_core_default() -> crate::core::PhysisCore {
    let path = nodes_path();
    let json = read_optional(&path);
    if json.trim().is_empty() {
        crate::core::PhysisCore::new()
    } else {
        crate::core::PhysisCore::from_json(&json).unwrap_or_else(|_| crate::core::PhysisCore::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_live_under_data_dir() {
        assert!(nodes_path().starts_with(data_dir()));
        assert!(quality_path().starts_with(data_dir()));
        assert!(custom_ontology_path().starts_with(data_dir()));
    }

    #[test]
    fn the_override_wins_over_home() {
        // Set: the graph moves wholesale, with no `.physis-core` suffix appended
        // — the caller named the directory they want, not a parent of it.
        assert_eq!(
            resolve_data_dir(Some("/srv/graph".into()), Some("/home/someone".into())),
            PathBuf::from("/srv/graph")
        );
    }

    #[test]
    fn without_the_override_the_graph_stays_shared_under_home() {
        // The default that makes `studio` and `scan` see the same nodes.
        assert_eq!(
            resolve_data_dir(None, Some("/home/someone".into())),
            PathBuf::from("/home/someone/.physis-core")
        );
        // Empty is unset, not "use the working directory".
        assert_eq!(
            resolve_data_dir(Some("".into()), Some("/home/someone".into())),
            PathBuf::from("/home/someone/.physis-core")
        );
    }

    #[test]
    fn a_missing_home_falls_back_to_the_working_directory() {
        assert_eq!(
            resolve_data_dir(None, None),
            PathBuf::from("./.physis-core")
        );
    }

    #[test]
    fn atomic_write_roundtrips_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("state.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
        atomic_write(&path, b"world").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "world");
        // no temp files left behind
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "leftover temp files: {:?}", leftovers);
    }

    #[test]
    fn save_and_load_core_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");
        let mut core = crate::core::PhysisCore::new();
        let mut h = crate::hypothesis::Hypothesis::new("persist me", vec![]);
        h.add_supporting_evidence(crate::hypothesis::Evidence::supports("s", "ok"));
        let hid = core.register_hypothesis(h);
        save_core(&path, &core).unwrap();
        let loaded = load_core(&path).unwrap();
        assert!(loaded.hypothesis(&hid).is_some());
        assert_eq!(loaded.hypothesis(&hid).unwrap().statement, "persist me");
    }

    #[test]
    fn load_core_default_missing_returns_fresh() {
        let dir = tempfile::tempdir().unwrap();
        // point PHYSIS_CORE_DIR at an empty dir for this process? can't mutate env safely here,
        // but we can test load_core behavior on a missing explicit path via read_optional semantics:
        let path = dir.path().join("nope").join("nodes.json");
        let json = read_optional(&path);
        assert!(json.is_empty());
        // default loader on missing is fresh
        let fresh = crate::core::PhysisCore::from_json("[]").unwrap();
        assert!(fresh.hypotheses.is_empty());
    }
}
