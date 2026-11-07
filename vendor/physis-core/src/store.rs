//! Simple JSON-file persistence under `~/.physis-core/`.

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
}
