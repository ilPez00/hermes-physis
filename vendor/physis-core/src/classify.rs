//! Semiotic-grid classification: embed text, score against ontology entries,
//! rank DOMAIN×MODE cells.
//!
//! Scoring is **nearest-entry (max cosine)**: each cell's score is the cosine of
//! the query against its best-matching member entry, not a mean centroid. A small
//! top-k average smooths single-entry noise, and a domain prior disambiguates
//! same-domain modes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::embed::VectorEmbed;
use crate::models::cosine_sim;
use crate::models::Facets;
use crate::ontology::OntologyLoader;

/// How many top member-entry similarities to average per cell.
/// Adaptive: small cells use TOP_K=1 (specificity), medium use 2, large use 3.
const TOP_K: usize = 2; // base value; adaptive logic applies at call site

/// Per-cell top-k selection for classification scoring.
///
/// [`TopKStrategy::Adaptive`] is the shipped default (see
/// [`adaptive_top_k`](Self::adaptive_top_k) on the classifier);
/// [`TopKStrategy::Fixed`] pins one k for every cell — the calibration
/// lever behind the PLAN 8.8 precision investigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopKStrategy {
    /// Cell-size-aware smoothing (1–3 entries → 1, 4–10 → 2, 11–30 → 3, 31+ → 4).
    Adaptive,
    /// The same k for every cell, regardless of size.
    Fixed(usize),
}

/// Nested-classification blend weight: final = ALPHA·domain + (1-ALPHA)·cell.
const ALPHA: f32 = 0.4;

/// Mean of the top-`k` values of an already-descending-sorted slice.
fn topk_mean(sorted_desc: &[f32], k: usize) -> f32 {
    if sorted_desc.is_empty() {
        return 0.0;
    }
    let k = k.min(sorted_desc.len()).max(1);
    sorted_desc.iter().take(k).sum::<f32>() / k as f32
}

/// Score for a single DOMAIN×MODE cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellScore {
    pub domain: String,
    pub mode: String,
    pub score: f32,
    pub entries: Vec<String>,
    /// Dominant facet profile across the cell's member entries.
    pub facets: Facets,
}

/// One populated cell with its member entry embeddings.
#[derive(Clone)]
pub struct Cell {
    pub domain: String,
    pub mode: String,
    /// Per-entry embeddings (one row per member entry).
    pub embeddings: Vec<Vec<f32>>,
    /// Member entry display names (parallel to `embeddings`).
    pub entries: Vec<String>,
    /// Per-entry facets (parallel to `embeddings`).
    pub facets: Vec<Facets>,
}

impl Cell {
    pub fn new(
        domain: String,
        mode: String,
        embeddings: Vec<Vec<f32>>,
        entries: Vec<String>,
        facets: Vec<Facets>,
    ) -> Self {
        Self {
            domain,
            mode,
            embeddings,
            entries,
            facets,
        }
    }
}

/// Pre-computed cell member embeddings ready for classification.
pub struct CellClassifier {
    pub cells: Vec<Cell>,
}

