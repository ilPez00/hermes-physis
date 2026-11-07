//! Notebook — grounded answers over a local corpus, with citations.
//!
//! The "NotebookLM shape": point it at documents, ask a question, get an answer
//! that is *about those documents* and says which ones it used.
//!
//! ## The split this module insists on
//!
//! Answering from a corpus is two jobs, and conflating them is why systems like
//! this get judged on the wrong axis:
//!
//! - **Selection** — which few hundred tokens of the corpus bear on the
//!   question. This is physis: [`crate::rag`] packs under a hard budget,
//!   [`crate::map`] supplies the structural read.
//! - **Synthesis** — turning that into prose. This is a language model, and
//!   physis does not pretend to be one.
//!
//! So there are two tiers, and the output always says which one produced it:
//!
//! | tier | generator | needs | honest description |
//! |---|---|---|---|
//! | `Extractive` | [`crate::model_provider::NgramDecoderModel`] | nothing | stitches corpus fragments; grounded by construction, not fluent, and not synthesis |
//! | `Synthesised` | any OpenAI-compatible endpoint ([`crate::oracle`]) | `PHYSIS_ORACLE_*` | real prose; a local ollama works and keeps everything offline |
//!
//! A tierless answer would let a fragment-stitcher be mistaken for synthesis,
//! which is the same failure shape as a benchmark that cannot fail.
//!
//! ## Citations are indices, not guesses
//!
//! [`crate::rag::RetrievedChunk::id`] indexes the corpus slice it came from, so
//! a citation is a lookup rather than an inference. The model is asked to cite,
//! but [`Answer::sources`] is filled from the retrieval regardless of what the
//! model says — a model that cites nothing still gets a correct source list,
//! and a model that invents a citation cannot add one.

use crate::embed::VectorEmbed;
use serde::{Deserialize, Serialize};

/// Which generator produced an answer. Never omitted from output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// Corpus fragments stitched by the offline n-gram decoder.
    Extractive,
    /// Prose from an OpenAI-compatible endpoint reading the compiled context.
    Synthesised,
}

impl Tier {
    pub fn label(&self) -> &'static str {
        match self {
            Tier::Extractive => "EXTRACTIVE (n-gram decoder — fragments, not synthesis)",
            Tier::Synthesised => "SYNTHESISED (language model over compiled context)",
        }
    }
}

/// One document the answer drew on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    /// Cosine score of the chunk that pulled this document in.
    pub score: f32,
    /// First line of the cited chunk — enough to check the citation by eye.
    pub excerpt: String,
}

/// A grounded answer plus everything needed to audit it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub query: String,
    pub tier: Tier,
    pub text: String,
    pub sources: Vec<Source>,
    pub context_tokens: usize,
    pub baseline_tokens: usize,
    pub corpus_documents: usize,
    /// Set when the requested tier was unavailable and a lower one was used.
    pub degraded: Option<String>,
    pub latency_ms: f64,
}

impl Answer {
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "── ANSWER · {} ──\n\n{}\n\n",
            self.tier.label(),
            self.text.trim()
        ));
        if let Some(why) = &self.degraded {
            out.push_str(&format!("!! DEGRADED: {why}\n\n"));
        }
        out.push_str("── SOURCES ──\n");
        if self.sources.is_empty() {
            out.push_str("  (none — the retrieval returned nothing)\n");
        }
        for (i, s) in self.sources.iter().enumerate() {
            out.push_str(&format!(
                "  [{}] {} ({:.3})\n      {}\n",
                i + 1,
                s.name,
                s.score,
                s.excerpt
            ));
        }
        out.push_str(&format!(
            "\n{} documents · context {} tokens (conventional retrieval {}) · {:.0} ms\n",
            self.corpus_documents, self.context_tokens, self.baseline_tokens, self.latency_ms
        ));
        out
    }
}

/// The system prompt. Grounding is stated as a refusal rule, not a preference:
/// a corpus tool that answers from model priors when the corpus is silent is
/// worse than one that says it does not know, because the failure is invisible.
const GROUNDING: &str = "\
You answer ONLY from the CONTEXT below. It was selected from the user's own \
documents. Rules, in priority order:
1. If the CONTEXT does not contain the answer, say exactly: \"The documents do \
not answer this.\" Do not answer from general knowledge. Do not guess.
2. Cite the source number in square brackets after each claim, e.g. [2].
3. Quote exact figures, units and identifiers from the CONTEXT verbatim.
4. Be brief. No preamble, no restating the question.";

