//! The chain — every measured mechanism in this crate, composed, with its null.
//!
//! ## Why this module exists
//!
//! A dependency audit of this crate (2026-09-12) found that the mechanisms with
//! the **strongest measured evidence compose with nothing**. Each had in-degree
//! zero — exported, tested, reachable from no other module:
//!
//! | mechanism | measured against a construction-matched null | in-degree |
//! |---|---|---|
//! | [`crate::propose`] | top-3 0.712 vs permuted-label null 0.136 (+0.531) | 0 |
//! | [`crate::transform`] | link prediction +0.291 / +0.189 (E53) | 0 |
//! | [`crate::coverage`] | duplicate control Welch *t* = +12.11, 2000 → 141 | 0 |
//! | [`crate::becoming`] | order-aware drift vs collision | 0 |
//!
//! Meanwhile composition had been invested in the retrieval stack, whose own
//! benchmark scores *identically* under a real sentence transformer and a
//! random-projection lexical hash. The project had wired its weakest-evidenced
//! stack and stranded its strongest-evidenced parts. This module is the fix.
//!
//! ## The pass
//!
//! ```text
//!   corpus
//!     ├─ map        structure: repeats, differences, contradictions
//!     ├─ rag        the bounded context (what a model should read)
//!     ├─ coverage   which records the ontology cannot place today
//!     ├─ propose    a SHORTLIST per unplaced record — never a verdict
//!     ├─ coverage   what each shortlisted cell would actually buy
//!     ├─ becoming   is a recurring term drifting, or two things sharing a name
//!     └─ null       the same pass over shuffled labels
//! ```
//!
//! ## The null is not optional
//!
//! Every stage above earned its place by beating a construction-matched
//! control, so the composition ships one too: [`ChainReport::null`] runs the
//! proposal stage over a **label-permuted** classifier — same cells, same
//! embeddings, shuffled cell assignment. Degree and geometry survive; only the
//! pairing dies. A chain that cannot beat that is reporting geometry, not
//! knowledge, and [`ChainReport::discriminates`] says so in the output rather
//! than leaving it to the reader.
//!
//! ## What this deliberately is not
//!
//! It does not decide anything. `propose` returns a shortlist because asking
//! the machine to *judge* ("does this belong here?") was refuted at chance
//! across this project's whole research track, while asking it to *propose*
//! ("which few should a person look at?") was not. That distinction is the
//! reason the chain is useful and the reason it stops where it does.

use crate::classify::{Cell, CellClassifier};
use crate::embed::VectorEmbed;
use serde::{Deserialize, Serialize};

/// One record the ontology cannot confidently place, with what to do about it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unplaced {
    pub id: String,
    /// Best cosine any existing entry achieves — how far from placed it is.
    pub live_score: f32,
    /// The shortlist. Read as "look at these first", not as an answer.
    pub shortlist: Vec<String>,
    /// `CandidateGain::lift` of the top-ranked candidate: total confidence it
    /// would add across every record. Threshold-independent, and a candidate
    /// duplicating an existing entry scores exactly 0.0.
    pub top_gain: f32,
}

/// A term whose occurrences suggest its meaning moved, or split in two.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drift {
    pub term: String,
    /// `Becoming` (time-separated: it changed) or `Split` (interleaved: two
    /// things share a name). Order is what separates these, and it is the one
    /// signal every other similarity method in this crate throws away.
    pub trajectory: String,
    pub separation: f32,
    pub runs_z: f64,
}

/// Everything one pass produces, including the arm that could sink it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainReport {
    pub documents: usize,
    // structure (map)
    pub repeats: usize,
    pub differences: usize,
    pub contradictions: usize,
    pub structure_hash: String,
    // context (rag)
    pub baseline_tokens: usize,
    pub context_tokens: usize,
    // coverage + propose
    pub unplaced: Vec<Unplaced>,
    /// Mean gain of the top shortlisted cell over all unplaced records.
    pub mean_gain: f32,
    /// The same statistic under a label-permuted classifier.
    pub null: f32,
    // becoming
    pub drifts: Vec<Drift>,
    /// New cells `discovery` proposed from the texts the ontology misses.
    pub candidates: usize,
    pub embedder: String,
}

impl ChainReport {
    /// Did the real arm beat its control by a margin worth acting on?
    ///
    /// 0.02 is deliberately low: the question is whether there is *any* signal
    /// above the permuted arm, not whether it is large. A chain that cannot
    /// clear this is not reporting knowledge.
    pub fn discriminates(&self) -> bool {
        self.testable() && self.mean_gain - self.null > 0.02
    }

