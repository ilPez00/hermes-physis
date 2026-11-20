//! Central engine state — coherence nodes, certified/isolated branches,
//! first-class hypotheses, typed graph relationships, process cycles, and
//! an epistemic audit trail.
//!
//! Maintains competing interpretations of reality, evaluates their coherence with
//! observations, processes, evidence, and outcomes, and preferentially retains
//! interpretations that continue to work.

use std::collections::HashMap;

use crate::coherence_dimensions::CoherenceProfile;
use crate::coherence_query::{
    EpistemicQuery, EpistemicQueryResult, FailedPredictionSummary, HypothesisSummary,
};
use crate::contradiction::{Contradiction, ContradictionParty};
use crate::embed::VectorEmbed;
use crate::epistemic::{EpistemicAuditTrail, EpistemicEvent, EpistemicEventType};
use crate::explanation::{ExplanationReport, HistoricalPrecedent};
use crate::hypothesis::{Hypothesis, HypothesisStatus};
use crate::models::*;
use crate::process::ProcessCycle;
use crate::provenance::ProvenanceChain;
use crate::relation::{RelationType, TypedEdge};

/// Bumped when the on-disk shape changes. v1 was a bare array of nodes and is
/// still readable; v2 is the object below, which carries the whole engine state.
const PERSIST_VERSION: u32 = 2;

/// Borrowed view used for writing, so saving never clones the graph.
#[derive(serde::Serialize)]
struct PersistedCore<'a> {
    version: u32,
    nodes: Vec<&'a CoherenceNode>,
    hypotheses: &'a HashMap<String, Hypothesis>,
    contradictions: &'a Vec<Contradiction>,
    provenance_chains: &'a HashMap<String, ProvenanceChain>,
    edges: &'a Vec<TypedEdge>,
    epistemic_audit: &'a EpistemicAuditTrail,
    process_cycles: &'a Vec<ProcessCycle>,
    certified_branches: &'a Vec<CertifiedBranch>,
    isolated_branches: &'a Vec<IsolatedBranch>,
    dream_archive: &'a Vec<DreamResult>,
}

/// Owned counterpart used for reading. Every collection defaults, so a file
/// written by an older build (or a newer one missing a field) still loads.
#[derive(serde::Deserialize)]
struct PersistedCoreOwned {
    #[serde(default)]
    nodes: Vec<CoherenceNode>,
    #[serde(default)]
    hypotheses: HashMap<String, Hypothesis>,
    #[serde(default)]
    contradictions: Vec<Contradiction>,
    #[serde(default)]
    provenance_chains: HashMap<String, ProvenanceChain>,
    #[serde(default)]
    edges: Vec<TypedEdge>,
    #[serde(default)]
    epistemic_audit: EpistemicAuditTrail,
    #[serde(default)]
    process_cycles: Vec<ProcessCycle>,
    #[serde(default)]
    certified_branches: Vec<CertifiedBranch>,
    #[serde(default)]
    isolated_branches: Vec<IsolatedBranch>,
    #[serde(default)]
    dream_archive: Vec<DreamResult>,
}

/// Central engine state.
#[derive(Debug, Default, Clone)]
pub struct PhysisCore {
    /// All coherence nodes keyed by ID.
    pub nodes: HashMap<String, CoherenceNode>,
    /// Branches certified as coherent clusters.
    pub certified_branches: Vec<CertifiedBranch>,
    /// Branches flagged as low-coherence outliers.
    pub isolated_branches: Vec<IsolatedBranch>,
    /// Archive of dream evaluation results.
    pub dream_archive: Vec<DreamResult>,
    /// label → node id, for O(1) dedup of labeled registrations.
    label_index: HashMap<String, String>,
    /// Embedder id stamped onto every node registered here (provenance).
    embedder_id: Option<String>,
    /// First-class hypotheses (interpretations with evidence, status, provenance).
    pub hypotheses: HashMap<String, Hypothesis>,
    /// Explicit contradictions between claims.
    pub contradictions: Vec<Contradiction>,
    /// Provenance chains for audit trails.
    pub provenance_chains: HashMap<String, ProvenanceChain>,
    /// Rich typed graph edges connecting nodes, hypotheses, and observations.
    pub edges: Vec<TypedEdge>,
    /// Replayable timeline of epistemic events.
    pub epistemic_audit: EpistemicAuditTrail,
    /// Process and PDCA execution cycles.
    pub process_cycles: Vec<ProcessCycle>,
}

