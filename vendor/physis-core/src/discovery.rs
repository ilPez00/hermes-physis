//! Discovery — which corpus texts does the current ontology fail to cover, and
//! what entries would cover them?
//!
//! Ported from physis-pro. Scoring is raw best-entry cosine (undiluted by the
//! blended classifier), texts below threshold are unmapped, unmapped texts are
//! clustered by embedding similarity, and each surviving cluster becomes a
//! proposed new ontology entry. `auto_retune` rescues the "all covered" trap by
//! raising the threshold to a signal-aware quantile when the first pass finds
//! nothing (P10).

use serde::{Deserialize, Serialize};

use crate::classify::CellClassifier;
use crate::embed::VectorEmbed;
use crate::models::cosine_sim;

#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub coverage_threshold: f32,
    pub cluster_sim: f32,
    pub min_cluster: usize,
    pub max_hints: usize,
    pub auto_retune: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            coverage_threshold: 0.55,
            cluster_sim: 0.70,
            min_cluster: 2,
            max_hints: 8,
            auto_retune: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedDomain {
    pub name: String,
    pub domain: String,
    pub mode: String,
    pub hints: Vec<String>,
    pub samples: Vec<String>,
    pub count: usize,
    pub coverage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub total: usize,
    pub covered: usize,
    pub unmapped: usize,
    pub proposals: Vec<ProposedDomain>,
    pub sim_min: f32,
    pub sim_mean: f32,
    pub sim_max: f32,
    pub threshold_used: f32,
    pub auto_retuned: bool,
}

/// Tiny bilingual stopword list — just enough to keep hints meaningful.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "for", "with", "is", "are", "was", "be",
    "this", "that", "it", "at", "by", "from", "as", "we", "il", "lo", "la", "i", "gli", "le", "un",
    "uno", "una", "di", "del", "della", "e", "o", "che", "per", "con", "su", "da", "dal", "dalla",
    "al", "alla", "in", "non", "si", "ha", "sono", "è",
];

fn terms(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|w| w.trim_matches('\'').to_lowercase())
        .filter(|w| w.len() > 2 && !STOPWORDS.contains(&w.as_str()))
}

/// Most frequent terms across a cluster, ties broken alphabetically (determinism).
pub(crate) fn top_terms(texts: &[&str], max: usize) -> Vec<String> {
    let mut freq: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for t in texts {
        for w in terms(t) {
            *freq.entry(w).or_default() += 1;
        }
    }
    let mut ranked: Vec<(String, usize)> = freq.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.into_iter().take(max).map(|(w, _)| w).collect()
}

/// One scored corpus text: its best-entry cosine and the nearest cell.
struct Scored {
    text: String,
    embedding: Vec<f32>,
    best: f32,
    domain: String,
    mode: String,
}

fn sim_stats(sims: &[f32]) -> (f32, f32) {
    if sims.is_empty() {
        return (0.0, 0.0);
    }
    let n = sims.len() as f32;
    let mean = sims.iter().copied().sum::<f32>() / n;
    let var = sims.iter().map(|s| (s - mean) * (s - mean)).sum::<f32>() / n;
    (mean, var.sqrt())
}

fn tail_threshold(sorted_sims: &[f32], frac: f32) -> Option<f32> {
    if sorted_sims.is_empty() {
        return None;
    }
    let q = ((sorted_sims.len() as f32 * frac) as usize).min(sorted_sims.len() - 1);
    Some(sorted_sims[q])
}

/// Split scored texts at `threshold`, cluster the unmapped, draft proposals.
fn cluster_at(
    scored: &[Scored],
    threshold: f32,
    cfg: &DiscoveryConfig,
) -> (usize, Vec<ProposedDomain>) {
    let unmapped: Vec<&Scored> = scored.iter().filter(|s| s.best < threshold).collect();
    let covered = scored.len() - unmapped.len();

    let mut assigned = vec![false; unmapped.len()];
    let mut proposals: Vec<ProposedDomain> = Vec::new();
    for i in 0..unmapped.len() {
        if assigned[i] {
            continue;
        }
        assigned[i] = true;
        let mut members = vec![i];
        for j in (i + 1)..unmapped.len() {
            if !assigned[j]
                && cosine_sim(&unmapped[i].embedding, &unmapped[j].embedding) > cfg.cluster_sim
            {
                assigned[j] = true;
                members.push(j);
            }
        }
        if members.len() < cfg.min_cluster {
            continue;
        }
        let member_texts: Vec<&str> = members.iter().map(|&k| unmapped[k].text.as_str()).collect();
        let hints = top_terms(&member_texts, cfg.max_hints);
        let name = {
            let mut words: Vec<String> = hints.iter().take(2).cloned().collect();
            for w in &mut words {
                if let Some(c) = w.get_mut(0..1) {
                    let up = c.to_uppercase();
                    w.replace_range(0..1, &up);
                }
            }
            if words.is_empty() {
                "Unnamed Cluster".to_string()
            } else {
                words.join(" & ")
            }
        };
        let coverage =
            members.iter().map(|&k| unmapped[k].best).sum::<f32>() / members.len() as f32;
        proposals.push(ProposedDomain {
            name,
            domain: unmapped[members[0]].domain.clone(),
            mode: unmapped[members[0]].mode.clone(),
            hints,
            samples: member_texts
                .iter()
                .take(3)
                .map(ToString::to_string)
                .collect(),
            count: members.len(),
            coverage,
        });
    }
    proposals.sort_by_key(|p| std::cmp::Reverse(p.count));
    (covered, proposals)
}