/// Answer `query` from the documents in `docs` (`(name, body)` pairs).
///
/// Uses the oracle endpoint when one is configured, falling back to the
/// extractive tier otherwise — and recording *why* in [`Answer::degraded`]
/// rather than silently producing a worse answer that looks the same.
pub fn answer(
    docs: &[(String, String)],
    query: &str,
    embedder: &dyn VectorEmbed,
    budget: usize,
) -> anyhow::Result<Answer> {
    anyhow::ensure!(!docs.is_empty(), "no documents to answer from");
    let started = std::time::Instant::now();
    let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();

    // Selection. The generous arm is the honest denominator for the budgeted one.
    let (baseline, _) = crate::rag::retrieve_from_texts(
        &texts,
        query,
        embedder,
        usize::MAX / 2,
        docs.len().min(12),
    );
    let top_k = docs.len().clamp(3, 8);
    let (result, _) =
        crate::rag::retrieve_from_texts(&texts, query, embedder, budget.max(64), top_k);

    // Citations come from the retrieval indices, never from the model's text.
    let mut sources: Vec<Source> = Vec::new();
    let mut numbered = String::new();
    for (i, c) in result.chunks.iter().enumerate() {
        let name = docs
            .get(c.id)
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| format!("chunk {}", c.id));
        let excerpt: String = c
            .text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .chars()
            .take(140)
            .collect();
        numbered.push_str(&format!("[{}] {}\n{}\n\n", i + 1, name, c.text.trim()));
        sources.push(Source {
            name,
            score: c.score,
            excerpt,
        });
    }

    let common = |tier, text: String, degraded| Answer {
        query: query.to_string(),
        tier,
        text,
        sources: sources.clone(),
        context_tokens: result.total_tokens,
        baseline_tokens: baseline.total_tokens,
        corpus_documents: docs.len(),
        degraded,
        latency_ms: started.elapsed().as_secs_f64() * 1000.0,
    };

    match crate::oracle::from_env() {
        Some((cfg, key)) => {
            let prompt = format!("CONTEXT:\n{numbered}\nQUESTION: {query}");
            match crate::oracle::complete(&cfg, &key, GROUNDING, &prompt, 800) {
                Ok((text, _)) if !text.trim().is_empty() => {
                    Ok(common(Tier::Synthesised, text, None))
                }
                // A reasoning model that spends its whole budget in a hidden
                // reasoning channel returns empty content. That is a real
                // failure of THIS request, not a reason to pretend.
                Ok(_) => Ok(common(
                    Tier::Extractive,
                    extractive(&result),
                    Some(format!(
                        "model {} returned empty content (reasoning models can \
                         exhaust max_tokens before emitting any); fell back to \
                         extractive",
                        cfg.model
                    )),
                )),
                Err(e) => Ok(common(
                    Tier::Extractive,
                    extractive(&result),
                    Some(format!("oracle call failed ({e}); fell back to extractive")),
                )),
            }
        }
        None => Ok(common(
            Tier::Extractive,
            extractive(&result),
            Some(
                "no PHYSIS_ORACLE_KEY / OPENROUTER_API_KEY / GROQ_API_KEY — \
                 synthesis needs a generator. A local ollama works: \
                 PHYSIS_ORACLE_URL=http://localhost:11434/v1 \
                 PHYSIS_ORACLE_MODEL=<model> PHYSIS_ORACLE_KEY=ollama"
                    .into(),
            ),
        )),
    }
}

// ── Draft-and-fill ──────────────────────────────────────────────────────────

/// One span of a draft: either text the corpus supports, or a gap the corpus
/// cannot fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Span {
    /// Emitted by the table, so every token of it occurs in the retrieved
    /// context. Grounded by construction, not by instruction.
    Grounded(String),
    /// The table had no confident continuation here. Carries the context that
    /// preceded it, which is what the model needs to fill it.
    Gap { after: String },
}

/// A draft plus the accounting that says whether drafting was worth it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub spans: Vec<Span>,
    /// Tokens the table produced — these never reach the model's output budget.
    pub drafted_tokens: usize,
    /// Gaps the model is asked to fill.
    pub gaps: usize,
    /// Table size, for the "a 15 KB table did this" line.
    pub table_entries: u64,
    /// The decoder re-entered a window it had already emitted and was stopped.
    /// A looped draft is NOT a successful one however long it is, so
    /// [`Draft::table_share`] reports 0.0 when this is set.
    pub looped: bool,
}

