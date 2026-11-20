//! TMS / "why supported (and affected) now" explanation surfaces.
//!
//! Deterministic, read-only helpers that compose existing evidence, provenance,
//! audit, revision/DependsOn, and stability signals into inspectable snapshots.
//! These are diagnostics/explanations — not verdicts, not certification, and not
//! a full per-conclusion ATMS.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::coherence_dimensions::CoherenceProfile;
use crate::core::PhysisCore;
use crate::dream::{self, StabilityFlag};
use crate::epistemic::EpistemicEvent;
use crate::hypothesis::{FitnessBreakdown, HypothesisStatus};
use crate::models::Score;
use crate::revision::{self, AffectedSetExplanation};

/// A compact, structured view of why a hypothesis stands as it does now and
/// what DependsOn-selected subjects it may affect (or be affected through its
/// referenced nodes). Purely descriptive/composed from existing state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportSnapshot {
    pub hypothesis_id: String,
    pub statement: String,
    pub status: HypothesisStatus,
    pub confidence: Score,
    pub coherence: Score,
    pub fitness: Score,
    pub fitness_breakdown: FitnessBreakdown,
    pub coherence_profile: CoherenceProfile,
    pub supporting_evidence: usize,
    pub contradicting_evidence: usize,
    pub open_predictions: usize,
    /// Distinct intake ids cited by supporting/contradicting evidence, in
    /// encounter order.
    #[serde(default)]
    pub evidence_intake_ids: Vec<String>,
    /// Distinct intake ids cited by the hypothesis' provenance chain, in
    /// encounter order.
    #[serde(default)]
    pub provenance_intake_ids: Vec<String>,
    pub ontology_refs: Vec<String>,
    /// DependsOn-affected hypothesis sets per referenced node (diagnostic blast
    /// radius / support neighborhood), in ontology_ref order.
    #[serde(default)]
    pub affected_by_ref: Vec<AffectedSetExplanation>,
    /// Union of dependent hypothesis ids across `affected_by_ref` (excluding the
    /// subject itself), sorted.
    #[serde(default)]
    pub dependent_hypothesis_ids: Vec<String>,
    /// Recent audit events for this subject (most recent last), bounded.
    #[serde(default)]
    pub recent_events: Vec<EpistemicEvent>,
    /// Inspectable stability/oscillation flags for this subject from audit history
    /// (deterministic, read-only diagnostic; never a verdict).
    #[serde(default)]
    pub stability_flags: Vec<StabilityFlag>,
    /// True when there is active contradiction pressure (contradicting evidence
    /// present or status is Contradicted).
    pub contradiction_pressure: bool,
}

