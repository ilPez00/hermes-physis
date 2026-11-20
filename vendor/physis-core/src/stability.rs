//! Shadow perturbation, stability margins, and ablation diagnostics.
//!
//! Robustness is not a property a hypothesis has in isolation; it is what
//! remains of its standing when the evidence, links, or representations it
//! depends on are stressed. This module runs **controlled perturbations in a
//! shadow [`crate::delta_engine::EvaluationContext`]** — never committing —
//! and reports how fitness, status, and coherence move.
//!
//! The translation is deliberate (CQFT-style ablation / perturbation
//! survival, made auditable): retract or drop evidence (with the DependsOn
//! cascade), sever or add typed relations, and apply bounded deterministic
//! embedding shifts. Each scenario is an ablation: compare coupled vs
//! decoupled/removed factors and read the margin. Reports are diagnostics,
//! not verdicts — a fragile hypothesis is flagged, not auto-demoted except
//! through the engine's existing, explicit retraction semantics.

use serde::{Deserialize, Serialize};

use crate::delta_engine::{self, EvaluationContext, RevisionWalk};
use crate::hypothesis::{Hypothesis, HypothesisStatus};
use crate::models::{CoherenceNode, Score};
use crate::relation::TypedEdge;

/// Deterministic xorshift64 in [0,1).
fn rand01(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    ((*state as f64) / (u64::MAX as f64)) as f32
}

/// A single perturbation operation applied inside a shadow scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerturbationOp {
    /// Retract evidence from a hypothesis by source (with DependsOn cascade).
    RetractEvidence {
        hypothesis_id: String,
        source: String,
    },
    /// Remove all typed edges matching (source_id, target_id) in either direction.
    SeverRelation {
        source_id: String,
        target_id: String,
    },
    /// Add a typed edge for the duration of the scenario.
    CreateRelation { edge: TypedEdge },
    /// Apply a bounded, deterministic jitter to a node embedding and renormalize.
    /// `magnitude` is the per-dimension noise scale (clamped >= 0).
    EmbeddingNoise {
        node_id: String,
        magnitude: f32,
        seed: u64,
    },
    /// Scale a node's embedding toward/away from the origin then renormalize
    /// (a deterministic "confidence" ablation of the representation).
    EmbeddingScale { node_id: String, factor: f32 },
}

/// A named perturbation scenario (one or more ops applied together = coupled).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerturbationScenario {
    pub label: String,
    pub ops: Vec<PerturbationOp>,
}

impl PerturbationScenario {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ops: Vec::new(),
        }
    }
    pub fn with_op(mut self, op: PerturbationOp) -> Self {
        self.ops.push(op);
        self
    }
}

/// Snapshot of a hypothesis relevant to stability assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HypothesisSnapshot {
    pub id: String,
    pub status: HypothesisStatus,
    pub fitness: Score,
    pub coherence: Score,
    pub confidence: Score,
    pub supporting_evidence: usize,
    pub contradicting_evidence: usize,
    pub open_predictions: usize,
}

impl HypothesisSnapshot {
    pub fn capture(h: &Hypothesis) -> Self {
        Self {
            id: h.id.clone(),
            status: h.status,
            fitness: h.fitness,
            coherence: h.coherence,
            confidence: h.confidence,
            supporting_evidence: h.supporting_evidence.len(),
            contradicting_evidence: h.contradicting_evidence.len(),
            open_predictions: h.pending_predictions(),
        }
    }
}

/// Stability margin for one hypothesis under one scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisMargin {
    pub hypothesis_id: String,
    pub before: HypothesisSnapshot,
    pub after: Option<HypothesisSnapshot>,
    pub fitness_delta: Score,
    pub coherence_delta: Score,
    pub status_flipped: bool,
    /// True when the hypothesis degraded (fitness dropped or status worsened/flipped).
    pub degraded: bool,
    /// Evidence sources present on the hypothesis (dominance hints).
    pub evidence_sources: Vec<String>,
    /// Ontology refs (links/DependsOn anchors) the hypothesis depends on.
    pub ontology_refs: Vec<String>,
}

