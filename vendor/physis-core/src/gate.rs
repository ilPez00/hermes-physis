//! Verification/promotion gates for semantic-observer candidate claims.
//!
//! Gates turn observer claims into *proposals and decisions*, never into truth.
//! They reuse existing mechanisms: evidence/provenance requirements,
//! [`crate::alignment`] for embedding-vs-structure agreement, contradiction
//! on conflict, [`crate::stability`] robustness/ablation for lessons,
//! [`crate::dream`] recurrence/stability signals, and conservative abstention.
//!
//! The dispatch/assessment helpers ([`assess_claim`], [`assess_claims_with_intake`],
//! [`GateOutcome`]/[`AssessedIntake`]) record append-only intake (what was asserted)
//! and return proposals/decisions without registering hypotheses/edges/contradictions
//! or certifying/merging — callers decide whether to record candidates explicitly.
//!
//! All helpers are deterministic, side-effect-free in that sense, and treat
//! extractor confidence as advisory — promotion requires evidence/consistency,
//! not confidence.

use serde::{Deserialize, Serialize};

use crate::alignment::{self, AlignmentConfig, AlignmentReport, StructureSources};
use crate::contradiction::{Contradiction, ContradictionParty};
use crate::core::PhysisCore;
use crate::dream::{self, StabilityFlag};
use crate::hypothesis::{Hypothesis, HypothesisStatus};
use crate::ingest::{
    self, CandidateClaimPacket, ClaimPayload, ConsolidatedLesson, EquivalenceProposal,
    GroundedConcept, ProposedRelation,
};
use crate::relation::TypedEdge;
use crate::stability::{self, PerturbationOp, PerturbationScenario};

/// Gate outcome (abstention-first, never authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateDecision {
    /// Evidence/consistency supports admitting as a candidate/proposal.
    AcceptCandidate,
    /// Insufficient evidence or ambiguous: do not admit, gather more.
    Defer,
    /// Conflicts with established structure/evidence: reject as stated.
    Reject,
    /// Cannot judge safely (missing structure/evidence) — abstain.
    Abstain,
}

/// Stable, machine-readable reason codes for gate decisions (counts/proxies
/// only, never a verdict). `#[serde(default)]` on result fields keeps old
/// serialized results parseable. Prefer codes over string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateReasonCode {
    MissingProvenance,
    LossyCitation,
    MissingExtractorId,
    RelationEndpointsMissing,
    InvalidEquivalenceEndpoints,
    AdvisoryConfidence,
    AdvisorySimilarity,
    PairAnalysed,
    CandidateEquivalenceAgreement,
    ConflictSurfaced,
    AmbiguousSimilarity,
    EndpointsNotBothKnown,
    NoTestablePrediction,
    TestablePredictionPresent,
    NoCitedEpisodes,
    CitesEpisodes,
    HistoryStabilityFlags,
    ShadowRobustness,
    NoExistingSubjectRobustnessSkipped,
    RelevanceBelowFloor,
    RelevanceAtOrAboveFloor,
    AdvisoryScoreOutOfRange,
    NoObjectId,
    NoSnippet,
    ExtractorConfidenceAdvisory,
    MissingProvenanceAnchor,
    OpaquePayload,
    PayloadKind,
    NoCellMappedType,
    MatchesKnownLabel,
    UnmappedLabel,
    EmptyConcept,
    EmptyLessonStatement,
    Other,
}

/// Assessment of a proposed relation claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationGateResult {
    pub decision: GateDecision,
    pub proposed_edge: TypedEdge,
    pub proposed_hypothesis: Hypothesis,
    pub reasons: Vec<String>,
    /// Stable reason codes (counts/proxies only). `#[serde(default)]` for backcompat.
    #[serde(default)]
    pub reason_codes: Vec<GateReasonCode>,
}