/// Build a support snapshot for `hypothesis_id`.
///
/// Read-only and deterministic given `core` state. Returns `None` if the
/// hypothesis is unknown. `recent_event_limit` bounds trailing audit events
/// included (0 = none).
pub fn explain_support_snapshot(
    core: &PhysisCore,
    hypothesis_id: &str,
    recent_event_limit: usize,
) -> Option<SupportSnapshot> {
    let h = core.hypotheses.get(hypothesis_id)?;

    let mut evidence_intake_ids: Vec<String> = Vec::new();
    for e in h
        .supporting_evidence
        .iter()
        .chain(h.contradicting_evidence.iter())
    {
        if let Some(id) = &e.intake_id {
            if !evidence_intake_ids.contains(id) {
                evidence_intake_ids.push(id.clone());
            }
        }
    }

    let provenance_intake_ids = core
        .provenance_chains
        .get(hypothesis_id)
        .map(|p| p.cited_intake_ids())
        .unwrap_or_default();

    let nodes: Vec<_> = core.nodes.values().cloned().collect();
    let hyps: Vec<_> = core.hypotheses.values().cloned().collect();

    let mut affected_by_ref = Vec::new();
    let mut dep_union: BTreeSet<String> = BTreeSet::new();
    for node_ref in &h.ontology_refs {
        let ex = revision::explain_affected_set(&nodes, &hyps, &core.edges, node_ref);
        for hid in &ex.affected_hypothesis_ids {
            if hid != hypothesis_id {
                dep_union.insert(hid.clone());
            }
        }
        affected_by_ref.push(ex);
    }

    let mut recent_events: Vec<EpistemicEvent> = core
        .epistemic_audit
        .history_for(hypothesis_id)
        .into_iter()
        .cloned()
        .collect();
    if recent_event_limit > 0 && recent_events.len() > recent_event_limit {
        let start = recent_events.len() - recent_event_limit;
        recent_events = recent_events[start..].to_vec();
    } else if recent_event_limit == 0 {
        recent_events.clear();
    }

    let stability_flags: Vec<StabilityFlag> = dream::analyse_stability(&core.epistemic_audit)
        .into_iter()
        .filter(|f| f.subject_id == hypothesis_id)
        .collect();

    Some(SupportSnapshot {
        hypothesis_id: h.id.clone(),
        statement: h.statement.clone(),
        status: h.status,
        confidence: h.confidence,
        coherence: h.coherence,
        fitness: h.fitness,
        fitness_breakdown: h.fitness_breakdown.clone(),
        coherence_profile: h.coherence_profile.clone(),
        supporting_evidence: h.supporting_evidence.len(),
        contradicting_evidence: h.contradicting_evidence.len(),
        open_predictions: h.pending_predictions(),
        evidence_intake_ids,
        provenance_intake_ids,
        ontology_refs: h.ontology_refs.clone(),
        affected_by_ref,
        dependent_hypothesis_ids: dep_union.into_iter().collect(),
        recent_events,
        stability_flags,
        contradiction_pressure: !h.contradicting_evidence.is_empty()
            || h.status == HypothesisStatus::Contradicted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PhysisCore;
    use crate::hypothesis::{Evidence, Hypothesis};
    use crate::relation::{RelationType, TypedEdge};

    #[test]
    fn support_snapshot_collects_evidence_intakes_and_dependents() {
        let mut core = PhysisCore::new();
        let a = core.register_node_vec(vec![1.0, 0.0]);
        let b = core.register_node_vec(vec![0.0, 1.0]);
        // a depends on b: mutating b makes a a DependsOn-dependent reached by the walk from b.
        core.add_edge(TypedEdge::new(
            RelationType::DependsOn,
            a.clone(),
            b.clone(),
        ));

        let mut h = Hypothesis::new("h", vec![]);
        h.ontology_refs.push(b.clone());
        let ev = Evidence::supports("s1", "ok").with_intake_id("intake-x");
        h.add_supporting_evidence(ev);
        let hid = core.register_hypothesis(h);

        let mut dep_h = Hypothesis::new("dep", vec![]);
        dep_h.ontology_refs.push(a.clone());
        let dep_id = core.register_hypothesis(dep_h);

        core.provenance_chain(&hid)
            .add_link(crate::provenance::ProvenanceLink::new("c", "s").with_intake_id("prov-y"));

        let snap = explain_support_snapshot(&core, &hid, 10).unwrap();
        assert_eq!(snap.hypothesis_id, hid);
        assert_eq!(snap.evidence_intake_ids, vec!["intake-x".to_string()]);
        assert_eq!(snap.provenance_intake_ids, vec!["prov-y".to_string()]);
        assert_eq!(snap.ontology_refs, vec![b.clone()]);
        // walking from b reaches a (DependsOn dependent) and dep_h references a
        assert!(snap.dependent_hypothesis_ids.contains(&dep_id));
        assert!(!snap.contradiction_pressure);
    }

    #[test]
    fn support_snapshot_marks_contradiction_pressure() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("h", vec![]);
        h.add_contradicting_evidence(Evidence::contradicts("s", "no"));
        let hid = core.register_hypothesis(h);
        let snap = explain_support_snapshot(&core, &hid, 0).unwrap();
        assert!(snap.contradiction_pressure);
        assert!(snap.recent_events.is_empty());
        assert!(snap.stability_flags.is_empty());
    }

    #[test]
    fn support_snapshot_surfaces_stability_flags_for_subject() {
        use crate::epistemic::{EpistemicEvent, EpistemicEventType};
        let mut core = PhysisCore::new();
        let h = Hypothesis::new("h", vec![]);
        let hid = core.register_hypothesis(h);
        // craft oscillating status history for this subject
        let base = chrono::Utc::now();
        let mk = |off: i64, post: &str| {
            let mut ev = EpistemicEvent::new(EpistemicEventType::StatusTransition, &hid, post)
                .with_transition("candidate", post);
            ev.timestamp = base + chrono::Duration::seconds(off);
            ev
        };
        core.record_epistemic_event(mk(1, "supported"));
        core.record_epistemic_event(mk(2, "contradicted"));
        core.record_epistemic_event(mk(3, "supported"));
        core.record_epistemic_event(mk(4, "contradicted"));
        let snap = explain_support_snapshot(&core, &hid, 10).unwrap();
        assert!(!snap.stability_flags.is_empty());
        assert!(snap.stability_flags.iter().all(|f| f.subject_id == hid));
    }
}