/// Outcome of running one scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioOutcome {
    pub scenario: String,
    pub margins: Vec<HypothesisMargin>,
    /// Aggregate fitness delta over tracked hypotheses (sum).
    pub aggregate_fitness_delta: Score,
    /// How many tracked hypotheses flipped status.
    pub status_flips: usize,
    /// How many tracked hypotheses degraded.
    pub degraded: usize,
    /// Revision walks triggered by retractions/cascades (DependOn closure evidence).
    pub revision_walks: Vec<RevisionWalk>,
    /// Notes on ablation/coupling (e.g. which ops ran, edges removed/added).
    pub notes: Vec<String>,
}

/// Full robustness/ablation report over a set of scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustnessReport {
    pub scenarios: Vec<ScenarioOutcome>,
    /// Hypotheses tracked across scenarios (ids).
    pub tracked_hypotheses: Vec<String>,
}

impl RobustnessReport {
    /// True if no tracked hypothesis flipped or degraded in any scenario.
    pub fn all_stable(&self) -> bool {
        self.scenarios
            .iter()
            .all(|s| s.status_flips == 0 && s.degraded == 0)
    }
}

fn l2_normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 1e-12 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

fn apply_embedding_op(node: &mut CoherenceNode, op: &PerturbationOp) {
    match op {
        PerturbationOp::EmbeddingNoise {
            magnitude, seed, ..
        } => {
            let mag = magnitude.max(0.0);
            let mut s = *seed ^ (node.id.len() as u64).wrapping_mul(0xD1B54A32D192ED03);
            for x in node.embedding.iter_mut() {
                let r = (rand01(&mut s) * 2.0 - 1.0) * mag;
                *x += r;
            }
            l2_normalize(&mut node.embedding);
        }
        PerturbationOp::EmbeddingScale { factor, .. } => {
            let f = if factor.is_finite() { *factor } else { 1.0 };
            for x in node.embedding.iter_mut() {
                *x *= f;
            }
            l2_normalize(&mut node.embedding);
        }
        _ => {}
    }
}