    /// Was there anything for the control to be compared against?
    ///
    /// With no unplaced records both arms score 0.0 and the comparison is
    /// vacuous. Reporting that as "not above the null" would be the same
    /// category error as a benchmark that cannot fail: it states a negative
    /// result where no test was run. "No gaps found" is a finding about the
    /// corpus; "no signal" is a finding about the method, and they must not
    /// print the same line.
    pub fn testable(&self) -> bool {
        // Three ways the comparison can be vacuous, and all three have been hit
        // while building this: no unplaced records, no candidates to score, and
        // both arms scoring exactly zero because no proposal can lift anything.
        //
        // The last one is the subtle one. `lift` is max(0, cos(candidate) -
        // live), so on a corpus the ontology already matches at 0.92 no new cell
        // can clear the bar and BOTH arms are 0.0 by arithmetic. Printing "not
        // above the null" there states a verdict on the method where no test
        // ran — the same category error as a benchmark that cannot fail, which
        // is the thing this whole module exists to avoid.
        !self.unplaced.is_empty()
            && self.candidates > 0
            && (self.mean_gain > 0.0 || self.null > 0.0)
    }

    pub fn render(&self) -> String {
        let mut o = String::new();
        o.push_str(&format!(
            "── PHYSIS CHAIN ──\n{} documents · embedder {}\n\n",
            self.documents, self.embedder
        ));
        o.push_str(&format!(
            "STRUCTURE   {} repeats · {} differences · {} contradictions ({})\n",
            self.repeats,
            self.differences,
            self.contradictions,
            &self.structure_hash[..12.min(self.structure_hash.len())]
        ));
        let pct = if self.baseline_tokens == 0 {
            0.0
        } else {
            100.0 * (1.0 - self.context_tokens as f32 / self.baseline_tokens as f32)
        };
        o.push_str(&format!(
            "CONTEXT     {} → {} tokens ({:.0}% smaller)\n",
            self.baseline_tokens, self.context_tokens, pct
        ));
        o.push_str(&format!(
            "COVERAGE    {} record(s) the ontology cannot place · {} new cell(s) proposed\n\n",
            self.unplaced.len(),
            self.candidates
        ));

        if !self.unplaced.is_empty() {
            o.push_str("UNPLACED — shortlist, not a verdict\n");
            for u in self.unplaced.iter().take(8) {
                o.push_str(&format!(
                    "  {:<34} live {:.3}  gain {:.3}\n      → {}\n",
                    u.id.chars().take(34).collect::<String>(),
                    u.live_score,
                    u.top_gain,
                    u.shortlist.join("  |  ")
                ));
            }
            o.push('\n');
        }

        if !self.drifts.is_empty() {
            o.push_str("DRIFT — order-aware: did it change, or is it two things?\n");
            for d in self.drifts.iter().take(6) {
                o.push_str(&format!(
                    "  {:<24} {:<9} separation {:.3}  runs_z {:+.2}\n",
                    d.term, d.trajectory, d.separation, d.runs_z
                ));
            }
            o.push('\n');
        }

        if !self.testable() {
            let why = if self.unplaced.is_empty() {
                "every record is already placed, so there is no shortlist to score"
            } else if self.candidates == 0 {
                "discovery proposed no new cells, so there was nothing to rank"
            } else {
                "no proposed cell lifts any record above what the live ontology \n\
                 \x20           already scores, so both arms are 0.0 by arithmetic"
            };
            o.push_str(&format!(
                "CONTROL     not run — {why}.\n\
                 \x20           A fact about this corpus, not a verdict on the method.\n"
            ));
            return o;
        }
        o.push_str(&format!(
            "CONTROL     real {:.4} vs shuffled-grouping null {:.4}   Δ {:+.4}\n",
            self.mean_gain,
            self.null,
            self.mean_gain - self.null
        ));
        o.push_str(if self.discriminates() {
            "            The shortlist carries signal the permuted arm does not.\n"
        } else {
            "            NOT ABOVE THE NULL. This pass is reporting geometry, not\n\
             \x20           knowledge. Do not act on the shortlist.\n"
        });
        o
    }
}