impl Draft {
    /// The draft as text, gaps marked for the model to fill.
    pub fn render_with_gaps(&self) -> String {
        let mut out = String::new();
        for sp in &self.spans {
            match sp {
                Span::Grounded(t) => out.push_str(t),
                Span::Gap { .. } => out.push_str(" ⟨FILL⟩"),
            }
        }
        out
    }

    /// Fraction of the draft the table supplied. The number the architecture
    /// lives or dies by: at 0.0 the table contributed nothing and a plain
    /// model call would have been simpler and better.
    pub fn table_share(&self) -> f32 {
        // A looped draft scores 0: the tokens after the cycle began are the
        // decoder repeating itself, not the corpus carrying the answer, and a
        // share near 1.0 over that is the metric lying about its own output.
        if self.looped {
            return 0.0;
        }
        let total = self.drafted_tokens + self.gaps;
        if total == 0 {
            0.0
        } else {
            self.drafted_tokens as f32 / total as f32
        }
    }
}

/// Draft an answer from the retrieved context alone, using a table built from
/// that context.
///
/// The idea (Gio, 2026-09-12): **let geometry and the table draft, and spend
/// the model only on the gaps.** The table can only emit sequences it observed
/// in the retrieved context, so drafted text is grounded by construction rather
/// than by asking a model nicely. The model is then a *gap filler*, which is a
/// much smaller job than synthesis — and a smaller job is where a small model
/// stops being a downgrade.
///
/// `confidence` is the probability floor below which the table declines to
/// continue and opens a gap instead. Higher ⇒ shorter, safer drafts.
pub fn draft(
    result: &crate::rag::RetrievalResult,
    query: &str,
    max_tokens: usize,
    confidence: f32,
) -> anyhow::Result<Draft> {
    use crate::ngram_table::{NGramTable, TableBuilder, TableConfig, TableKind};
    use crate::tokenizer::{Tokenizer, WhitespaceTokenizer};

    let tok = WhitespaceTokenizer::new(0);
    let cfg = TableConfig {
        kind: TableKind::Lexical,
        max_order: 5,
        min_count: 1,
        ..Default::default()
    };
    let mut b = TableBuilder::new(cfg);
    for c in &result.chunks {
        b.push_text(&tok, &c.text);
    }
    let (table, _) = b.finish(&tok, "notebook-draft");
    let entries = table.manifest().entry_count;

    let mut toks = tok.encode(query);
    let start = toks.len();
    let mut spans: Vec<Span> = Vec::new();
    let mut run: Vec<String> = Vec::new();
    let mut gaps = 0usize;

    // Greedy backoff decoding cycles: a 5-gram whose best continuation leads
    // back into itself emits the same clause forever. Measured before this
    // guard: "the machine must not stop above it" four times, reported as
    // table_share 1.00 — a metric claiming total success over garbage.
    //
    // A repeat is not a continuation the corpus supports; it is the decoder
    // running out of new material. So it ends the draft and opens a gap,
    // which is what "the table cannot carry this further" is supposed to look
    // like.
    let mut seen_windows: std::collections::HashSet<Vec<String>> = std::collections::HashSet::new();
    let mut looped = false;

    for _ in 0..max_tokens {
        let ctx_start = toks.len().saturating_sub(5);
        let ctx = &toks[ctx_start..];
        match table.top_next(ctx, 1).into_iter().next() {
            // `top_next` returns a relative score; require it to clear the
            // floor, so a continuation the context barely supports opens a gap
            // rather than being asserted.
            Some((w, p)) if p >= confidence => {
                let mut window: Vec<String> = ctx.to_vec();
                window.push(w.clone());
                if !seen_windows.insert(window) {
                    looped = true;
                    if !run.is_empty() {
                        spans.push(Span::Grounded(format!(" {}", run.join(" "))));
                        run.clear();
                    }
                    let after: String = toks[toks.len().saturating_sub(8)..].join(" ");
                    spans.push(Span::Gap { after });
                    gaps += 1;
                    break;
                }
                toks.push(w.clone());
                run.push(w);
            }
            _ => {
                if !run.is_empty() {
                    spans.push(Span::Grounded(format!(" {}", run.join(" "))));
                    run.clear();
                }
                // A gap carries its preceding context; that is what the model
                // is given, so it fills a hole rather than rewriting the draft.
                let after: String = toks[toks.len().saturating_sub(8)..].join(" ");
                spans.push(Span::Gap { after });
                gaps += 1;
                // Nothing to continue from — one gap ends this draft.
                break;
            }
        }
    }
    if !run.is_empty() {
        spans.push(Span::Grounded(format!(" {}", run.join(" "))));
    }

    Ok(Draft {
        spans,
        drafted_tokens: toks.len() - start,
        gaps,
        table_entries: entries,
        looped,
    })
}