/// Run discovery: which of `texts` does the current ontology fail to cover,
/// and what entries would cover them?
pub fn discover(
    texts: &[String],
    clf: &CellClassifier,
    embedder: &dyn VectorEmbed,
    cfg: &DiscoveryConfig,
) -> DiscoveryReport {
    const EMBED_CHUNK: usize = 16;
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(refs.len());
    for chunk in refs.chunks(EMBED_CHUNK) {
        embeddings.extend(embedder.embed_batch(chunk));
    }

    let (mut sim_min, mut sim_max, mut sim_sum) = (f32::INFINITY, f32::NEG_INFINITY, 0.0f32);
    let scored: Vec<Scored> = texts
        .iter()
        .zip(embeddings)
        .map(|(text, emb)| {
            let (best, d, m) =
                clf.best_entry_sim(&emb)
                    .unwrap_or((0.0, String::new(), String::new()));
            sim_min = sim_min.min(best);
            sim_max = sim_max.max(best);
            sim_sum += best;
            Scored {
                text: text.clone(),
                embedding: emb,
                best,
                domain: d,
                mode: m,
            }
        })
        .collect();
    let n = texts.len().max(1) as f32;
    let (sim_min, sim_max) = if texts.is_empty() {
        (0.0, 0.0)
    } else {
        (sim_min, sim_max)
    };

    let mut threshold = cfg.coverage_threshold;
    let (mut covered, mut proposals) = cluster_at(&scored, threshold, cfg);
    let mut auto_retuned = false;

    if proposals.is_empty() && cfg.auto_retune && !scored.is_empty() {
        let mut sims: Vec<f32> = scored.iter().map(|s| s.best).collect();
        sims.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let (mean, stdev) = sim_stats(&sims);
        let k = if scored.len() <= 8 { 0.5 } else { 1.0 };
        let mean_plus = mean + k * stdev.max(1e-4);
        let tail = tail_threshold(&sims, 0.20).unwrap_or(mean_plus);
        let mut candidates: Vec<f32> = vec![mean_plus, tail, sim_max];
        candidates.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        candidates.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

        for cand in candidates {
            if cand <= threshold {
                continue;
            }
            let (c2, p2) = cluster_at(&scored, cand, cfg);
            if !p2.is_empty() {
                threshold = cand;
                covered = c2;
                proposals = p2;
                auto_retuned = true;
                break;
            }
        }

        if proposals.is_empty() && scored.len() >= 3 {
            let loosened = DiscoveryConfig {
                min_cluster: 1,
                ..cfg.clone()
            };
            if let Some(&worst) = sims.first() {
                let cand = worst.max(threshold + 1e-3);
                let (c2, p2) = cluster_at(&scored, cand, &loosened);
                if !p2.is_empty() {
                    threshold = cand;
                    covered = c2;
                    proposals = p2;
                    auto_retuned = true;
                }
            }
        }
    }

    DiscoveryReport {
        total: texts.len(),
        covered,
        unmapped: texts.len() - covered,
        proposals,
        sim_min,
        sim_mean: sim_sum / n,
        sim_max,
        threshold_used: threshold,
        auto_retuned,
    }
}