/// The control: candidates built by the same pipeline from the same texts, with
/// discovery's *clustering* replaced by an arbitrary grouping.
///
/// ## Two nulls that were wrong first, recorded so neither is rebuilt
///
/// **Null 1 — permuted cell labels.** Measured `Δ +0.0000`, exactly, and that
/// was not luck. [`crate::coverage::CandidateGain::lift`] is
/// `max(0, cos(candidate, record) - live(record))`; `live` is a max over every
/// entry embedding. Neither term reads a label, so the arm was *provably
/// invariant* — a control that cannot move the statistic it controls.
///
/// **Null 2 — candidates drawn from the corpus at random.** Measured
/// `Δ -2.6247`: the null beat the real arm by 37x. Also not luck. A candidate
/// whose embedding *is* a corpus document scores ~1.0 against that document, so
/// the arm was not a null but a cheat — it smuggled the answers in as the
/// control.
///
/// The lesson both share: a null must differ from the real arm in exactly one
/// thing, and that thing must be the claim. Here the claim is *discovery chose
/// these groupings well*, so the null keeps the texts, the count, the
/// name-and-hints construction and the embedding path, and replaces only which
/// text lands in which group.
fn shuffled_grouping_candidates(
    real: &[Cell],
    texts: &[String],
    embedder: &dyn VectorEmbed,
    seed: u64,
) -> Vec<Cell> {
    if real.is_empty() || texts.is_empty() {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..texts.len()).collect();
    let mut s = seed | 1;
    for i in (1..order.len()).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        order.swap(i, (s % (i as u64 + 1)) as usize);
    }
    let mut cursor = 0usize;
    real.iter()
        .map(|c| {
            // Same number of member embeddings as the real candidate, built the
            // same way — from text — but from arbitrarily chosen texts.
            // Build the name and hints EXACTLY as discovery does — `top_terms`
            // over the group's member texts — so the only difference between
            // the arms is which texts landed in the group.
            //
            // An earlier attempt embedded raw document text here and the null
            // beat the real arm 37x. That was not discovery losing; it was the
            // arms constructing their candidates differently, so a document
            // vector was being compared against an abstract cluster name.
            let n = c.embeddings.len().max(1);
            let group: Vec<&str> = (0..n.max(3))
                .map(|_| {
                    let t = texts[order[cursor % order.len()]].as_str();
                    cursor += 1;
                    t
                })
                .collect();
            let hints = crate::discovery::top_terms(&group, 8);
            let mut words: Vec<String> = hints.iter().take(2).cloned().collect();
            for w in &mut words {
                if let Some(ch) = w.get_mut(0..1) {
                    let up = ch.to_uppercase();
                    w.replace_range(0..1, &up);
                }
            }
            let name = if words.is_empty() {
                "Unnamed Cluster".to_string()
            } else {
                words.join(" & ")
            };
            let mut embeddings = vec![embedder.embed(&name)];
            embeddings.extend(
                hints
                    .iter()
                    .take(n.saturating_sub(1))
                    .map(|h| embedder.embed(h)),
            );
            let n = embeddings.len();
            Cell {
                domain: c.domain.clone(),
                mode: c.mode.clone(),
                embeddings,
                entries: std::iter::repeat_n(name.clone(), n).collect(),
                facets: vec![Default::default(); n],
            }
        })
        .collect()
}

/// Mean top-candidate gain over the unplaced records — the statistic both the
/// real and permuted arms are scored on.
fn mean_top_gain(
    clf: &CellClassifier,
    candidates: &[Cell],
    records: &[(String, Vec<f32>)],
    unplaced: &[String],
    threshold: f32,
) -> f32 {
    if unplaced.is_empty() || candidates.is_empty() {
        return 0.0;
    }
    let ranked = crate::coverage::rank_candidates(clf, candidates, records, threshold);
    ranked
        .first()
        .map(|(_, g)| g.lift.max(0.0) / unplaced.len() as f32)
        .unwrap_or(0.0)
}