/// Gate a proposed relation: requires provenance/citation and yields a proposed
/// edge+hypothesis (Candidate). Never asserts truth, never commits.
pub fn gate_relation(claim: &CandidateClaimPacket, rel: &ProposedRelation) -> RelationGateResult {
    let (proposed_edge, proposed_hypothesis) = crate::ingest::relation_proposal(claim, rel);
    let mut reasons = Vec::new();
    let mut decision = GateDecision::AcceptCandidate;

    let mut reason_codes = Vec::new();
    if claim.source.uri.is_empty() && claim.raw_ref.is_none() {
        decision = GateDecision::Defer;
        reasons.push("missing provenance (no source uri/raw_ref)".into());
        reason_codes.push(GateReasonCode::MissingProvenance);
    }
    if claim.source.span.is_none() {
        reasons.push("no source span (lossy citation)".into());
        reason_codes.push(GateReasonCode::LossyCitation);
    }
    if rel.subject.trim().is_empty() || rel.object.trim().is_empty() {
        decision = GateDecision::Reject;
        reasons.push("relation endpoints missing".into());
        reason_codes.push(GateReasonCode::RelationEndpointsMissing);
    }
    // Confidence is advisory only.
    reasons.push(format!(
        "advisory_confidence={}",
        claim
            .confidence
            .map(|c| format!("{:.3}", c))
            .unwrap_or_else(|| "n/a".into())
    ));
    reason_codes.push(GateReasonCode::AdvisoryConfidence);
    RelationGateResult {
        decision,
        proposed_edge,
        proposed_hypothesis,
        reasons,
        reason_codes,
    }
}

/// Assessment of an equivalence/merge proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionGateResult {
    pub decision: GateDecision,
    /// The proposed pair (left, right), for traceability/evaluation.
    #[serde(default)]
    pub pair: Option<(String, String)>,
    /// Present when structural/embedding agreement is measurable.
    #[serde(default)]
    pub alignment: Option<AlignmentReport>,
    /// Conflict surfaced as a contradiction (never a merge).
    #[serde(default)]
    pub contradiction: Option<Contradiction>,
    pub reasons: Vec<String>,
    /// Stable reason codes (counts/proxies only). `#[serde(default)]` for backcompat.
    #[serde(default)]
    pub reason_codes: Vec<GateReasonCode>,
}

/// Gate an entity-resolution/equivalence proposal. **Never auto-merges.**
///
/// Uses structural/embedding agreement as a signal when the entities exist as
/// nodes in `core` (via [`alignment::analyse`] over the current
/// graph); otherwise abstains/defers. Conflict yields a contradiction record
/// candidate rather than a merge.
pub fn gate_equivalence(
    core: &PhysisCore,
    claim: &CandidateClaimPacket,
    eq: &EquivalenceProposal,
) -> ResolutionGateResult {
    let mut reasons = Vec::new();
    let mut reason_codes = Vec::new();
    let mut alignment = None;
    let mut contradiction = None;

    if eq.left.trim().is_empty() || eq.right.trim().is_empty() || eq.left == eq.right {
        return ResolutionGateResult {
            decision: GateDecision::Reject,
            pair: Some((eq.left.clone(), eq.right.clone())),
            alignment: None,
            contradiction: None,
            reasons: vec!["invalid equivalence endpoints".into()],
            reason_codes: vec![GateReasonCode::InvalidEquivalenceEndpoints],
        };
    }

    let left_known = core.nodes.contains_key(&eq.left) || core.node_by_label(&eq.left).is_some();
    let right_known = core.nodes.contains_key(&eq.right) || core.node_by_label(&eq.right).is_some();

    // If both resolve to known nodes, probe alignment signal over declared structure.
    let left_id = core
        .node_by_label(&eq.left)
        .map(|n| n.id.clone())
        .unwrap_or_else(|| eq.left.clone());
    let right_id = core
        .node_by_label(&eq.right)
        .map(|n| n.id.clone())
        .unwrap_or_else(|| eq.right.clone());

    let decision = if left_known && right_known {
        let nodes: Vec<_> = core.nodes.values().cloned().collect();
        let cfg = AlignmentConfig {
            k: 5,
            sources: StructureSources::declared_relations(),
            ..AlignmentConfig::default()
        };
        let report = alignment::analyse(&nodes, &core.edges, &cfg);
        // Summarize per-node gaps for the pair (if analysed).
        let lg = report
            .nodes
            .iter()
            .find(|n| n.node_id == left_id)
            .map(|n| n.gap);
        let rg = report
            .nodes
            .iter()
            .find(|n| n.node_id == right_id)
            .map(|n| n.gap);
        reasons.push(format!(
            "pair_analysed left={:?} right={:?} mean_gap={:.3} null_gap={:.3}",
            lg, rg, report.mean_gap, report.null_gap
        ));
        reason_codes.push(GateReasonCode::PairAnalysed);
        alignment = Some(report);

        let sim = eq.similarity.unwrap_or(0.0);
        reasons.push(format!("advisory_similarity={:.3}", sim));
        reason_codes.push(GateReasonCode::AdvisorySimilarity);
        // Conservative: agreement signal + reasonable similarity => candidate link proposal;
        // strong disagreement/conflict => contradiction candidate, not merge.
        if sim >= 0.8 {
            reasons.push(
                "embedding/advisory agreement suggests candidate equivalence (proposal only)"
                    .into(),
            );
            reason_codes.push(GateReasonCode::CandidateEquivalenceAgreement);
            GateDecision::AcceptCandidate
        } else if sim > 0.0 && sim < 0.3 {
            reasons.push("low agreement: conflict surfaced, not a merge".into());
            reason_codes.push(GateReasonCode::ConflictSurfaced);
            let a = ContradictionParty::new(
                format!("{} == {}", eq.left, eq.right),
                format!("claim:{}", claim.id),
            )
            .with_confidence(claim.confidence.unwrap_or(0.4));
            let b = ContradictionParty::new(
                format!("{} != {}", eq.left, eq.right),
                "alignment/structure",
            );
            contradiction = Some(Contradiction::new(a, b));
            GateDecision::Reject
        } else {
            reasons.push("ambiguous/unknown similarity: defer resolution".into());
            reason_codes.push(GateReasonCode::AmbiguousSimilarity);
            GateDecision::Defer
        }
    } else {
        reasons.push(format!(
            "endpoints not both known in core (left_known={}, right_known={})",
            left_known, right_known
        ));
        reason_codes.push(GateReasonCode::EndpointsNotBothKnown);
        GateDecision::Abstain
    };

    ResolutionGateResult {
        decision,
        pair: Some((eq.left.clone(), eq.right.clone())),
        alignment,
        contradiction,
        reasons,
        reason_codes,
    }
}