/// A raw ontology entry feed for the classifier, decoupled from any specific
/// loader type so physis-pro can hand its own `DomainDef`s straight in.
#[derive(Debug, Clone, Default)]
pub struct EntrySeed {
    pub name: String,
    pub domain: String,
    pub mode: String,
    /// Space-joined text the entry is embedded from (name + hints).
    pub text: String,
    /// Orthogonal facets the entry carries (default: none).
    pub facets: Facets,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct ClassificationMetadata {
    /// Speaker who said/wrote the text
    pub speaker: Option<String>,
    /// Context or situation
    pub context: Option<String>,
    /// Source document or folder
    pub source_document: Option<String>,
    /// Custom tags for categorization
    pub tags: Vec<String>,
    /// Any additional key-value metadata
    pub extra: std::collections::HashMap<String, String>,
}

/// A sentence classified into a semiotic cell (domain x mode) with metadata
#[derive(Clone, Debug, Serialize)]
pub struct ClassifiedSentence {
    /// The original text
    pub text: String,
    /// The (domain, mode) cell it maps to
    pub cell: (String, String), // (domain, mode) pair
    /// Metadata enabling grid slicing and filtering
    pub metadata: ClassificationMetadata,
    /// Confidence score of the classification
    pub confidence: f32,
    /// When this was classified (ISO timestamp)
    pub classified_at: Option<String>,
}

impl Default for ClassifiedSentence {
    fn default() -> Self {
        Self {
            text: String::new(),
            cell: (String::new(), String::new()),
            metadata: ClassificationMetadata::default(),
            confidence: 0.0,
            classified_at: None,
        }
    }
}

/// Metadata filter for classifying with speaker/context awareness
#[derive(Debug, Clone, Default)]
pub struct MetadataFilter {
    /// Only classify texts from these speakers
    pub speakers: Vec<String>,
    /// Only classify texts with these contexts
    pub contexts: Vec<String>,
    /// Only classify texts with these tags
    pub tags: Vec<String>,
}

impl MetadataFilter {
    /// Check if this filter allows a given metadata
    pub fn allows(&self, metadata: &ClassificationMetadata) -> bool {
        // Speaker filter
        if let Some(ref want) = self.speakers.first() {
            if let Some(ref have) = metadata.speaker.as_ref() {
                if want != have {
                    return false;
                }
            } else {
                return false; // wants speaker but text has none
            }
        }
        // Context filter
        if let Some(ref want) = self.contexts.first() {
            if let Some(ref have) = metadata.context.as_ref() {
                if want != have {
                    return false;
                }
            } else {
                return false; // wants context but text has none
            }
        }
        // Tags filter (all specified tags must be present)
        for tag in &self.tags {
            if !metadata.tags.iter().any(|t| t == tag) {
                return false;
            }
        }
        true
    }
}

// A caller-supplied facet constraint. Cells whose member facets don't match the
/// specified dimensions are down-weighted so facet-known queries rank better.
#[derive(Debug, Clone, Default)]
pub struct FacetFilter {
    pub lifecycle: Option<crate::models::LifecyclePhase>,
    pub agency: Option<crate::models::Agency>,
    pub scale: Option<crate::models::Scale>,
    pub abstraction: Option<crate::models::Abstraction>,
    pub sub_domain: Option<String>,
    pub sub_mode: Option<String>,
}

impl FacetFilter {
    /// How many dimensions the caller constrained (0 ⇒ no filtering).
    fn weight(&self) -> usize {
        let mut n = 0;
        if self.lifecycle.is_some() {
            n += 1;
        }
        if self.agency.is_some() {
            n += 1;
        }
        if self.scale.is_some() {
            n += 1;
        }
        if self.abstraction.is_some() {
            n += 1;
        }
        if self.sub_domain.is_some() {
            n += 1;
        }
        if self.sub_mode.is_some() {
            n += 1;
        }
        n
    }

