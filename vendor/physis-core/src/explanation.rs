//! Structured explanation model for accountable AI interpretations.
//!
//! Answers:
//!   - "Why do you believe this?"
//!   - "What evidence supports/contradicts this?"
//!   - "What historical precedents exist?"
//!   - "What predictions were made and what actually happened?"
//!   - "How coherent and fit is this interpretation?"

use serde::{Deserialize, Serialize};

use crate::coherence_dimensions::CoherenceProfile;
use crate::hypothesis::{Evidence, FitnessBreakdown, HypothesisStatus, Prediction};
use crate::models::Score;
use crate::provenance::ProvenanceChain;

/// A historical precedent (similar case from past cycles or observations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPrecedent {
    pub case_id: String,
    pub description: String,
    pub outcome_worked: bool,
    pub similarity: Score,
    pub context_match: Option<String>,
}

/// A fully structured explanation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationReport {
    /// Subject identifier (hypothesis id, node id, or observation id).
    pub subject_id: String,
    /// Human-readable interpretation statement.
    pub statement: String,
    /// Status in the state machine (Candidate, Supported, Contradicted, Certified, etc.).
    pub status: HypothesisStatus,
    /// List of explicit supporting evidence with sources.
    pub supporting_evidence: Vec<Evidence>,
    /// List of explicit contradicting evidence with sources.
    pub contradicting_evidence: Vec<Evidence>,
    /// Relevant historical precedents and their observed outcomes.
    pub historical_precedents: Vec<HistoricalPrecedent>,
    /// Expected consequences / predictions made.
    pub expected_consequences: Vec<Prediction>,
    /// Observed consequences / prediction outcomes.
    pub observed_consequences: Vec<Prediction>,
    /// Multidimensional coherence profile.
    pub coherence_profile: CoherenceProfile,
    /// Composite coherence score in [0.0, 1.0].
    pub coherence_score: Score,
    /// Decomposed fitness breakdown.
    pub fitness_breakdown: FitnessBreakdown,
    /// Overall empirical fitness in [0.0, 1.0].
    pub fitness_score: Score,
    /// Epistemic confidence in [0.0, 1.0].
    pub confidence: Score,
    /// Full provenance chain tracing source and reasoning steps.
    pub provenance_chain: ProvenanceChain,
    /// Formatted explanation text.
    pub human_readable_summary: String,
}

impl ExplanationReport {
    /// G4: every intake id cited by this report's evidence, in encounter
    /// order — each resolves to the ingest episode that produced a link, so
    /// a reader can jump from an explanation to the raw intake in the audit
    /// trail.
    pub fn cited_intake_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        for e in &self.supporting_evidence {
            if let Some(id) = &e.intake_id {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
        for e in &self.contradicting_evidence {
            if let Some(id) = &e.intake_id {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
        ids
    }

    /// Alias for render_ascii.
    pub fn ascii_summary(&self) -> String {
        self.render_ascii()
    }

    /// Generate a structured human-readable representation of the explanation.
    pub fn render_ascii(&self) -> String {
        let mut out = String::new();
        out.push_str("=== PHYSIS EXPLANATION REPORT ===\n");
        out.push_str(&format!("INTERPRETATION:\n  {}\n\n", self.statement));
        out.push_str(&format!(
            "STATUS:\n  {} (Confidence: {:.2}, Coherence: {:.2}, Fitness: {:.2})\n\n",
            self.status.as_str(),
            self.confidence,
            self.coherence_score,
            self.fitness_score
        ));

        out.push_str(&format!(
            "COHERENCE PROFILE:\n  {}\n\n",
            self.coherence_profile.summary()
        ));

        out.push_str(&format!(
            "FITNESS BREAKDOWN:\n  empirical={:.2} pred_succ={:.2} logic={:.2} sem={:.2} (contra_penalty=-{:.2}, fail_penalty=-{:.2})\n\n",
            self.fitness_breakdown.empirical_support,
            self.fitness_breakdown.predictive_success,
            self.fitness_breakdown.logical_consistency,
            self.fitness_breakdown.semantic_fit,
            self.fitness_breakdown.contradiction_penalty,
            self.fitness_breakdown.failed_prediction_penalty
        ));

        out.push_str(&format!(
            "SUPPORTING EVIDENCE ({}):\n",
            self.supporting_evidence.len()
        ));
        if self.supporting_evidence.is_empty() {
            out.push_str("  (none recorded)\n");
        } else {
            for e in &self.supporting_evidence {
                out.push_str(&format!(
                    "  ✓ [{}] {} (conf: {:.2})\n",
                    e.source, e.claim, e.confidence
                ));
            }
        }
        out.push('\n');

        out.push_str(&format!(
            "CONTRADICTING EVIDENCE ({}):\n",
            self.contradicting_evidence.len()
        ));
        if self.contradicting_evidence.is_empty() {
            out.push_str("  (none recorded)\n");
        } else {
            for e in &self.contradicting_evidence {
                out.push_str(&format!(
                    "  ✗ [{}] {} (conf: {:.2})\n",
                    e.source, e.claim, e.confidence
                ));
            }
        }
        out.push('\n');

        if !self.historical_precedents.is_empty() {
            out.push_str(&format!(
                "HISTORICAL PRECEDENTS ({}):\n",
                self.historical_precedents.len()
            ));
            for p in &self.historical_precedents {
                let res = if p.outcome_worked {
                    "SUCCESS"
                } else {
                    "FAILURE"
                };
                out.push_str(&format!(
                    "  • [{}] {} (sim: {:.2}) → {}\n",
                    p.case_id, p.description, p.similarity, res
                ));
            }
            out.push('\n');
        }

        if !self.expected_consequences.is_empty() {
            out.push_str(&format!(
                "PREDICTIONS & OUTCOMES ({}):\n",
                self.expected_consequences.len()
            ));
            for pred in &self.expected_consequences {
                let status = match pred.correct {
                    Some(true) => "VERIFIED TRUE",
                    Some(false) => "REFUTED FALSE",
                    None => "PENDING OBSERVATION",
                };
                out.push_str(&format!(
                    "  • Prediction: {}\n    Outcome: {:?} [{}]\n",
                    pred.statement, pred.actual_outcome, status
                ));
            }
            out.push('\n');
        }

        out.push_str(&format!(
            "PROVENANCE:\n  {}\n",
            self.provenance_chain.summary()
        ));
        out
    }
}