/// Assessment of a consolidated lesson (policy hypothesis).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonGateResult {
    pub decision: GateDecision,
    pub proposed_hypothesis: Hypothesis,
    /// Stability flags over the subject's history (if any).
    #[serde(default)]
    pub stability_flags: Vec<StabilityFlag>,
    /// Whether shadow robustness showed no degradation in a basic probe.
    #[serde(default)]
    pub robustness_stable: Option<bool>,
    pub reasons: Vec<String>,
    /// Stable reason codes (counts/proxies only). `#[serde(default)]` for backcompat.
    #[serde(default)]
    pub reason_codes: Vec<GateReasonCode>,
}

/// Gate a consolidated lesson before it may influence planning.
///
/// Requires a testable statement, treats the lesson as a Candidate hypothesis
/// (with optional prediction), and probes stability/robustness signals when a
/// matching subject history exists. Never promotes to truth.
pub fn gate_lesson(
    core: &PhysisCore,
    claim: &CandidateClaimPacket,
    lesson: &ConsolidatedLesson,
) -> LessonGateResult {
    let mut reasons = Vec::new();
    let mut reason_codes = Vec::new();
    let mut decision = GateDecision::AcceptCandidate;

    if lesson.statement.trim().is_empty() {
        return LessonGateResult {
            decision: GateDecision::Reject,
            proposed_hypothesis: Hypothesis::new("", vec![]),
            stability_flags: vec![],
            robustness_stable: None,
            reasons: vec!["empty lesson statement".into()],
            reason_codes: vec![GateReasonCode::EmptyLessonStatement],
        };
    }

    let mut proposed = crate::ingest::lesson_proposal(claim, lesson);
    if lesson.prediction.is_none() {
        decision = GateDecision::Defer;
        reasons.push("no testable prediction: defer (lesson must be testable)".into());
        reason_codes.push(GateReasonCode::NoTestablePrediction);
    } else {
        reasons.push("testable prediction present".into());
        reason_codes.push(GateReasonCode::TestablePredictionPresent);
    }
    if lesson.episodes.is_empty() {
        reasons.push("no cited episodes (weak grounding)".into());
        reason_codes.push(GateReasonCode::NoCitedEpisodes);
        if decision == GateDecision::AcceptCandidate {
            decision = GateDecision::Defer;
        }
    } else {
        reasons.push(format!("cites {} episode(s)", lesson.episodes.len()));
        reason_codes.push(GateReasonCode::CitesEpisodes);
    }

    // History stability over any existing hypothesis matching the statement anchor.
    let stability_flags: Vec<StabilityFlag> = core
        .hypotheses
        .values()
        .filter(|h| h.statement == lesson.statement)
        .flat_map(|h| {
            dream::analyse_stability(&core.epistemic_audit)
                .into_iter()
                .filter(|f| f.subject_id == h.id)
                .collect::<Vec<_>>()
        })
        .collect();
    if !stability_flags.is_empty() {
        reasons.push(format!(
            "history stability flags: {}",
            stability_flags.len()
        ));
        decision = GateDecision::Defer;
    }

    // Basic shadow robustness probe: retract this claim's own cited evidence source (no-op
    // if absent) should not degrade unrelated state; we report stability as "unknown" when
    // the hypothesis is not yet in core (it is a proposal).
    let mut robustness_stable = None;
    if let Some(existing_id) = core
        .hypotheses
        .values()
        .find(|h| h.statement == lesson.statement)
        .map(|h| h.id.clone())
    {
        let nodes: Vec<_> = core.nodes.values().cloned().collect();
        let hyps: Vec<_> = core.hypotheses.values().cloned().collect();
        let sc = PerturbationScenario::new("lesson-evidence-ablation").with_op(
            PerturbationOp::RetractEvidence {
                hypothesis_id: existing_id.clone(),
                source: format!("claim:{}", claim.id),
            },
        );
        let report =
            stability::assess_robustness(&nodes, &hyps, &core.edges, &[sc], Some(&[existing_id]));
        let stable = report.all_stable();
        robustness_stable = Some(stable);
        reasons.push(format!("shadow robustness_stable={}", stable));
        reason_codes.push(GateReasonCode::ShadowRobustness);
        if !stable {
            decision = GateDecision::Defer;
        }
    } else {
        reasons
            .push("no existing subject in core: robustness probe skipped (proposal-only)".into());
        reason_codes.push(GateReasonCode::NoExistingSubjectRobustnessSkipped);
    }

    // Ensure it stays a candidate (never auto-promoted).
    proposed.status = HypothesisStatus::Candidate;

    LessonGateResult {
        decision,
        proposed_hypothesis: proposed,
        stability_flags,
        robustness_stable,
        reasons,
        reason_codes,
    }
}

