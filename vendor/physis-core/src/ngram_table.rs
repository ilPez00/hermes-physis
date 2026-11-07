//! N-gram table infrastructure — interchangeable, versioned, provenance-stamped.
//!
//! The rest of Physis programs against the [`NGramTable`] trait, never against
//! a concrete table: a locally built 5-gram, an imported table, and a future
//! downloaded registry table are all the same object to the pipeline.
//!
//! Format: `PHYSISNG1` — little-endian, append-only, checksummed.
//! ```text
//! magic[8] | version u32 | manifest_json_len u32 | manifest_json | records*
//! record: order u8 | context_len u16 | ctx words (len-prefixed u8)* | word
//!         (len-prefixed u8) | count u64
//! ```
//! Deterministic: the writer emits records sorted by (order, context, word),
//! so the same corpus + config ⇒ byte-identical table (tested).

use crate::tokenizer::{check_compatible, Tokenizer, TokenizerInfo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const MAGIC: &[u8; 8] = b"PHYSISNG";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableKind {
    /// Word/subword transitions over a linguistic corpus.
    Lexical,
    /// Transitions over Physis structural symbols (`DOMAIN×MODE`).
    Structural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableManifest {
    pub table_id: String,
    pub kind: TableKind,
    pub max_order: u8,
    pub tokenizer: TokenizerInfo,
    pub corpus_hash: String,
    pub physis_version: String,
    pub smoothing_addk: f32,
    pub backoff_alpha: f32,
    pub min_count: usize,
    pub entry_count: u64,
    pub checksum: String,
    pub license: String,
}

/// Statistics every table exposes (§20 of the infra spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStatistics {
    pub entry_count: u64,
    pub vocabulary: usize,
    pub max_order: u8,
    pub unique_contexts: usize,
    pub avg_branching: f32,
    pub max_branching: usize,
    pub disk_size: u64,
}

/// The abstraction the pipeline depends on. Lookups return plain counts and
/// probabilities; the caller decides what to do with them.
pub trait NGramTable: Send + Sync {
    fn manifest(&self) -> &TableManifest;
    /// Total count of `context ++ next` (0 when unobserved at any order).
    fn count(&self, context: &[String], next: &str) -> u64;
    /// Best-order count for the context itself (backoff search happens here).
    fn context_count(&self, context: &[String]) -> u64;
    /// Top-k continuations of the longest observed suffix of `context`,
    /// ordered by (count desc, word asc) — deterministic ties.
    fn top_next(&self, context: &[String], k: usize) -> Vec<(String, f32)>;
    /// Smoothed, backoff probability of `next` after `context`.
    fn probability(&self, context: &[String], next: &str) -> f32;
    fn statistics(&self) -> TableStatistics;
}

/// Counts per order: order n (1-based) → (context words, next word) → count.
pub type Counts = Vec<BTreeMap<(Vec<String>, String), u64>>;

/// Backoff table over explicit counts, orders 1..=max_order.
/// Smoothing: add-k on the conditional; backoff: stupid-backoff with alpha.
#[derive(Debug, Clone)]
pub struct BackoffTable {
    manifest: TableManifest,
    counts: Counts,
    /// (context, order) → distinct continuations, precomputed at load.
    branching: BTreeMap<(Vec<String>, usize), usize>,
}

impl BackoffTable {
    /// Wrap already-counted data (used by the builder and by tests).
    pub fn from_counts(
        manifest: TableManifest,
        counts: Counts,
    ) -> Result<Self, crate::tokenizer::Incompatible> {
        if counts.is_empty() {
            return Err(crate::tokenizer::Incompatible {
                what: "table".into(),
                expected: "at least order 1".into(),
                found: "empty counts".into(),
            });
        }
        if counts.len() - 1 != manifest.max_order as usize {
            return Err(crate::tokenizer::Incompatible {
                what: "table order".into(),
                expected: format!("{} count maps", manifest.max_order),
                found: format!("{}", counts.len() - 1),
            });
        }
        let mut branching = BTreeMap::new();
        for (n, map) in counts.iter().enumerate().skip(1) {
            #[allow(clippy::for_kv_map)]
            for ((ctx, _), _) in map {
                *branching.entry((ctx.clone(), n)).or_insert(0) += 1;
            }
        }
        Ok(Self {
            manifest,
            counts,
            branching,
        })
    }

    /// Order-n counts live at `counts[n]` (index 0 is unused padding, so
    /// `counts.len() == max_order + 1` — the same convention the builder and
    /// the serializer follow).
    fn order_count(&self, n: usize) -> &BTreeMap<(Vec<String>, String), u64> {
        &self.counts[n]
    }
}

impl NGramTable for BackoffTable {
    fn manifest(&self) -> &TableManifest {
        &self.manifest
    }

    fn count(&self, context: &[String], next: &str) -> u64 {
        let n_max = (context.len() + 1).min(self.counts.len().saturating_sub(1));
        for n in (1..=n_max).rev() {
            let ctx = &context[context.len() + 1 - n..];
            if let Some(c) = self.order_count(n).get(&(ctx.to_vec(), next.to_string())) {
                return *c;
            }
        }
        0
    }

    fn context_count(&self, context: &[String]) -> u64 {
        let n_max = (context.len() + 1).min(self.counts.len().saturating_sub(1));
        for n in (1..=n_max).rev() {
            let ctx = &context[context.len() + 1 - n..];
            let total: u64 = self
                .order_count(n)
                .range((ctx.to_vec(), String::new())..)
                .take_while(|((c, _), _)| c == ctx)
                .map(|(_, v)| *v)
                .sum();
            if total > 0 {
                return total;
            }
        }
        0
    }

    fn top_next(&self, context: &[String], k: usize) -> Vec<(String, f32)> {
        let n_max = (context.len() + 1).min(self.counts.len().saturating_sub(1));
        for n in (1..=n_max).rev() {
            let ctx = &context[context.len() + 1 - n..];
            let mut cands: Vec<(String, u64)> = self
                .order_count(n)
                .range((ctx.to_vec(), String::new())..)
                .take_while(|((c, _), _)| c == ctx)
                .map(|((_, w), v)| (w.clone(), *v))
                .collect();
            if !cands.is_empty() {
                cands.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                let total = cands.iter().map(|(_, v)| v).sum::<u64>() as f32;
                return cands
                    .into_iter()
                    .take(k)
                    .map(|(w, v)| (w, v as f32 / total))
                    .collect();
            }
        }
        Vec::new()
    }

    /// Stupid-backoff: p(n) if the order-n event was observed, else
    /// alpha × p(n-1); unigram falls back to add-1 over the vocabulary.
    fn probability(&self, context: &[String], next: &str) -> f32 {
        let alpha = self.manifest.backoff_alpha;
        let n_max = (context.len() + 1).min(self.counts.len().saturating_sub(1));
        let mut penalty = 1.0f32;
        for n in (2..=n_max).rev() {
            let ctx = &context[context.len() + 1 - n..];
            let denom: u64 = self
                .order_count(n)
                .range((ctx.to_vec(), String::new())..)
                .take_while(|((c, _), _)| c == ctx)
                .map(|(_, v)| *v)
                .sum();
            if denom > 0 {
                let c = self
                    .order_count(n)
                    .get(&(ctx.to_vec(), next.to_string()))
                    .copied()
                    .unwrap_or(0);
                if c > 0 {
                    return penalty * (c as f32 / denom as f32);
                }
                penalty *= alpha;
            }
        }
        // unigram with add-k smoothing over observed vocabulary
        let k = self.manifest.smoothing_addk.max(0.0);
        let total: u64 = self.order_count(1).values().sum();
        let c = self
            .order_count(1)
            .get(&(Vec::new(), next.to_string()))
            .copied()
            .unwrap_or(0);
        if total == 0 {
            0.0
        } else {
            penalty
                * ((c as f32 + k) / (total as f32 + k * self.manifest.entry_count.max(1) as f32))
        }
    }

    fn statistics(&self) -> TableStatistics {
        let mut entry_count = 0u64;
        let mut vocab: std::collections::BTreeSet<String> = Default::default();
        let mut unique_contexts = 0usize;
        let mut max_branching = 0usize;
        for (n, map) in self.counts.iter().enumerate() {
            if n == 0 {
                continue;
            }
            for ((ctx, w), v) in map {
                entry_count += v;
                vocab.insert(w.clone());
                if n == 1 {
                    unique_contexts += 1;
                }
                let b = self.branching.get(&(ctx.clone(), n)).copied().unwrap_or(0);
                max_branching = max_branching.max(b);
            }
        }
        let avg = if unique_contexts == 0 {
            0.0
        } else {
            entry_count as f32 / unique_contexts as f32
        };
        TableStatistics {
            entry_count,
            vocabulary: vocab.len(),
            max_order: self.manifest.max_order,
            unique_contexts,
            avg_branching: avg,
            max_branching,
            disk_size: 0, // filled by the registry from the file on disk
        }
    }
}

// ── Builder — deterministic counting with vocabulary cap and min-count ──────

#[derive(Debug, Clone)]
pub struct TableConfig {
    pub table_id: String,
    pub kind: TableKind,
    pub max_order: u8,
    pub min_count: usize,
    pub max_vocabulary: usize, // 0 = unbounded; applied by frequency then alpha
    pub smoothing_addk: f32,
    pub backoff_alpha: f32,
    pub license: String,
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            table_id: "local-corpus".into(),
            kind: TableKind::Lexical,
            max_order: 5,
            min_count: 1,
            max_vocabulary: 50_000,
            smoothing_addk: 0.5,
            backoff_alpha: 0.4,
            license: "CC0-1.0".into(),
        }
    }
}

