//! Coverage impact — which candidate cells would let the ontology place
//! records it currently cannot.
//!
//! ## Why this exists
//!
//! [`crate::discovery`] proposes new ontology entries but has no way to say
//! which proposals are worth keeping. This is that missing half, and it is
//! deliberately the *narrow* half: it does not judge whether a candidate is a
//! good concept, only whether adding it changes what the ontology can place.
//!
//! ## Why the check is shaped this way
//!
//! A long research track (`research/perspective-discovery/` in the
//! superproject) tried nine mechanisms for certifying discovered structure and
//! all nine failed. Every one asked a REPRESENTATIONAL question — is this
//! split correct, does a lexicon recognise it, do many pairs agree on it — and
//! answered it with a statistic computed over the same embedding space that
//! produced the candidate.
//!
//! This asks an OPERATIONAL question instead: does adding the candidate make
//! records classifiable that were not? The corpus of records is external to
//! whatever produced the candidate, so the check cannot be satisfied by the
//! geometry agreeing with itself.
//!
//! Measured over 5-fold cross-validation on 691 held-out records: re-adding an
//! entry the ontology already contains scores **exactly zero** on 2000 out of
//! 2000 trials — correctly, since records near it were already covered — while
//! genuine interpolations score above zero (Welch t = +12.11). The check is not
//! fooled by a candidate that adds nothing, and it needs neither a human nor a
//! lexicon to say so. In that run it reduced 2000 candidates to 141 worth
//! reading.
//!
//! ## Two measures, and why both are here
//!
//! [`CandidateGain`] reports the same event two ways, because they were
//! measured separately and they do different jobs.
//!
//! **`count()` filters.** How many records cross the confidence threshold. This
//! is the measure the cross-validation above validated, and it is the one to
//! filter on: a candidate that rescues nothing provably changes nothing.
//!
//! **`lift` ranks.** Total confidence added across every record, threshold-free:
//! `sum over records of max(0, cos(candidate, record) - live(record))`. A later
//! experiment (1,638 proposals, 6,234 operational records, a 10% hold-out of
//! real ontology entries as ground truth) found ordering by lift puts a genuine
//! missing concept in 12 of the first 25 rows — precision 0.480 against a 0.132
//! base rate, exact p = 2.3e-5 — where ordering by the count reaches 0.320.
//! [`rank_candidates`] therefore sorts on lift.
//!
//! The gap is not a tuning detail. **The threshold rule goes nearly inert on
//! operational text.** Records written by a running operation already match the
//! live ontology strongly (median best-entry cosine 0.741 on the corpus
//! measured), and a candidate interpolated between two existing entries can
//! rarely beat *every* known entry for *any* record: at that median threshold
//! only 272 of 1,638 candidates rescued even one record. The 5-fold run above
//! looked healthier because its records were held-out ontology *entries*, which
//! sit further out (median ~0.5) and leave the bar reachable. Lift has no bar
//! and degrades smoothly instead.
//!
//! Both measures kill a duplicate the same way: the live score is already a
//! maximum over all entries, so a copy of one cannot exceed it, and its lift is
//! exactly 0.0 by construction rather than by luck.
//!
//! ## What it does NOT tell you
//!
//! **Coverage is not correctness.** A candidate that swallows records into a
//! wrong cell scores exactly like one that captures a real gap — both make the
//! records classifiable. Deciding which is which needs labels, and is the
//! question those nine mechanisms failed to answer. Use this to *shrink the
//! pile*, then have a person read what survives; do not use it to auto-accept.
//!
//! Ranking does not soften that. A well-ordered worklist is still a worklist:
//! the ordering was validated against *missing* concepts (a hold-out), never
//! against *wrong* ones. The companion measurement is blunt about the other
//! side — asked to score entries against the cell they were actually filed in,
//! the same cosine machinery detects real misfilings at **AUC 0.410**, with
//! chance inside the confidence interval. It finds gaps; it cannot find
//! mistakes.
//!
//! Two other limits worth knowing before trusting a ranking:
//!
//! - **It ranks gaps the records talk about.** The hold-out concepts used to
//!   validate it carry heavy corpus traffic (median max-record cosine 0.793).
//!   A concept the ontology is missing *and* the operation never writes about
//!   produces no lift and cannot surface here, by construction.
//! - **Not every candidate is rankable.** In the measured run 1,236 of 1,638
//!   proposals (75%) lifted any record at all; the rest tie at zero and fall
//!   back to candidate order.
//!
//! ```no_run
//! use physis_core::coverage::rank_candidates;
//! # fn demo(classifier: &physis_core::classify::CellClassifier,
//! #         candidates: &[physis_core::classify::Cell],
//! #         records: &[(String, Vec<f32>)]) {
//! // Strongest-first by continuous lift; read the head of the list.
//! for (idx, gain) in rank_candidates(classifier, candidates, records, 0.85) {
//!     println!(
//!         "candidate {idx}: lift {:.3}, would newly cover {} records",
//!         gain.lift,
//!         gain.count()
//!     );
//! }
//! # }
//! ```