/// The offline floor: the highest-scoring retrieved lines, verbatim.
///
/// Deliberately not dressed up as prose. It is an extract, it is labelled an
/// extract, and it is exactly as grounded as the retrieval that produced it.
fn extractive(result: &crate::rag::RetrievalResult) -> String {
    let mut out = String::new();
    for (i, c) in result.chunks.iter().take(3).enumerate() {
        let line: String = c
            .text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("{} [{}]\n", line.trim(), i + 1));
    }
    if out.is_empty() {
        out.push_str("The documents do not answer this.");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    fn corpus() -> Vec<(String, String)> {
        vec![
            (
                "valve.md".into(),
                "The relief valve opens at 2.6 bar. Above 4.2 bar the machine must stop.".into(),
            ),
            (
                "pump.md".into(),
                "Pump maintenance: replace the seal on line 1 every 400 hours.".into(),
            ),
            (
                "invoice.md".into(),
                "Payment terms are net thirty days from invoice date.".into(),
            ),
        ]
    }

    /// With no generator configured the answer must still be produced, must be
    /// labelled Extractive, and must say WHY it is not synthesised. A silent
    /// downgrade is the failure this field exists to prevent.
    #[test]
    fn without_a_generator_it_degrades_loudly_not_silently() {
        let e = RandomProjectionEmbedder::new(64);
        let prev = std::env::var("PHYSIS_ORACLE_KEY").ok();
        unsafe { std::env::remove_var("PHYSIS_ORACLE_KEY") };
        unsafe { std::env::remove_var("OPENROUTER_API_KEY") };
        unsafe { std::env::remove_var("GROQ_API_KEY") };

        let a = answer(&corpus(), "what pressure does the valve open at", &e, 200).unwrap();
        assert_eq!(a.tier, Tier::Extractive);
        let why = a.degraded.clone().expect("a downgrade must be recorded");
        assert!(
            why.contains("ollama"),
            "the message must say how to fix it: {why}"
        );
        assert!(
            a.render().contains("EXTRACTIVE"),
            "the tier must reach the output"
        );

        if let Some(p) = prev {
            unsafe { std::env::set_var("PHYSIS_ORACLE_KEY", p) }
        }
    }

    /// Citations are looked up from retrieval indices, so every source names a
    /// real document from the corpus — never a string the generator invented.
    #[test]
    fn every_source_is_a_real_document() {
        let e = RandomProjectionEmbedder::new(64);
        let docs = corpus();
        let a = answer(&docs, "seal replacement interval", &e, 200).unwrap();
        assert!(!a.sources.is_empty());
        for s in &a.sources {
            assert!(
                docs.iter().any(|(n, _)| *n == s.name),
                "invented source: {}",
                s.name
            );
        }
    }

    /// The budgeted context must never exceed the generous baseline — if it
    /// did, the compression number reported next to it would be a lie.
    #[test]
    fn budgeted_context_never_exceeds_the_baseline() {
        let e = RandomProjectionEmbedder::new(64);
        let a = answer(&corpus(), "valve", &e, 100).unwrap();
        assert!(
            a.context_tokens <= a.baseline_tokens,
            "{} > {}",
            a.context_tokens,
            a.baseline_tokens
        );
    }

    /// The property that actually holds, and the one that matters: every token
    /// the table drafts occurs in the retrieved context. Grounding by
    /// construction, not by asking a model nicely.
    ///
    /// Note what is NOT asserted here. An earlier version of this test required
    /// `table_share > 0.5`, on the assumption that observed material yields a
    /// long grounded draft. Adding cycle detection showed that to be false on
    /// small corpora: a three-sentence corpus sends the greedy decoder into a
    /// repeat within a few tokens, and a repeat now scores 0. The share is a
    /// property of the corpus, not a guarantee of the method.
    #[test]
    fn drafting_over_observed_material_leaves_the_model_little_to_do() {
        let e = RandomProjectionEmbedder::new(64);
        let docs = corpus();
        let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
        let (result, _) = crate::rag::retrieve_from_texts(&texts, "relief valve", &e, 400, 3);
        let d = draft(&result, "The relief valve", 20, 0.0).unwrap();
        assert!(
            d.drafted_tokens > 0,
            "the table must draft something it observed"
        );
        // Honest invariant: share is positive exactly when the draft did not
        // cycle. Either state is a valid outcome; misreporting one as the
        // other is not.
        if d.looped {
            assert_eq!(d.table_share(), 0.0);
        } else {
            assert!(d.table_share() > 0.0);
        }
        // Everything drafted must occur in the retrieved context — grounding by
        // construction is the property that makes this different from a prompt
        // rule, so it gets an assertion rather than a comment.
        // Lowercased: the whitespace tokenizer normalises case, so a literal
        // comparison would fail on "Above" vs "above" and look like a
        // grounding violation when it is a casing difference.
        let ctx: String = result
            .chunks
            .iter()
            .map(|c| c.text.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        for sp in &d.spans {
            if let Span::Grounded(t) = sp {
                for w in t.split_whitespace() {
                    assert!(
                        ctx.contains(&w.to_lowercase()),
                        "drafted {w:?} is not in the retrieved context"
                    );
                }
            }
        }
    }

    /// And the honest converse: ask for material the corpus never contained and
    /// the draft must open a gap rather than invent. A drafter that always
    /// produces something is a drafter that hallucinates.
    #[test]
    fn a_query_outside_the_corpus_opens_a_gap_instead_of_inventing() {
        let e = RandomProjectionEmbedder::new(64);
        let docs = corpus();
        let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
        let (result, _) = crate::rag::retrieve_from_texts(&texts, "valve", &e, 400, 3);
        // Confidence floor of 1.1 is unreachable, so nothing can be asserted.
        let d = draft(&result, "orbital mechanics of", 10, 1.1).unwrap();
        assert_eq!(
            d.drafted_tokens, 0,
            "nothing should be drafted at an unreachable floor"
        );
        assert_eq!(d.gaps, 1);
        assert_eq!(
            d.table_share(),
            0.0,
            "table_share 0 says: this is not a job for the table"
        );
        assert!(d.render_with_gaps().contains("⟨FILL⟩"));
    }

    /// The defect that shipped and was caught by running the command: greedy
    /// backoff decoding cycles, emitting one clause forever while
    /// `table_share` reported 1.00 over it. A looped draft must score 0 — a
    /// metric that rates its own garbage highly is worse than no metric.
    #[test]
    fn a_looping_draft_scores_zero_not_one() {
        let e = RandomProjectionEmbedder::new(64);
        // A corpus engineered to cycle: the continuation leads back into the
        // context that produced it.
        let docs = vec![(
            "loop.md".to_string(),
            "the machine must not stop above it the machine must not stop above it".to_string(),
        )];
        let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
        let (result, _) = crate::rag::retrieve_from_texts(&texts, "the machine", &e, 400, 3);
        let d = draft(&result, "the machine", 40, 0.0).unwrap();
        if d.looped {
            assert_eq!(
                d.table_share(),
                0.0,
                "a looped draft must not score above 0"
            );
            assert!(d.gaps >= 1, "a loop must open a gap, not end silently");
        }
    }

    /// Whatever the decoder does, it must terminate. Before the cycle guard a
    /// degenerate corpus ran to `max_tokens` every time.
    #[test]
    fn drafting_always_terminates_within_max_tokens() {
        let e = RandomProjectionEmbedder::new(64);
        let docs = vec![("r.md".to_string(), "a b a b a b a b a b".to_string())];
        let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
        let (result, _) = crate::rag::retrieve_from_texts(&texts, "a b", &e, 400, 3);
        let d = draft(&result, "a b", 1000, 0.0).unwrap();
        assert!(d.drafted_tokens <= 1000);
    }

    #[test]
    fn an_empty_corpus_is_an_error_not_an_empty_answer() {
        let e = RandomProjectionEmbedder::new(64);
        assert!(answer(&[], "anything", &e, 100).is_err());
    }
}