/// Run a set of perturbation scenarios against base state, returning a shadow
/// robustness report. Base nodes/hypotheses/edges are never mutated.
pub fn assess_robustness(
    base_nodes: &[CoherenceNode],
    base_hypotheses: &[Hypothesis],
    base_edges: &[TypedEdge],
    scenarios: &[PerturbationScenario],
    track: Option<&[String]>,
) -> RobustnessReport {
    let tracked: Vec<String> = match track {
        Some(ids) => ids.to_vec(),
        None => {
            let mut ids: Vec<String> = base_hypotheses.iter().map(|h| h.id.clone()).collect();
            ids.sort();
            ids
        }
    };

    let mut outcomes = Vec::new();

    for sc in scenarios {
        // Build scenario-local edge list (supports sever/create ablations).
        let mut edges: Vec<TypedEdge> = base_edges.to_vec();
        let mut notes = Vec::new();
        for op in &sc.ops {
            match op {
                PerturbationOp::SeverRelation {
                    source_id,
                    target_id,
                } => {
                    let before = edges.len();
                    edges.retain(|e| {
                        !((e.source_id == *source_id && e.target_id == *target_id)
                            || (e.source_id == *target_id && e.target_id == *source_id))
                    });
                    notes.push(format!(
                        "sever {}<->{} (removed {} edge(s))",
                        source_id,
                        target_id,
                        before - edges.len()
                    ));
                }
                PerturbationOp::CreateRelation { edge } => {
                    edges.push(edge.clone());
                    notes.push(format!(
                        "create edge {} -> {} ({:?})",
                        edge.source_id, edge.target_id, edge.relation_type
                    ));
                }
                _ => {}
            }
        }

        let mut ctx = EvaluationContext::from_base(base_nodes, base_hypotheses, &edges);
        let mut revision_walks = Vec::new();

        // Apply embedding perturbations to shadow nodes.
        for op in &sc.ops {
            match op {
                PerturbationOp::EmbeddingNoise { node_id, .. }
                | PerturbationOp::EmbeddingScale { node_id, .. } => {
                    if let Some(n) = ctx.ensure_shadowed(node_id) {
                        apply_embedding_op(n, op);
                        notes.push(format!("embedding perturbation on {}", node_id));
                    } else {
                        notes.push(format!(
                            "embedding perturbation skipped (unknown node {})",
                            node_id
                        ));
                    }
                }
                _ => {}
            }
        }

        // Apply evidence retractions with cascade.
        for op in &sc.ops {
            if let PerturbationOp::RetractEvidence {
                hypothesis_id,
                source,
            } = op
            {
                if let Some(report) =
                    delta_engine::retract_evidence_with_cascade(&mut ctx, hypothesis_id, source)
                {
                    notes.push(format!(
                        "retract evidence '{}' from {} (removed {}, dependents revised {})",
                        source,
                        hypothesis_id,
                        report.removed,
                        report.dependents_revised.len()
                    ));
                    revision_walks.push(report.revision_walk);
                } else {
                    notes.push(format!(
                        "retract evidence '{}' from {} skipped (unknown hypothesis)",
                        source, hypothesis_id
                    ));
                }
            }
        }

        // If edges/embeddings changed but no retraction ran, dependents of affected
        // structure may still need a fitness recompute signal; recompute tracked
        // hypotheses in shadow so margins reflect current shadowed graph state.
        for hid in &tracked {
            if let Some(h) = ctx.ensure_shadowed_hypothesis(hid) {
                h.recompute_fitness();
            }
        }

        let mut margins = Vec::new();
        let mut status_flips = 0usize;
        let mut degraded = 0usize;
        let mut agg = 0.0f32;

        for hid in &tracked {
            let before_h = match base_hypotheses.iter().find(|h| &h.id == hid) {
                Some(h) => h,
                None => continue,
            };
            let before = HypothesisSnapshot::capture(before_h);
            let after_h = ctx.effective_hypothesis(hid);
            let after = after_h.map(HypothesisSnapshot::capture);
            let (fitness_delta, coherence_delta, status_flipped, is_degraded) = match &after {
                Some(a) => {
                    let fd = a.fitness - before.fitness;
                    let cd = a.coherence - before.coherence;
                    let flipped = a.status != before.status;
                    let deg = fd < -1e-6 || flipped;
                    (fd, cd, flipped, deg)
                }
                None => (0.0, 0.0, false, false),
            };
            if status_flipped {
                status_flips += 1;
            }
            if is_degraded {
                degraded += 1;
            }
            agg += fitness_delta;

            let mut evidence_sources: Vec<String> = before_h
                .supporting_evidence
                .iter()
                .chain(before_h.contradicting_evidence.iter())
                .map(|e| e.source.clone())
                .collect();
            evidence_sources.sort();
            evidence_sources.dedup();

            margins.push(HypothesisMargin {
                hypothesis_id: hid.clone(),
                before,
                after,
                fitness_delta,
                coherence_delta,
                status_flipped,
                degraded: is_degraded,
                evidence_sources,
                ontology_refs: before_h.ontology_refs.clone(),
            });
        }

        outcomes.push(ScenarioOutcome {
            scenario: sc.label.clone(),
            margins,
            aggregate_fitness_delta: agg,
            status_flips,
            degraded,
            revision_walks,
            notes,
        });
    }

    RobustnessReport {
        scenarios: outcomes,
        tracked_hypotheses: tracked,
    }
}

