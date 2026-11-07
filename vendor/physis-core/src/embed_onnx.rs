//! ONNX MiniLM embedder — production-grade semantic embeddings via the `ort`
//! runtime (feature `embed-onnx`). Falls back to deterministic random
//! projection when the model is unavailable.

use std::path::Path;
use std::sync::Mutex;

use ort::value::{DynTensorValueType, Tensor};

use crate::embed::VectorEmbed;

/// Which pooling strategy the model export expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingStrategy {
    /// Use the `[CLS]` token (first token) as the sentence vector.
    Cls,
    /// Mean of all non-padded token vectors.
    Mean,
}

/// Configuration for the ONNX embedder.
#[derive(Debug, Clone)]
pub struct OnnxConfig {
    pub dim: usize,
    pub max_length: usize,
    pub model_dir: Option<String>,
    pub pooling: PoolingStrategy,
    /// ONNX Runtime intra-op thread count. `None` (the default) uses all
    /// available parallelism. Set `Some(1)` to pin single-threaded execution.
    ///
    /// Note on determinism: this knob is exposed because parallel float
    /// reduction order is a *plausible* reproducibility hazard in general — but
    /// on this model it was measured NOT to be one. A direct probe
    /// (`examples/probe_embedding_determinism.rs`) found embeddings
    /// bit-identical across runs and across thread counts. Reach for this only
    /// with evidence; don't assume threading is the cause of a reproducibility
    /// problem, which is a mistake this codebase already made once.
    pub intra_threads: Option<usize>,
}

impl Default for OnnxConfig {
    fn default() -> Self {
        Self {
            dim: 384,
            max_length: 128,
            model_dir: None,
            pooling: PoolingStrategy::Mean,
            intra_threads: None,
        }
    }
}

/// Embedder that loads an ONNX MiniLM model for semantic embeddings.
///
/// Falls back to deterministic random projection when the model or tokenizer
/// is not available at the given path.
pub struct OnnxEmbedder {
    dim: usize,
    session: Mutex<Option<ort::session::Session>>,
    tokenizer: Option<tokenizers::Tokenizer>,
    /// Directory path where model.onnx and tokenizer.json are stored.
    pub model_path: String,
    max_length: usize,
    /// Whether the model's input signature declares `token_type_ids`. BERT-family
    /// exports (MiniLM, BGE-base) do; XLM-R-family exports (BGE-M3) don't.
    wants_token_type_ids: bool,
    pooling: PoolingStrategy,
}

/// Load an ONNX session, logging the actual `ort` error on failure instead of
/// swallowing it.
fn load_onnx_session(
    model_path: &Path,
    intra_threads: Option<usize>,
) -> Option<ort::session::Session> {
    use ort::session::builder::GraphOptimizationLevel;
    let threads = intra_threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });
    let build = |opt: GraphOptimizationLevel| -> ort::Result<ort::session::Session> {
        ort::session::Session::builder()?
            .with_optimization_level(opt)?
            .with_intra_threads(threads)?
            .commit_from_file(model_path)
    };
    match build(GraphOptimizationLevel::Level3) {
        Ok(s) => Some(s),
        Err(e_opt) => {
            eprintln!(
                "warning: optimized load of {} failed ({e_opt}); retrying with optimization disabled",
                model_path.display()
            );
            match build(GraphOptimizationLevel::Disable) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!(
                        "warning: failed to load ONNX model {}: {e}",
                        model_path.display()
                    );
                    None
                }
            }
        }
    }
}