    /// Match ratio in `[0, 1]` of this filter against a cell's aggregated facets.
    fn match_ratio(&self, cell: &Facets) -> f32 {
        let total = self.weight();
        if total == 0 {
            return 1.0;
        }
        let mut hits = 0;
        if let (Some(want), Some(have)) = (self.lifecycle, cell.lifecycle) {
            if want == have {
                hits += 1;
            }
        }
        if let (Some(want), Some(have)) = (self.agency, cell.agency) {
            if want == have {
                hits += 1;
            }
        }
        if let (Some(want), Some(have)) = (self.scale, cell.scale) {
            if want == have {
                hits += 1;
            }
        }
        if let (Some(want), Some(have)) = (self.abstraction, cell.abstraction) {
            if want == have {
                hits += 1;
            }
        }
        if let (Some(want), Some(have)) = (&self.sub_domain, &cell.sub_domain) {
            if want.eq_ignore_ascii_case(have) {
                hits += 1;
            }
        }
        if let (Some(want), Some(have)) = (&self.sub_mode, &cell.sub_mode) {
            if want.eq_ignore_ascii_case(have) {
                hits += 1;
            }
        }
        hits as f32 / total as f32
    }
}

impl CellClassifier {
    /// Embed every entry seed (its `text`), group embeddings by DOMAIN×MODE cell.
    /// Takes an embed closure instead of the `VectorEmbed` trait so external
    /// crates with their own embedder traits (physis-pro) can hook in directly.
    pub fn build_seeds<F>(seeds: impl IntoIterator<Item = EntrySeed>, embed: F) -> Self
    where
        F: Fn(&str) -> Vec<f32>,
    {
        type Acc = (Vec<Vec<f32>>, Vec<String>, Vec<Facets>);
        let mut acc: HashMap<(String, String), Acc> = HashMap::new();
        for seed in seeds {
            let emb = embed(&seed.text);
            let entry = acc
                .entry((seed.domain.clone(), seed.mode.clone()))
                .or_insert_with(|| (Vec::new(), Vec::new(), Vec::new()));
            entry.0.push(emb);
            entry.1.push(seed.name);
            entry.2.push(seed.facets);
        }
        let cells = acc
            .into_iter()
            .map(|((domain, mode), (embeddings, entries, facets))| Cell {
                domain,
                mode,
                embeddings,
                entries,
                facets,
            })
            .collect();
        Self { cells }
    }

    /// Rebuild from pre-built cells (used by tests / adapters that already
    /// constructed the internal shape).
    pub fn from_cells(cells: Vec<Cell>) -> Self {
        Self { cells }
    }

    /// Embed every classification ontology entry ("name + hints") and group the
    /// embeddings by DOMAIN×MODE cell, carrying each entry's facets.
    pub fn build(ontology: &OntologyLoader, embedder: &dyn VectorEmbed) -> Self {
        let seeds = ontology.classification_domains().filter_map(|def| {
            let (domain, mode) = match (def.domain.as_deref(), def.mode.as_deref()) {
                (Some(d), Some(m)) => (d.to_string(), m.to_string()),
                _ => return None,
            };
            let mut text = def.name.clone();
            for hint in &def.hints {
                text.push(' ');
                text.push_str(hint);
            }
            Some(EntrySeed {
                name: def.name.clone(),
                domain,
                mode,
                text,
                facets: def.facets.clone(),
            })
        });
        Self::build_seeds(seeds, |t| embedder.embed(t))
    }

    /// Number of populated cells.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Adaptive TOP_K based on cell entry count.
    ///
    /// **No longer the default** (2026-08-24, PLAN 8.8): measurement under
    /// bge-base showed smoothing hurts once cells grow large — averaging
    /// dilutes the precise anchor that max would have used. Kept reachable
    /// via [`TopKStrategy`] for recalibration sweeps; `Fixed(1)` ships.
    pub fn adaptive_top_k(num_entries: usize) -> usize {
        let base = TOP_K; // = 2, the medium default
        match num_entries {
            1..=3 => 1,
            4..=10 => base,      // = 2
            11..=30 => base + 1, // = 3
            _ => base + 2,       // = 4
        }
    }

    /// Classify a pre-computed embedding with soft nested (domain→mode) scoring.
    ///
    /// Cell score = **max cosine to any member entry** (`Fixed(1)`).
    /// Measured 2026-08-24 under bge-base-en on the curated fixture set
    /// (PLAN 8.8): max beats every smoothed variant on cell precision
    /// (CORE 0.782 vs 0.636 adaptive) and ties on domain, because the
    /// 2026-08-14 ontology expansion turned top-k averaging into a dilution
    /// machine — loosely-related entries drag precise anchors toward the cell
    /// mean. `classify_with_topk` keeps the alternatives reachable.
    pub fn classify(&self, embedding: &[f32]) -> Vec<CellScore> {
        self.classify_with_topk(embedding, TopKStrategy::Fixed(1))
    }

