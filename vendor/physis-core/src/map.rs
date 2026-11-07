//! The Difference/Repetition structural map — Demo A ("the corpus becoming
//! visible").
//!
//! Composes existing, tested machinery; adds no new representation:
//! - [`crate::discovery`] deterministic clustering → **what repeats**
//! - per-text best-cell similarity below the threshold → **what differs**
//! - high lexical overlap + low embedding similarity inside a cluster →
//!   **contradiction candidates** (reported, never merged)
//! - a sha256 over the sorted cluster signatures → the **reproducibility
//!   hash** shown on screen (same corpus ⇒ identical map, deterministically).
//!
//! Everything traces back to source files: every entry carries its path.

use crate::classify::CellClassifier;
use crate::discovery::{self, DiscoveryConfig};
use crate::embed::VectorEmbed;
use crate::ontology::OntologyLoader;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatCluster {
    /// Stable id: r-001 … in deterministic (name, first-path) order.
    pub id: String,
    pub name: String,
    pub hints: Vec<String>,
    pub count: usize,
    pub coverage: f32,
    /// Provenance: every instance, path + title.
    pub instances: Vec<SourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceItem {
    pub path: String,
    pub title: String,
    /// Best grid-cell similarity — below threshold, hence "different".
    pub best_sim: f32,
    /// How much lonelier this doc is than the corpus median nearest
    /// neighbour — the honest "unseen combination" measure, relative so it
    /// works with any embedder (random projection or semantic).
    pub separation: f32,
    pub nearest_cluster: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionPair {
    pub a: SourceRef,
    pub b: SourceRef,
    pub cluster: String,
    pub lexical_overlap: f32,
    pub embedding_sim: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapReport {
    pub docs: usize,
    pub repeats: Vec<RepeatCluster>,
    pub differences: Vec<DifferenceItem>,
    pub contradictions: Vec<ContradictionPair>,
    pub threshold_used: f32,
    /// sha256 over the sorted cluster signatures — run twice, same hash.
    pub structure_hash: String,
    pub auto_retuned: bool,
}

impl MapReport {
    /// The hero-screen numbers, one line each — all real, all computed.
    pub fn hero_lines(&self) -> Vec<String> {
        vec![
            format!("{} DOCUMENTS", self.docs),
            format!("↓ PHYSIS"),
            format!("{} RECURRING PATTERNS", self.repeats.len()),
            format!("{} SIGNIFICANT DIFFERENCES", self.differences.len()),
            format!("{} CONTRADICTION CANDIDATES", self.contradictions.len()),
            format!("structure hash {}", &self.structure_hash[..12]),
        ]
    }
}

/// Negated-obligation markers. One side affirming what the other forbids is a
/// contradiction even at near-identical wording (the duplicate trap).
const NEGATION_MARKERS: &[&str] = &[
    "not ",
    "no ",
    "non ",
    "mai ",
    "vietato",
    "divieto",
    "proibito",
    "must not",
    "shall not",
    "non può",
    "non deve",
];

fn negation_hits(text: &str) -> usize {
    let low = text.to_lowercase();
    NEGATION_MARKERS
        .iter()
        .filter(|m| low.contains(m.trim_end()))
        .count()
}

/// First number-bearing word (kept as text).
fn leading_number(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .find(|t| {
            t.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })
        .map(|t| t.chars().take_while(|c| c.is_alphanumeric()).collect())
}

/// A number with a decimal point — i.e. a *measurement*, not an instance id.
fn decimal_number(text: &str) -> Option<String> {
    leading_number(text).filter(|n| n.contains('.'))
}

fn lexical_overlap(a: &str, b: &str) -> f32 {
    let toks = |t: &str| -> std::collections::BTreeSet<String> {
        t.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3)
            .map(|w| w.to_lowercase())
            .collect()
    };
    let (a, b) = (toks(a), toks(b));
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(&b).count();
    inter as f32 / a.union(&b).count() as f32
}

/// Deterministic cosine (no library dependency on models.rs to keep map.rs
/// self-composable).
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (dot, na, nb) = a
        .iter()
        .zip(b)
        .fold((0.0f32, 0.0f32, 0.0f32), |(d, x, y), (p, q)| {
            (d + p * q, x + p * p, y + q * q)
        });
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// Build the map over an ingested corpus. Deterministic given the same files
/// and the same (default) embedder — the random-projection embedder is seeded
/// and the clustering is order-stable.
pub fn build_map(
    docs: &[(String, String)], // (path, body) — provenance preserved
    embedder: &dyn VectorEmbed,
    threshold: Option<f32>,
) -> anyhow::Result<MapReport> {
    let ontology = OntologyLoader::load_all();
    let clf = CellClassifier::build(&ontology, embedder);
    let cfg = DiscoveryConfig {
        min_cluster: 2,
        ..DiscoveryConfig::default()
    };
    let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
    let report = discovery::discover(&texts, &clf, embedder, &cfg);
    let threshold = threshold.unwrap_or(report.threshold_used);

    // ── what repeats: corpus-internal nearest-neighbour union-find ──────────
    // Deterministic and embedder-agnostic: two documents repeat each other
    // when their vectors are nearly identical; transitive closure groups the
    // family. (Grid-coverage proposals remain available via `discovery`.)
    let doc_embs: Vec<Vec<f32>> = texts.iter().map(|t| embedder.embed(t)).collect();
    // Median nearest-neighbour similarity = what "repeated" looks like here.
    let nnds: Vec<f32> = doc_embs
        .iter()
        .enumerate()
        .map(|(i, emb)| {
            doc_embs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, e)| cosine(emb, e))
                .fold(-1.0f32, f32::max)
        })
        .collect();
    let mut sorted_nnds = nnds.clone();
    sorted_nnds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_nnd = if sorted_nnds.is_empty() {
        0.0
    } else {
        sorted_nnds[sorted_nnds.len() / 2]
    };
    let n_docs = docs.len();
    let mut parent: Vec<usize> = (0..n_docs).collect();
    #[allow(clippy::ptr_arg)]
    fn find(p: &mut Vec<usize>, mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    // Repeat = a pair at least as close as the corpus's own median nearest
    // neighbour (relative, so it adapts to any embedder instead of a magic
    // constant that only fits one space).
    let repeat_sim = median_nnd - 0.02;
    for i in 0..n_docs {
        for j in i + 1..n_docs {
            if cosine(&doc_embs[i], &doc_embs[j]) >= repeat_sim {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a.max(b)] = a.min(b);
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n_docs {
        groups.entry(find(&mut parent, i)).or_default().push(i);
    }
    let mut repeats: Vec<RepeatCluster> = Vec::new();
    for (k, (_, members)) in groups
        .into_iter()
        .enumerate()
        .filter(|(_, (_, m))| m.len() >= 2)
    {
        let instances: Vec<SourceRef> = members
            .iter()
            .take(24)
            .map(|&i| SourceRef {
                path: docs[i].0.clone(),
                title: title_of(&docs[i].0),
            })
            .collect();
        // Name from the first member's title — stable and human-checkable.
        let name = instances[0].title.clone();
        let hints = top_shared_terms(&members, &texts);
        repeats.push(RepeatCluster {
            id: format!("r-{:03}", k + 1),
            name,
            hints,
            count: members.len(),
            coverage: 1.0,
            instances,
        });
    }

    // ── what differs ─────────────────────────────────────────────────────────
    // Two honest signals, both deterministic:
    // 1. grid fit: best-cell similarity below the working threshold;
    // 2. unseen combination: no near neighbour in the corpus itself
    //    (nearest-neighbour similarity far below the repeat clusters' own).
    let doc_embs: Vec<Vec<f32>> = texts.iter().map(|t| embedder.embed(t)).collect();
    let nnds: Vec<f32> = doc_embs
        .iter()
        .enumerate()
        .map(|(i, emb)| {
            doc_embs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, e)| cosine(emb, e))
                .fold(-1.0f32, f32::max)
        })
        .collect();
    // Median nearest-neighbour similarity = what "repeated" looks like here.
    let mut sorted_nnds = nnds.clone();
    sorted_nnds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_nnd = if sorted_nnds.is_empty() {
        0.0
    } else {
        sorted_nnds[sorted_nnds.len() / 2]
    };
    let margin = 0.05;
    let mut differences: Vec<DifferenceItem> = Vec::new();
    for (i, ((path, _body), _text)) in docs.iter().zip(texts.iter()).enumerate() {
        let emb = &doc_embs[i];
        let (best, d, m) = clf
            .best_entry_sim(emb)
            .unwrap_or((0.0, String::new(), String::new()));
        let separation = median_nnd - nnds[i];
        // Primary signal: loneliness vs the corpus median (works with any
        // embedder). Grid fit is reported, not used as a blanket flag — with
        // an auto-retuned threshold it would flag an entire healthy corpus.
        if separation > margin {
            differences.push(DifferenceItem {
                path: path.clone(),
                title: title_of(path),
                best_sim: best,
                separation,
                nearest_cluster: if d.is_empty() {
                    None
                } else {
                    Some(format!("{}×{}", d, m))
                },
            });
        }
    }
    differences.sort_by(|a, b| {
        b.separation
            .partial_cmp(&a.separation)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── contradictions: same cluster, overlapping words, distant vectors ────
    let mut contradictions: Vec<ContradictionPair> = Vec::new();
    for cluster in &repeats {
        let items: Vec<(usize, &(String, String))> = docs
            .iter()
            .enumerate()
            .filter(|(_, (path, _))| cluster.instances.iter().any(|s| &s.path == path))
            .collect();
        for (ii, (i_idx, (pa, ba))) in items.iter().enumerate() {
            for (j_idx, (pb, bb)) in items.iter().skip(ii + 1) {
                if pa == pb {
                    continue;
                }
                let lex = lexical_overlap(ba, bb);
                if lex < 0.5 {
                    continue;
                }
                // positions in `items` are docs indices via enumerate earlier
                let sim = cosine(&doc_embs[*i_idx], &doc_embs[*j_idx]);
                let na = negation_hits(ba);
                let nb = negation_hits(bb);
                let (la, lb) = (decimal_number(ba), decimal_number(bb));
                let number_clash = matches!((la, lb), (Some(x), Some(y)) if x != y);
                // A duplicate is NOT an agreement when the wording of a decisive
                // point is flipped: surface it instead of merging or silently
                // keeping both. (README: dedup must not hide disagreement.)
                if sim < 0.55 || (lex >= 0.5 && (na != nb || number_clash)) {
                    contradictions.push(ContradictionPair {
                        a: SourceRef {
                            path: pa.clone(),
                            title: title_of(pa),
                        },
                        b: SourceRef {
                            path: pb.clone(),
                            title: title_of(pb),
                        },
                        cluster: cluster.name.clone(),
                        lexical_overlap: lex,
                        embedding_sim: sim,
                    });
                }
            }
        }
        if contradictions.len() >= 24 {
            break; // visible tension, not an unbounded matrix
        }
    }

    // ── reproducibility hash over the structural result ─────────────────────
    let mut hasher = Sha256::new();
    for r in &repeats {
        hasher.update(r.id.as_bytes());
        hasher.update(r.name.as_bytes());
        hasher.update(r.count.to_le_bytes());
        for h in &r.hints {
            hasher.update(b"|");
            hasher.update(h.as_bytes());
        }
    }
    for d in &differences {
        hasher.update(b"d");
        hasher.update(d.path.as_bytes());
    }
    let structure_hash = format!("{:x}", hasher.finalize());

    Ok(MapReport {
        docs: docs.len(),
        repeats,
        differences,
        contradictions,
        threshold_used: threshold,
        structure_hash,
        auto_retuned: report.auto_retuned,
    })
}

/// Directories that are never corpus sources: VCS metadata, build output,
/// dependency trees and caches. They add noise and can dwarf the real corpus.
const CORPUS_SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    "vendor",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".cache",
    ".tox",
];

/// Default per-file cap (1 MiB). Override with `PHYSIS_CORPUS_MAX_BYTES`.
const DEFAULT_CORPUS_MAX_BYTES: u64 = 1024 * 1024;
/// Default corpus cap. Override with `PHYSIS_CORPUS_MAX_FILES`.
const DEFAULT_CORPUS_MAX_FILES: usize = 20_000;

/// A file is treated as binary when a NUL byte appears in its first 8 KiB.
/// That catches ELF/Mach-O/images/PDF while keeping UTF-8, UTF-16-with-BOM and
/// source files.
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|b| *b == 0)
}