impl OnnxEmbedder {
    /// Create a new ONNX embedder with parameters from an `OnnxConfig`.
    pub fn with_config(config: &OnnxConfig) -> Self {
        let dim = config.dim;
        let max_length = config.max_length;
        let model_dir = config.model_dir.as_deref().unwrap_or("./models");
        // Accept either a flat layout (model_dir/model.onnx) or an `onnx/`
        // subdir (model_dir/onnx/model.onnx).
        let flat = Path::new(model_dir).join("model.onnx");
        let nested = Path::new(model_dir).join("onnx/model.onnx");
        let model_path = if flat.exists() { flat } else { nested };
        let tok_path = Path::new(model_dir).join("tokenizer.json");

        let session = if model_path.exists() {
            load_onnx_session(&model_path, config.intra_threads)
        } else {
            None
        };

        let tokenizer = if tok_path.exists() {
            tokenizers::Tokenizer::from_file(&tok_path).ok()
        } else {
            None
        };

        let wants_token_type_ids = session
            .as_ref()
            .map(|s| s.inputs().iter().any(|i| i.name() == "token_type_ids"))
            .unwrap_or(true);

        Self {
            dim,
            session: Mutex::new(session),
            tokenizer,
            model_path: model_dir.to_string(),
            max_length,
            wants_token_type_ids,
            pooling: config.pooling,
        }
    }

    /// Create a new ONNX embedder with default parameters (dim=384, max_length=128).
    pub fn new(model_dir: &str) -> Self {
        Self::with_config(&OnnxConfig {
            model_dir: Some(model_dir.to_string()),
            ..OnnxConfig::default()
        })
    }

    /// True when both the ONNX session and tokenizer are loaded successfully.
    pub fn is_available(&self) -> bool {
        self.session.lock().unwrap().is_some() && self.tokenizer.is_some()
    }
}

