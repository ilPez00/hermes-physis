//! Semantic-observer ingest contract (candidate claims, never authority).
//!
//! External observers (e.g. Cognee sidecar/RS, extractors, consolidators) may
//! discover entities, relations, resolutions, grounded concepts, lessons, or
//! retrieval hits. **Every emission is a candidate claim with provenance** —
//! never a verified fact, never a canonical edge, never a merge by fiat.
//!
//! This module defines a uniform, deterministic, append-only-friendly envelope
//! ([`CandidateClaimPacket`]) plus normalization/conversion helpers, and a
//! versioned lossy-explicit interchange container ([`CandidateClaimEnvelope`])
//! for adapter/sidecar mapping. Mapping is intentionally loss-aware: fields an
//! observer provides that have no first-class home are recorded in
//! [`CandidateClaimPacket::lossy_fields`] so gaps are visible, not silent.
//! Invariants:
//!
//! - observer output can create/strengthen candidates and cite evidence, but
//!   cannot assert verified truth, delete history, or bypass TMS/contradictions;
//! - every accepted influence must be replayable to its intake+evidence;
//! - promotion requires evidence/consistency, not extractor confidence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::hypothesis::{Evidence, Hypothesis, HypothesisStatus};
use crate::models::Score;
use crate::observe::Observation;
use crate::provenance::ProvenanceLink;
use crate::relation::{RelationType, TypedEdge};

/// Stable source/provenance reference for a claim (span-level when known).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimSource {
    /// URI/path/subject of the source document or episode.
    #[serde(default)]
    pub uri: String,
    /// Opaque span/offset reference (e.g. "12..48", "line:3", selector). Empty
    /// when the extractor cannot cite a span (recorded as lossy by callers).
    #[serde(default)]
    pub span: Option<String>,
    /// Source kind hint (doc/code/trace/conversation/web/sensor/episode).
    #[serde(default)]
    pub kind: Option<String>,
}

/// Identity and configuration of the producing observer/extractor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractorInfo {
    /// Observer/extractor id (e.g. "cognee:gliner", "cognee:llm", "mock").
    #[serde(default)]
    pub id: String,
    /// Version string of the extractor/model/pipeline.
    #[serde(default)]
    pub version: Option<String>,
    /// Hash/id of the configuration used (for replay/drift tracking).
    #[serde(default)]
    pub config_hash: Option<String>,
}

impl ExtractorInfo {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: None,
            config_hash: None,
        }
    }
}

/// A mentioned/resolved entity candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMention {
    /// Proposed label/name.
    pub label: String,
    /// Optional stable/known id this mention refers to (if pre-resolved).
    #[serde(default)]
    pub refers_to: Option<String>,
    /// Optional concept/type grounding hint.
    #[serde(default)]
    pub concept: Option<String>,
}

/// A proposed relation/fact between two entity references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposedRelation {
    pub subject: String,
    pub predicate: String,
    /// Best-effort mapping to a core relation type, if grounded.
    #[serde(default)]
    pub relation_type: Option<RelationType>,
    pub object: String,
    /// Free-text fact/claim as asserted by the observer.
    #[serde(default)]
    pub fact: Option<String>,
}

/// A proposed equivalence/merge between mentions/entities (never auto-merged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquivalenceProposal {
    pub left: String,
    pub right: String,
    /// Advisory similarity/confidence from the observer.
    #[serde(default)]
    pub similarity: Option<Score>,
    #[serde(default)]
    pub rationale: Option<String>,
}

/// Grounding of a concept/label onto a Physis cell/type (explicit, versioned).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundedConcept {
    pub concept: String,
    /// Proposed ontology/cell key (DOMAIN×MODE or label).
    #[serde(default)]
    pub cell: Option<String>,
    /// Proposed/observed type or predicate mapping.
    #[serde(default)]
    pub mapped_type: Option<String>,
}

/// A consolidated operational lesson/policy hypothesis (must be testable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidatedLesson {
    pub statement: String,
    /// Testable expectation/prediction if the lesson holds.
    #[serde(default)]
    pub prediction: Option<String>,
    /// Episode/trace ids this lesson cites.
    #[serde(default)]
    pub episodes: Vec<String>,
}

