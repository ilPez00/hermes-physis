//! Filing proposal — narrow the grid to a shortlist a person chooses from.
//!
//! ## Why this exists
//!
//! Every mechanism this project built for *certifying* a filing has been
//! refuted. On the only non-synthetic ground truth available — a blind
//! re-filing of 582 entries — every scorer sits at chance: plain cosine 0.519,
//! a structural constraint check 0.536, an ancestry check 0.534. A vocabulary
//! scorer that appeared to win at 0.664 turned out to be measuring entry
//! length. The README's status notice is right about that and stays right.
//!
//! All of those asked the machine to **judge**: *does this entry belong here?*
//! This module asks it to **propose**: *which few cells should a person look at
//! first?* Same geometry, different question, and the second one works.
//!
//! ## What was measured
//!
//! Against the same blind re-filing, scored as top-k containment of the cell a
//! human actually chose, with centroids built only from decisions already made
//! (`research/mapper/E21_PROPOSER.md` in the superproject):
//!
//! | at 436 decisions seen | top-1 | top-3 | top-5 |
//! |---|---|---|---|
//! | **this module**             | 0.447 | **0.712** | 0.791 |
//! | `old_cell`→`new_cell` table | 0.184 | 0.368 | 0.474 |
//! | majority cell               | 0.089 | 0.255 | 0.359 |
//! | **permuted labels (null)**  | 0.036 | 0.136 | 0.233 |
//! | entry length alone          | 0.010 | 0.036 | 0.071 |
//!
//! It beats its construction-matched null by +0.531 top-3 and a lookup table by
//! +0.299, and the curve was still climbing when the data ran out — more
//! confirmed decisions kept paying.
//!
//! **The number to hold onto is that top-1 ≈ 0.45 reproduces an accuracy this
//! project had already recorded as a failure.** The geometry did not improve.
//! Returning three candidates instead of one is the entire difference.
//!
//! ## What this deliberately does not do
//!
//! It does not say an entry *belongs* in the cell it proposes, and a shortlist
//! that omits the right cell is a shortlist, not a verdict. Standing rule for
//! this project: *coverage filters and ranks; it does not verify.* The same
//! applies here.
//!
//! Two further limits worth knowing before trusting it:
//!
//! - **It predicts a filer's policy, not a truth.** The measurements above come
//!   from one rater's decisions. Given a different filer's confirmed decisions
//!   it will propose what *that* filer would do.
//! - **The `mode` axis is much weaker than `domain`** — top-2 0.678 against
//!   0.837 over five domains. Domain is a clean low-rank structure; mode, as
//!   the grid currently defines it, is not. Propose the domain confidently and
//!   expect the person to pick the mode.

use crate::embed::VectorEmbed;

/// Hint weight that beat both alternatives on both axes.
///
/// Entries are embedded as `name + hints`, but hints are subject-matter
/// vocabulary and are **over-weighted** at full strength: they help the domain
/// axis and actively hurt the mode axis. Sweeping the weight found a single
/// optimum for both axes at one half — better than dropping hints entirely
/// (α = 0) *and* better than the full-strength text (α = 1), worth +0.054 top-3
/// over the latter, unanimous across 24 paired splits (*t* = +12.13).
///
/// Above 1.0 the hints swamp the name and accuracy falls sharply.
pub const DEFAULT_HINT_WEIGHT: f32 = 0.5;

fn normalise(mut v: Vec<f32>) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > f32::EPSILON {
        for x in &mut v {
            *x /= n;
        }
    }
    v
}

/// Blend a name-only and a name+hints embedding at `alpha`.
///
/// `alpha = 0` is the name alone, `1.0` the full name+hints text. See
/// [`DEFAULT_HINT_WEIGHT`]. Returns a unit vector. If the two inputs differ in
/// length the shorter one governs, so a mismatched embedder cannot panic here.
pub fn blend_hint_weight(name_vec: &[f32], name_hints_vec: &[f32], alpha: f32) -> Vec<f32> {
    normalise(
        name_vec
            .iter()
            .zip(name_hints_vec.iter())
            .map(|(n, h)| n + alpha * (h - n))
            .collect(),
    )
}

