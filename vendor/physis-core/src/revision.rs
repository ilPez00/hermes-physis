//! Action-preview and affected-set / TMS explanation helpers (deterministic).
//!
//! "Preview" evaluates a proposed mutation in a shadow context (delta/impact,
//! propagation steps, revision walk, adjudication) without committing.
//! "Affected-set / why-affected" helpers explain DependOn-selected dependents
//! and retraction cascades as inspectable, bounded query results. These are
//! read-only diagnostics over existing TMS/revision semantics — not a full
//! per-conclusion ATMS and not an auto-commit/apply path.

use serde::{Deserialize, Serialize};

use crate::delta_engine::{
    self, DeltaParams, EvaluationContext, OntologyDeltaReport, OntologyMutation,
    RetractionCascadeReport, RevisionWalk,
};
use crate::hypothesis::Hypothesis;
use crate::models::CoherenceNode;
use crate::relation::TypedEdge;

/// A shadow preview of one mutation: full delta report plus the revision walk
/// that would select dependents. Base state is untouched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPreview {
    pub report: OntologyDeltaReport,
    /// DependOn-selected dependents of the target node (proposal order).
    pub revision_walk: RevisionWalk,
}

/// Preview a mutation against base state in a shadow context (no commit).
pub fn preview_mutation(
    base_nodes: &[CoherenceNode],
    base_hypotheses: &[Hypothesis],
    base_edges: &[TypedEdge],
    mutation: &OntologyMutation,
    params: &DeltaParams,
) -> MutationPreview {
    let mut ctx = EvaluationContext::from_base(base_nodes, base_hypotheses, base_edges);
    let report = delta_engine::evaluate_mutation_with_params(&mut ctx, mutation.clone(), params);
    let revision_walk = report
        .revision_walk
        .clone()
        .unwrap_or_else(|| ctx.depends_on_walk(&mutation.target_node_id));
    MutationPreview {
        report,
        revision_walk,
    }
}

/// Explanation of which hypotheses are selected as dependents of a node walk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedSetExplanation {
    pub target_node_id: String,
    pub revision_walk: RevisionWalk,
    /// Hypotheses (ids) whose ontology refs include any walked node, in walk
    /// step order then id order, deduped.
    pub affected_hypothesis_ids: Vec<String>,
}

/// Compute the DependOn-affected hypothesis set for a target node (read-only).
pub fn explain_affected_set(
    base_nodes: &[CoherenceNode],
    base_hypotheses: &[Hypothesis],
    base_edges: &[TypedEdge],
    target_node_id: &str,
) -> AffectedSetExplanation {
    let ctx = EvaluationContext::from_base(base_nodes, base_hypotheses, base_edges);
    let revision_walk = ctx.depends_on_walk(target_node_id);
    let all_hyps = ctx.effective_hypotheses_all();
    let mut selected: Vec<String> = Vec::new();
    for step in &revision_walk.steps {
        let mut matching: Vec<String> = all_hyps
            .iter()
            .filter(|h| h.ontology_refs.iter().any(|r| r == &step.node_id))
            .map(|h| h.id.clone())
            .collect();
        matching.sort();
        for hid in matching {
            if !selected.contains(&hid) {
                selected.push(hid);
            }
        }
    }
    AffectedSetExplanation {
        target_node_id: target_node_id.to_string(),
        revision_walk,
        affected_hypothesis_ids: selected,
    }
}

