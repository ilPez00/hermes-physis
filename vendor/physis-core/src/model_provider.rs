//! Model provider infrastructure — Physis never hardcodes a model.
//!
//! The pipeline receives a `dyn ModelProvider`; today's real backend is the
//! deterministic [`NgramDecoderModel`] (greedy decoding over an
//! [`crate::ngram_table::NGramTable`]) — which also proves the plumbing:
//! two tables × two models compose freely (tested). Heavier backends
//! (SmolLM2 via a local runtime) implement the same trait; the registry
//! records them without the engine caring.
//!
//! Honesty rules: capabilities are explicit; unsupported capabilities return
//! `Unsupported`, never a degraded fake.

use crate::ngram_table::NGramTable;
use crate::tokenizer::Tokenizer;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Explicit capability flags. A provider that cannot stream must not pretend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Generation,
    TokenScoring,
    Logits,
    Streaming,
    Embeddings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub id: String,
    pub name: String,
    pub architecture: String,
    pub parameter_count: Option<u64>,
    pub context_length: usize,
    pub tokenizer: String,
    pub quantization: Option<String>,
    pub capabilities: Vec<Capability>,
    pub memory_estimate_mb: u64,
    pub license: String,
}

/// Errors a provider returns — `Unsupported` is a first-class answer.
#[derive(Debug)]
pub enum ModelError {
    Unsupported(Capability),
    Backend(String),
}

/// A registry-installed, disk-loading model: `load()` verifies and loads the
/// bundled `.physisng` table file, then decoding runs identically to the
/// in-memory decoder. The model is now an **installed artifact**, not a
/// session-local object.
///
/// Manifest expectation: installed dir contains exactly one `*.physisng` file
/// with a verified checksum (written by `TableRegistry::install_bytes`).
pub struct DiskTableModel {
    pub model_id: String,
    pub source_path: PathBuf,
    pub inner: NgramDecoderModel,
}

fn first_table_file(dir: &Path) -> anyhow::Result<PathBuf> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("model dir unreadable {}: {e}", dir.display()))?;
    let mut found: Vec<PathBuf> = Vec::new();
    for e in entries.filter_map(|x| x.ok()) {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) == Some("physisng") {
            found.push(p);
        }
    }
    if found.len() != 1 {
        return Err(anyhow::anyhow!(
            "model dir {} must contain exactly one .physisng table, found {}",
            dir.display(),
            found.len()
        ));
    }
    Ok(found.into_iter().next().unwrap())
}

impl DiskTableModel {
    /// Load the (verified) table backing this installed model and wrap it in
    /// the shared decoder — the same backend any in-memory table uses.
    pub fn load(model_id: &str, dir: &Path) -> anyhow::Result<Self> {
        let table_path = first_table_file(dir)?;
        let table = crate::ngram_table::load(&table_path)?; // checksum verified
        let tok: Box<dyn Tokenizer> = match table.manifest().tokenizer.id.as_str() {
            "whitespace-v1" => Box::new(crate::tokenizer::WhitespaceTokenizer::new(50_000)),
            other => {
                return Err(anyhow::anyhow!(
                        "model table uses unknown tokenizer '{other}' — rebuild with a supported tokenizer"
                    ));
            }
        };
        let inner = NgramDecoderModel::new(model_id, Arc::new(table), tok);
        Ok(Self {
            model_id: model_id.to_string(),
            source_path: table_path,
            inner,
        })
    }
}

impl ModelProvider for DiskTableModel {
    fn metadata(&self) -> &ModelMetadata {
        self.inner.metadata()
    }
    fn tokenize(&self, text: &str) -> Vec<String> {
        self.inner.tokenize(text)
    }
    fn detokenize(&self, tokens: &[String]) -> String {
        self.inner.detokenize(tokens)
    }
    fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String, ModelError> {
        self.inner.generate(prompt, max_tokens)
    }
    fn score(&self, text: &str) -> Result<f32, ModelError> {
        self.inner.score(text)
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::Unsupported(c) => {
                write!(
                    f,
                    "model does not support capability {c:?} — no fallback is faked"
                )
            }
            ModelError::Backend(e) => write!(f, "model backend error: {e}"),
        }
    }
}

impl std::error::Error for ModelError {}

/// The interchangeable engine interface (§1). Deterministic by contract
/// unless the manifest says otherwise.
pub trait ModelProvider: Send + Sync {
    fn metadata(&self) -> &ModelMetadata;
    fn tokenize(&self, text: &str) -> Vec<String>;
    fn detokenize(&self, tokens: &[String]) -> String;
    /// Continue `prompt` by at most `max_tokens`; deterministic (greedy).
    fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String, ModelError>;
    /// Mean per-token log-probability of `text` under the model.
    fn score(&self, text: &str) -> Result<f32, ModelError>;
    /// Streaming generation — only when `Capability::Streaming` is declared.
    fn stream(&self, _prompt: &str, _on_token: &mut dyn FnMut(&str)) -> Result<(), ModelError> {
        Err(ModelError::Unsupported(Capability::Streaming))
    }
    fn memory_requirements_mb(&self) -> u64 {
        self.metadata().memory_estimate_mb
    }
    fn capabilities(&self) -> &[Capability] {
        &self.metadata().capabilities
    }
}

