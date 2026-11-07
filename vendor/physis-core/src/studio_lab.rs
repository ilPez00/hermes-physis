//! Studio extension surfaces: the Flow inspector, the Semiotics Lab map,
//! the coherence checker, near-duplicate detection, and process workflows.
//!
//! Split out of `studio.rs` to keep each file reviewable; all handlers share
//! the same `StudioState` and are merged into the studio router at startup.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::models::CoherenceNode;
use crate::studio::StudioState;

type Shared = Arc<RwLock<StudioState>>;

pub fn router() -> Router<Shared> {
    Router::new()
        .route("/api/flow/graph", get(flow_graph))
        .route("/api/flow/processes", get(processes_list))
        .route("/api/flow/processes/demo", post(process_demo))
        .route("/api/lab/map", get(lab_map))
        .route("/api/lab/neighbors", post(lab_neighbors))
        .route("/api/coherence/check", post(coherence_check))
        .route("/api/core/dups", get(dup_clusters))
}

// ── PCA (power iteration) ─────────────────────────────────────────────

/// Project vectors onto their two leading principal components.
/// Power iteration on the covariance, deflating after the first axis.
fn pca2(vectors: &[Vec<f32>]) -> Vec<(f32, f32)> {
    if vectors.is_empty() {
        return vec![];
    }
    let dim = vectors[0].len();
    let n = vectors.len() as f32;

    // Mean-center once; every pass reuses the centered copy.
    let mut mean = vec![0.0f32; dim];
    for v in vectors {
        for (i, x) in v.iter().enumerate() {
            mean[i] += x / n;
        }
    }
    let centered: Vec<Vec<f32>> = vectors
        .iter()
        .map(|v| v.iter().zip(&mean).map(|(x, m)| x - m).collect())
        .collect();

    // Covariance × vector, without materializing dim×dim (384² floats is
    // wasteful when only two axes are needed).
    let cov_mul = |v: &[f32]| -> Vec<f32> {
        let mut out = vec![0.0f32; dim];
        for row in &centered {
            let r = row.dot(v);
            for (o, x) in out.iter_mut().zip(row) {
                *o += x * r / n;
            }
        }
        out
    };

    fn normalize(v: &mut [f32]) -> bool {
        // Returns false when the vector collapsed (degenerate input).
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm < 1e-9 {
            return false;
        }
        for x in v.iter_mut() {
            *x /= norm;
        }
        true
    }

    let mut pc1: Vec<f32> = (0..dim).map(|i| ((i % 7) as f32 + 0.5) / 7.5).collect();
    for _ in 0..60 {
        let mut next = cov_mul(&pc1);
        if !normalize(&mut next) {
            break;
        }
        let drift: f32 = next.iter().zip(&pc1).map(|(a, b)| (a - b).abs()).sum();
        pc1 = next;
        if drift < 1e-6 {
            break;
        }
    }

    let mut pc2: Vec<f32> = (0..dim)
        .map(|i| (((i + 3) % 11) as f32 + 0.5) / 11.5)
        .collect();
    for _ in 0..80 {
        // Deflate pc1 out of the working vector before each multiply.
        let proj: f32 = pc2.dot(&pc1);
        let resid: Vec<f32> = pc2.iter().zip(&pc1).map(|(a, b)| a - proj * b).collect();
        let mut next = cov_mul(&resid);
        if !normalize(&mut next) {
            break;
        }
        let drift: f32 = next.iter().zip(&pc2).map(|(a, b)| (a - b).abs()).sum();
        pc2 = next;
        if drift < 1e-6 {
            break;
        }
    }

    vectors
        .iter()
        .map(|v| {
            (
                v.iter().zip(&pc1).map(|(a, b)| a * b).sum(),
                v.iter().zip(&pc2).map(|(a, b)| a * b).sum(),
            )
        })
        .collect()
}

trait Dot {
    fn dot(&self, other: &Self) -> f32;
}

