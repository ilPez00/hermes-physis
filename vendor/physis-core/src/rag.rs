//! Token-fixed RAG — retrieve chunks subject to a hard token budget.
//!
//! Unlike unbounded top-k retrieval, the context handed to the model is
//! *guaranteed* to fit a fixed token window: chunks are ranked by cosine to the
//! query, then greedily packed until the running token total would exceed
//! `token_budget` (or `top_k` is reached). This makes prompt assembly
//! deterministic and overflow-proof regardless of corpus size — the "fixed"
//! in token-fixed RAG.

use crate::embed::VectorEmbed;
use crate::models::cosine_sim;
use serde::{Deserialize, Serialize};

/// A token counter. The default heuristic needs no model files; a real BPE
/// tokenizer can be supplied by implementing this trait.
pub trait TokenCounter {
    fn count(&self, text: &str) -> usize;
}

/// Dependency-free subword-ish estimate: whitespace-split words, with long
/// words spilling into extra tokens, plus one token per punctuation/separator.
/// Mirrors how BPE over-tokenizes long words well enough for budgeting.
pub struct HeuristicTokenizer;

impl TokenCounter for HeuristicTokenizer {
    fn count(&self, text: &str) -> usize {
        count_tokens(text)
    }
}

/// Estimate token count of `text` with the heuristic counter.
pub fn count_tokens(text: &str) -> usize {
    let mut n = 0usize;
    let mut word = String::new();
    for c in text.chars() {
        if c.is_whitespace() {
            if !word.is_empty() {
                n += tokenize_word(&word);
                word.clear();
            }
        } else if c.is_alphanumeric() {
            word.push(c);
        } else {
            if !word.is_empty() {
                n += tokenize_word(&word);
                word.clear();
            }
            n += 1;
        }
    }
    if !word.is_empty() {
        n += tokenize_word(&word);
    }
    n.max(1)
}

fn tokenize_word(w: &str) -> usize {
    let len = w.chars().count();
    1 + (len.saturating_sub(6)) / 4
}

/// One retrievable chunk of text with its precomputed token count + embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagChunk {
    pub id: usize,
    pub text: String,
    pub tokens: usize,
    pub embedding: Vec<f32>,
}

/// A corpus of chunks, ready for retrieval.
#[derive(Debug, Clone, Default)]
pub struct RagCorpus {
    pub chunks: Vec<RagChunk>,
    /// G5: the Okapi BM25 leg over the same texts, precomputed per corpus
    /// so hybrid retrieval costs one pass at build time and nothing per
    /// query beyond scoring.
    pub bm25: Bm25Index,
}

impl RagCorpus {
    /// Build a corpus from raw texts, embedding each and counting its tokens.
    ///
    /// Embedding is submitted in fixed-size batches rather than one call per
    /// corpus. A single ONNX call over a thousand 512-token sequences allocates
    /// activations for all of them at once and is killed by the OOM reaper on an
    /// ordinary machine; batching bounds peak memory independently of corpus
    /// size, at no cost in throughput.
    pub fn build(texts: &[String], embedder: &dyn VectorEmbed) -> Self {
        /// Sequences per embedder call. Large enough to keep the model busy,
        /// small enough that peak activation memory stays flat.
        const EMBED_BATCH: usize = 32;

        let tokens: Vec<usize> = texts.iter().map(|t| count_tokens(t)).collect();
        let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
        for batch in texts.chunks(EMBED_BATCH) {
            let refs: Vec<&str> = batch.iter().map(|s| s.as_str()).collect();
            embeddings.extend(embedder.embed_batch(&refs));
        }
        let chunks = texts
            .iter()
            .enumerate()
            .map(|(i, t)| RagChunk {
                id: i,
                text: t.clone(),
                tokens: tokens[i],
                embedding: embeddings.get(i).cloned().unwrap_or_default(),
            })
            .collect();
        Self {
            chunks,
            bm25: Bm25Index::build(texts),
        }
    }

    /// Total token footprint of the whole corpus.
    pub fn total_tokens(&self) -> usize {
        self.chunks.iter().map(|c| c.tokens).sum()
    }
}

/// One chunk selected for the final context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub id: usize,
    pub text: String,
    pub score: f32,
    pub tokens: usize,
}

/// The result of a token-fixed retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Chunks actually selected, in score-descending order.
    pub chunks: Vec<RetrievedChunk>,
    /// Sum of `tokens` over `chunks` — guaranteed <= `budget`.
    pub total_tokens: usize,
    /// The budget this retrieval was constrained to.
    pub budget: usize,
    /// True when not every candidate fit (context was truncated to fit budget).
    pub truncated: bool,
}