/// A retrieval hit contributed by an observer (candidate, ranked by it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalHit {
    pub object_id: String,
    /// Observer-assigned score (advisory).
    #[serde(default)]
    pub score: Score,
    /// Strategy/reason label from the observer.
    #[serde(default)]
    pub strategy: Option<String>,
    /// Snippet/text the observer surfaced.
    #[serde(default)]
    pub snippet: Option<String>,
}

/// Typed payload of a candidate claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ClaimPayload {
    Entity(EntityMention),
    Relation(ProposedRelation),
    Equivalence(EquivalenceProposal),
    Grounding(GroundedConcept),
    Lesson(ConsolidatedLesson),
    Retrieval(RetrievalHit),
    /// Fallback for observer-specific payloads (kept loss-aware).
    Opaque(serde_json::Value),
}

impl ClaimPayload {
    pub fn kind_label(&self) -> &'static str {
        match self {
            ClaimPayload::Entity(_) => "entity",
            ClaimPayload::Relation(_) => "relation",
            ClaimPayload::Equivalence(_) => "equivalence",
            ClaimPayload::Grounding(_) => "grounding",
            ClaimPayload::Lesson(_) => "lesson",
            ClaimPayload::Retrieval(_) => "retrieval",
            ClaimPayload::Opaque(_) => "opaque",
        }
    }
}

/// Uniform, provenance-carrying candidate claim emitted by a semantic observer.
///
/// Confidence is advisory (extractor belief), never authority. Packets are
/// normalizable and replayable given the same inputs/versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateClaimPacket {
    /// Stable claim id (deterministic over normalized content when derived).
    pub id: String,
    /// Intake/episode/batch id grouping related claims.
    #[serde(default)]
    pub intake_id: Option<String>,
    /// Source/provenance reference.
    #[serde(default)]
    pub source: ClaimSource,
    /// Producing observer/extractor.
    #[serde(default)]
    pub extractor: ExtractorInfo,
    /// When the source asserted/produced the fact (episode/reference time).
    #[serde(default)]
    pub asserted_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the observer observed/ingested it.
    #[serde(default)]
    pub observed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Hash/ref of the raw content this claim was derived from.
    #[serde(default)]
    pub raw_ref: Option<String>,
    /// Advisory extractor confidence in [0,1] (not corroboration, not truth).
    #[serde(default)]
    pub confidence: Option<Score>,
    /// The candidate payload.
    pub payload: ClaimPayload,
    /// Fields the observer provided that could not be mapped losslessly.
    #[serde(default)]
    pub lossy_fields: Vec<String>,
    /// Optional opaque extras (COGX pass-through fragments, etc.).
    #[serde(default)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

impl CandidateClaimPacket {
    /// Deterministic id derived from the claim's normalized, provenance-bearing
    /// content (stable for replay when inputs/versions match).
    pub fn derive_id(
        extractor: &ExtractorInfo,
        source: &ClaimSource,
        intake_id: Option<&str>,
        payload: &ClaimPayload,
        raw_ref: Option<&str>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(extractor.id.as_bytes());
        h.update(b"\0");
        h.update(extractor.version.as_deref().unwrap_or("").as_bytes());
        h.update(b"\0");
        h.update(source.uri.as_bytes());
        h.update(b"\0");
        h.update(source.span.as_deref().unwrap_or("").as_bytes());
        h.update(b"\0");
        h.update(intake_id.unwrap_or("").as_bytes());
        h.update(b"\0");
        h.update(raw_ref.unwrap_or("").as_bytes());
        h.update(b"\0");
        // Payload serialized canonically enough for a stable fingerprint.
        if let Ok(bytes) = serde_json::to_vec(payload) {
            h.update(&bytes);
        }
        let digest = h.finalize();
        format!("claim:{:x}", digest)
    }