/// Generic abstention-first acceptance summary over any claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimGateSummary {
    pub decision: GateDecision,
    pub reasons: Vec<String>,
    /// Stable reason codes (counts/proxies only). `#[serde(default)]` for backcompat.
    #[serde(default)]
    pub reason_codes: Vec<GateReasonCode>,
}

/// Assessment of an observer retrieval-hit claim for context fusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalHitGateResult {
    pub decision: GateDecision,
    /// Query↔snippet cosine similarity when measurable, else `None`.
    pub relevance: Option<f32>,
    pub reasons: Vec<String>,
    /// Stable reason codes (counts/proxies only). `#[serde(default)]` for backcompat.
    #[serde(default)]
    pub reason_codes: Vec<GateReasonCode>,
}

/// Gate an observer retrieval-hit claim against the query before fusion.
///
/// Deterministic and abstention-first: provenance and attribution are checked
/// first (defer/reject with named causes), then the query↔snippet cosine
/// similarity is compared against `min_score` — below the floor defers (the
/// observer may know something, but this packet is not the place for it).
/// The embeddings are supplied by the caller (the gate stays pure); the
/// advisory score is recorded, never trusted: it cannot raise the decision.
/// The floor's meaning is embedder-dependent: with a semantic embedder it
/// separates topical relevance; with a lexical or random-projection embedder
/// it separates only coarse token overlap (same-language text clusters high).
/// Nothing here asserts truth — acceptance only means "may enter the packet
/// as an attributed, unverified candidate".
pub fn gate_retrieval_hit(
    query_emb: &[f32],
    snippet_emb: &[f32],
    claim: &CandidateClaimPacket,
    hit: &crate::ingest::RetrievalHit,
    min_score: f32,
) -> RetrievalHitGateResult {
    let mut reasons = Vec::new();
    if claim.extractor.id.trim().is_empty() {
        return RetrievalHitGateResult {
            decision: GateDecision::Defer,
            relevance: None,
            reasons: vec!["missing extractor id".into()],
            reason_codes: vec![GateReasonCode::MissingExtractorId],
        };
    }
    if claim.source.uri.is_empty() && claim.raw_ref.is_none() {
        return RetrievalHitGateResult {
            decision: GateDecision::Defer,
            relevance: None,
            reasons: vec!["missing provenance anchor".into()],
            reason_codes: vec![GateReasonCode::MissingProvenanceAnchor],
        };
    }
    if hit.object_id.trim().is_empty() {
        return RetrievalHitGateResult {
            decision: GateDecision::Reject,
            relevance: None,
            reasons: vec!["retrieval hit has no object id (cannot attribute)".into()],
            reason_codes: vec![GateReasonCode::NoObjectId],
        };
    }
    if hit.score < 0.0 || hit.score > 1.0 {
        return RetrievalHitGateResult {
            decision: GateDecision::Reject,
            relevance: None,
            reasons: vec![format!("advisory score out of range: {}", hit.score)],
            reason_codes: vec![GateReasonCode::AdvisoryScoreOutOfRange],
        };
    }
    let snippet = hit.snippet.as_deref().unwrap_or("").trim();
    if snippet.is_empty() {
        return RetrievalHitGateResult {
            decision: GateDecision::Defer,
            relevance: None,
            reasons: vec!["no snippet to check relevance against".into()],
            reason_codes: vec![GateReasonCode::NoSnippet],
        };
    }
    let relevance = crate::models::cosine_sim(query_emb, snippet_emb);
    let mut reason_codes = Vec::new();
    if relevance < min_score {
        reasons.push(format!(
            "relevance {relevance:.3} below floor {min_score:.3}"
        ));
        reason_codes.push(GateReasonCode::RelevanceBelowFloor);
        reasons.push(
            "deferred: the observer may know something, this packet is not the place for it".into(),
        );
        return RetrievalHitGateResult {
            decision: GateDecision::Defer,
            relevance: Some(relevance),
            reasons,
            reason_codes,
        };
    }
    reasons.push(format!("relevance={relevance:.3} >= floor {min_score:.3}"));
    reason_codes.push(GateReasonCode::RelevanceAtOrAboveFloor);
    reasons.push("extractor confidence is advisory, not authority".into());
    reason_codes.push(GateReasonCode::ExtractorConfidenceAdvisory);
    RetrievalHitGateResult {
        decision: GateDecision::AcceptCandidate,
        relevance: Some(relevance),
        reasons,
        reason_codes,
    }
}

