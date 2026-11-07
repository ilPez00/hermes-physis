//! Multi-dimensional coherence model.
//!
//! Physis distinguishes:
//!
//!   semantic coherence      — how well does this match the ontology and vocabulary?
//!   logical coherence       — is this internally consistent?
//!   temporal coherence      — is this consistent with the timeline?
//!   causal coherence        — does the cause-effect chain hold?
//!   procedural coherence    — does this follow the expected process steps?
//!   empirical coherence     — does this match observed measurements?
//!
//! These are kept separate internally. A composite score can be calculated
//! for ranking, but the underlying dimensions remain inspectable.

use serde::{Deserialize, Serialize};

use crate::models::Score;

/// One dimension of coherence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoherenceDimension {
    /// Score in [0.0, 1.0].
    pub score: Score,
    /// Weight in the composite (default 1.0).
    pub weight: Score,
    /// Human-readable label.
    pub label: String,
}

impl Default for CoherenceDimension {
    fn default() -> Self {
        Self::neutral("unspecified")
    }
}

impl CoherenceDimension {
    pub fn new(label: impl Into<String>, score: Score, weight: Score) -> Self {
        Self {
            label: label.into(),
            score: score.clamp(0.0, 1.0),
            weight: weight.max(0.0),
        }
    }

    pub fn neutral(label: impl Into<String>) -> Self {
        Self::new(label, 0.5, 1.0)
    }
}

/// The full coherence profile of an interpretation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoherenceProfile {
    pub semantic: CoherenceDimension,
    pub logical: CoherenceDimension,
    pub temporal: CoherenceDimension,
    pub causal: CoherenceDimension,
    pub procedural: CoherenceDimension,
    pub empirical: CoherenceDimension,
}

impl CoherenceProfile {
    pub fn new() -> Self {
        Self {
            semantic: CoherenceDimension::neutral("semantic"),
            logical: CoherenceDimension::neutral("logical"),
            temporal: CoherenceDimension::neutral("temporal"),
            causal: CoherenceDimension::neutral("causal"),
            procedural: CoherenceDimension::neutral("procedural"),
            empirical: CoherenceDimension::neutral("empirical"),
        }
    }

    /// Weighted composite score. Returns 0.0 if all weights are zero.
    pub fn composite(&self) -> Score {
        let total_weight = self.semantic.weight
            + self.logical.weight
            + self.temporal.weight
            + self.causal.weight
            + self.procedural.weight
            + self.empirical.weight;
        if total_weight == 0.0 {
            return 0.0;
        }
        (self.semantic.score * self.semantic.weight
            + self.logical.score * self.logical.weight
            + self.temporal.score * self.temporal.weight
            + self.causal.score * self.causal.weight
            + self.procedural.score * self.procedural.weight
            + self.empirical.score * self.empirical.weight)
            / total_weight
    }

    /// Set a dimension by name (convenience for dynamic updates).
    pub fn set(&mut self, label: &str, score: Score, weight: Score) {
        let dim = CoherenceDimension::new("", score, weight);
        match label {
            "semantic" => self.semantic = dim,
            "logical" => self.logical = dim,
            "temporal" => self.temporal = dim,
            "causal" => self.causal = dim,
            "procedural" => self.procedural = dim,
            "empirical" => self.empirical = dim,
            _ => {}
        }
    }

    /// Short summary string.
    pub fn summary(&self) -> String {
        format!(
            "sem={:.2} log={:.2} tmp={:.2} cau={:.2} pro={:.2} emp={:.2} → {:.2}",
            self.semantic.score,
            self.logical.score,
            self.temporal.score,
            self.causal.score,
            self.procedural.score,
            self.empirical.score,
            self.composite()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_weights_dimensions() {
        let mut profile = CoherenceProfile::new();
        profile.semantic = CoherenceDimension::new("semantic", 0.9, 2.0);
        profile.empirical = CoherenceDimension::new("empirical", 0.1, 1.0);
        // composite = (0.9*2 + 0.1*1 + 0.5*4) / 7 = (1.8 + 0.1 + 2.0) / 7 = 3.9/7 ≈ 0.557
        let c = profile.composite();
        assert!((c - 0.557).abs() < 0.01, "composite was {c}");
    }
}