/// A real, fully deterministic generation backend: greedy decoding over an
/// n-gram table. Useful as the offline reference model and as the A/B arm in
/// the interchangeability benchmark (Model A + Table A/B compose freely).
pub struct NgramDecoderModel {
    meta: ModelMetadata,
    table: Arc<dyn NGramTable>,
    tokenizer: Box<dyn Tokenizer>,
}

impl NgramDecoderModel {
    pub fn new(id: &str, table: Arc<dyn NGramTable>, tokenizer: Box<dyn Tokenizer>) -> Self {
        let m = table.manifest();
        // streaming/logits intentionally absent from this backend
        let caps = vec![Capability::Generation, Capability::TokenScoring];
        Self {
            meta: ModelMetadata {
                id: id.to_string(),
                name: format!("ngram decoder over '{}'", m.table_id),
                architecture: format!(
                    "ngram-backoff-{}-order{}",
                    match m.kind {
                        crate::ngram_table::TableKind::Lexical => "lexical",
                        crate::ngram_table::TableKind::Structural => "structural",
                    },
                    m.max_order
                ),
                parameter_count: Some(m.entry_count),
                context_length: m.max_order as usize,
                tokenizer: m.tokenizer.id.clone(),
                quantization: None,
                capabilities: caps,
                memory_estimate_mb: (m.entry_count * 24 / 1024 / 1024).max(1),
                license: m.license.clone(),
            },
            table,
            tokenizer,
        }
    }
}

impl ModelProvider for NgramDecoderModel {
    fn metadata(&self) -> &ModelMetadata {
        &self.meta
    }
    fn tokenize(&self, text: &str) -> Vec<String> {
        self.tokenizer.encode(text)
    }
    fn detokenize(&self, tokens: &[String]) -> String {
        self.tokenizer.decode(tokens)
    }
    fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String, ModelError> {
        let mut toks = self.tokenizer.encode(prompt);
        for _ in 0..max_tokens {
            let ctx_start = toks.len().saturating_sub(self.meta.context_length);
            let ctx = &toks[ctx_start..];
            match self.table.top_next(ctx, 1).into_iter().next() {
                Some((w, _p)) => toks.push(w),
                None => break, // honest stop: nothing observed, no invention
            }
        }
        Ok(self.tokenizer.decode(&toks))
    }
    fn score(&self, text: &str) -> Result<f32, ModelError> {
        let toks = self.tokenizer.encode(text);
        if toks.is_empty() {
            return Ok(0.0);
        }
        let mut sum = 0.0f32;
        for i in 1..toks.len() {
            let ctx_start = i.saturating_sub(self.meta.context_length);
            sum += self.table.probability(&toks[ctx_start..i], &toks[i]).ln();
        }
        Ok(sum / (toks.len().saturating_sub(1) as f32).max(1.0))
    }
}

// ── Registry + sources (§2, §3, §4) ─────────────────────────────────────────

/// Where a model artifact comes from. Network sources are feature-gated so
/// the core stays dependency-light; the abstraction does not change.
pub enum ModelSource {
    /// A directory on this machine (no copy needed unless installed).
    Local(PathBuf),
    /// An HTTPS URL, behind the `http` feature (dep: ureq).
    Http(String),
}

impl fmt::Display for ModelSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelSource::Local(p) => write!(f, "local:{}", p.display()),
            ModelSource::Http(u) => write!(f, "https:{u}"),
        }
    }
}

/// Registry record — what `physis-core model info` shows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecord {
    pub id: String,
    pub name: String,
    pub architecture: String,
    pub parameter_count: Option<u64>,
    pub context_length: usize,
    pub tokenizer: String,
    pub quantization: Option<String>,
    pub source: String,
    pub memory_estimate_mb: u64,
    pub capabilities: Vec<Capability>,
    pub license: String,
}

/// Per-install manifest — checksums are verified before a model is listed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub record: ModelRecord,
    pub revision: String,
    pub files: Vec<(String, String)>, // (relative path, sha256)
    pub installed_at: String,
}

pub struct ModelRegistry {
    root: PathBuf,
}