/// Conservative top-level gate: a claim with no payload support/provenance is
/// deferred/abstained; confidence alone never accepts.
pub fn gate_claim(claim: &CandidateClaimPacket) -> ClaimGateSummary {
    let mut reasons = Vec::new();
    let mut reason_codes = Vec::new();
    let mut decision = GateDecision::AcceptCandidate;
    if claim.extractor.id.trim().is_empty() {
        decision = GateDecision::Defer;
        reasons.push("missing extractor id".into());
        reason_codes.push(GateReasonCode::MissingExtractorId);
    }
    if claim.source.uri.is_empty() && claim.raw_ref.is_none() {
        decision = GateDecision::Defer;
        reasons.push("missing provenance anchor".into());
        reason_codes.push(GateReasonCode::MissingProvenanceAnchor);
    }
    match &claim.payload {
        ClaimPayload::Opaque(_) => {
            reasons.push("opaque payload: requires explicit mapping".into());
            reason_codes.push(GateReasonCode::OpaquePayload);
            if decision == GateDecision::AcceptCandidate {
                decision = GateDecision::Defer;
            }
        }
        _ => {
            reasons.push(format!("payload_kind={}", claim.payload.kind_label()));
            reason_codes.push(GateReasonCode::PayloadKind);
        }
    }
    reasons.push("extractor confidence is advisory, not authority".into());
    reason_codes.push(GateReasonCode::ExtractorConfidenceAdvisory);
    ClaimGateSummary {
        decision,
        reasons,
        reason_codes,
    }
}

/// Assessment of an ontology-grounding claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingGateResult {
    pub decision: GateDecision,
    /// Proposed hypothesis describing the grounding (candidate, never certified).
    pub proposed_hypothesis: Hypothesis,
    pub reasons: Vec<String>,
    /// Stable reason codes (counts/proxies only). `#[serde(default)]` for backcompat.
    #[serde(default)]
    pub reason_codes: Vec<GateReasonCode>,
}