    /// Same scoring with an explicit top-k strategy — the tuning seam behind
    /// PLAN 8.8: the 2026-08-14 ontology expansion grew cells into higher
    /// smoothing buckets and cell-level precision fell. This lets a harness
    /// sweep strategies against fixtures and pick a new default on measurement.
    pub fn classify_with_topk(&self, embedding: &[f32], strat: TopKStrategy) -> Vec<CellScore> {
        let k_of = |n: usize| match strat {
            TopKStrategy::Adaptive => Self::adaptive_top_k(n),
            TopKStrategy::Fixed(k) => k,
        };
        let mut domain_cell_scores: HashMap<&str, Vec<f32>> = HashMap::new();
        let mut cell_scores: Vec<f32> = Vec::with_capacity(self.cells.len());
        let mut ranked_entries: Vec<Vec<String>> = Vec::with_capacity(self.cells.len());
        for cell in &self.cells {
            let k = k_of(cell.embeddings.len());
            // Pair each member's similarity with its name so the displayed
            // exemplars can be ordered by the similarity that produced them.
            let mut scored: Vec<(f32, &String)> = cell
                .embeddings
                .iter()
                .zip(&cell.entries)
                .map(|(e, name)| (cosine_sim(embedding, e), name))
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let sims: Vec<f32> = scored.iter().map(|(s, _)| *s).collect();
            let cs = topk_mean(&sims, k);
            cell_scores.push(cs);
            ranked_entries.push(scored.into_iter().map(|(_, name)| name.clone()).collect());
            domain_cell_scores
                .entry(cell.domain.as_str())
                .or_default()
                .push(cs);
        }

        let domain_score: HashMap<&str, f32> = domain_cell_scores
            .into_iter()
            .map(|(d, mut cs)| {
                cs.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                (d, topk_mean(&cs, 2))
            })
            .collect();

        let mut results: Vec<CellScore> = self
            .cells
            .iter()
            .zip(cell_scores)
            .zip(ranked_entries)
            .map(|((cell, cell_score), entries)| {
                let ds = *domain_score.get(cell.domain.as_str()).unwrap_or(&0.0);
                // Population weight: larger cells have more reliable top entries,
                // so their scores get a modest boost; small cells get a small boost
                // to prevent them from being drowned out by big cells.
                let pop_weight = if cell.embeddings.len() >= 10 {
                    1.0 + (cell.embeddings.len() as f32 / 100.0).min(0.2)
                } else if cell.embeddings.len() <= 3 {
                    1.0 + (3.0 - cell.embeddings.len() as f32) / 10.0
                } else {
                    1.0
                };
                let score = (ALPHA * ds + (1.0 - ALPHA) * cell_score) * pop_weight;
                CellScore {
                    domain: cell.domain.clone(),
                    mode: cell.mode.clone(),
                    score,
                    // Already ranked by per-entry similarity in the scoring loop.
                    entries,
                    facets: Facets::aggregate(&cell.facets),
                }
            })
            .collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // `pop_weight` multiplies a blend of two cosines, so a well-populated cell
        // can score above 1.0 — outside the range every consumer assumes. Callers
        // then clamp (`QualityTracker::adjust_score` does), and since the overflow
        // is common, the top handful of cells all flattened to exactly 1.000 and the
        // CLI printed a ten-way tie with full bars over scores that were in fact
        // distinct and correctly ordered.
        //
        // Rescale by the maximum instead of clipping at it. Division by a positive
        // constant is strictly order-preserving, so the ranking and the argmax are
        // untouched — only the ceiling moves back to 1.0 and the spread survives.
        if let Some(max) = results.first().map(|r| r.score) {
            if max > 1.0 {
                for r in &mut results {
                    r.score /= max;
                }
            }
        }
        results
    }

