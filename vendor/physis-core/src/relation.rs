//! Rich typed graph relations for epistemic, causal, and procedural structures.
//!
//! Relationships connect nodes, hypotheses, observations, events, and processes.
//! Each relationship carries provenance, confidence, temporal validity, and evidence.

use serde::{Deserialize, Serialize};

use crate::models::Score;
use crate::temporal::TemporalValidity;

/// The semantic type of a relationship in the knowledge / context graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    /// Source provides evidential support for target.
    Supports,
    /// Source directly contradicts or refutes target.
    Contradicts,
    /// Source is a causal factor producing target.
    Causes,
    /// Source modulates or influences the likelihood/intensity of target.
    Influences,
    /// Source occurs before target in time or workflow.
    Precedes,
    /// Source occurs after target in time or workflow.
    Follows,
    /// Source depends on target being true or completed.
    DependsOn,
    /// Source requires target as a prerequisite or input.
    Requires,
    /// Source yields or outputs target.
    Produces,
    /// Source is a measurement or metric gauging target.
    Measures,
    /// Source is an empirical observation of target.
    Observes,
    /// Source provides an explanatory model/interpretation for target.
    Explains,
    /// Source forecasts or anticipates target.
    Predicts,
    /// Source invalidates target's validity or assumptions.
    Invalidates,
    /// Source was derived from target via an epistemic or computational step.
    DerivedFrom,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::Supports => "SUPPORTS",
            RelationType::Contradicts => "CONTRADICTS",
            RelationType::Causes => "CAUSES",
            RelationType::Influences => "INFLUENCES",
            RelationType::Precedes => "PRECEDES",
            RelationType::Follows => "FOLLOWS",
            RelationType::DependsOn => "DEPENDS_ON",
            RelationType::Requires => "REQUIRES",
            RelationType::Produces => "PRODUCES",
            RelationType::Measures => "MEASURES",
            RelationType::Observes => "OBSERVES",
            RelationType::Explains => "EXPLAINS",
            RelationType::Predicts => "PREDICTS",
            RelationType::Invalidates => "INVALIDATES",
            RelationType::DerivedFrom => "DERIVED_FROM",
        }
    }

    /// Is this an evidential/epistemic relationship?
    pub fn is_epistemic(&self) -> bool {
        matches!(
            self,
            RelationType::Supports
                | RelationType::Contradicts
                | RelationType::Explains
                | RelationType::Predicts
                | RelationType::Invalidates
                | RelationType::DerivedFrom
        )
    }

    /// Is this a causal/physical relationship?
    pub fn is_causal(&self) -> bool {
        matches!(self, RelationType::Causes | RelationType::Influences)
    }

    /// Is this a temporal/procedural relationship?
    pub fn is_procedural(&self) -> bool {
        matches!(
            self,
            RelationType::Precedes
                | RelationType::Follows
                | RelationType::DependsOn
                | RelationType::Requires
                | RelationType::Produces
                | RelationType::Measures
                | RelationType::Observes
        )
    }
}

/// A typed edge between two entities in Physis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedEdge {
    /// Unique identifier for this relationship edge.
    pub id: String,
    /// Type of relationship.
    pub relation_type: RelationType,
    /// Source node / hypothesis / observation ID.
    pub source_id: String,
    /// Target node / hypothesis / observation ID.
    pub target_id: String,
    /// Confidence in this relationship [0.0, 1.0].
    pub confidence: Score,
    /// Optional strength/weight of the relationship.
    pub weight: Score,
    /// Temporal validity window during which this relationship holds.
    pub temporal: TemporalValidity,
    /// Provenance reference explaining why this edge exists.
    #[serde(default)]
    pub provenance: Option<String>,
    /// Specific text or measurement evidence supporting this relationship.
    #[serde(default)]
    pub evidence: Option<String>,
    /// When this edge was recorded.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl TypedEdge {
    pub fn new(
        relation_type: RelationType,
        source_id: impl Into<String>,
        target_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            relation_type,
            source_id: source_id.into(),
            target_id: target_id.into(),
            confidence: 1.0,
            weight: 1.0,
            temporal: TemporalValidity::permanent(),
            provenance: None,
            evidence: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_confidence(mut self, confidence: Score) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_weight(mut self, weight: Score) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_provenance(mut self, provenance: impl Into<String>) -> Self {
        self.provenance = Some(provenance.into());
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    pub fn with_temporal(mut self, temporal: TemporalValidity) -> Self {
        self.temporal = temporal;
        self
    }
}
