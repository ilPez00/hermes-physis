//! Embedding–structure alignment (a Γ-like, auditable diagnostic).
//!
//! A CQFT-flavored intuition — that a system's representational geometry can
//! disagree with its relational substrate — translated into terms this engine
//! can actually measure: does a node's **embedding neighbourhood** agree with
//! its **declared/derived structural neighbourhood**?
//!
//! This is a tension/diagnostic signal, not a verdict and not a truth metric.
//! It reports *where* and *how much* embedding similarity and graph structure
//! pull apart, so downstream reasoning (coverage, explanation, revision
//! analysis) can treat the disagreement as inspectable evidence rather than as
//! an invisible assumption.
//!
//! ## What counts as "structure"
//!
//! The structural neighbourhood is built from sources the caller selects via
//! [`StructureSources`]. The default stays closest to revision semantics:
//! declared typed relations (all [`crate::relation::TypedEdge`]s). Callers may
//! also include `DependsOn`-only relations, shared `cell_pin` links, and
//! label-prefix links — the same derived links [`crate::delta_engine`] uses
//! for adjacency — but each choice is explicit and recorded in the report.
//!
//! ## The null is not optional
//!
//! Raw alignment is only meaningful relative to what the same geometry would
//! produce by chance. [`AlignmentReport::null_gap`] is computed over a
//! **degree-preserving label permutation** of the structural neighbourhoods:
//! each node keeps its structural degree, but the identities of its structural
//! neighbours are reassigned by a deterministic shuffle. Embeddings, the kNN
//! geometry, and degree survive; only the pairing between embedding space and
//! structure is destroyed. [`AlignmentReport::discriminates`] reports whether
//! the observed gap is lower than that null (more aligned than chance).
//!
//! A report that does not beat its null is reporting geometry, not
//! structure–embedding agreement, and must be read that way.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::models::{cosine_sim, CoherenceNode, Score};
use crate::relation::{RelationType, TypedEdge};

/// Which structural sources define a node's structural neighbourhood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureSources {
    /// Include all declared typed edges (both directions).
    pub typed_edges: bool,
    /// Restrict typed edges to `DependsOn` only (overrides `typed_edges` when
    /// `typed_edges` is false and this is true; combined as a union otherwise).
    pub depends_on_only: bool,
    /// Include nodes sharing the same `cell_pin`.
    pub shared_cell_pin: bool,
    /// Include label-prefix links ("parent → child").
    pub label_links: bool,
}

impl StructureSources {
    /// Declared relations only — the default, closest to revision semantics.
    pub fn declared_relations() -> Self {
        Self {
            typed_edges: true,
            depends_on_only: false,
            shared_cell_pin: false,
            label_links: false,
        }
    }

    /// DependsOn edges only.
    pub fn depends_on() -> Self {
        Self {
            typed_edges: false,
            depends_on_only: true,
            shared_cell_pin: false,
            label_links: false,
        }
    }

    /// The full adjacency used by delta-engine breadth propagation.
    pub fn adjacency_like() -> Self {
        Self {
            typed_edges: true,
            depends_on_only: false,
            shared_cell_pin: true,
            label_links: true,
        }
    }

    fn uses_typed(&self) -> bool {
        self.typed_edges || self.depends_on_only
    }
}

/// Configuration for alignment analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// Embedding neighbourhood size (k nearest by cosine).
    pub k: usize,
    /// Structural sources to consider.
    pub sources: StructureSources,
    /// Deterministic seed for the permuted-null construction.
    pub null_seed: u64,
    /// Ignore nodes whose embedding is empty.
    pub skip_empty_embeddings: bool,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            k: 5,
            sources: StructureSources::declared_relations(),
            null_seed: 0xA11_6EED,
            skip_empty_embeddings: true,
        }
    }
}

/// Per-node alignment between embedding kNN and structural neighbours.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAlignment {
    pub node_id: String,
    /// |kNN ∩ structural| / |kNN ∪ structural| (1.0 when both empty or equal).
    pub jaccard: Score,
    /// |kNN ∩ structural| / k (fraction of embedding neighbours explained by
    /// structure). 1.0 when k == 0.
    pub embedding_coverage_by_structure: Score,
    /// |kNN ∩ structural| / |structural| (fraction of structural neighbours
    /// present in embedding kNN). 1.0 when there are no structural neighbours.
    pub structure_coverage_by_embedding: Score,
    /// 1 - jaccard: higher means embedding geometry and structure disagree.
    pub gap: Score,
    pub embedding_knn: Vec<String>,
    pub structural_neighbours: Vec<String>,
}