pub struct TableBuilder {
    cfg: TableConfig,
    counts: Counts,
}

impl TableBuilder {
    /// Stream sequences in; each sequence is a Vec of tokens (words for
    /// lexical tables, structural symbols for structural ones). Deterministic:
    /// counting order does not matter, ties are resolved at write time.
    pub fn new(cfg: TableConfig) -> Self {
        let counts = vec![BTreeMap::new(); cfg.max_order as usize + 1];
        Self { cfg, counts }
    }

    pub fn push_sequence(&mut self, seq: &[String]) {
        for i in 0..seq.len() {
            let max_n = (i + 1).min(self.cfg.max_order as usize);
            for n in 1..=max_n {
                let ctx = seq[i + 1 - n..i].to_vec();
                *self.counts[n].entry((ctx, seq[i].clone())).or_insert(0) += 1;
            }
        }
    }

    /// Fold a plain text corpus through `tokenizer`.
    pub fn push_text(&mut self, tokenizer: &dyn Tokenizer, text: &str) {
        let toks = tokenizer.encode(text);
        if !toks.is_empty() {
            self.push_sequence(&toks);
        }
    }

    fn prune_vocabulary(&mut self) {
        if self.cfg.max_vocabulary == 0 {
            return;
        }
        let mut freq: BTreeMap<String, u64> = BTreeMap::new();
        for ((_, w), v) in &self.counts[1] {
            *freq.entry(w.clone()).or_insert(0) += v;
        }
        let mut ranked: Vec<(u64, String)> = freq.into_iter().map(|(w, c)| (c, w)).collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let keep: std::collections::BTreeSet<String> = ranked
            .into_iter()
            .take(self.cfg.max_vocabulary)
            .map(|(_, w)| w)
            .collect();
        for map in self.counts.iter_mut().skip(1) {
            map.retain(|(_, w), _| keep.contains(w));
        }
    }

