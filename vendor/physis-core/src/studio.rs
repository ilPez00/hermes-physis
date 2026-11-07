//! Ontology Studio — a self-contained web GUI (axum + embedded single-page app)
//! for browsing, editing, classifying against, and feeding back on the semiotic
//! grid. Run with `physis-core studio --port 3000`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::classify::CellClassifier;
use crate::core::PhysisCore;
use crate::discovery::{self, DiscoveryConfig, DiscoveryReport};
use crate::embed::{RandomProjectionEmbedder, VectorEmbed};
use crate::models::*;
use crate::ontology::OntologyLoader;
use crate::quality::QualityTracker;
use crate::store;

const APP_HTML: &str = include_str!("studio/app.html");
const APP_CSS: &str = include_str!("studio/app.css");
const APP_JS: &str = include_str!("studio/app.js");

/// Shared, mutable studio state.
pub struct StudioState {
    pub embedder: Box<dyn VectorEmbed>,
    /// "random-projection" or "onnx"; surfaced so the UI can label vectors.
    pub embedder_kind: String,
    /// True when a real model is loaded (semantic), vs deterministic fallback.
    pub semantic: bool,
    pub ontology: OntologyLoader,
    /// Custom entries edited in the UI, persisted to custom_ontology.json.
    pub custom_entries: HashMap<String, DomainDef>,
    pub classifier: CellClassifier,
    pub quality: QualityTracker,
    /// The coherence-node graph — the same one the CLI's scan/search/dream
    /// commands read, so the studio and the CLI share one corpus.
    pub core: PhysisCore,
    pub data_dir: PathBuf,
    /// Gantt chart for project timeline tracking.
    pub gantt: Option<GanttChart>,
}

impl StudioState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self::with_embedder(data_dir, None)
    }

    /// Build the studio with an embedder. When `model_dir` is Some and the
    /// `embed-onnx` feature is enabled, an ONNX MiniLM embedder is attempted;
    /// otherwise (or on failure) it falls back to deterministic random projection.
    pub fn with_embedder(data_dir: PathBuf, model_dir: Option<String>) -> Self {
        let (embedder, embedder_kind, semantic) = build_embedder(model_dir);
        let mut ontology = OntologyLoader::load_all();

        // Load persisted custom entries (from previous studio sessions).
        let custom_entries = load_custom_entries(&data_dir);
        if !custom_entries.is_empty() {
            ontology.merge_custom("studio", custom_entries.clone());
        }

        let classifier = CellClassifier::build(&ontology, embedder.as_ref());
        let quality = QualityTracker::load_or_new(&quality_path(&data_dir));
        let mut core = load_core(&data_dir);
        core.set_embedder_id(embedder_kind.clone());
        Self {
            embedder,
            embedder_kind,
            semantic,
            ontology,
            classifier,
            custom_entries,
            quality,
            core,
            data_dir,
            gantt: None,
        }
    }

    /// Rebuild the classifier (call after ontology edits) and save custom entries.
    fn rebuild(&mut self) {
        let mut ontology = OntologyLoader::load_all();
        ontology.merge_custom("studio", self.custom_entries.clone());
        self.ontology = ontology;
        self.classifier = CellClassifier::build(&self.ontology, self.embedder.as_ref());
        let _ = save_custom_entries(&self.data_dir, &self.custom_entries);
    }

    fn save_quality(&self) {
        let _ = self.quality.save(&quality_path(&self.data_dir));
    }

    pub fn save_core(&self) {
        let _ = save_core(&self.data_dir, &self.core);
    }
}

/// Resolve the active embedder: ONNX when requested + available, else RP.
///
/// Model directory resolution order: explicit argument (CLI `--model`),
/// `PHYSIS_MODEL_DIR`, then `./models` when it looks like a model home.
/// `PHYSIS_EMBEDDER=random-projection` forces the offline embedder even when
/// a model is present.
#[cfg_attr(not(feature = "embed-onnx"), allow(unused_variables))]
fn build_embedder(model_dir: Option<String>) -> (Box<dyn VectorEmbed>, String, bool) {
    #[cfg(feature = "embed-onnx")]
    {
        let requested = std::env::var("PHYSIS_EMBEDDER")
            .map(|v| v != "random-projection" && v.trim() != "off")
            .unwrap_or(true);
        let dir = model_dir
            .or_else(|| std::env::var("PHYSIS_MODEL_DIR").ok())
            .or_else(|| {
                let p = std::path::Path::new("models");
                (p.join("model.onnx").exists() || p.join("onnx/model.onnx").exists())
                    .then(|| "models".to_string())
            });
        if requested {
            if let Some(dir) = dir {
                // A 768-d model truncated into a 384-d buffer silently throws
                // away half its signal — read the native width when exported
                // alongside a transformers config.json.
                let dim = crate::embed_onnx::native_dim(&dir).unwrap_or(384);
                let onnx =
                    crate::embed_onnx::OnnxEmbedder::with_config(&crate::embed_onnx::OnnxConfig {
                        dim,
                        model_dir: Some(dir.clone()),
                        ..crate::embed_onnx::OnnxConfig::default()
                    });
                if onnx.is_available() {
                    eprintln!(
                        "Ontology Studio embedder: ONNX ({dir}, {} dims)",
                        onnx.dimension()
                    );
                    return (Box::new(onnx), "onnx".to_string(), true);
                }
                eprintln!("warning: no ONNX model at {dir}; falling back to random projection");
            }
        }
    }
    let rp = RandomProjectionEmbedder::new(384);
    (Box::new(rp), "random-projection".to_string(), false)
}

pub type Shared = Arc<RwLock<StudioState>>;

pub async fn run(port: u16) -> anyhow::Result<()> {
    run_with_model(port, None).await
}

/// Run the studio with an optional ONNX model directory (None = random projection).
pub async fn run_with_model(port: u16, model_dir: Option<String>) -> anyhow::Result<()> {
    let state: Shared = Arc::new(RwLock::new(StudioState::with_embedder(
        store::data_dir(),
        model_dir,
    )));

    let app = Router::new()
        .route("/", get(index))
        .route("/app.css", get(app_css))
        .route("/app.js", get(app_js))
        .route("/api/health", get(health_json))
        .route("/api/console", get(console_json))
        .route("/api/edition", get(edition_json))
        .route("/api/ontology", get(ontology_json))
        .route("/api/grid", get(grid_json))
        .route("/api/classify", post(classify))
        .route("/api/embedder", get(embedder_json))
        .route("/api/ontology/upsert", post(upsert_entry))
        .route("/api/ontology/delete", post(delete_entry))
        .route("/api/ingest", post(ingest_dir))
        .route("/api/map", post(structural_map))
        .route("/api/infra", get(infra_status))
        .route("/api/ingest/promote", post(promote_proposal))
        .route("/api/quality", get(quality_json))
        .route("/api/quality/fail", post(quality_fail))
        .route("/api/quality/pass", post(quality_pass))
        .route("/api/core/snapshot", get(core_snapshot))
        .route("/api/core/nodes", get(core_nodes))
        .route("/api/core/scan", post(core_scan))
        .route("/api/core/search", post(core_search))
        .route("/api/core/assert", post(core_assert))
        .route("/api/core/dream", post(core_dream))
        .route(
            "/api/v1/communities",
            get(crate::studio_communities::api_communities),
        )
        // MVP Node & Importers API
        .route("/api/v1/coherence/edit", post(api_node_edit))
        .route("/api/v1/coherence/delete", post(api_node_delete))
        .route("/api/v1/search/nodes", post(api_node_search))
        .route("/api/v1/vault/import", post(api_vault_import))
        .route("/api/v1/history/import", post(api_history_import))
        .route("/api/v1/praxis/backfill", post(api_praxis_backfill))
        // Epistemic Hypotheses & Explanations API
        .route(
            "/api/v1/hypotheses",
            get(api_hypotheses_list).post(api_hypothesis_create),
        )
        .route("/api/v1/hypotheses/:id", get(api_hypothesis_get))
        .route(
            "/api/v1/hypotheses/:id/evidence",
            post(api_hypothesis_evidence),
        )
        .route(
            "/api/v1/hypotheses/:id/prediction",
            post(api_hypothesis_prediction),
        )
        .route(
            "/api/v1/hypotheses/:id/transition",
            post(api_hypothesis_transition),
        )
        .route("/api/v1/explanation/:id", get(api_explanation_get))
        // Contradictions API
        .route(
            "/api/v1/contradictions",
            get(api_contradictions_list).post(api_contradiction_create),
        )
        .route(
            "/api/v1/contradictions/:id/resolve",
            post(api_contradiction_resolve),
        )
        // Epistemic Audit & Replay API
        .route("/api/v1/epistemic/audit", get(api_epistemic_audit))
        .route("/api/v1/epistemic/replay", get(api_epistemic_replay))
        .route("/api/v1/ontology/discover", post(api_ontology_discover))
        .merge(crate::studio_lab::router())
        .with_state(state);

    // Loopback by default: every ingest/scan route reads arbitrary local paths,
    // so the studio must not be reachable off-box unless someone opts in with
    // PHYSIS_STUDIO_HOST (e.g. 0.0.0.0 inside a container).
    let host = std::env::var("PHYSIS_STUDIO_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("Physis Studio → http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(APP_HTML)
}

async fn app_css() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        APP_CSS,
    )
        .into_response()
}