/// Embed one ontology entry at the measured hint weight.
///
/// Costs two embedder calls rather than one. [`CellClassifier::build`] embeds
/// `name + hints` in a single call; this is the same entry at `alpha`.
///
/// [`CellClassifier::build`]: crate::classify::CellClassifier::build
pub fn embed_entry(
    name: &str,
    hints: &[String],
    embedder: &dyn VectorEmbed,
    alpha: f32,
) -> Vec<f32> {
    let mut with_hints = String::from(name);
    for hint in hints {
        with_hints.push(' ');
        with_hints.push_str(hint);
    }
    let name_vec = embedder.embed(name);
    if hints.is_empty() {
        return normalise(name_vec);
    }
    blend_hint_weight(&name_vec, &embedder.embed(&with_hints), alpha)
}

/// One proposed cell, with the score that ranked it.
#[derive(Debug, Clone, PartialEq)]
pub struct CellProposal {
    pub domain: String,
    pub mode: String,
    /// Cosine to the cell's centroid, in `[-1.0, 1.0]`.
    ///
    /// Useful for ordering and for showing a person how close the call was.
    /// It is **not** a probability and not a confidence that the cell is right.
    pub score: f32,
}

impl CellProposal {
    /// `DOMAIN/MODE`, the form the grid is usually written in.
    pub fn cell(&self) -> String {
        format!("{}/{}", self.domain, self.mode)
    }
}

/// Proposes cells from filings a person has already confirmed.
///
/// Built from decisions rather than from the ontology's authored anchors: the
/// point is to learn where *this* filer puts things. See the module docs.
#[derive(Debug, Clone, Default)]
pub struct Proposer {
    cells: Vec<(String, String, Vec<f32>)>,
    decisions: usize,
}

impl Proposer {
    /// Build from confirmed filings: `(domain, mode, embedding)` per entry.
    ///
    /// Embeddings should come from [`embed_entry`] so the proposer and the
    /// entries it scores share a representation. Entries in the same cell are
    /// averaged; empty input yields a proposer that returns nothing.
    pub fn from_decisions<I, D, M>(decisions: I) -> Self
    where
        I: IntoIterator<Item = (D, M, Vec<f32>)>,
        D: Into<String>,
        M: Into<String>,
    {
        let mut cells: Vec<(String, String, Vec<f32>, usize)> = Vec::new();
        let mut seen = 0usize;
        for (domain, mode, vec) in decisions {
            if vec.is_empty() {
                continue;
            }
            seen += 1;
            let (domain, mode) = (domain.into(), mode.into());
            match cells
                .iter_mut()
                .find(|(d, m, _, _)| *d == domain && *m == mode)
            {
                Some((_, _, acc, n)) => {
                    for (a, b) in acc.iter_mut().zip(vec.iter()) {
                        *a += b;
                    }
                    *n += 1;
                }
                None => cells.push((domain, mode, vec, 1)),
            }
        }
        // Deterministic order, so equal scores break the same way every run —
        // the ordering guarantee the rest of this crate makes.
        cells.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        Self {
            cells: cells
                .into_iter()
                .map(|(d, m, acc, n)| (d, m, normalise(acc.iter().map(|x| x / n as f32).collect())))
                .collect(),
            decisions: seen,
        }
    }

    /// The `k` cells worth looking at first, best score first.
    ///
    /// Returns fewer than `k` when fewer cells have been seen. **Read it as a
    /// shortlist for a person, not as an answer** — at k = 3 the right cell was
    /// present about two times in three on the corpus this was measured on.
    pub fn propose(&self, embedding: &[f32], k: usize) -> Vec<CellProposal> {
        if k == 0 || embedding.is_empty() {
            return Vec::new();
        }
        let query = normalise(embedding.to_vec());
        let mut scored: Vec<CellProposal> = self
            .cells
            .iter()
            .map(|(domain, mode, centroid)| CellProposal {
                domain: domain.clone(),
                mode: mode.clone(),
                score: query.iter().zip(centroid.iter()).map(|(a, b)| a * b).sum(),
            })
            .collect();
        // Stable sort over an already-deterministic order: ties keep cell order.
        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(k);
        scored
    }

    /// Cells seen at least once.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Decisions this was built from.
    ///
    /// Worth surfacing: accuracy climbed with this number throughout the range
    /// measured, and had not plateaued at 436.
    pub fn decision_count(&self) -> usize {
        self.decisions
    }