/// Convenience: assess robustness of one hypothesis to retracting each of its
/// evidence sources independently (per-source ablation). Deterministic order.
pub fn evidence_ablation(
    base_nodes: &[CoherenceNode],
    base_hypotheses: &[Hypothesis],
    base_edges: &[TypedEdge],
    hypothesis_id: &str,
) -> RobustnessReport {
    let mut sources: Vec<String> = base_hypotheses
        .iter()
        .find(|h| h.id == hypothesis_id)
        .map(|h| {
            h.supporting_evidence
                .iter()
                .chain(h.contradicting_evidence.iter())
                .map(|e| e.source.clone())
                .collect()
        })
        .unwrap_or_default();
    sources.sort();
    sources.dedup();
    let scenarios: Vec<PerturbationScenario> = sources
        .into_iter()
        .map(|s| {
            PerturbationScenario::new(format!("retract:{s}")).with_op(
                PerturbationOp::RetractEvidence {
                    hypothesis_id: hypothesis_id.to_string(),
                    source: s,
                },
            )
        })
        .collect();
    assess_robustness(
        base_nodes,
        base_hypotheses,
        base_edges,
        &scenarios,
        Some(&[hypothesis_id.to_string()]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::Evidence;

    fn hyp(id: &str, refs: Vec<&str>) -> Hypothesis {
        let mut h = Hypothesis::new("h", vec![1.0, 0.0, 0.0]);
        h.id = id.to_string();
        h.ontology_refs = refs.into_iter().map(|s| s.to_string()).collect();
        h.supporting_evidence
            .push(Evidence::supports("s1", "ok").with_intake_id("i1"));
        h.contradicting_evidence
            .push(Evidence::contradicts("c1", "bad").with_intake_id("i2"));
        h.recompute_fitness();
        h
    }

    fn node(id: &str, emb: Vec<f32>) -> CoherenceNode {
        let mut n = CoherenceNode::new(emb);
        n.id = id.to_string();
        n
    }

    #[test]
    fn retracting_evidence_changes_fitness_in_shadow_only() {
        let nodes = vec![node("n1", vec![1.0, 0.0, 0.0])];
        let h = hyp("h1", vec!["n1"]);
        let base_fit = h.fitness;
        let report = evidence_ablation(&nodes, std::slice::from_ref(&h), &[], "h1");
        assert_eq!(report.scenarios.len(), 2, "one per source");
        // Base hypothesis untouched.
        assert_eq!(h.fitness, base_fit);
        // At least one ablation should move fitness (removing contradicting or supporting).
        assert!(
            report
                .scenarios
                .iter()
                .any(|s| s.margins[0].fitness_delta.abs() > 1e-6),
            "some ablation should affect fitness"
        );
    }

    #[test]
    fn embedding_noise_is_deterministic() {
        let nodes = vec![node("n1", vec![1.0, 0.0, 0.0])];
        let h = hyp("h1", vec!["n1"]);
        let sc = PerturbationScenario::new("noise").with_op(PerturbationOp::EmbeddingNoise {
            node_id: "n1".to_string(),
            magnitude: 0.1,
            seed: 42,
        });
        let r1 = assess_robustness(
            &nodes,
            std::slice::from_ref(&h),
            &[],
            std::slice::from_ref(&sc),
            Some(&["h1".into()]),
        );
        let r2 = assess_robustness(&nodes, &[h], &[], &[sc], Some(&["h1".into()]));
        assert_eq!(
            r1.scenarios[0].margins[0].after.as_ref().unwrap().fitness,
            r2.scenarios[0].margins[0].after.as_ref().unwrap().fitness
        );
    }

    #[test]
    fn sever_relation_scenario_runs_without_panic() {
        let nodes = vec![node("a", vec![1.0, 0.0]), node("b", vec![0.0, 1.0])];
        let h = hyp("h1", vec!["a"]);
        let edge = TypedEdge::new(crate::relation::RelationType::DependsOn, "a", "b");
        let sc = PerturbationScenario::new("sever").with_op(PerturbationOp::SeverRelation {
            source_id: "a".to_string(),
            target_id: "b".to_string(),
        });
        let report = assess_robustness(&nodes, &[h], &[edge], &[sc], Some(&["h1".into()]));
        assert_eq!(report.scenarios.len(), 1);
        assert!(report.scenarios[0]
            .notes
            .iter()
            .any(|n| n.contains("sever")));
    }
}