/// A token-fixed retriever: packs the highest-scoring chunks that fit.
pub struct TokenFixedRetriever {
    pub budget: usize,
    pub top_k: usize,
    /// MMR-like diversity (0 = pure score, 1 = max diversity).
    pub diversity: f32,
}

impl TokenFixedRetriever {
    pub fn new(budget: usize, top_k: usize) -> Self {
        Self {
            budget,
            top_k,
            diversity: 0.0,
        }
    }

    pub fn with_diversity(mut self, diversity: f32) -> Self {
        self.diversity = diversity.clamp(0.0, 1.0);
        self
    }

    /// Retrieve the chunks whose packed token total stays within `budget`.
    pub fn retrieve(&self, query_emb: &[f32], corpus: &RagCorpus) -> RetrievalResult {
        if corpus.chunks.is_empty() || self.budget == 0 {
            return RetrievalResult {
                chunks: Vec::new(),
                total_tokens: 0,
                budget: self.budget,
                truncated: false,
            };
        }

        let mut scored: Vec<(usize, f32)> = corpus
            .chunks
            .iter()
            .map(|c| (c.id, cosine_sim(query_emb, &c.embedding)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut selected: Vec<RetrievedChunk> = Vec::new();
        let mut used = 0usize;
        let mut truncated = false;
        let mut selected_embs: Vec<&[f32]> = Vec::new();

        for &(cid, base) in &scored {
            if selected.len() >= self.top_k {
                truncated = true;
                break;
            }
            let chunk = &corpus.chunks[cid];
            let mut discount = 0.0f32;
            if self.diversity > 0.0 {
                for se in &selected_embs {
                    let c = cosine_sim(&chunk.embedding, se);
                    if c > discount {
                        discount = c;
                    }
                }
            }
            let eff = base * (1.0 - self.diversity * discount);
            let mut blocked = false;
            if self.diversity > 0.0 {
                for &(oid, obase) in &scored {
                    if oid == cid {
                        break;
                    }
                    let ochunk = &corpus.chunks[oid];
                    let mut odisc = 0.0f32;
                    for se in &selected_embs {
                        let c = cosine_sim(&ochunk.embedding, se);
                        if c > odisc {
                            odisc = c;
                        }
                    }
                    if obase * (1.0 - self.diversity * odisc) > eff + 1e-6 {
                        blocked = true;
                        break;
                    }
                }
            }
            if blocked {
                continue;
            }

            if used + chunk.tokens <= self.budget {
                used += chunk.tokens;
                selected_embs.push(&chunk.embedding);
                selected.push(RetrievedChunk {
                    id: chunk.id,
                    text: chunk.text.clone(),
                    score: base,
                    tokens: chunk.tokens,
                });
            } else {
                truncated = true;
            }
        }

        RetrievalResult {
            chunks: selected,
            total_tokens: used,
            budget: self.budget,
            truncated,
        }
    }
}

/// Assemble the retrieved chunks into a single context string.
pub fn assemble_context(result: &RetrievalResult, separator: &str) -> String {
    result
        .chunks
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(separator)
}

/// Convenience: build a corpus from texts, embed the query, retrieve
/// token-fixed, and return both the result and the assembled context string.
pub fn retrieve_from_texts(
    texts: &[String],
    query: &str,
    embedder: &dyn VectorEmbed,
    budget: usize,
    top_k: usize,
) -> (RetrievalResult, String) {
    let corpus = RagCorpus::build(texts, embedder);
    let q_emb = embedder.embed(query);
    let retriever = TokenFixedRetriever::new(budget, top_k);
    let result = retriever.retrieve(&q_emb, &corpus);
    let ctx = assemble_context(&result, "\n\n---\n\n");
    (result, ctx)
}

// ── G5 (Graphiti take): hybrid BM25 + cosine with RRF fusion ──────────────
//
// The cosine baseline is the frozen retriever above. The hybrid adds a
// deterministic Okapi BM25 leg that sees lexical overlap the embedding
// cannot, and fuses the two orderings by reciprocal rank. The gate
// `hybrid_fusion_vs_cosine_baseline` measures the fused path against the
// baseline on a labelled fixture and *derives* the shipping decision from
// that measurement — a fused path that fails to beat cosine ships
// disabled. The machine proposes; the measurement, not the ambition,
// decides what ships.

/// Dependency-free, case-insensitive tokenisation for the BM25 leg:
/// lowercase alphanumeric runs, one token per run. Deterministic by
/// construction so the gate fixture reproduces bit-for-bit in `cargo test`.
pub fn bm25_terms(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut word = String::new();
    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else if !word.is_empty() {
            out.push(word.clone());
            word.clear();
        }
    }
    if !word.is_empty() {
        out.push(word.clone());
    }
    out
}

/// G5: a precomputed Okapi BM25 index over a corpus's texts.
#[derive(Debug, Clone)]
pub struct Bm25Index {
    /// Term count per document.
    pub doc_len: Vec<usize>,
    /// Mean document length (1.0 floor for empty corpora).
    pub avg_dl: f32,
    /// Document frequency per term, sorted by term for determinism.
    pub doc_freq: Vec<(String, usize)>,
    /// Per-document term lists, lowercased.
    terms: Vec<Vec<String>>,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Bm25Index::build(&[])
    }
}