/// Render proposals as a ready-to-edit ontology JSON (the same shape as the
/// config/ files) so the studio can round-trip a gap straight into the grid.
pub fn to_ontology_json(report: &DiscoveryReport, kind: &str) -> String {
    let domains: Vec<serde_json::Value> = report
        .proposals
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "category": "Discovered",
                "domain": if p.domain.is_empty() { "STUDY" } else { p.domain.as_str() },
                "mode": if p.mode.is_empty() { "WORK" } else { p.mode.as_str() },
                "axis_kind": kind,
                "axis_name": "discovered",
                "unit": "items",
                "hints": p.hints,
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({ "kind": kind, "domains": domains }))
        .expect("ontology json serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;
    use crate::ontology::OntologyLoader;

    fn clf_and_embedder() -> (CellClassifier, RandomProjectionEmbedder) {
        let ontology = OntologyLoader::load_all();
        let embedder = RandomProjectionEmbedder::new(64);
        let clf = CellClassifier::build(&ontology, &embedder);
        (clf, embedder)
    }

    #[test]
    fn identical_unmapped_texts_cluster_into_one_proposal() {
        let (clf, embedder) = clf_and_embedder();
        let texts = vec![
            "zorgon flux capacitor recalibration".to_string(),
            "zorgon flux capacitor recalibration".to_string(),
            "zorgon flux capacitor recalibration".to_string(),
        ];
        let cfg = DiscoveryConfig {
            coverage_threshold: 1.1,
            ..Default::default()
        };
        let report = discover(&texts, &clf, &embedder, &cfg);
        assert_eq!(report.unmapped, 3);
        assert_eq!(report.proposals.len(), 1);
        let p = &report.proposals[0];
        assert_eq!(p.count, 3);
        assert!(p.hints.contains(&"zorgon".to_string()));
    }

    #[test]
    fn zero_threshold_covers_everything() {
        let (clf, embedder) = clf_and_embedder();
        let texts = vec!["anything at all".to_string(), "another text".to_string()];
        let cfg = DiscoveryConfig {
            coverage_threshold: -1.0,
            auto_retune: false,
            ..Default::default()
        };
        let report = discover(&texts, &clf, &embedder, &cfg);
        assert_eq!(report.covered, 2);
        assert!(report.proposals.is_empty());
    }

    #[test]
    fn singletons_dropped_below_min_cluster() {
        let (clf, embedder) = clf_and_embedder();
        let texts = vec![
            "wibble grommet anodizing".to_string(),
            "completely unrelated quasar telemetry".to_string(),
        ];
        let cfg = DiscoveryConfig {
            coverage_threshold: 1.1,
            ..Default::default()
        };
        let report = discover(&texts, &clf, &embedder, &cfg);
        assert_eq!(report.unmapped, 2);
        assert!(report.proposals.is_empty());
    }

    #[test]
    fn top_terms_skips_stopwords_bilingual() {
        let t = [
            "la manutenzione del cuscinetto",
            "the maintenance of the cuscinetto",
        ];
        let terms = top_terms(&t, 4);
        assert!(terms.contains(&"cuscinetto".to_string()));
        assert!(!terms.contains(&"the".to_string()));
        assert!(!terms.contains(&"della".to_string()));
    }

    #[test]
    fn auto_retune_surfaces_gap_for_diffuse_corpus() {
        let (clf, embedder) = clf_and_embedder();
        let texts: Vec<String> = (0..5)
            .map(|i| format!("zorgon flux capacitor recalibration variant {i}"))
            .collect();
        let cfg = DiscoveryConfig {
            coverage_threshold: 0.55,
            auto_retune: true,
            ..Default::default()
        };
        let report = discover(&texts, &clf, &embedder, &cfg);
        assert!(
            !report.proposals.is_empty(),
            "P10: diffuse corpus must still surface a gap"
        );
        assert!(report.auto_retuned);
        assert!(report.covered < report.total);
    }

    #[test]
    fn ontology_json_round_trips_through_loader() {
        let report = DiscoveryReport {
            total: 3,
            covered: 0,
            unmapped: 3,
            proposals: vec![ProposedDomain {
                name: "Zorgon & Flux".into(),
                domain: "STUDY".into(),
                mode: "WORK".into(),
                hints: vec!["zorgon".into(), "flux".into()],
                samples: vec!["zorgon flux capacitor".into()],
                count: 3,
                coverage: 0.12,
            }],
            sim_min: 0.1,
            sim_mean: 0.12,
            sim_max: 0.15,
            threshold_used: 0.55,
            auto_retuned: false,
        };
        let json = to_ontology_json(&report, "discovered_test");
        let map = OntologyLoader::load_from_str(&json).expect("draft loads via the real loader");
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("Zorgon & Flux").unwrap().domain.as_deref(),
            Some("STUDY")
        );
    }
}