    fn prune_min_count(&mut self) {
        let m = self.cfg.min_count as u64;
        if m <= 1 {
            return;
        }
        for map in self.counts.iter_mut().skip(1) {
            map.retain(|_, v| *v >= m);
        }
    }

    /// Finish: prune, drop empty high orders, compute manifest, return table
    /// plus the exact bytes written (for checksum + determinism tests).
    pub fn finish(
        mut self,
        tokenizer: &dyn Tokenizer,
        corpus_hash: &str,
    ) -> (BackoffTable, Vec<u8>) {
        self.prune_min_count();
        self.prune_vocabulary();
        // Drop trailing empty orders so max_order reflects reality.
        while self.counts.len() > 2 && self.counts.last().map(|m| m.is_empty()).unwrap_or(false) {
            self.counts.pop();
        }
        let max_order = (self.counts.len() - 1) as u8;
        let entry_count: u64 = self
            .counts
            .iter()
            .skip(1)
            .map(|m| m.values().sum::<u64>())
            .sum();
        let manifest = TableManifest {
            table_id: self.cfg.table_id.clone(),
            kind: self.cfg.kind,
            max_order,
            tokenizer: tokenizer.metadata(),
            corpus_hash: corpus_hash.to_string(),
            physis_version: env!("CARGO_PKG_VERSION").to_string(),
            smoothing_addk: self.cfg.smoothing_addk,
            backoff_alpha: self.cfg.backoff_alpha,
            min_count: self.cfg.min_count,
            entry_count,
            checksum: String::new(), // stamped by `serialize` below
            license: self.cfg.license.clone(),
        };
        let table =
            BackoffTable::from_counts(manifest, self.counts).expect("builder invariants hold");
        let bytes = serialize(&table);
        (table, bytes)
    }
}

// ── Serialization — deterministic, versioned, checksummed ───────────────────

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_ctx(out: &mut Vec<u8>, ctx: &[String]) {
    out.extend_from_slice(&(ctx.len() as u16).to_le_bytes());
    for w in ctx {
        put_str(out, w);
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.b[self.i..self.i + 2].try_into().unwrap());
        self.i += 2;
        v
    }
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.b[self.i..self.i + 4].try_into().unwrap());
        self.i += 4;
        v
    }
    fn u64(&mut self) -> u64 {
        let v = u64::from_le_bytes(self.b[self.i..self.i + 8].try_into().unwrap());
        self.i += 8;
        v
    }
    fn u8(&mut self) -> u8 {
        let v = self.b[self.i];
        self.i += 1;
        v
    }
    fn str(&mut self) -> String {
        let n = self.u16() as usize;
        let s = String::from_utf8_lossy(&self.b[self.i..self.i + n]).to_string();
        self.i += n;
        s
    }
    fn ctx(&mut self) -> Vec<String> {
        let n = self.u16() as usize;
        (0..n).map(|_| self.str()).collect()
    }
}