/// Preview an evidence retraction with its dependent cascade in shadow.
pub fn preview_retraction_cascade(
    base_nodes: &[CoherenceNode],
    base_hypotheses: &[Hypothesis],
    base_edges: &[TypedEdge],
    hypothesis_id: &str,
    source: &str,
) -> Option<RetractionCascadeReport> {
    let mut ctx = EvaluationContext::from_base(base_nodes, base_hypotheses, base_edges);
    delta_engine::retract_evidence_with_cascade(&mut ctx, hypothesis_id, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PhysisCore;
    use crate::delta_engine::MutationOp;
    use crate::hypothesis::{Evidence, Hypothesis};
    use crate::relation::{RelationType, TypedEdge};

    #[test]
    fn preview_mutation_does_not_mutate_base() {
        let mut core = PhysisCore::new();
        let nid = core.register_node_vec(vec![1.0, 0.0]);
        core.assert_coherence(&nid, 0.8);
        let before = core.nodes.get(&nid).unwrap().coherence_score;
        let nodes: Vec<_> = core.nodes.values().cloned().collect();
        let p = preview_mutation(
            &nodes,
            &[],
            &[],
            &OntologyMutation::new(
                nid.clone(),
                MutationOp::EmbeddingShift {
                    old_embedding: vec![1.0, 0.0],
                    new_embedding: vec![0.0, 1.0],
                },
            ),
            &DeltaParams::default(),
        );
        assert_eq!(core.nodes.get(&nid).unwrap().coherence_score, before);
        assert_eq!(p.report.mutation_source.target_node_id, nid);
        assert!(!p.revision_walk.truncated);
    }

    #[test]
    fn affected_set_follows_depends_on_closure() {
        // a -> b (DependsOn) and hypothesis references b
        let mut core = PhysisCore::new();
        let a = core.register_node_vec(vec![1.0, 0.0]);
        let b = core.register_node_vec(vec![0.0, 1.0]);
        core.add_edge(TypedEdge::new(
            RelationType::DependsOn,
            b.clone(),
            a.clone(),
        ));
        let mut h = Hypothesis::new("h", vec![]);
        h.ontology_refs.push(b.clone());
        let hid = core.register_hypothesis(h);

        let nodes: Vec<_> = core.nodes.values().cloned().collect();
        let hyps: Vec<_> = core.hypotheses.values().cloned().collect();
        let ex = explain_affected_set(&nodes, &hyps, &core.edges, &b);
        assert!(ex.affected_hypothesis_ids.contains(&hid));
    }

    #[test]
    fn retraction_preview_is_shadow_only() {
        let mut core = PhysisCore::new();
        let nid = core.register_node_vec(vec![1.0]);
        let mut h = Hypothesis::new("h", vec![]);
        h.ontology_refs.push(nid.clone());
        h.add_supporting_evidence(Evidence::supports("s1", "ok"));
        let hid = core.register_hypothesis(h);
        let nodes: Vec<_> = core.nodes.values().cloned().collect();
        let hyps: Vec<_> = core.hypotheses.values().cloned().collect();
        let rep = preview_retraction_cascade(&nodes, &hyps, &core.edges, &hid, "s1").unwrap();
        assert_eq!(rep.removed, 1);
        // base untouched
        assert_eq!(
            core.hypotheses.get(&hid).unwrap().supporting_evidence.len(),
            1
        );
    }

    #[test]
    fn staged_mutation_commits_and_records_audit() {
        let mut core = PhysisCore::new();
        let before_events = core.epistemic_audit.events.len();
        let mut staged = super::StagedMutation::begin(&core);
        staged.working.register_node_vec(vec![1.0, 0.0]);
        assert_eq!(core.nodes.len(), 0, "base untouched while staged");
        let working_events_before_commit = staged.working.epistemic_audit.events.len();
        let receipt = staged.commit(&mut core, "add node").unwrap();
        assert_eq!(core.nodes.len(), 1);
        // commit appends exactly one transaction audit event on top of working state
        assert_eq!(
            core.epistemic_audit.events.len(),
            working_events_before_commit + 1
        );
        assert!(core.epistemic_audit.events.len() >= before_events + 1);
        assert!(receipt.audit_event_id.is_some());
        assert_eq!(
            core.epistemic_audit.events.last().map(|e| e.id.as_str()),
            receipt.audit_event_id.as_deref()
        );
        assert_ne!(receipt.fingerprint_before, receipt.fingerprint_after);
    }

    #[test]
    fn staged_mutation_refuses_on_drift_and_rollback_keeps_base() {
        let mut core = PhysisCore::new();
        let staged = super::StagedMutation::begin(&core);
        // mutate base after staging -> drift
        core.register_node_vec(vec![2.0, 0.0]);
        let err = staged.check_drift(&core).unwrap_err();
        assert_ne!(err.expected, err.actual);

        let staged2 = super::StagedMutation::begin(&core);
        core.register_node_vec(vec![3.0, 0.0]);
        let rb = staged2.rollback(&core);
        assert_eq!(rb.fingerprint_base.nodes, core.nodes.len());
        assert_eq!(core.nodes.len(), 2, "rollback never altered base");
    }

    #[test]
    fn staged_preview_reflects_working_state_not_base() {
        let mut core = PhysisCore::new();
        let nid = core.register_node_vec(vec![1.0, 0.0]);
        let mut staged = super::StagedMutation::begin(&core);
        // mutate working copy only
        if let Some(n) = staged.working.nodes.get_mut(&nid) {
            n.coherence_score = 0.2;
        }
        let p = staged.preview_on_working(
            &OntologyMutation::new(
                nid.clone(),
                MutationOp::EmbeddingShift {
                    old_embedding: vec![1.0, 0.0],
                    new_embedding: vec![0.0, 1.0],
                },
            ),
            &DeltaParams::default(),
        );
        assert_eq!(p.report.mutation_source.target_node_id, nid);
        // base unchanged
        assert_ne!(core.nodes.get(&nid).unwrap().coherence_score, 0.2);
    }

    #[test]
    fn staged_commit_and_persist_writes_atomically_and_checks_drift() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");

        let mut core = PhysisCore::new();
        core.register_node_vec(vec![1.0, 0.0]); // base state
        let mut staged = super::StagedMutation::begin(&core);
        staged.working.register_node_vec(vec![0.0, 1.0]);
        let receipt = staged
            .commit_and_persist(&mut core, &path, "persisted")
            .unwrap();
        assert!(receipt.audit_event_id.is_some());
        assert_eq!(core.nodes.len(), 2);
        let loaded = crate::core::PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(
            loaded.epistemic_audit.events.len(),
            core.epistemic_audit.events.len()
        );

        // drift refusal: no write, target unchanged
        let before = core.nodes.len();
        let before_json = core.to_json().unwrap();
        let staged2 = super::StagedMutation::begin(&core);
        core.register_node_vec(vec![3.0, 0.0]); // drift base after staging
        let err = staged2
            .commit_and_persist(&mut core, &path, "nope")
            .unwrap_err();
        assert!(err.to_string().contains("drift"));
        assert_eq!(core.nodes.len(), before + 1); // drift mutation remains in memory (we did it)
                                                  // file still holds previously committed state
        let disk = crate::core::PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(disk.nodes.len(), before);
        assert_ne!(core.to_json().unwrap(), before_json);
    }

    #[test]
    fn transact_and_persist_applies_mutation_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");
        let mut core = PhysisCore::new();
        let receipt = super::transact_and_persist(&mut core, &path, "add", |w| {
            w.register_node_vec(vec![1.0, 0.0]);
            w.register_node_vec(vec![0.0, 1.0]);
        })
        .unwrap();
        assert!(receipt.audit_event_id.is_some());
        assert_eq!(core.nodes.len(), 2);
        let loaded = crate::core::PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(loaded.nodes.len(), 2);

        // drift refusal path via transact_and_persist: simulate by manual staging isn't direct,
        // but transact checks drift at commit against current target (unchanged during f) so success expected.
        super::transact(&mut core, "noop", |_w| {}).unwrap();
        assert_eq!(core.nodes.len(), 2);
    }
}

