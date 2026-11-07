//! Coherence Query API — first-class query interface for epistemic questions.
//!
//! Enables structured inquiries into why things are believed, what contradicts them,
//! what happened after predictions, and what ontology gaps exist.

use serde::{Deserialize, Serialize};

use crate::explanation::ExplanationReport;
use crate::hypothesis::{HypothesisStatus, Prediction, Revision};
use crate::models::Score;
use crate::relation::TypedEdge;

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