impl VectorEmbed for OnnxEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let tokenizer = match &self.tokenizer {
            Some(t) => t,
            None => return fallback_embed(text, self.dim),
        };

        let encoding = match tokenizer.encode(text, true) {
            Ok(e) => e,
            Err(_) => return fallback_embed(text, self.dim),
        };

        let actual_len = encoding.len().min(self.max_length);
        let mut ids: Vec<i64> = encoding
            .get_ids()
            .iter()
            .take(self.max_length)
            .map(|&v| v as i64)
            .collect();
        ids.resize(self.max_length, 0i64);
        let mut type_ids: Vec<i64> = encoding
            .get_type_ids()
            .iter()
            .take(self.max_length)
            .map(|&v| v as i64)
            .collect();
        type_ids.resize(self.max_length, 0i64);
        let mask: Vec<i64> = (0..self.max_length)
            .map(|i| if i < actual_len { 1i64 } else { 0i64 })
            .collect();

        let shape = vec![1i64, self.max_length as i64];
        let input_ids = Tensor::<i64>::from_array((shape.clone(), ids)).ok();
        let attn_mask = Tensor::<i64>::from_array((shape.clone(), mask)).ok();
        let tok_types = Tensor::<i64>::from_array((shape, type_ids)).ok();

        let (ids_t, mask_t, types_t) = match (input_ids, attn_mask, tok_types) {
            (Some(a), Some(b), Some(c)) => (a, b, c),
            _ => return fallback_embed(text, self.dim),
        };

        let mut inputs: Vec<(&str, ort::value::Value<ort::value::DynValueTypeMarker>)> = vec![
            ("input_ids", ids_t.into()),
            ("attention_mask", mask_t.into()),
        ];
        if self.wants_token_type_ids {
            inputs.push(("token_type_ids", types_t.into()));
        }

        let mut session_guard = self.session.lock().unwrap();
        let s = match session_guard.as_mut() {
            Some(s) => s,
            None => return fallback_embed(text, self.dim),
        };

        let outputs = match s.run(inputs) {
            Ok(o) => o,
            Err(_) => return fallback_embed(text, self.dim),
        };

        let hidden = match outputs
            .get("last_hidden_state")
            .or_else(|| outputs.get("sentence_embedding"))
            .or_else(|| (outputs.len() > 0).then(|| &outputs[0]))
        {
            Some(v) => v,
            None => return fallback_embed(text, self.dim),
        };

        let tensor_ref = match hidden.downcast_ref::<DynTensorValueType>() {
            Ok(t) => t,
            Err(_) => return fallback_embed(text, self.dim),
        };

        let view = match tensor_ref.try_extract_array::<f32>() {
            Ok(v) => v,
            Err(_) => return fallback_embed(text, self.dim),
        };

        let shape = view.shape();
        let tokens = if shape.len() >= 2 {
            shape[1]
        } else {
            return fallback_embed(text, self.dim);
        };
        let features = if shape.len() >= 3 { shape[2] } else { self.dim };
        let limit = tokens.min(actual_len);

        let mut pooled = vec![0.0f32; self.dim];
        match self.pooling {
            PoolingStrategy::Cls => {
                for d in 0..features.min(self.dim) {
                    pooled[d] = view[[0, 0, d]];
                }
            }
            PoolingStrategy::Mean => {
                let mut count = 0usize;
                for t in 0..limit {
                    count += 1;
                    for d in 0..features.min(self.dim) {
                        pooled[d] += view[[0, t, d]];
                    }
                }
                if count > 0 {
                    for v in &mut pooled {
                        *v /= count as f32;
                    }
                }
            }
        }

        let norm = pooled.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
        pooled.iter_mut().for_each(|x| *x /= norm);
        pooled
    }

    /// Batched override: tokenize all texts, run a single `[N, max_length]`
    /// session pass, then masked mean-pool each row. Falls back to per-text
    /// `embed` whenever the fast path can't apply.
    fn embed_batch(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        if texts.is_empty() {
            return vec![];
        }
        let per_text = || texts.iter().map(|t| self.embed(t)).collect::<Vec<_>>();
        let tokenizer = match &self.tokenizer {
            Some(t) => t,
            None => return per_text(),
        };

        let n = texts.len();
        let mut ids_flat: Vec<i64> = Vec::with_capacity(n * self.max_length);
        let mut mask_flat: Vec<i64> = Vec::with_capacity(n * self.max_length);
        let mut types_flat: Vec<i64> = Vec::with_capacity(n * self.max_length);
        let mut lens: Vec<usize> = Vec::with_capacity(n);

        for &text in texts {
            let encoding = match tokenizer.encode(text, true) {
                Ok(e) => e,
                Err(_) => return per_text(),
            };
            let actual_len = encoding.len().min(self.max_length);
            lens.push(actual_len);
            let mut ids: Vec<i64> = encoding
                .get_ids()
                .iter()
                .take(self.max_length)
                .map(|&v| v as i64)
                .collect();
            ids.resize(self.max_length, 0);
            let mut type_ids: Vec<i64> = encoding
                .get_type_ids()
                .iter()
                .take(self.max_length)
                .map(|&v| v as i64)
                .collect();
            type_ids.resize(self.max_length, 0);
            let mask: Vec<i64> = (0..self.max_length)
                .map(|i| if i < actual_len { 1i64 } else { 0i64 })
                .collect();
            ids_flat.extend(ids);
            mask_flat.extend(mask);
            types_flat.extend(type_ids);
        }

        let shape = vec![n as i64, self.max_length as i64];
        let (Some(ids_t), Some(mask_t), Some(types_t)) = (
            Tensor::<i64>::from_array((shape.clone(), ids_flat)).ok(),
            Tensor::<i64>::from_array((shape.clone(), mask_flat)).ok(),
            Tensor::<i64>::from_array((shape, types_flat)).ok(),
        ) else {
            return per_text();
        };
        let mut inputs: Vec<(&str, ort::value::Value<ort::value::DynValueTypeMarker>)> = vec![
            ("input_ids", ids_t.into()),
            ("attention_mask", mask_t.into()),
        ];
        if self.wants_token_type_ids {
            inputs.push(("token_type_ids", types_t.into()));
        }

        let result: Option<Vec<Vec<f32>>> = 'run: {
            let mut session_guard = self.session.lock().unwrap();
            let s = match session_guard.as_mut() {
                Some(s) => s,
                None => break 'run None,
            };
            let outputs = match s.run(inputs) {
                Ok(o) => o,
                Err(_) => break 'run None,
            };
            let hidden = match outputs
                .get("last_hidden_state")
                .or_else(|| outputs.get("sentence_embedding"))
                .or_else(|| (outputs.len() > 0).then(|| &outputs[0]))
            {
                Some(v) => v,
                None => break 'run None,
            };
            let tensor_ref = match hidden.downcast_ref::<DynTensorValueType>() {
                Ok(t) => t,
                Err(_) => break 'run None,
            };
            let view = match tensor_ref.try_extract_array::<f32>() {
                Ok(v) => v,
                Err(_) => break 'run None,
            };

            let vshape = view.shape();
            let tokens = if vshape.len() >= 2 {
                vshape[1]
            } else {
                break 'run None;
            };
            let features = if vshape.len() >= 3 {
                vshape[2]
            } else {
                self.dim
            };

            let mut out = Vec::with_capacity(n);
            for (r, &actual_len) in lens.iter().enumerate() {
                let limit = tokens.min(actual_len);
                let mut pooled = vec![0.0f32; self.dim];
                match self.pooling {
                    PoolingStrategy::Cls => {
                        for d in 0..features.min(self.dim) {
                            pooled[d] = view[[r, 0, d]];
                        }
                    }
                    PoolingStrategy::Mean => {
                        let mut count = 0usize;
                        for t in 0..limit {
                            count += 1;
                            for d in 0..features.min(self.dim) {
                                pooled[d] += view[[r, t, d]];
                            }
                        }
                        if count > 0 {
                            for v in &mut pooled {
                                *v /= count as f32;
                            }
                        }
                    }
                }
                let norm = pooled.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
                pooled.iter_mut().for_each(|x| *x /= norm);
                out.push(pooled);
            }
            Some(out)
        };
        result.unwrap_or_else(per_text)
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