// ── Reversible staged mutation (coarse drift-checked, auditable commit) ─────

/// Coarse, deterministic fingerprint of mutable core state for drift detection.
///
/// Compared on commit to refuse applying a staged mutation onto a base that
/// changed since staging. This is a cheap guard (counts + last audit marker),
/// not a cryptographic/content hash of every field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFingerprint {
    pub nodes: usize,
    pub hypotheses: usize,
    pub contradictions: usize,
    pub provenance_chains: usize,
    pub edges: usize,
    pub audit_events: usize,
    pub last_audit_id: Option<String>,
    pub watermarks: usize,
    pub process_cycles: usize,
    pub certified_branches: usize,
    pub isolated_branches: usize,
    pub dream_archive: usize,
}

/// Compute a coarse drift-detection fingerprint for `core`.
pub fn fingerprint_state(core: &crate::core::PhysisCore) -> StateFingerprint {
    let last_audit_id = core.epistemic_audit.events.last().map(|e| e.id.clone());
    StateFingerprint {
        nodes: core.nodes.len(),
        hypotheses: core.hypotheses.len(),
        contradictions: core.contradictions.len(),
        provenance_chains: core.provenance_chains.len(),
        edges: core.edges.len(),
        audit_events: core.epistemic_audit.events.len(),
        last_audit_id,
        watermarks: core.epistemic_audit.watermarks.len(),
        process_cycles: core.process_cycles.len(),
        certified_branches: core.certified_branches.len(),
        isolated_branches: core.isolated_branches.len(),
        dream_archive: core.dream_archive.len(),
    }
}