    /// Ensure id is deterministic over normalized content and record obvious
    /// loss (e.g. missing span) without mutating payload meaning.
    pub fn normalize(mut self) -> Self {
        if self.id.trim().is_empty() || self.id == "claim:auto" {
            self.id = Self::derive_id(
                &self.extractor,
                &self.source,
                self.intake_id.as_deref(),
                &self.payload,
                self.raw_ref.as_deref(),
            );
        }
        if self.source.span.as_deref().unwrap_or("").is_empty()
            && !self.lossy_fields.iter().any(|f| f == "source.span")
        {
            self.lossy_fields.push("source.span".to_string());
        }
        self.lossy_fields.sort();
        self.lossy_fields.dedup();
        self
    }

    /// Append-only intake observation for this claim (source records the
    /// observer; body references the claim). The observation is a record of
    /// what the observer asserted, not a verified fact.
    pub fn to_observation(&self) -> Observation {
        let subj = if self.source.uri.is_empty() {
            self.id.clone()
        } else {
            self.source.uri.clone()
        };
        let mut o = Observation::new(format!("observer:{}", self.extractor.id), subj)
            .by(self.extractor.id.clone())
            .with_body(format!(
                "candidate {} claim={} intake={} raw={}",
                self.payload.kind_label(),
                self.id,
                self.intake_id.as_deref().unwrap_or("-"),
                self.raw_ref.as_deref().unwrap_or("-"),
            ));
        if let Some(at) = self.asserted_at.or(self.observed_at) {
            o.at = at;
        }
        o
    }

    /// Provenance link citing this claim's intake/source.
    pub fn provenance_link(&self, claim_text: impl Into<String>) -> ProvenanceLink {
        let mut link = ProvenanceLink::new(
            claim_text,
            if self.source.uri.is_empty() {
                self.extractor.id.clone()
            } else {
                self.source.uri.clone()
            },
        );
        if let Some(at) = self.asserted_at.or(self.observed_at) {
            link.timestamp = at;
        }
        if let Some(intake) = &self.intake_id {
            link = link.with_intake_id(intake.clone());
        }
        if let Some(raw) = &self.raw_ref {
            link.raw_reference = Some(raw.clone());
        }
        link
    }

    /// Conservative evidence stub derived from this claim (advisory weight,
    /// intake-cited). Callers choose polarity/support context.
    pub fn evidence(
        &self,
        source_label: impl Into<String>,
        claim_text: impl Into<String>,
    ) -> Evidence {
        let mut ev = Evidence::supports(source_label, claim_text)
            .with_weight(self.confidence.unwrap_or(0.5).clamp(0.0, 1.0));
        if let Some(intake) = &self.intake_id {
            ev = ev.with_intake_id(intake.clone());
        }
        if let Some(at) = self.asserted_at.or(self.observed_at) {
            ev.observed_at = Some(at);
        }
        ev
    }
}

/// Build a refutable candidate hypothesis from a claim (Candidate, never
/// certified/promoted-to-truth). The claim's intake is cited as evidence but
/// the hypothesis remains `Candidate` (provenance is not corroboration).
pub fn hypothesis_from_claim(
    claim: &CandidateClaimPacket,
    statement: impl Into<String>,
    embedding: Vec<f32>,
) -> Hypothesis {
    let stmt = statement.into();
    let mut h = Hypothesis::new(stmt.clone(), embedding);
    let ev = claim.evidence(format!("claim:{}", claim.id), stmt);
    h.add_supporting_evidence(ev);
    // Reset standing: the claim cites an observation, it does not corroborate.
    h.status = HypothesisStatus::Candidate;
    h.recompute_fitness();
    h
}

