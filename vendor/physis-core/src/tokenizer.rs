//! Tokenizer abstraction (infrastructure layer).
//!
//! Both a [`crate::model_provider::ModelProvider`] and a lexical
//! [`crate::ngram_table::NGramTable`] record which tokenizer they were built
//! against; the compatibility gate in [`crate::ngram_table`] refuses to mix
//! vocabularies instead of silently translating between them.
//!
//! Deterministic by contract: same string in, same token ids out, always.

use serde::{Deserialize, Serialize};

/// What a tokenizer implementation provides. Every method is total — no
/// backend may panic on odd input; unknown text maps to a defined id.
pub trait Tokenizer: Send + Sync {
    /// Stable identifier recorded in manifests (`whitespace-v1`, …).
    fn id(&self) -> &'static str;
    /// Encode text into token strings.
    fn encode(&self, text: &str) -> Vec<String>;
    /// Decode tokens back into text (round-trip lossy is allowed, but
    /// documented per implementation).
    fn decode(&self, tokens: &[String]) -> String;
    /// Vocabulary size as observed by this tokenizer (bound or exact).
    fn vocab_size(&self) -> usize;
    /// Human-readable provenance line for manifests and the Studio.
    fn metadata(&self) -> TokenizerInfo {
        TokenizerInfo {
            id: self.id().to_string(),
            vocab_size: self.vocab_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenizerInfo {
    pub id: String,
    pub vocab_size: usize,
}

/// Why a tokenizer/table/model triple is incompatible — surfaced verbatim to
/// the user, never silently worked around (house rule: no silent translation).
#[derive(Debug, Clone, PartialEq)]
pub struct Incompatible {
    pub what: String,
    pub expected: String,
    pub found: String,
}

impl std::fmt::Display for Incompatible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "incompatible {}: expected {}, found {}. \
             Use a compatible artifact or rebuild it.",
            self.what, self.expected, self.found
        )
    }
}

impl std::error::Error for Incompatible {}

/// Check two tokenizer ids for compatibility. Equal ids are compatible;
/// everything else is refused — we do not guess between subword schemes.
pub fn check_compatible(expected: &str, found: &str) -> Result<(), Incompatible> {
    if expected == found {
        Ok(())
    } else {
        Err(Incompatible {
            what: "tokenizer".into(),
            expected: expected.into(),
            found: found.into(),
        })
    }
}

/// Whitespace tokenizer: split on non-alphanumeric runs, lowercase. The
/// deterministic default for lexical tables built from plain corpora.
#[derive(Debug, Clone, Default)]
pub struct WhitespaceTokenizer {
    vocab_cap: usize,
}

impl WhitespaceTokenizer {
    pub fn new(vocab_cap: usize) -> Self {
        Self { vocab_cap }
    }
}

impl Tokenizer for WhitespaceTokenizer {
    fn id(&self) -> &'static str {
        "whitespace-v1"
    }
    fn encode(&self, text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_lowercase())
            .collect()
    }
    fn decode(&self, tokens: &[String]) -> String {
        tokens.join(" ")
    }
    fn vocab_size(&self) -> usize {
        // Open vocabulary; the cap (when set) is enforced by the table
        // builder, so report it as the effective bound.
        if self.vocab_cap == 0 {
            usize::MAX
        } else {
            self.vocab_cap
        }
    }
}

/// Structural tokenizer: wraps Physis classification output. Its tokens are
/// structural symbols (`DOMAIN×MODE`), not words — used by structural tables
/// where the "vocabulary" is the grid, not the language.
#[derive(Debug, Clone, Default)]
pub struct StructuralTokenizer {
    pub states: Vec<String>,
}

impl Tokenizer for StructuralTokenizer {
    fn id(&self) -> &'static str {
        "physis-structural-v1"
    }
    fn encode(&self, text: &str) -> Vec<String> {
        // Pass through pre-classified symbol sequences: input is expected to
        // already look like "HEAL×MAINTAIN FABRICATE×PLAN ...".
        text.split_whitespace().map(|s| s.to_string()).collect()
    }
    fn decode(&self, tokens: &[String]) -> String {
        tokens.join(" ")
    }
    fn vocab_size(&self) -> usize {
        if self.states.is_empty() {
            70 // the 5×14 grid
        } else {
            self.states.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_roundtrip_is_stable() {
        let t = WhitespaceTokenizer::new(0);
        let toks = t.encode("Pump maintenance, required!");
        assert_eq!(toks, vec!["pump", "maintenance", "required"]);
        assert_eq!(t.decode(&toks), "pump maintenance required");
        // determinism: same input, same output, twice
        assert_eq!(t.encode("Pump maintenance, required!"), toks);
    }

    #[test]
    fn compatibility_gate_refuses_mismatches() {
        assert!(check_compatible("whitespace-v1", "whitespace-v1").is_ok());
        let err = check_compatible("whitespace-v1", "smollm2-bpe").unwrap_err();
        assert!(err.to_string().contains("rebuild"));
    }
}