/// Refusal because the base state drifted since staging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftError {
    pub expected: StateFingerprint,
    pub actual: StateFingerprint,
}

impl std::fmt::Display for DriftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "staged mutation refused: base state drifted (expected audit_events={}, actual audit_events={}, expected last_audit_id={:?}, actual last_audit_id={:?})",
            self.expected.audit_events, self.actual.audit_events, self.expected.last_audit_id, self.actual.last_audit_id
        )
    }
}

impl std::error::Error for DriftError {}

/// Receipt for a committed staged mutation (audit-recorded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitReceipt {
    pub fingerprint_before: StateFingerprint,
    pub fingerprint_after: StateFingerprint,
    /// Id of the commit audit event recorded on the committed state.
    #[serde(default)]
    pub audit_event_id: Option<String>,
}

/// Receipt for a rolled-back staged mutation (base unchanged).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackReceipt {
    pub fingerprint_base: StateFingerprint,
}

/// A reversible, drift-checked staged mutation over a cloned working state.
///
/// `working` is a full clone of the base captured at `begin`; callers may mutate
/// it freely (or run shadow previews against it) without affecting `target`.
/// `commit` refuses if the target drifted since staging and, on success, swaps
/// the working state into the target and records an audit event. `rollback`
/// discards the working state and leaves the target untouched.
///
/// This is a generic core helper: it stages state changes, it does not itself
/// certify truth or bypass contradiction/TMS semantics of the operations the
/// caller performs on `working`.
pub struct StagedMutation {
    original_fingerprint: StateFingerprint,
    /// Mutable working copy (clone of base at begin).
    pub working: crate::core::PhysisCore,
}

impl StagedMutation {
    /// Stage a mutation against `target`: captures a coarse fingerprint and a
    /// working clone. The target is not borrowed/mutated.
    pub fn begin(target: &crate::core::PhysisCore) -> Self {
        Self {
            original_fingerprint: fingerprint_state(target),
            working: target.clone(),
        }
    }

    /// Preview a node mutation against the current *working* state (shadow, no
    /// commit to working or target). Convenience wrapper over [`preview_mutation`].
    pub fn preview_on_working(
        &self,
        mutation: &OntologyMutation,
        params: &DeltaParams,
    ) -> MutationPreview {
        let nodes: Vec<_> = self.working.nodes.values().cloned().collect();
        let hyps: Vec<_> = self.working.hypotheses.values().cloned().collect();
        preview_mutation(&nodes, &hyps, &self.working.edges, mutation, params)
    }

    /// Check whether `target` still matches the fingerprint captured at begin.
    pub fn check_drift(&self, target: &crate::core::PhysisCore) -> Result<(), Box<DriftError>> {
        let actual = fingerprint_state(target);
        if actual == self.original_fingerprint {
            Ok(())
        } else {
            Err(Box::new(DriftError {
                expected: self.original_fingerprint.clone(),
                actual,
            }))
        }
    }

    /// Commit the working state into `target` if the target has not drifted.
    ///
    /// Records an append-only `ObservationIngested` audit event on the committed
    /// state describing the transaction (record of the commit action, not a
    /// claim of verified truth). Returns the before/after fingerprints and the
    /// commit event id.
    pub fn commit(
        mut self,
        target: &mut crate::core::PhysisCore,
        description: impl Into<String>,
    ) -> Result<CommitReceipt, Box<DriftError>> {
        let (prepared, receipt) = self.prepare_commit(target, description)?;
        *target = prepared;
        Ok(receipt)
    }

