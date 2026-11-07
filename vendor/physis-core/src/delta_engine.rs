//! Dynamic Ontology Delta & Propagation Reasoning Engine.
//!
//! When a single node or node property mutates, this engine simulates the
//! cascade effects on graph coherence and hypothesis statuses in an isolated
//! shadow frame — without writing to persistent storage or modifying the base
//! state vectors passed into the context. The caller inspects the
//! [`OntologyDeltaReport`] and decides whether to commit.
//!
//! ## Propagation Model
//!
//! A topological wave traverses the adjacency graph starting from the mutated
//! node. Each hop attenuates the impact by the decay factor γ (default 0.85):
//!
//! ```text
//! ΔImpact(N) = CosineSimilarity(E_old, E_new) × γ^depth
//! ```
//!
//! Only nodes within the propagation radius (depth ≤ [`MAX_PROPAGATION_DEPTH`]
//! and impact ≥ [`MIN_IMPACT`]) are evaluated and reported.
//!
//! ## Revision selection (A2, Atlas take)
//!
//! The breadth wave reports *node* coherence deltas only. Which *hypotheses*
//! are revised is selected by the declared `DependsOn` closure of the mutated
//! node ([`EvaluationContext::depends_on_walk`]) — a shared cell pin, a label
//! prefix, or a similar embedding no longer pulls a hypothesis into revision.
//! Graphs that declare no `DependsOn` edges fall back to the breadth-affected
//! selection, logged in [`RevisionWalk::fallback_breadth_used`].

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::contradiction::ResolutionStatus;
use crate::hypothesis::{Evidence, Hypothesis, HypothesisStatus};
use crate::models::{cosine_sim, CoherenceNode, Score};
use crate::relation::{RelationType, TypedEdge};

/// Decay factor γ for impact attenuation during topological propagation.
pub const GAMMA: f32 = 0.85;

/// Score-degradation threshold ϵ for hypothesis status transitions.
/// When a hypothesis's coherence drops beyond this, a Supported → Contradicted
/// transition is triggered.
pub const DEGRADATION_THRESHOLD: f32 = 0.25;

/// Maximum propagation depth in the DFS wave.
pub const MAX_PROPAGATION_DEPTH: usize = 5;

/// Minimum impact factor below which propagation stops.
pub const MIN_IMPACT: f32 = 0.01;

/// A2 (Atlas take): hard cap on visited nodes for the DependsOn revision
/// walk (Atlas `MAX_NODES_DEFAULT = 5000`). The walk truncates at the cap
/// and says so via [`RevisionWalk::truncated`] — never silently.
pub const MAX_REVISION_WALK_NODES: usize = 5000;

// ── Mutation & Shadows ───────────────────────────────────────────────────

/// A single atomic mutation operation on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationOp {
    /// Set or replace a JSON property on the node.
    PropertySet {
        key: String,
        old_val: serde_json::Value,
        new_val: serde_json::Value,
    },
    /// Shift the node's embedding vector.
    EmbeddingShift {
        old_embedding: Vec<f32>,
        new_embedding: Vec<f32>,
    },
    /// Sever an outgoing relationship edge to another node.
    RelationSevered { target_id: String },
    /// Create a new outgoing relationship edge to another node.
    RelationCreated { target_id: String, weight: f32 },
}

/// A complete mutation event targeting a specific node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyMutation {
    pub id: String,
    pub target_node_id: String,
    pub operation: MutationOp,
    pub triggered_by: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl OntologyMutation {
    /// Convenience constructor with a fresh UUID and the current timestamp.
    pub fn new(target_node_id: impl Into<String>, operation: MutationOp) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            target_node_id: target_node_id.into(),
            operation,
            triggered_by: None,
            timestamp: Utc::now(),
        }
    }

    /// Mark this mutation as being caused by another mutation or event.
    pub fn with_trigger(mut self, trigger_id: impl Into<String>) -> Self {
        self.triggered_by = Some(trigger_id.into());
        self
    }
}

/// Transient frame for isolated evaluation.
///
/// `base_nodes` and `base_hypotheses` are immutable references to the real
/// graph state — never mutated. `shadow_nodes` and `shadow_hypotheses` are
/// owned copies that receive the mutation and all propagation side-effects.
/// The caller inspects [`OntologyDeltaReport`] and decides whether to commit
/// the shadow state back to persistent storage.
pub struct EvaluationContext<'a> {
    /// The unmodified graph nodes (read-only).
    pub base_nodes: &'a [CoherenceNode],
    /// The unmodified hypotheses (read-only).
    pub base_hypotheses: &'a [Hypothesis],
    /// Typed edges that define graph adjacency (read-only).
    pub edges: &'a [TypedEdge],
    /// Shadow copies of nodes that have been touched by the mutation / wave.
    pub shadow_nodes: HashMap<String, CoherenceNode>,
    /// Shadow copies of hypotheses whose fitness has been recomputed.
    pub shadow_hypotheses: HashMap<String, Hypothesis>,
    /// Append-only log of every mutation applied in this frame.
    pub mutation_log: Vec<OntologyMutation>,
}