/// Load a corpus directory: **any text file**, recursively, deterministic
/// order (sorted paths). Returns (path, body).
///
/// Formerly filtered to `md|txt|json`, which made the context compiler blind
/// to source code — a code question over a repository could only see its
/// README. The loader now accepts every text file and rejects binaries by
/// content, with size/count caps so a huge tree cannot stall a caller.
pub fn load_corpus(dir: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let max_bytes: u64 = std::env::var("PHYSIS_CORPUS_MAX_BYTES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_CORPUS_MAX_BYTES);
    let max_files: usize = std::env::var("PHYSIS_CORPUS_MAX_FILES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_CORPUS_MAX_FILES);

    let mut out: Vec<(String, String)> = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&d)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        entries.sort();
        for p in entries {
            if out.len() >= max_files {
                break;
            }
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !CORPUS_SKIP_DIRS.contains(&name) {
                    stack.push(p);
                }
                continue;
            }
            let Ok(meta) = std::fs::metadata(&p) else {
                continue;
            };
            if !meta.is_file() || meta.len() < 40 || meta.len() > max_bytes {
                continue;
            }
            let Ok(bytes) = std::fs::read(&p) else {
                continue;
            };
            if looks_binary(&bytes) {
                continue;
            }
            let body = String::from_utf8_lossy(&bytes).to_string();
            if body.trim().len() > 40 {
                out.push((p.display().to_string(), body));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Most frequent terms shared by a cluster's documents (ties alphabetical).
fn top_shared_terms(members: &[usize], texts: &[String]) -> Vec<String> {
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    for &i in members {
        let seen: std::collections::BTreeSet<String> = texts[i]
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3)
            .map(|w| w.to_lowercase())
            .collect();
        for w in seen {
            *freq.entry(w).or_insert(0) += 1;
        }
    }
    let mut ranked: Vec<(usize, String)> = freq.into_iter().map(|(w, c)| (c, w)).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked
        .into_iter()
        .filter(|(c, _)| *c >= 2)
        .map(|(_, w)| w)
        .take(6)
        .collect()
}

fn title_of(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Render the human report (hero block + drill-downs).
pub fn render(report: &MapReport) -> String {
    let mut out = String::new();
    for l in report.hero_lines() {
        out.push_str(&l);
        out.push('\n');
    }
    out.push_str("\nWHAT REPEATS\n");
    for r in &report.repeats {
        out.push_str(&format!(
            "  {} — {} instances — {}\n    hints: {}\n",
            r.name,
            r.count,
            r.instances
                .first()
                .map(|s| s.path.clone())
                .unwrap_or_default(),
            r.hints.join(", ")
        ));
    }
    out.push_str("\nWHAT DIFFERS\n");
    for d in report.differences.iter().take(12) {
        out.push_str(&format!(
            "  {} — best cell sim {:.2}{}\n",
            d.title,
            d.best_sim,
            d.nearest_cluster
                .as_ref()
                .map(|c| format!(" (nearest: {c})"))
                .unwrap_or_default()
        ));
    }
    out.push_str("\nCONTRADICTIONS (reported, never merged)\n");
    for c in &report.contradictions {
        out.push_str(&format!(
            "  {} ↔ {} in [{}] — lexical {:.2}, vector {:.2}\n",
            c.a.title, c.b.title, c.cluster, c.lexical_overlap, c.embedding_sim
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    fn corpus() -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = Vec::new();
        for i in 0..6 {
            v.push((
                format!("a{i}.md"),
                format!(
                    "Pump maintenance required: replace the seal on line {i}, vibration rising after bearing wear. Schedule downtime tonight."
                ),
            ));
        }
        for i in 0..5 {
            v.push((
                format!("b{i}.md"),
                format!(
                    "Invoice {i}: payment terms net 30, purchase order approved, vendor shipping confirmation received by the office."
                ),
            ));
        }
        v.push((
            "x.md".into(),
            "Quantum tessellation of the mandelbrot choir sings beneath the ice.".into(),
        ));
        v
    }

    #[test]
    fn map_is_deterministic_and_finds_repeats_and_differences() {
        let e = RandomProjectionEmbedder::new(64);
        let docs = corpus();
        let r1 = build_map(&docs, &e, None).unwrap();
        let r2 = build_map(&docs, &e, None).unwrap();
        assert_eq!(
            r1.structure_hash, r2.structure_hash,
            "same corpus, same map"
        );
        assert!(!r1.repeats.is_empty(), "repeated situations found");
        assert!(!r1.differences.is_empty(), "the outlier is found");
        // The outlier is different ⇒ it ranks first (lowest best-cell sim).
        assert!(
            r1.differences
                .first()
                .map(|d| d.path.contains("x.md"))
                .unwrap_or(false),
            "the mandelbrot outlier is the most different thing in the corpus"
        );
        assert!(
            r1.repeats.iter().all(|c| !c.instances.is_empty()),
            "provenance kept"
        );
    }

    #[test]
    fn corpus_loader_is_sorted_and_recursive() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("b.txt"),
            "later file with a body long enough to survive the corpus loader filter",
        )
        .unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(
            tmp.path().join("sub/a.md"),
            "nested file also carrying more than forty characters of body text",
        )
        .unwrap();
        let docs = load_corpus(tmp.path()).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs[0].0.ends_with("b.txt"), "sorted: b before sub/a");
    }

    #[test]
    fn corpus_loader_takes_any_text_file_and_skips_binaries() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("auth.py"),
            "def rotate_token(token):\n    return token[::-1]  # long enough body\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("lib.rs"),
            "pub fn rotate_token(token: &str) -> String { token.chars().rev().collect() }\n",
        )
        .unwrap();
        // NUL byte in the first block => binary, not corpus text.
        std::fs::write(tmp.path().join("blob.bin"), [0u8, 1, 2, 3, 0, 5, 6, 7]).unwrap();
        std::fs::create_dir(tmp.path().join("target")).unwrap();
        std::fs::write(
            tmp.path().join("target/generated.rs"),
            "fn generated() { /* build output should not enter the corpus */ }\n",
        )
        .unwrap();
        let docs = load_corpus(tmp.path()).unwrap();
        let paths: Vec<&str> = docs.iter().map(|d| d.0.as_str()).collect();
        assert!(paths.iter().any(|p| p.ends_with("auth.py")), "python included: {paths:?}");
        assert!(paths.iter().any(|p| p.ends_with("lib.rs")), "rust included: {paths:?}");
        assert!(!paths.iter().any(|p| p.ends_with("blob.bin")), "binary excluded");
        assert!(!paths.iter().any(|p| p.contains("target/")), "build dir skipped");
    }
}