use crate::classify::{Cell, CellClassifier};
use crate::models::cosine_sim;

/// What one candidate cell would add.
///
/// Two measures of the same event, and they are not interchangeable — see the
/// module docs. [`newly_covered`](Self::newly_covered) is the threshold-crossing
/// count and is what makes this a *filter*; [`lift`](Self::lift) is the
/// continuous form and is what makes it a *ranking*.
///
/// Not `Eq`: `lift` is a float. Compare with `==` (derived `PartialEq`) or on
/// the field you actually mean.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CandidateGain {
    /// Ids of records that are not confidently placed today and would be
    /// placed by this candidate, in the order the records were supplied.
    pub newly_covered: Vec<String>,

    /// Total confidence this candidate would add across **every** record:
    /// `sum over records of max(0, cos(candidate, record) - live(record))`.
    ///
    /// Unlike [`newly_covered`](Self::newly_covered) this does not depend on
    /// the threshold, so it stays informative on data where the threshold is
    /// out of reach — which is the normal case on operational text, where
    /// records already match the live ontology strongly (measured median
    /// best-entry cosine 0.741) and few candidates can push one over a bar.
    ///
    /// A candidate that duplicates an existing entry scores **exactly 0.0**,
    /// by the same argument that makes the count-based filter sound: the live
    /// score is already a maximum over all entries, so a copy of one of them
    /// can never exceed it. The duplicate control holds for both measures.
    pub lift: f32,
}

impl CandidateGain {
    /// How many records this candidate would rescue.
    pub fn count(&self) -> usize {
        self.newly_covered.len()
    }

    /// Whether the candidate changes anything at all. A candidate that fails
    /// this is provably not worth a reader's attention: it leaves every record
    /// exactly where it was.
    ///
    /// This is the *filter* half — the one cross-validated at Welch t = +12.11
    /// against re-adding an existing entry. Use it to shrink the pile, then use
    /// [`rank_candidates`] to order what survives.
    pub fn is_useful(&self) -> bool {
        !self.newly_covered.is_empty()
    }
}

/// Best raw single-entry cosine for each record under the live ontology.
///
/// Raw best-entry similarity rather than the blended classifier score, matching
/// [`crate::discovery`]: the blend's population weighting is a ranking aid, and
/// mixing it into a coverage threshold would make the threshold mean different
/// things in dense and sparse cells.
fn live_scores(classifier: &CellClassifier, records: &[(String, Vec<f32>)]) -> Vec<f32> {
    records
        .iter()
        .map(|(_, e)| {
            classifier
                .best_entry_sim(e)
                .map(|(s, _, _)| s)
                .unwrap_or(f32::NEG_INFINITY)
        })
        .collect()
}

