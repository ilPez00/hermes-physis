//! Receipt-carrying context compilation (deterministic, budgeted).
//!
//! A library primitive for "retrieve with receipts": given ranked candidates,
//! select under a hard heuristic budget and explain *why* each object made the
//! cut (score, rank, strategy, reasons) and why others were dropped. The packet
//! is an inspectable artifact for agents/lenses — it is **not** a cost/API-bill
//! claim and **not** a proof of completeness.
//!
//! Observer-derived candidates are first-class but explicitly marked unverified;
//! Physis (the compiler/caller) owns selection, budget, and omission reasons.

use serde::{Deserialize, Serialize};

/// Identity of the embedder/encoder used to produce scores (if any).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedderInfo {
    /// Name/id of the embedder.
    #[serde(default)]
    pub name: String,
    /// Whether the embedder provides semantic (model) similarity vs lexical/hash.
    #[serde(default)]
    pub semantic: bool,
}

/// A candidate considered for inclusion in the compiled context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompileCandidate {
    /// Stable object/claim id.
    pub id: String,
    /// Kind/kind-label (symbol/file/chunk/hypothesis/claim/...).
    #[serde(default)]
    pub kind: String,
    /// Text/content representation considered for budgeting.
    #[serde(default)]
    pub text: String,
    /// Heuristic token/content units for this candidate.
    #[serde(default)]
    pub tokens: usize,
    /// Fused/primary score (higher = better).
    #[serde(default)]
    pub score: f32,
    /// Strategy/strategies that produced this candidate.
    #[serde(default)]
    pub strategies: Vec<String>,
    /// Human/engine-readable reasons/features.
    #[serde(default)]
    pub reasons: Vec<String>,
    /// True if this candidate originated from an external observer (unverified).
    #[serde(default)]
    pub observer_sourced: bool,
    /// Provenance/source label (optional).
    #[serde(default)]
    pub source: Option<String>,
}

/// Why a candidate was dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    OverBudget,
    Duplicate,
    Empty,
    Filtered,
}

/// Receipt for one selected item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionReceipt {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    /// Rank among selected items (0 = highest).
    pub rank: usize,
    pub score: f32,
    pub tokens: usize,
    #[serde(default)]
    pub strategies: Vec<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub observer_sourced: bool,
    #[serde(default)]
    pub source: Option<String>,
}

/// Receipt for one dropped candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropReceipt {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    pub score: f32,
    pub tokens: usize,
    pub reason: DropReason,
    #[serde(default)]
    pub detail: Option<String>,
}

/// Budget/selection accounting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompileStats {
    pub budget: usize,
    pub selected_tokens: usize,
    pub candidate_count: usize,
    pub selected_count: usize,
    pub dropped_count: usize,
    pub dropped_tokens: usize,
    pub utilization: f32,
    #[serde(default)]
    pub strategies: Vec<String>,
    #[serde(default)]
    pub duplicates: usize,
    #[serde(default)]
    pub truncated: bool,
    /// Cache status hint (miss/hit/unknown) — informational.
    #[serde(default)]
    pub cache: String,
}

/// Deterministic, receipt-carrying compiled context packet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContext {
    pub query: String,
    pub stats: CompileStats,
    #[serde(default)]
    pub embedder: EmbedderInfo,
    pub selected: Vec<SelectionReceipt>,
    pub dropped: Vec<DropReceipt>,
}

impl CompiledContext {
    /// Selected ids in rank order.
    pub fn selected_ids(&self) -> Vec<String> {
        self.selected.iter().map(|s| s.id.clone()).collect()
    }
}

/// Options controlling compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    /// Hard heuristic budget in token/content units.
    pub budget: usize,
    /// Mark truncated when any non-duplicate candidate could not fit.
    #[serde(default = "default_true")]
    pub mark_truncated: bool,
    /// Cache status label to record.
    #[serde(default)]
    pub cache: String,
}
fn default_true() -> bool {
    true
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            budget: 800,
            mark_truncated: true,
            cache: "unknown".into(),
        }
    }
}