/// Run the whole pass over `docs`.
pub fn run(
    docs: &[(String, String)],
    query: &str,
    embedder: &dyn VectorEmbed,
    clf: &CellClassifier,
    budget: usize,
    threshold: f32,
    embedder_kind: &str,
) -> anyhow::Result<ChainReport> {
    anyhow::ensure!(!docs.is_empty(), "no documents to run the chain over");

    // ── structure ───────────────────────────────────────────────────────────
    let map = crate::map::build_map(docs, embedder, None)?;

    // ── context ─────────────────────────────────────────────────────────────
    let ctx = crate::map::compile_context(docs, embedder, query, budget)?;

    // ── coverage: what cannot be placed ─────────────────────────────────────
    let records: Vec<(String, Vec<f32>)> = docs
        .iter()
        .map(|(name, body)| (name.clone(), embedder.embed(body)))
        .collect();
    let unplaced_ids = crate::coverage::uncovered(clf, &records, threshold);

    // ── propose: a shortlist for each, built from the ontology's own cells ──
    let proposer = crate::propose::Proposer::from_decisions(clf.cells.iter().flat_map(|c| {
        c.embeddings
            .iter()
            .map(move |e| (c.domain.clone(), c.mode.clone(), e.clone()))
    }));

    // Candidates must be NEW cells, not the ontology's own. `CandidateGain`
    // scores a duplicate of an existing entry at exactly 0.0 by construction,
    // so ranking the live cells against themselves returns 0.000 for every
    // record — which is what the first run of this chain did, uniformly, and
    // it looked like a result rather than the tautology it was.
    //
    // `discovery` is the module that proposes genuinely new cells from the
    // texts the ontology fails to cover. That is the input coverage was
    // written to score.
    let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
    let disc = crate::discovery::discover(
        &texts,
        clf,
        embedder,
        &crate::discovery::DiscoveryConfig::default(),
    );
    let candidates: Vec<Cell> = disc
        .proposals
        .iter()
        .map(|p| {
            let mut emb = vec![embedder.embed(&p.name)];
            emb.extend(p.hints.iter().take(4).map(|h| embedder.embed(h)));
            let n = emb.len();
            Cell {
                domain: p.domain.clone(),
                mode: p.mode.clone(),
                embeddings: emb,
                entries: std::iter::repeat_n(p.name.clone(), n).collect(),
                facets: vec![Default::default(); n],
            }
        })
        .collect();

    let ranked = crate::coverage::rank_candidates(clf, &candidates, &records, threshold);
    let top_gain = ranked.first().map(|(_, g)| g.lift).unwrap_or(0.0);

    let mut unplaced = Vec::new();
    for id in &unplaced_ids {
        let Some((_, emb)) = records.iter().find(|(n, _)| n == id) else {
            continue;
        };
        let live = clf.best_entry_sim(emb).map(|(s, _, _)| s).unwrap_or(0.0);
        unplaced.push(Unplaced {
            id: id.clone(),
            live_score: live,
            shortlist: proposer.propose(emb, 3).iter().map(|p| p.cell()).collect(),
            top_gain,
        });
    }

    // ── becoming: order matters, so repeats are read in document order ──────
    let mut drifts = Vec::new();
    for family in map.repeats.iter().take(8) {
        let occs: Vec<crate::becoming::Occurrence> = docs
            .iter()
            .enumerate()
            .filter(|(_, (name, _))| family.instances.iter().any(|i| i.path == *name))
            .map(|(i, (_, body))| crate::becoming::Occurrence {
                at: i as i64,
                context: embedder.embed(body),
            })
            .collect();
        if occs.len() < 6 {
            continue; // `becoming` says so itself rather than guessing
        }
        let v = crate::becoming::classify(&occs);
        if !matches!(v.trajectory, crate::becoming::Trajectory::Stable) {
            drifts.push(Drift {
                term: family.name.clone(),
                trajectory: format!("{:?}", v.trajectory),
                separation: v.separation,
                runs_z: v.runs_z,
            });
        }
    }

    // ── the control, same pass, shuffled labels ─────────────────────────────
    let null_candidates =
        shuffled_grouping_candidates(&candidates, &texts, embedder, 0x9E3779B97F4A7C15);
    // Same classifier, same records, same candidate count, same construction.
    // The ONLY difference is whether discovery chose the grouping.
    let null = mean_top_gain(clf, &null_candidates, &records, &unplaced_ids, threshold);
    let mean_gain = mean_top_gain(clf, &candidates, &records, &unplaced_ids, threshold);

    Ok(ChainReport {
        documents: docs.len(),
        repeats: map.repeats.len(),
        differences: map.differences.len(),
        contradictions: map.contradictions.len(),
        structure_hash: map.structure_hash.clone(),
        baseline_tokens: ctx.baseline_tokens,
        context_tokens: ctx.physis_tokens,
        unplaced,
        mean_gain,
        null,
        drifts,
        candidates: candidates.len(),
        embedder: embedder_kind.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    fn corpus() -> Vec<(String, String)> {
        vec![
            (
                "a.md".into(),
                "Pump maintenance: replace the seal on line 1.".into(),
            ),
            (
                "b.md".into(),
                "Pump maintenance: replace the seal on line 2.".into(),
            ),
            ("c.md".into(), "Payment terms are net thirty days.".into()),
        ]
    }

    /// A vacuous comparison must never print as a lost one. Three ways it can be
    /// vacuous, and all three were hit while building this module.
    #[test]
    fn a_vacuous_comparison_is_not_a_verdict() {
        let mut r = ChainReport {
            documents: 1,
            repeats: 0,
            differences: 0,
            contradictions: 0,
            structure_hash: "x".into(),
            baseline_tokens: 10,
            context_tokens: 5,
            unplaced: vec![],
            mean_gain: 0.0,
            null: 0.0,
            drifts: vec![],
            candidates: 0,
            embedder: "test".into(),
        };
        assert!(!r.testable(), "no unplaced records: nothing to score");
        assert!(!r.discriminates());
        assert!(r.render().contains("not run"));

        r.unplaced = vec![Unplaced {
            id: "x".into(),
            live_score: 0.5,
            shortlist: vec![],
            top_gain: 0.0,
        }];
        assert!(!r.testable(), "no candidates: nothing to rank");

        r.candidates = 2;
        assert!(!r.testable(), "both arms exactly 0: vacuous by arithmetic");

        r.mean_gain = 0.1;
        assert!(r.testable(), "now there is something to compare");
    }

    /// `discriminates` must require BOTH a real comparison and a real margin.
    /// A report that loses to its control must say so in the rendered output,
    /// not only in a boolean nobody reads.
    #[test]
    fn losing_to_the_control_reaches_the_output() {
        let r = ChainReport {
            documents: 3,
            repeats: 1,
            differences: 0,
            contradictions: 0,
            structure_hash: "abcdef123456".into(),
            baseline_tokens: 100,
            context_tokens: 50,
            unplaced: vec![Unplaced {
                id: "z".into(),
                live_score: 0.4,
                shortlist: vec!["A/B".into()],
                top_gain: 0.01,
            }],
            mean_gain: 0.01,
            null: 0.90,
            drifts: vec![],
            candidates: 1,
            embedder: "test".into(),
        };
        assert!(r.testable());
        assert!(!r.discriminates(), "0.01 must not beat 0.90");
        assert!(r.render().contains("NOT ABOVE THE NULL"));
    }

    /// The winning branch must reach the output too. Only the losing one was
    /// covered, so `discriminates() == true` had never been exercised: a verdict
    /// path that no test can distinguish from unreachable.
    #[test]
    fn beating_the_control_reaches_the_output() {
        let r = ChainReport {
            documents: 3,
            repeats: 1,
            differences: 0,
            contradictions: 0,
            structure_hash: "abcdef123456".into(),
            baseline_tokens: 100,
            context_tokens: 50,
            unplaced: vec![Unplaced {
                id: "z".into(),
                live_score: 0.4,
                shortlist: vec!["A/B".into()],
                top_gain: 0.30,
            }],
            mean_gain: 0.30,
            null: 0.05,
            drifts: vec![],
            candidates: 1,
            embedder: "test".into(),
        };
        assert!(r.testable());
        assert!(
            r.discriminates(),
            "0.30 must beat 0.05 by more than the 0.02 margin"
        );
        assert!(r.render().contains("carries signal"));
        assert!(!r.render().contains("NOT ABOVE THE NULL"));

        // And the margin itself must bite: 0.02 is not "more than 0.02".
        let edge = ChainReport {
            mean_gain: 0.07,
            null: 0.05,
            ..r
        };
        assert!(
            !edge.discriminates(),
            "a margin of exactly 0.02 must not pass"
        );
    }

    /// The pass runs end to end and reports the embedder it used — a run that
    /// does not say which embedder produced it is not reproducible.
    #[test]
    fn the_pass_runs_and_names_its_embedder() {
        let e = RandomProjectionEmbedder::new(64);
        let docs = corpus();
        let ontology = crate::ontology::OntologyLoader::load_all();
        let clf = CellClassifier::build(&ontology, &e);
        let r = run(&docs, "pump", &e, &clf, 200, 0.75, "random-projection").unwrap();
        assert_eq!(r.documents, 3);
        assert_eq!(r.embedder, "random-projection");
        assert!(r.context_tokens <= r.baseline_tokens);
    }

    #[test]
    fn an_empty_corpus_is_an_error() {
        let e = RandomProjectionEmbedder::new(64);
        let ontology = crate::ontology::OntologyLoader::load_all();
        let clf = CellClassifier::build(&ontology, &e);
        assert!(run(&[], "q", &e, &clf, 100, 0.75, "test").is_err());
    }
}