impl Dot for [f32] {
    fn dot(&self, other: &Self) -> f32 {
        self.iter().zip(other).map(|(a, b)| a * b).sum()
    }
}

// ── GET /api/flow/graph — everything the Flow canvas renders ─────────

async fn flow_graph(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();

    let mut nodes: Vec<serde_json::Value> = Vec::new();
    let mut edges: Vec<serde_json::Value> = Vec::new();

    // Corpus nodes — the substrate of the graph.
    for n in s.core.nodes.values().filter(|n| n.label.is_some()) {
        nodes.push(node_json(n));
    }
    // Hypotheses — competing interpretations rendered as bright hexagons.
    for h in s.core.hypotheses.values() {
        nodes.push(serde_json::json!({
            "id": h.id,
            "kind": "hypothesis",
            "label": h.statement,
            "status": h.status,
            "fitness": h.fitness,
            "coherence": h.coherence,
            "cell": h.ontology_refs.first().cloned().unwrap_or_default(),
        }));
        for ev in &h.supporting_evidence {
            edges.push(edge_json(
                &h.id,
                &format!("ev:{}", ev.source),
                "Supports",
                ev.confidence,
            ));
        }
        for ev in &h.contradicting_evidence {
            edges.push(edge_json(
                &h.id,
                &format!("ev:{}", ev.source),
                "Contradicts",
                ev.confidence,
            ));
        }
    }

    // Typed graph edges between known entities.
    for e in &s.core.edges {
        edges.push(serde_json::json!({
            "source": e.source_id,
            "target": e.target_id,
            "relation": e.relation_type,
            "confidence": e.confidence,
        }));
    }

    // Contradictions — red tension links between claim parties.
    let conflicts: Vec<serde_json::Value> = s
        .core
        .contradictions
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "a": c.claim_a.claim,
                "b": c.claim_b.claim,
                "resolution": c.resolution,
                "explanation": c.explanation,
                "detected_at": c.detected_at.to_rfc3339(),
            })
        })
        .collect();

    // Workflows — PDCA cycles as stage pipelines.
    let workflows: Vec<serde_json::Value> = s.core.process_cycles.iter().map(cycle_json).collect();

    Json(serde_json::json!({
        "nodes": nodes,
        "edges": edges,
        "conflicts": conflicts,
        "workflows": workflows,
        "certified_branches": s.core.certified_branches.len(),
        "isolated_branches": s.core.isolated_branches.len(),
        "dreams": s.core.dream_archive.len(),
    }))
    .into_response()
}

fn node_json(n: &CoherenceNode) -> serde_json::Value {
    serde_json::json!({
        "id": n.id,
        "kind": "node",
        "label": n.label.clone().unwrap_or_else(|| n.id.clone()),
        "score": n.coherence_score,
        "verdict": n.asserted,
        "cell": n.cell_pin.as_ref().map(|(d, m)| format!("{d}×{m}")).unwrap_or_default(),
    })
}

fn edge_json(source: &str, target: &str, relation: &str, confidence: f32) -> serde_json::Value {
    serde_json::json!({ "source": source, "target": target, "relation": relation, "confidence": confidence })
}