/// Compile candidates into a receipt-carrying packet under a hard budget.
///
/// Deterministic: candidates are sorted by score desc, then tokens asc, then id
/// asc; selection is a greedy first-fit in that order with duplicate-id removal.
/// No external state, no model calls.
pub fn compile_context(
    query: impl Into<String>,
    mut candidates: Vec<CompileCandidate>,
    opts: &CompileOptions,
    embedder: EmbedderInfo,
) -> CompiledContext {
    let query = query.into();
    // Stable ranking.
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.tokens.cmp(&b.tokens))
            .then(a.id.cmp(&b.id))
    });

    let mut selected: Vec<SelectionReceipt> = Vec::new();
    let mut dropped: Vec<DropReceipt> = Vec::new();
    let mut used = 0usize;
    let mut dropped_tokens = 0usize;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut duplicates = 0usize;
    let mut all_strats: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for c in &candidates {
        for s in &c.strategies {
            all_strats.insert(s.clone());
        }
        if c.id.trim().is_empty() || c.text.is_empty() && c.tokens == 0 {
            dropped_tokens += c.tokens;
            dropped.push(DropReceipt {
                id: c.id.clone(),
                kind: c.kind.clone(),
                score: c.score,
                tokens: c.tokens,
                reason: DropReason::Empty,
                detail: Some("empty id/content".into()),
            });
            continue;
        }
        if !seen.insert(c.id.clone()) {
            duplicates += 1;
            dropped_tokens += c.tokens;
            dropped.push(DropReceipt {
                id: c.id.clone(),
                kind: c.kind.clone(),
                score: c.score,
                tokens: c.tokens,
                reason: DropReason::Duplicate,
                detail: None,
            });
            continue;
        }
        let t = c.tokens;
        if used.saturating_add(t) <= opts.budget {
            let rank = selected.len();
            let mut reasons = c.reasons.clone();
            if c.observer_sourced {
                reasons.push("observer_sourced_unverified".into());
            }
            selected.push(SelectionReceipt {
                id: c.id.clone(),
                kind: c.kind.clone(),
                rank,
                score: c.score,
                tokens: t,
                strategies: c.strategies.clone(),
                reasons,
                observer_sourced: c.observer_sourced,
                source: c.source.clone(),
            });
            used += t;
        } else {
            dropped_tokens += t;
            dropped.push(DropReceipt {
                id: c.id.clone(),
                kind: c.kind.clone(),
                score: c.score,
                tokens: t,
                reason: DropReason::OverBudget,
                detail: Some("exceeds remaining budget".into()),
            });
        }
    }

    let truncated =
        opts.mark_truncated && dropped.iter().any(|d| d.reason == DropReason::OverBudget);
    let utilization = if opts.budget == 0 {
        0.0
    } else {
        (used as f32 / opts.budget as f32).min(1.0)
    };

    CompiledContext {
        query,
        stats: CompileStats {
            budget: opts.budget,
            selected_tokens: used,
            candidate_count: candidates.len(),
            selected_count: selected.len(),
            dropped_count: dropped.len(),
            dropped_tokens,
            utilization,
            strategies: all_strats.into_iter().collect(),
            duplicates,
            truncated,
            cache: opts.cache.clone(),
        },
        embedder,
        selected,
        dropped,
    }
}