impl<'a> EvaluationContext<'a> {
    /// Build a fresh shadow frame from base data with empty mutation log.
    pub fn from_base(
        base_nodes: &'a [CoherenceNode],
        base_hypotheses: &'a [Hypothesis],
        edges: &'a [TypedEdge],
    ) -> Self {
        Self {
            base_nodes,
            base_hypotheses,
            edges,
            shadow_nodes: HashMap::new(),
            shadow_hypotheses: HashMap::new(),
            mutation_log: Vec::new(),
        }
    }

    // ── Effective (shadow-preferred) lookups ────────────────────────────

    /// Resolve a node ID to its effective representation: the shadow copy if
    /// one exists (mutation applied), otherwise the base reference.
    pub fn effective_node(&self, id: &str) -> Option<&CoherenceNode> {
        self.shadow_nodes
            .get(id)
            .or_else(|| self.base_nodes.iter().find(|n| n.id == id))
    }

    /// All nodes visible in this frame — shadow copies first, then base nodes
    /// not yet shadowed.
    pub fn effective_nodes_all(&self) -> Vec<&CoherenceNode> {
        let mut result: Vec<&CoherenceNode> = self.shadow_nodes.values().collect();
        for n in self.base_nodes {
            if !self.shadow_nodes.contains_key(&n.id) {
                result.push(n);
            }
        }
        result
    }

    /// Resolve a hypothesis ID to its effective representation.
    pub fn effective_hypothesis(&self, id: &str) -> Option<&Hypothesis> {
        self.shadow_hypotheses
            .get(id)
            .or_else(|| self.base_hypotheses.iter().find(|h| h.id == id))
    }

    /// All hypotheses visible in this frame.
    pub fn effective_hypotheses_all(&self) -> Vec<&Hypothesis> {
        let mut result: Vec<&Hypothesis> = self.shadow_hypotheses.values().collect();
        for h in self.base_hypotheses {
            if !self.shadow_hypotheses.contains_key(&h.id) {
                result.push(h);
            }
        }
        result
    }

    // ── Shadow management ───────────────────────────────────────────────

    /// Ensure a node exists in the shadow frame by cloning from base if needed.
    /// Returns a mutable reference to the shadow copy.
    pub fn ensure_shadowed(&mut self, node_id: &str) -> Option<&mut CoherenceNode> {
        if !self.shadow_nodes.contains_key(node_id) {
            if let Some(base) = self.base_nodes.iter().find(|n| n.id == node_id) {
                self.shadow_nodes.insert(node_id.to_string(), base.clone());
            }
        }
        self.shadow_nodes.get_mut(node_id)
    }

    /// Ensure a hypothesis exists in the shadow frame by cloning from base
    /// if needed (T3 retractions must stay isolated until committed).
    /// Returns a mutable reference to the shadow copy.
    pub fn ensure_shadowed_hypothesis(&mut self, hypothesis_id: &str) -> Option<&mut Hypothesis> {
        if !self.shadow_hypotheses.contains_key(hypothesis_id) {
            if let Some(base) = self.base_hypotheses.iter().find(|h| h.id == hypothesis_id) {
                self.shadow_hypotheses
                    .insert(hypothesis_id.to_string(), base.clone());
            }
        }
        self.shadow_hypotheses.get_mut(hypothesis_id)
    }

    // ── Coherence computation ───────────────────────────────────────────

    /// Recompute the coherence score for a node: the mean cosine similarity to
    /// its k nearest neighbours across the entire effective graph.
    pub fn compute_coherence(&self, node_id: &str) -> Score {
        let all = self.effective_nodes_all();
        if all.len() <= 1 {
            return 1.0;
        }

        let query = match self.effective_node(node_id) {
            Some(n) => &n.embedding,
            None => return 0.0,
        };

        let k = 5.min(all.len() - 1);
        let mut sims: Vec<f32> = all
            .iter()
            .filter(|n| n.id != node_id && !n.embedding.is_empty())
            .map(|n| cosine_sim(query, &n.embedding))
            .collect();

        if sims.is_empty() {
            return 1.0;
        }

        sims.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        sims.truncate(k);

        let avg = sims.iter().sum::<f32>() / sims.len() as f32;
        avg.max(0.0)
    }

    // ── Adjacency ───────────────────────────────────────────────────────