/// Assess a grounded-concept claim against known labels/cells.
///
/// Deterministic, label-based (no embedding, no network): if the proposed
/// cell/mapped_type matches a known label, accept as a candidate grounding
/// proposal; if unmapped, defer as a discovery/coverage candidate. Never
/// certifies the mapping and never auto-applies it.
pub fn assess_grounding(
    claim: &CandidateClaimPacket,
    grounded: &GroundedConcept,
    known_labels: &[String],
) -> GroundingGateResult {
    let mut reasons = Vec::new();
    let stmt = format!(
        "ground concept '{}' as {}{}",
        grounded.concept,
        grounded.cell.as_deref().unwrap_or("-"),
        grounded
            .mapped_type
            .as_ref()
            .map(|t| format!(" ({})", t))
            .unwrap_or_default()
    );
    let proposed_hypothesis = ingest::hypothesis_from_claim(claim, stmt, Vec::new());

    if grounded.concept.trim().is_empty() {
        return GroundingGateResult {
            decision: GateDecision::Reject,
            proposed_hypothesis,
            reasons: vec!["empty concept".into()],
            reason_codes: vec![GateReasonCode::EmptyConcept],
        };
    }
    let target = grounded
        .cell
        .clone()
        .or_else(|| grounded.mapped_type.clone());
    let Some(t) = target else {
        reasons.push("no cell/mapped_type proposed: unmapped discovery candidate".into());
        return GroundingGateResult {
            decision: GateDecision::Defer,
            proposed_hypothesis,
            reasons,
            reason_codes: vec![GateReasonCode::NoCellMappedType],
        };
    };
    let t_norm = t.to_lowercase();
    let matched = known_labels.iter().any(|k| k.to_lowercase() == t_norm);
    if matched {
        reasons.push(format!("matches known label/cell '{}'", t));
        GroundingGateResult {
            decision: GateDecision::AcceptCandidate,
            proposed_hypothesis,
            reasons,
            reason_codes: vec![GateReasonCode::MatchesKnownLabel],
        }
    } else {
        reasons.push(format!(
            "unmapped label/cell '{}': route to discovery/coverage as candidate",
            t
        ));
        GroundingGateResult {
            decision: GateDecision::Defer,
            proposed_hypothesis,
            reasons,
            reason_codes: vec![GateReasonCode::UnmappedLabel],
        }
    }
}

// ── Dispatch / batch assessment (intake + proposals, no registration) ───────

/// Typed outcome of gating one candidate claim (decision/proposal only).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "gate", rename_all = "snake_case")]
pub enum GateOutcome {
    Relation(RelationGateResult),
    Equivalence(ResolutionGateResult),
    Lesson(LessonGateResult),
    Grounding(GroundingGateResult),
    Claim(ClaimGateSummary),
}

impl GateOutcome {
    /// The gate decision for this outcome.
    pub fn decision(&self) -> GateDecision {
        match self {
            GateOutcome::Relation(r) => r.decision,
            GateOutcome::Equivalence(r) => r.decision,
            GateOutcome::Lesson(r) => r.decision,
            GateOutcome::Grounding(r) => r.decision,
            GateOutcome::Claim(r) => r.decision,
        }
    }
}

/// Assess a single candidate claim against current core/ontology context.
///
/// Returns a typed gate outcome with proposals/decisions but registers nothing
/// (no hypotheses, no edges, no contradictions, no merges). Confidence is
/// advisory; acceptance is candidate-only where applicable. `known_labels` is
/// used for grounding assessment (pass an empty slice if none).
pub fn assess_claim(
    core: &PhysisCore,
    claim: &CandidateClaimPacket,
    known_labels: &[String],
) -> GateOutcome {
    match &claim.payload {
        ClaimPayload::Relation(rel) => GateOutcome::Relation(gate_relation(claim, rel)),
        ClaimPayload::Equivalence(eq) => {
            GateOutcome::Equivalence(gate_equivalence(core, claim, eq))
        }
        ClaimPayload::Lesson(lesson) => GateOutcome::Lesson(gate_lesson(core, claim, lesson)),
        ClaimPayload::Grounding(g) => {
            GateOutcome::Grounding(assess_grounding(claim, g, known_labels))
        }
        // Entity / Retrieval / Opaque: conservative top-level gate (candidate/defer).
        _ => GateOutcome::Claim(gate_claim(claim)),
    }
}

/// Result of recording intake and assessing one claim (no registration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessedIntake {
    pub receipt: crate::epistemic::IntakeReceipt,
    pub outcome: GateOutcome,
}

impl AssessedIntake {
    /// The gate decision for this assessed claim.
    pub fn decision(&self) -> GateDecision {
        self.outcome.decision()
    }
}

