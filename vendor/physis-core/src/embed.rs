//! Deterministic, dependency-light embedding. The `VectorEmbed` trait is the
//! seam where a heavier embedder (ONNX/MiniLM) can plug in later; the default
//! `RandomProjectionEmbedder` ships zero model files and is fully deterministic.

use std::collections::HashMap;
use std::sync::Mutex;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

/// Trait for embedding text into fixed-dimension vectors.
///
/// Implementors must guarantee the same input always produces the same vector
/// (determinism), and the vector must be L2-normalized (unit length).
pub trait VectorEmbed: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn embed_batch(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
    fn dimension(&self) -> usize;
}

impl<T: VectorEmbed + ?Sized> VectorEmbed for Box<T> {
    fn embed(&self, text: &str) -> Vec<f32> {
        (**self).embed(text)
    }
    fn dimension(&self) -> usize {
        (**self).dimension()
    }
}

/// Hash a text into n-gram hashes using SHA-256.
fn hash_ngrams(text: &str, n: usize) -> Vec<u64> {
    let padded = format!(" {text} ");
    let chars: Vec<char> = padded.chars().collect();
    let mut hashes = Vec::new();
    for i in 0..chars.len().saturating_sub(n - 1) {
        let gram: String = chars[i..i + n].iter().collect();
        let mut h = Sha256::new();
        h.update(gram.as_bytes());
        let result = h.finalize();
        hashes.push(u64::from_le_bytes(result[..8].try_into().unwrap()));
    }
    hashes
}

/// Generate a random unit vector of dimension `dim` seeded by `seed`.
fn random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut v = Vec::with_capacity(dim);
    for _ in 0..dim {
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        v.push(z as f32);
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    v.iter_mut().for_each(|x| *x /= norm);
    v
}

/// Deterministic random-projection embedder using feature hashing.
///
/// Maps any text to a fixed-dimension vector (default 384) using locality-sensitive
/// hashing with random projections. Same input → same vector (seed fixed at
/// construction). Cheap and model-free; cosine between related texts is meaningful
/// but coarser than a trained model.
pub struct RandomProjectionEmbedder {
    dim: usize,
    seed: u64,
    basis: Mutex<HashMap<u64, Vec<f32>>>,
}

impl RandomProjectionEmbedder {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            seed: 42,
            basis: Mutex::new(HashMap::new()),
        }
    }

    fn get_basis(&self, key: u64) -> Vec<f32> {
        let mut cache = self.basis.lock().unwrap();
        cache
            .entry(key)
            .or_insert_with(|| random_vector(self.dim, key ^ self.seed))
            .clone()
    }
}

impl VectorEmbed for RandomProjectionEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let lower = text.to_lowercase();
        let mut vec = vec![0.0f32; self.dim];

        for h in hash_ngrams(&lower, 1) {
            let basis = self.get_basis(h);
            for (i, val) in basis.iter().enumerate() {
                vec[i] += val;
            }
        }
        for h in hash_ngrams(&lower, 2) {
            let basis = self.get_basis(h ^ 0xFFFF);
            for (i, val) in basis.iter().enumerate() {
                vec[i] += val * 0.5;
            }
        }

        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
        vec.iter_mut().for_each(|x| *x /= norm);
        vec
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

/// Required margin between the related-pair similarity and the distractor-pair
/// similarity (lexically disjoint probes; see [`semantic_self_test`]).
pub const SEMANTIC_PROBE_MARGIN: f32 = 0.05;

/// (related A, related B, unrelated distractor) — A and B MUST share no token.
///
/// The original probe compared "a photograph of a dog" with "a photograph of a
/// puppy": four of five tokens shared, so any bag-of-words/feature-hashing
/// embedder passed it and a non-semantic engine reported itself semantic. These
/// disjoint pairs are the criterion instead (PLAN.md H2).
pub const SEMANTIC_PROBES: [(&str, &str, &str); 3] = [
    ("car", "automobile", "banana"),
    ("dog", "puppy", "spreadsheet"),
    (
        "spindle motor overheated and tripped",
        "thermal fault shut down the drive",
        "invoice payment terms net thirty days",
    ),
];

/// Semantic self-test: for EVERY lexically disjoint related pair, the pair must
/// clearly out-score its distractor, and all vectors must be finite. A model
/// that loads but emits garbage fails; so does any purely lexical hasher, which
/// was the whole point of the disjointness.
pub fn semantic_self_test(e: &dyn VectorEmbed) -> bool {
    let finite = |v: &[f32]| !v.is_empty() && v.iter().all(|x| x.is_finite());
    for (a_text, b_text, d_text) in SEMANTIC_PROBES {
        let a = e.embed(a_text);
        let b = e.embed(b_text);
        let d = e.embed(d_text);
        if !(finite(&a) && finite(&b) && finite(&d)) {
            return false;
        }
        let related = crate::models::cosine_sim(&a, &b);
        let unrelated = crate::models::cosine_sim(&a, &d);
        if related <= unrelated + SEMANTIC_PROBE_MARGIN {
            return false;
        }
    }
    true
}

