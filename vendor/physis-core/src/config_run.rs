//! Directive 2 §16 — one configuration mechanism for the run pipeline.
//!
//! The pipeline reads [`RunConfig`] (JSON), never a hardcoded model/table/corpus.
//! Command-line flags overrides the file. Result:
//!
//! ```text
//! physis-core run --config demo.json
//! physis-core run --config demo.json --model smollm2-360m --ngram my-5gram
//! ```
//!
//! Changing model or n-gram never requires a source change and never shuffles
//! Physis — the boundaries stay dirty-clean per the infra directive.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgramSel {
    /// A registry id (previously built/imported), or null to build on the fly.
    pub registry_id: Option<String>,
    /// N-gram order used when building on the fly.
    pub order: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Corpus directory (markdown/txt) to map AND to build the table from.
    pub corpus: PathBuf,
    /// Query whose continuation/context the pipeline answers.
    pub query: String,
    /// Context token budget for the compression leg.
    pub budget: usize,
    /// Model identifier — a registry label; the backend is chosen by the
    /// engine, not by the config. `null` ⇒ engine default (offline decoder).
    pub model: Option<String>,
    pub ngram: NgramSel,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            corpus: PathBuf::from("examples/demo-corpus"),
            query: "the pump".into(),
            budget: 400,
            model: None,
            ngram: NgramSel {
                registry_id: None,
                order: 5,
            },
        }
    }
}

/// Load JSON config; unparsable/absent is a hard error with the file path so
/// operators see exactly which artifact to fix.
pub fn load_config(path: &PathBuf) -> anyhow::Result<RunConfig> {
    anyhow::ensure!(path.is_file(), "config not found: {}", path.display());
    Ok(serde_json::from_slice::<RunConfig>(&std::fs::read(path)?)?)
}

/// Write the example config (used by `physis-core run --example` and the test).
pub fn example_json() -> String {
    serde_json::to_string_pretty(&RunConfig::default()).unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_defaults() -> anyhow::Result<()> {
        let cfg = RunConfig::default();
        let bytes = serde_json::to_vec_pretty(&cfg)?;
        let back: RunConfig = serde_json::from_slice(&bytes)?;
        assert_eq!(
            back.corpus.display().to_string(),
            cfg.corpus.display().to_string()
        );
        assert_eq!(back.budget, 400);
        assert_eq!(back.ngram.order, 5);
        assert!(back.model.is_none());
        Ok(())
    }

    #[test]
    fn missing_file_is_a_clear_error() {
        let e = load_config(&PathBuf::from("definitely-not-here.json")).unwrap_err();
        assert!(e.to_string().contains("definitely-not-here"));
    }
}