async fn app_js() -> Response {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        APP_JS,
    )
        .into_response()
}

/// One call the header polls: everything the status bar shows.
async fn health_json(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    Json(serde_json::json!({
        "ok": true,
        "embedder": s.embedder_kind,
        "semantic": s.semantic,
        "dimension": s.embedder.dimension(),
        // Count what the classifier is actually built from. Phase 10c folded
        // `machine_domains` into `classification_domains()`, so this now equals
        // `all_domains().len()` — the old "advertised 730 vs 623 scoreable"
        // gap is closed. Keep the count tied to what feeds the grid.
        "entries": s.ontology.entry_count(),
        "custom": s.custom_entries.len(),
        "cells": s.classifier.cell_count(),
        "nodes": s.core.nodes.len(),
        "coherence_index": s.core.coherence_index(),
        "failures": s.quality.failures.len(),
        "penalties": s.quality.cell_penalties.len(),
        "data_dir": s.data_dir.display().to_string(),
    }))
    .into_response()
}

async fn console_json(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    Json(serde_json::json!({
        "ok": true,
        "gantt": s.gantt.as_ref().map(|g| {
            (
                g.tasks.len(),
                g.critical_path.len(),
            )
        }).unwrap_or((0, 0)),
        "classifier_cells": s.classifier.cell_count(),
        "ontology_domains": s.ontology.all_domains().len(),
        "custom_entries": s.custom_entries.len(),
    }))
    .into_response()
}

/// GET /api/edition — what is installed on this machine, and what Pro adds.
///
/// The studio is Core's UI, but a Core user has no way to find out that Pro
/// exists, or that they already have it, without leaving the app. This answers
/// both from the same detection the `physis` front door uses, so the CLI banner
/// and the studio's upgrade panel can never disagree.
///
/// Core links nothing from Pro — it reports paths on disk. Whether a licence is
/// valid stays Pro's business; Core does not hold the verifying key.
async fn edition_json() -> Response {
    let e = crate::edition::Edition::detect();
    Json(serde_json::json!({
        "name": e.name(),
        "has_pro": e.has_pro(),
        "pro_cli": e.pro_cli.as_ref().map(|p| p.display().to_string()),
        "pro_web": e.pro_web.as_ref().map(|p| p.display().to_string()),
        "upgrade_url": crate::edition::UPGRADE_URL,
        "adds": crate::edition::PRO_SUMMARY
            .iter()
            .map(|(title, detail)| serde_json::json!({ "title": title, "detail": detail }))
            .collect::<Vec<_>>(),
    }))
    .into_response()
}

#[derive(serde::Serialize)]
struct EmbedderJson {
    kind: String,
    semantic: bool,
    dimension: usize,
}

async fn embedder_json(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    Json(EmbedderJson {
        kind: s.embedder_kind.clone(),
        semantic: s.semantic,
        dimension: s.embedder.dimension(),
    })
    .into_response()
}

// ── Serialization DTOs ───────────────────────────────────────────────

#[derive(serde::Serialize)]
struct OntologyJson {
    categories: Vec<CategoryJson>,
    entry_count: usize,
    custom_count: usize,
}

#[derive(serde::Serialize)]
struct CategoryJson {
    name: String,
    entries: Vec<EntryJson>,
}

#[derive(serde::Serialize)]
struct EntryJson {
    name: String,
    category: Option<String>,
    domain: String,
    mode: String,
    axis_kind: String,
    axis_name: String,
    unit: String,
    hints: Vec<String>,
    /// Orthogonal facets (lifecycle/agency/scale/abstraction + sub-domain/mode).
    /// Sent so the edit modal can prefill them — without this the studio would
    /// silently blank an entry's facets every time it was saved.
    facets: Facets,
    /// True when this entry was edited in the studio (persisted custom).
    custom: bool,
}

fn entry_json(name: &str, d: &DomainDef, custom: bool) -> EntryJson {
    EntryJson {
        name: name.to_string(),
        category: d.category.clone(),
        domain: d.domain.clone().unwrap_or_default(),
        mode: d.mode.clone().unwrap_or_default(),
        axis_kind: d.axis_kind.clone().unwrap_or_default(),
        axis_name: d.axis_name.clone().unwrap_or_default(),
        unit: d.unit.clone(),
        hints: d.hints.clone(),
        facets: d.facets.clone(),
        custom,
    }
}

async fn ontology_json(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let all = s.ontology.all_domains();
    let mut cats: HashMap<String, Vec<EntryJson>> = HashMap::new();
    let mut uncat = Vec::new();
    for (k, d) in &all {
        let ej = entry_json(k, d, s.custom_entries.contains_key(k));
        match d.category.clone() {
            Some(c) => cats.entry(c).or_default().push(ej),
            None => uncat.push(ej),
        }
    }
    let mut categories: Vec<CategoryJson> = cats
        .into_iter()
        .map(|(name, mut entries)| {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            CategoryJson { name, entries }
        })
        .collect();
    categories.sort_by(|a, b| a.name.cmp(&b.name));
    if !uncat.is_empty() {
        categories.insert(
            0,
            CategoryJson {
                name: "(uncategorized)".into(),
                entries: uncat,
            },
        );
    }
    Json(OntologyJson {
        categories,
        entry_count: all.len(),
        custom_count: s.custom_entries.len(),
    })
    .into_response()
}

#[derive(serde::Serialize)]
struct GridJson {
    domains: Vec<AxisJson>,
    modes: Vec<AxisJson>,
    cells: Vec<GridCellJson>,
    /// Heatmap matrix [domain][mode].
    heatmap: Vec<Vec<u32>>,
}

/// One row or column of the grid.
#[derive(serde::Serialize)]
struct AxisJson {
    name: String,
    /// False for the built-in HumanDomain/HumanMode variants, true for an axis
    /// the loaded ontology introduced — an edited entry, a promoted discovery
    /// proposal, or a config using vocabulary outside the 5×14 block (see
    /// config/ontology_facets.example.json: EXCHANGE×PERSUADE, DEFEND×TEACH).
    extra: bool,
    /// Entries anywhere on this row/column — how much of the grid it carries.
    total: u32,
}

#[derive(serde::Serialize)]
struct GridCellJson {
    domain: String,
    mode: String,
    entries: Vec<String>,
    count: u32,
}

/// Build one axis: the canonical variants in their declared order, then every
/// value the ontology actually uses that isn't one of them, sorted.
///
/// The grid is a view of the loaded ontology, not of the enums. Deriving the
/// axes from `HumanDomain::all()` alone silently hid every entry outside the
/// 5×14 block: `classify` keys cells on whatever strings an entry carries, so
/// a promoted discovery proposal or an edited entry could score on a cell the
/// UI had no row for — invisible, uneditable, unauditable.
fn build_axis(canonical: &[&str], used: &HashMap<String, u32>) -> Vec<AxisJson> {
    let mut axis: Vec<AxisJson> = canonical
        .iter()
        .map(|name| AxisJson {
            name: (*name).to_string(),
            extra: false,
            total: used.get(*name).copied().unwrap_or(0),
        })
        .collect();
    let mut extras: Vec<(&String, &u32)> = used
        .iter()
        .filter(|(name, _)| !canonical.contains(&name.as_str()))
        .collect();
    extras.sort_by(|a, b| a.0.cmp(b.0));
    axis.extend(extras.into_iter().map(|(name, total)| AxisJson {
        name: name.clone(),
        extra: true,
        total: *total,
    }));
    axis
}