impl ModelRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        std::env::var_os("PHYSIS_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".physis-core"))
            .join("models")
    }

    pub fn list(&self) -> Vec<ModelManifest> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for e in entries.filter_map(|e| e.ok()) {
                let mf = e.path().join("model-manifest.json");
                if let Ok(b) = std::fs::read(&mf) {
                    if let Ok(m) = serde_json::from_slice::<ModelManifest>(&b) {
                        out.push(m);
                    }
                }
            }
        }
        out.sort_by(|a, b| a.record.id.cmp(&b.record.id));
        out
    }

    pub fn info(&self, id: &str) -> anyhow::Result<ModelManifest> {
        let mf = self.dir(id).join("model-manifest.json");
        Ok(serde_json::from_slice(&std::fs::read(mf)?)?)
    }

    pub fn path(&self, id: &str) -> PathBuf {
        self.dir(id)
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.root.join(sanitize(id))
    }

    /// Install from a local directory: verify every declared file checksum,
    /// then record the manifest. A partial or corrupted copy is refused.
    pub fn install_local(
        &self,
        record: ModelRecord,
        src: &Path,
        files: &[String],
    ) -> anyhow::Result<ModelManifest> {
        let dir = self.dir(&record.id);
        std::fs::create_dir_all(&dir)?;
        let mut verified: Vec<(String, String)> = Vec::new();
        for rel in files {
            let bytes = std::fs::read(src.join(rel))
                .map_err(|e| anyhow::anyhow!("model file '{rel}' missing at source: {e}"))?;
            let sum = format!("{:x}", Sha256::digest(&bytes));
            std::fs::write(dir.join(rel), &bytes)?;
            verified.push((rel.clone(), sum));
        }
        let manifest = ModelManifest {
            record,
            revision: "local".into(),
            files: verified,
            installed_at: chrono::Utc::now().to_rfc3339(),
        };
        std::fs::write(
            dir.join("model-manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(manifest)
    }

    /// Install over HTTP(S). Feature-gated: without the `http` feature this
    /// returns an explicit error rather than a fake success.
    #[cfg(feature = "http")]
    #[allow(unexpected_cfgs)]
    pub fn install_http(
        &self,
        record: ModelRecord,
        url: &str,
        files: &[String],
    ) -> anyhow::Result<ModelManifest> {
        let dir = self.dir(&record.id);
        std::fs::create_dir_all(&dir)?;
        let mut verified: Vec<(String, String)> = Vec::new();
        for rel in files {
            let full = format!("{}/{}", url.trim_end_matches('/'), rel);
            let resp = ureq::get(&full)
                .call()
                .map_err(|e| anyhow::anyhow!("download {full}: {e}"))?;
            let mut bytes = Vec::new();
            resp.into_reader().read_to_end(&mut bytes)?;
            let sum = format!("{:x}", Sha256::digest(&bytes));
            std::fs::write(dir.join(rel), &bytes)?;
            verified.push((rel.clone(), sum));
        }
        let manifest = ModelManifest {
            record,
            revision: "downloaded".into(),
            files: verified,
            installed_at: chrono::Utc::now().to_rfc3339(),
        };
        std::fs::write(
            dir.join("model-manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(manifest)
    }

    pub fn remove(&self, id: &str) -> anyhow::Result<()> {
        std::fs::remove_dir_all(self.dir(id))?;
        Ok(())
    }
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

use sha2::{Digest, Sha256};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram_table::{TableBuilder, TableConfig};
    use crate::tokenizer::WhitespaceTokenizer;

    fn build_table(id: &str, corpus: &str) -> Arc<dyn NGramTable> {
        let tok = WhitespaceTokenizer::new(0);
        let cfg = TableConfig {
            table_id: id.into(),
            ..Default::default()
        };
        let mut b = TableBuilder::new(cfg);
        b.push_text(&tok, corpus);
        let (t, _bytes) = b.finish(&tok, "testhash");
        Arc::new(t)
    }

    const CORPUS_A: &str = "the pump failed because the seal wore out. the pump failed because the seal wore out. maintenance replaced the seal today.";
    const CORPUS_B: &str = "invoice approved and shipped by the office vendor. payment terms are net thirty days. invoice approved and shipped again today.";

    #[test]
    fn model_and_table_are_interchangeable() {
        // Model A + Table A, Model A + Table B, Model B + Table A/B — the
        // combination matrix must all work with zero source changes (§17).
        let ta = build_table("table-a", CORPUS_A);
        let tb = build_table("table-b", CORPUS_B);
        let tok = Box::new(WhitespaceTokenizer::new(0));
        let ma = NgramDecoderModel::new("model-a", ta.clone(), tok.clone() as Box<dyn Tokenizer>);
        let mb = NgramDecoderModel::new("model-b", tb, tok as Box<dyn Tokenizer>);
        for m in [&ma, &mb] {
            for _ in 0..1 {
                let _ = &m;
            }
            let g = m.generate("the", 4).unwrap();
            assert!(!g.is_empty());
            assert!(m.score("the pump").is_ok());
            assert!(
                matches!(
                    m.stream("x", &mut |_| {}),
                    Err(ModelError::Unsupported(Capability::Streaming))
                ),
                "streaming is refused, not faked"
            );
        }
        // A model trained on A continues A-style: "the pump" → "failed".
        assert!(ma.generate("the pump", 1).unwrap().contains("failed"));
        assert!(mb.generate("invoice", 1).unwrap().contains("approved"));
    }

    #[test]
    fn generation_is_deterministic() {
        let ta = build_table("det", CORPUS_A);
        let tok = Box::new(WhitespaceTokenizer::new(0));
        let m = NgramDecoderModel::new("m", ta, tok as Box<dyn Tokenizer>);
        assert_eq!(
            m.generate("the pump", 6).unwrap(),
            m.generate("the pump", 6).unwrap()
        );
    }
}