/// Aggregate alignment report with a construction-matched null.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentReport {
    pub config: AlignmentConfig,
    pub nodes: Vec<NodeAlignment>,
    /// Mean per-node gap (observed).
    pub mean_gap: Score,
    /// Mean per-node gap under the permuted-structure null.
    pub null_gap: Score,
    /// mean_gap - null_gap (negative => more aligned than chance).
    pub delta_vs_null: Score,
    /// Number of nodes analysed.
    pub analysed: usize,
    /// Number of nodes skipped (e.g. empty embeddings).
    pub skipped: usize,
    /// Human-readable note when the null cannot discriminate (e.g. no
    /// structure at all), so a non-result is not misread as agreement.
    pub caveat: Option<String>,
}

impl AlignmentReport {
    /// True when the observed structure–embedding agreement beats the
    /// permuted-structure null (lower gap than chance). False when the null is
    /// inapplicable or when geometry, not structure, explains the alignment.
    pub fn discriminates(&self) -> bool {
        self.caveat.is_none() && self.mean_gap + 1e-6 < self.null_gap
    }
}

/// Build structural neighbourhoods for all nodes under `sources`.
pub fn structural_neighbours(
    nodes: &[CoherenceNode],
    edges: &[TypedEdge],
    sources: &StructureSources,
) -> BTreeMap<String, Vec<String>> {
    let mut by_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in nodes {
        by_id.entry(n.id.clone()).or_default();
    }

    if sources.uses_typed() {
        for e in edges {
            if sources.depends_on_only && e.relation_type != RelationType::DependsOn {
                continue;
            }
            // Only connect ids we know about; keep deterministic order later.
            if by_id.contains_key(&e.source_id) && by_id.contains_key(&e.target_id) {
                by_id
                    .entry(e.source_id.clone())
                    .or_default()
                    .push(e.target_id.clone());
                by_id
                    .entry(e.target_id.clone())
                    .or_default()
                    .push(e.source_id.clone());
            }
        }
    }

    if sources.shared_cell_pin {
        // Group by cell_pin.
        let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        for n in nodes {
            if let Some(ref pin) = n.cell_pin {
                groups.entry(pin.clone()).or_default().push(n.id.clone());
            }
        }
        for ids in groups.values() {
            for i in 0..ids.len() {
                for j in 0..ids.len() {
                    if i != j {
                        by_id
                            .entry(ids[i].clone())
                            .or_default()
                            .push(ids[j].clone());
                    }
                }
            }
        }
    }

    if sources.label_links {
        // A child is any node whose label starts with "parent → ".
        for parent in nodes {
            let Some(ref plabel) = parent.label else {
                continue;
            };
            let prefix = format!("{plabel} → ");
            for child in nodes {
                if child.id == parent.id {
                    continue;
                }
                if child
                    .label
                    .as_deref()
                    .is_some_and(|l| l.starts_with(&prefix))
                {
                    by_id
                        .entry(parent.id.clone())
                        .or_default()
                        .push(child.id.clone());
                    by_id
                        .entry(child.id.clone())
                        .or_default()
                        .push(parent.id.clone());
                }
            }
        }
    }

    // Dedup and sort for determinism.
    for neigh in by_id.values_mut() {
        neigh.sort();
        neigh.dedup();
    }
    by_id
}

/// k nearest neighbours by cosine similarity (descending), excluding self and
/// empty embeddings. Ties broken by id for determinism.
pub fn embedding_knn(node: &CoherenceNode, nodes: &[CoherenceNode], k: usize) -> Vec<String> {
    if k == 0 || node.embedding.is_empty() {
        return Vec::new();
    }
    let mut sims: Vec<(&str, f32)> = nodes
        .iter()
        .filter(|n| n.id != node.id && !n.embedding.is_empty())
        .map(|n| (n.id.as_str(), cosine_sim(&node.embedding, &n.embedding)))
        .collect();
    sims.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    sims.truncate(k);
    sims.into_iter().map(|(id, _)| id.to_string()).collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> Score {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as Score;
    let union = a.union(b).count() as Score;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

fn align_node(knn: &[String], structural: &[String]) -> (Score, Score, Score, Score) {
    let a: HashSet<String> = knn.iter().cloned().collect();
    let b: HashSet<String> = structural.iter().cloned().collect();
    let inter = a.intersection(&b).count() as Score;
    let jac = jaccard(&a, &b);
    let cov_e = if knn.is_empty() {
        1.0
    } else {
        inter / knn.len() as Score
    };
    let cov_s = if structural.is_empty() {
        1.0
    } else {
        inter / structural.len() as Score
    };
    let gap = 1.0 - jac;
    (jac, cov_e, cov_s, gap)
}

/// Deterministic permutation of 0..n using xorshift64 seeded by `seed`.
fn permuted_indices(n: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..n).collect();
    let mut s = seed | 1;
    for i in (1..n).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        idx.swap(i, (s as usize) % (i + 1));
    }
    idx
}