async fn grid_json(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();

    // Build a HashMap<(domain, mode), Vec<name>> from classification ontology,
    // counting per-axis totals as we go.
    let mut cell_map: HashMap<(String, String), Vec<String>> = HashMap::new();
    let mut domain_totals: HashMap<String, u32> = HashMap::new();
    let mut mode_totals: HashMap<String, u32> = HashMap::new();
    for def in s.ontology.classification_domains() {
        if let (Some(d), Some(m)) = (def.domain.as_deref(), def.mode.as_deref()) {
            cell_map
                .entry((d.to_string(), m.to_string()))
                .or_default()
                .push(def.name.clone());
            *domain_totals.entry(d.to_string()).or_default() += 1;
            *mode_totals.entry(m.to_string()).or_default() += 1;
        }
    }

    let canonical_domains: Vec<&str> = HumanDomain::all().iter().map(|d| d.as_str()).collect();
    let canonical_modes: Vec<&str> = HumanMode::all().iter().map(|m| m.as_str()).collect();
    let domains = build_axis(&canonical_domains, &domain_totals);
    let modes = build_axis(&canonical_modes, &mode_totals);

    let mut heatmap = vec![vec![0u32; modes.len()]; domains.len()];
    let mut cells = Vec::new();
    for (di, d) in domains.iter().enumerate() {
        for (mi, m) in modes.iter().enumerate() {
            let mut entries = cell_map
                .get(&(d.name.clone(), m.name.clone()))
                .cloned()
                .unwrap_or_default();
            entries.sort();
            let count = entries.len() as u32;
            cells.push(GridCellJson {
                domain: d.name.clone(),
                mode: m.name.clone(),
                entries,
                count,
            });
            heatmap[di][mi] = count;
        }
    }
    Json(GridJson {
        domains,
        modes,
        cells,
        heatmap,
    })
    .into_response()
}

#[derive(serde::Deserialize)]
struct ClassifyReq {
    text: String,
}

#[derive(serde::Serialize)]
struct ClassifyResp {
    query_embedding_dim: usize,
    results: Vec<CellScoreJson>,
    quality_penalties: HashMap<String, f32>,
    /// Nearest single ontology entry — the sanity check on whether the top cell
    /// won on real similarity or on an empty grid.
    best_entry: Option<BestEntryJson>,
    /// Recalled corpus nodes closest to the same text, so a classification can
    /// be read next to the documents that back it.
    recall: Vec<RecallJson>,
}

#[derive(serde::Serialize)]
struct BestEntryJson {
    similarity: f32,
    domain: String,
    mode: String,
}

#[derive(serde::Serialize)]
struct CellScoreJson {
    domain: String,
    mode: String,
    raw_score: f32,
    adjusted_score: f32,
    entries: Vec<String>,
    /// Facets carried by the entries in this cell (the dimensions the 2-D grid
    /// cannot hold) — deduped for display.
    facets: Vec<String>,
}

/// Facet values on a cell, flattened to `kind:value` strings for the UI.
fn facet_labels(f: &Facets) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(v) = &f.lifecycle {
        out.push(format!("lifecycle:{v:?}"));
    }
    if let Some(v) = &f.agency {
        out.push(format!("agency:{v:?}"));
    }
    if let Some(v) = &f.scale {
        out.push(format!("scale:{v:?}"));
    }
    if let Some(v) = &f.abstraction {
        out.push(format!("abstraction:{v:?}"));
    }
    if let Some(v) = &f.sub_domain {
        out.push(format!("sub-domain:{v}"));
    }
    if let Some(v) = &f.sub_mode {
        out.push(format!("sub-mode:{v}"));
    }
    out
}

async fn classify(State(state): State<Shared>, Json(req): Json<ClassifyReq>) -> Response {
    let s = state.read().unwrap();
    let embedding = s.embedder.embed(&req.text);
    let results = s.classifier.classify(&embedding);
    let resp = ClassifyResp {
        query_embedding_dim: s.embedder.dimension(),
        results: results
            .iter()
            .take(10)
            .map(|r| {
                let key = format!("{}\x00{}", r.domain, r.mode);
                CellScoreJson {
                    domain: r.domain.clone(),
                    mode: r.mode.clone(),
                    raw_score: r.score,
                    adjusted_score: s.quality.adjust_score(&key, r.score),
                    entries: r.entries.iter().take(12).cloned().collect(),
                    facets: facet_labels(&r.facets),
                }
            })
            .collect(),
        quality_penalties: s.quality.cell_penalties.clone(),
        best_entry: s
            .classifier
            .best_entry_sim(&embedding)
            .map(|(similarity, domain, mode)| BestEntryJson {
                similarity,
                domain,
                mode,
            }),
        recall: s
            .core
            .search_nodes(&embedding, 5)
            .into_iter()
            .map(|(id, label, score)| RecallJson::new(&s.core, id, label, score))
            .collect(),
    };
    Json(resp).into_response()
}

#[derive(serde::Deserialize)]
struct EntryReq {
    name: String,
    category: Option<String>,
    domain: String,
    mode: String,
    // The editor always posts these four, but the endpoint is also the scripting
    // surface for the ontology. Without `default` an omitted `unit` rejected the
    // whole request with a deserialize error, so the minimum useful payload was
    // seven fields wide when only name/domain/mode carry meaning. They default to
    // empty, which is exactly what the editor sends for "not set".
    #[serde(default)]
    axis_kind: String,
    #[serde(default)]
    axis_name: String,
    #[serde(default)]
    unit: String,
    #[serde(default)]
    hints: String,
    /// Orthogonal facets, deserialized by the same `Facets` impl the ontology
    /// JSON loader uses — so the studio and the config files cannot drift on
    /// spelling. Absent/partial objects fall back to `None` per field.
    #[serde(default)]
    facets: Facets,
}

async fn upsert_entry(State(state): State<Shared>, Json(req): Json<EntryReq>) -> Response {
    if req.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "name required").into_response();
    }
    let hints: Vec<String> = req
        .hints
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let def = DomainDef {
        name: req.name.clone(),
        category: req.category.clone().filter(|c| !c.trim().is_empty()),
        domain: Some(req.domain.clone()),
        mode: Some(req.mode.clone()),
        axis_kind: Some(req.axis_kind.clone()),
        axis_name: Some(req.axis_name.clone()),
        unit: req.unit.clone(),
        facets: req.facets.clone(),
        hints,
    };
    {
        let mut s = state.write().unwrap();
        s.custom_entries.insert(req.name.clone(), def);
        s.rebuild();
    }
    Json(serde_json::json!({ "ok": true, "upserted": req.name })).into_response()
}

#[derive(serde::Deserialize)]
struct DelReq {
    name: String,
}

async fn delete_entry(State(state): State<Shared>, Json(req): Json<DelReq>) -> Response {
    let mut s = state.write().unwrap();
    let removed = s.custom_entries.remove(&req.name).is_some();
    s.rebuild();
    Json(serde_json::json!({ "ok": removed, "removed": req.name })).into_response()
}

/// Text extensions ingested by the studio discovery pass.
const TEXT_EXTS: &[&str] = &[
    "txt", "md", "rs", "py", "json", "toml", "csv", "log", "xml", "yml", "yaml",
];

#[derive(serde::Deserialize)]
struct IngestReq {
    dir: String,
}

#[derive(serde::Deserialize)]
struct MapReq {
    dir: String,
}

#[derive(serde::Serialize)]
struct IngestResp {
    files: usize,
    lines: usize,
    report: DiscoveryReport,
}

/// POST /api/map — the corpus structural map as JSON (Demo A surface of Phase
/// 6). The Studio reuses the same `map::build_map` as the CLI, so the page
/// and the shell always agree: repeats, differences, contradictions,
/// provenance, and the reproducibility hash.
async fn structural_map(State(state): State<Shared>, Json(req): Json<MapReq>) -> Response {
    let dir = std::path::PathBuf::from(&req.dir);
    if !dir.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            format!("not a directory: {}", dir.display()),
        )
            .into_response();
    }
    let s = state.read().unwrap();
    let docs = match crate::map::load_corpus(&dir) {
        Ok(d) => d,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("corpus load: {e}")).into_response();
        }
    };
    let report = match crate::map::build_map(&docs, s.embedder.as_ref(), None) {
        Ok(r) => r,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("map: {e}")).into_response();
        }
    };
    match serde_json::to_string(&report) {
        Ok(body) => ([("content-type", "application/json")], body).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize: {e}")).into_response(),
    }
}