    /// Find all node IDs adjacent to `node_id` via typed edges, shared
    /// cell pins, and label-prefix links.
    pub fn find_neighbors(&self, node_id: &str) -> Vec<String> {
        let mut neighbors: HashSet<String> = HashSet::new();

        // 1. Typed edges (both directions)
        for edge in self.edges {
            if edge.source_id == node_id {
                neighbors.insert(edge.target_id.clone());
            } else if edge.target_id == node_id {
                neighbors.insert(edge.source_id.clone());
            }
        }

        // 2. Shared cell_pin — nodes pinned to the same (domain, mode) cell
        if let Some(node) = self.effective_node(node_id) {
            if let Some(ref cell_pin) = node.cell_pin {
                for n in self.base_nodes {
                    if n.id != node_id && n.cell_pin.as_ref() == Some(cell_pin) {
                        neighbors.insert(n.id.clone());
                    }
                }
                for n in self.shadow_nodes.values() {
                    if n.id != node_id && n.cell_pin.as_ref() == Some(cell_pin) {
                        neighbors.insert(n.id.clone());
                    }
                }
            }
        }

        // 3. Label links — nodes whose label starts with "parent → "
        if let Some(node) = self.effective_node(node_id) {
            if let Some(ref label) = node.label {
                let prefix = format!("{} → ", label);
                for n in self.base_nodes {
                    if n.id == node_id {
                        continue;
                    }
                    if let Some(ref nlabel) = n.label {
                        if nlabel.starts_with(&prefix) {
                            neighbors.insert(n.id.clone());
                        }
                    }
                }
            }
        }

        neighbors.into_iter().collect()
    }

    /// Pre-compute the full adjacency map for all effective nodes.
    pub fn build_adjacency(&self) -> HashMap<String, Vec<String>> {
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for n in self.effective_nodes_all() {
            adjacency.insert(n.id.clone(), self.find_neighbors(&n.id));
        }
        adjacency
    }

    /// A2 (Atlas take): BFS over *declared* `DependsOn` edges only — no cell
    /// pins, no label-prefix links, no cosine. Visited-set termination, cycle
    /// recording, hop and node caps. This is a proposal-order, not a write.
    pub fn depends_on_walk(&self, mutated_id: &str) -> RevisionWalk {
        // Reverse index: for a mutated target T, its dependents are the
        // sources S of edges S DependsOn T. Sorted for determinism.
        let mut dependents_of: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in self.edges {
            if edge.relation_type == RelationType::DependsOn {
                dependents_of
                    .entry(edge.target_id.as_str())
                    .or_default()
                    .push(edge.source_id.as_str());
            }
        }
        for list in dependents_of.values_mut() {
            list.sort_unstable();
            list.dedup();
        }

        let mut walk = RevisionWalk::default();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        visited.insert(mutated_id.to_string());
        queue.push_back((mutated_id.to_string(), 0));

        while let Some((node_id, hops)) = queue.pop_front() {
            if walk.steps.len() >= MAX_REVISION_WALK_NODES {
                walk.truncated = true;
                break;
            }
            walk.steps.push(WalkStep {
                node_id: node_id.clone(),
                hops,
            });
            if hops > 0 {
                walk.dependents.push(WalkStep {
                    node_id: node_id.clone(),
                    hops,
                });
            }
            if hops >= MAX_PROPAGATION_DEPTH {
                continue; // children would exceed the hop cap
            }
            if let Some(dependents) = dependents_of.get(node_id.as_str()) {
                for dep in dependents {
                    if visited.contains(*dep) {
                        // Cycle: record the closing edge, never re-enqueue.
                        walk.cycles.push([node_id.clone(), dep.to_string()]);
                        continue;
                    }
                    visited.insert(dep.to_string());
                    queue.push_back((dep.to_string(), hops + 1));
                }
            }
        }
        walk
    }
}

// ── Report Structures ────────────────────────────────────────────────────

/// A coherence delta for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDelta {
    pub node_id: String,
    pub previous_coherence: Score,
    pub new_coherence: Score,
    pub delta_type: String,
}

/// A status transition for a single hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisTransition {
    pub hypothesis_id: String,
    pub previous_status: HypothesisStatus,
    pub new_status: HypothesisStatus,
    pub trigger_reason: String,
}

/// Full report of the mutation evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyDeltaReport {
    pub mutation_source: OntologyMutation,
    pub affected_nodes: Vec<NodeDelta>,
    pub hypothesis_status_shifts: Vec<HypothesisTransition>,
    pub net_coherence_delta: f32,
    /// A2: the DependsOn revision walk that selected the revised hypotheses.
    /// `None` only when no walk ran (zero-shift early return).
    #[serde(default)]
    pub revision_walk: Option<RevisionWalk>,
    /// A5: routing decisions for every proposed demotion (rationale record
    /// only; no queue surface ships in P1).
    #[serde(default)]
    pub adjudications: Vec<AdjudicationDecision>,
}