// ── Demo B — PHYSIS CONTEXT COMPILER ─────────────────────────────────────────
//
// Reuses the existing `rag::TokenFixedRetriever` for the actual retrieval and
// layers the structural map on top. The compression ratio is measured from
// the real corpus, never estimated: we count the tokens a generous
// conventional retrieval would place in context and the tokens Physis admits
// under the budget, and divide.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextReport {
    pub query: String,
    pub budget: usize,
    pub source_documents: usize,
    pub baseline_tokens: usize,
    pub physis_tokens: usize,
    /// 1 − physis_tokens/baseline_tokens ∈ 0..1 (real, from this run).
    pub compression_ratio: f32,
    pub repeats: usize,
    pub differences: usize,
    pub contradictions: usize,
    pub clusters: Vec<String>,
    pub physis_context: String,
}

impl ContextReport {
    pub fn render(&self) -> String {
        let saved = (self.compression_ratio * 100.0).round();
        format!(
            "CONTEXT BUDGET {} tokens\nSOURCE DOCUMENTS {}\nSTRUCTURAL CLUSTERS {}\nREPEATED PATTERNS {}\nSIGNIFICANT DIFFERENCES {}\nCONTRADICTION CANDIDATES {}\n\nCONVENTIONAL RETRIEVAL  {} tokens (nothing discarded)\nPHYSIS COMPILED CONTEXT  {} tokens (budget-capped)\nCONTEXT REDUCED        {}%\n",
            self.budget,
            self.source_documents,
            self.clusters.len(),
            self.repeats,
            self.differences,
            self.contradictions,
            self.baseline_tokens,
            self.physis_tokens,
            saved
        )
    }
}