/// For a proposed relation claim, produce a *proposed* typed edge (caller
/// decides whether/when to admit it) and a candidate hypothesis describing it.
pub fn relation_proposal(
    claim: &CandidateClaimPacket,
    rel: &ProposedRelation,
) -> (TypedEdge, Hypothesis) {
    let rtype =
        rel.relation_type
            .unwrap_or_else(|| match rel.predicate.to_ascii_lowercase().as_str() {
                "supports" => RelationType::Supports,
                "contradicts" | "refutes" => RelationType::Contradicts,
                "causes" => RelationType::Causes,
                "influences" => RelationType::Influences,
                "precedes" | "before" => RelationType::Precedes,
                "follows" | "after" => RelationType::Follows,
                "depends_on" | "depends-on" | "depends" => RelationType::DependsOn,
                "requires" => RelationType::Requires,
                "produces" => RelationType::Produces,
                "measures" => RelationType::Measures,
                "observes" => RelationType::Observes,
                "explains" => RelationType::Explains,
                "predicts" => RelationType::Predicts,
                "invalidates" => RelationType::Invalidates,
                "derived_from" | "derived-from" => RelationType::DerivedFrom,
                _ => RelationType::Influences,
            });
    let fact = rel
        .fact
        .clone()
        .unwrap_or_else(|| format!("{} {} {}", rel.subject, rel.predicate, rel.object));
    let edge = TypedEdge::new(rtype, rel.subject.clone(), rel.object.clone())
        .with_confidence(claim.confidence.unwrap_or(0.5).clamp(0.0, 1.0))
        .with_provenance(format!("claim:{} source:{}", claim.id, claim.source.uri))
        .with_evidence(fact.clone());
    let h = hypothesis_from_claim(claim, fact, Vec::new());
    (edge, h)
}

/// For a consolidated lesson claim, produce a candidate (policy) hypothesis,
/// optionally attaching the testable prediction. Remains Candidate; promotion
/// requires evidence/consistency/stability checks by the caller.
pub fn lesson_proposal(claim: &CandidateClaimPacket, lesson: &ConsolidatedLesson) -> Hypothesis {
    let mut h = hypothesis_from_claim(claim, lesson.statement.clone(), Vec::new());
    if let Some(pred) = &lesson.prediction {
        h.add_prediction(crate::hypothesis::Prediction::new(pred.clone()));
    }
    h
}

/// Record an observer claim as an append-only intake event.
///
/// This records *what the observer asserted* (an observation), never verified
/// truth: it advances the per-stream watermark and emits an
/// `ObservationIngested` audit event citing the intake/assertion. It does not
/// register hypotheses, edges, or contradictions — use the gate helpers to
/// derive proposals and decide whether to record them as candidates.
pub fn record_candidate_intake(
    core: &mut crate::core::PhysisCore,
    claim: &CandidateClaimPacket,
    stream_id: &str,
    arrival: chrono::DateTime<chrono::Utc>,
) -> crate::epistemic::IntakeReceipt {
    let episode = claim.asserted_at.or(claim.observed_at);
    let receipt = core
        .epistemic_audit
        .note_intake(stream_id.to_string(), arrival, episode);
    let source_uri = if claim.source.uri.is_empty() {
        "-"
    } else {
        claim.source.uri.as_str()
    };
    let mut ev = crate::epistemic::EpistemicEvent::new(
        crate::epistemic::EpistemicEventType::ObservationIngested,
        claim.id.clone(),
        format!(
            "observer {} ingested {} claim (source={})",
            claim.extractor.id,
            claim.payload.kind_label(),
            source_uri
        ),
    )
    .with_source(claim.extractor.id.clone());
    ev.intake_id = claim.intake_id.clone();
    ev.asserted_at = claim.asserted_at;
    core.record_epistemic_event(ev);
    receipt
}

/// Record a batch of claims as append-only intake events (one per claim) under
/// the same stream. Returns receipts in claim order.
pub fn record_candidate_intake_batch(
    core: &mut crate::core::PhysisCore,
    claims: &[CandidateClaimPacket],
    stream_id: &str,
    arrival: chrono::DateTime<chrono::Utc>,
) -> Vec<crate::epistemic::IntakeReceipt> {
    claims
        .iter()
        .map(|c| record_candidate_intake(core, c, stream_id, arrival))
        .collect()
}