/// Append-only intake plus gate assessment for a batch of observer claims.
///
/// For each claim (in order): records what the observer asserted as an intake
/// observation/audit event, then computes the gate outcome/proposals. This never
/// registers hypotheses, edges, or contradictions and never certifies truth or
/// merges identities — callers may inspect outcomes and decide whether to record
/// candidates explicitly.
pub fn assess_claims_with_intake(
    core: &mut PhysisCore,
    claims: &[CandidateClaimPacket],
    known_labels: &[String],
    stream_id: &str,
    arrival: chrono::DateTime<chrono::Utc>,
) -> Vec<AssessedIntake> {
    claims
        .iter()
        .map(|claim| {
            let receipt = crate::ingest::record_candidate_intake(core, claim, stream_id, arrival);
            let outcome = assess_claim(core, claim, known_labels);
            AssessedIntake { receipt, outcome }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PhysisCore;
    use crate::embed::VectorEmbed;
    use crate::ingest::{
        ClaimPayload, ClaimSource, ConsolidatedLesson, EntityMention, EquivalenceProposal,
        ExtractorInfo, ProposedRelation, RetrievalHit,
    };
    use crate::relation::RelationType;

    #[test]
    fn a_relevant_well_provenanced_hit_is_accepted_with_its_relevance_recorded() {
        let embedder = crate::RandomProjectionEmbedder::new(64);
        let hit = RetrievalHit {
            object_id: "obj:relay".into(),
            score: 0.9,
            strategy: Some("vec".into()),
            snippet: Some("relay calibration requires the bench harness".into()),
        };
        let claim = base_claim(ClaimPayload::Retrieval(hit.clone()));
        let q = embedder.embed("relay calibration bench harness");
        let s = embedder.embed(hit.snippet.as_deref().unwrap());
        let r = gate_retrieval_hit(&q, &s, &claim, &hit, 0.9);
        assert_eq!(r.decision, GateDecision::AcceptCandidate);
        let rel = r.relevance.unwrap();
        assert!((0.9..1.0).contains(&rel), "relevance was {rel}");
        assert!(r.reasons.iter().any(|x| x.contains("advisory")));
    }

    #[test]
    fn an_irrelevant_hit_defers_with_the_floor_named() {
        let embedder = crate::RandomProjectionEmbedder::new(64);
        let hit = RetrievalHit {
            object_id: "obj:menu".into(),
            score: 0.9,
            strategy: None,
            snippet: Some("the cafeteria lunch menu rotates daily".into()),
        };
        let claim = base_claim(ClaimPayload::Retrieval(hit.clone()));
        let q = embedder.embed("relay calibration bench harness");
        let s = embedder.embed(hit.snippet.as_deref().unwrap());
        // Measured with this embedder: the relay query scores 0.934 against a
        // relevant snippet and 0.892 against this unrelated one — a floor of
        // 0.9 separates them. The floor's meaning is embedder-dependent; with
        // a non-semantic embedder only coarse separation is possible.
        let r = gate_retrieval_hit(&q, &s, &claim, &hit, 0.9);
        assert_eq!(r.decision, GateDecision::Defer);
        let rel = r.relevance.unwrap();
        assert!((rel - 0.892).abs() < 0.01, "relevance was {rel}");
        assert!(r
            .reasons
            .iter()
            .any(|x| x.contains("below floor") && x.contains("0.900")));
    }

    #[test]
    fn malformed_hits_reject_with_named_causes() {
        let embedder = crate::RandomProjectionEmbedder::new(64);
        let q = embedder.embed("relay calibration");
        let s = embedder.embed("some snippet");

        let no_id = RetrievalHit {
            object_id: "  ".into(),
            score: 0.5,
            strategy: None,
            snippet: Some("some snippet".into()),
        };
        let r = gate_retrieval_hit(
            &q,
            &s,
            &base_claim(ClaimPayload::Retrieval(no_id.clone())),
            &no_id,
            0.1,
        );
        assert_eq!(r.decision, GateDecision::Reject);
        assert!(r.reasons.iter().any(|x| x.contains("no object id")));

        let bad_score = RetrievalHit {
            object_id: "obj:x".into(),
            score: 1.5,
            strategy: None,
            snippet: Some("some snippet".into()),
        };
        let r = gate_retrieval_hit(
            &q,
            &s,
            &base_claim(ClaimPayload::Retrieval(bad_score.clone())),
            &bad_score,
            0.1,
        );
        assert_eq!(r.decision, GateDecision::Reject);
        assert!(r.reasons.iter().any(|x| x.contains("out of range")));

        let no_snippet = RetrievalHit {
            object_id: "obj:x".into(),
            score: 0.5,
            strategy: None,
            snippet: None,
        };
        let r = gate_retrieval_hit(
            &q,
            &s,
            &base_claim(ClaimPayload::Retrieval(no_snippet.clone())),
            &no_snippet,
            0.1,
        );
        assert_eq!(r.decision, GateDecision::Defer);
        assert!(r.relevance.is_none());
        assert!(r.reasons.iter().any(|x| x.contains("no snippet")));
    }

    fn base_claim(payload: ClaimPayload) -> CandidateClaimPacket {
        CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i1".into()),
            source: ClaimSource {
                uri: "doc://s".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("obs"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(0.6),
            payload,
            lossy_fields: vec![],
            extras: Default::default(),
        }
        .normalize()
    }

    #[test]
    fn relation_gate_requires_provenance_and_yields_candidate() {
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let mut claim = base_claim(ClaimPayload::Relation(rel.clone()));
        let ok = gate_relation(&claim, &rel);
        assert_eq!(ok.decision, GateDecision::AcceptCandidate);
        assert_eq!(ok.proposed_hypothesis.status, HypothesisStatus::Candidate);

        claim.source.uri.clear();
        claim.raw_ref = None;
        let bad = gate_relation(&claim, &rel);
        assert_eq!(bad.decision, GateDecision::Defer);
    }

    #[test]
    fn equivalence_gate_never_merges_and_abstains_when_unknown() {
        let core = PhysisCore::new();
        let claim = base_claim(ClaimPayload::Entity(EntityMention {
            label: "x".into(),
            refers_to: None,
            concept: None,
        }));
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.9),
            rationale: None,
        };
        let res = gate_equivalence(&core, &claim, &eq);
        assert_eq!(res.decision, GateDecision::Abstain);
        assert_eq!(res.pair, Some(("n1".to_string(), "n2".to_string())));
        assert!(res.contradiction.is_none());
    }

    #[test]
    fn lesson_gate_defers_without_prediction() {
        let core = PhysisCore::new();
        let lesson = ConsolidatedLesson {
            statement: "cache helps".into(),
            prediction: None,
            episodes: vec!["e".into()],
        };
        let claim = base_claim(ClaimPayload::Lesson(lesson.clone()));
        let res = gate_lesson(&core, &claim, &lesson);
        assert_eq!(res.decision, GateDecision::Defer);
        assert_eq!(res.proposed_hypothesis.status, HypothesisStatus::Candidate);
    }

    #[test]
    fn grounding_assessment_is_deterministic_and_never_certifies() {
        use crate::ingest::GroundedConcept;
        let claim = base_claim(ClaimPayload::Grounding(GroundedConcept {
            concept: "cache".into(),
            cell: Some("MEMORY".into()),
            mapped_type: None,
        }));
        let known = vec!["MEMORY".to_string(), "IO".to_string()];
        let ok = assess_grounding(
            &claim,
            &GroundedConcept {
                concept: "cache".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            },
            &known,
        );
        assert_eq!(ok.decision, GateDecision::AcceptCandidate);
        assert_eq!(ok.proposed_hypothesis.status, HypothesisStatus::Candidate);

        let unk = assess_grounding(
            &claim,
            &GroundedConcept {
                concept: "cache".into(),
                cell: Some("NEWCELL".into()),
                mapped_type: None,
            },
            &known,
        );
        assert_eq!(unk.decision, GateDecision::Defer);
        assert!(unk.reasons.iter().any(|r| r.contains("unmapped")));
    }

    #[test]
    fn assess_claims_with_intake_records_without_registering() {
        let mut core = PhysisCore::new();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let claims = vec![base_claim(ClaimPayload::Relation(rel))];
        let before_h = core.hypotheses.len();
        let before_e = core.edges.len();
        let before_ev = core.epistemic_audit.events.len();
        let assessed =
            assess_claims_with_intake(&mut core, &claims, &[], "obs", chrono::Utc::now());
        assert_eq!(assessed.len(), 1);
        assert!(matches!(assessed[0].outcome, GateOutcome::Relation(_)));
        assert_eq!(core.hypotheses.len(), before_h);
        assert_eq!(core.edges.len(), before_e);
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 1);
        assert_eq!(
            assessed[0].outcome.decision(),
            GateDecision::AcceptCandidate
        );
        if let GateOutcome::Relation(r) = &assessed[0].outcome {
            assert_eq!(r.decision, GateDecision::AcceptCandidate);
        }
        // Typed outcome is serializable for adapter/trace interchange.
        let j = serde_json::to_value(&assessed[0].outcome).unwrap();
        assert!(j.get("gate").is_some());
    }
}
