//! Cell linkage — which `(domain, mode)` cells does real data bridge, and
//! via which items.
//!
//! ## Why this exists, and why it looks so plain
//!
//! A long research track (see `research/perspective-discovery/` in the
//! superproject) tried seven structurally distinct ways to *discover* stable
//! groupings from embedding geometry — margin/silhouette gating, three
//! kNN-consistency variants, cross-embedder corroboration, density peaks,
//! cross-embedder split agreement, capacity-constrained training loss, and
//! plain k-means. All seven failed on real data. Re-derived clusters do not
//! survive a corpus growing (58% anchor overlap after +25% data), and nothing
//! survives an embedder swap (ARI ~0.10).
//!
//! The one thing that *is* stable is the ontology's own cells: they are
//! hand-authored, so they cannot drift, and they are the same on every run.
//! This module therefore does not discover fixed points. It takes the fixed
//! points as given and measures the **links between them**.
//!
//! ## The rule
//!
//! Each text contributes one bridge, between its top-scoring cell and its
//! second-scoring cell. That is the whole rule. Deliberately:
//!
//! - **no threshold** — an earlier version calibrated a similarity delta and
//!   degenerated, flagging 98% of entries, a rate that restates "domains
//!   overlap" and carries no signal;
//! - **no k**, and **no randomness** — nothing to seed, nothing to tune;
//! - **ties broken on the cell key**, never on iteration order, which is the
//!   bug that made earlier results non-reproducible (see `ontology.rs`).
//!
//! No claim is made about any individual item being "genuinely cross-cutting".
//! The signal is aggregate: a cell pair that keeps appearing as many different
//! items' runner-up is meaningfully linked; a rare pairing is noise.
//!
//! ```no_run
//! use physis_core::classify::CellClassifier;
//! use physis_core::linkage::LinkageGraph;
//! # fn demo(classifier: &CellClassifier, embedder: &dyn physis_core::embed::VectorEmbed) {
//! let graph = LinkageGraph::build(classifier, embedder, ["hybrid battery", "quiet morning walk"]);
//! for link in graph.strongest(10) {
//!     println!("{}/{} <-> {}/{}  ({} bridges)", link.a.0, link.a.1, link.b.0, link.b.1, link.bridge_count);
//! }
//! # }
//! ```

use crate::classify::CellClassifier;
use crate::embed::VectorEmbed;
use std::collections::HashMap;

/// A `(domain, mode)` pair naming one cell.
pub type CellKey = (String, String);

/// One link between two cells, with the items that bridge them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellLink {
    /// The lower cell key of the pair (ordering is canonical, so a link
    /// between X and Y is the same link whichever way it was found).
    pub a: CellKey,
    /// The higher cell key of the pair.
    pub b: CellKey,
    /// How many texts bridged this pair. This is the signal.
    pub bridge_count: usize,
    /// The bridging texts, in input order.
    pub bridges: Vec<String>,
}

impl CellLink {
    /// Whether the two cells sit in different domains. These are the links a
    /// single-label classification cannot represent at all.
    pub fn is_cross_domain(&self) -> bool {
        self.a.0 != self.b.0
    }
}

/// The linkage graph over an ontology's cells.
#[derive(Debug, Clone, Default)]
pub struct LinkageGraph {
    links: Vec<CellLink>,
}

impl LinkageGraph {
    /// Build the graph by classifying each text and bridging its top two cells.
    ///
    /// Texts whose classification yields fewer than two cells contribute
    /// nothing — there is no pair to record, and inventing a self-link would
    /// put mass on the diagonal that no data supports.
    pub fn build<I, S>(classifier: &CellClassifier, embedder: &dyn VectorEmbed, texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut acc: HashMap<(CellKey, CellKey), Vec<String>> = HashMap::new();
        for text in texts {
            let text = text.as_ref();
            let scored = classifier.classify(&embedder.embed(text));
            if scored.len() < 2 {
                continue;
            }
            let first = (scored[0].domain.clone(), scored[0].mode.clone());
            let second = (scored[1].domain.clone(), scored[1].mode.clone());
            if first == second {
                continue;
            }
            // Canonical ordering so the pair is identified by its members, not
            // by which happened to score higher for this particular text.
            let key = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            acc.entry(key).or_default().push(text.to_string());
        }

        let mut links: Vec<CellLink> = acc
            .into_iter()
            .map(|((a, b), bridges)| CellLink {
                a,
                b,
                bridge_count: bridges.len(),
                bridges,
            })
            .collect();
        // Sorted strongest-first, ties broken on the cell keys. The tiebreak is
        // not cosmetic: the source is a HashMap, whose iteration order Rust
        // randomizes per process, and a stable sort would faithfully preserve
        // that randomness among equal counts.
        links.sort_by(|x, y| {
            y.bridge_count
                .cmp(&x.bridge_count)
                .then_with(|| x.a.cmp(&y.a))
                .then_with(|| x.b.cmp(&y.b))
        });
        Self { links }
    }

    /// Every link, strongest first.
    pub fn links(&self) -> &[CellLink] {
        &self.links
    }

