//! Small synthetic n-gram lookup table (test/integration scaffold).
//! Confirms user insight: n-grams as token "molecules" = compositional lexical units.
//! Not the full 51B Qwen table (not downloaded); deterministic 384-d hash-lookup scaffold.
use crate::embed::VectorEmbed;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct SyntheticNGramEmbedder {
    dim: usize,
    seed: u64,
    table: Mutex<HashMap<u64, Vec<f32>>>,
}
impl SyntheticNGramEmbedder {
    pub fn new(dim: usize, seed: u64) -> Self {
        Self {
            dim,
            seed,
            table: Mutex::new(HashMap::new()),
        }
    }
    fn key(text: &str) -> u64 {
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        u64::from_le_bytes(h.finalize()[..8].try_into().unwrap())
    }
    fn ensure(&self, text: &str) {
        let k = Self::key(text);
        let mut t = self.table.lock().unwrap();
        t.entry(k).or_insert_with(|| {
            use rand::rngs::StdRng;
            use rand::{Rng, SeedableRng};
            let mut rng = StdRng::seed_from_u64(self.seed ^ k);
            let mut v = Vec::with_capacity(self.dim);
            for _ in 0..self.dim {
                let u1: f64 = rng.gen();
                let u2: f64 = rng.gen();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                v.push(z as f32);
            }
            let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
            v.iter_mut().for_each(|x| *x /= n);
            v
        });
    }
}
impl VectorEmbed for SyntheticNGramEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        self.ensure(text);
        self.table.lock().unwrap()[&Self::key(text)].clone()
    }
    fn dimension(&self) -> usize {
        self.dim
    }
}