/// Highest cosine from `embedding` to any member entry of `cell`.
fn cell_score(cell: &Cell, embedding: &[f32]) -> f32 {
    cell.embeddings
        .iter()
        .map(|e| cosine_sim(embedding, e))
        .fold(f32::NEG_INFINITY, f32::max)
}

/// Ids of records the live ontology cannot place at or above `threshold`.
///
/// This is the pile a candidate has to make a dent in. If it is empty, every
/// candidate will score zero and the ranking carries no information — pick a
/// threshold against the observed score distribution rather than an absolute
/// guess.
pub fn uncovered(
    classifier: &CellClassifier,
    records: &[(String, Vec<f32>)],
    threshold: f32,
) -> Vec<String> {
    let live = live_scores(classifier, records);
    records
        .iter()
        .zip(&live)
        .filter(|(_, &s)| s < threshold)
        .map(|((id, _), _)| id.clone())
        .collect()
}

/// What a single candidate cell would newly cover.
pub fn candidate_gain(
    classifier: &CellClassifier,
    candidate: &Cell,
    records: &[(String, Vec<f32>)],
    threshold: f32,
) -> CandidateGain {
    let live = live_scores(classifier, records);
    gain_against(&live, candidate, records, threshold)
}

/// Score every candidate and return them strongest-first.
///
/// **Ordered by [`CandidateGain::lift`], not by the rescue count.** Measured on
/// 1,638 proposals against 6,234 operational records, with a 10% hold-out of
/// real ontology entries as ground truth: ranking by lift puts a real missing
/// concept in **12 of the first 25** rows (precision 0.480 against a 0.132 base
/// rate, exact hypergeometric p = 2.3e-5), while ranking by the threshold count
/// reaches 0.320. Four controls — proposal popularity, the proposer's own
/// geometry, parent similarity, and how empty the region of the ontology is —
/// all sit on the random floor at the head of the list, so the ordering
/// information is coming from the records and not from the embedding geometry
/// that produced the candidates.
///
/// Read the head, not the tail. Precision decays 0.480 -> 0.360 -> 0.193 across
/// k = 25 -> 100 -> 400 and is inside the base rate by 400. This orders a
/// worklist; it does not sort a pile into good and bad.
///
/// The live scores are computed once and shared, so this is far cheaper than
/// calling [`candidate_gain`] in a loop. Ties break on candidate index, never
/// on iteration order, so the ranking is reproducible.
pub fn rank_candidates(
    classifier: &CellClassifier,
    candidates: &[Cell],
    records: &[(String, Vec<f32>)],
    threshold: f32,
) -> Vec<(usize, CandidateGain)> {
    let live = live_scores(classifier, records);
    let mut scored: Vec<(usize, CandidateGain)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (i, gain_against(&live, c, records, threshold)))
        .collect();
    scored.sort_by(|a, b| b.1.lift.total_cmp(&a.1.lift).then_with(|| a.0.cmp(&b.0)));
    scored
}