/// Versioned interchange envelope for observer-emitted candidate claims.
///
/// A generic, lossy-explicit container (e.g. for COGX/sidecar export mapping)
/// that adapters can produce/consume without making the observer authoritative.
/// Carries provenance-carrying claims as proposals/candidates only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateClaimEnvelope {
    /// Envelope schema version (for adapter evolution/compat checks).
    #[serde(default = "default_envelope_version")]
    pub schema_version: u32,
    /// Producing observer id (if known).
    #[serde(default)]
    pub observer_id: Option<String>,
    /// Extractor identity/versions (if uniform across claims).
    #[serde(default)]
    pub extractor: Option<ExtractorInfo>,
    /// Candidate claims (each still provenance-carrying and unverified).
    #[serde(default)]
    pub claims: Vec<CandidateClaimPacket>,
    /// Fields/semantics dropped or degraded by export/mapping (explicit).
    #[serde(default)]
    pub lossy_fields: Vec<String>,
    /// Opaque adapter/export extras (COGX fragments, etc.).
    #[serde(default)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

fn default_envelope_version() -> u32 {
    1
}

impl CandidateClaimEnvelope {
    pub fn new(observer_id: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            observer_id: Some(observer_id.into()),
            extractor: None,
            claims: Vec::new(),
            lossy_fields: Vec::new(),
            extras: serde_json::Map::new(),
        }
    }

    /// Normalize all contained claims (deterministic ids/content).
    pub fn normalize(mut self) -> Self {
        self.claims = self.claims.into_iter().map(|c| c.normalize()).collect();
        self
    }

    /// Record append-only intake for all claims and return receipts (no registration).
    pub fn record_intake(
        &self,
        core: &mut crate::core::PhysisCore,
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> Vec<crate::epistemic::IntakeReceipt> {
        record_candidate_intake_batch(core, &self.claims, stream_id, arrival)
    }

    /// Append-only intake plus gate assessment for all claims (no registration,
    /// no certification/merge). Returns proposals/decisions in claim order.
    pub fn assess_with_intake(
        &self,
        core: &mut crate::core::PhysisCore,
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> Vec<crate::gate::AssessedIntake> {
        crate::gate::assess_claims_with_intake(core, &self.claims, known_labels, stream_id, arrival)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relation::RelationType;

    fn sample_packet() -> CandidateClaimPacket {
        CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("intake-1".into()),
            source: ClaimSource {
                uri: "doc://a".into(),
                span: Some("0..10".into()),
                kind: Some("doc".into()),
            },
            extractor: ExtractorInfo {
                id: "cognee:mock".into(),
                version: Some("0.1".into()),
                config_hash: Some("cfg".into()),
            },
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("raw:abc".into()),
            confidence: Some(0.7),
            payload: ClaimPayload::Entity(EntityMention {
                label: "alpha".into(),
                refers_to: None,
                concept: None,
            }),
            lossy_fields: vec![],
            extras: BTreeMap::new(),
        }
    }

    #[test]
    fn normalization_is_deterministic_and_records_span_loss() {
        let a = sample_packet().normalize();
        let b = sample_packet().normalize();
        assert_eq!(a.id, b.id);
        assert!(!a.lossy_fields.iter().any(|f| f == "source.span"));

        let mut lossy = sample_packet();
        lossy.source.span = None;
        let lossy_for_id = lossy.clone();
        let l = lossy.normalize();
        assert!(l.lossy_fields.iter().any(|f| f == "source.span"));
        assert_eq!(l.id, lossy_for_id.normalize().id);
    }

    #[test]
    fn serde_round_trip_and_compat() {
        let p = sample_packet().normalize();
        let j = serde_json::to_string(&p).unwrap();
        let de: CandidateClaimPacket = serde_json::from_str(&j).unwrap();
        assert_eq!(p, de);

        // Compat: missing optional/serde(default) fields parse fine.
        let minimal = r#"{
            "id":"claim:x",
            "payload":{"kind":"relation","data":{"subject":"a","predicate":"depends_on","object":"b"}}
        }"#;
        let m: CandidateClaimPacket = serde_json::from_str(minimal).unwrap();
        assert_eq!(m.id, "claim:x");
        assert!(m.lossy_fields.is_empty());
        assert!(matches!(m.payload, ClaimPayload::Relation(_)));
    }

    #[test]
    fn claim_to_observation_and_candidate_hypothesis() {
        let p = sample_packet().normalize();
        let obs = p.to_observation();
        assert!(obs.source.starts_with("observer:"));
        assert_eq!(obs.produced_by.as_deref(), Some("cognee:mock"));
        assert!(obs.body.contains(&p.id));

        let h = hypothesis_from_claim(&p, "alpha exists", vec![0.0, 1.0]);
        assert_eq!(h.status, HypothesisStatus::Candidate);
        assert!(!h.supporting_evidence.is_empty());
        assert_eq!(
            h.supporting_evidence[0].intake_id.as_deref(),
            Some("intake-1")
        );
    }

    #[test]
    fn relation_and_lesson_proposals_are_candidates() {
        let p = sample_packet().normalize();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let (edge, h) = relation_proposal(&p, &rel);
        assert_eq!(edge.relation_type, RelationType::DependsOn);
        assert_eq!(edge.source_id, "a");
        assert_eq!(h.status, HypothesisStatus::Candidate);

        let lesson = ConsolidatedLesson {
            statement: "retry fixes flake".into(),
            prediction: Some("flake rate drops".into()),
            episodes: vec!["ep1".into()],
        };
        let hl = lesson_proposal(&p, &lesson);
        assert_eq!(hl.status, HypothesisStatus::Candidate);
        assert_eq!(hl.predictions.len(), 1);
    }

    #[test]
    fn record_candidate_intake_is_append_only_and_cites_intake() {
        let mut core = crate::core::PhysisCore::new();
        let claim = sample_packet().normalize();
        let arrival = chrono::Utc::now();
        let receipt = record_candidate_intake(&mut core, &claim, "cognee", arrival);
        assert_eq!(receipt.stream_id, "cognee");
        assert!(!receipt.late);
        let ingested: Vec<_> = core
            .epistemic_audit
            .events
            .iter()
            .filter(|e| e.event_type == crate::epistemic::EpistemicEventType::ObservationIngested)
            .collect();
        assert_eq!(ingested.len(), 1);
        assert_eq!(ingested[0].intake_id.as_deref(), Some("intake-1"));
        assert_eq!(ingested[0].source.as_deref(), Some("cognee:mock"));
        // No hypotheses/edges/contradictions created by intake alone.
        assert!(core.hypotheses.is_empty());
        assert!(core.edges.is_empty());
        assert!(core.contradictions.is_empty());
    }

    #[test]
    fn envelope_normalizes_records_and_roundtrips() {
        let mut core = crate::core::PhysisCore::new();
        let mut env = CandidateClaimEnvelope::new("mock-observer");
        env.claims.push(sample_packet());
        env.lossy_fields.push("export:lossy".into());
        let env = env.normalize();
        assert_eq!(env.schema_version, 1);
        assert_eq!(env.observer_id.as_deref(), Some("mock-observer"));
        assert!(env.claims[0].id.starts_with("claim:"));

        // Lossy-explicit versioned interchange roundtrip.
        let j = serde_json::to_string(&env).unwrap();
        let de: CandidateClaimEnvelope = serde_json::from_str(&j).unwrap();
        assert_eq!(de, env);
        let minimal = r#"{"claims":[]}"#;
        let m: CandidateClaimEnvelope = serde_json::from_str(minimal).unwrap();
        assert_eq!(m.schema_version, 1);
        assert!(m.claims.is_empty());

        let before_ev = core.epistemic_audit.events.len();
        let receipts = env.record_intake(&mut core, "obs-stream", chrono::Utc::now());
        assert_eq!(receipts.len(), 1);
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 1);
        assert!(core.hypotheses.is_empty());
        assert!(core.edges.is_empty());
        assert!(core.contradictions.is_empty());

        // Assess path also records intake and returns proposals without registration.
        let assessed = env.assess_with_intake(&mut core, &[], "obs-stream", chrono::Utc::now());
        assert_eq!(assessed.len(), 1);
        assert_eq!(core.hypotheses.len(), 0);
        assert_eq!(core.edges.len(), 0);
        assert!(core.epistemic_audit.events.len() >= before_ev + 2);
    }
}