    /// Whether any decision has been recorded.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[f32]) -> Vec<f32> {
        xs.to_vec()
    }

    #[test]
    fn proposes_the_cell_its_decisions_came_from() {
        let p = Proposer::from_decisions(vec![
            ("HEAL", "REST", v(&[1.0, 0.0, 0.0])),
            ("HEAL", "REST", v(&[0.9, 0.1, 0.0])),
            ("STUDY", "LEARN", v(&[0.0, 1.0, 0.0])),
            ("BOND", "CREATE", v(&[0.0, 0.0, 1.0])),
        ]);
        assert_eq!(p.cell_count(), 3);
        assert_eq!(p.decision_count(), 4);
        let got = p.propose(&[1.0, 0.05, 0.0], 1);
        assert_eq!(got[0].cell(), "HEAL/REST");
    }

    #[test]
    fn top_k_widens_the_shortlist_and_is_capped_by_cells_seen() {
        let p = Proposer::from_decisions(vec![
            ("HEAL", "REST", v(&[1.0, 0.0, 0.0])),
            ("STUDY", "LEARN", v(&[0.0, 1.0, 0.0])),
        ]);
        assert_eq!(p.propose(&[0.0, 1.0, 0.0], 1).len(), 1);
        assert_eq!(
            p.propose(&[0.0, 1.0, 0.0], 5).len(),
            2,
            "cannot exceed cells seen"
        );
        // The right cell can be absent from top-1 and present in the shortlist.
        let near_tie = p.propose(&[0.6, 0.8, 0.0], 2);
        assert_eq!(near_tie[0].cell(), "STUDY/LEARN");
        assert_eq!(near_tie[1].cell(), "HEAL/REST");
    }

    #[test]
    fn scores_are_descending_and_ordering_is_deterministic() {
        let p = Proposer::from_decisions(vec![
            ("STUDY", "LEARN", v(&[0.0, 1.0, 0.0])),
            ("HEAL", "REST", v(&[1.0, 0.0, 0.0])),
            ("BOND", "CREATE", v(&[0.0, 0.0, 1.0])),
        ]);
        let a = p.propose(&[0.5, 0.5, 0.5], 3);
        let b = p.propose(&[0.5, 0.5, 0.5], 3);
        assert_eq!(a, b, "same query must give the same order");
        assert!(a.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn empty_and_degenerate_inputs_do_not_panic() {
        let empty = Proposer::from_decisions(Vec::<(String, String, Vec<f32>)>::new());
        assert!(empty.is_empty());
        assert!(empty.propose(&[1.0, 0.0], 3).is_empty());
        let p = Proposer::from_decisions(vec![("HEAL", "REST", v(&[1.0, 0.0]))]);
        assert!(p.propose(&[], 3).is_empty(), "no embedding, no proposal");
        assert!(p.propose(&[1.0, 0.0], 0).is_empty(), "k=0 asks for nothing");
    }

    #[test]
    fn zero_vectors_are_skipped_rather_than_poisoning_a_centroid() {
        let p = Proposer::from_decisions(vec![
            ("HEAL", "REST", Vec::new()),
            ("HEAL", "REST", v(&[1.0, 0.0])),
        ]);
        assert_eq!(p.decision_count(), 1, "the empty vector is not a decision");
        assert_eq!(p.propose(&[1.0, 0.0], 1)[0].cell(), "HEAL/REST");
    }

    #[test]
    fn hint_weight_interpolates_between_name_and_name_plus_hints() {
        let name = v(&[1.0, 0.0]);
        let hints = v(&[0.0, 1.0]);
        let at_zero = blend_hint_weight(&name, &hints, 0.0);
        assert!((at_zero[0] - 1.0).abs() < 1e-6, "alpha=0 is the name alone");
        let at_one = blend_hint_weight(&name, &hints, 1.0);
        assert!((at_one[1] - 1.0).abs() < 1e-6, "alpha=1 is the full text");
        let half = blend_hint_weight(&name, &hints, DEFAULT_HINT_WEIGHT);
        assert!(
            (half[0] - half[1]).abs() < 1e-6,
            "alpha=0.5 sits between them"
        );
        let norm = half.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "result is a unit vector");
    }
}