/// One node visited by the A2 DependsOn revision walk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WalkStep {
    pub node_id: String,
    /// Justification hops from the mutated node (0 = the mutated node itself).
    pub hops: usize,
}

/// A2 (Atlas take): result of the DependsOn-only revision walk.
///
/// An edge `S DependsOn T` means S's standing depends on T, so dependents of
/// the mutated node M are the sources of M-incoming DependsOn edges. The walk
/// is BFS shallow-first with a visited set; proposals are not writes — the
/// caller decides whether to commit.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RevisionWalk {
    /// The mutated node (hops 0) and every dependent reached, BFS order.
    pub steps: Vec<WalkStep>,
    /// The reached dependents (hops ≥ 1), BFS order — a subset of `steps`.
    pub dependents: Vec<WalkStep>,
    /// DependsOn edges that closed back onto an already-visited node, as
    /// `[dependent_id, already_visited_id]` in encounter order.
    pub cycles: Vec<[String; 2]>,
    /// True if the walk stopped early at [`MAX_REVISION_WALK_NODES`].
    pub truncated: bool,
    /// True if the graph declared no DependsOn dependents and the engine
    /// fell back to the breadth-affected selection. Logged, never silent
    /// (A2 risk note: sparse DependsOn graphs must not go blind).
    pub fallback_breadth_used: bool,
    /// Hypothesis ids selected for revision, in revision order (walk BFS
    /// order when the walk drove the selection; breadth order on fallback).
    pub revised_hypotheses: Vec<String>,
}

/// A5 (Atlas take, `adjudication.py`): distance beyond the degradation
/// threshold over which the engine stops applying a demotion and defers to
/// review with a recorded rationale (Atlas
/// `CONFIDENCE_DELTA_STRATEGIC_FLOOR = 0.15` — same number, same meaning).
pub const ADJUDICATION_STRATEGIC_FLOOR: Score = 0.15;

/// A5: how a proposed status demotion is routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjudicationRoute {
    /// Small Δ beyond ϵ: applied as before.
    AutoApply,
    /// Large Δ: the demotion is proposed, not applied — rationale recorded,
    /// resolution stays Open until explicit human action.
    StrategicReview,
    /// Certified beliefs are core-protected: flagged, never auto-demoted
    /// (Atlas "flag without acting").
    CoreProtected,
}

/// A5 (rationale record only — P1 ships no queue surface): one routing
/// decision, recorded on the report for every proposed demotion whatever
/// the route. The approve/reject/adjust/synthesize flow is a later item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjudicationDecision {
    pub hypothesis_id: String,
    pub route: AdjudicationRoute,
    /// The status the engine would have applied. Unapplied for
    /// `StrategicReview` / `CoreProtected`.
    pub proposed_status: HypothesisStatus,
    pub coherence_delta: Score,
    /// `Open` for the review routes (recorded before any queue surface);
    /// `APreferred` when the engine resolved in favour of the observation.
    pub resolution: ResolutionStatus,
    pub rationale: String,
}

/// A5: route a proposed demotion. Certified is core-protected regardless
/// of Δ; beyond ϵ + the strategic floor the engine defers to review;
/// between ϵ and ϵ + floor it applies as before.
pub fn route_transition(
    prev_status: HypothesisStatus,
    coherence_delta: Score,
) -> AdjudicationRoute {
    if prev_status == HypothesisStatus::Certified {
        AdjudicationRoute::CoreProtected
    } else if coherence_delta > DEGRADATION_THRESHOLD + ADJUDICATION_STRATEGIC_FLOOR {
        AdjudicationRoute::StrategicReview
    } else {
        AdjudicationRoute::AutoApply
    }
}

// ── T3 (JTMS take): evidence retraction with dependent cascade ──────────

/// T3 (JTMS take): the report of an evidence retraction with its dependent
/// cascade, evaluated in the shadow frame only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetractionCascadeReport {
    pub hypothesis_id: String,
    /// The retracted evidence source.
    pub source: String,
    /// How many evidence items were removed.
    pub removed: usize,
    /// Dependent hypotheses re-evaluated in the shadow frame, in A2 walk
    /// order (the retracted hypothesis itself is excluded — it is the one
    /// retracted, not a dependent of itself).
    pub dependents_revised: Vec<String>,
    /// True when any walk stopped early at [`MAX_REVISION_WALK_NODES`].
    pub truncated: bool,
    /// The A2 walk that selected the dependents (proposal order, cycles).
    pub revision_walk: RevisionWalk,
}