/// GET /api/infra — model + n-gram + embedder registry status (the
/// introspection leg of Phase 6). Selection state the UI can bind to: the
/// Studio shows real artifacts, never invented backends.
async fn infra_status(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let nreg =
        crate::ngram_table::TableRegistry::new(crate::ngram_table::TableRegistry::default_root());
    let mreg = crate::model_provider::ModelRegistry::new(
        crate::model_provider::ModelRegistry::default_root(),
    );
    let tables: Vec<String> = nreg.list().iter().map(|(id, _)| id.clone()).collect();
    let models: Vec<String> = mreg.list().iter().map(|m| m.record.id.clone()).collect();
    let body = serde_json::json!({
        "embedder": s.embedder_kind,
        "semantic": s.semantic,
        "tables": tables,
        "models": models,
        "physis": env!("CARGO_PKG_VERSION"),
    });
    match serde_json::to_string(&body) {
        Ok(b) => ([("content-type", "application/json")], b).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize: {e}")).into_response(),
    }
}

async fn ingest_dir(State(state): State<Shared>, Json(req): Json<IngestReq>) -> Response {
    let dir = std::path::PathBuf::from(&req.dir);
    if !dir.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            format!("not a directory: {}", dir.display()),
        )
            .into_response();
    }
    let s = state.read().unwrap();

    // Walk the dir, read text files, split into corpus lines.
    let mut lines: Vec<String> = Vec::new();
    let mut files = 0usize;
    for path in collect_text_files(&dir) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        files += 1;
        for line in text.lines() {
            let line = line.trim();
            if line.len() > 12 && line.chars().count() < 300 {
                lines.push(line.to_string());
            }
        }
    }

    let cfg = DiscoveryConfig::default();
    let report = discovery::discover(&lines, &s.classifier, s.embedder.as_ref(), &cfg);
    Json(IngestResp {
        files,
        lines: lines.len(),
        report,
    })
    .into_response()
}

#[derive(serde::Deserialize)]
struct PromoteReq {
    name: String,
    category: Option<String>,
    domain: String,
    mode: String,
    hints: Vec<String>,
    samples: Vec<String>,
}

async fn promote_proposal(State(state): State<Shared>, Json(req): Json<PromoteReq>) -> Response {
    let mut s = state.write().unwrap();
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return (StatusCode::BAD_REQUEST, "proposal name required").into_response();
    }
    let def = DomainDef {
        name: name.clone(),
        category: req.category.or(Some("Discovered".into())),
        domain: Some(req.domain),
        mode: Some(req.mode),
        axis_kind: Some("discovered".into()),
        axis_name: Some("discovered".into()),
        unit: "items".into(),
        hints: req.hints,
        facets: Facets::default(),
    };
    s.custom_entries.insert(name.clone(), def);
    s.rebuild();
    Json(serde_json::json!({ "ok": true, "promoted": name, "samples": req.samples }))
        .into_response()
}

#[derive(serde::Serialize)]
struct QualityJson {
    failures: Vec<serde_json::Value>,
    penalties: HashMap<String, f32>,
    boosts: HashMap<String, f32>, // Will be computed from BoostInfo below
}

async fn quality_json(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let failures: Vec<serde_json::Value> = s
        .quality
        .failures
        .iter()
        .rev()
        .take(20)
        .map(|f| {
            serde_json::json!({
                "id": &f.id[..8.min(f.id.len())],
                "feedback": f.feedback.chars().take(120).collect::<String>(),
                "top_cell": f.top_cell,
                "top_score": f.top_score,
                "severity": f.severity,
                "timestamp": f.timestamp.to_rfc3339(),
            })
        })
        .collect();
    let boosts_map: HashMap<String, f32> = s
        .quality
        .cell_boosts
        .iter()
        .map(|(k, b)| (k.clone(), *b))
        .collect();
    Json(QualityJson {
        failures,
        penalties: s.quality.cell_penalties.clone(),
        boosts: boosts_map,
    })
    .into_response()
}

#[derive(serde::Deserialize)]
struct FailReq {
    feedback: String,
}