/// Compile a fixed-budget, structure-tagged context for `query` over `docs`.
/// Split one document into pieces that can actually fit a token budget.
///
/// Whole-file packing silently dropped every document larger than the budget:
/// a 300-token request over a source tree returned an empty context because
/// every `.rs` file was bigger than 300 tokens. Chunking keeps the file in play
/// and the `#partN` suffix preserves provenance.
fn split_document(path: &str, body: &str, max_tokens: usize) -> Vec<(String, String)> {
    let max_tokens = max_tokens.max(64);
    if crate::rag::count_tokens(body) <= max_tokens {
        return vec![(path.to_string(), body.to_string())];
    }
    let mut pieces: Vec<(String, String)> = Vec::new();
    let mut current = String::new();
    for line in body.lines() {
        let candidate = if current.is_empty() {
            line.to_string()
        } else {
            format!("{current}\n{line}")
        };
        if !current.is_empty() && crate::rag::count_tokens(&candidate) > max_tokens {
            let n = pieces.len() + 1;
            pieces.push((format!("{path}#part{n}"), std::mem::take(&mut current)));
        }
        if crate::rag::count_tokens(&line) > max_tokens {
            // A single enormous line (minified JS, one-line JSON): cut on
            // whitespace so it can still contribute.
            let mut buf = String::new();
            for word in line.split_whitespace() {
                let cand = if buf.is_empty() { word.to_string() } else { format!("{buf} {word}") };
                if !buf.is_empty() && crate::rag::count_tokens(&cand) > max_tokens {
                    let n = pieces.len() + 1;
                    pieces.push((format!("{path}#part{n}"), std::mem::take(&mut buf)));
                }
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(word);
            }
            if !buf.is_empty() {
                current = buf;
            }
        } else if current.is_empty() {
            current = line.to_string();
        } else {
            current.push('\n');
            current.push_str(line);
        }
    }
    if !current.trim().is_empty() {
        let n = pieces.len() + 1;
        pieces.push((format!("{path}#part{n}"), current));
    }
    if pieces.is_empty() {
        vec![(path.to_string(), body.to_string())]
    } else {
        pieces
    }
}