/// Convert observer-emitted retrieval-hit claims into compile candidates.
///
/// Each `ClaimPayload::Retrieval` becomes an observer-sourced, unverified
/// candidate attributed to the hit's strategy and the claim's extractor.
/// Non-retrieval claims are ignored. Token counts use `tokens_for` so callers
/// can apply their own heuristic counter; return 0 to mark a hit empty (it
/// will be dropped with reason).
pub fn candidates_from_retrieval_claims<F>(
    claims: &[crate::ingest::CandidateClaimPacket],
    mut tokens_for: F,
) -> Vec<CompileCandidate>
where
    F: FnMut(&crate::ingest::CandidateClaimPacket, &crate::ingest::RetrievalHit) -> usize,
{
    let mut out = Vec::new();
    for c in claims {
        if let crate::ingest::ClaimPayload::Retrieval(hit) = &c.payload {
            let tokens = tokens_for(c, hit);
            let text = hit.snippet.clone().unwrap_or_default();
            let mut strategies = Vec::new();
            if let Some(s) = &hit.strategy {
                strategies.push(s.clone());
            }
            strategies.push(format!("observer:{}", c.extractor.id));
            out.push(CompileCandidate {
                id: if hit.object_id.is_empty() {
                    c.id.clone()
                } else {
                    hit.object_id.clone()
                },
                kind: "retrieval_hit".into(),
                text,
                tokens,
                score: hit.score,
                strategies,
                reasons: vec![format!("observer_claim={}", c.id)],
                observer_sourced: true,
                source: if c.source.uri.is_empty() {
                    c.raw_ref.clone()
                } else {
                    Some(c.source.uri.clone())
                },
            });
        }
    }
    out
}

/// Fuse engine-local and observer-sourced candidates into one packet.
///
/// This is a pure convenience over [`compile_context`]: observer candidates are
/// included as attributed, ablatable inputs (marked `observer_sourced`) and
/// Physis still enforces budget and records drop/selection reasons. Callers
/// should gate/measure before relying on observer strategies (off by default
/// unless they show measured win over core baselines).
pub fn compile_fused(
    query: impl Into<String>,
    engine_candidates: Vec<CompileCandidate>,
    observer_candidates: Vec<CompileCandidate>,
    opts: &CompileOptions,
    embedder: EmbedderInfo,
) -> CompiledContext {
    let mut all = engine_candidates;
    // Ensure observer attribution is preserved even if caller forgot.
    for mut c in observer_candidates {
        c.observer_sourced = true;
        if !c.reasons.iter().any(|r| r == "observer_sourced_unverified") {
            c.reasons.push("observer_sourced_unverified".into());
        }
        all.push(c);
    }
    compile_context(query, all, opts, embedder)
}

/// Compile engine retrieval results fused with observer retrieval-hit claims.
///
/// Engine chunks are local; observer claims become attributed, unverified
/// candidates (`observer_sourced`). Budget/truncation follow the retrieval.
/// Callers should enable observer fusion only when measured to win over
/// core-only baselines (ablatable).
pub fn compile_from_retrieval_with_observer_claims<F>(
    query: impl Into<String>,
    retrieval: &crate::rag::RetrievalResult,
    engine_strategies: &[&str],
    observer_claims: &[crate::ingest::CandidateClaimPacket],
    embedder: EmbedderInfo,
    tokens_for: F,
) -> CompiledContext
where
    F: FnMut(&crate::ingest::CandidateClaimPacket, &crate::ingest::RetrievalHit) -> usize,
{
    let engine_candidates: Vec<CompileCandidate> = retrieval
        .chunks
        .iter()
        .map(|c| CompileCandidate {
            id: format!("chunk:{}", c.id),
            kind: "chunk".into(),
            text: c.text.clone(),
            tokens: c.tokens,
            score: c.score,
            strategies: engine_strategies.iter().map(|s| s.to_string()).collect(),
            reasons: vec![format!("score={:.4}", c.score)],
            observer_sourced: false,
            source: None,
        })
        .collect();
    let observer_candidates = candidates_from_retrieval_claims(observer_claims, tokens_for);
    let opts = CompileOptions {
        budget: retrieval.budget,
        mark_truncated: retrieval.truncated,
        cache: "unknown".into(),
    };
    compile_fused(
        query,
        engine_candidates,
        observer_candidates,
        &opts,
        embedder,
    )
}