    /// The `n` strongest links.
    pub fn strongest(&self, n: usize) -> &[CellLink] {
        &self.links[..n.min(self.links.len())]
    }

    /// Only the links that cross a domain boundary — the structure a
    /// single-label classification cannot express.
    pub fn cross_domain(&self) -> impl Iterator<Item = &CellLink> {
        self.links.iter().filter(|l| l.is_cross_domain())
    }

    /// Number of distinct cell pairs bridged.
    pub fn len(&self) -> usize {
        self.links.len()
    }

    /// Whether no pair was bridged at all.
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{Cell, CellClassifier};
    use crate::models::Facets;

    /// Orthogonal basis vectors, so cell membership is unambiguous and the
    /// test asserts the linkage rule rather than the embedder's judgment.
    fn basis(dim: usize, i: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[i] = 1.0;
        v
    }

    struct FixedEmbed {
        map: Vec<(String, Vec<f32>)>,
    }
    impl VectorEmbed for FixedEmbed {
        fn embed(&self, text: &str) -> Vec<f32> {
            self.map
                .iter()
                .find(|(t, _)| t == text)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| vec![0.0; 4])
        }
        fn dimension(&self) -> usize {
            4
        }
    }

    fn cell(domain: &str, mode: &str, v: Vec<f32>) -> Cell {
        Cell {
            domain: domain.to_string(),
            mode: mode.to_string(),
            entries: vec!["seed".to_string()],
            embeddings: vec![v],
            facets: vec![Facets::default()],
        }
    }

    fn fixture() -> CellClassifier {
        CellClassifier::from_cells(vec![
            cell("HEAL", "REST", basis(4, 0)),
            cell("BOND", "REST", basis(4, 1)),
            cell("STUDY", "WORK", basis(4, 2)),
        ])
    }

    #[test]
    fn bridges_the_top_two_cells_and_counts_them() {
        let clf = fixture();
        // Leans on cell 0, next-closest cell 1 — so every one of these texts
        // should bridge exactly the HEAL/REST <-> BOND/REST pair.
        let v = vec![0.9, 0.4, 0.0, 0.0];
        let embedder = FixedEmbed {
            map: vec![("a".into(), v.clone()), ("b".into(), v.clone())],
        };
        let g = LinkageGraph::build(&clf, &embedder, ["a", "b"]);
        assert_eq!(g.len(), 1, "both texts bridge the same pair");
        let link = &g.links()[0];
        assert_eq!(link.bridge_count, 2);
        assert_eq!(link.bridges, vec!["a".to_string(), "b".to_string()]);
        assert!(
            link.is_cross_domain(),
            "HEAL and BOND are different domains"
        );
    }

    #[test]
    fn link_identity_is_canonical_regardless_of_which_cell_scored_higher() {
        let clf = fixture();
        // One text leans to cell 0 then 1; the other leans to cell 1 then 0.
        let embedder = FixedEmbed {
            map: vec![
                ("first".into(), vec![0.9, 0.4, 0.0, 0.0]),
                ("second".into(), vec![0.4, 0.9, 0.0, 0.0]),
            ],
        };
        let g = LinkageGraph::build(&clf, &embedder, ["first", "second"]);
        assert_eq!(
            g.len(),
            1,
            "the same unordered pair must not be recorded as two different links"
        );
        assert_eq!(g.links()[0].bridge_count, 2);
    }

    #[test]
    fn output_is_identical_across_repeated_builds() {
        // Guards the bug this module documents: HashMap iteration order is
        // randomized per process, so an untied sort produces a different graph
        // on every run from identical input.
        let clf = fixture();
        let embedder = FixedEmbed {
            map: vec![
                ("x".into(), vec![0.9, 0.4, 0.0, 0.0]),
                ("y".into(), vec![0.0, 0.9, 0.4, 0.0]),
                ("z".into(), vec![0.4, 0.0, 0.9, 0.0]),
            ],
        };
        let first = LinkageGraph::build(&clf, &embedder, ["x", "y", "z"]);
        for _ in 0..8 {
            let again = LinkageGraph::build(&clf, &embedder, ["x", "y", "z"]);
            assert_eq!(first.links(), again.links(), "linkage must be reproducible");
        }
    }

    #[test]
    fn cross_domain_filter_excludes_same_domain_links() {
        let clf = CellClassifier::from_cells(vec![
            cell("HEAL", "REST", basis(4, 0)),
            cell("HEAL", "WORK", basis(4, 1)),
        ]);
        let embedder = FixedEmbed {
            map: vec![("t".into(), vec![0.9, 0.4, 0.0, 0.0])],
        };
        let g = LinkageGraph::build(&clf, &embedder, ["t"]);
        assert_eq!(g.len(), 1);
        assert!(!g.links()[0].is_cross_domain());
        assert_eq!(g.cross_domain().count(), 0);
    }

    #[test]
    fn empty_input_yields_an_empty_graph() {
        let clf = fixture();
        let embedder = FixedEmbed { map: vec![] };
        let g = LinkageGraph::build(&clf, &embedder, Vec::<&str>::new());
        assert!(g.is_empty());
        assert_eq!(
            g.strongest(5).len(),
            0,
            "strongest must not panic on an empty graph"
        );
    }
}