fn gain_against(
    live: &[f32],
    candidate: &Cell,
    records: &[(String, Vec<f32>)],
    threshold: f32,
) -> CandidateGain {
    // Adding a cell can only raise a record's best score, so a record can never
    // move from covered to uncovered. "Newly covered" is therefore exactly
    // "was below the threshold, and this candidate is at or above it".
    let mut newly_covered = Vec::new();
    let mut lift = 0.0f32;
    for ((id, e), &s) in records.iter().zip(live) {
        let cand = cell_score(candidate, e);
        if s < threshold && cand >= threshold {
            newly_covered.push(id.clone());
        }
        // An ontology with no entries scores every record at -inf; -1.0 is the
        // floor of cosine and keeps the lift finite in that degenerate case.
        let base = if s.is_finite() { s } else { -1.0 };
        if cand > base {
            lift += cand - base;
        }
    }
    CandidateGain {
        newly_covered,
        lift,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Facets;

    fn basis(dim: usize, i: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[i] = 1.0;
        v
    }

    fn cell(domain: &str, mode: &str, embeddings: Vec<Vec<f32>>) -> Cell {
        Cell {
            domain: domain.to_string(),
            mode: mode.to_string(),
            entries: embeddings.iter().map(|_| "e".to_string()).collect(),
            facets: embeddings.iter().map(|_| Facets::default()).collect(),
            embeddings,
        }
    }

    fn fixture() -> CellClassifier {
        CellClassifier::from_cells(vec![
            cell("HEAL", "REST", vec![basis(4, 0)]),
            cell("BOND", "REST", vec![basis(4, 1)]),
        ])
    }

    #[test]
    fn uncovered_lists_only_records_below_threshold() {
        let clf = fixture();
        let records = vec![
            ("on_top".to_string(), basis(4, 0)), // cosine 1.0 to a known entry
            ("far_away".to_string(), basis(4, 3)), // orthogonal to everything
        ];
        let u = uncovered(&clf, &records, 0.5);
        assert_eq!(u, vec!["far_away".to_string()]);
    }

    #[test]
    fn a_candidate_covering_the_gap_scores_it() {
        let clf = fixture();
        let records = vec![("far_away".to_string(), basis(4, 3))];
        let candidate = cell("NEW", "CELL", vec![basis(4, 3)]);
        let g = candidate_gain(&clf, &candidate, &records, 0.5);
        assert_eq!(g.count(), 1);
        assert!(g.is_useful());
        assert_eq!(g.newly_covered, vec!["far_away".to_string()]);
    }

    /// The behaviour the cross-validation actually established: re-adding
    /// something the ontology already has rescues nothing, because the records
    /// near it were already covered. This is what makes the check a usable
    /// filter rather than a rubber stamp — it scored 0 on 2000/2000 trials.
    #[test]
    fn duplicating_an_existing_entry_gains_nothing() {
        let clf = fixture();
        let records = vec![
            ("on_top".to_string(), basis(4, 0)),
            ("far_away".to_string(), basis(4, 3)),
        ];
        let duplicate = cell("HEAL", "REST", vec![basis(4, 0)]);
        let g = candidate_gain(&clf, &duplicate, &records, 0.5);
        assert_eq!(
            g.count(),
            0,
            "a duplicate must not be credited with coverage"
        );
        assert!(!g.is_useful());
    }

    #[test]
    fn ranking_is_strongest_first_and_reproducible() {
        let clf = fixture();
        let records = vec![
            ("a".to_string(), basis(4, 2)),
            ("b".to_string(), basis(4, 3)),
        ];
        let candidates = vec![
            cell("X", "1", vec![basis(4, 2)]),              // covers one
            cell("Y", "2", vec![basis(4, 2), basis(4, 3)]), // covers both
            cell("Z", "3", vec![basis(4, 0)]),              // covers none
        ];
        let ranked = rank_candidates(&clf, &candidates, &records, 0.5);
        assert_eq!(
            ranked[0].0, 1,
            "the candidate covering both must rank first"
        );
        assert_eq!(ranked[0].1.count(), 2);
        assert_eq!(ranked[2].1.count(), 0);
        for _ in 0..8 {
            assert_eq!(rank_candidates(&clf, &candidates, &records, 0.5), ranked);
        }
    }

    /// The property that makes `lift` safe to rank on: the same duplicate that
    /// scores zero rescues also scores exactly 0.0 lift, and for the same
    /// reason — `live` is already a maximum over every entry, so a copy of one
    /// cannot exceed it anywhere. Exact, not approximate.
    #[test]
    fn duplicating_an_existing_entry_lifts_exactly_zero() {
        let clf = fixture();
        let records = vec![
            ("on_top".to_string(), basis(4, 0)),
            ("between".to_string(), vec![0.6, 0.8, 0.0, 0.0]),
            ("far_away".to_string(), basis(4, 3)),
        ];
        let duplicate = cell("HEAL", "REST", vec![basis(4, 0)]);
        let g = candidate_gain(&clf, &duplicate, &records, 0.5);
        assert_eq!(g.lift, 0.0, "a duplicate must not be credited with lift");
        assert_eq!(g.count(), 0);
    }

    /// Why the ranking changed. This is the operational-text shape in
    /// miniature: every record already clears the threshold, so no candidate
    /// can rescue one and the count-based order is a flat tie of zeroes —
    /// while lift still separates candidates by how much confidence they
    /// actually add, and prefers the one that helps two records over the one
    /// that helps a single record more.
    #[test]
    fn lift_ranks_where_the_threshold_rule_is_inert() {
        let clf = CellClassifier::from_cells(vec![
            cell("HEAL", "REST", vec![basis(5, 0)]),
            cell("BOND", "REST", vec![basis(5, 1)]),
        ]);
        // Each record sits at cosine ~0.3 from HEAL/REST and is otherwise in a
        // dimension of its own.
        let records = vec![
            ("r0".to_string(), vec![0.3, 0.0, 0.954, 0.0, 0.0]),
            ("r1".to_string(), vec![0.3, 0.0, 0.0, 0.954, 0.0]),
            ("r2".to_string(), vec![0.3, 0.0, 0.0, 0.0, 0.954]),
        ];
        let threshold = 0.2; // below every live score: nothing is uncovered
        assert!(
            uncovered(&clf, &records, threshold).is_empty(),
            "setup: nothing is uncovered"
        );

        let candidates = vec![
            cell("NONE", "AT-ALL", vec![basis(5, 1)]), // duplicates a known entry
            cell("ONE", "RECORD", vec![basis(5, 2)]),  // lands squarely on r0
            cell("TWO", "RECORDS", vec![vec![0.0, 0.0, 0.707, 0.707, 0.0]]),
        ];
        let ranked = rank_candidates(&clf, &candidates, &records, threshold);

        for (i, g) in &ranked {
            assert_eq!(
                g.count(),
                0,
                "candidate {i}: the threshold rule must be inert here"
            );
        }
        assert_eq!(
            ranked[0].0, 2,
            "helping two records beats helping one record more"
        );
        assert_eq!(ranked[1].0, 1);
        assert_eq!(ranked[2].0, 0);
        assert_eq!(
            ranked[2].1.lift, 0.0,
            "a duplicate of a known entry lifts nothing"
        );
        assert!(ranked[0].1.lift > ranked[1].1.lift);
        for _ in 0..8 {
            assert_eq!(
                rank_candidates(&clf, &candidates, &records, threshold),
                ranked
            );
        }
    }

    /// An empty ontology scores every record at -inf; the lift must stay finite
    /// so the ordering is still a total order rather than a pile of NaNs.
    #[test]
    fn an_empty_ontology_yields_finite_lift() {
        let clf = CellClassifier::from_cells(vec![]);
        let records = vec![("r".to_string(), basis(4, 0))];
        let candidate = cell("NEW", "CELL", vec![basis(4, 0)]);
        let g = candidate_gain(&clf, &candidate, &records, 0.5);
        assert!(
            g.lift.is_finite(),
            "lift must not be infinite against an empty ontology"
        );
        assert!(
            (g.lift - 2.0).abs() < 1e-6,
            "cosine 1.0 above the -1.0 floor"
        );
    }

    #[test]
    fn nothing_uncovered_means_no_candidate_can_gain() {
        // Guards the documented degenerate case: with a threshold every record
        // already clears, the ranking is uniformly zero and carries no signal.
        let clf = fixture();
        let records = vec![("on_top".to_string(), basis(4, 0))];
        assert!(uncovered(&clf, &records, -1.0).is_empty());
        let candidate = cell("NEW", "CELL", vec![basis(4, 0)]);
        assert_eq!(candidate_gain(&clf, &candidate, &records, -1.0).count(), 0);
    }
}