pub fn compile_context(
    docs: &[(String, String)],
    embedder: &dyn crate::embed::VectorEmbed,
    query: &str,
    budget: usize,
) -> anyhow::Result<ContextReport> {
    anyhow::ensure!(!docs.is_empty(), "no documents to compile context from");
    // Retrieval packs whole chunks, so a document must be split to fit the
    // budget or it can never be selected (the old whole-file behaviour made a
    // 300-token request over a source tree return an empty context).
    let chunked: Vec<(String, String)> = docs
        .iter()
        .flat_map(|(p, b)| split_document(p, b, budget.max(64)))
        .collect();
    let texts: Vec<String> = chunked.iter().map(|(_, b)| b.clone()).collect();

    // Baseline: a conventional retrieval that discards nothing (generous
    // budget, top documents). Its token count is the honest denominator.
    let (base_res, _) = crate::rag::retrieve_from_texts(
        &texts,
        query,
        embedder,
        usize::MAX / 2,
        chunked.len().min(12),
    );
    // Count the ASSEMBLED baseline, not the sum of its chunk tokens.
    //
    // `physis_tokens` below counts the joined context string, which carries the
    // "\n\n---\n\n" separators; `base_res.total_tokens` is a bare sum that does
    // not. Comparing them made the ratio an apples-to-oranges figure — on a
    // large corpus the budget cap dominates and it looks right, but on a
    // three-document corpus the separator overhead alone inverts it and the
    // "compressed" context measures LARGER than the baseline. Caught by a chain
    // test asserting context_tokens <= baseline_tokens.
    //
    // Both sides now count the same thing: the string a model would actually be
    // handed.
    let baseline_context = crate::rag::assemble_context(&base_res, "\n\n---\n\n");
    let baseline_tokens = crate::rag::count_tokens(&baseline_context);

    // Physis: the same retriever, but capped at the fixed budget.
    let top_k = chunked.len().clamp(3, 8);
    let (_res, physis_context) =
        crate::rag::retrieve_from_texts(&texts, query, embedder, budget.max(64), top_k);
    let physis_tokens = crate::rag::count_tokens(&physis_context);

    // Structural tag: the deterministic map over the same corpus.
    let map = build_map(docs, embedder, None)?;
    let ratio = if baseline_tokens == 0 {
        0.0
    } else {
        ((baseline_tokens.saturating_sub(physis_tokens)) as f32 / baseline_tokens as f32)
            .clamp(0.0, 1.0)
    };

    Ok(ContextReport {
        query: query.to_string(),
        budget,
        source_documents: docs.len(),
        baseline_tokens,
        physis_tokens,
        compression_ratio: ratio,
        repeats: map.repeats.len(),
        differences: map.differences.len(),
        contradictions: map.contradictions.len(),
        clusters: map.repeats.iter().map(|r| r.name.clone()).take(8).collect(),
        physis_context,
    })
}