/// Convenience: compile directly from RAG retrieval results, attributing the
/// given strategies. RAG chunks are engine-local (not observer-sourced).
pub fn compile_from_retrieval(
    query: impl Into<String>,
    retrieval: &crate::rag::RetrievalResult,
    strategies: &[&str],
    embedder: EmbedderInfo,
) -> CompiledContext {
    let candidates = retrieval
        .chunks
        .iter()
        .map(|c| CompileCandidate {
            id: format!("chunk:{}", c.id),
            kind: "chunk".into(),
            text: c.text.clone(),
            tokens: c.tokens,
            score: c.score,
            strategies: strategies.iter().map(|s| s.to_string()).collect(),
            reasons: vec![format!("score={:.4}", c.score)],
            observer_sourced: false,
            source: None,
        })
        .collect();
    let opts = CompileOptions {
        budget: retrieval.budget,
        mark_truncated: retrieval.truncated,
        cache: "unknown".into(),
    };
    compile_context(query, candidates, &opts, embedder)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: &str, score: f32, tokens: usize, obs: bool) -> CompileCandidate {
        CompileCandidate {
            id: id.into(),
            kind: "k".into(),
            text: id.into(),
            tokens,
            score,
            strategies: vec!["s".into()],
            reasons: vec![],
            observer_sourced: obs,
            source: None,
        }
    }

    #[test]
    fn budget_and_receipts_are_deterministic() {
        let cands = vec![
            cand("b", 0.9, 10, false),
            cand("a", 0.9, 5, true),
            cand("c", 0.5, 20, false),
        ];
        let out = compile_context(
            "q",
            cands.clone(),
            &CompileOptions {
                budget: 15,
                ..Default::default()
            },
            EmbedderInfo {
                name: "x".into(),
                semantic: false,
            },
        );
        // a (score tie, fewer tokens) then b; c dropped over budget
        assert_eq!(out.selected_ids(), vec!["a", "b"]);
        assert_eq!(out.stats.selected_tokens, 15);
        assert!(out.selected[0].observer_sourced);
        assert!(out.selected[0]
            .reasons
            .iter()
            .any(|r| r.contains("observer_sourced")));
        assert!(out.stats.truncated);
        let out2 = compile_context(
            "q",
            cands,
            &CompileOptions {
                budget: 15,
                ..Default::default()
            },
            EmbedderInfo {
                name: "x".into(),
                semantic: false,
            },
        );
        assert_eq!(out, out2);
    }

    #[test]
    fn duplicates_and_empty_are_dropped_with_reasons() {
        let cands = vec![
            cand("a", 1.0, 1, false),
            cand("a", 0.5, 1, false),
            CompileCandidate {
                id: "".into(),
                kind: "".into(),
                text: "".into(),
                tokens: 0,
                score: 0.0,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: false,
                source: None,
            },
        ];
        let out = compile_context(
            "q",
            cands,
            &CompileOptions {
                budget: 10,
                ..Default::default()
            },
            EmbedderInfo::default(),
        );
        assert_eq!(out.selected.len(), 1);
        assert_eq!(out.stats.duplicates, 1);
        assert!(out
            .dropped
            .iter()
            .any(|d| d.reason == DropReason::Duplicate));
        assert!(out.dropped.iter().any(|d| d.reason == DropReason::Empty));
    }

    #[test]
    fn observer_retrieval_claims_become_unverified_candidates() {
        use crate::ingest::{
            CandidateClaimPacket, ClaimPayload, ClaimSource, ExtractorInfo, RetrievalHit,
        };
        let claim = CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i".into()),
            source: ClaimSource {
                uri: "doc://x".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("cognee:mock"),
            asserted_at: None,
            observed_at: None,
            raw_ref: None,
            confidence: Some(0.4),
            payload: ClaimPayload::Retrieval(RetrievalHit {
                object_id: "obj:1".into(),
                score: 0.7,
                strategy: Some("vec".into()),
                snippet: Some("hello".into()),
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        }
        .normalize();
        let cands = candidates_from_retrieval_claims(&[claim], |_, _| 2);
        assert_eq!(cands.len(), 1);
        assert!(cands[0].observer_sourced);
        assert_eq!(cands[0].id, "obj:1");
        let out = compile_fused(
            "q",
            vec![cand("e", 0.9, 3, false)],
            cands,
            &CompileOptions {
                budget: 10,
                ..Default::default()
            },
            EmbedderInfo::default(),
        );
        assert!(out.selected.iter().any(|s| s.observer_sourced));
        assert!(out.selected.iter().any(|s| !s.observer_sourced));
    }
}
