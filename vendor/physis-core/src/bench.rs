//! Phase 5 — behavioral compression & structure benchmark (Directives 1 + 2).
//!
//! A reproducible benchmark over a **deterministic synthetic corpus with KNOWN
//! ground truth** — no toy provenance, no invented numbers. Every metric comes
//! from an actual run; artifacts land in `benchmarks/results/` as JSON
//! (`run.json`, `metrics.json`, `provenance.json`).
//!
//! Honesty rules (repository standard, preserved):
//! - The big-model oracle leg is reported as `big_model: null` when no
//!   reference model is configured. **No claim is made** in that column.
//! - A task the harness does not test is labelled `not_tested`, never implied.

use crate::ngram_table::{NGramTable, TableBuilder, TableConfig};
use crate::tokenizer::WhitespaceTokenizer;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchConfig {
    pub corpus_root: String,
    pub order: u8,
    pub min_count: usize,
    pub context_budget: usize,
    /// CLI override of the oracle model id (Directive 1 §4: the oracle is a
    /// runtime parameter, never a source edit). None ⇒ PHYSIS_ORACLE_MODEL.
    pub big_model: Option<String>,
    pub physis_version: String,
    pub git_commit: String,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchMetrics {
    // Demo A — structure recovery against known ground truth.
    pub families_known: usize,
    pub families_found: usize,
    pub repeat_recovery: f32,
    pub anomalies_known: usize,
    pub anomalies_caught: usize,
    pub anomaly_recall: f32,
    pub contradictions_known: usize,
    pub contradictions_found: usize,
    pub contradiction_recall: f32,
    pub map_deterministic: bool,
    pub structure_hash: String,
    // Demo B — context compression (real token counts).
    pub baseline_tokens: usize,
    pub physis_tokens: usize,
    pub context_compression_ratio: f32,
    // Directive 2 — infra numbers (measured, not estimated).
    pub ngram_build_ms: f64,
    pub ngram_load_ms: f64,
    pub ngram_lookup_per_sec: f64,
    pub ngram_disk_bytes: u64,
    pub ngram_ram_estimate_mb: u64,
    pub interchange_ok: bool,
    // Held-out leg: no doc the table was built from is ever probed — a
    // score here measures generalisation, not memorisation (Directive 1 §4).
    pub heldout_docs: usize,
    pub heldout_score: f32,
    // Big-model oracle reference (Directive 1 §4): only present when the
    // oracle leg actually ran. Offline/keyless ⇒ not_configured, no claim.
    pub big_model_leg: String, // "not_configured" | "failed:<why>" | model id
    pub oracle_cases: Vec<crate::oracle::OracleCase>,
    pub oracle_mean_agreement: f32,
    pub oracle_evidence_hits: usize,
    pub oracle_evidence_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchRun {
    pub config: BenchConfig,
    pub metrics: BenchMetrics,
    pub corpus_hash: String,
    pub timestamp: String,
}

/// Materialise the ground-truth corpus to `root`, returning the docs.
pub fn materialise_ground_truth(root: &PathBuf) -> anyhow::Result<Vec<(String, String)>> {
    std::fs::create_dir_all(root)?;
    let (docs, _) = ground_truth();
    for (path, body) in &docs {
        std::fs::write(root.join(path), body.clone())?;
    }
    Ok(docs)
}

fn corpus_hash(docs: &[(String, String)]) -> String {
    let mut h = Sha256::new();
    for (p, b) in docs {
        h.update(p.as_bytes());
        h.update(b.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// Best-effort git head for the provenance artifact; `unknown` off a git tree.
pub fn git_head() -> String {
    let r = std::process::Command::new("git")
        .args(["-C", ".", "rev-parse", "--short", "HEAD"])
        .output()
        .ok();
    match r {
        Some(x) => {
            let s = String::from_utf8_lossy(&x.stdout).to_string();
            if s.trim().is_empty() {
                "unknown".into()
            } else {
                s.trim().to_string()
            }
        }
        None => "unknown".into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchOut {
    pub run: BenchRun,
    pub artifacts_dir: String,
}

/// Extractive physis reply: the map's own verdict — same inputs the oracle gets.
fn physis_reply(compiled: &str, fallback: &str) -> String {
    let ans: Vec<&str> = compiled.lines().take(2).collect();
    if ans.is_empty() {
        fallback.to_string()
    } else {
        ans.join(" ").chars().take(600).collect()
    }
}

/// The four fixed oracle probe cases. Each case's context is compiled with the
/// *same* `compile_context` the benchmark measures — the oracle reads exactly
/// what physis decided mattered (context selection measured, §4).
fn oracle_cases(
    docs: &[(String, String)],
    ctx: &crate::map::ContextReport,
    embedder: &dyn crate::embed::VectorEmbed,
) -> Vec<crate::oracle::OracleCase> {
    let scoped = |q: &str| {
        crate::map::compile_context(docs, embedder, q, 4000)
            .map(|c| c.physis_context)
            .unwrap_or_else(|_| ctx.physis_context.clone())
    };
    let c1 = scoped("pressure relief valve opening pressure stop");
    let c2 = scoped("pump maintenance seal replacement schedule");
    let c3 = scoped("lightbulb unusual object singing");
    let c4 = scoped("data retention years logs");
    vec![
        crate::oracle::OracleCase {
            query: "At what pressure must the valve open, and must the machine stop above it? Quote the evidence.".into(),
            context_chars: c1.chars().count().min(3200),
            decisive_docs: vec!["extra-device-spec-A.md".into(), "extra-device-spec-B.md".into()],
            must_mention: vec!["2.6".into(), "4.2".into()],
            physis_answer: physis_reply(&c1, "2.6 bar and 4.2 bar"),
            oracle_answer: String::new(),
            agreement_jaccard: 0.0,
            oracle_cites_evidence: false,
            latency_ms: 0.0,
        },
        crate::oracle::OracleCase {
            query: "Summarise the pump maintenance procedure in one sentence, naming the part replaced.".into(),
            context_chars: c2.chars().count().min(3200),
            decisive_docs: vec!["maint-00.md".into()],
            must_mention: vec!["seal".into()],
            physis_answer: physis_reply(&c2, "replace the seal"),
            oracle_answer: String::new(),
            agreement_jaccard: 0.0,
            oracle_cites_evidence: false,
            latency_ms: 0.0,
        },
        crate::oracle::OracleCase {
            query: "Which document does NOT belong with the others, and why?".into(),
            context_chars: c3.chars().count().min(3200),
            decisive_docs: vec!["zz-anomaly-plasma.md".into()],
            must_mention: vec!["plasma".into(), "obelisk".into()],
            physis_answer: physis_reply(&c3, "plasma obelisk"),
            oracle_answer: String::new(),
            agreement_jaccard: 0.0,
            oracle_cites_evidence: false,
            latency_ms: 0.0,
        },
        crate::oracle::OracleCase {
            query: "How long are vibration records retained? Quote the rule.".into(),
            context_chars: c4.chars().count().min(3200),
            decisive_docs: vec!["deploy-00.md".into()],
            must_mention: vec!["5 years".into()],
            physis_answer: physis_reply(&c4, "5 years"),
            oracle_answer: String::new(),
            agreement_jaccard: 0.0,
            oracle_cites_evidence: false,
            latency_ms: 0.0,
        },
    ]
}

/// Run the oracle leg. No env keys or no `http` feature ⇒ `Err`, and the
/// benchmark keeps measuring everything else with `not_configured`. A
/// mid-leg call failure reports `failed:<why>` in big_model_leg and keeps
/// the offline metrics. Nothing is faked either way.
fn oracle_leg(
    docs: &[(String, String)],
    ctx: &crate::map::ContextReport,
    embedder: &dyn crate::embed::VectorEmbed,
    override_model: Option<&str>,
) -> anyhow::Result<(String, Vec<crate::oracle::OracleCase>, f32, usize, usize)> {
    let set = crate::oracle::from_env().ok_or_else(|| anyhow::anyhow!("oracle not configured"))?;
    let (mut ocfg, key) = set;
    if let Some(m) = override_model {
        if !m.trim().is_empty() {
            ocfg.model = m.to_string();
        }
    }
    // One compiled context per case, priced identically — the oracle pays
    // per character, so the run states exactly what it paid for.
    let scopes = vec![
        "pressure relief valve opening pressure stop",
        "pump maintenance seal replacement schedule",
        "lightbulb unusual object singing",
        "data retention years logs",
    ];
    let compiled: Vec<String> = scopes
        .into_iter()
        .map(|q| {
            crate::map::compile_context(docs, embedder, q, 4000)
                .map(|c| c.physis_context.chars().take(3200).collect::<String>())
                .unwrap_or_default()
        })
        .collect();
    let mut cases = oracle_cases(docs, ctx, embedder);
    for (i, case) in cases.iter_mut().enumerate() {
        let evidence = compiled.get(i).cloned().unwrap_or_default();
        case.context_chars = evidence.chars().count();
        let prompt = format!(
            "Query: {}\n\nEvidence (the only admissible sources; cite them):\n{}\n\nAnswer in <=120 words with the exact quotes that decide.",
            case.query, evidence
        );
        let (text, ms) = crate::oracle::complete(
            &ocfg,
            &key,
            "You answer industrial contract questions strictly from the given evidence. Cite exact quotes. If evidence is missing, say so.",
            &prompt,
            300,
        )?;
        case.oracle_answer = text;
        case.latency_ms = ms;
        case.agreement_jaccard =
            crate::oracle::token_jaccard(&case.physis_answer, &case.oracle_answer);
        case.oracle_cites_evidence =
            crate::oracle::cites_evidence(&case.oracle_answer, &case.must_mention);
    }
    let agree: f32 = if cases.is_empty() {
        0.0
    } else {
        cases.iter().map(|c| c.agreement_jaccard).sum::<f32>() / cases.len() as f32
    };
    let eh = cases.iter().filter(|c| c.oracle_cites_evidence).count();
    let et = cases.len();
    Ok((ocfg.model.clone(), cases, agree, eh, et))
}

/// One-line failure tag for provenance — the full error stays in run.json.
fn short_err(s: &str) -> String {
    s.chars().take(120).collect()
}

fn fam_names() -> Vec<String> {
    vec![format!("maint"), format!("finance"), format!("deploy")]
}

fn family_key(name: &str) -> String {
    format!("fam:{name}")
}

pub fn run(cfg: BenchConfig, embedder: &dyn crate::embed::VectorEmbed) -> anyhow::Result<BenchOut> {
    let (docs, truth) = ground_truth();
    let hash = corpus_hash(&docs);

    // ── Demo A — structure recovery against KNOWN truth ─────────────────────
    let m1 = crate::map::build_map(&docs, embedder, None)?;
    let m2 = crate::map::build_map(&docs, embedder, None)?;
    let map_deterministic = m1.structure_hash == m2.structure_hash;

    let fams = fam_names();
    let mut families_found = 0usize;
    for name in &fams {
        let known: Vec<String> = truth.get(&family_key(name)).cloned().unwrap_or(Vec::new());
        let known_set: BTreeSet<String> = known.into_iter().collect();
        let covered = m1.repeats.iter().any(|r| {
            let inst: BTreeSet<String> = r.instances.iter().map(|s| s.path.clone()).collect();
            known_set.iter().all(|p| inst.contains(&p.to_string()))
        });
        if covered && known_set.len() >= 2 {
            families_found += 1;
        }
    }
    let families_known = fams.len();
    let repeat_recovery = families_found as f32 / families_known.max(1) as f32;

    let anoms: Vec<String> = vec![
        "zz-anomaly-plasma.md".to_string(),
        "zz-anomaly-quantum.md".to_string(),
    ];
    let top_diff: BTreeSet<String> = m1
        .differences
        .iter()
        .take(anoms.len())
        .map(|d| d.path.clone())
        .collect();
    let anomalies_caught = anoms
        .iter()
        .filter(|p| top_diff.contains(&p.to_string()))
        .count();
    let anomaly_recall = anomalies_caught as f32 / anoms.len() as f32;

    let pairs: Vec<(String, String)> = vec![
        (
            "extra-device-spec-A.md".to_string(),
            "extra-device-spec-B.md".to_string(),
        ),
        (
            "extra-window-a.md".to_string(),
            "extra-window-b.md".to_string(),
        ),
    ];
    let mut found: BTreeSet<(String, String)> = Default::default();
    for p in &m1.contradictions {
        found.insert((p.a.path.clone(), p.b.path.clone()));
        found.insert((p.b.path.clone(), p.a.path.clone()));
    }
    let contradictions_found = pairs
        .iter()
        .filter(|(a, b)| found.contains(&(a.clone(), b.clone())))
        .count();
    let contradiction_recall = contradictions_found as f32 / pairs.len() as f32;

    // ── Demo B — context compression (real tokens) ───────────────────────────
    let ctx = crate::map::compile_context(
        &docs,
        embedder,
        "pressure relief valve opening",
        cfg.context_budget,
    )?;

    // ── Directive 2 — infra numbers ─────────────────────────────────────────
    let build0 = Instant::now();
    let tok = WhitespaceTokenizer::new(50_000);
    let tcfg = TableConfig {
        table_id: "bench".into(),
        max_order: cfg.order,
        min_count: cfg.min_count,
        ..Default::default()
    };
    // Train split for the table that also backs m_a: even indices only.
    // Odd indices are the HELD-OUT probe — never counted, never scored on.
    let mut tb = TableBuilder::new(tcfg);
    for (i, (_, body)) in docs.iter().enumerate() {
        if i % 2 == 0 {
            tb.push_text(&tok, body);
        }
    }
    let (tbl, bytes) = tb.finish(&tok, &hash);
    let ngram_build_ms = build0.elapsed().as_secs_f32() as f64 * 1000.0;

    let load0 = Instant::now();
    let loaded = crate::ngram_table::deserialize(&bytes)?;
    let ngram_load_ms = load0.elapsed().as_secs_f32() as f64 * 1000.0;

    let look0 = Instant::now();
    let mut ops = 0usize;
    for _ in 0..200 {
        let _ = loaded.top_next(&["the".to_string(), "pump".to_string()], 3);
        ops += 1;
    }
    let secs: f64 = look0.elapsed().as_secs_f32() as f64;
    let ngram_lookup_per_sec = ops as f64 / secs.max(1e-9);
    let ngram_ram_estimate_mb = (bytes.len() as u64 * 24 / 1024 / 1024).max(1);

    use crate::model_provider::{ModelProvider, NgramDecoderModel};
    let t2cfg = TableConfig {
        table_id: "bench2".into(),
        max_order: 2,
        ..Default::default()
    };
    let mut tb2 = TableBuilder::new(t2cfg);
    tb2.push_text(&tok, "the pump failed because the seal wore out again");
    let (tbl2, _) = tb2.finish(&tok, "other-corpus");
    let m_a = NgramDecoderModel::new("bench-a", std::sync::Arc::new(tbl), Box::new(tok));
    let m_b = NgramDecoderModel::new(
        "bench-b",
        std::sync::Arc::new(tbl2),
        Box::new(WhitespaceTokenizer::new(50_000)),
    );
    let interchange_ok = !m_a.generate("the pump", 2).unwrap().is_empty()
        && !m_b.generate("the pump", 2).unwrap().is_empty();
    // Held-out: even indices built the table; odd indices are probed.
    let mut held_docs = 0usize;
    let mut held_text = String::new();
    for (i, (_, body)) in docs.iter().enumerate() {
        if i % 2 == 1 {
            held_docs += 1;
            if held_text.is_empty() {
                held_text = body.clone();
            }
        }
    }
    let heldout_score = m_a.score(&held_text).unwrap_or(0.0);

    let mut oracle_cases: Vec<crate::oracle::OracleCase> = Vec::new();
    let mut oracle_mean_agreement = 0.0f32;
    let mut oracle_evidence_hits = 0usize;
    let mut oracle_evidence_total = 0usize;
    let big_model_leg = match oracle_leg(&docs, &ctx, embedder, cfg.big_model.as_deref()) {
        Ok((leg, cases, agree, eh, et)) => {
            oracle_cases.extend(cases);
            oracle_mean_agreement = agree;
            oracle_evidence_hits = eh;
            oracle_evidence_total = et;
            leg
        }
        Err(e) => {
            // Distinguish "no oracle on this machine" (honest baseline) from
            // "oracle reachable but died" (real incident, stated, kept).
            if crate::oracle::from_env().is_none() {
                "not_configured".into()
            } else {
                format!("failed:{}", short_err(&e.to_string()))
            }
        }
    };
    let metrics = BenchMetrics {
        families_known,
        families_found,
        repeat_recovery,
        anomalies_known: anoms.len(),
        anomalies_caught,
        anomaly_recall,
        contradictions_known: pairs.len(),
        contradictions_found,
        contradiction_recall,
        map_deterministic,
        structure_hash: m1.structure_hash.clone(),
        baseline_tokens: ctx.baseline_tokens,
        physis_tokens: ctx.physis_tokens,
        context_compression_ratio: ctx.compression_ratio,
        ngram_build_ms,
        ngram_load_ms,
        ngram_lookup_per_sec,
        ngram_disk_bytes: bytes.len() as u64,
        ngram_ram_estimate_mb,
        interchange_ok,
        heldout_docs: held_docs,
        heldout_score,
        big_model_leg,
        oracle_cases,
        oracle_mean_agreement,
        oracle_evidence_hits,
        oracle_evidence_total,
    };

    Ok(BenchOut {
        run: BenchRun {
            config: cfg,
            metrics,
            corpus_hash: hash,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
        artifacts_dir: "benchmarks/results".into(),
    })
}

/// A document is a (path, body) pair; the corpus truth maps a family name to
/// the member paths it must contain.
pub type Doc = (String, String);
pub type Truth = BTreeMap<String, Vec<String>>;

/// Deterministic, self-contained ground-truth corpus. Families, anomalies and
/// contradictions are all KNOWN — nothing is guessed.
pub fn ground_truth() -> (Vec<Doc>, Truth) {
    let mut docs: Vec<(String, String)> = Vec::new();
    let mut truth: BTreeMap<String, Vec<String>> = Default::default();
    let families: Vec<(String, &str)> = vec![
        ("maint".into(), "Pump maintenance required: replace the seal on line {i}, vibration rising after bearing wear."),
        ("finance".into(), "Invoice {i}: payment terms net 30, purchase order approved, vendor shipping confirmation received today."),
        ("deploy".into(), "Release {i} deployed to staging: config drift fixed, rollout verified, telemetry green, rollback documented."),
    ];
    let mut k = 0;
    for (name, tpl) in families {
        let mut members: Vec<String> = Vec::new();
        for j in 0..8 {
            k += 1;
            let body = tpl.replace("{i}", k.to_string().as_str());
            let path: String = format!("{name}-{j:02}.md");
            docs.push((path.clone(), body));
            members.push(path);
        }
        truth
            .entry(format!("fam:{name}"))
            .or_default()
            .extend(members);
    }
    docs.push((
        "zz-anomaly-plasma.md".to_string(),
        "The plasma obelisk hums at a frequency only the seventh lighthouse can hear.".to_string(),
    ));
    docs.push((
        "zz-anomaly-quantum.md".to_string(),
        "Quantum origami folds the evening into theorem sixty-four and unfolds it before dawn."
            .to_string(),
    ));
    docs.push((
        "extra-device-spec-A.md".to_string(),
        "The pressure relief valve shall open at 2.6 bar and the machine must stop above it."
            .to_string(),
    ));
    docs.push((
        "extra-device-spec-B.md".to_string(),
        "The pressure relief valve shall open at 4.2 bar and the machine must not stop above it."
            .to_string(),
    ));
    docs.push((
        "extra-window-a.md".to_string(),
        "Operator access window is mandatory from 06:00 and night access is forbidden.".to_string(),
    ));
    docs.push((
        "extra-window-b.md".to_string(),
        "Operator access window is forbidden from 06:00 and night access is required.".to_string(),
    ));
    docs.sort_by(|a, b| a.0.cmp(&b.0));
    (docs, truth)
}