fn cycle_json(c: &crate::process::ProcessCycle) -> serde_json::Value {
    let plan_tasks: Vec<serde_json::Value> = c
        .plan
        .as_ref()
        .map(|p| {
            p.tasks
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "title": t.title,
                        "state": t.state,
                        "deps": t.dependencies,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let state_counts: HashMap<String, usize> =
        plan_tasks.iter().fold(HashMap::new(), |mut acc, t| {
            *acc.entry(t["state"].as_str().unwrap_or("Pending").to_string())
                .or_default() += 1;
            acc
        });
    serde_json::json!({
        "id": c.id,
        "plan_name": c.plan.as_ref().map(|p| p.name.clone()),
        "goal": c.plan.as_ref().and_then(|p| p.goal_id.clone()),
        "tasks": plan_tasks,
        "task_states": state_counts,
        "constraints": c.plan.as_ref().map(|p| p.constraints.len()).unwrap_or(0),
        "measurements": c.measurements.len(),
        "nominal_measurements": c.measurements.iter().filter(|m| m.is_nominal).count(),
        "deviations": c.deviations.len(),
        "max_severity": c.deviations.iter().map(|d| d.severity).fold(0.0f32, f32::max),
        "interventions": c.interventions.len(),
        "open_interventions": c.interventions.iter().filter(|i| !i.resolved).count(),
        "outcome": c.outcome.as_ref().map(|o| serde_json::json!({
            "success": o.success,
            "scrap": o.scrap_or_error_count,
            "summary": o.summary,
            "at": o.completion_time.to_rfc3339(),
        })),
    })
}

// ── GET /api/flow/processes · POST /api/flow/processes/demo ──────────

async fn processes_list(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let list: Vec<serde_json::Value> = s.core.process_cycles.iter().map(cycle_json).collect();
    Json(list).into_response()
}

async fn process_demo(State(state): State<Shared>) -> Response {
    use crate::process::*;
    let now = chrono::Utc::now();
    let task = |id: &str, title: &str, state: TaskState, deps: &[&str]| ProcessTask {
        id: id.to_string(),
        title: title.to_string(),
        state,
        assigned_resources: vec![],
        dependencies: deps.iter().map(|s| s.to_string()).collect(),
        expected_duration_secs: Some(600),
    };
    let mut cycle = ProcessCycle::new(format!("demo-{}", uuid_v4_short()));
    cycle.plan = Some(ProcessPlan {
        id: "plan-cnc-batch".into(),
        goal_id: Some("goal-oee-92".into()),
        name: "CNC batch run — bracket rev C".into(),
        tasks: vec![
            task(
                "t1",
                "Fixture setup & datum probe",
                TaskState::Completed,
                &[],
            ),
            task("t2", "Rough milling op 10", TaskState::Completed, &["t1"]),
            task("t3", "Finish milling op 20", TaskState::Completed, &["t2"]),
            task("t4", "Deburr & wash", TaskState::InProgress, &["t3"]),
            task(
                "t5",
                "CMM first-article inspection",
                TaskState::Pending,
                &["t4"],
            ),
            task("t6", "Pack & label lot", TaskState::Blocked, &["t5"]),
        ],
        constraints: vec![ProcessConstraint {
            id: "c1".into(),
            name: "Surface roughness".into(),
            metric: "ra_um".into(),
            min_value: None,
            max_value: Some(0.8),
            is_hard_constraint: true,
        }],
        created_at: now,
    });
    cycle.record_measurement(ProcessMeasurement {
        id: "m1".into(),
        metric: "spindle_load_pct".into(),
        value: 62.0,
        unit: "%".into(),
        machine_or_source: "vmc-07".into(),
        timestamp: now,
        is_nominal: true,
    });
    cycle.record_measurement(ProcessMeasurement {
        id: "m2".into(),
        metric: "coolant_temp_c".into(),
        value: 41.5,
        unit: "°C".into(),
        machine_or_source: "vmc-07".into(),
        timestamp: now,
        is_nominal: false,
    });
    cycle.record_deviation(ProcessDeviation {
        id: "d1".into(),
        task_or_process_id: "t3".into(),
        metric: "coolant_temp_c".into(),
        expected_value: 35.0,
        observed_value: 41.5,
        severity: 0.55,
        detected_at: now,
        description: "Coolant creeping past nominal; chiller may be fouled.".into(),
    });
    cycle.record_intervention(ProcessIntervention {
        id: "i1".into(),
        deviation_id: Some("d1".into()),
        operator: "gio".into(),
        action_taken: "Backflushed chiller filter; target 36°C within 20 min".into(),
        timestamp: now,
        expected_recovery: "coolant_temp_c ≤ 36".into(),
        actual_recovery: None,
        resolved: false,
    });

    let mut s = state.write().unwrap();
    let id = cycle.id.clone();
    s.core.record_process_cycle(cycle);
    s.save_core();
    Json(serde_json::json!({ "ok": true, "cycle": id })).into_response()
}

fn uuid_v4_short() -> String {
    // Cheap unique suffix; full UUID semantics aren't needed for demo ids.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 + d.as_secs())
        .unwrap_or(0);
    format!("{nanos:x}")
}

// ── GET /api/lab/map — PCA projection of ontology entries + corpus ───

async fn lab_map(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();

    // One embedding per ontology entry, tagged with its cell.
    let mut texts_meta: Vec<(String, String, String)> = Vec::new(); // name, domain, mode
    let mut vectors: Vec<Vec<f32>> = Vec::new();
    for cell in &s.classifier.cells {
        for (i, emb) in cell.embeddings.iter().enumerate() {
            let name = cell.entries.get(i).cloned().unwrap_or_default();
            texts_meta.push((name, cell.domain.clone(), cell.mode.clone()));
            vectors.push(emb.clone());
        }
    }

    let entry_count = vectors.len();

    // Fold corpus nodes into the same space — but only those embedded by the
    // active embedder. Mixing a stale random-projection corpus into a semantic
    // projection scatters ghost outliers across the map.
    let active_kind = s.embedder_kind.clone();
    let node_ids: Vec<String> = s
        .core
        .nodes
        .values()
        .filter(|n| n.label.is_some())
        .filter(|n| n.embedder.as_deref().unwrap_or(&active_kind) == active_kind)
        .map(|n| n.id.clone())
        .collect();
    for id in &node_ids {
        if let Some(n) = s.core.nodes.get(id) {
            vectors.push(n.embedding.clone());
        }
    }

    let proj = pca2(&vectors);

    let entries: Vec<serde_json::Value> = proj[..entry_count]
        .iter()
        .zip(&texts_meta)
        .map(|((x, y), (name, domain, mode))| {
            serde_json::json!({ "name": name, "domain": domain, "mode": mode, "x": x, "y": y })
        })
        .collect();

    let corpus: Vec<serde_json::Value> = proj[entry_count..]
        .iter()
        .zip(&node_ids)
        .filter_map(|((x, y), id)| {
            let n = s.core.nodes.get(id)?;
            Some(serde_json::json!({
                "id": id,
                "label": n.label.clone().unwrap_or_default(),
                "score": n.coherence_score,
                "verdict": n.asserted,
                "x": x,
                "y": y,
            }))
        })
        .collect();

    Json(serde_json::json!({ "entries": entries, "corpus": corpus })).into_response()
}

// ── POST /api/lab/neighbors — nearest ontology entries to free text ──

#[derive(serde::Deserialize)]
struct NeighborsReq {
    text: String,
    #[serde(default = "default_k")]
    k: usize,
}

fn default_k() -> usize {
    12
}

async fn lab_neighbors(State(state): State<Shared>, Json(req): Json<NeighborsReq>) -> Response {
    let s = state.read().unwrap();
    let q = s.embedder.embed(&req.text);
    let mut scored: Vec<(f32, &str, &str, &str)> = Vec::new();
    for cell in &s.classifier.cells {
        for (i, emb) in cell.embeddings.iter().enumerate() {
            let sim = crate::models::cosine_sim(&q, emb);
            let name = cell.entries.get(i).map(String::as_str).unwrap_or("");
            scored.push((sim, name, &cell.domain, &cell.mode));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(req.k.min(50));
    let out: Vec<serde_json::Value> = scored
        .iter()
        .map(|(sim, name, domain, mode)| {
            serde_json::json!({ "name": name, "domain": domain, "mode": mode, "similarity": sim })
        })
        .collect();
    Json(serde_json::json!({ "query": req.text, "neighbors": out })).into_response()
}

// ── POST /api/coherence/check — six-axis verdict on arbitrary text ───

#[derive(serde::Deserialize)]
struct CoherenceCheckReq {
    text: String,
}

async fn coherence_check(
    State(state): State<Shared>,
    Json(req): Json<CoherenceCheckReq>,
) -> Response {
    let s = state.read().unwrap();
    let q = s.embedder.embed(&req.text);

    // Semantic — quality-adjusted best cell score from the classifier.
    let cells = s.classifier.classify(&q);
    let top = cells.first().cloned();
    let semantic = top
        .as_ref()
        .map(|c| {
            let key = format!("{}\x00{}", c.domain, c.mode);
            s.quality.adjust_score(&key, c.score)
        })
        .unwrap_or(0.0);

    // Ontological grounding — cosine to the nearest seed entry anywhere.
    let ontological = s
        .classifier
        .best_entry_sim(&q)
        .map(|(sim, _, _)| sim)
        .unwrap_or(0.0);
    // Logical — distance to registered contradictions and contradicting claims.
    // Claim parties are often recorded without embeddings (the REST surface
    // stores free text), so measure against a just-in-time embedding instead
    // of silently skipping them.
    let mut worst_claim_sim = 0.0f32;
    for c in &s.core.contradictions {
        for party in [&c.claim_a, &c.claim_b] {
            let sim = if !party.embedding.is_empty() {
                crate::models::cosine_sim(&q, &party.embedding)
            } else {
                crate::models::cosine_sim(&q, &s.embedder.embed(&party.claim))
            };
            worst_claim_sim = worst_claim_sim.max(sim);
        }
    }
    for h in s.core.hypotheses.values() {
        for ev in &h.contradicting_evidence {
            if ev.embedding.is_empty() {
                continue;
            }
            worst_claim_sim = worst_claim_sim.max(crate::models::cosine_sim(&q, &ev.embedding));
        }
    }
    // Trained models keep unrelated sentences in a high similarity band
    // (~0.5–0.7 for BGE), so raw cosine overstates collision. Only overlap
    // past a hinge counts as genuine contradiction pressure.
    const CONTRA_HINGE: f32 = 0.72;
    let contra_pressure = ((worst_claim_sim - CONTRA_HINGE) / (1.0 - CONTRA_HINGE)).clamp(0.0, 1.0);
    // High overlap with a known contradiction drags logical coherence down.
    let logical = 1.0 - contra_pressure * 0.9;

    // Empirical — agreement with judged corpus nodes.
    let mut best_success = 0.0f32;
    let mut best_failure = 0.0f32;
    let mut any_success = false;
    let mut any_failure = false;
    for n in s.core.nodes.values() {
        match n.asserted {
            Some(a) if a > 0.0 => {
                any_success = true;
                best_success = best_success.max(crate::models::cosine_sim(&q, &n.embedding));
            }
            Some(a) if a < 0.0 => {
                any_failure = true;
                best_failure = best_failure.max(crate::models::cosine_sim(&q, &n.embedding));
            }
            _ => {}
        }
    }
    let empirical = match (any_success, any_failure) {
        (true, true) => (0.5 + (best_success - best_failure)).clamp(0.0, 1.0),
        (true, false) => best_success.clamp(0.0, 1.0),
        (false, true) => (1.0 - best_failure).clamp(0.0, 1.0),
        (false, false) => 0.5,
    };

    // Procedural — alignment with recorded workflow steps. Cross-domain
    // cosines under a semantic model hover around 0.5, so only a clearly
    // matched step lifts the score above neutral.
    let mut procedural = 0.5f32;
    let mut proc_hits: Vec<String> = Vec::new();
    for cycle in &s.core.process_cycles {
        if let Some(plan) = &cycle.plan {
            for t in &plan.tasks {
                let tv = s.embedder.embed(&t.title);
                let sim = crate::models::cosine_sim(&q, &tv);
                if sim > procedural {
                    procedural = sim;
                    proc_hits.push(t.title.clone());
                }
            }
        }
    }
    if procedural < 0.6 {
        procedural = 0.5;
        proc_hits.clear();
    }

    // Temporal — recency of epistemic events touching similar subjects.
    let temporal = {
        use crate::epistemic::EpistemicEventType;
        let recent = s
            .core
            .epistemic_audit
            .events
            .iter()
            .rev()
            .take(50)
            .any(|e| e.event_type != EpistemicEventType::BeliefFormed);
        if recent {
            0.6
        } else {
            0.5
        }
    };

    let profile = crate::coherence_dimensions::CoherenceProfile {
        semantic: crate::coherence_dimensions::CoherenceDimension::new("semantic", semantic, 1.0),
        logical: crate::coherence_dimensions::CoherenceDimension::new("logical", logical, 1.2),
        temporal: crate::coherence_dimensions::CoherenceDimension::new("temporal", temporal, 0.8),
        causal: crate::coherence_dimensions::CoherenceDimension::neutral("causal"),
        procedural: crate::coherence_dimensions::CoherenceDimension::new(
            "procedural",
            procedural,
            0.8,
        ),
        empirical: crate::coherence_dimensions::CoherenceDimension::new(
            "empirical",
            empirical,
            1.2,
        ),
    };

    let composite = profile.composite();
    let verdict = if logical < 0.45 && composite < 0.6 {
        "contradicted"
    } else if composite >= 0.65 {
        "coherent"
    } else if composite >= 0.45 {
        "tension"
    } else {
        "incoherent"
    };

    let recall: Vec<serde_json::Value> = s
        .core
        .search_nodes(&q, 5)
        .into_iter()
        .map(|(id, label, score)| serde_json::json!({ "id": id, "label": label, "score": score }))
        .collect();

    let top_cells: Vec<serde_json::Value> = cells
        .iter()
        .take(6)
        .map(|c| {
            serde_json::json!({
                "domain": c.domain,
                "mode": c.mode,
                "score": c.score,
            })
        })
        .collect();

    Json(serde_json::json!({
        "profile": {
            "semantic": semantic,
            "ontological": ontological,
            "logical": logical,
            "empirical": empirical,
            "procedural": procedural,
            "temporal": temporal,
        },
        "composite": composite,
        "verdict": verdict,
        "top_cells": top_cells,
        "recall": recall,
        "contradiction_pressure": contra_pressure,
        "process_hits": proc_hits,
    }))
    .into_response()
}

// ── GET /api/core/dups — near-duplicate clusters in the corpus ───────

#[derive(serde::Deserialize)]
struct DupsQuery {
    #[serde(default = "default_dup_threshold")]
    threshold: f32,
}

fn default_dup_threshold() -> f32 {
    0.92
}

async fn dup_clusters(
    State(state): State<Shared>,
    axum::extract::Query(q): axum::extract::Query<DupsQuery>,
) -> Response {
    let s = state.read().unwrap();
    let labeled: Vec<&CoherenceNode> = s
        .core
        .nodes
        .values()
        .filter(|n| n.label.is_some())
        .collect();

    // Union-find over pairwise similarities above threshold.
    let mut parent: Vec<usize> = (0..labeled.len()).collect();
    fn find(parent: &mut Vec<usize>, i: usize) -> usize {
        if parent[i] != i {
            let root = find(parent, parent[i]);
            parent[i] = root;
        }
        parent[i]
    }
    for i in 0..labeled.len() {
        for j in (i + 1)..labeled.len() {
            let sim = crate::models::cosine_sim(&labeled[i].embedding, &labeled[j].embedding);
            if sim >= q.threshold {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[rj] = ri;
                }
            }
        }
    }

    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..labeled.len() {
        groups.entry(find(&mut parent, i)).or_default().push(i);
    }
    let clusters: Vec<serde_json::Value> = groups
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|g| {
            serde_json::json!(g
                .iter()
                .map(|&i| serde_json::json!({
                    "id": labeled[i].id,
                    "label": labeled[i].label,
                }))
                .collect::<Vec<_>>())
        })
        .collect();

    Json(serde_json::json!({
        "threshold": q.threshold,
        "scanned": labeled.len(),
        "clusters": clusters,
    }))
    .into_response()
}