/// T3 (JTMS take): retract every piece of evidence for `hypothesis_id`
/// originating from `source` inside the shadow frame, then walk the
/// declared DependsOn dependents of every node the hypothesis references
/// and re-evaluate each dependent in shadow ([`Hypothesis::recompute_fitness`]).
/// Base state is untouched — the caller inspects the report and decides
/// whether to commit. Returns `None` when the hypothesis does not exist.
pub fn retract_evidence_with_cascade(
    ctx: &mut EvaluationContext,
    hypothesis_id: &str,
    source: &str,
) -> Option<RetractionCascadeReport> {
    let (removed, refs) = {
        let h = ctx
            .ensure_shadowed_hypothesis(hypothesis_id)
            .expect("retract_evidence_with_cascade: hypothesis must exist");
        (h.retract_evidence(source), h.ontology_refs.clone())
    };

    // The A2 walk selects dependents of the *nodes* the retracted hypothesis
    // references; everything selected is re-evaluated in the shadow frame.
    // The walk itself reports walked *nodes*; the node→hypothesis mapping
    // mirrors the A2 selection in `evaluate_mutation` (step order, sorted
    // within a step, deduped across steps).
    let mut revised: Vec<String> = Vec::new();
    let mut truncated = false;
    let mut revision_walk: Option<RevisionWalk> = None;
    for node_ref in refs {
        let walk = ctx.depends_on_walk(&node_ref);
        truncated = truncated || walk.truncated;
        if revision_walk.is_none() {
            revision_walk = Some(walk.clone());
        }

        let all_hyps = ctx.effective_hypotheses_all();
        let mut selected: Vec<String> = Vec::new();
        for step in &walk.steps {
            let mut matching: Vec<String> = all_hyps
                .iter()
                .filter(|h| h.ontology_refs.iter().any(|r| r == &step.node_id))
                .map(|h| h.id.clone())
                .collect();
            matching.sort();
            for hid in matching {
                if hid != hypothesis_id && !selected.contains(&hid) {
                    selected.push(hid);
                }
            }
        }

        for hid in selected {
            if !revised.contains(&hid) {
                revised.push(hid.clone());
            }
            if let Some(dh) = ctx.ensure_shadowed_hypothesis(&hid) {
                dh.recompute_fitness();
            }
        }
    }

    Some(RetractionCascadeReport {
        hypothesis_id: String::from(hypothesis_id),
        source: String::from(source),
        removed,
        dependents_revised: revised,
        truncated,
        revision_walk: revision_walk.unwrap_or_default(),
    })
}

// ── Core Propagation ─────────────────────────────────────────────────────

/// Classify a coherence delta.
fn classify_delta(delta: f32) -> &'static str {
    if delta.abs() < 1e-6 {
        "Unchanged"
    } else if delta > 0.0 {
        "Strengthened"
    } else if delta < -DEGRADATION_THRESHOLD {
        "Invalidated"
    } else {
        "Shifted"
    }
}

/// Depth-first collection of affected nodes with impact factors.
///
/// The impact factor for a node at depth d is `source_sim × γ^d`.
/// Propagation stops when depth exceeds [`MAX_PROPAGATION_DEPTH`] or impact
/// falls below [`MIN_IMPACT`].
fn dfs_collect(
    current_id: &str,
    adjacency: &HashMap<String, Vec<String>>,
    depth: usize,
    visited: &mut HashSet<String>,
    affected: &mut Vec<(String, f32)>,
    source_sim: f32,
) {
    if depth > MAX_PROPAGATION_DEPTH || visited.contains(current_id) {
        return;
    }
    visited.insert(current_id.to_string());

    let impact = source_sim * GAMMA.powi(depth as i32);
    if impact.abs() < MIN_IMPACT && depth > 0 {
        return;
    }

    affected.push((current_id.to_string(), impact));

    if let Some(neighbors) = adjacency.get(current_id) {
        for neighbor_id in neighbors {
            if !visited.contains(neighbor_id) {
                dfs_collect(
                    neighbor_id,
                    adjacency,
                    depth + 1,
                    visited,
                    affected,
                    source_sim,
                );
            }
        }
    }
}