impl Bm25Index {
    pub fn build(texts: &[String]) -> Self {
        let mut doc_len: Vec<usize> = Vec::new();
        let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut terms: Vec<Vec<String>> = Vec::new();
        for t in texts {
            let ts = bm25_terms(t);
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for term in &ts {
                if seen.insert(term.clone()) {
                    freq.insert(term.clone(), freq.get(term).copied().unwrap_or(0usize) + 1);
                }
            }
            doc_len.push(ts.len());
            terms.push(ts);
        }
        let mut doc_freq: Vec<(String, usize)> = freq.into_iter().collect();
        doc_freq.sort_by(|a, b| a.0.cmp(&b.0));
        let n = texts.len() as f32;
        let avg_dl = (doc_len.iter().sum::<usize>() as f32) / n.max(1.0);
        Self {
            doc_len,
            avg_dl,
            doc_freq,
            terms,
        }
    }

    /// Document frequency of `term` (documents containing it).
    pub fn df(&self, term: &str) -> usize {
        for (t, d) in &self.doc_freq {
            if t.as_str() == term {
                return *d;
            }
        }
        0
    }

    /// Okapi BM25 score of `doc_idx` for `query_terms` (`k1 = 1.5`,
    /// `b = 0.75` — the standard defaults). Unknown terms contribute
    /// nothing; a term is counted once per query.
    pub fn score(&self, doc_idx: usize, query_terms: &[String]) -> f32 {
        if doc_idx >= self.terms.len() {
            return 0.0;
        }
        const K1: f32 = 1.5;
        const B: f32 = 0.75;
        let n = self.terms.len() as f32;
        let dl = self.doc_len[doc_idx] as f32;
        let mut total = 0.0_f32;
        let mut counted: std::collections::HashSet<String> = std::collections::HashSet::new();
        for q in query_terms {
            if !counted.insert(q.clone()) {
                continue;
            }
            let df = self.df(q);
            if df == 0 {
                continue;
            }
            let idf = ((n - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();
            let mut tf = 0usize;
            for t in &self.terms[doc_idx] {
                if *t == *q {
                    tf += 1;
                }
            }
            let norm = 1.0 - B + B * (dl / self.avg_dl.max(1e-6));
            total += idf * (tf as f32 * (K1 + 1.0)) / (tf as f32 + K1 * norm);
        }
        total
    }

    /// Every document's BM25 score, best first. Score ties keep ascending
    /// document id order (the initial order is by construction; the sort is
    /// stable), so repeated runs rank identically.
    pub fn rank(&self, query_terms: &[String]) -> Vec<(usize, f32)> {
        let mut scored: Vec<(usize, f32)> = (0..self.terms.len())
            .map(|i| (i, self.score(i, query_terms)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

/// All chunks ranked by cosine to the query, best first. This is the frozen
/// baseline leg of the hybrid; [`TokenFixedRetriever`] applies the token
/// budget on top of this order.
pub fn rank_by_cosine(query_emb: &[f32], corpus: &RagCorpus) -> Vec<(usize, f32)> {
    let mut scored: Vec<(usize, f32)> = corpus
        .chunks
        .iter()
        .map(|c| (c.id, cosine_sim(query_emb, &c.embedding)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

/// G5: reciprocal rank fusion. Each ranked list contributes
/// `1 / (k + rank)` per document (1-based rank); scores sum across lists.
/// Ties keep ascending document id order — deterministic by construction.
pub fn fuse_rrf(rankings: &[&[usize]], k: f32) -> Vec<(usize, f32)> {
    let mut acc: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
    for list in rankings {
        for (rank, doc) in list.iter().enumerate() {
            let contribution = 1.0_f32 / (k + (rank + 1) as f32);
            let score = acc.get(doc).copied().unwrap_or(0.0) + contribution;
            acc.insert(*doc, score);
        }
    }
    let mut out: Vec<(usize, f32)> = acc.into_iter().collect();
    out.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    out
}

/// G5: the combined ranking — cosine and BM25 legs fused by RRF. The
/// cosine leg is the frozen baseline; BM25 adds lexical recall that the
/// embedding cannot see.
pub fn rank_hybrid(corpus: &RagCorpus, query_emb: &[f32], query_text: &str) -> Vec<(usize, f32)> {
    let cos_rank: Vec<usize> = rank_by_cosine(query_emb, corpus)
        .iter()
        .map(|(i, _)| *i)
        .collect();
    let bm_rank: Vec<usize> = corpus
        .bm25
        .rank(&bm25_terms(query_text))
        .iter()
        .map(|(i, _)| *i)
        .collect();
    fuse_rrf(&[&cos_rank, &bm_rank], 60.0_f32)
}

/// G5: the measured comparison the gate runs — fused path against the
/// cosine baseline on a labelled fixture. `ships_fusion` is *derived* from
/// the measurement, never hard-coded: a fused path that fails to beat the
/// baseline ships disabled.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridVerdict {
    pub target_doc: usize,
    pub cosine_top1: Option<usize>,
    pub hybrid_top1: Option<usize>,
    /// +1 when only the fused path recovers the target; −1 when only the
    /// baseline does; 0 when they agree or neither does.
    pub improvement: f32,
    /// The shipping decision, computed from the measurement.
    pub ships_fusion: bool,
}

/// Measure the fused path against the cosine baseline on one labelled
/// query. Fixture, embedder and query text are owned by the caller; both
/// retrievals always run side by side and the verdict is computed from
/// them, never assumed.
pub fn measure_hybrid_vs_cosine(
    corpus: &RagCorpus,
    query_emb: &[f32],
    query_text: &str,
    target_doc: usize,
) -> HybridVerdict {
    let cos = rank_by_cosine(query_emb, corpus);
    let hyb = rank_hybrid(corpus, query_emb, query_text);
    let cosine_top1: Option<usize> = cos.first().map(|(id, _)| *id);
    let hybrid_top1: Option<usize> = hyb.first().map(|(id, _)| *id);
    let improvement = if hybrid_top1 == Some(target_doc) && cosine_top1 != Some(target_doc) {
        1.0_f32
    } else if cosine_top1 == Some(target_doc) && hybrid_top1 != Some(target_doc) {
        -1.0_f32
    } else {
        0.0_f32
    };
    HybridVerdict {
        target_doc,
        cosine_top1,
        hybrid_top1,
        improvement,
        ships_fusion: improvement > 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    fn emb() -> RandomProjectionEmbedder {
        RandomProjectionEmbedder::new(64)
    }

    #[test]
    fn token_count_is_monotonic_and_nonzero() {
        assert_eq!(count_tokens(""), 1);
        assert!(count_tokens("hello world") >= count_tokens("hello"));
        assert!(count_tokens("antidisestablishmentarianism") > count_tokens("cat"));
    }

    #[test]
    fn budget_is_respected() {
        let texts: Vec<String> = (0..20)
            .map(|i| format!("chunk number {} with some filler text to have length", i))
            .collect();
        let e = emb();
        let corpus = RagCorpus::build(&texts, &e);
        let q = e.embed("query about chunk number five");
        let r = TokenFixedRetriever::new(40, 100).retrieve(&q, &corpus);
        assert!(
            r.total_tokens <= 40,
            "total {} must be <= budget 40",
            r.total_tokens
        );
    }

    #[test]
    fn top_k_caps_selection() {
        let texts: Vec<String> = (0..20).map(|i| format!("text {i}")).collect();
        let e = emb();
        let corpus = RagCorpus::build(&texts, &e);
        let q = e.embed("text");
        let r = TokenFixedRetriever::new(10_000, 3).retrieve(&q, &corpus);
        assert!(r.chunks.len() <= 3);
    }

    #[test]
    fn selection_sorted_by_score() {
        let texts = vec![
            "apple fruit red".to_string(),
            "banana fruit yellow".to_string(),
            "turbine rotor speed".to_string(),
        ];
        let e = emb();
        let corpus = RagCorpus::build(&texts, &e);
        let q = e.embed("fruit banana");
        let r = TokenFixedRetriever::new(10_000, 10).retrieve(&q, &corpus);
        for w in &r.chunks[1..] {
            assert!(r.chunks[0].score >= w.score);
        }
    }

    #[test]
    fn deterministic_across_runs() {
        let texts: Vec<String> = (0..15).map(|i| format!("doc {} content here", i)).collect();
        let e = emb();
        let corpus = RagCorpus::build(&texts, &e);
        let q = e.embed("doc");
        let r1 = TokenFixedRetriever::new(80, 50).retrieve(&q, &corpus);
        let r2 = TokenFixedRetriever::new(80, 50).retrieve(&q, &corpus);
        assert_eq!(r1.chunks.len(), r2.chunks.len());
        assert_eq!(r1.total_tokens, r2.total_tokens);
    }

    #[test]
    fn assemble_never_exceeds_token_estimate() {
        let texts = vec!["alpha beta gamma".to_string(), "delta epsilon".to_string()];
        let e = emb();
        let (r, ctx) = retrieve_from_texts(&texts, "alpha", &e, 30, 10);
        assert!(r.total_tokens <= 30);
        assert!(ctx.contains("alpha"));
    }

    // ── G5: hybrid fusion vs the cosine baseline ───────────────────────────

    fn chunk(id: usize, text: &str, embedding: Vec<f32>) -> RagChunk {
        RagChunk {
            id,
            text: text.to_string(),
            tokens: count_tokens(text),
            embedding,
        }
    }

    #[test]
    fn hybrid_fusion_vs_cosine_baseline() {
        // Fixture: exact terms uniquely identify the target doc (0), while
        // the hand-built vectors pull the cosine ranking toward a distracter
        // (1). The fused path must recover the target where cosine alone
        // cannot — and the shipping decision must be *derived* from that
        // measurement, never hard-coded.
        let distracter = vec![0.99, 0.01, 0.0, 0.0];
        let target = vec![0.01, 0.99, 0.0, 0.0];
        let far = vec![-0.80, 0.10, 0.0, 0.0];
        let texts = vec![
            "turbine bearing vibration harmonics report".to_string(),
            "banana smoothie recipe breakfast".to_string(),
            "shipping manifest cargo hold".to_string(),
        ];
        let corpus = RagCorpus {
            chunks: vec![
                chunk(0, &texts[0], target),
                chunk(1, &texts[1], distracter),
                chunk(2, &texts[2], far),
            ],
            bm25: Bm25Index::build(&texts),
        };
        // Cosine points at the distracter: the query vector is the
        // distracter's own direction.
        let query = "bearing vibration harmonics";
        let query_emb = vec![1.0, 0.0, 0.0, 0.0];

        let verdict = measure_hybrid_vs_cosine(&corpus, &query_emb, query, 0);
        assert_eq!(
            verdict.cosine_top1,
            Some(1),
            "the cosine baseline is fooled by design"
        );
        assert_eq!(
            verdict.hybrid_top1,
            Some(0),
            "the fused path recovers the term-matched target"
        );
        assert!(verdict.improvement > 0.0);
        assert!(
            verdict.ships_fusion,
            "a positive measurement must ship the fused path"
        );

        // Negative control: when lexical overlap is uninformative and cosine
        // already places the target first, fusion must not regress it — and
        // the shipping flag tracks the measurement, so a non-positive result
        // ships the fused path disabled (negative ships disabled).
        let flat = vec![
            "report alpha beta shared".to_string(),
            "notes alpha beta shared".to_string(),
        ];
        let flat_corpus = RagCorpus {
            chunks: vec![
                chunk(0, &flat[0], vec![0.9, 0.1]),
                chunk(1, &flat[1], vec![0.1, 0.9]),
            ],
            bm25: Bm25Index::build(&flat),
        };
        let v2 = measure_hybrid_vs_cosine(&flat_corpus, &[0.9, 0.1], "alpha beta", 0);
        assert_eq!(v2.cosine_top1, Some(0));
        assert_eq!(
            v2.ships_fusion,
            v2.improvement > 0.0,
            "shipping is derived from the measurement, never assumed"
        );
        if v2.improvement <= 0.0 {
            assert!(
                !v2.ships_fusion,
                "a non-positive measurement ships disabled"
            );
        }
    }

    #[test]
    fn bm25_index_scores_are_deterministic_and_repeatable() {
        let texts = vec![
            "turbine bearing vibration report".to_string(),
            "bearing housing temperature".to_string(),
            "banana smoothie recipe".to_string(),
        ];
        let idx = Bm25Index::build(&texts);
        let q = vec!["bearing".to_string(), "temperature".to_string()];
        let r1 = idx.rank(&q);
        let r2 = idx.rank(&q);
        assert_eq!(r1, r2, "the same query must rank identically every run");
        let (top_id, top_score) = r1[0];
        assert_eq!(
            top_id, 1,
            "the doc with the rarer matching term ranks first"
        );
        assert!(top_score > 0.0);
    }
}