impl PhysisCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stamp the embedder id onto every node registered after this call.
    pub fn set_embedder_id(&mut self, id: impl Into<String>) {
        self.embedder_id = Some(id.into());
    }

    pub fn embedder_id(&self) -> Option<&str> {
        self.embedder_id.as_deref()
    }

    /// Register a vector, optionally retaining source text as a browsable label.
    /// Labeled registrations are deduped by label (idempotent re-scans).
    pub fn register_node_vec_labeled(
        &mut self,
        embedding: Vec<f32>,
        label: Option<String>,
    ) -> String {
        if let Some(ref l) = label {
            if let Some(existing) = self.label_index.get(l) {
                return existing.clone();
            }
        }
        let mut node = CoherenceNode::new(embedding.clone());
        node.label = label.clone();
        node.embedder = self.embedder_id.clone();
        let id = node.id.clone();
        if let Some(l) = label {
            self.label_index.insert(l, id.clone());
        }
        self.nodes.insert(id.clone(), node);
        self.update_coherence(&id);

        self.epistemic_audit.record(
            EpistemicEvent::new(
                EpistemicEventType::ObservationIngested,
                &id,
                format!(
                    "Registered node vector (label: {:?})",
                    self.nodes[&id].label
                ),
            )
            .with_source(self.embedder_id.as_deref().unwrap_or("unknown_embedder")),
        );

        id
    }

    pub fn register_node_vec(&mut self, embedding: Vec<f32>) -> String {
        self.register_node_vec_labeled(embedding, None)
    }

    /// Register from text (embed first using an embedder).
    pub fn register_node_from_text(&mut self, text: &str, embedder: &dyn VectorEmbed) -> String {
        let embedding = embedder.embed(text);
        self.register_node_vec_labeled(embedding, Some(text.to_string()))
    }

    /// Report whether a node actually worked. Score clamped to [-1, 1].
    pub fn assert_coherence(&mut self, id: &str, score: Score) -> bool {
        match self.nodes.get_mut(id) {
            Some(node) => {
                let clamped = score.clamp(-1.0, 1.0);
                node.asserted = Some(clamped);
                true
            }
            None => false,
        }
    }

    /// Mean of the reported verdicts, or `None` when nothing has been judged.
    pub fn asserted_index(&self) -> Option<Score> {
        let judged: Vec<Score> = self.nodes.values().filter_map(|n| n.asserted).collect();
        if judged.is_empty() {
            return None;
        }
        Some(judged.iter().sum::<Score>() / judged.len() as Score)
    }

    /// Nodes someone reported as not working, worst first.
    pub fn asserted_failures(&self) -> Vec<&CoherenceNode> {
        let mut out: Vec<&CoherenceNode> = self
            .nodes
            .values()
            .filter(|n| n.is_asserted_failure())
            .collect();
        out.sort_by(|a, b| {
            a.asserted
                .unwrap_or(0.0)
                .partial_cmp(&b.asserted.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }

    /// Recall labeled nodes by similarity to `query`. Embeddings are L2-normalized.
    pub fn search_nodes(&self, query: &[f32], max: usize) -> Vec<(String, String, Score)> {
        let mut scored: Vec<(String, String, Score)> = self
            .nodes
            .values()
            .filter_map(|n| {
                let label = n.label.as_ref()?;
                let dot: f32 = n.embedding.iter().zip(query).map(|(a, b)| a * b).sum();
                Some((n.id.clone(), label.clone(), dot))
            })
            .collect();
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max);
        scored
    }

    /// Search by text.
    pub fn search_text(
        &self,
        query: &str,
        embedder: &dyn VectorEmbed,
        max: usize,
    ) -> Vec<(String, String, Score)> {
        self.search_nodes(&embedder.embed(query), max)
    }

    /// Look up a node by its exact label string.
    pub fn node_by_label(&self, label: &str) -> Option<&CoherenceNode> {
        let id = self.label_index.get(label)?;
        self.nodes.get(id)
    }

    /// Modify an existing node's content in place while preserving its id,
    /// asserted verdict, edges, and provenance.
    pub fn edit_node(
        &mut self,
        id: &str,
        new_label: &str,
        new_embedding: Vec<f32>,
        pin: PinEdit,
        embedder_id: Option<&str>,
    ) -> anyhow::Result<NodeEditOutcome> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("unknown node {id}"))?;
        if let Some(old) = node.label.take() {
            if self.label_index.get(&old).map(String::as_str) == Some(id) {
                self.label_index.remove(&old);
            }
        }
        node.label = Some(new_label.to_string());
        node.embedding = new_embedding;
        node.embedder = embedder_id.map(str::to_string);
        match pin {
            PinEdit::Keep => {}
            PinEdit::Set(domain, mode) => node.cell_pin = Some((domain, mode)),
            PinEdit::Clear => node.cell_pin = None,
        }
        self.label_index
            .insert(new_label.to_string(), id.to_string());
        self.update_coherence(id);
        let node = &self.nodes[id];
        Ok(NodeEditOutcome {
            node_id: id.to_string(),
            label: node.label.clone().unwrap_or_default(),
            pinned_cell: node.cell_pin.clone(),
            asserted: node.asserted,
            coherence_score: node.coherence_score,
        })
    }

    /// Remove a node and its label index entry. Returns false when the id is unknown.
    pub fn delete_node(&mut self, id: &str) -> bool {
        let Some(node) = self.nodes.remove(id) else {
            return false;
        };
        if let Some(label) = node.label {
            if self.label_index.get(&label).map(String::as_str) == Some(id) {
                self.label_index.remove(&label);
            }
        }
        true
    }

    /// Delete every node whose label is `label` or begins with `label → `.
    pub fn delete_nodes_by_label_prefix(&mut self, label: &str) -> Vec<String> {
        let prefix = format!("{label} → ");
        let ids: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| {
                n.label
                    .as_deref()
                    .is_some_and(|l| l == label || l.starts_with(&prefix))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            self.delete_node(id);
        }
        ids
    }

    /// Update coherence score for a node (mean cosine to k nearest neighbors).
    fn update_coherence(&mut self, node_id: &str) {
        if self.nodes.len() <= 1 {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.coherence_score = 1.0;
            }
            return;
        }

        let query = match self.nodes.get(node_id) {
            Some(n) => n.embedding.clone(),
            None => return,
        };

        let k = 5.min(self.nodes.len() - 1);
        let mut sims: Vec<f32> = self
            .nodes
            .values()
            .filter(|n| n.id != node_id)
            .map(|n| cosine_sim(&query, &n.embedding))
            .collect();
        sims.sort_by(|a, b| b.partial_cmp(a).unwrap());
        sims.truncate(k);

        let avg = if sims.is_empty() {
            1.0
        } else {
            sims.iter().sum::<f32>() / sims.len() as f32
        };

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.coherence_score = avg.max(0.0);
        }
    }

    /// Consistency check in vector space.
    pub fn check_consistency(&self, query_embedding: &[f32], threshold: f32) -> ConsistencyResult {
        for node in self.nodes.values() {
            let sim = cosine_sim(query_embedding, &node.embedding);
            if sim > threshold {
                let gap = 1.0 - sim;
                let refutation = ConstructiveRefutation::new(
                    query_embedding.to_vec(),
                    vec![node.id.clone()],
                    "",
                    gap,
                );
                return ConsistencyResult::Conflict(refutation);
            }
        }
        ConsistencyResult::Clean
    }

    /// Group nodes into certified dense clusters.
    pub fn certify_branches(&mut self) -> Vec<CertifiedBranch> {
        let mut certified = Vec::new();
        let high_nodes: Vec<&CoherenceNode> = self
            .nodes
            .values()
            .filter(|n| n.coherence_score > 0.7)
            .collect();
        if !high_nodes.is_empty() {
            let ids: Vec<String> = high_nodes.iter().map(|n| n.id.clone()).collect();
            let branch = CertifiedBranch {
                branch_id: uuid::Uuid::new_v4().to_string(),
                node_ids: ids,
                centroid: Vec::new(),
                stability_score: 0.85,
            };
            certified.push(branch.clone());
            self.certified_branches.push(branch);
        }
        certified
    }

    /// Detect isolated outliers.
    pub fn detect_contradictions(&mut self) -> Vec<IsolatedBranch> {
        let mut isolated = Vec::new();
        let low_nodes: Vec<&CoherenceNode> = self
            .nodes
            .values()
            .filter(|n| n.coherence_score < 0.3)
            .collect();
        for node in low_nodes {
            let branch = IsolatedBranch {
                branch_id: uuid::Uuid::new_v4().to_string(),
                node_ids: vec![node.id.clone()],
                outlier_score: node.coherence_score,
            };
            isolated.push(branch.clone());
            self.isolated_branches.push(branch);
        }
        isolated
    }

    /// Dream over asserted failures.
    pub fn dream(&mut self) -> Vec<DreamResult> {
        let failures = self.asserted_failures();
        if failures.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        for failure in failures {
            // Preference-aware dream seeding: when a preference is set,
            // bias the dream outcome toward preference-aligned states,
            // producing computational analogues of volition at levels 5-6.
            let preference_bias = failure.preference.unwrap_or(0.0);
            let outcome_bias = preference_bias * 0.1;

            let result = DreamResult {
                dream_id: uuid::Uuid::new_v4().to_string(),
                nodes_tested: vec![failure.id.clone()],
                outcome: 0.5 + outcome_bias,
                prevented_failure: preference_bias > 0.0,
                coherence_delta: preference_bias.abs() * 0.05,
            };
            results.push(result);
        }
        self.dream_archive.extend(results.clone());
        results
    }

    /// Mean coherence across all nodes.
    pub fn coherence_index(&self) -> Score {
        if self.nodes.is_empty() {
            return 1.0;
        }
        let sum: Score = self.nodes.values().map(|n| n.coherence_score).sum();
        sum / self.nodes.len() as Score
    }

    pub fn snapshot(&self) -> CoherenceSnapshot {
        let total = self.nodes.len();
        let high = self
            .nodes
            .values()
            .filter(|n| n.coherence_score > 0.7)
            .count();
        let mid = self
            .nodes
            .values()
            .filter(|n| n.coherence_score > 0.3 && n.coherence_score <= 0.7)
            .count();
        let low = self
            .nodes
            .values()
            .filter(|n| n.coherence_score <= 0.3)
            .count();
        let rated = |want: CoherenceRating| {
            self.nodes
                .values()
                .filter(|n| n.rating() == Some(want))
                .count()
        };
        CoherenceSnapshot {
            total_nodes: total,
            high_coherence: high,
            mid_coherence: mid,
            low_coherence: low,
            certified_branches_count: self.certified_branches.len(),
            isolated_branches_count: self.isolated_branches.len(),
            dream_cycle_count: self.dream_archive.len(),
            coherence_index: self.coherence_index(),
            cluster_count: self.certified_branches.len(),
            outlier_count: self.isolated_branches.len(),
            asserted_success: rated(CoherenceRating::Success),
            asserted_inert: rated(CoherenceRating::Inert),
            asserted_failure: rated(CoherenceRating::Failure),
            asserted_index: self.asserted_index(),
        }
    }

    /// Serialize all nodes to JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(&PersistedCore {
            version: PERSIST_VERSION,
            nodes: self.nodes.values().collect(),
            hypotheses: &self.hypotheses,
            contradictions: &self.contradictions,
            provenance_chains: &self.provenance_chains,
            edges: &self.edges,
            epistemic_audit: &self.epistemic_audit,
            process_cycles: &self.process_cycles,
            certified_branches: &self.certified_branches,
            isolated_branches: &self.isolated_branches,
            dream_archive: &self.dream_archive,
        })?)
    }

    /// Restore a core from JSON, in either persistence format.
    ///
    /// The original format was a bare array of nodes, which meant every other
    /// field — hypotheses, contradictions, the epistemic audit trail — was
    /// dropped on every save. `physis-core hypothesis create` printed
    /// "Registered hypothesis [id]" and the next `hypothesis list` reported
    /// zero; `audit` and `contradiction list` were empty for the same reason.
    ///
    /// A bare array is still accepted so existing `nodes.json` files load
    /// unchanged; they simply start with the other collections empty.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        let (nodes, mut core) = if value.is_array() {
            (
                serde_json::from_value::<Vec<CoherenceNode>>(value)?,
                Self::new(),
            )
        } else {
            let p: PersistedCoreOwned = serde_json::from_value(value)?;
            let mut core = Self::new();
            core.hypotheses = p.hypotheses;
            core.contradictions = p.contradictions;
            core.provenance_chains = p.provenance_chains;
            core.edges = p.edges;
            core.epistemic_audit = p.epistemic_audit;
            core.process_cycles = p.process_cycles;
            core.certified_branches = p.certified_branches;
            core.isolated_branches = p.isolated_branches;
            core.dream_archive = p.dream_archive;
            (p.nodes, core)
        };

        for n in nodes {
            if let Some(ref label) = n.label {
                core.label_index.insert(label.clone(), n.id.clone());
            }
            core.nodes.insert(n.id.clone(), n);
        }
        Ok(core)
    }

    /// Atomically persist this core's full state to `path` as JSON.
    ///
    /// Concrete persisted/write-path helper (integrates with staged workflows:
    /// commit a [`crate::revision::StagedMutation`] into a target core, then
    /// persist the committed target, or use
    /// [`crate::revision::StagedMutation::commit_and_persist`] to do both with a
    /// drift check and no in-memory/disk divergence on write failure).
    pub fn save_to_path(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        crate::store::save_core(path.as_ref(), self)
    }

    /// Load a core from `path` (JSON, v1 bare-array or v2 object).
    pub fn load_from_path(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        crate::store::load_core(path.as_ref())
    }

    /// Stage a mutation over a working clone, apply `f`, then commit into `self`.
    ///
    /// High-level in-memory workflow orchestration (drift-checked, audited commit
    /// event). `f` must not certify truth or bypass contradiction/TMS semantics.
    pub fn transact<F>(
        &mut self,
        description: impl Into<String>,
        f: F,
    ) -> Result<crate::revision::CommitReceipt, Box<crate::revision::DriftError>>
    where
        F: FnOnce(&mut Self),
    {
        crate::revision::transact(self, description, f)
    }

    /// Stage a mutation, apply `f`, commit, and atomically persist to `path`.
    ///
    /// Concrete persisted/write-path workflow: drift checked before write; on
    /// refusal/write failure neither in-memory state nor `path` is modified.
    pub fn transact_and_persist<F>(
        &mut self,
        path: impl AsRef<std::path::Path>,
        description: impl Into<String>,
        f: F,
    ) -> anyhow::Result<crate::revision::CommitReceipt>
    where
        F: FnOnce(&mut Self),
    {
        crate::revision::transact_and_persist(self, path, description, f)
    }

    /// Load from the default nodes path, or a fresh empty core if missing/empty.
    pub fn load_default() -> Self {
        crate::store::load_core_default()
    }

    /// Atomically persist to the default nodes path.
    pub fn save_default(&self) -> anyhow::Result<()> {
        crate::store::save_core_default(self)
    }

    /// Append-only: record an acted command/outcome into the epistemic audit as
    /// observation events (intent + outcome). Records what was attempted/observed,
    /// not verified truth/certification; creates no hypotheses, edges, or
    /// contradictions.
    pub fn record_acted(&mut self, acted: &crate::act::Acted) -> Vec<String> {
        crate::act::record_acted_in_audit(self, acted)
    }

    /// Stage-record an acted outcome into core state and commit (in-memory
    /// workflow orchestration, drift-checked, audited commit event).
    pub fn transact_record_acted(
        &mut self,
        acted: &crate::act::Acted,
        description: impl Into<String>,
    ) -> Result<crate::revision::CommitReceipt, Box<crate::revision::DriftError>> {
        self.transact(description, |w| {
            w.record_acted(acted);
        })
    }

    /// Stage-record an acted outcome, commit, and atomically persist to `path`.
    /// On refusal/write failure neither in-memory state nor `path` is modified.
    pub fn transact_record_acted_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        acted: &crate::act::Acted,
        description: impl Into<String>,
    ) -> anyhow::Result<crate::revision::CommitReceipt> {
        self.transact_and_persist(path, description, |w| {
            w.record_acted(acted);
        })
    }

    /// Stage a mutation, apply `f`, commit, and atomically persist to the
    /// default core store path.
    pub fn transact_and_persist_default<F>(
        &mut self,
        description: impl Into<String>,
        f: F,
    ) -> anyhow::Result<crate::revision::CommitReceipt>
    where
        F: FnOnce(&mut Self),
    {
        crate::revision::transact_and_persist_default(self, description, f)
    }

    /// Stage-record an acted outcome, commit, and atomically persist to the
    /// default core store path.
    pub fn transact_record_acted_and_persist_default(
        &mut self,
        acted: &crate::act::Acted,
        description: impl Into<String>,
    ) -> anyhow::Result<crate::revision::CommitReceipt> {
        self.transact_and_persist_default(description, |w| {
            w.record_acted(acted);
        })
    }

    /// Stage and record append-only observer intake, commit, and atomically
    /// persist to `path`. Returns commit receipt and intake receipts.
    ///
    /// Intake records what observers asserted (audit/watermarks) but never
    /// verified truth; creates no hypotheses, edges, or contradictions.
    pub fn transact_ingest_observer_claims_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        claims: &[crate::ingest::CandidateClaimPacket],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        let mut staged = crate::revision::StagedMutation::begin(self);
        let receipts = staged
            .working
            .ingest_observer_claims(claims, stream_id, arrival);
        let commit = staged.commit_and_persist(self, path, description)?;
        Ok((commit, receipts))
    }

    /// Stage and record append-only observer intake, commit, and atomically
    /// persist to the default core store path.
    pub fn transact_ingest_observer_claims_and_persist_default(
        &mut self,
        claims: &[crate::ingest::CandidateClaimPacket],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        self.transact_ingest_observer_claims_and_persist(
            crate::store::nodes_path(),
            claims,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage and record append-only observer intake from an interchange
    /// envelope, commit, and atomically persist to `path`. Candidate-only,
    /// never verified truth; no hypotheses/edges/contradictions registered.
    pub fn transact_ingest_envelope_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        self.transact_ingest_observer_claims_and_persist(
            path,
            &envelope.claims,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage and record append-only observer intake from an interchange
    /// envelope, commit, and atomically persist to the default core store path.
    pub fn transact_ingest_envelope_and_persist_default(
        &mut self,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        self.transact_ingest_envelope_and_persist(
            crate::store::nodes_path(),
            envelope,
            stream_id,
            arrival,
            description,
        )
    }

    /// Observe via `observer`, then stage+record append-only intake, commit, and
    /// atomically persist to `path`. Observer extraction runs before staging; on
    /// refusal/write failure neither in-memory state nor `path` is modified.
    pub fn observe_and_ingest_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        let claims = observer.ingest(inputs)?;
        self.transact_ingest_observer_claims_and_persist(
            path,
            &claims,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage+record append-only intake and gate assessment, commit, and
    /// atomically persist to `path`. Returns proposals/decisions (intake recorded,
    /// no hypotheses/edges/contradictions registered, no certification/merge).
    pub fn transact_assess_observer_claims_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        claims: &[crate::ingest::CandidateClaimPacket],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        let mut staged = crate::revision::StagedMutation::begin(self);
        let assessed =
            staged
                .working
                .assess_observer_claims(claims, known_labels, stream_id, arrival);
        let commit = staged.commit_and_persist(self, path, description)?;
        Ok((commit, assessed))
    }

    /// Observe via `observer`, then stage+record append-only intake and gate
    /// assessment, commit, and atomically persist to `path`. Observer extraction
    /// runs before staging; no proposals are registered/certified/merged.
    // The signature mirrors the staged pipeline (path, observer, inputs, labels,
    // options, reason); collapsing it would hide the pipeline's shape.
    #[allow(clippy::too_many_arguments)]
    pub fn observe_and_assess_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        let claims = observer.ingest(inputs)?;
        self.transact_assess_observer_claims_and_persist(
            path,
            &claims,
            known_labels,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage+record append-only intake and gate assessment, commit, and
    /// atomically persist to the default core store path.
    pub fn transact_assess_observer_claims_and_persist_default(
        &mut self,
        claims: &[crate::ingest::CandidateClaimPacket],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        self.transact_assess_observer_claims_and_persist(
            crate::store::nodes_path(),
            claims,
            known_labels,
            stream_id,
            arrival,
            description,
        )
    }

    /// Observe via `observer`, then stage+record append-only intake, commit, and
    /// atomically persist to the default core store path.
    pub fn observe_and_ingest_and_persist_default(
        &mut self,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        let claims = observer.ingest(inputs)?;
        self.transact_ingest_observer_claims_and_persist_default(
            &claims,
            stream_id,
            arrival,
            description,
        )
    }

    /// Observe via `observer`, then stage+record append-only intake and gate
    /// assessment, commit, and atomically persist to the default core store path.
    pub fn observe_and_assess_and_persist_default(
        &mut self,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        let claims = observer.ingest(inputs)?;
        self.transact_assess_observer_claims_and_persist_default(
            &claims,
            known_labels,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage+record append-only intake and gate assessment from an interchange
    /// envelope, commit, and atomically persist to `path`. Returns proposals/
    /// decisions only (intake recorded) — no hypotheses/edges/contradictions
    /// registered, no certification/merge.
    pub fn transact_assess_envelope_and_persist(
        &mut self,
        path: impl AsRef<std::path::Path>,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        self.transact_assess_observer_claims_and_persist(
            path,
            &envelope.claims,
            known_labels,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage+record append-only intake and gate assessment from an interchange
    /// envelope, commit, and atomically persist to the default core store path.
    pub fn transact_assess_envelope_and_persist_default(
        &mut self,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        self.transact_assess_envelope_and_persist(
            crate::store::nodes_path(),
            envelope,
            known_labels,
            stream_id,
            arrival,
            description,
        )
    }

    /// Stage and record append-only observer intake, commit (in-memory, no
    /// persistence). Intake records what observers asserted (audit/watermarks)
    /// but never verified truth; creates no hypotheses, edges, or contradictions.
    /// On drift refusal neither in-memory state is modified.
    pub fn transact_ingest_observer_claims(
        &mut self,
        claims: &[crate::ingest::CandidateClaimPacket],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> Result<
        (
            crate::revision::CommitReceipt,
            Vec<crate::epistemic::IntakeReceipt>,
        ),
        Box<crate::revision::DriftError>,
    > {
        let mut staged = crate::revision::StagedMutation::begin(self);
        let receipts = staged
            .working
            .ingest_observer_claims(claims, stream_id, arrival);
        let commit = staged.commit(self, description)?;
        Ok((commit, receipts))
    }

    /// Stage and record append-only observer intake from an interchange
    /// envelope, commit (in-memory, no persistence). Candidate-only, never
    /// verified truth; no hypotheses/edges/contradictions registered.
    pub fn transact_ingest_envelope(
        &mut self,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> Result<
        (
            crate::revision::CommitReceipt,
            Vec<crate::epistemic::IntakeReceipt>,
        ),
        Box<crate::revision::DriftError>,
    > {
        self.transact_ingest_observer_claims(&envelope.claims, stream_id, arrival, description)
    }

    /// Stage+record append-only intake and gate assessment, commit (in-memory,
    /// no persistence). Returns proposals/decisions (intake recorded, no
    /// hypotheses/edges/contradictions registered, no certification/merge).
    pub fn transact_assess_observer_claims(
        &mut self,
        claims: &[crate::ingest::CandidateClaimPacket],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> Result<
        (
            crate::revision::CommitReceipt,
            Vec<crate::gate::AssessedIntake>,
        ),
        Box<crate::revision::DriftError>,
    > {
        let mut staged = crate::revision::StagedMutation::begin(self);
        let assessed =
            staged
                .working
                .assess_observer_claims(claims, known_labels, stream_id, arrival);
        let commit = staged.commit(self, description)?;
        Ok((commit, assessed))
    }

    /// Stage+record append-only intake and gate assessment from an interchange
    /// envelope, commit (in-memory, no persistence). Returns proposals/decisions
    /// only — no hypotheses/edges/contradictions registered, no certification/merge.
    pub fn transact_assess_envelope(
        &mut self,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> Result<
        (
            crate::revision::CommitReceipt,
            Vec<crate::gate::AssessedIntake>,
        ),
        Box<crate::revision::DriftError>,
    > {
        self.transact_assess_observer_claims(
            &envelope.claims,
            known_labels,
            stream_id,
            arrival,
            description,
        )
    }

    /// Observe via `observer`, then stage+record append-only intake, commit
    /// (in-memory, no persistence). Observer extraction runs before staging; on
    /// refusal/drift/extraction failure in-memory state is not modified by the
    /// transaction. Intake is candidate-only (never verified truth) and creates
    /// no hypotheses, edges, or contradictions.
    pub fn transact_observe_and_ingest(
        &mut self,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::epistemic::IntakeReceipt>,
    )> {
        let claims = observer.ingest(inputs)?;
        self.transact_ingest_observer_claims(&claims, stream_id, arrival, description)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// Observe via `observer`, then stage+record append-only intake and gate
    /// assessment, commit (in-memory, no persistence). Observer extraction runs
    /// before staging; returns proposals/decisions only (intake recorded) with no
    /// hypotheses/edges/contradictions registered and no certification/merge.
    pub fn transact_observe_and_assess(
        &mut self,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
        description: impl Into<String>,
    ) -> anyhow::Result<(
        crate::revision::CommitReceipt,
        Vec<crate::gate::AssessedIntake>,
    )> {
        let claims = observer.ingest(inputs)?;
        self.transact_assess_observer_claims(&claims, known_labels, stream_id, arrival, description)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    // ── First-Class Hypotheses ─────────────────────────────────────────

    /// Register a new hypothesis and record epistemic event.
    pub fn register_hypothesis(&mut self, hypothesis: Hypothesis) -> String {
        let id = hypothesis.id.clone();
        self.epistemic_audit.record(
            EpistemicEvent::new(
                EpistemicEventType::HypothesisGenerated,
                &id,
                format!("Hypothesis formulated: {}", hypothesis.statement),
            )
            .with_transition("none", hypothesis.status.as_str())
            .with_metric(hypothesis.fitness),
        );
        self.hypotheses.insert(id.clone(), hypothesis);
        id
    }

    /// Get a hypothesis by id.
    pub fn hypothesis(&self, id: &str) -> Option<&Hypothesis> {
        self.hypotheses.get(id)
    }

    /// Get a mutable hypothesis by id.
    pub fn hypothesis_mut(&mut self, id: &str) -> Option<&mut Hypothesis> {
        self.hypotheses.get_mut(id)
    }

    /// Transition a hypothesis to a new status.
    pub fn transition_hypothesis(
        &mut self,
        id: &str,
        new_status: HypothesisStatus,
        description: impl Into<String>,
        trigger: Option<String>,
    ) -> bool {
        let desc = description.into();
        if let Some(h) = self.hypotheses.get_mut(id) {
            let prior = h.status.as_str().to_string();
            h.transition_to(new_status, desc.clone(), trigger.clone());
            self.epistemic_audit.record(
                EpistemicEvent::new(EpistemicEventType::StatusTransition, id, desc)
                    .with_transition(prior, new_status.as_str())
                    .with_metric(h.fitness),
            );
            true
        } else {
            false
        }
    }

    /// Project a hypothesis's own revision history into the audit trail.
    ///
    /// # PH-017 — why a projection rather than more emit sites
    ///
    /// `Hypothesis` mutates status in five places and has no access to the
    /// audit trail, so the packet's instinct was to emit from each. That would
    /// keep two records in sync by discipline, and discipline is what failed:
    /// the original defect was a `revise` helper that wrote the destination
    /// status into both ends of the transition, so the history claimed nothing
    /// ever changed while `list` derived standing from fitness at read time.
    ///
    /// A projection removes the disagreement by construction. `revision_history`
    /// is the single record of what changed; the audit trail is a view over it.
    /// A future mutation path cannot forget to emit, because it does not emit —
    /// it revises, and revisions are projected.
    ///
    /// Idempotent: only revisions newer than the last projected event are
    /// recorded, so calling this repeatedly does not duplicate history. Only
    /// **real** transitions are projected — a revision whose status did not move
    /// is audited on the hypothesis but is not a transition, because the log
    /// records transitions rather than heartbeats.
    pub fn project_revisions(&mut self, id: &str) -> usize {
        let Some(h) = self.hypotheses.get(id) else {
            return 0;
        };
        let last_seen = self
            .epistemic_audit
            .events
            .iter()
            .filter(|e| e.subject_id == id && e.event_type == EpistemicEventType::StatusTransition)
            .map(|e| e.timestamp)
            .max();

        let pending: Vec<_> = h
            .revision_history
            .iter()
            .filter(|r| last_seen.is_none_or(|t| r.timestamp > t))
            .filter(|r| r.previous_status != r.new_status)
            .cloned()
            .collect();
        let fitness = h.fitness;

        let n = pending.len();
        for r in pending {
            let mut ev = EpistemicEvent::new(
                EpistemicEventType::StatusTransition,
                id,
                r.description.clone(),
            )
            .with_transition(r.previous_status.as_str(), r.new_status.as_str())
            .with_metric(fitness);
            // Keep the revision's own clock: an event stamped "now" would make
            // `replay --at T` answer with a belief the machine did not hold at T.
            ev.timestamp = r.timestamp;
            self.epistemic_audit.record(ev);
        }
        n
    }

    /// Project every hypothesis's revisions. Cheap and idempotent; call it
    /// wherever the store is about to be read for standing.
    pub fn project_all_revisions(&mut self) -> usize {
        let ids: Vec<String> = self.hypotheses.keys().cloned().collect();
        ids.iter().map(|id| self.project_revisions(id)).sum()
    }

    // ── Contradictions ─────────────────────────────────────────────────

    /// Record an explicit contradiction between two claims without overwriting.
    pub fn record_contradiction(&mut self, contradiction: Contradiction) -> String {
        let id = contradiction.id.clone();
        self.epistemic_audit.record(
            EpistemicEvent::new(
                EpistemicEventType::ContradictionDetected,
                &id,
                format!(
                    "Contradiction detected between [{}] and [{}]",
                    contradiction.claim_a.claim, contradiction.claim_b.claim
                ),
            )
            .with_source("contradiction_handler"),
        );
        self.contradictions.push(contradiction);
        id
    }

    /// Detect contradictions between two hypotheses by comparing their claims.
    pub fn detect_hypothesis_contradiction(&self, a: &str, b: &str) -> Option<Contradiction> {
        let ha = self.hypotheses.get(a)?;
        let hb = self.hypotheses.get(b)?;
        let party_a = ContradictionParty::new(ha.statement.clone(), "hypothesis")
            .with_confidence(ha.confidence);
        let party_b = ContradictionParty::new(hb.statement.clone(), "hypothesis")
            .with_confidence(hb.confidence);
        Some(Contradiction::new(party_a, party_b))
    }

    // ── Typed Graph Edges ──────────────────────────────────────────────

    /// Add a typed relationship edge to the context graph.
    pub fn add_edge(&mut self, edge: TypedEdge) {
        self.edges.push(edge);
    }

    /// Find all typed edges where `entity_id` is source or target.
    pub fn edges_for(&self, entity_id: &str) -> Vec<&TypedEdge> {
        self.edges
            .iter()
            .filter(|e| e.source_id == entity_id || e.target_id == entity_id)
            .collect()
    }

    // ── Process Cycles ─────────────────────────────────────────────────

    /// Register a process cycle.
    pub fn record_process_cycle(&mut self, cycle: ProcessCycle) {
        self.process_cycles.push(cycle);
    }

    // ── Epistemic Audit & Reconstruction ───────────────────────────────

    /// Record an arbitrary epistemic event.
    pub fn record_epistemic_event(&mut self, event: EpistemicEvent) {
        self.epistemic_audit.record(event);
    }

    /// Reconstruct belief state about `subject_id` at instant `when`.
    pub fn reconstruct_belief_at(
        &self,
        subject_id: &str,
        when: chrono::DateTime<chrono::Utc>,
    ) -> Option<HypothesisStatus> {
        self.epistemic_audit.reconstruct_status_at(subject_id, when)
    }

    // ── Provenance & Explanation ───────────────────────────────────────

    /// Get or create a provenance chain for a subject.
    pub fn provenance_chain(&mut self, subject: impl Into<String>) -> &mut ProvenanceChain {
        let key = subject.into();
        self.provenance_chains.entry(key).or_default()
    }

    /// Generate a fully structured explanation report for a hypothesis.
    pub fn full_explanation_report(&self, id: &str) -> Option<ExplanationReport> {
        let h = self.hypotheses.get(id)?;
        let prov = self.provenance_chains.get(id).cloned().unwrap_or_default();

        let mut precedents = Vec::new();
        for node in self.nodes.values() {
            if let Some(verdict) = node.asserted {
                let sim = cosine_sim(&h.embedding, &node.embedding);
                if sim > 0.6 {
                    precedents.push(HistoricalPrecedent {
                        case_id: node.id.clone(),
                        description: node
                            .label
                            .clone()
                            .unwrap_or_else(|| "unlabeled case".to_string()),
                        outcome_worked: verdict > 0.0,
                        similarity: sim,
                        context_match: None,
                    });
                }
            }
        }

        let report = ExplanationReport {
            subject_id: h.id.clone(),
            statement: h.statement.clone(),
            status: h.status,
            supporting_evidence: h.supporting_evidence.clone(),
            contradicting_evidence: h.contradicting_evidence.clone(),
            historical_precedents: precedents,
            expected_consequences: h.predictions.clone(),
            observed_consequences: h
                .predictions
                .iter()
                .filter(|p| p.actual_outcome.is_some())
                .cloned()
                .collect(),
            coherence_profile: h.coherence_profile.clone(),
            coherence_score: h.coherence,
            fitness_breakdown: h.fitness_breakdown.clone(),
            fitness_score: h.fitness,
            confidence: h.confidence,
            provenance_chain: prov,
            human_readable_summary: String::new(),
        };

        let rendered = report.render_ascii();
        let mut final_report = report;
        final_report.human_readable_summary = rendered;
        Some(final_report)
    }

    /// String summary explanation of a hypothesis.
    pub fn explain_hypothesis(&self, id: &str) -> Option<String> {
        self.full_explanation_report(id)
            .map(|r| r.human_readable_summary)
    }

    // ── Upgraded Dream Loop ────────────────────────────────────────────

    /// Dream over unresolved failures: generate candidate hypotheses that could
    /// restore coherence, rank candidates by multi-dimensional coherence, and
    /// retain them as candidates (never silently certifying them).
    pub fn dream_hypotheses(&mut self) -> Vec<Hypothesis> {
        let failures: Vec<(String, Option<String>, Vec<f32>, Score)> = self
            .asserted_failures()
            .iter()
            .map(|f| {
                (
                    f.id.clone(),
                    f.label.clone(),
                    f.embedding.clone(),
                    f.coherence_score,
                )
            })
            .collect();

        let mut candidates = Vec::new();

        for (fid, flabel, femb, fcoh) in failures {
            let statement = format!(
                "Candidate hypothesis explaining failure in node: {}",
                flabel.as_deref().unwrap_or(&fid)
            );
            let mut hyp = Hypothesis::new(statement, femb);
            hyp.status = HypothesisStatus::Candidate;
            hyp.coherence_profile.semantic.score = fcoh;
            hyp.coherence_profile.empirical.score = 0.2; // Reflects failure observation
            hyp.coherence = hyp.coherence_profile.composite();
            hyp.recompute_fitness();

            self.epistemic_audit.record(
                EpistemicEvent::new(
                    EpistemicEventType::HypothesisGenerated,
                    &hyp.id,
                    format!(
                        "Dream loop generated candidate explanation for failure in {}",
                        fid
                    ),
                )
                .with_source("dream_loop")
                .with_metric(hyp.fitness),
            );

            self.hypotheses.insert(hyp.id.clone(), hyp.clone());
            candidates.push(hyp);
        }

        candidates
    }

    // ── Coherence Query API Execution ──────────────────────────────────

    /// Execute a structured epistemic query against Physis.
    pub fn query_coherence(&self, query: &EpistemicQuery) -> EpistemicQueryResult {
        match query {
            EpistemicQuery::WhyIsBelieved { id } => {
                if let Some(rep) = self.full_explanation_report(id) {
                    EpistemicQueryResult::Explanation(Box::new(rep))
                } else {
                    EpistemicQueryResult::Explanation(Box::new(ExplanationReport {
                        subject_id: id.clone(),
                        statement: "Subject not found".to_string(),
                        status: HypothesisStatus::Isolated,
                        supporting_evidence: Vec::new(),
                        contradicting_evidence: Vec::new(),
                        historical_precedents: Vec::new(),
                        expected_consequences: Vec::new(),
                        observed_consequences: Vec::new(),
                        coherence_profile: CoherenceProfile::new(),
                        coherence_score: 0.0,
                        fitness_breakdown: Default::default(),
                        fitness_score: 0.0,
                        confidence: 0.0,
                        provenance_chain: Default::default(),
                        human_readable_summary: "Subject not found".to_string(),
                    }))
                }
            }
            EpistemicQuery::WhatContradicts { id } => {
                let contradicting_claims: Vec<String> = self
                    .contradictions
                    .iter()
                    .filter(|c| c.claim_a.claim.contains(id) || c.claim_b.claim.contains(id))
                    .map(|c| format!("A: {} vs B: {}", c.claim_a.claim, c.claim_b.claim))
                    .collect();
                let conflicting_edges: Vec<TypedEdge> = self
                    .edges
                    .iter()
                    .filter(|e| {
                        (e.source_id == *id || e.target_id == *id)
                            && e.relation_type == RelationType::Contradicts
                    })
                    .cloned()
                    .collect();
                EpistemicQueryResult::Contradictions {
                    subject_id: id.clone(),
                    contradicting_claims,
                    conflicting_edges,
                }
            }
            EpistemicQuery::WhatHappenedAfterPrediction { id } => {
                let preds = self
                    .hypotheses
                    .get(id)
                    .map(|h| h.predictions.clone())
                    .unwrap_or_default();
                EpistemicQueryResult::PredictionsOutcome {
                    subject_id: id.clone(),
                    predictions: preds,
                }
            }
            EpistemicQuery::SimilarCasesSucceeded { embedding, limit } => {
                let mut matches: Vec<(String, Score, bool)> = self
                    .nodes
                    .values()
                    .filter_map(|n| {
                        if let Some(v) = n.asserted {
                            if v > 0.0 {
                                let sim = cosine_sim(embedding, &n.embedding);
                                return Some((
                                    n.label.clone().unwrap_or_else(|| n.id.clone()),
                                    sim,
                                    true,
                                ));
                            }
                        }
                        None
                    })
                    .collect();
                matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                matches.truncate(*limit);
                EpistemicQueryResult::SimilarCases { cases: matches }
            }
            EpistemicQuery::SimilarCasesFailed { embedding, limit } => {
                let mut matches: Vec<(String, Score, bool)> = self
                    .nodes
                    .values()
                    .filter_map(|n| {
                        if let Some(v) = n.asserted {
                            if v < 0.0 {
                                let sim = cosine_sim(embedding, &n.embedding);
                                return Some((
                                    n.label.clone().unwrap_or_else(|| n.id.clone()),
                                    sim,
                                    false,
                                ));
                            }
                        }
                        None
                    })
                    .collect();
                matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                matches.truncate(*limit);
                EpistemicQueryResult::SimilarCases { cases: matches }
            }
            EpistemicQuery::WhyChangedInterpretation { id } => {
                let (status, revs) = self
                    .hypotheses
                    .get(id)
                    .map(|h| (h.status, h.revision_history.clone()))
                    .unwrap_or((HypothesisStatus::Isolated, Vec::new()));
                EpistemicQueryResult::RevisionHistory {
                    subject_id: id.clone(),
                    current_status: status,
                    revisions: revs,
                }
            }
            EpistemicQuery::InsufficientOntologyConcepts => {
                let gaps: Vec<String> = self
                    .isolated_branches
                    .iter()
                    .flat_map(|b| b.node_ids.clone())
                    .collect();
                EpistemicQueryResult::OntologyGaps {
                    uncovered_cells: gaps,
                    candidate_domains: Vec::new(),
                }
            }
            EpistemicQuery::StrongestHypotheses { limit } => {
                let mut hyps: Vec<HypothesisSummary> = self
                    .hypotheses
                    .values()
                    .map(|h| HypothesisSummary {
                        id: h.id.clone(),
                        statement: h.statement.clone(),
                        status: h.status,
                        fitness: h.fitness,
                        coherence: h.coherence,
                        confidence: h.confidence,
                        evidence_count: h.evidence_count(),
                        predictions_count: h.predictions.len(),
                    })
                    .collect();
                hyps.sort_by(|a, b| {
                    b.fitness
                        .partial_cmp(&a.fitness)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                hyps.truncate(*limit);
                EpistemicQueryResult::RankedHypotheses { hypotheses: hyps }
            }
            EpistemicQuery::FailedPredictions { limit } => {
                let mut failed_preds = Vec::new();
                for h in self.hypotheses.values() {
                    for p in &h.predictions {
                        if p.correct == Some(false) {
                            failed_preds.push(FailedPredictionSummary {
                                hypothesis_id: h.id.clone(),
                                hypothesis_statement: h.statement.clone(),
                                prediction_statement: p.statement.clone(),
                                expected_outcome: p.expected_outcome.clone(),
                                actual_outcome: p.actual_outcome.clone(),
                                made_at: p.made_at,
                                observed_at: p.observed_at,
                            });
                        }
                    }
                }
                failed_preds.truncate(*limit);
                EpistemicQueryResult::FailedPredictionsList {
                    failures: failed_preds,
                }
            }
            EpistemicQuery::HighestEmpiricalFitness { limit } => {
                let mut hyps: Vec<HypothesisSummary> = self
                    .hypotheses
                    .values()
                    .map(|h| HypothesisSummary {
                        id: h.id.clone(),
                        statement: h.statement.clone(),
                        status: h.status,
                        fitness: h.fitness_breakdown.empirical_support,
                        coherence: h.coherence,
                        confidence: h.confidence,
                        evidence_count: h.evidence_count(),
                        predictions_count: h.predictions.len(),
                    })
                    .collect();
                hyps.sort_by(|a, b| {
                    b.fitness
                        .partial_cmp(&a.fitness)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                hyps.truncate(*limit);
                EpistemicQueryResult::RankedHypotheses { hypotheses: hyps }
            }
            EpistemicQuery::WhySupported {
                id,
                recent_event_limit,
            } => EpistemicQueryResult::SupportSnapshot {
                subject_id: id.clone(),
                snapshot: self.support_snapshot(id, *recent_event_limit).map(Box::new),
            },
            EpistemicQuery::WhatIsAffectedBy { node_id } => EpistemicQueryResult::AffectedSet {
                explanation: self.affected_set(node_id),
            },
        }
    }
    /// Deterministic "why supported / affected now" snapshot for a hypothesis
    /// (evidence/provenance intake citations, DependsOn-affected dependents,
    /// recent audit events). Read-only diagnostic, not a verdict.
    pub fn support_snapshot(
        &self,
        hypothesis_id: &str,
        recent_event_limit: usize,
    ) -> Option<crate::support::SupportSnapshot> {
        crate::support::explain_support_snapshot(self, hypothesis_id, recent_event_limit)
    }

    /// Append-only intake of observer candidate claims (never verified truth):
    /// advances per-stream watermarks and records `ObservationIngested` audit
    /// events citing intake/assertion/extractor/source. Creates no hypotheses,
    /// edges, or contradictions — gate proposals explicitly.
    pub fn ingest_observer_claims(
        &mut self,
        claims: &[crate::ingest::CandidateClaimPacket],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> Vec<crate::epistemic::IntakeReceipt> {
        crate::ingest::record_candidate_intake_batch(self, claims, stream_id, arrival)
    }

    /// DependsOn-affected hypothesis set for a node (read-only diagnostic).
    pub fn affected_set(&self, node_id: &str) -> crate::revision::AffectedSetExplanation {
        let nodes: Vec<_> = self.nodes.values().cloned().collect();
        let hyps: Vec<_> = self.hypotheses.values().cloned().collect();
        crate::revision::explain_affected_set(&nodes, &hyps, &self.edges, node_id)
    }
    /// Append-only intake plus gate assessment for observer claims: records what
    /// was asserted (audit/watermarks) and returns proposals/decisions, but
    /// registers no hypotheses/edges/contradictions and never certifies/merges.
    pub fn assess_observer_claims(
        &mut self,
        claims: &[crate::ingest::CandidateClaimPacket],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> Vec<crate::gate::AssessedIntake> {
        crate::gate::assess_claims_with_intake(self, claims, known_labels, stream_id, arrival)
    }

    /// Append-only intake from an interchange envelope (candidate-only, never
    /// verified truth). Convenience over [`Self::ingest_observer_claims`]; no
    /// hypotheses, edges, or contradictions are registered.
    pub fn ingest_envelope(
        &mut self,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> Vec<crate::epistemic::IntakeReceipt> {
        self.ingest_observer_claims(&envelope.claims, stream_id, arrival)
    }

    /// Append-only intake plus gate assessment from an interchange envelope.
    /// Records what was asserted and returns proposals/decisions only — no
    /// hypotheses/edges/contradictions registered, never certifies/merges.
    pub fn assess_envelope(
        &mut self,
        envelope: &crate::ingest::CandidateClaimEnvelope,
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> Vec<crate::gate::AssessedIntake> {
        self.assess_observer_claims(&envelope.claims, known_labels, stream_id, arrival)
    }

    /// Observe inputs via a semantic observer, then append-only intake + gate
    /// assessment (records what was asserted and returns proposals/decisions).
    /// Registers no hypotheses/edges/contradictions and never certifies/merges.
    /// Candidate-only: caller decides whether to record proposals explicitly.
    pub fn observe_and_assess(
        &mut self,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        known_labels: &[String],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Vec<crate::gate::AssessedIntake>> {
        let claims = observer.ingest(inputs)?;
        Ok(self.assess_observer_claims(&claims, known_labels, stream_id, arrival))
    }

    /// Observe inputs via a semantic observer, then append-only intake only
    /// (records what was asserted: audit/watermarks). Registers no hypotheses,
    /// edges, or contradictions and performs no gate assessment — use
    /// [`Self::observe_and_assess`] or [`Self::assess_observer_claims`] when you
    /// also want candidate proposals/decisions.
    pub fn observe_and_ingest(
        &mut self,
        observer: &dyn crate::observer::SemanticObserver,
        inputs: &[crate::observer::ObserverInput],
        stream_id: &str,
        arrival: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Vec<crate::epistemic::IntakeReceipt>> {
        let claims = observer.ingest(inputs)?;
        Ok(self.ingest_observer_claims(&claims, stream_id, arrival))
    }
}

#[cfg(test)]
mod ph017_tests {
    use super::*;
    use crate::hypothesis::{Evidence, Hypothesis};

    /// The invariant the whole packet exists for: what `replay` reconstructs
    /// must equal what `list` reports. This is the assertion that would have
    /// caught the defect years earlier.
    #[test]
    fn replay_agrees_with_the_derived_status() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("under test", vec![]);
        h.add_supporting_evidence(Evidence::supports("a", "held"));
        let id = h.id.clone();
        let derived = h.status;
        core.hypotheses.insert(id.clone(), h);

        core.project_all_revisions();

        let replayed = core
            .epistemic_audit
            .reconstruct_status_at(&id, chrono::Utc::now());
        assert_eq!(
            replayed,
            Some(derived),
            "replay must reconstruct the same standing `list` derives"
        );
    }

    /// A transition must be replayable AT THE TIME IT HAPPENED, not at the time
    /// it was projected — otherwise `replay --at T` answers with a belief the
    /// machine did not hold at T.
    #[test]
    fn projection_preserves_the_revisions_own_clock() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("timed", vec![]);
        h.add_supporting_evidence(Evidence::supports("a", "held"));
        let revision_time = h.revision_history.last().unwrap().timestamp;
        let id = h.id.clone();
        core.hypotheses.insert(id.clone(), h);
        core.project_all_revisions();

        let ev = core
            .epistemic_audit
            .events
            .iter()
            .find(|e| e.subject_id == id)
            .expect("the transition must be projected");
        assert_eq!(
            ev.timestamp, revision_time,
            "the event keeps the revision's clock"
        );
    }

    /// Idempotent: the projection is a view, and calling it twice must not
    /// duplicate history.
    #[test]
    fn projecting_twice_records_nothing_the_second_time() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("once", vec![]);
        h.add_supporting_evidence(Evidence::supports("a", "held"));
        core.hypotheses.insert(h.id.clone(), h);

        let first = core.project_all_revisions();
        assert!(first >= 1, "something must be projected the first time");
        assert_eq!(core.project_all_revisions(), 0, "and nothing the second");
    }

    /// The log records transitions, not heartbeats. A revision that did not
    /// cross a status boundary is audited on the hypothesis but is not a
    /// transition, and must not enter the trail.
    #[test]
    fn a_revision_that_changed_nothing_is_not_projected() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("steady", vec![]);
        h.add_supporting_evidence(Evidence::supports("a", "one")); // Candidate -> Supported
        h.add_supporting_evidence(Evidence::supports("b", "two")); // Supported -> Supported
                                                                   // Three revisions exist: "Created" (Candidate->Candidate, construction
                                                                   // rather than a transition), the real Candidate->Supported move, and a
                                                                   // second Supported->Supported that crossed nothing.
        assert_eq!(
            h.revision_history.len(),
            3,
            "all three are audited on the hypothesis"
        );
        assert_eq!(
            h.revision_history
                .iter()
                .filter(|r| r.previous_status != r.new_status)
                .count(),
            1,
            "but only one of them is a transition"
        );
        core.hypotheses.insert(h.id.clone(), h);

        assert_eq!(
            core.project_all_revisions(),
            1,
            "but only the real transition reaches the trail"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contradiction::ResolutionStatus;
    use crate::embed::RandomProjectionEmbedder;
    use crate::hypothesis::{Evidence, EvidencePolarity, Hypothesis, HypothesisStatus};
    use crate::provenance::ProvenanceLink;

    fn fixture_embedder() -> RandomProjectionEmbedder {
        RandomProjectionEmbedder::new(64)
    }

    fn fixture_core() -> PhysisCore {
        let mut g = PhysisCore::new();
        let emb = fixture_embedder();
        g.register_node_from_text("exercise running success", &emb);
        g.register_node_from_text("diet no sugar success", &emb);
        g.register_node_from_text("compile physis core", &emb);
        g
    }

    #[test]
    fn register_and_search() {
        let g = fixture_core();
        let emb = fixture_embedder();
        let q = emb.embed("exercise running success");
        let hits = g.search_nodes(&q, 3);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].1, "exercise running success");
        assert!(hits[0].2 >= hits[1].2 && hits[1].2 >= hits[2].2);
    }

    #[test]
    fn labeled_registration_is_deduped() {
        let mut g = PhysisCore::new();
        let emb = fixture_embedder();
        let id1 = g.register_node_from_text("src/main.rs", &emb);
        let id2 = g.register_node_from_text("src/main.rs", &emb);
        assert_eq!(id1, id2);
        assert_eq!(g.nodes.len(), 1);
        g.register_node_vec(vec![0.1; 64]);
        g.register_node_vec(vec![0.1; 64]);
        assert_eq!(g.nodes.len(), 3);
    }

    #[test]
    fn coherence_index_and_snapshot() {
        let g = fixture_core();
        let idx = g.coherence_index();
        assert!((0.0..=1.0).contains(&idx));
        let snap = g.snapshot();
        assert_eq!(snap.total_nodes, 3);
    }

    #[test]
    fn asserted_axis_is_separate_from_density() {
        let mut g = PhysisCore::new();
        let emb = fixture_embedder();
        let a = g.register_node_from_text("deploy script that always works", &emb);
        let b = g.register_node_from_text("deploy script that always works, but fails", &emb);

        assert_eq!(g.nodes[&a].rating(), None);
        assert_eq!(g.asserted_index(), None);

        let density_before = g.nodes[&b].coherence_score;
        assert!(g.assert_coherence(&b, -1.0));
        assert_eq!(g.nodes[&b].rating(), Some(CoherenceRating::Failure));
        assert_eq!(g.nodes[&b].coherence_score, density_before);
        assert_eq!(g.asserted_index(), Some(-1.0));

        g.assert_coherence(&a, 1.0);
        assert_eq!(g.asserted_index(), Some(0.0));
        g.assert_coherence(&a, 9.0);
        assert_eq!(g.nodes[&a].asserted, Some(1.0));
        assert!(!g.assert_coherence("no-such-node", 1.0));
    }

    #[test]
    fn dream_replays_asserted_failures_only() {
        let mut g = PhysisCore::new();
        let emb = fixture_embedder();
        let failed = g.register_node_from_text("migration that corrupted the index", &emb);
        let worked = g.register_node_from_text("migration that corrupted the index, fixed", &emb);

        g.assert_coherence(&failed, -1.0);
        g.assert_coherence(&worked, 1.0);
        g.nodes.get_mut(&failed).unwrap().coherence_score = 0.9;
        g.nodes.get_mut(&worked).unwrap().coherence_score = 0.1;

        let dreams = g.dream();
        let tested: Vec<&String> = dreams.iter().flat_map(|d| &d.nodes_tested).collect();
        assert!(tested.contains(&&failed));
        assert!(!tested.contains(&&worked));
    }

    #[test]
    fn dream_hypotheses_generates_candidates_not_certified() {
        let mut g = PhysisCore::new();
        let emb = fixture_embedder();
        let failed = g.register_node_from_text("extruder motor stall during ramp", &emb);
        g.assert_coherence(&failed, -1.0);

        let candidate_hyps = g.dream_hypotheses();
        assert_eq!(candidate_hyps.len(), 1);
        assert_eq!(candidate_hyps[0].status, HypothesisStatus::Candidate);
        assert!(candidate_hyps[0].statement.contains("extruder motor stall"));
    }

    #[test]
    fn nodes_survive_json_roundtrip() {
        let mut g = PhysisCore::new();
        g.set_embedder_id("fixture");
        let emb = fixture_embedder();
        let id = g.register_node_from_text("morning yoga completed", &emb);
        g.assert_coherence(&id, -1.0);
        let json = g.to_json().unwrap();

        let restored = PhysisCore::from_json(&json).unwrap();
        assert_eq!(restored.nodes.len(), 1);
        assert_eq!(restored.nodes[&id].asserted, Some(-1.0));
        let mut again = restored;
        let id2 = again.register_node_from_text("morning yoga completed", &emb);
        assert_eq!(id2, id);
        assert_eq!(again.nodes.len(), 1);
    }

    /// `to_json` serialized ONLY `nodes`, so every save silently discarded the
    /// hypotheses, contradictions and audit trail. The CLI printed "Registered
    /// hypothesis [id]" and the next `hypothesis list` said zero — the headline
    /// truth-maintenance and audit features could not survive a process exit.
    #[test]
    fn hypotheses_contradictions_and_audit_survive_a_save() {
        let emb = fixture_embedder();
        let mut g = PhysisCore::new();
        let node = g.register_node_from_text("nozzle clogged mid-print", &emb);

        let hyp = Hypothesis::new("clogs correlate with humidity", emb.embed("humidity"));
        let hid = g.register_hypothesis(hyp);
        let audit_len = g.epistemic_audit.events.len();
        assert!(audit_len > 0, "registering must record an epistemic event");

        let restored = PhysisCore::from_json(&g.to_json().unwrap()).unwrap();
        assert_eq!(restored.nodes.len(), 1, "nodes still round-trip");
        assert!(restored.nodes.contains_key(&node));
        assert_eq!(
            restored.hypotheses.len(),
            1,
            "hypotheses must survive the save"
        );
        assert_eq!(
            restored.hypotheses[&hid].statement,
            "clogs correlate with humidity",
        );
        assert_eq!(
            restored.epistemic_audit.events.len(),
            audit_len,
            "the audit trail is the replay source and must persist in full",
        );
    }

    /// Existing `~/.physis-core/nodes.json` files are a bare array. Reading one
    /// must keep working, or upgrading silently wipes a user's corpus.
    #[test]
    fn the_legacy_bare_array_format_still_loads() {
        let emb = fixture_embedder();
        let mut g = PhysisCore::new();
        let id = g.register_node_from_text("legacy node", &emb);
        let legacy = serde_json::to_string(&g.nodes.values().collect::<Vec<_>>()).unwrap();
        assert!(
            legacy.trim_start().starts_with('['),
            "fixture must be the old shape"
        );

        let restored = PhysisCore::from_json(&legacy).unwrap();
        assert_eq!(restored.nodes.len(), 1);
        assert!(restored.nodes.contains_key(&id));
        assert!(
            restored.hypotheses.is_empty(),
            "legacy files carry no hypotheses"
        );

        // And a legacy file re-saved comes back in the new format with its
        // labels still deduping, so the upgrade is a one-way ratchet.
        let again = PhysisCore::from_json(&restored.to_json().unwrap()).unwrap();
        assert_eq!(again.nodes.len(), 1);
    }

    #[test]
    fn certify_and_contradict() {
        let mut g = fixture_core();
        g.certify_branches();
        g.detect_contradictions();
        assert!(g.certified_branches.len() + g.isolated_branches.len() > 0 || g.nodes.len() == 3);
    }

    #[test]
    fn epistemic_audit_reconstruction() {
        let mut g = PhysisCore::new();
        let emb = fixture_embedder();
        let v = emb.embed("nozzle temperature nominal");
        let hyp = Hypothesis::new("Nozzle temperature nominal", v);
        let id = g.register_hypothesis(hyp.clone());

        let t0 = chrono::Utc::now();
        g.transition_hypothesis(
            &id,
            HypothesisStatus::Supported,
            "Sensor reading confirms",
            None,
        );
        let t1 = chrono::Utc::now();
        g.transition_hypothesis(
            &id,
            HypothesisStatus::Contradicted,
            "Thermal camera indicates freeze",
            None,
        );

        assert_eq!(
            g.reconstruct_belief_at(&id, t0),
            Some(HypothesisStatus::Candidate)
        );
        assert_eq!(
            g.reconstruct_belief_at(&id, t1),
            Some(HypothesisStatus::Supported)
        );
        assert_eq!(
            g.reconstruct_belief_at(&id, chrono::Utc::now()),
            Some(HypothesisStatus::Contradicted)
        );
    }

    #[test]
    fn hypothesis_lifecycle() {
        let mut core = PhysisCore::new();
        let h = Hypothesis::new("the machine is overheating", vec![0.1; 64]);
        let id = core.register_hypothesis(h);
        assert!(core.hypotheses.contains_key(&id));

        core.transition_hypothesis(&id, HypothesisStatus::Supported, "evidence found", None);
        assert_eq!(core.hypotheses[&id].status, HypothesisStatus::Supported);
        assert_eq!(core.hypotheses[&id].revision_history.len(), 2);
    }

    #[test]
    fn hypothesis_evidence_and_fitness() {
        let mut core = PhysisCore::new();
        let h = Hypothesis::new("temperature exceeds threshold", vec![0.2; 64]);
        let id = core.register_hypothesis(h);

        let evidence = Evidence {
            source: "sensor_01".to_string(),
            polarity: EvidencePolarity::Supports,
            confidence: 0.9,
            claim: "reading 218.4C".to_string(),
            observed_at: Some(chrono::Utc::now()),
            embedding: Vec::new(),
            context: Vec::new(),
            intake_id: None,
        };
        core.hypothesis_mut(&id)
            .unwrap()
            .add_supporting_evidence(evidence);
        assert_eq!(core.hypotheses[&id].supporting_evidence.len(), 1);
    }

    #[test]
    fn contradiction_is_first_class() {
        let mut core = PhysisCore::new();
        let ha = Hypothesis::new("pressure is normal", vec![0.1; 64]);
        let hb = Hypothesis::new("pressure is high", vec![0.2; 64]);
        let id_a = core.register_hypothesis(ha);
        let id_b = core.register_hypothesis(hb);

        let contradiction = core.detect_hypothesis_contradiction(&id_a, &id_b);
        assert!(contradiction.is_some());
        let c = contradiction.unwrap();
        assert_eq!(c.resolution, ResolutionStatus::Open);

        core.record_contradiction(c);
        assert_eq!(core.contradictions.len(), 1);
    }

    #[test]
    fn provenance_chain_is_traceable() {
        let mut core = PhysisCore::new();
        let h = Hypothesis::new("sensor reads 218.4C", vec![0.1; 64]);
        let id = core.register_hypothesis(h);

        let chain = core.provenance_chain(&id);
        chain.add_link(ProvenanceLink::new("sensor_01 reports 218.4C", "sensor_01"));
        chain.add_link(ProvenanceLink::new("threshold is 200C", "spec_sheet"));

        let explanation = core.explain_hypothesis(&id);
        assert!(explanation.is_some(), "explanation should exist");
        let text = explanation.unwrap();
        assert!(
            text.contains("spec_sheet"),
            "provenance should be visible: {}",
            text
        );
        assert!(
            text.contains("threshold is 200C"),
            "provenance should be visible: {}",
            text
        );
    }

    #[test]
    fn explain_returns_structured_output() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("process is stable", vec![0.3; 64]);
        h.confidence = 0.8;
        h.coherence = 0.9;
        h.add_supporting_evidence(Evidence {
            source: "log".to_string(),
            polarity: EvidencePolarity::Supports,
            confidence: 0.9,
            claim: "no alarms".to_string(),
            observed_at: Some(chrono::Utc::now()),
            embedding: Vec::new(),
            context: Vec::new(),
            intake_id: None,
        });
        h.recompute_fitness();
        let id = core.register_hypothesis(h);

        let explanation = core.explain_hypothesis(&id).unwrap();
        assert!(explanation.contains("process is stable"));
        assert!(explanation.contains("Confidence: 0.80"));
        assert!(explanation.contains("log"));
        assert!(explanation.contains("Fitness:"));
    }

    #[test]
    fn query_coherence_support_and_affected_surfaces() {
        use crate::coherence_query::{EpistemicQuery, EpistemicQueryResult};
        use crate::relation::{RelationType, TypedEdge};

        let mut core = PhysisCore::new();
        let a = core.register_node_vec(vec![1.0, 0.0]);
        let b = core.register_node_vec(vec![0.0, 1.0]);
        core.add_edge(TypedEdge::new(
            RelationType::DependsOn,
            a.clone(),
            b.clone(),
        ));

        let mut h = Hypothesis::new("subject", vec![]);
        h.ontology_refs.push(b.clone());
        h.add_supporting_evidence(Evidence::supports("s1", "ok").with_intake_id("i1"));
        let hid = core.register_hypothesis(h);

        let mut dep = Hypothesis::new("dependent", vec![]);
        dep.ontology_refs.push(a.clone());
        let dep_id = core.register_hypothesis(dep);

        match core.query_coherence(&EpistemicQuery::WhySupported {
            id: hid.clone(),
            recent_event_limit: 10,
        }) {
            EpistemicQueryResult::SupportSnapshot {
                subject_id,
                snapshot,
            } => {
                assert_eq!(subject_id, hid);
                let snap = snapshot.expect("snapshot");
                assert_eq!(snap.evidence_intake_ids, vec!["i1".to_string()]);
                assert!(snap.dependent_hypothesis_ids.contains(&dep_id));
            }
            other => panic!("unexpected result: {:?}", other),
        }

        match core.query_coherence(&EpistemicQuery::WhatIsAffectedBy { node_id: b }) {
            EpistemicQueryResult::AffectedSet { explanation } => {
                assert!(explanation.affected_hypothesis_ids.contains(&hid));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn assess_observer_claims_records_intake_without_registration() {
        use crate::ingest::{
            CandidateClaimPacket, ClaimPayload, ClaimSource, ExtractorInfo, ProposedRelation,
        };
        use crate::relation::RelationType;

        let mut core = PhysisCore::new();
        let rel = ProposedRelation {
            subject: "x".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "y".into(),
            fact: None,
        };
        let claim = CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i1".into()),
            source: ClaimSource {
                uri: "d".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("obs"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(0.5),
            payload: ClaimPayload::Relation(rel),
            lossy_fields: vec![],
            extras: Default::default(),
        }
        .normalize();

        let before_h = core.hypotheses.len();
        let before_e = core.edges.len();
        let before_ev = core.epistemic_audit.events.len();
        let assessed = core.assess_observer_claims(&[claim], &[], "obs", chrono::Utc::now());
        assert_eq!(assessed.len(), 1);
        assert_eq!(core.hypotheses.len(), before_h);
        assert_eq!(core.edges.len(), before_e);
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 1);
        assert_eq!(
            assessed[0].outcome.decision(),
            crate::gate::GateDecision::AcceptCandidate
        );
    }

    #[test]
    fn observe_and_assess_intakes_and_returns_proposals_without_registration() {
        use crate::ingest::{
            CandidateClaimPacket, ClaimPayload, ClaimSource, EntityMention, ExtractorInfo,
        };
        use crate::observer::{MockObserver, ObserverInput};

        let mut core = PhysisCore::new();
        let pkt = CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i1".into()),
            source: ClaimSource {
                uri: "doc://x".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("mock"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(0.5),
            payload: ClaimPayload::Entity(EntityMention {
                label: "cache".into(),
                refers_to: None,
                concept: None,
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        };
        let obs = MockObserver::new("mock").with_fixed_packets(vec![pkt]);
        let inputs = vec![ObserverInput {
            source: "doc://x".into(),
            content: "cache".into(),
            kind: None,
        }];
        let before_h = core.hypotheses.len();
        let before_e = core.edges.len();
        let before_ev = core.epistemic_audit.events.len();
        let assessed = core
            .observe_and_assess(&obs, &inputs, &[], "mock-stream", chrono::Utc::now())
            .unwrap();
        assert_eq!(assessed.len(), 1);
        assert_eq!(core.hypotheses.len(), before_h);
        assert_eq!(core.edges.len(), before_e);
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 1);
        // Conservative gate: entity claim with provenance is accepted as a candidate
        // summary (gate_claim) — never truth, never registration.
        assert_eq!(
            assessed[0].outcome.decision(),
            crate::gate::GateDecision::AcceptCandidate
        );
        assert!(core.contradictions.is_empty());
    }

    #[test]
    fn observe_and_ingest_records_intake_without_assessment_registration() {
        use crate::ingest::{
            CandidateClaimPacket, ClaimPayload, ClaimSource, EntityMention, ExtractorInfo,
        };
        use crate::observer::{MockObserver, ObserverInput};

        let mut core = PhysisCore::new();
        let pkt = CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i2".into()),
            source: ClaimSource {
                uri: "doc://y".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("mock"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(0.5),
            payload: ClaimPayload::Entity(EntityMention {
                label: "cache".into(),
                refers_to: None,
                concept: None,
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        };
        let obs = MockObserver::new("mock").with_fixed_packets(vec![pkt]);
        let inputs = vec![ObserverInput {
            source: "doc://y".into(),
            content: "cache".into(),
            kind: None,
        }];
        let before_h = core.hypotheses.len();
        let before_e = core.edges.len();
        let before_ev = core.epistemic_audit.events.len();
        let receipts = core
            .observe_and_ingest(&obs, &inputs, "mock-stream", chrono::Utc::now())
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(core.hypotheses.len(), before_h);
        assert_eq!(core.edges.len(), before_e);
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 1);
        assert!(core.contradictions.is_empty());
    }

    #[test]
    fn query_why_supported_unknown_returns_none_snapshot() {
        use crate::coherence_query::{EpistemicQuery, EpistemicQueryResult};
        let core = PhysisCore::new();
        match core.query_coherence(&EpistemicQuery::WhySupported {
            id: "missing".into(),
            recent_event_limit: 5,
        }) {
            EpistemicQueryResult::SupportSnapshot {
                subject_id,
                snapshot,
            } => {
                assert_eq!(subject_id, "missing");
                assert!(snapshot.is_none());
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn save_and_load_path_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("persist me", vec![]);
        h.add_supporting_evidence(Evidence::supports("s", "ok"));
        let hid = core.register_hypothesis(h);
        core.save_to_path(&path).unwrap();
        let loaded = PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(loaded.hypothesis(&hid).unwrap().statement, "persist me");
    }

    #[test]
    fn transact_and_persist_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");
        let mut core = PhysisCore::new();
        let receipt = core
            .transact_and_persist(&path, "seed", |w| {
                w.register_node_vec(vec![1.0, 0.0]);
            })
            .unwrap();
        assert!(receipt.audit_event_id.is_some());
        assert_eq!(core.nodes.len(), 1);
        let loaded = PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(loaded.nodes.len(), 1);

        core.transact("in-memory", |w| {
            w.register_node_vec(vec![0.0, 1.0]);
        })
        .unwrap();
        assert_eq!(core.nodes.len(), 2);
    }

    #[test]
    fn transact_record_acted_persists_observational_audit() {
        use crate::act::Acted;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");
        let mut core = PhysisCore::new();
        let acted = Acted {
            command: "echo hi".into(),
            exit_code: Some(0),
            duration_ms: 3,
            stdout_lines: 1,
            stderr_tail: String::new(),
            bearing: vec![],
            observed: vec![1, 2],
        };
        let before = core.epistemic_audit.events.len();
        let receipt = core
            .transact_record_acted_and_persist(&path, &acted, "record act")
            .unwrap();
        assert!(receipt.audit_event_id.is_some());
        // 2 acted observation events + 1 commit transaction event
        assert_eq!(core.epistemic_audit.events.len(), before + 3);
        assert!(core.hypotheses.is_empty());
        assert!(core.edges.is_empty());
        let loaded = PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(
            loaded.epistemic_audit.events.len(),
            core.epistemic_audit.events.len()
        );
    }

    #[test]
    fn transact_ingest_and_assess_observer_claims_persist_without_registration() {
        use crate::ingest::{
            CandidateClaimPacket, ClaimPayload, ClaimSource, ExtractorInfo, ProposedRelation,
        };
        use crate::relation::RelationType;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("core.json");
        let mut core = PhysisCore::new();

        let claim = CandidateClaimPacket {
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
            payload: ClaimPayload::Relation(ProposedRelation {
                subject: "a".into(),
                predicate: "depends_on".into(),
                relation_type: Some(RelationType::DependsOn),
                object: "b".into(),
                fact: None,
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        }
        .normalize();

        let before_h = core.hypotheses.len();
        let before_e = core.edges.len();
        let before_ev = core.epistemic_audit.events.len();

        let (commit, receipts) = core
            .transact_ingest_observer_claims_and_persist(
                &path,
                std::slice::from_ref(&claim),
                "obs-stream",
                chrono::Utc::now(),
                "ingest claims",
            )
            .unwrap();
        assert!(commit.audit_event_id.is_some());
        assert_eq!(receipts.len(), 1);
        assert_eq!(core.hypotheses.len(), before_h);
        assert_eq!(core.edges.len(), before_e);
        // 1 intake observation event + 1 commit transaction event
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 2);
        let loaded = PhysisCore::load_from_path(&path).unwrap();
        assert_eq!(
            loaded.epistemic_audit.events.len(),
            core.epistemic_audit.events.len()
        );

        // assess path: records intake + assessment but no registration
        let dir2 = tempfile::tempdir().unwrap();
        let path2 = dir2.path().join("core2.json");
        let mut core2 = PhysisCore::new();
        let before_ev2 = core2.epistemic_audit.events.len();
        let (commit2, assessed) = core2
            .transact_assess_observer_claims_and_persist(
                &path2,
                std::slice::from_ref(&claim),
                &[],
                "obs-stream",
                chrono::Utc::now(),
                "assess claims",
            )
            .unwrap();
        assert!(commit2.audit_event_id.is_some());
        assert_eq!(assessed.len(), 1);
        assert!(matches!(
            assessed[0].outcome,
            crate::gate::GateOutcome::Relation(_)
        ));
        assert_eq!(core2.hypotheses.len(), 0);
        assert_eq!(core2.edges.len(), 0);
        assert_eq!(core2.epistemic_audit.events.len(), before_ev2 + 2);
        let loaded2 = PhysisCore::load_from_path(&path2).unwrap();
        assert_eq!(
            loaded2.epistemic_audit.events.len(),
            core2.epistemic_audit.events.len()
        );
    }
}

#[cfg(test)]
mod envelope_workflow_tests {
    use super::*;
    use crate::ingest::{
        CandidateClaimEnvelope, CandidateClaimPacket, ClaimPayload, ClaimSource, ExtractorInfo,
        ProposedRelation,
    };
    use crate::relation::RelationType;

    fn sample_claim() -> CandidateClaimPacket {
        CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i-env".into()),
            source: ClaimSource {
                uri: "doc://env".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("obs"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(0.6),
            payload: ClaimPayload::Relation(ProposedRelation {
                subject: "a".into(),
                predicate: "depends_on".into(),
                relation_type: Some(RelationType::DependsOn),
                object: "b".into(),
                fact: None,
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        }
        .normalize()
    }

    #[test]
    fn ingest_and_assess_envelope_convenience_without_registration() {
        let mut env = CandidateClaimEnvelope::new("obs-env");
        env.claims.push(sample_claim());
        let env = env.normalize();

        let mut core = PhysisCore::new();
        let before_h = core.hypotheses.len();
        let before_e = core.edges.len();
        let before_ev = core.epistemic_audit.events.len();
        let receipts = core.ingest_envelope(&env, "obs-stream", chrono::Utc::now());
        assert_eq!(receipts.len(), 1);
        assert_eq!(core.hypotheses.len(), before_h);
        assert_eq!(core.edges.len(), before_e);
        assert_eq!(core.epistemic_audit.events.len(), before_ev + 1);

        let mut core2 = PhysisCore::new();
        let before_ev2 = core2.epistemic_audit.events.len();
        let assessed = core2.assess_envelope(&env, &[], "obs-stream", chrono::Utc::now());
        assert_eq!(assessed.len(), 1);
        assert!(matches!(
            assessed[0].outcome,
            crate::gate::GateOutcome::Relation(_)
        ));
        assert_eq!(core2.hypotheses.len(), 0);
        assert_eq!(core2.edges.len(), 0);
        assert_eq!(core2.contradictions.len(), 0);
        // intake recorded as part of assessment
        assert!(core2.epistemic_audit.events.len() > before_ev2);

        // in-memory staged transact variants (audited commit event, no persistence)
        let mut core3 = PhysisCore::new();
        let before_ev3 = core3.epistemic_audit.events.len();
        let (commit3, receipts3) = core3
            .transact_ingest_envelope(&env, "obs-stream", chrono::Utc::now(), "ingest env")
            .unwrap();
        assert!(commit3.audit_event_id.is_some());
        assert_eq!(receipts3.len(), 1);
        assert_eq!(core3.hypotheses.len(), 0);
        assert_eq!(core3.edges.len(), 0);
        // intake + commit transaction event
        assert_eq!(core3.epistemic_audit.events.len(), before_ev3 + 2);

        let mut core4 = PhysisCore::new();
        let before_ev4 = core4.epistemic_audit.events.len();
        let (commit4, assessed4) = core4
            .transact_assess_envelope(&env, &[], "obs-stream", chrono::Utc::now(), "assess env")
            .unwrap();
        assert!(commit4.audit_event_id.is_some());
        assert_eq!(assessed4.len(), 1);
        assert!(matches!(
            assessed4[0].outcome,
            crate::gate::GateOutcome::Relation(_)
        ));
        assert_eq!(core4.hypotheses.len(), 0);
        assert_eq!(core4.edges.len(), 0);
        assert_eq!(core4.contradictions.len(), 0);
        assert_eq!(core4.epistemic_audit.events.len(), before_ev4 + 2);

        // in-memory staged observe+transact variants (extraction before staging)
        struct FixedObserver {
            claims: Vec<CandidateClaimPacket>,
        }
        impl crate::observer::SemanticObserver for FixedObserver {
            fn observer_id(&self) -> &str {
                "fixed"
            }
            fn ingest(
                &self,
                _inputs: &[crate::observer::ObserverInput],
            ) -> anyhow::Result<Vec<CandidateClaimPacket>> {
                Ok(self.claims.clone())
            }
        }
        let observer = FixedObserver {
            claims: env.claims.clone(),
        };
        let mut core5 = PhysisCore::new();
        let before_ev5 = core5.epistemic_audit.events.len();
        let (commit5, receipts5) = core5
            .transact_observe_and_ingest(
                &observer,
                &[],
                "obs-stream",
                chrono::Utc::now(),
                "observe ingest",
            )
            .unwrap();
        assert!(commit5.audit_event_id.is_some());
        assert_eq!(receipts5.len(), 1);
        assert_eq!(core5.hypotheses.len(), 0);
        assert_eq!(core5.edges.len(), 0);
        assert_eq!(core5.epistemic_audit.events.len(), before_ev5 + 2);

        let mut core6 = PhysisCore::new();
        let before_ev6 = core6.epistemic_audit.events.len();
        let (commit6, assessed6) = core6
            .transact_observe_and_assess(
                &observer,
                &[],
                &[],
                "obs-stream",
                chrono::Utc::now(),
                "observe assess",
            )
            .unwrap();
        assert!(commit6.audit_event_id.is_some());
        assert_eq!(assessed6.len(), 1);
        assert!(matches!(
            assessed6[0].outcome,
            crate::gate::GateOutcome::Relation(_)
        ));
        assert_eq!(core6.hypotheses.len(), 0);
        assert_eq!(core6.edges.len(), 0);
        assert_eq!(core6.contradictions.len(), 0);
        assert_eq!(core6.epistemic_audit.events.len(), before_ev6 + 2);
    }
}