/// Recompute the fitness breakdown for a hypothesis after a node mutation.
///
/// Updates `semantic_fit` (cosine alignment with referenced nodes),
/// `logical_consistency` (penalised by contradicting evidence), and the
/// overall `coherence` score, then delegates to
/// [`Hypothesis::recompute_fitness`] for the composite.
fn recompute_hypothesis_fitness(
    hyp: &mut Hypothesis,
    ctx: &EvaluationContext,
    affected_node_ids: &[String],
) {
    let affected_refs: Vec<&CoherenceNode> = hyp
        .ontology_refs
        .iter()
        .filter(|id| affected_node_ids.contains(id))
        .filter_map(|id| ctx.effective_node(id))
        .filter(|n| !n.embedding.is_empty())
        .collect();

    if !hyp.embedding.is_empty() && !affected_refs.is_empty() {
        let sims: Vec<f32> = affected_refs
            .iter()
            .map(|n| {
                let raw = cosine_sim(&hyp.embedding, &n.embedding);
                (raw + 1.0) / 2.0 // map [-1, 1] → [0, 1]
            })
            .collect();

        let new_semantic_fit = sims.iter().sum::<f32>() / sims.len() as f32;
        let new_semantic_fit = new_semantic_fit.clamp(0.0, 1.0);

        hyp.fitness_breakdown.semantic_fit = new_semantic_fit;
        hyp.coherence_profile.semantic.score = new_semantic_fit;
        hyp.coherence = new_semantic_fit;
    }

    // Logical consistency degrades proportionally to contradicting evidence
    let contra_count = hyp.contradicting_evidence.len() as Score;
    let logical = 1.0 - (contra_count * 0.1).min(0.5);
    hyp.fitness_breakdown.logical_consistency = logical;
    hyp.coherence_profile.logical.score = logical;

    hyp.recompute_fitness();
}