async fn quality_fail(State(state): State<Shared>, Json(req): Json<FailReq>) -> Response {
    let mut s = state.write().unwrap();
    let centroids = s.classifier.cell_centroids();
    let (top_cell, top_score, severity) = {
        let f = s.quality.report_failure(&req.feedback, &centroids, None);
        (f.top_cell.clone(), f.top_score, f.severity)
    };
    s.save_quality();
    Json(serde_json::json!({
        "ok": true,
        "top_cell": top_cell,
        "top_score": top_score,
        "severity": severity,
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct PassReq {
    cell: String,
}

async fn quality_pass(State(state): State<Shared>, Json(req): Json<PassReq>) -> Response {
    let mut s = state.write().unwrap();
    s.quality.report_success(&req.cell);
    s.save_quality();
    Json(serde_json::json!({ "ok": true, "boosted": req.cell })).into_response()
}

// ── Coherence graph (corpus) ─────────────────────────────────────────
//
// The CLI could already scan a directory into coherence nodes, search them,
// judge them, and dream over the failures — but none of it was reachable from
// the studio, so the GUI could only ever talk about the ontology in the
// abstract. These routes put the corpus and the grid on the same screen.

/// One recalled node, with the two axes the engine keeps separate: derived
/// density (`coherence`) and the human verdict (`asserted`).
#[derive(serde::Serialize)]
struct RecallJson {
    id: String,
    label: String,
    score: f32,
    coherence: f32,
    asserted: Option<f32>,
}

impl RecallJson {
    fn new(core: &PhysisCore, id: String, label: String, score: f32) -> Self {
        let node = core.nodes.get(&id);
        Self {
            coherence: node.map(|n| n.coherence_score).unwrap_or_default(),
            asserted: node.and_then(|n| n.asserted),
            // Labels are whole file excerpts; the UI only ever renders a line.
            label: label.chars().take(240).collect(),
            id,
            score,
        }
    }
}

async fn core_snapshot(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    Json(s.core.snapshot()).into_response()
}

#[derive(serde::Deserialize)]
struct NodesQuery {
    #[serde(default = "default_max")]
    max: usize,
    /// "all" | "unjudged" | "failure" — the assert queue defaults to unjudged.
    #[serde(default)]
    filter: String,
}

fn default_max() -> usize {
    30
}

async fn core_nodes(
    State(state): State<Shared>,
    axum::extract::Query(q): axum::extract::Query<NodesQuery>,
) -> Response {
    let s = state.read().unwrap();
    let mut nodes: Vec<&CoherenceNode> = s
        .core
        .nodes
        .values()
        .filter(|n| n.label.is_some())
        .filter(|n| match q.filter.as_str() {
            "unjudged" => n.asserted.is_none(),
            "failure" => n.is_asserted_failure(),
            _ => true,
        })
        .collect();
    // Weakest first: the nodes that most need a human verdict lead the queue.
    nodes.sort_by(|a, b| {
        a.coherence_score
            .partial_cmp(&b.coherence_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let out: Vec<RecallJson> = nodes
        .into_iter()
        .take(q.max.min(200))
        .map(|n| {
            RecallJson::new(
                &s.core,
                n.id.clone(),
                n.label.clone().unwrap_or_default(),
                n.coherence_score,
            )
        })
        .collect();
    Json(serde_json::json!({ "nodes": out })).into_response()
}

#[derive(serde::Deserialize)]
struct ScanReq {
    dir: String,
}

async fn core_scan(State(state): State<Shared>, Json(req): Json<ScanReq>) -> Response {
    let dir = std::path::PathBuf::from(&req.dir);
    if !dir.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            format!("not a directory: {}", dir.display()),
        )
            .into_response();
    }
    let files = collect_text_files(&dir);
    let scanned = files.len();
    let mut skipped = 0usize;
    let mut labels: Vec<String> = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            skipped += 1;
            continue;
        };
        let chunks = chunk_text(&text);
        if chunks.is_empty() {
            skipped += 1;
            continue;
        }
        for (i, chunk) in chunks.iter().enumerate() {
            // The label is both the display text and the dedupe key, so it
            // carries the passage index — re-scanning a file updates nothing
            // and duplicates nothing.
            labels.push(format!("{}#{i}: {chunk}", path.display()));
        }
    }
    let capped = labels.len() > MAX_SCAN_NODES;
    labels.truncate(MAX_SCAN_NODES);

    // Embed under a READ lock: this is the slow half, and holding the write
    // lock across it would freeze every other request — including the health
    // poll the header runs — for the whole scan.
    let embeddings: Vec<Vec<f32>> = {
        let s = state.read().unwrap();
        labels.iter().map(|l| s.embedder.embed(l)).collect()
    };

    let mut s = state.write().unwrap();
    // Labels dedupe, so the honest count is how much the graph actually grew —
    // a second scan of the same folder reports 0 new, not "registered" again.
    let before = s.core.nodes.len();
    let seen = labels.len();
    for (label, embedding) in labels.into_iter().zip(embeddings) {
        s.core.register_node_vec_labeled(embedding, Some(label));
    }
    let registered = s.core.nodes.len() - before;
    s.save_core();
    let snapshot = s.core.snapshot();
    Json(serde_json::json!({
        "files": scanned,
        "passages": seen,
        "registered": registered,
        "skipped": skipped,
        "capped": capped,
        "max_nodes": MAX_SCAN_NODES,
        "snapshot": snapshot,
    }))
    .into_response()
}

/// Ceiling on passages registered by one scan. Registering a node re-scores it
/// against every existing node, so ingestion is quadratic: an unbounded scan of
/// a source tree wedges the studio for minutes. Point it at a folder that
/// matters, not at a repository root.
const MAX_SCAN_NODES: usize = 2000;

/// Longest passage kept as one node. The CLI's whole-file scan dropped
/// anything over 5000 chars outright, which meant a docs folder of real
/// documents ingested as nothing; splitting keeps long files searchable and
/// keeps each vector about one topic instead of a whole file's average.
const CHUNK_CHARS: usize = 1200;
/// Below this a passage is a heading or a stray line — no recall value.
const MIN_CHUNK_CHARS: usize = 40;

/// Split text into passages on blank lines, packing paragraphs up to
/// `CHUNK_CHARS` and hard-splitting any single paragraph longer than that.
fn chunk_text(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        for piece in hard_split(para) {
            if !buf.is_empty() && buf.chars().count() + piece.chars().count() > CHUNK_CHARS {
                out.push(std::mem::take(&mut buf));
            }
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(&piece);
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out.retain(|c| c.chars().count() >= MIN_CHUNK_CHARS);
    out
}

/// Break one oversized paragraph on char boundaries (never byte offsets —
/// slicing a multi-byte char would panic).
fn hard_split(para: &str) -> Vec<String> {
    if para.chars().count() <= CHUNK_CHARS {
        return vec![para.to_string()];
    }
    para.chars()
        .collect::<Vec<char>>()
        .chunks(CHUNK_CHARS)
        .map(|c| c.iter().collect())
        .collect()
}

#[derive(serde::Deserialize)]
struct SearchReq {
    query: String,
    #[serde(default = "default_search_max")]
    max: usize,
}

fn default_search_max() -> usize {
    10
}

async fn core_search(State(state): State<Shared>, Json(req): Json<SearchReq>) -> Response {
    let s = state.read().unwrap();
    let hits: Vec<RecallJson> = s
        .core
        .search_text(&req.query, s.embedder.as_ref(), req.max.min(100))
        .into_iter()
        .map(|(id, label, score)| RecallJson::new(&s.core, id, label, score))
        .collect();
    Json(serde_json::json!({ "hits": hits, "nodes": s.core.nodes.len() })).into_response()
}

#[derive(serde::Deserialize)]
struct AssertReq {
    id: String,
    verdict: String,
}

/// success/inert/failure → the score the engine stores on the asserted axis.
fn verdict_score(verdict: &str) -> Option<f32> {
    match verdict.to_lowercase().as_str() {
        "success" => Some(1.0),
        "inert" => Some(0.0),
        "failure" => Some(-1.0),
        _ => None,
    }
}

async fn core_assert(State(state): State<Shared>, Json(req): Json<AssertReq>) -> Response {
    let Some(score) = verdict_score(&req.verdict) else {
        return (
            StatusCode::BAD_REQUEST,
            "verdict must be success|inert|failure",
        )
            .into_response();
    };
    let mut s = state.write().unwrap();
    if !s.core.assert_coherence(&req.id, score) {
        return (StatusCode::NOT_FOUND, format!("no node {}", req.id)).into_response();
    }
    s.save_core();
    Json(serde_json::json!({ "ok": true, "asserted": score, "snapshot": s.core.snapshot() }))
        .into_response()
}

async fn core_dream(State(state): State<Shared>) -> Response {
    let mut s = state.write().unwrap();
    let dreams = s.core.dream();
    s.save_core();
    Json(serde_json::json!({ "dreams": dreams, "snapshot": s.core.snapshot() })).into_response()
}

/// Directories a scan must never descend into: build output and dependency
/// trees are megabytes of machine-written text that bury whatever the user
/// actually pointed at.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    "build",
    ".next",
    "vendor",
];

/// Recursive walk returning every readable text file under `dir`.
fn collect_text_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if SKIP_DIRS.contains(&name) {
                    continue;
                }
                stack.push(path);
            } else if TEXT_EXTS.contains(&path.extension().and_then(|e| e.to_str()).unwrap_or("")) {
                out.push(path);
            }
        }
    }
    out
}

// ── Persistence helpers ──────────────────────────────────────────────

fn quality_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("quality.json")
}

fn custom_ontology_file(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("custom_ontology.json")
}

/// Same file the CLI persists to, so `physis-core scan` and the studio's
/// Corpus tab operate on one graph rather than two divergent ones.
fn nodes_file(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("nodes.json")
}

fn load_core(data_dir: &std::path::Path) -> PhysisCore {
    let json = std::fs::read_to_string(nodes_file(data_dir)).unwrap_or_default();
    if json.trim().is_empty() {
        return PhysisCore::new();
    }
    PhysisCore::from_json(&json).unwrap_or_else(|_| PhysisCore::new())
}

fn save_core(data_dir: &std::path::Path, core: &PhysisCore) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(data_dir);
    std::fs::write(nodes_file(data_dir), core.to_json()?)?;
    Ok(())
}

fn load_custom_entries(data_dir: &std::path::Path) -> HashMap<String, DomainDef> {
    let path = custom_ontology_file(data_dir);
    let contents = std::fs::read_to_string(&path).unwrap_or_default();
    if contents.trim().is_empty() {
        return HashMap::new();
    }
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_custom_entries(
    data_dir: &std::path::Path,
    entries: &HashMap<String, DomainDef>,
) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(data_dir);
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(custom_ontology_file(data_dir), json)?;
    Ok(())
}

// ── Additional MVP & Epistemic Handlers ─────────────────────────────────────

#[derive(serde::Deserialize)]
struct ApiNodeEditReq {
    id: String,
    label: String,
    domain: Option<String>,
    mode: Option<String>,
    #[serde(default)]
    clear_pin: bool,
}