/// Degree-preserving structural null: keep each node's structural neighbour
/// *count* but replace the neighbour identities by a deterministic shuffle of
/// the node id list (excluding self). Embeddings and kNN geometry are untouched.
fn null_structural(
    nodes: &[CoherenceNode],
    observed: &BTreeMap<String, Vec<String>>,
    seed: u64,
) -> BTreeMap<String, Vec<String>> {
    let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let n = ids.len();
    let mut out = BTreeMap::new();
    for (i, id) in ids.iter().enumerate() {
        let deg = observed
            .get(id)
            .map(|v| v.len())
            .unwrap_or(0)
            .min(n.saturating_sub(1));
        if deg == 0 {
            out.insert(id.clone(), Vec::new());
            continue;
        }
        // Build a pool excluding self, permuted deterministically per node.
        let mut pool: Vec<String> = ids
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, v)| v.clone())
            .collect();
        // Mix seed with node index for per-node variety while staying deterministic.
        let local_seed = seed ^ ((i as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let perm = permuted_indices(pool.len(), local_seed);
        pool = perm.into_iter().map(|p| pool[p].clone()).collect();
        pool.truncate(deg);
        pool.sort();
        out.insert(id.clone(), pool);
    }
    out
}

/// Compute embedding–structure alignment over `nodes` with the given edges.
pub fn analyse(
    nodes: &[CoherenceNode],
    edges: &[TypedEdge],
    config: &AlignmentConfig,
) -> AlignmentReport {
    let structural = structural_neighbours(nodes, edges, &config.sources);

    // Precompute kNN per node (deterministic).
    let mut knn_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in nodes {
        knn_map.insert(n.id.clone(), embedding_knn(n, nodes, config.k));
    }

    let mut rows: Vec<NodeAlignment> = Vec::new();
    let mut skipped = 0usize;

    for n in nodes {
        if config.skip_empty_embeddings && n.embedding.is_empty() {
            skipped += 1;
            continue;
        }
        let knn = knn_map.get(&n.id).cloned().unwrap_or_default();
        let st = structural.get(&n.id).cloned().unwrap_or_default();
        let (jac, cov_e, cov_s, gap) = align_node(&knn, &st);
        rows.push(NodeAlignment {
            node_id: n.id.clone(),
            jaccard: jac,
            embedding_coverage_by_structure: cov_e,
            structure_coverage_by_embedding: cov_s,
            gap,
            embedding_knn: knn,
            structural_neighbours: st,
        });
    }

    let mean_gap = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(|r| r.gap).sum::<Score>() / rows.len() as Score
    };

    // Null: permute structural neighbourhood identities, keep degrees and kNN.
    let null_st = null_structural(nodes, &structural, config.null_seed);
    let mut null_gaps: Vec<Score> = Vec::new();
    for n in nodes {
        if config.skip_empty_embeddings && n.embedding.is_empty() {
            continue;
        }
        let knn = knn_map.get(&n.id).cloned().unwrap_or_default();
        let st = null_st.get(&n.id).cloned().unwrap_or_default();
        let (_, _, _, gap) = align_node(&knn, &st);
        null_gaps.push(gap);
    }
    let null_gap = if null_gaps.is_empty() {
        mean_gap
    } else {
        null_gaps.iter().sum::<Score>() / null_gaps.len() as Score
    };

    // Caveat when structure carries no information (all degrees zero) or too few
    // nodes to say anything — the null and observed coincide by construction.
    let any_structure = rows.iter().any(|r| !r.structural_neighbours.is_empty());
    let caveat = if rows.len() < 2 {
        Some("fewer than two analysable nodes; alignment is undefined".to_string())
    } else if !any_structure {
        Some(
            "no structural neighbours under the selected sources; null is inapplicable".to_string(),
        )
    } else {
        None
    };

    let analysed = rows.len();
    AlignmentReport {
        config: config.clone(),
        nodes: rows,
        mean_gap,
        null_gap,
        delta_vs_null: mean_gap - null_gap,
        analysed,
        skipped,
        caveat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CoherenceNode;

    fn node(id: &str, emb: Vec<f32>, pin: Option<(&str, &str)>) -> CoherenceNode {
        let mut n = CoherenceNode::new(emb);
        n.id = id.to_string();
        n.cell_pin = pin.map(|(d, m)| (d.to_string(), m.to_string()));
        n
    }

    #[test]
    fn aligned_structure_beats_permuted_null() {
        // Two tight embedding clusters that match declared edges: (a,b,c) and (d,e,f).
        let nodes = vec![
            node("a", vec![1.0, 0.0, 0.0], None),
            node("b", vec![0.98, 0.1, 0.0], None),
            node("c", vec![0.97, 0.0, 0.1], None),
            node("d", vec![0.0, 1.0, 0.0], None),
            node("e", vec![0.1, 0.98, 0.0], None),
            node("f", vec![0.0, 0.97, 0.1], None),
        ];
        let edges = vec![
            TypedEdge::new(RelationType::DependsOn, "a", "b"),
            TypedEdge::new(RelationType::DependsOn, "a", "c"),
            TypedEdge::new(RelationType::DependsOn, "b", "c"),
            TypedEdge::new(RelationType::DependsOn, "d", "e"),
            TypedEdge::new(RelationType::DependsOn, "d", "f"),
            TypedEdge::new(RelationType::DependsOn, "e", "f"),
        ];
        let cfg = AlignmentConfig {
            k: 2,
            ..Default::default()
        };
        let report = analyse(&nodes, &edges, &cfg);
        assert!(report.caveat.is_none(), "caveat: {:?}", report.caveat);
        assert!(
            report.discriminates(),
            "aligned structure should beat null: mean {:.3} vs null {:.3}",
            report.mean_gap,
            report.null_gap
        );
        assert!(report.mean_gap < 0.2, "gap too high: {}", report.mean_gap);
    }

    #[test]
    fn no_structure_is_inapplicable_not_aligned() {
        let nodes = vec![
            node("a", vec![1.0, 0.0], None),
            node("b", vec![0.9, 0.1], None),
            node("c", vec![0.0, 1.0], None),
        ];
        let report = analyse(&nodes, &[], &AlignmentConfig::default());
        assert!(report.caveat.is_some(), "must caveat absence of structure");
        assert!(!report.discriminates());
    }

    #[test]
    fn null_is_deterministic() {
        let nodes = vec![
            node("a", vec![1.0, 0.0, 0.0], None),
            node("b", vec![0.9, 0.1, 0.0], None),
            node("c", vec![0.0, 1.0, 0.0], None),
            node("d", vec![0.1, 0.9, 0.0], None),
        ];
        let edges = vec![
            TypedEdge::new(RelationType::Supports, "a", "c"),
            TypedEdge::new(RelationType::Supports, "b", "d"),
        ];
        let r1 = analyse(&nodes, &edges, &AlignmentConfig::default());
        let r2 = analyse(&nodes, &edges, &AlignmentConfig::default());
        assert_eq!(r1.null_gap, r2.null_gap);
        assert_eq!(r1.delta_vs_null, r2.delta_vs_null);
    }

    #[test]
    fn depends_on_source_filters() {
        let nodes = vec![
            node("a", vec![1.0, 0.0], None),
            node("b", vec![0.99, 0.0], None),
            node("c", vec![0.0, 1.0], None),
        ];
        let edges = vec![
            TypedEdge::new(RelationType::Supports, "a", "c"), // irrelevant to DependsOn
            TypedEdge::new(RelationType::DependsOn, "a", "b"),
        ];
        let cfg = AlignmentConfig {
            sources: StructureSources::depends_on(),
            ..Default::default()
        };
        let st = structural_neighbours(&nodes, &edges, &cfg.sources);
        assert_eq!(st["a"], vec!["b".to_string()]);
        assert!(st["c"].is_empty());
    }
}