    /// Commit the staged mutation and atomically persist the committed state to
    /// `path` (concrete persisted/write-path integration).
    ///
    /// Drift is checked against `target` before any write. The commit audit event
    /// is recorded in the state that is serialized; on write failure neither the
    /// in-memory `target` nor the file at `path` is modified. On success the
    /// persisted file and in-memory `target` are identical committed states.
    ///
    /// This does not certify truth or bypass contradiction/TMS semantics of the
    /// operations applied to the working state.
    pub fn commit_and_persist(
        mut self,
        target: &mut crate::core::PhysisCore,
        path: impl AsRef<std::path::Path>,
        description: impl Into<String>,
    ) -> anyhow::Result<CommitReceipt> {
        let path = path.as_ref();
        let (prepared, receipt) = self
            .prepare_commit(target, description)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let json = prepared.to_json()?;
        crate::store::atomic_write(path, json.as_bytes())?;
        *target = prepared;
        Ok(receipt)
    }

    /// Like [`Self::commit_and_persist`] but persists to the default core store path.
    pub fn commit_and_persist_default(
        self,
        target: &mut crate::core::PhysisCore,
        description: impl Into<String>,
    ) -> anyhow::Result<CommitReceipt> {
        self.commit_and_persist(target, crate::store::nodes_path(), description)
    }

    fn prepare_commit(
        &mut self,
        target: &crate::core::PhysisCore,
        description: impl Into<String>,
    ) -> Result<(crate::core::PhysisCore, CommitReceipt), Box<DriftError>> {
        self.check_drift(target)?;
        let fingerprint_before = fingerprint_state(target);

        let desc = description.into();
        let ev = crate::epistemic::EpistemicEvent::new(
            crate::epistemic::EpistemicEventType::ObservationIngested,
            "state:transaction",
            if desc.is_empty() {
                "staged mutation committed".to_string()
            } else {
                format!("staged mutation committed: {}", desc)
            },
        )
        .with_source("revision::staged_mutation");
        let audit_event_id = Some(ev.id.clone());
        self.working.record_epistemic_event(ev);

        let fingerprint_after = fingerprint_state(&self.working);
        let prepared = self.working.clone();

        Ok((
            prepared,
            CommitReceipt {
                fingerprint_before,
                fingerprint_after,
                audit_event_id,
            },
        ))
    }

    /// Discard the working state and leave the target unchanged.
    pub fn rollback(self, target: &crate::core::PhysisCore) -> RollbackReceipt {
        RollbackReceipt {
            fingerprint_base: fingerprint_state(target),
        }
    }
}

/// Stage a mutation, apply `f` to the working copy, then commit into `target`.
///
/// High-level in-memory workflow orchestration (no persistence). Drift is
/// checked at commit; on refusal `target` is unchanged. `f` must not certify
/// truth or bypass contradiction/TMS semantics.
pub fn transact<F>(
    target: &mut crate::core::PhysisCore,
    description: impl Into<String>,
    f: F,
) -> Result<CommitReceipt, Box<DriftError>>
where
    F: FnOnce(&mut crate::core::PhysisCore),
{
    let mut staged = StagedMutation::begin(target);
    f(&mut staged.working);
    staged.commit(target, description)
}

/// Stage a mutation, apply `f` to the working copy, commit, and atomically
/// persist the committed state to `path`.
///
/// Integrates staged commit/persist patterns for concrete persisted write-path
/// workflows: drift-checked, commit audit recorded in serialized state, and on
/// refusal/write failure neither in-memory `target` nor `path` is modified.
pub fn transact_and_persist<F>(
    target: &mut crate::core::PhysisCore,
    path: impl AsRef<std::path::Path>,
    description: impl Into<String>,
    f: F,
) -> anyhow::Result<CommitReceipt>
where
    F: FnOnce(&mut crate::core::PhysisCore),
{
    let mut staged = StagedMutation::begin(target);
    f(&mut staged.working);
    staged.commit_and_persist(target, path, description)
}

/// Like [`transact_and_persist`] but persists to the default core store path.
pub fn transact_and_persist_default<F>(
    target: &mut crate::core::PhysisCore,
    description: impl Into<String>,
    f: F,
) -> anyhow::Result<CommitReceipt>
where
    F: FnOnce(&mut crate::core::PhysisCore),
{
    transact_and_persist(target, crate::store::nodes_path(), description, f)
}