async fn api_node_edit(State(state): State<Shared>, Json(req): Json<ApiNodeEditReq>) -> Response {
    let mut s = state.write().unwrap();
    let new_emb = s.embedder.embed(&req.label);
    let pin = if req.clear_pin {
        PinEdit::Clear
    } else if let (Some(d), Some(m)) = (req.domain, req.mode) {
        PinEdit::Set(d, m)
    } else {
        PinEdit::Keep
    };
    let kind = s.embedder_kind.clone();
    match s
        .core
        .edit_node(&req.id, &req.label, new_emb, pin, Some(&kind))
    {
        Ok(outcome) => {
            s.save_core();
            Json(outcome).into_response()
        }
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ApiNodeDeleteReq {
    id: String,
}

async fn api_node_delete(
    State(state): State<Shared>,
    Json(req): Json<ApiNodeDeleteReq>,
) -> Response {
    let mut s = state.write().unwrap();
    if s.core.delete_node(&req.id) {
        s.save_core();
        Json(serde_json::json!({ "ok": true, "deleted": req.id })).into_response()
    } else {
        (StatusCode::NOT_FOUND, format!("node {} not found", req.id)).into_response()
    }
}

#[derive(serde::Deserialize)]
struct ApiNodeSearchReq {
    query: String,
    #[serde(default = "default_token_budget")]
    budget: usize,
    #[serde(default = "default_search_max")]
    top: usize,
}

fn default_token_budget() -> usize {
    600
}

async fn api_node_search(
    State(state): State<Shared>,
    Json(req): Json<ApiNodeSearchReq>,
) -> Response {
    let s = state.read().unwrap();
    let q_emb = s.embedder.embed(&req.query);
    let candidates: Vec<(usize, String, Vec<f32>)> = s
        .core
        .nodes
        .values()
        .filter_map(|n| {
            let l = n.label.as_deref()?;
            if l.trim().is_empty() {
                return None;
            }
            Some((l.to_string(), n.embedding.clone()))
        })
        .enumerate()
        .map(|(i, (l, e))| (i, l, e))
        .collect();

    let corpus = crate::rag::RagCorpus {
        chunks: candidates
            .iter()
            .map(|(i, l, e)| crate::rag::RagChunk {
                id: *i,
                text: l.clone(),
                tokens: crate::rag::count_tokens(l),
                embedding: e.clone(),
            })
            .collect(),
        bm25: crate::rag::Bm25Index::build(
            &candidates
                .iter()
                .map(|(_, l, _)| l.clone())
                .collect::<Vec<String>>(),
        ),
    };
    let result =
        crate::rag::TokenFixedRetriever::new(req.budget, req.top).retrieve(&q_emb, &corpus);
    Json(result).into_response()
}

#[derive(serde::Deserialize)]
struct ApiVaultImportReq {
    dir: String,
    #[serde(default = "default_git_max")]
    git: usize,
}

fn default_git_max() -> usize {
    50
}

async fn api_vault_import(
    State(state): State<Shared>,
    Json(req): Json<ApiVaultImportReq>,
) -> Response {
    let path = PathBuf::from(&req.dir);
    if !path.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            format!("dir {} not found", req.dir),
        )
            .into_response();
    }
    let mut docs = crate::vault::scan_vault(&path);
    if req.git > 0 {
        let commits = crate::vault::scan_git_log(&path, req.git);
        docs.extend(commits);
    }
    let pairs = crate::vault::collect_labels(&docs);
    let mut s = state.write().unwrap();
    let before = s.core.nodes.len();
    for (label, text) in &pairs {
        let emb = s.embedder.embed(text);
        s.core.register_node_vec_labeled(emb, Some(label.clone()));
    }
    let registered = s.core.nodes.len() - before;
    s.save_core();
    Json(serde_json::json!({
        "status": "ok",
        "docs": docs.len(),
        "registered": registered,
        "deduped": pairs.len().saturating_sub(registered),
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct ApiHistoryImportReq {
    file: String,
}

async fn api_history_import(
    State(state): State<Shared>,
    Json(req): Json<ApiHistoryImportReq>,
) -> Response {
    let path = PathBuf::from(&req.file);
    let (docs, name) = match crate::history::import_file(&path) {
        Ok(res) => res,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let mut s = state.write().unwrap();
    let before = s.core.nodes.len();
    for doc in &docs {
        let emb = s.embedder.embed(&doc.body);
        s.core
            .register_node_vec_labeled(emb, Some(doc.title.clone()));
    }
    let registered = s.core.nodes.len() - before;
    s.save_core();
    Json(serde_json::json!({
        "status": "ok",
        "name": name,
        "imported": docs.len(),
        "registered": registered,
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct ApiPraxisBackfillReq {
    json: Option<String>,
    file: Option<String>,
}

async fn api_praxis_backfill(
    State(state): State<Shared>,
    Json(req): Json<ApiPraxisBackfillReq>,
) -> Response {
    let text = if let Some(j) = req.json {
        j
    } else if let Some(f) = req.file {
        match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("failed reading {f}: {e}"))
                    .into_response()
            }
        }
    } else {
        return (StatusCode::BAD_REQUEST, "either json or file is required").into_response();
    };
    let records = crate::praxis::parse_export(&text);
    let mut s = state.write().unwrap();
    let before = s.core.nodes.len();
    for r in &records {
        let emb = s.embedder.embed(&r.body);
        let id = s
            .core
            .register_node_vec_labeled(emb, Some(format!("praxis:{}:{}", r.kind, r.title)));
        s.core.assert_coherence(&id, r.status.as_score());
    }
    let registered = s.core.nodes.len() - before;
    s.save_core();
    Json(serde_json::json!({
        "status": "ok",
        "records": records.len(),
        "registered": registered,
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct ApiCreateHypothesisReq {
    statement: String,
    #[serde(default)]
    assumptions: Option<Vec<String>>,
    #[serde(default)]
    confidence: Option<f32>,
}

async fn api_hypotheses_list(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let list: Vec<_> = s.core.hypotheses.values().cloned().collect();
    Json(list).into_response()
}

async fn api_hypothesis_create(
    State(state): State<Shared>,
    Json(req): Json<ApiCreateHypothesisReq>,
) -> Response {
    let mut s = state.write().unwrap();
    let emb = s.embedder.embed(&req.statement);
    let mut hyp = crate::hypothesis::Hypothesis::new(&req.statement, emb);
    if let Some(assumptions) = req.assumptions {
        hyp.assumptions = assumptions;
    }
    if let Some(conf) = req.confidence {
        hyp.confidence = conf;
    }
    let id = s.core.register_hypothesis(hyp.clone());
    s.save_core();
    let created = s.core.hypothesis(&id).cloned().unwrap_or(hyp);
    (StatusCode::CREATED, Json(created)).into_response()
}

async fn api_hypothesis_get(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let s = state.read().unwrap();
    match s.core.hypothesis(&id) {
        Some(h) => Json(h.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, format!("hypothesis {id} not found")).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ApiHypothesisEvidenceReq {
    claim: String,
    source: String,
    #[serde(default)]
    polarity: Option<String>,
    #[serde(default)]
    weight: Option<f32>,
}

async fn api_hypothesis_evidence(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<ApiHypothesisEvidenceReq>,
) -> Response {
    let mut s = state.write().unwrap();
    let pol = match req
        .polarity
        .as_deref()
        .unwrap_or("supporting")
        .to_lowercase()
        .as_str()
    {
        "contradicting" | "contra" | "refuting" => crate::hypothesis::EvidencePolarity::Contradicts,
        _ => crate::hypothesis::EvidencePolarity::Supports,
    };
    let mut ev = crate::hypothesis::Evidence::new(req.claim, req.source);
    if let Some(w) = req.weight {
        ev = ev.with_weight(w);
    }
    let res = if let Some(hyp) = s.core.hypotheses.get_mut(&id) {
        match pol {
            crate::hypothesis::EvidencePolarity::Supports => hyp.add_supporting(ev),
            crate::hypothesis::EvidencePolarity::Contradicts => hyp.add_contradicting(ev),
        }
        Some(hyp.clone())
    } else {
        None
    };
    match res {
        Some(h) => {
            s.save_core();
            Json(h).into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("hypothesis {id} not found")).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ApiHypothesisPredictionReq {
    statement: String,
    expected_outcome: Option<String>,
    actual_outcome: Option<String>,
    correct: Option<bool>,
    confidence: Option<f32>,
}

async fn api_hypothesis_prediction(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<ApiHypothesisPredictionReq>,
) -> Response {
    let mut s = state.write().unwrap();
    let mut pred = crate::hypothesis::Prediction::new(req.statement);
    if let Some(exp) = req.expected_outcome {
        pred.expected_outcome = Some(exp);
    }
    if let Some(act) = req.actual_outcome {
        pred.actual_outcome = Some(act);
    }
    if let Some(cor) = req.correct {
        pred.correct = Some(cor);
    }
    if let Some(conf) = req.confidence {
        pred.confidence = conf;
    }
    let res = if let Some(hyp) = s.core.hypotheses.get_mut(&id) {
        hyp.add_prediction(pred);
        Some(hyp.clone())
    } else {
        None
    };
    match res {
        Some(h) => {
            s.save_core();
            Json(h).into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("hypothesis {id} not found")).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ApiHypothesisTransitionReq {
    status: String,
    reason: String,
    trigger_id: Option<String>,
}

async fn api_hypothesis_transition(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<ApiHypothesisTransitionReq>,
) -> Response {
    let target_status = match req.status.to_lowercase().as_str() {
        "candidate" => crate::hypothesis::HypothesisStatus::Candidate,
        "supported" => crate::hypothesis::HypothesisStatus::Supported,
        "contradicted" => crate::hypothesis::HypothesisStatus::Contradicted,
        "confirmed" => crate::hypothesis::HypothesisStatus::Confirmed,
        "inert" => crate::hypothesis::HypothesisStatus::Inert,
        "failed" => crate::hypothesis::HypothesisStatus::Failed,
        "superseded" => crate::hypothesis::HypothesisStatus::Superseded,
        "isolated" => crate::hypothesis::HypothesisStatus::Isolated,
        "certified" => crate::hypothesis::HypothesisStatus::Certified,
        other => {
            return (StatusCode::BAD_REQUEST, format!("unknown status '{other}'")).into_response()
        }
    };
    let mut s = state.write().unwrap();
    let res = if s
        .core
        .transition_hypothesis(&id, target_status, &req.reason, req.trigger_id)
    {
        s.core.hypothesis(&id).cloned()
    } else {
        None
    };
    match res {
        Some(h) => {
            s.save_core();
            Json(h).into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("hypothesis {id} not found")).into_response(),
    }
}

async fn api_explanation_get(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let s = state.read().unwrap();
    match s.core.full_explanation_report(&id) {
        Some(report) => Json(report).into_response(),
        None => (StatusCode::NOT_FOUND, format!("hypothesis {id} not found")).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ApiCreateContradictionReq {
    claim_a: crate::contradiction::ContradictionParty,
    claim_b: crate::contradiction::ContradictionParty,
    resolution: Option<crate::contradiction::ResolutionStatus>,
    explanation: Option<String>,
}

async fn api_contradictions_list(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let list = s.core.contradictions.clone();
    Json(list).into_response()
}

async fn api_contradiction_create(
    State(state): State<Shared>,
    Json(req): Json<ApiCreateContradictionReq>,
) -> Response {
    let mut s = state.write().unwrap();
    let mut contra = crate::contradiction::Contradiction::new(req.claim_a, req.claim_b);
    if let Some(res) = req.resolution {
        contra.resolution = res;
    }
    if let Some(exp) = req.explanation {
        contra.explanation = Some(exp);
    }
    let id = s.core.record_contradiction(contra.clone());
    s.save_core();
    let created = s
        .core
        .contradictions
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .unwrap_or(contra);
    (StatusCode::CREATED, Json(created)).into_response()
}

#[derive(serde::Deserialize)]
struct ApiResolveContradictionReq {
    resolution: crate::contradiction::ResolutionStatus,
    explanation: Option<String>,
}

async fn api_contradiction_resolve(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<ApiResolveContradictionReq>,
) -> Response {
    let mut s = state.write().unwrap();
    let updated = if let Some(c) = s.core.contradictions.iter_mut().find(|c| c.id == id) {
        c.resolution = req.resolution;
        if let Some(exp) = req.explanation {
            c.explanation = Some(exp);
        }
        Some(c.clone())
    } else {
        None
    };
    match updated {
        Some(c) => {
            s.save_core();
            Json(c).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            format!("contradiction {id} not found"),
        )
            .into_response(),
    }
}

async fn api_epistemic_audit(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    Json(s.core.epistemic_audit.events.clone()).into_response()
}

#[derive(serde::Deserialize)]
struct ApiReplayReq {
    subject_id: String,
    at: String,
}

async fn api_epistemic_replay(
    State(state): State<Shared>,
    axum::extract::Query(params): axum::extract::Query<ApiReplayReq>,
) -> Response {
    let s = state.read().unwrap();
    let when = match chrono::DateTime::parse_from_rfc3339(&params.at) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("invalid timestamp '{}': {e}", params.at),
            )
                .into_response()
        }
    };
    let belief = s.core.reconstruct_belief_at(&params.subject_id, when);
    Json(serde_json::json!({
        "subject_id": params.subject_id,
        "at": params.at,
        "belief": belief,
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct ApiOntologyDiscoverReq {
    texts: Vec<String>,
    #[serde(default)]
    min_cluster_size: Option<usize>,
}

async fn api_ontology_discover(
    State(state): State<Shared>,
    Json(req): Json<ApiOntologyDiscoverReq>,
) -> Response {
    let s = state.read().unwrap();
    let mut config = crate::discovery::DiscoveryConfig::default();
    if let Some(m) = req.min_cluster_size {
        config.min_cluster = m;
    }
    let report =
        crate::discovery::discover(&req.texts, &s.classifier, s.embedder.as_ref(), &config);
    Json(report).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull the `value="…"` list out of one `<select id="…">` in the embedded
    /// app. The entry editor is built in JS, so the markup lives in app.js.
    fn option_values(select_id: &str) -> Vec<String> {
        let start = APP_JS
            .find(&format!("id=\"{select_id}\""))
            .unwrap_or_else(|| panic!("{select_id} not found in app.js"));
        let rest = &APP_JS[start..];
        let end = rest.find("</select>").expect("select is closed");
        rest[..end]
            .match_indices("value=\"")
            .map(|(i, m)| {
                let v = &rest[i + m.len()..];
                v[..v.find('"').expect("value is quoted")].to_string()
            })
            .filter(|v| !v.is_empty()) // the "—" placeholder
            .collect()
    }

    /// The facet dropdowns live in an HTML string literal, so nothing in Rust
    /// forces them to spell the enum variants correctly — and they didn't:
    /// the UI once offered Self / Organization / Concept / Implementation /
    /// Instance, none of which are variants. Every option must deserialize.
    #[test]
    fn studio_facet_options_match_the_enum_variants() {
        for v in option_values("mLifecycle") {
            serde_json::from_value::<LifecyclePhase>(serde_json::Value::String(v.clone()))
                .unwrap_or_else(|e| panic!("lifecycle option {v:?} is not a variant: {e}"));
        }
        for v in option_values("mAgency") {
            serde_json::from_value::<Agency>(serde_json::Value::String(v.clone()))
                .unwrap_or_else(|e| panic!("agency option {v:?} is not a variant: {e}"));
        }
        for v in option_values("mScale") {
            serde_json::from_value::<Scale>(serde_json::Value::String(v.clone()))
                .unwrap_or_else(|e| panic!("scale option {v:?} is not a variant: {e}"));
        }
        for v in option_values("mAbstraction") {
            serde_json::from_value::<Abstraction>(serde_json::Value::String(v.clone()))
                .unwrap_or_else(|e| panic!("abstraction option {v:?} is not a variant: {e}"));
        }
    }

    /// An upsert must carry the editor's facets onto the stored `DomainDef`.
    /// Before this, `EntryReq` declared the facet fields, the handler wrote
    /// `Facets::default()`, and editing an entry silently erased its facets.
    #[test]
    fn upsert_request_carries_facets_through() {
        let req: EntryReq = serde_json::from_value(serde_json::json!({
            "name": "Pump Maintenance",
            "category": "reliability",
            "domain": "CONSTRUCT",
            "mode": "MAINTAIN",
            "axis_kind": "machine",
            "axis_name": "operational",
            "unit": "tasks",
            "hints": "lubricate, inspect",
            "facets": { "lifecycle": "Operate", "agency": "Automated", "sub_domain": "Repair" },
        }))
        .expect("entry with facets deserializes");
        assert_eq!(req.facets.lifecycle, Some(LifecyclePhase::Operate));
        assert_eq!(req.facets.agency, Some(Agency::Automated));
        assert_eq!(req.facets.sub_domain.as_deref(), Some("Repair"));
        // Unset facets stay None rather than becoming a bogus default.
        assert_eq!(req.facets.scale, None);
    }

    /// `/api/ontology/upsert` is the scripting surface for the grid, not just
    /// the editor's private endpoint. Name/domain/mode are the entry and its two
    /// coordinates; everything else is decoration and must be omittable, or a
    /// curl one-liner fails with a deserialize error on a field it left blank.
    #[test]
    fn an_upsert_needs_only_the_name_and_its_two_coordinates() {
        let req: EntryReq = serde_json::from_value(serde_json::json!({
            "name": "soldering a joint",
            "domain": "FORGE",
            "mode": "TEMPER",
        }))
        .expect("minimal entry deserializes");
        assert_eq!(req.domain, "FORGE");
        assert_eq!(req.mode, "TEMPER");
        assert_eq!(req.unit, "");
        assert_eq!(req.hints, "");
        assert_eq!(req.axis_kind, "");
        assert_eq!(req.axis_name, "");

        // domain and mode stay REQUIRED on purpose: they are the cell key, and
        // defaulting them to "" would mint a nameless ""×"" cell that then shows
        // up as a real axis in the dynamic grid.
        assert!(
            serde_json::from_value::<EntryReq>(serde_json::json!({ "name": "orphan" })).is_err(),
            "an entry with no cell coordinates must be rejected, not defaulted"
        );
    }

    /// The shell only works if it actually pulls in the split assets; a rename
    /// on either side would otherwise ship a blank, silent page.
    #[test]
    fn app_html_loads_the_split_assets() {
        assert!(
            APP_HTML.contains("href=\"app.css\""),
            "app.html must link app.css"
        );
        assert!(
            APP_HTML.contains("src=\"app.js\""),
            "app.html must load app.js"
        );
        // Every nav target must have a matching section, or the tab is dead.
        // `edition` is the upgrade surface — the only place a Core user can
        // learn Pro exists, so a missing section would silently remove it.
        for tab in [
            "classify", "grid", "ontology", "corpus", "discover", "quality", "edition",
        ] {
            assert!(
                APP_HTML.contains(&format!("data-tab=\"{tab}\"")),
                "nav button for {tab}"
            );
            assert!(
                APP_HTML.contains(&format!("id=\"tab-{tab}\"")),
                "section for {tab}"
            );
        }
    }

    /// `app.js` keeps its own `TABS` array, which drives lazy loading and the
    /// 1..n number shortcuts. If it falls out of step with the markup, a tab
    /// still renders but never loads its data and its hotkey targets the wrong
    /// pane — neither of which throws, so nothing would report it.
    #[test]
    fn the_js_tab_list_matches_the_nav_buttons() {
        let nav: Vec<&str> = APP_HTML
            .match_indices("data-tab=\"")
            .map(|(i, pat)| {
                let rest = &APP_HTML[i + pat.len()..];
                &rest[..rest.find('"').expect("unterminated data-tab")]
            })
            .collect();
        assert!(!nav.is_empty(), "app.html must declare nav tabs");

        let line = APP_JS
            .lines()
            .find(|l| l.trim_start().starts_with("const TABS"))
            .expect("app.js must declare TABS");
        for tab in &nav {
            assert!(
                line.contains(&format!("'{tab}'")),
                "app.js TABS is missing {tab:?}: {line}",
            );
        }
        assert_eq!(
            line.matches('\'').count() / 2,
            nav.len(),
            "TABS and the nav must list the same tabs: {line}",
        );
    }

    /// The quality tracker keys cells on `DOMAIN\0MODE`, so the UI has to send
    /// that separator back verbatim. Writing a literal NUL into a source file
    /// is a trap (greps go binary, editors mangle it) — it must stay escaped.
    #[test]
    fn app_js_uses_the_nul_cell_separator_escaped() {
        assert!(
            !APP_JS.contains('\0'),
            "app.js must not carry a raw NUL byte"
        );
        assert!(
            APP_JS.contains(r"'\u0000'"),
            "app.js must build cell keys with an escaped NUL"
        );
    }

    /// Real docs blow past any whole-file limit, so a scan has to split them —
    /// and the split has to stay char-safe and drop noise-sized fragments.
    #[test]
    fn chunk_text_packs_paragraphs_and_splits_long_ones() {
        let short = chunk_text("tiny\n\nalso tiny");
        assert!(
            short.is_empty(),
            "sub-threshold text yields no nodes: {short:?}"
        );

        let para = "the pump seal was replaced during the planned shutdown window. ".repeat(4);
        let packed = chunk_text(&format!("{para}\n\n{para}"));
        assert_eq!(packed.len(), 1, "small paragraphs pack into one passage");

        // Multi-byte chars: a byte-offset split here would panic.
        let long = "é".repeat(CHUNK_CHARS * 2 + 10);
        let split = chunk_text(&long);
        assert_eq!(
            split.len(),
            2,
            "two full passages; the 10-char tail is noise"
        );
        assert!(split.iter().all(|c| c.chars().count() == CHUNK_CHARS));
    }

    /// The grid must be a view of the ontology, not of the enums: an edited
    /// entry or a promoted discovery proposal can introduce any axis string.
    /// A fixed 5×14 block hides those entries from the UI while `classify`
    /// still returns them.
    #[test]
    fn grid_axes_extend_past_the_builtin_variants() {
        let canonical: Vec<&str> = HumanDomain::all().iter().map(|d| d.as_str()).collect();
        let used: HashMap<String, u32> = [
            ("STUDY".to_string(), 3),
            ("EXCHANGE".to_string(), 1),
            ("DEFEND".to_string(), 2),
        ]
        .into_iter()
        .collect();

        let axis = build_axis(&canonical, &used);
        let names: Vec<&str> = axis.iter().map(|a| a.name.as_str()).collect();
        // Canonical order first, then the extras alphabetically.
        assert_eq!(
            names,
            [
                "HEAL",
                "CONSTRUCT",
                "FABRICATE",
                "BOND",
                "STUDY",
                "DEFEND",
                "EXCHANGE"
            ]
        );
        assert!(
            axis.iter().take(5).all(|a| !a.extra),
            "built-in axes are not flagged extra"
        );
        assert!(
            axis[5].extra && axis[6].extra,
            "ontology-introduced axes are flagged"
        );
        // Totals come from the ontology, so an unused canonical axis reads 0.
        assert_eq!(axis[0].total, 0);
        assert_eq!(axis[4].total, 3);
        assert_eq!(axis[5].total, 2);
    }

    /// The editor writes axes as free text, so the row an entry lands on must
    /// be whatever it says — including a mode nobody has used before.
    #[test]
    fn a_new_axis_gets_its_own_row() {
        let canonical: Vec<&str> = HumanMode::all().iter().map(|m| m.as_str()).collect();
        let used: HashMap<String, u32> = [("NEGOTIATE".to_string(), 1)].into_iter().collect();
        let axis = build_axis(&canonical, &used);
        assert_eq!(axis.len(), canonical.len() + 1);
        assert_eq!(axis.last().unwrap().name, "NEGOTIATE");
        assert!(axis.last().unwrap().extra);
    }

    #[test]
    fn verdicts_map_to_the_asserted_axis() {
        assert_eq!(verdict_score("success"), Some(1.0));
        assert_eq!(verdict_score("Inert"), Some(0.0));
        assert_eq!(verdict_score("FAILURE"), Some(-1.0));
        assert_eq!(verdict_score("maybe"), None);
    }

    /// Scan and gap-discovery walk the same tree, so they must agree on what
    /// counts as text — and neither may follow a directory into oblivion.
    #[test]
    fn collect_text_files_recurses_and_filters_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let skipped = dir.path().join("target");
        std::fs::create_dir(&skipped).unwrap();
        std::fs::write(dir.path().join("a.md"), "top level").unwrap();
        std::fs::write(nested.join("b.rs"), "fn main() {}").unwrap();
        std::fs::write(nested.join("c.bin"), [0u8, 1, 2]).unwrap();
        // Build output is machine-written noise; a scan must not ingest it.
        std::fs::write(skipped.join("d.rs"), "generated").unwrap();

        let mut names: Vec<String> = collect_text_files(dir.path())
            .iter()
            .filter_map(|p| p.file_name()?.to_str().map(str::to_string))
            .collect();
        names.sort();
        assert_eq!(names, vec!["a.md", "b.rs"]);
    }

    /// The studio and the CLI must share one graph file — a studio scan has to
    /// be visible to `physis-core search`, and vice versa.
    #[test]
    fn core_round_trips_through_the_cli_nodes_file() {
        let dir = tempfile::tempdir().unwrap();
        let embedder = RandomProjectionEmbedder::new(64);
        let mut core = PhysisCore::new();
        let id = core.register_node_from_text("pump seal replaced", &embedder);
        core.assert_coherence(&id, -1.0);
        save_core(dir.path(), &core).unwrap();

        assert_eq!(
            nodes_file(dir.path()).file_name(),
            crate::store::nodes_path().file_name()
        );
        let reloaded = load_core(dir.path());
        assert_eq!(reloaded.nodes.len(), 1);
        assert_eq!(reloaded.nodes[&id].asserted, Some(-1.0));
    }

    /// A missing or corrupt nodes file must not take the studio down with it.
    #[test]
    fn load_core_tolerates_a_missing_or_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_core(dir.path()).nodes.len(), 0);
        std::fs::write(nodes_file(dir.path()), "{not json").unwrap();
        assert_eq!(load_core(dir.path()).nodes.len(), 0);
    }

    /// A request with no `facets` key at all still parses — the studio's older
    /// clients (and the promote-proposal path) send none.
    #[test]
    fn upsert_request_without_facets_still_parses() {
        let req: EntryReq = serde_json::from_value(serde_json::json!({
            "name": "x", "domain": "STUDY", "mode": "WORK",
            "axis_kind": "", "axis_name": "", "unit": "", "hints": "",
        }))
        .expect("entry without facets deserializes");
        assert_eq!(req.facets, Facets::default());
    }
}