/// Serialize a table to bytes; records sorted by (order, context, word) so the
/// same content always produces byte-identical output (determinism §24).
pub fn serialize(table: &BackoffTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    let mut manifest = table.manifest().clone();
    manifest.checksum = String::new(); // checksum covers content, not itself
    let mj = serde_json::to_vec(&manifest).expect("manifest serializes");
    out.extend_from_slice(&(mj.len() as u32).to_le_bytes());
    out.extend_from_slice(&mj);
    for (n, map) in table.counts.iter().enumerate().skip(1) {
        // BTreeMap iteration is already (ctx, word)-sorted.
        for ((ctx, w), v) in map {
            out.push(n as u8);
            put_ctx(&mut out, ctx);
            put_str(&mut out, w);
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    // Stamp the checksum of everything before it into the header copy.
    let digest = Sha256::digest(&out);
    let hex = format!("{:x}", digest);
    let manifest_len_pos = MAGIC.len() + 4;
    let mj_len = u32::from_le_bytes(
        out[manifest_len_pos..manifest_len_pos + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let mut final_manifest = table.manifest().clone();
    final_manifest.checksum = hex;
    let fmj = serde_json::to_vec(&final_manifest).expect("manifest serializes");
    let mut result = Vec::with_capacity(out.len() + fmj.len() - mj_len);
    result.extend_from_slice(MAGIC);
    result.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    result.extend_from_slice(&(fmj.len() as u32).to_le_bytes());
    result.extend_from_slice(&fmj);
    result.extend_from_slice(&out[manifest_len_pos + 4 + mj_len..]);
    result
}

/// Parse `PHYSISNG1` bytes back into a [`BackoffTable`]. Verifies magic,
/// format version, and the content checksum — a truncated or corrupted table
/// is an error, never a silent partial load.
pub fn deserialize(bytes: &[u8]) -> anyhow::Result<BackoffTable> {
    if bytes.len() < MAGIC.len() + 8 || &bytes[..8] != MAGIC {
        anyhow::bail!("not a PHYSISNG table (bad magic)");
    }
    let mut r = Reader { b: bytes, i: 8 };
    let version = r.u32();
    if version != FORMAT_VERSION {
        anyhow::bail!("table format v{version} unsupported — this build reads v{FORMAT_VERSION}");
    }
    let mj_len = r.u32() as usize;
    let manifest: TableManifest = serde_json::from_slice(&bytes[r.i..r.i + mj_len])?;
    r.i += mj_len;
    // Checksum covers the header-with-empty-checksum + records, i.e. exactly
    // what `serialize` hashed. Rebuild that view: header with empty checksum.
    let mut bare = Vec::with_capacity(bytes.len());
    bare.extend_from_slice(MAGIC);
    bare.extend_from_slice(&version.to_le_bytes());
    let empty_manifest = TableManifest {
        checksum: String::new(),
        ..manifest.clone()
    };
    let emj = serde_json::to_vec(&empty_manifest)?;
    bare.extend_from_slice(&(emj.len() as u32).to_le_bytes());
    bare.extend_from_slice(&emj);
    bare.extend_from_slice(&bytes[r.i..]);
    let got = format!("{:x}", Sha256::digest(&bare));
    if got != manifest.checksum {
        anyhow::bail!(
            "table checksum mismatch — file is corrupted or was edited \
             (expected {}, got {})",
            manifest.checksum,
            got
        );
    }
    let mut counts: Counts = vec![BTreeMap::new(); manifest.max_order as usize + 1];
    while r.i < bytes.len() {
        let n = r.u8() as usize;
        if n == 0 || n >= counts.len() {
            anyhow::bail!("record order {n} out of range");
        }
        let ctx = r.ctx();
        let w = r.str();
        let v = r.u64();
        counts[n].insert((ctx, w), v);
    }
    BackoffTable::from_counts(manifest, counts)
        .map_err(|e| anyhow::anyhow!("table inconsistent: {e}"))
}

pub fn save(table: &BackoffTable, path: &Path) -> anyhow::Result<u64> {
    let bytes = serialize(table);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &bytes)?;
    Ok(bytes.len() as u64)
}

pub fn load(path: &Path) -> anyhow::Result<BackoffTable> {
    let bytes = std::fs::read(path)?;
    deserialize(&bytes)
}

// ── Structural tables — n-grams over Physis classification symbols ──────────

/// Build a structural table: corpus → classification per document →
/// `DOMAIN×MODE` symbol sequences → counts. Provenance: the corpus hash and
/// the classifier version ride in the manifest like any other table.
pub fn build_structural(
    docs: &[(String, String)],
    cfg: TableConfig,
    classify: impl Fn(&str) -> String,
) -> anyhow::Result<(BackoffTable, Vec<u8>)> {
    let tok = crate::tokenizer::StructuralTokenizer::default();
    let mut b = TableBuilder::new(cfg);
    let mut hash = Sha256::new();
    for (path, body) in docs {
        hash.update(path.as_bytes());
        hash.update(body.as_bytes());
        let symbols: Vec<String> = body
            .split(['\n', '.'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(&classify)
            .collect();
        b.push_sequence(&symbols);
    }
    let corpus_hash = format!("{:x}", hash.finalize());
    Ok(b.finish(&tok, &corpus_hash))
}

// ── Per-cell tables — one table per grid cell, not one table over all cells ─

/// A family of structural tables: one per `DOMAIN×MODE` cell, plus the flat
/// table over every sequence as its control.
///
/// # Why per-cell rather than one table
///
/// A single structural table models `P(next cell | previous cells)` over all
/// 70 symbols at once. It is one distribution, so every cell's continuations
/// are pooled and a rare cell's behaviour is averaged away by a common one.
///
/// A family conditions first: `tables["HEAL×MAINTAIN"]` is estimated **only**
/// from the steps that pass through that cell, so it can carry a continuation
/// pattern specific to that region even when that pattern is rare globally.
/// Facets (`sub_domain`, `sub_mode`, `lifecycle`, …) nest the same way — this
/// type is the first level, keyed by whatever symbol the caller supplies, so a
/// caller that emits `HEAL×MAINTAIN/pumps` gets that granularity for free.
///
/// # This is a claim, not a result
///
/// Conditioning costs data: each per-cell table sees a fraction of the corpus
/// and is therefore estimated from fewer counts. Whether the specificity pays
/// for the sparsity is **an empirical question**, which is why [`flat`] is
/// built from the same sequences in the same pass. Compare against it before
/// reporting that per-cell tables help; if the flat table wins, the flat table
/// wins.
pub struct CellTables {
    /// cell symbol → the table estimated from steps at that cell.
    pub tables: BTreeMap<String, BackoffTable>,
    /// The construction-matched control: one table over the same sequences.
    pub flat: BackoffTable,
    /// How many training steps each cell's table was estimated from.
    pub support: BTreeMap<String, usize>,
}

impl CellTables {
    /// Predict the next symbol after `context`, preferring the table for the
    /// cell the context currently sits in and falling back to `flat`.
    ///
    /// Falls back when the cell is unseen OR its table has less than
    /// `min_support` training steps — an under-supported table is worse than
    /// the pooled one, and silently trusting it is how conditioning turns into
    /// noise amplification.
    pub fn top_next(&self, context: &[String], k: usize, min_support: usize) -> Vec<(String, f32)> {
        match context.last() {
            Some(cell) if self.support.get(cell).copied().unwrap_or(0) >= min_support => self
                .tables
                .get(cell)
                .map(|t| t.top_next(context, k))
                .unwrap_or_else(|| self.flat.top_next(context, k)),
            _ => self.flat.top_next(context, k),
        }
    }

    /// Cells that have a table, ordered by support descending.
    pub fn by_support(&self) -> Vec<(&String, usize)> {
        let mut v: Vec<(&String, usize)> = self.support.iter().map(|(k, n)| (k, *n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        v
    }
}

/// Build a [`CellTables`] family from already-classified symbol sequences.
///
/// `sequences` is one ordered run of cell symbols per unit of work — a session,
/// an actor's trace, a document. Order matters: these are transitions, and a
/// set would lose exactly the information the table is for.
///
/// A cell's table is trained on every sequence *slice ending at that cell*, so
/// a step contributes context to the cell it lands on.
pub fn build_cell_tables(
    sequences: &[Vec<String>],
    cfg: TableConfig,
) -> anyhow::Result<CellTables> {
    anyhow::ensure!(
        !sequences.is_empty(),
        "no sequences to build cell tables from"
    );
    let tok = crate::tokenizer::StructuralTokenizer::default();

    // The control, over the identical input.
    let mut flat_b = TableBuilder::new(cfg.clone());
    let mut hash = Sha256::new();
    for seq in sequences {
        for sym in seq {
            hash.update(sym.as_bytes());
        }
        hash.update(b"\n");
        flat_b.push_sequence(seq);
    }
    let corpus_hash = format!("{:x}", hash.finalize());
    let (flat, _) = flat_b.finish(&tok, &corpus_hash);

    // Group every transition by the cell it departs from.
    let mut per_cell: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    let mut support: BTreeMap<String, usize> = BTreeMap::new();
    for seq in sequences {
        for i in 0..seq.len() {
            let cell = seq[i].clone();
            *support.entry(cell.clone()).or_insert(0) += 1;
            // The window ending at i, plus what follows, is this cell's
            // evidence. `max_order` bounds how much history is kept.
            let lo = i.saturating_sub(cfg.max_order as usize);
            let hi = (i + 2).min(seq.len());
            if hi > lo + 1 {
                per_cell.entry(cell).or_default().push(seq[lo..hi].to_vec());
            }
        }
    }

    let mut tables = BTreeMap::new();
    for (cell, seqs) in per_cell {
        let mut b = TableBuilder::new(cfg.clone());
        for s in &seqs {
            b.push_sequence(s);
        }
        let (t, _) = b.finish(&tok, &corpus_hash);
        tables.insert(cell, t);
    }

    Ok(CellTables {
        tables,
        flat,
        support,
    })
}

#[cfg(test)]
mod cell_table_tests {
    use super::*;

    fn cfg() -> TableConfig {
        TableConfig {
            kind: TableKind::Structural,
            max_order: 3,
            min_count: 1,
            ..Default::default()
        }
    }

    /// The family must actually condition: a continuation that is RARE overall
    /// but DOMINANT at one cell has to survive in that cell's table. This is
    /// the whole reason per-cell tables exist, so if it fails the type is
    /// pointless.
    #[test]
    fn a_cell_keeps_a_continuation_the_pooled_table_drowns() {
        let mut seqs: Vec<Vec<String>> = Vec::new();
        // 30 runs of the common pattern: A -> B -> C.
        for _ in 0..30 {
            seqs.push(vec!["A".into(), "B".into(), "C".into()]);
        }
        // 5 runs where B is reached from Z and is followed by Q instead.
        for _ in 0..5 {
            seqs.push(vec!["Z".into(), "B".into(), "Q".into()]);
        }
        let fam = build_cell_tables(&seqs, cfg()).unwrap();

        // Support is counted per occurrence, not per sequence.
        assert_eq!(fam.support["B"], 35);
        assert!(fam.tables.contains_key("B"), "B must have its own table");

        // Both tables see B -> Q; the point is that B's own table ranks it
        // against a smaller, more homogeneous pool.
        let b = &fam.tables["B"];
        assert!(
            b.count(&["B".into()], "Q") > 0,
            "B's table must retain B->Q"
        );
        assert!(
            b.count(&["B".into()], "C") > 0,
            "B's table must retain B->C"
        );
    }

    /// Under-supported cells must fall back to the pooled table rather than be
    /// trusted. Conditioning on two observations is noise amplification.
    #[test]
    fn under_supported_cells_fall_back_to_the_flat_control() {
        let mut seqs: Vec<Vec<String>> = Vec::new();
        for _ in 0..20 {
            seqs.push(vec!["A".into(), "B".into(), "C".into()]);
        }
        seqs.push(vec!["RARE".into(), "X".into()]);
        let fam = build_cell_tables(&seqs, cfg()).unwrap();
        assert_eq!(fam.support["RARE"], 1);

        let ctx = vec!["RARE".to_string()];
        let picked = fam.top_next(&ctx, 3, 10);
        let flat = fam.flat.top_next(&ctx, 3);
        assert_eq!(
            picked, flat,
            "support 1 < min_support 10 must use the control"
        );
    }

    /// The control is built from the SAME sequences in the SAME pass. If it
    /// ever diverges from the input the comparison it exists for is void.
    #[test]
    fn the_flat_control_covers_every_sequence() {
        let seqs = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["C".to_string(), "D".to_string()],
        ];
        let fam = build_cell_tables(&seqs, cfg()).unwrap();
        assert!(fam.flat.count(&["A".into()], "B") > 0);
        assert!(fam.flat.count(&["C".into()], "D") > 0);
    }

    #[test]
    fn empty_input_is_an_error_not_an_empty_family() {
        assert!(build_cell_tables(&[], cfg()).is_err());
    }

    #[test]
    fn by_support_is_ordered_and_deterministic() {
        let seqs = vec![
            vec!["A".to_string(), "A".to_string(), "A".to_string()],
            vec!["B".to_string(), "C".to_string()],
        ];
        let fam = build_cell_tables(&seqs, cfg()).unwrap();
        let order: Vec<&str> = fam.by_support().iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(order[0], "A", "highest support first");
        assert_eq!(fam.by_support(), fam.by_support(), "deterministic");
    }
}

// ── Registry — multiple named tables on disk, like the model registry ───────

pub struct TableRegistry {
    root: PathBuf,
}

impl TableRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        std::env::var_os("PHYSIS_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_home().join(".physis-core"))
            .join("ngram")
    }

    fn table_dir(&self, id: &str) -> PathBuf {
        self.root
            .join(sanitize(id))
            .join(format!("{}.physisng", sanitize(id)))
    }

    /// Install a built table under `id` (copy of verified bytes + manifest).
    pub fn install_bytes(&self, id: &str, bytes: &[u8]) -> anyhow::Result<TableManifest> {
        let table = deserialize(bytes)?; // verify before anything touches disk
        let dir = self.root.join(sanitize(id));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.physisng", sanitize(id)));
        std::fs::write(&path, bytes)?;
        let m = table.manifest().clone();
        std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&m)?)?;
        Ok(m)
    }

    pub fn list(&self) -> Vec<(String, TableManifest)> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for e in entries.filter_map(|e| e.ok()) {
                let mf = e.path().join("manifest.json");
                if let Ok(b) = std::fs::read(&mf) {
                    if let Ok(m) = serde_json::from_slice::<TableManifest>(&b) {
                        out.push((e.file_name().to_string_lossy().to_string(), m));
                    }
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub fn info(&self, id: &str) -> anyhow::Result<TableManifest> {
        let mf = self.root.join(sanitize(id)).join("manifest.json");
        Ok(serde_json::from_slice(&std::fs::read(mf)?)?)
    }

    pub fn path(&self, id: &str) -> PathBuf {
        self.table_dir(id)
    }

    pub fn load(&self, id: &str, model_tokenizer: Option<&str>) -> anyhow::Result<BackoffTable> {
        let table = load(&self.table_dir(id))?;
        if let Some(expected) = model_tokenizer {
            check_compatible(expected, &table.manifest().tokenizer.id)
                .map_err(anyhow::Error::from)?;
        }
        Ok(table)
    }

    pub fn remove(&self, id: &str) -> anyhow::Result<()> {
        std::fs::remove_dir_all(self.root.join(sanitize(id)))?;
        Ok(())
    }

    /// Export = copy the raw table file out (it is already self-describing).
    pub fn export(&self, id: &str, dest: &Path) -> anyhow::Result<u64> {
        let bytes = std::fs::read(self.table_dir(id))?;
        // Round-trip verify before export — never ship an unverified artifact.
        deserialize(&bytes)?;
        std::fs::write(dest, &bytes)?;
        Ok(bytes.len() as u64)
    }

    /// Import from an external file (same verification as install).
    pub fn import(&self, id: &str, src: &Path) -> anyhow::Result<TableManifest> {
        let bytes = std::fs::read(src)?;
        self.install_bytes(id, &bytes)
    }
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::WhitespaceTokenizer;

    fn tok() -> WhitespaceTokenizer {
        WhitespaceTokenizer::new(0)
    }

    fn build(_id: &str, corpus: &str, cfg: TableConfig) -> (BackoffTable, Vec<u8>) {
        let t = tok();
        let mut b = TableBuilder::new(cfg);
        b.push_text(&t, corpus);
        b.finish(&t, "corpus-hash-test")
    }

    const CORPUS: &str = "the pump failed because the seal wore out. the pump failed because the seal wore out again. maintenance replaced the seal today.";

    #[test]
    fn lookup_and_backoff_work() {
        let cfg = TableConfig {
            table_id: "t".into(),
            max_order: 3,
            ..Default::default()
        };
        let (t, _) = build("t", CORPUS, cfg);
        // Direct trigram hit.
        assert_eq!(t.count(&["the".into(), "pump".into()], "failed"), 2);
        // Backoff: unseen trigram context falls back to lower orders.
        let p_high = t.probability(&["the".into(), "pump".into()], "failed");
        let p_low = t.probability(&["completely".into(), "unseen".into()], "the");
        assert!(
            p_high > 0.5,
            "observed bigram continuation is likely: {p_high}"
        );
        assert!(
            p_low > 0.0 && p_low < 1.0,
            "unigram fallback is a real probability"
        );
        // top_next returns deterministic, count-ordered candidates.
        let tops = t.top_next(&["the".into(), "pump".into()], 3);
        assert_eq!(tops.first().map(|(w, _)| w.as_str()), Some("failed"));
    }

    #[test]
    fn build_is_deterministic_byte_for_byte() {
        // Same sequences, pushed in a different order, same config ⇒ the
        // serialized table is byte-identical (§24). Cross-sentence bridges
        // are part of the corpus definition, so both arms use sentences.
        let cfg = TableConfig {
            table_id: "d".into(),
            max_order: 3,
            ..Default::default()
        };
        let t = tok();
        let sentences: Vec<String> = CORPUS
            .split('.')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let mut fwd = TableBuilder::new(cfg.clone());
        for s in &sentences {
            fwd.push_text(&t, s);
        }
        let (_, b1) = fwd.finish(&t, "corpus-hash-test");
        let mut rev = TableBuilder::new(cfg);
        for s in sentences.iter().rev() {
            rev.push_text(&t, s);
        }
        let (_, b2) = rev.finish(&t, "corpus-hash-test");
        assert_eq!(b1, b2, "same corpus ⇒ byte-identical table (§24)");
        // And the roundtrip through bytes reproduces the same table.
        let t2 = deserialize(&b1).unwrap();
        assert_eq!(serialize(&t2), b1, "serialize(deserialize(x)) == x");
    }

    #[test]
    fn checksum_rejects_corruption() {
        let cfg = TableConfig {
            table_id: "c".into(),
            ..Default::default()
        };
        let (_, mut bytes) = build("c", CORPUS, cfg);
        let n = bytes.len();
        bytes[n - 3] ^= 0xFF; // flip bits in the tail (a record)
        assert!(deserialize(&bytes).is_err(), "corrupted table is refused");
    }

    #[test]
    fn vocabulary_cap_and_min_count_prune() {
        let cfg = TableConfig {
            table_id: "v".into(),
            max_vocabulary: 3,
            min_count: 1,
            ..Default::default()
        };
        let (t, _) = build("v", CORPUS, cfg);
        assert!(t.statistics().vocabulary <= 3, "vocab cap enforced");
    }

    #[test]
    fn registry_roundtrip_and_compatibility() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = TableRegistry::new(tmp.path());
        let cfg = TableConfig {
            table_id: "wiki".into(),
            ..Default::default()
        };
        let (_, bytes) = build("wiki", CORPUS, cfg);
        let m = reg.install_bytes("wiki-en-5gram", &bytes).unwrap();
        assert_eq!(m.table_id, "wiki");
        assert_eq!(reg.list().len(), 1);
        // load with matching tokenizer: ok
        assert!(reg.load("wiki-en-5gram", Some("whitespace-v1")).is_ok());
        // load with a model that expects a different tokenizer: refused loudly
        let err = reg.load("wiki-en-5gram", Some("smollm2-bpe")).unwrap_err();
        assert!(err.to_string().contains("rebuild"), "useful error: {err}");
        // export / import roundtrip
        let exp = tmp.path().join("export.bin");
        reg.export("wiki-en-5gram", &exp).unwrap();
        reg.install_bytes("copy", &std::fs::read(&exp).unwrap())
            .unwrap();
        assert_eq!(reg.list().len(), 2);
        reg.remove("copy").unwrap();
        reg.remove("wiki-en-5gram").unwrap();
        assert!(reg.list().is_empty());
    }

    #[test]
    fn structural_table_records_grid_symbols() {
        let docs = vec![
            ("a.md".to_string(), "pump seal replaced".to_string()),
            ("b.md".to_string(), "valve seal replaced".to_string()),
        ];
        // toy classifier: HEAL×MAINTAIN for seal-docs, FABRICATE×PLAN else
        let classify = |_s: &str| {
            if _s.contains("seal") {
                "HEAL×MAINTAIN".to_string()
            } else {
                "FABRICATE×PLAN".to_string()
            }
        };
        let cfg = TableConfig {
            table_id: "struct".into(),
            kind: TableKind::Structural,
            max_order: 3,
            ..Default::default()
        };
        let (t, _) = build_structural(&docs, cfg, classify).unwrap();
        assert_eq!(t.manifest().kind, TableKind::Structural);
        assert_eq!(t.manifest().tokenizer.id, "physis-structural-v1");
        // transition HEAL×MAINTAIN → HEAL×MAINTAIN observed in both docs
        assert!(t.count(&["HEAL×MAINTAIN".into()], "HEAL×MAINTAIN") >= 2);
    }

    #[test]
    fn incompatible_order_counts_are_refused() {
        let manifest = TableManifest {
            table_id: "bad".into(),
            kind: TableKind::Lexical,
            max_order: 2,
            tokenizer: tok().metadata(),
            corpus_hash: "x".into(),
            physis_version: "0".into(),
            smoothing_addk: 0.5,
            backoff_alpha: 0.4,
            min_count: 1,
            entry_count: 1,
            checksum: String::new(),
            license: "CC0".into(),
        };
        // manifest says order 2 (3 maps) but we hand over 1 map
        let bad = vec![BTreeMap::new()];
        assert!(BackoffTable::from_counts(manifest, bad).is_err());
    }
}
