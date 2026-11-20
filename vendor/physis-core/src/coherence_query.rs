//! Coherence Query API — first-class query interface for epistemic questions.
//!
//! Enables structured inquiries into why things are believed, what contradicts them,
//! what happened after predictions, and what ontology gaps exist.

use serde::{Deserialize, Serialize};

use crate::explanation::ExplanationReport;
use crate::hypothesis::{HypothesisStatus, Prediction, Revision};
use crate::models::Score;
use crate::relation::TypedEdge;
use crate::revision::AffectedSetExplanation;
use crate::support::SupportSnapshot;

/// Query request specifying the question being asked of Physis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpistemicQuery {
    /// Why is hypothesis or node X believed?
    WhyIsBelieved { id: String },
    /// What evidence and claims contradict X?
    WhatContradicts { id: String },
    /// What happened after prediction(s) associated with X?
    WhatHappenedAfterPrediction { id: String },
    /// Find historical cases similar to the query vector that succeeded.
    SimilarCasesSucceeded { embedding: Vec<f32>, limit: usize },
    /// Find historical cases similar to the query vector that failed.
    SimilarCasesFailed { embedding: Vec<f32>, limit: usize },
    /// Why did Physis change its interpretation of X?
    WhyChangedInterpretation { id: String },
    /// Which ontology concepts or domains have gaps or insufficient coverage?
    InsufficientOntologyConcepts,
    /// List top hypotheses ranked by composite fitness / coherence.
    StrongestHypotheses { limit: usize },
    /// List all hypotheses with failed predictions for audit.
    FailedPredictions { limit: usize },
    /// List hypotheses with highest empirical support.
    HighestEmpiricalFitness { limit: usize },
    /// Deterministic "why supported / affected now" snapshot for a hypothesis
    /// (evidence/provenance intake citations, DependsOn dependents, recent audit
    /// events). Read-only diagnostic, not a verdict.
    WhySupported {
        id: String,
        recent_event_limit: usize,
    },
    /// DependsOn-selected affected hypothesis set for a node mutation origin
    /// (read-only diagnostic / blast-radius explanation).
    WhatIsAffectedBy { node_id: String },
}

/// Structured result of an epistemic query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpistemicQueryResult {
    Explanation(Box<ExplanationReport>),
    Contradictions {
        subject_id: String,
        contradicting_claims: Vec<String>,
        conflicting_edges: Vec<TypedEdge>,
    },
    PredictionsOutcome {
        subject_id: String,
        predictions: Vec<Prediction>,
    },
    SimilarCases {
        cases: Vec<(String, Score, bool)>, // (id/label, similarity, outcome_succeeded)
    },
    RevisionHistory {
        subject_id: String,
        current_status: HypothesisStatus,
        revisions: Vec<Revision>,
    },
    OntologyGaps {
        uncovered_cells: Vec<String>,
        candidate_domains: Vec<String>,
    },
    RankedHypotheses {
        hypotheses: Vec<HypothesisSummary>,
    },
    FailedPredictionsList {
        failures: Vec<FailedPredictionSummary>,
    },
    /// Support/affected snapshot for a hypothesis (None if unknown).
    SupportSnapshot {
        subject_id: String,
        snapshot: Option<Box<SupportSnapshot>>,
    },
    /// DependsOn-affected set explanation for a node.
    AffectedSet {
        explanation: AffectedSetExplanation,
    },
}

/// Compact summary of a hypothesis for ranked listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisSummary {
    pub id: String,
    pub statement: String,
    pub status: HypothesisStatus,
    pub fitness: Score,
    pub coherence: Score,
    pub confidence: Score,
    pub evidence_count: usize,
    pub predictions_count: usize,
}

/// Summary of a failed prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedPredictionSummary {
    pub hypothesis_id: String,
    pub hypothesis_statement: String,
    pub prediction_statement: String,
    pub expected_outcome: Option<String>,
    pub actual_outcome: Option<String>,
    pub made_at: chrono::DateTime<chrono::Utc>,
    pub observed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_and_affected_query_variants_are_well_formed() {
        let q = EpistemicQuery::WhySupported {
            id: "h1".into(),
            recent_event_limit: 5,
        };
        let j = serde_json::to_value(&q).unwrap();
        assert!(j.get("WhySupported").is_some());

        let q2 = EpistemicQuery::WhatIsAffectedBy {
            node_id: "n1".into(),
        };
        let j2 = serde_json::to_value(&q2).unwrap();
        assert!(j2.get("WhatIsAffectedBy").is_some());
    }
}
