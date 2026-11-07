//! Contradiction handling — explicit representation of conflicting claims.
//!
//! Physis must NOT silently overwrite conflicting information.
//! A contradiction is a first-class object with:
//!   - sources
//!   - authority
//!   - timestamps
//!   - confidence
//!   - resolution status

use serde::{Deserialize, Serialize};

use crate::models::Score;
use crate::temporal::TemporalValidity;

/// How a contradiction was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStatus {
    /// A is currently preferred; B retained as contradictory evidence.
    APreferred,
    /// B is currently preferred.
    BPreferred,
    /// Both are retained; contradiction remains open.
    Open,
    /// Both are considered valid in different contexts.
    Contextual,
    /// A third explanation C supersedes both.
    Superseded,
}

/// A structured contradiction between two claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    /// Unique identifier.
    pub id: String,
    /// First claim.
    pub claim_a: ContradictionParty,
    /// Second claim (negation of A).
    pub claim_b: ContradictionParty,
    /// Current resolution.
    pub resolution: ResolutionStatus,
    /// Optional explanation of the resolution.
    #[serde(default)]
    pub explanation: Option<String>,
    /// When this contradiction was detected.
    pub detected_at: chrono::DateTime<chrono::Utc>,
    /// Temporal validity — contradictions can be time-bound.
    pub temporal: TemporalValidity,
}

impl Contradiction {
    pub fn new(claim_a: ContradictionParty, claim_b: ContradictionParty) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            claim_a,
            claim_b,
            resolution: ResolutionStatus::Open,
            explanation: None,
            detected_at: chrono::Utc::now(),
            temporal: TemporalValidity::permanent(),
        }
    }
}

/// One side of a contradiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionParty {
    /// The claim text.
    pub claim: String,
    /// Source of this claim (file, sensor, operator, etc.).
    pub source: String,
    /// Authority level of this source (0.0 = low, 1.0 = high).
    #[serde(default = "default_half")]
    pub authority: Score,
    /// Confidence in this specific claim.
    #[serde(default = "default_half")]
    pub confidence: Score,
    /// When this claim was made.
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional embedding for similarity.
    #[serde(default)]
    pub embedding: Vec<f32>,
    /// Context tags (machine, process, operator, etc.).
    #[serde(default)]
    pub context: Vec<String>,
}

fn default_half() -> f32 {
    0.5
}

impl ContradictionParty {
    pub fn new(claim: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            claim: claim.into(),
            source: source.into(),
            authority: 0.5,
            confidence: 0.5,
            timestamp: chrono::Utc::now(),
            embedding: Vec::new(),
            context: Vec::new(),
        }
    }

    pub fn with_authority(mut self, authority: Score) -> Self {
        self.authority = authority.clamp(0.0, 1.0);
        self
    }

    pub fn with_confidence(mut self, confidence: Score) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}