    /// Classify with metadata tracking, returning a `ClassifiedSentence` for the top result.
    /// Uses the top-scoring cell and incorporates metadata for grid slicing.
    /// The `filter` can restrict which texts/metadata are considered; pass `Default::default()`
    /// for no filtering.
    pub fn classify_with_metadata(
        &self,
        embedding: &[f32],
        metadata: &ClassificationMetadata,
        filter: &MetadataFilter,
    ) -> Option<ClassifiedSentence> {
        let results = self.classify(embedding);
        if results.is_empty() {
            return None;
        }
        let top = &results[0];

        // Apply metadata filter - if filter is set, ensure the top result's cell matches
        // In the future, this could weight cells by metadata relevance
        if !filter.allows(metadata) {
            // Filter doesn't match; return None
            return None;
        }

        Some(ClassifiedSentence {
            text: "classified text".to_string(), // TODO: pass text in future
            cell: (top.domain.clone(), top.mode.clone()),
            metadata: metadata.clone(),
            confidence: top.score,
            classified_at: Some(
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3f")
                    .to_string(),
            ),
        })
    }

    /// Classify with an optional facet constraint. Cells whose aggregated facets
    /// match the filter are up-weighted; the more dimensions match, the higher
    /// the factor (1.0 when the filter is empty, so this degrades to `classify`).
    pub fn classify_filtered(&self, embedding: &[f32], filter: &FacetFilter) -> Vec<CellScore> {
        let mut results = self.classify(embedding);
        for r in &mut results {
            if let Some(cell) = self
                .cells
                .iter()
                .find(|c| c.domain == r.domain && c.mode == r.mode)
            {
                let ratio = filter.match_ratio(&Facets::aggregate(&cell.facets));
                r.score *= 0.5 + 0.5 * ratio;
            }
        }
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Embed `text` and classify it in one call.
    pub fn classify_text(&self, text: &str, embedder: &dyn VectorEmbed) -> Vec<CellScore> {
        self.classify(&embedder.embed(text))
    }

    /// Propose top-k cells from accumulated decisions for a given text.
    ///
    /// Uses the existing cell classifier's state to find the nearest cells
    /// to the query text. This is the "filing proposer" mechanism: given text a
    /// user has already confirmed/filed, propose which cells it likely belongs in.
    ///
    /// Returns `(cell_key, confidence, centroid_distance)` tuples sorted by
    /// confidence descending. No new dependencies — pure vector arithmetic over
    /// the existing classifier state.
    ///
    /// The `filter` can restrict which cells are considered; pass `Default::default()`
    /// for no filtering.
    pub fn propose_cells(
        &self,
        text: &str,
        embedder: &dyn VectorEmbed,
        _filter: &MetadataFilter,
    ) -> Vec<((String, String), f32, f32)> {
        let embedding = embedder.embed(text);
        let results = self.classify(&embedding);

        let mut proposals: Vec<((String, String), f32, f32)> = Vec::new();
        for cell_result in &results {
            let cell_key = (cell_result.domain.clone(), cell_result.mode.clone());
            let centroid = self
                .cells
                .iter()
                .find(|c| c.domain == cell_result.domain && c.mode == cell_result.mode)
                .map(|c| {
                    let dim = c.embeddings[0].len();
                    let n = c.embeddings.len() as f32;
                    let mut mean = vec![0.0f32; dim];
                    for e in &c.embeddings {
                        for (i, v) in e.iter().enumerate() {
                            mean[i] += v;
                        }
                    }
                    mean.iter_mut().for_each(|v| *v /= n);
                    // Compute distance from query embedding to centroid
                    let mut dist_sq = 0.0f32;
                    for (a, b) in embedding.iter().zip(mean.iter()) {
                        let d = a - b;
                        dist_sq += d * d;
                    }
                    dist_sq.sqrt()
                })
                .unwrap_or(0.0f32);
            let confidence = cell_result.score;
            proposals.push((cell_key, confidence, centroid));
        }

        proposals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        proposals
    }

    /// Adaptive hint weighting: `normalise(v_name + 0.5·(v_hints − v_name))`
    /// Simplifies to averaging the name and hints embeddings, then L2-normalising.
    /// Worth ~+0.054 top-3 over the current representation (E24/E28, unanimous).
    pub fn hint_weighted_embed(name_embed: &[f32], hints_embed: &[f32]) -> Vec<f32> {
        let dim = name_embed.len();
        let mut combined = vec![0.0f32; dim];
        for i in 0..dim {
            combined[i] = (name_embed[i] + hints_embed[i]) / 2.0f32;
        }
        let norm: f32 = combined.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
        combined.iter_mut().for_each(|v| *v /= norm);
        combined
    }

    /// Mean-of-entries centroid per cell, keyed `"DOMAIN\x00MODE"` — the shape
    /// the quality tracker consumes for penalty targeting.
    pub fn cell_centroids(&self) -> HashMap<String, Vec<f32>> {
        self.cells
            .iter()
            .filter(|c| !c.embeddings.is_empty())
            .map(|c| {
                let dim = c.embeddings[0].len();
                let n = c.embeddings.len() as f32;
                let mut mean = vec![0.0f32; dim];
                for e in &c.embeddings {
                    for (i, v) in e.iter().enumerate() {
                        mean[i] += v;
                    }
                }
                mean.iter_mut().for_each(|v| *v /= n);
                (format!("{}\x00{}", c.domain, c.mode), mean)
            })
            .collect()
    }

    /// Raw best single-entry cosine and its owning cell — the ontology coverage
    /// of an embedding, undiluted by blended scoring. Low values mean "no
    /// existing entry is near this text".
    pub fn best_entry_sim(&self, embedding: &[f32]) -> Option<(f32, String, String)> {
        let mut best: Option<(f32, String, String)> = None;
        for cell in &self.cells {
            for e in &cell.embeddings {
                let s = cosine_sim(embedding, e);
                if best.as_ref().map(|(b, _, _)| s > *b).unwrap_or(true) {
                    best = Some((s, cell.domain.clone(), cell.mode.clone()));
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Agency, LifecyclePhase};

    fn basis(dim: usize, axis: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; dim];
        v[axis] = 1.0;
        v
    }

    fn cell(domain: &str, mode: &str, embeddings: Vec<Vec<f32>>) -> Cell {
        let entries = (0..embeddings.len())
            .map(|i| format!("{domain}/{mode}#{i}"))
            .collect();
        let facets = vec![Facets::default(); embeddings.len()];
        Cell::new(
            domain.to_string(),
            mode.to_string(),
            embeddings,
            entries,
            facets,
        )
    }

    /// A populated cell gets `pop_weight` > 1.0 applied on top of a blend of
    /// cosines, which used to push scores past 1.0. Consumers clamp there, so
    /// the leaders all collapsed onto exactly 1.000 and the CLI printed a tie
    /// with identical full bars over scores that were genuinely ordered.
    #[test]
    fn scores_stay_within_range_without_flattening_the_ranking() {
        // 12 embeddings puts the cell over the >=10 threshold for pop_weight.
        let many = |axis: usize| vec![basis(4, axis); 12];
        let clf = CellClassifier {
            cells: vec![
                cell("A", "X", many(0)),
                cell("A", "Y", many(0)),
                cell("B", "Z", many(1)),
            ],
        };
        let out = clf.classify(&basis(4, 0));

        for r in &out {
            assert!(
                r.score <= 1.0 + f32::EPSILON,
                "{}x{} scored {} — above the range every consumer clamps to",
                r.domain,
                r.mode,
                r.score
            );
            assert!(r.score >= 0.0, "scores must not go negative: {}", r.score);
        }
        // Rescaling is order-preserving: still sorted, and the winner is intact.
        assert!(
            out.windows(2).all(|w| w[0].score >= w[1].score),
            "must stay sorted"
        );
        assert_eq!(out[0].domain, "A");
        // And the losing cell is still visibly worse rather than tied at 1.000.
        assert!(
            out.last().unwrap().score < out[0].score,
            "a non-matching cell must not tie with the winner"
        );
    }

    #[test]
    fn classify_ranks_matching_cell_first_and_sorts_desc() {
        let clf = CellClassifier {
            cells: vec![
                cell("A", "X", vec![basis(4, 0)]),
                cell("A", "Y", vec![basis(4, 1)]),
                cell("B", "X", vec![basis(4, 2)]),
            ],
        };
        let results = clf.classify(&basis(4, 0));
        assert_eq!(results.len(), 3);
        assert_eq!(
            (results[0].domain.as_str(), results[0].mode.as_str()),
            ("A", "X")
        );
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn nearest_entry_scoring_resists_dense_cell_hub_bias() {
        let hub = basis(4, 3);
        let dense = cell("B", "X", vec![hub.clone(); 10]);
        let sparse = cell("A", "X", vec![basis(4, 0)]);
        let clf = CellClassifier {
            cells: vec![dense, sparse],
        };
        let results = clf.classify(&basis(4, 0));
        assert_eq!(results[0].domain, "A");
    }

    #[test]
    fn best_entry_sim_returns_global_max() {
        let clf = CellClassifier {
            cells: vec![
                cell("A", "X", vec![basis(4, 1), basis(4, 0)]),
                cell("B", "Y", vec![basis(4, 2)]),
            ],
        };
        let (score, domain, mode) = clf.best_entry_sim(&basis(4, 0)).unwrap();
        assert!((score - 1.0).abs() < 1e-6);
        assert_eq!((domain.as_str(), mode.as_str()), ("A", "X"));
    }

    #[test]
    fn cell_centroids_mean_and_key_format() {
        let clf = CellClassifier {
            cells: vec![cell("A", "X", vec![vec![1.0, 0.0], vec![0.0, 1.0]])],
        };
        let centroids = clf.cell_centroids();
        let c = centroids
            .get("A\x00X")
            .expect("key must be DOMAIN\\x00MODE");
        assert_eq!(c, &vec![0.5, 0.5]);
    }

    #[test]
    fn classify_carries_aggregated_facets() {
        let mut c = cell("A", "X", vec![basis(4, 0), basis(4, 0), basis(4, 0)]);
        c.facets = vec![
            Facets {
                lifecycle: Some(LifecyclePhase::Operate),
                agency: Some(Agency::SelfActor),
                ..Default::default()
            },
            Facets {
                lifecycle: Some(LifecyclePhase::Operate),
                agency: Some(Agency::SelfActor),
                ..Default::default()
            },
            Facets {
                lifecycle: Some(LifecyclePhase::Design),
                agency: Some(Agency::Automated),
                ..Default::default()
            },
        ];
        let clf = CellClassifier { cells: vec![c] };
        let r = &clf.classify(&basis(4, 0))[0];
        assert_eq!(r.domain, "A");
        assert_eq!(r.facets.lifecycle, Some(LifecyclePhase::Operate));
        // 2/3 SelfActor, 2/3 Operate → clear majorities.
        assert_eq!(r.facets.agency, Some(Agency::SelfActor));
    }

    #[test]
    fn classify_filtered_upweights_matching_facets() {
        let mut c = cell("A", "X", vec![basis(4, 0)]);
        c.facets = vec![Facets {
            lifecycle: Some(LifecyclePhase::Operate),
            ..Default::default()
        }];
        let other = cell("B", "Y", vec![basis(4, 0)]);
        let clf = CellClassifier {
            cells: vec![c, other.clone()],
        };

        // Without a filter both cells tie on embedding; A should still surface via facets.
        let filter = FacetFilter {
            lifecycle: Some(LifecyclePhase::Operate),
            ..Default::default()
        };
        let ranked = clf.classify_filtered(&basis(4, 0), &filter);
        assert_eq!(
            (ranked[0].domain.as_str(), ranked[0].mode.as_str()),
            ("A", "X")
        );
    }

    #[test]
    fn classify_filtered_empty_is_identity() {
        let clf = CellClassifier {
            cells: vec![cell("A", "X", vec![basis(4, 0)])],
        };
        let a = clf.classify(&basis(4, 0));
        let b = clf.classify_filtered(&basis(4, 0), &FacetFilter::default());
        assert!((a[0].score - b[0].score).abs() < 1e-6);
    }
}