/// Memoizes `embed` by exact text, for the lifetime of one process.
///
/// `map::compile_context` runs `retrieve_from_texts` twice and `build_map`
/// once over the SAME corpus, so a 30-document corpus issued ~800 embed calls
/// for ~30 distinct texts. At 18 ms/embed that is ~14 s of pure repetition per
/// command. This is a per-process memo, not a persistent cache: it has no disk
/// format to version and no invalidation question to get wrong.
///
/// ponytail: unbounded map, one process, corpus-sized. Add an LRU bound if a
/// long-lived server ever wraps its embedder in this.
pub struct MemoEmbedder {
    inner: Box<dyn VectorEmbed>,
    memo: std::sync::Mutex<std::collections::HashMap<String, Vec<f32>>>,
}

impl MemoEmbedder {
    pub fn new(inner: Box<dyn VectorEmbed>) -> Self {
        Self {
            inner,
            memo: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl VectorEmbed for MemoEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        if let Some(v) = self.memo.lock().unwrap().get(text) {
            return v.clone();
        }
        let v = self.inner.embed(text);
        self.memo
            .lock()
            .unwrap()
            .insert(text.to_string(), v.clone());
        v
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
}

/// Which embedder [`select`] actually chose. Returned alongside the embedder so
/// every caller can *report* what it is running on instead of assuming.
pub type Selected = (Box<dyn VectorEmbed>, &'static str);

/// Resolve an embedder, newest-honest-first, and say which one won.
///
/// This exists because the CLI and the benchmark used to call
/// `RandomProjectionEmbedder::new(384)` unconditionally: every measured number
/// in `benchmarks/results/` was produced by a **lexical hash**, while
/// `OnnxEmbedder` sat exported-but-unused behind the `embed-onnx` feature. A
/// retrieval claim made on a non-semantic embedder is not a retrieval claim.
///
/// Cascade:
/// 1. `PHYSIS_EMBEDDER=random-projection` — explicit operator override. This is
///    the supported OFFLINE mode (deterministic, model-free, coarser), not a
///    failure, so it is honoured silently.
/// 2. `PHYSIS_MODEL_DIR` if set, else `./models/bge-base-en-v1.5` (768-d), else
///    `./models` (384-d MiniLM-style flat layout). Each candidate must load AND
///    pass [`semantic_self_test`]; one that loads but emits garbage is skipped
///    rather than trusted.
/// 3. Random projection, labelled `"random-projection"` so the caller can print
///    the truth. It fails the self-test on purpose — see
///    `random_projection_fails_the_semantic_probe`.
pub fn select(dim: usize) -> Selected {
    if std::env::var("PHYSIS_EMBEDDER")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "random-projection" || v == "random_projection" || v == "rp"
        })
        .unwrap_or(false)
    {
        return (
            Box::new(RandomProjectionEmbedder::new(dim)),
            "random-projection",
        );
    }

    #[cfg(feature = "embed-onnx")]
    {
        for (dir, d, pooling, label) in onnx_candidates() {
            // 128, not 512. Measured on MiniLM: 18.3 ms/embed at 128 vs
            // 108.1 ms at 512 — a 5.9x tax paid entirely in padding, because
            // the documents this crate maps are short. Raise it only for a
            // corpus with genuinely long chunks.
            let cfg = crate::embed_onnx::OnnxConfig {
                dim: d,
                max_length: std::env::var("PHYSIS_MODEL_MAX_LEN")
                    .ok()
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(128),
                model_dir: Some(dir),
                pooling,
                intra_threads: None,
            };
            let e = crate::embed_onnx::OnnxEmbedder::with_config(&cfg);
            if e.is_available() && semantic_self_test(&e) {
                return (Box::new(MemoEmbedder::new(Box::new(e))), label);
            }
        }
    }

    (
        Box::new(RandomProjectionEmbedder::new(dim)),
        "random-projection",
    )
}

/// Candidate ONNX model directories in preference order, with the dimension and
/// pooling each export needs. `PHYSIS_MODEL_DIR` short-circuits the list.
#[cfg(feature = "embed-onnx")]
fn onnx_candidates() -> Vec<(
    String,
    usize,
    crate::embed_onnx::PoolingStrategy,
    &'static str,
)> {
    use crate::embed_onnx::PoolingStrategy::Mean;
    if let Ok(dir) = std::env::var("PHYSIS_MODEL_DIR") {
        let dir = dir.trim().to_string();
        if !dir.is_empty() {
            let dim = std::env::var("PHYSIS_MODEL_DIM")
                .ok()
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(768);
            return vec![(dir, dim, Mean, "onnx-explicit")];
        }
    }
    vec![
        ("./models/bge-base-en-v1.5".into(), 768, Mean, "bge-base"),
        ("./models".into(), 384, Mean, "minilm"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_normalized() {
        let e = RandomProjectionEmbedder::new(64);
        let v1 = e.embed("hello world");
        let v2 = e.embed("hello world");
        assert_eq!(v1, v2, "same input must produce same vector");
        let norm: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "must be unit normalized, got {norm}"
        );
        assert_eq!(e.embed("").len(), 64);
    }

    #[test]
    fn different_inputs_differ() {
        let e = RandomProjectionEmbedder::new(64);
        assert_ne!(e.embed("hello"), e.embed("world"));
    }

    #[test]
    fn batch_matches_single() {
        let e = RandomProjectionEmbedder::new(64);
        let texts = &["first", "second", "third"];
        for (i, t) in texts.iter().enumerate() {
            assert_eq!(e.embed_batch(texts)[i], e.embed(t));
        }
    }

    /// Reproduces the ORIGINAL, lax probe (dog/puppy vs a spreadsheet) exactly.
    /// Kept only so a test can demonstrate it is no longer the criterion.
    fn old_lax_probe(e: &dyn VectorEmbed) -> bool {
        let a = e.embed("a photograph of a dog");
        let b = e.embed("a photograph of a puppy");
        let d = e.embed("quarterly corporate tax accounting spreadsheet");
        crate::models::cosine_sim(&a, &b) > crate::models::cosine_sim(&a, &d) + 0.05
    }

    /// The feature-hashing embedder is non-semantic by design. It passes the old
    /// token-overlap pair — which is why `/health` reported `"semantic": true`
    /// for it — but must FAIL the disjoint probe set. PLAN.md H2.
    #[test]
    fn random_projection_fails_the_semantic_probe() {
        let e = RandomProjectionEmbedder::new(384);
        assert!(
            !semantic_self_test(&e),
            "random projection must not pass the semantic self-test"
        );
    }

    /// Regression for the exact H2 blind spot: the old dog/puppy pair was the
    /// SOLE criterion and a bag-of-grams hash passed it. Now that pair still
    /// passes, but the engine no longer trusts it — the disjoint probes fail
    /// the same embedder. If someone reverts to the single lax pair, this fails.
    #[test]
    fn old_probe_is_not_the_semantic_criterion() {
        let e = RandomProjectionEmbedder::new(384);
        assert!(
            old_lax_probe(&e),
            "the lax dog/puppy probe should still pass for a lexical hasher"
        );
        assert!(
            !semantic_self_test(&e),
            "the disjoint probe set must reject the same lexical hasher"
        );
    }

    /// The override is the one branch that must work with no weights, no
    /// network and no feature flags — it is how an operator forces the offline
    /// mode. If this regresses, `PHYSIS_EMBEDDER=random-projection` silently
    /// loads the ONNX cascade instead, which is the exact "ignored instruction"
    /// failure the cascade doc warns about.
    #[test]
    fn select_honours_the_random_projection_override() {
        // Serialised against the sibling env test by running both in one test.
        let prev = std::env::var("PHYSIS_EMBEDDER").ok();
        for v in ["random-projection", "random_projection", "RP", " rp "] {
            unsafe { std::env::set_var("PHYSIS_EMBEDDER", v) };
            let (e, kind) = select(384);
            assert_eq!(kind, "random-projection", "override {v:?} was ignored");
            assert_eq!(e.dimension(), 384);
            // And the thing it selected is the thing that fails the probe —
            // the label is not cosmetic.
            assert!(!semantic_self_test(e.as_ref()));
        }
        unsafe { std::env::remove_var("PHYSIS_EMBEDDER") };
        if let Some(p) = prev {
            unsafe { std::env::set_var("PHYSIS_EMBEDDER", p) };
        }
    }

    /// Whatever the cascade returns, the label must be honest: a
    /// `"random-projection"` label implies the probe fails, and any other label
    /// implies it passes. Without this, a future candidate that loads but emits
    /// garbage could be returned under a model name — the H2 failure shape
    /// (more confident, no more correct) rebuilt one layer up.
    #[test]
    fn select_label_always_matches_probe_outcome() {
        let prev = std::env::var("PHYSIS_EMBEDDER").ok();
        unsafe { std::env::remove_var("PHYSIS_EMBEDDER") };
        let (e, kind) = select(384);
        let passes = semantic_self_test(e.as_ref());
        if kind == "random-projection" {
            assert!(
                !passes,
                "random projection is labelled honestly but passed the probe"
            );
        } else {
            assert!(
                passes,
                "{kind} was selected without passing the semantic probe"
            );
        }
        if let Some(p) = prev {
            unsafe { std::env::set_var("PHYSIS_EMBEDDER", p) };
        }
    }

    /// Probe design contract: members of each related pair share NO token, or a
    /// bag-of-words embedder could pass by overlap again.
    #[test]
    fn semantic_probe_pairs_are_lexically_disjoint() {
        let tokenize = |s: &str| -> Vec<String> {
            s.split(|c: char| !c.is_alphanumeric())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_ascii_lowercase())
                .collect()
        };
        for (a, b, _) in SEMANTIC_PROBES {
            let (ta, tb) = (tokenize(a), tokenize(b));
            let shared: Vec<&String> = ta.iter().filter(|t| tb.contains(t)).collect();
            assert!(
                shared.is_empty(),
                "probe pair {a:?}/{b:?} shares tokens {shared:?} — a hash embedder could pass by overlap"
            );
        }
    }
}