/// Read `hidden_size` from a transformers `config.json` next to the model, so
/// callers never have to hard-code an output width per model family.
pub fn native_dim(model_dir: &str) -> Option<usize> {
    let cfg = std::fs::read_to_string(format!("{model_dir}/config.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&cfg).ok()?;
    v.get("hidden_size")?.as_u64().map(|h| h as usize)
}

fn fallback_embed(text: &str, dim: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; dim];
    let h: u64 = text
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(h);
    use rand::Rng;
    for x in &mut v {
        *x = rng.gen_range(-1.0..1.0);
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    v.iter_mut().for_each(|x| *x /= norm);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_unavailable_embedder() -> OnnxEmbedder {
        let dir =
            std::env::temp_dir().join(format!("physis-core-onnx-test-{}", std::process::id()));
        OnnxEmbedder::new(dir.to_str().unwrap())
    }

    #[test]
    fn not_available_without_model() {
        assert!(!temp_unavailable_embedder().is_available());
    }

    #[test]
    fn fallback_embed_dimension_and_normalized() {
        let e = temp_unavailable_embedder();
        let v = e.embed("hello");
        assert_eq!(v.len(), 384);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn fallback_embed_deterministic_and_differentiates() {
        let e = temp_unavailable_embedder();
        assert_eq!(e.embed("hello world"), e.embed("hello world"));
        assert_ne!(e.embed("foo"), e.embed("bar"));
        assert_eq!(e.dimension(), 384);
    }

    #[test]
    fn embed_batch_matches_per_text_without_model() {
        let e = temp_unavailable_embedder();
        let texts = ["alpha", "beta gamma", ""];
        let batch = e.embed_batch(&texts);
        assert_eq!(batch.len(), 3);
        for (i, t) in texts.iter().enumerate() {
            assert_eq!(batch[i], e.embed(t), "row {i}");
        }
    }
}