/// Evaluate a single mutation against the current graph state.
///
/// All computation happens inside `ctx` — no writes to persistent storage.
/// The returned [`OntologyDeltaReport`] captures every structural change:
/// node coherence deltas, hypothesis status shifts, and the net coherence
/// change across the affected sub-graph.
pub fn evaluate_mutation(
    ctx: &mut EvaluationContext,
    mutation: OntologyMutation,
) -> OntologyDeltaReport {
    ctx.mutation_log.push(mutation.clone());

    // A7 no-perturbation invariant: a zero-shift EmbeddingShift (old == new)
    // carries no information, so the wave returns empty without touching any
    // shadow state — fitness stays bit-identical, zero transitions emitted.
    if let MutationOp::EmbeddingShift {
        old_embedding,
        new_embedding,
    } = &mutation.operation
    {
        if old_embedding == new_embedding {
            return OntologyDeltaReport {
                mutation_source: mutation,
                affected_nodes: Vec::new(),
                hypothesis_status_shifts: Vec::new(),
                net_coherence_delta: 0.0,
                revision_walk: None,
                adjudications: Vec::new(),
            };
        }
    }

    let target_id = mutation.target_node_id.clone();

    // Resolve the target node from the effective (base or shadow) view.
    let base_node = ctx.effective_node(&target_id);

    // ── Isolation Phase: apply mutation to the shadow frame ─────────────
    let mut shadow_node = base_node
        .cloned()
        .unwrap_or_else(|| CoherenceNode::new(vec![]));

    let source_sim = match &mutation.operation {
        MutationOp::PropertySet { .. } => 1.0,
        MutationOp::EmbeddingShift {
            old_embedding: old_emb,
            new_embedding,
        } => {
            shadow_node.embedding = new_embedding.clone();
            if old_emb.len() == new_embedding.len() {
                cosine_sim(old_emb, new_embedding)
            } else {
                0.0
            }
        }
        MutationOp::RelationSevered { .. } => 1.0,
        MutationOp::RelationCreated { .. } => 1.0,
    };

    ctx.shadow_nodes.insert(target_id.clone(), shadow_node);

    // ── Topological Wave: DFS propagation ────────────────────────────────
    let adjacency = ctx.build_adjacency();

    let mut visited: HashSet<String> = HashSet::new();
    let mut affected: Vec<(String, f32)> = Vec::new(); // (node_id, impact)
    dfs_collect(
        &target_id,
        &adjacency,
        0,
        &mut visited,
        &mut affected,
        source_sim,
    );

    // ── Local Recalculation: update coherence scores ───────────────────
    let mut node_deltas: Vec<NodeDelta> = Vec::new();
    let mut net_delta = 0.0_f32;

    for (node_id, _impact) in &affected {
        // Capture pre-update coherence from the effective node (base or shadow)
        let prev_coherence = ctx
            .effective_node(node_id)
            .map(|n| n.coherence_score)
            .unwrap_or(0.0);

        // Recompute coherence considering the full effective graph
        let new_coherence = ctx.compute_coherence(node_id);

        // Persist the updated coherence in the shadow frame
        if let Some(node) = ctx.ensure_shadowed(node_id) {
            node.coherence_score = new_coherence;
        }

        let delta = new_coherence - prev_coherence;
        net_delta += delta;

        node_deltas.push(NodeDelta {
            node_id: node_id.clone(),
            previous_coherence: prev_coherence,
            new_coherence,
            delta_type: classify_delta(delta).to_string(),
        });
    }

    // ── Hypothesis Evaluation ──────────────────────────────────────────
    let mut hypothesis_transitions: Vec<HypothesisTransition> = Vec::new();
    let affected_node_ids: Vec<String> = affected.iter().map(|(id, _)| id.clone()).collect();

    // ── A2 (Atlas take): revision selection by declared dependency ───────
    //
    // Hypothesis revision is selected by the DependsOn closure of the
    // mutated node, NOT by the breadth wave above. Mere arrival — a similar
    // embedding, a shared cell pin, a label prefix — must not shift a
    // hypothesis; only a declared DependsOn edge justifies a revision. The
    // breadth wave keeps its node-coherence reporting job; it no longer
    // selects hypotheses. If the graph declares no DependsOn dependents the
    // engine falls back to the previous breadth-affected selection and logs
    // it (`fallback_breadth_used`) — a sparse DependsOn graph must not go
    // blind (A2 risk note).
    let mut walk = ctx.depends_on_walk(&target_id);
    let has_declared_dependents = !walk.dependents.is_empty();
    walk.fallback_breadth_used = !has_declared_dependents;

    let hyp_ids: Vec<String> = if has_declared_dependents {
        let all_hyps = ctx.effective_hypotheses_all();
        let mut ordered: Vec<String> = Vec::new();
        for step in &walk.steps {
            let mut matching: Vec<String> = all_hyps
                .iter()
                .filter(|h| h.ontology_refs.iter().any(|r| r == &step.node_id))
                .map(|h| h.id.clone())
                .collect();
            matching.sort();
            for id in matching {
                if !ordered.contains(&id) {
                    ordered.push(id);
                }
            }
        }
        ordered
    } else {
        // Logged fallback: the previous breadth-affected behaviour.
        ctx.effective_hypotheses_all()
            .into_iter()
            .filter(|h| {
                h.ontology_refs
                    .iter()
                    .any(|r| affected_node_ids.contains(r))
            })
            .map(|h| h.id.clone())
            .collect()
    };
    walk.revised_hypotheses = hyp_ids.clone();

    let mut adjudications: Vec<AdjudicationDecision> = Vec::new();
    for hyp_id in &hyp_ids {
        // Capture pre-mutation values from the effective (base or shadow) view
        let (prev_coherence, prev_fitness, prev_status) = {
            let hyp = ctx
                .effective_hypothesis(hyp_id)
                .expect("hypothesis must exist");
            (hyp.coherence, hyp.fitness, hyp.status)
        };

        // Clone into shadow for mutation
        let mut updated = ctx
            .effective_hypothesis(hyp_id)
            .expect("hypothesis must exist")
            .clone();

        recompute_hypothesis_fitness(&mut updated, ctx, &affected_node_ids);

        let coherence_delta = prev_coherence - updated.coherence;

        if coherence_delta > DEGRADATION_THRESHOLD
            && matches!(
                prev_status,
                HypothesisStatus::Supported
                    | HypothesisStatus::Confirmed
                    | HypothesisStatus::Certified
            )
        {
            let new_status = HypothesisStatus::Contradicted;
            let route = route_transition(prev_status, coherence_delta);
            let rationale = format!(
                "Semantic degradation Δcoherence={:.4} exceeds ε={} (mutation {}): \
                 coherence {:.4} → {:.4}, fitness {:.4} → {:.4}",
                coherence_delta,
                DEGRADATION_THRESHOLD,
                mutation.id,
                prev_coherence,
                updated.coherence,
                prev_fitness,
                updated.fitness,
            );

            // The degradation is recorded fact on every route: evidence
            // lands, fitness absorbs it. What differs by route is whether
            // the *status* may move without a human.
            let evidence = Evidence::contradicts(&mutation.id, rationale.clone());
            updated.contradicting_evidence.push(evidence);
            updated.recompute_fitness();

            match route {
                AdjudicationRoute::AutoApply => {
                    hypothesis_transitions.push(HypothesisTransition {
                        hypothesis_id: updated.id.clone(),
                        previous_status: prev_status,
                        new_status,
                        trigger_reason: format!(
                            "Semantic degradation exceeded threshold ε={} \
                             (Δcoherence={:.4})",
                            DEGRADATION_THRESHOLD, coherence_delta,
                        ),
                    });

                    // Reflect the transition on the shadow hypothesis
                    updated.status = new_status;
                    updated.revised_at = Utc::now();
                    updated.revision_history.push(crate::hypothesis::Revision {
                        timestamp: Utc::now(),
                        description: format!(
                            "Auto-transitioned to Contradicted by delta engine (mutation {})",
                            mutation.id
                        ),
                        previous_status: prev_status,
                        new_status,
                        trigger: Some(mutation.id.clone()),
                    });
                    adjudications.push(AdjudicationDecision {
                        hypothesis_id: updated.id.clone(),
                        route,
                        proposed_status: new_status,
                        coherence_delta,
                        resolution: ResolutionStatus::APreferred,
                        rationale,
                    });
                }
                AdjudicationRoute::StrategicReview | AdjudicationRoute::CoreProtected => {
                    // A5: the status stays as-is - the demotion is a
                    // proposal with a recorded rationale; the engine never
                    // auto-resolves it. The revision entry carries the
                    // proposal so the trail shows it without a queue surface.
                    updated.revised_at = Utc::now();
                    updated.revision_history.push(crate::hypothesis::Revision {
                        timestamp: Utc::now(),
                        description: format!(
                            "Proposed {new_status:?} by delta engine (mutation {}) - \
                             routed {route:?}, resolution Open: {rationale}",
                            mutation.id
                        ),
                        previous_status: prev_status,
                        new_status,
                        trigger: Some(mutation.id.clone()),
                    });
                    adjudications.push(AdjudicationDecision {
                        hypothesis_id: updated.id.clone(),
                        route,
                        proposed_status: new_status,
                        coherence_delta,
                        resolution: ResolutionStatus::Open,
                        rationale,
                    });
                }
            }
        }

        ctx.shadow_hypotheses.insert(updated.id.clone(), updated);
    }

    OntologyDeltaReport {
        mutation_source: mutation,
        affected_nodes: node_deltas,
        hypothesis_status_shifts: hypothesis_transitions,
        net_coherence_delta: net_delta,
        revision_walk: Some(walk),
        adjudications,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, embedding: Vec<f32>, coherence: Score) -> CoherenceNode {
        let mut n = CoherenceNode::new(embedding);
        n.id = id.to_string();
        n.coherence_score = coherence;
        n
    }

    #[test]
    fn property_set_does_not_change_embedding() {
        let mut n = node("n1", vec![1.0, 0.0, 0.0, 0.0], 0.5);
        n.label = Some("test_node".to_string());
        let edge_vec: Vec<TypedEdge> = vec![];
        let nodes = vec![n.clone()];
        let ctx = EvaluationContext::from_base(&nodes, &[], &edge_vec);
        assert_eq!(n.embedding, ctx.effective_node("n1").unwrap().embedding);
    }

    #[test]
    fn classify_delta_thresholds() {
        assert_eq!(classify_delta(0.0), "Unchanged");
        assert_eq!(classify_delta(0.5), "Strengthened");
        assert_eq!(classify_delta(-0.1), "Shifted");
        assert_eq!(classify_delta(-0.3), "Invalidated");
    }

    #[test]
    fn dfs_respects_max_depth() {
        // Chain: 0 - 1 - 2 - 3 - 4 - 5
        let adjacency: HashMap<String, Vec<String>> = [
            ("n0", vec!["n1"]),
            ("n1", vec!["n0", "n2"]),
            ("n2", vec!["n1", "n3"]),
            ("n3", vec!["n2", "n4"]),
            ("n4", vec!["n3", "n5"]),
            ("n5", vec!["n4"]),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect();

        let mut visited = HashSet::new();
        let mut affected = Vec::new();
        dfs_collect("n0", &adjacency, 0, &mut visited, &mut affected, 1.0);

        // With MAX_PROPAGATION_DEPTH=5, nodes n0..n5 should all be visited
        assert_eq!(affected.len(), 6);
        assert_eq!(affected[0].0, "n0");
        assert_eq!(affected[5].0, "n5");
    }

    #[test]
    fn ontology_mutation_serializes() {
        let mutation = OntologyMutation::new(
            "node_42",
            MutationOp::EmbeddingShift {
                old_embedding: vec![1.0, 0.0],
                new_embedding: vec![0.0, 1.0],
            },
        )
        .with_trigger("manual_review");

        let json = serde_json::to_string(&mutation).unwrap();
        let back: OntologyMutation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.target_node_id, "node_42");
        assert_eq!(back.triggered_by, Some("manual_review".to_string()));
    }

    #[test]
    fn report_serializes_to_json() {
        let report = OntologyDeltaReport {
            mutation_source: OntologyMutation::new(
                "n1",
                MutationOp::PropertySet {
                    key: "label".into(),
                    old_val: serde_json::Value::String("old".into()),
                    new_val: serde_json::Value::String("new".into()),
                },
            ),
            affected_nodes: vec![NodeDelta {
                node_id: "n1".to_string(),
                previous_coherence: 0.8,
                new_coherence: 0.6,
                delta_type: "Invalidated".to_string(),
            }],
            hypothesis_status_shifts: vec![],
            net_coherence_delta: -0.2,
            revision_walk: None,
            adjudications: vec![],
        };

        let json = serde_json::to_string(&report).unwrap();
        let back: OntologyDeltaReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.net_coherence_delta, -0.2);
        assert_eq!(back.affected_nodes[0].delta_type, "Invalidated");
    }
}
