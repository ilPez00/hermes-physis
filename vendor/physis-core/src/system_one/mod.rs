//! System-One semantic decision heads: typed, calibrated, interchangeable
//! backends at the bottom of a cost ladder — and the loop that builds them.
//!
//! ## The problem this answers
//!
//! Physis already decides things with an expensive, general mechanism: embed,
//! classify against the semiotic grid, ask a model, ask the oracle leg. That is
//! the right tool the *first* time a distinction is drawn. It is the wrong tool
//! the hundredth time the same distinction is drawn in the same region of the
//! corpus, because the answer stopped being a question.
//!
//! This module is the cheap rung. It defines one typed contract that every
//! backend implements — `rule | pattern | count-table | lexical(BM25) |
//! prototype | knn | ngram | SLM | Jev | LLM | human` — so the expensive rung
//! and the cheap rung are substitutable at the call site, and [`jit`] can move
//! a distinction down the ladder once it has been observed often enough and
//! validated well enough to be worth compiling.
//!
//! ## The contract, and where it comes from
//!
//! ```text
//! WorldSlice + OntologySlice + [HeadEvidence] → Decision<O>
//! Decision<O> { value, distribution, confidence, provenance, cost }
//! ```
//!
//! A `WorldSlice` is unstructured state (text + typed program state + an
//! embedding + the cell it was placed in). The `OntologySlice` is what the
//! system already knows about this region (entries, invariants, and the
//! semantic-delta map that says which distinctions are load-bearing). The
//! evidence is what was observed.
//!
//! The shape is deliberately the one TypeSafe's System One Models / Jev
//! advertise for typed probabilistic decisions
//! (<https://typesafe.ai/blog/introducing-system-one-models-and-jev>): *"the
//! possible outputs and structure are defined in advance"*, every answer
//! carries a **calibrated** probability, decisions are answerable in parallel
//! because the schema is static, and a low-cardinality label space is the norm.
//! Three consequences are wired into the types rather than the docs:
//!
//! - **No type errors are representable.** [`DecisionLabel`] is implemented by
//!   `bool`, `String`, `i64`, and by enums generated with [`label_set`]; a head
//!   cannot return a label outside its declared space, and
//!   [`Provenance::contract`] records which space it answered in.
//! - **Abstention is a value, not an error.** `Decision::value: None` means *no
//!   opinion*; the router escalates to the next rung. A head that guesses
//!   without evidence is a head that cannot be trusted, so the cheap rungs
//!   abstain rather than fill in.
//! - **Confidence is data.** [`Decision::confidence`] is what
//!   [`calibration::ConformalGate`] gates on; it is not a decoration, and it is
//!   not a re-scaled logit unless a calibration step put it there.
//!
//! ## Why heads are interchangeable but not identical
//!
//! Every backend answers in the same type, but the honest differences are in
//! the record: [`HeadKind`] (what it is), [`HeadSpec`] (which features it
//! reads, which label space it answers in, which ontology entries it depends
//! on), [`Provenance`] (where the answer came from, down to the rule text), and
//! [`Cost`] (measured, not estimated). Two rungs can therefore be swapped
//! without a caller noticing — and can also be *compared*, which is the point.
//!
//! ## Honest limits
//!
//! - A cheap head is a **compiled distinction**, not a small philosopher. It is
//!   only as good as the traces it was distilled from, and it is only safe
//!   inside the region those traces came from. [`HeadSpec`] carries that region
//!   so [`jit::DependencyIndex`] can invalidate the head when the ontology moves
//!   under it.
//! - Under the random-projection fallback embedder a prototype head matches
//!   surface form, not meaning — the same caveat as everywhere else in this
//!   crate. Nothing in this module selects an embedder on its own: the caller
//!   passes vectors in, so the choice stays visible at the call site.
//! - Nothing here calls a network. The LLM rung is the trait
//!   [`DecisionOracle`], which `physis-core` implements only behind the `http`
//!   feature (see [`heads::HttpOracle`]) and which a Pro deployment can
//!   implement directly. Offline, the strongest rung is the n-gram or SLM rung.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub mod calibration;
pub mod heads;
pub mod jit;
pub mod router;

pub use calibration::{
    accepted_accuracy, bootstrap_ci, brier, conformal_quantile, ece, paired_differences,
    wilson_interval, CalibrationReport, ConfidenceOutcome, ConformalGate,
};
pub use heads::{
    CountTableHead, Expr, HumanQueueHead, KnnHead, LexicalHead, ModelHead, OracleHead, PatternHead,
    PatternKind, PatternRule, PendingAsk, PrototypeHead, Rule, RuleConfig, RuleHead, StubOracle,
};
pub use jit::{
    detect_drift, detect_recurrence, evaluate_promotion, instantiate, pareto_frontier,
    plan_recompute, split_cell_key, synthesize, Audit, DecisionTrace, DependencyIndex, DriftSignal,
    HeadDependency, InvalidationPlan, JitConfig, JitOutcome, JitStats, ParetoPoint, Promotion,
    PromotionPolicy, Recurrence, RecurrenceConfig, SemanticJit, Split, SplitValue, SynthesisConfig,
    TraceStore,
};
pub use router::{
    AcceptanceGate, Attempt, Bandit, BetaPosterior, HeadRouter, RoutedDecision, RoutingPolicy,
};

#[cfg(feature = "http")]
pub use heads::HttpOracle;

// ── Backends ─────────────────────────────────────────────────────────────

/// The closed vocabulary of interchangeable backends, cheapest first.
///
/// The order is the cost ladder the router walks: it is `Ord` by declaration,
/// so "is there a cheaper rung than this?" is a comparison rather than a lookup
/// table that can drift out of date.
///
/// Every variant except `Jev`/`Llm`/`Human` has a real implementation in
/// [`heads`]. The three that do not require a network or a person; they exist so
/// that a deployment can *name* the rung it escalated to instead of pretending
/// the ladder ended.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum HeadKind {
    /// Deterministic predicate over typed state. Zero model, zero counting.
    ///
    /// The `Default` is `Rule` for one reason: an empty [`Provenance`] is only
    /// reachable through `Decision::abstain`, which fills the head in itself, so
    /// serde can round-trip a hand-written record without a bespoke visitor.
    #[default]
    Rule,
    /// Literal/prefix (or `regex`, with the `head-regex` feature) match on the
    /// world's text. Zero model.
    Pattern,
    /// Backed-off n-gram count table over `tokens → label`
    /// ([`crate::ngram_table`]). Zero model, the crate's own machinery.
    CountTable,
    /// BM25 over labeled exemplar texts. Zero model.
    Lexical,
    /// Nearest label centroid in embedding space — the same family as
    /// [`crate::classify::CellClassifier`]'s nearest-entry scoring.
    Prototype,
    /// k-NN exemplar classifier over embedding (+ typed features).
    Knn,
    /// A [`crate::model_provider::ModelProvider`] scoring candidate labels.
    /// N-gram-backed offline, SLM-backed when a real model is installed.
    Ngram,
    /// A System-One shaped remote oracle: static schema, parallel sampling,
    /// calibrated probabilities (TypeSafe Jev and equivalents).
    Jev,
    /// A free-text LLM behind an adapter that parses a typed answer out of it.
    /// Slower and uncalibrated by construction — the adapter must say so.
    Llm,
    /// A person, queued and learned from.
    Human,
}

impl HeadKind {
    pub fn as_str(self) -> &'static str {
        match self {
            HeadKind::Rule => "rule",
            HeadKind::Pattern => "pattern",
            HeadKind::CountTable => "count-table",
            HeadKind::Lexical => "lexical",
            HeadKind::Prototype => "prototype",
            HeadKind::Knn => "knn",
            HeadKind::Ngram => "ngram",
            HeadKind::Jev => "jev",
            HeadKind::Llm => "llm",
            HeadKind::Human => "human",
        }
    }

    /// Whether answering costs no model at all — deployable on a machine with
    /// no weights, no key and no network.
    pub fn zero_model(self) -> bool {
        matches!(
            self,
            HeadKind::Rule
                | HeadKind::Pattern
                | HeadKind::CountTable
                | HeadKind::Lexical
                | HeadKind::Prototype
                | HeadKind::Knn
        )
    }

    /// Whether answering bills per token.
    pub fn bills_tokens(self) -> bool {
        matches!(self, HeadKind::Jev | HeadKind::Llm)
    }

    /// Nominal per-call cost, used *before* a head has measurements.
    ///
    /// The remote rungs are vendor-published figures, not local measurements,
    /// and are marked nominal. Local rungs are order-of-magnitude estimates on
    /// purpose: [`router::HeadRouter`] replaces them with measured micros after
    /// the first calls, and the experiment reports both rather than conflating
    /// them.
    pub fn nominal_profile(self) -> CostProfile {
        match self {
            HeadKind::Rule => CostProfile::local(1, 0),
            HeadKind::Pattern => CostProfile::local(5, 0),
            HeadKind::CountTable => CostProfile::local(20, 1 << 20),
            HeadKind::Lexical => CostProfile::local(200, 1 << 20),
            HeadKind::Prototype => CostProfile::local(400, 384 * 4),
            HeadKind::Knn => CostProfile::local(1_500, 1 << 21),
            HeadKind::Ngram => CostProfile::local(30_000, 256 << 20),
            HeadKind::Jev => CostProfile::vendor(400_000, 400, 0.042, 0.0),
            HeadKind::Llm => CostProfile::vendor(4_000_000, 900, 3.0, 15.0),
            HeadKind::Human => CostProfile::vendor(600_000_000, 200, 0.0, 0.0),
        }
    }
}

/// Where a cost figure came from. A number without this is a claim, not a
/// measurement, and this crate does not publish those.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSource {
    /// From [`HeadKind::nominal_profile`] — an estimate.
    Nominal,
    /// A wall-clock measurement taken on this machine.
    Measured,
    /// Vendor-published pricing (Jev: <https://typesafe.ai>), not local.
    Vendor,
}

/// Vendor pricing constants, with their sources. Kept in one place so a
/// benchmark cannot quietly use one set of numbers and a report another.
///
/// - Jev: input `$0.042 / MTok`, **output free** (TypeSafe, Sep 2026).
/// - Frontier LLM rung: `$3 / MTok` in, `$15 / MTok` out — a mid-frontier
///   reference, used only to price the expensive arm, never as a claim about
///   intelligence.
pub mod pricing {
    pub const JEV_USD_PER_MTOK_IN: f64 = 0.042;
    pub const JEV_USD_PER_MTOK_OUT: f64 = 0.0;
    pub const FRONTIER_USD_PER_MTOK_IN: f64 = 3.0;
    pub const FRONTIER_USD_PER_MTOK_OUT: f64 = 15.0;
    /// Tokens a decision costs on the remote rungs: state in, label out.
    pub const REMOTE_DECISION_TOKENS_IN: u64 = 400;
    pub const REMOTE_DECISION_TOKENS_OUT: u64 = 8;
}

/// Nominal per-call cost of a rung, before it has been measured.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CostProfile {
    pub micros: u64,
    pub ram_bytes: u64,
    pub usd_per_mtok_in: f64,
    pub usd_per_mtok_out: f64,
    pub source: CostSource,
}

impl CostProfile {
    /// A locally-measurable rung: micros of wall clock, RAM of working set.
    pub const fn local(micros: u64, ram_bytes: u64) -> Self {
        Self {
            micros,
            ram_bytes,
            usd_per_mtok_in: 0.0,
            usd_per_mtok_out: 0.0,
            source: CostSource::Nominal,
        }
    }

    /// A remote rung: priced from vendor figures, latency from the vendor's
    /// published band rather than from this machine.
    pub const fn vendor(micros: u64, ram_bytes: u64, usd_in: f64, usd_out: f64) -> Self {
        Self {
            micros,
            ram_bytes,
            usd_per_mtok_in: usd_in,
            usd_per_mtok_out: usd_out,
            source: CostSource::Vendor,
        }
    }

    /// Whether answering this rung bills tokens at all.
    pub fn bills_tokens(&self) -> bool {
        self.usd_per_mtok_in > 0.0 || self.usd_per_mtok_out > 0.0
    }

    /// Estimated toll for one decision in **micro-dollars**, at the token
    /// assumption in [`pricing`]. Integer micro-dollars on purpose: a thousand
    /// cheap calls otherwise vanish into `f64` noise and the saving stops being
    /// visible in exactly the regime the ladder exists for.
    pub fn usd_micros_per_call(&self) -> u64 {
        if !self.bills_tokens() {
            return 0;
        }
        let in_usd = self.usd_per_mtok_in * (pricing::REMOTE_DECISION_TOKENS_IN as f64) / 1e6;
        let out_usd = self.usd_per_mtok_out * (pricing::REMOTE_DECISION_TOKENS_OUT as f64) / 1e6;
        ((in_usd + out_usd) * 1e6).round() as u64
    }

    pub fn as_cost(&self) -> Cost {
        Cost {
            micros: self.micros,
            ram_bytes: self.ram_bytes,
            tokens_in: if self.bills_tokens() {
                pricing::REMOTE_DECISION_TOKENS_IN
            } else {
                0
            },
            tokens_out: if self.bills_tokens() {
                pricing::REMOTE_DECISION_TOKENS_OUT
            } else {
                0
            },
            usd_micros: self.usd_micros_per_call(),
            source: self.source.into(),
        }
    }
}

/// What one decision actually cost. Every field is either measured or labelled
/// nominal/vendor by [`Cost::source`]; nothing is inferred at report time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    pub micros: u64,
    pub ram_bytes: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub usd_micros: u64,
    pub source: CostSourceMarker,
}

/// [`CostSource`] in a `Copy + Default` slot, so `Cost::default()` is the zero
/// cost with an honest label rather than a missing one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSourceMarker {
    #[default]
    Measured,
    Nominal,
    Vendor,
}

impl From<CostSource> for CostSourceMarker {
    fn from(s: CostSource) -> Self {
        match s {
            CostSource::Nominal => CostSourceMarker::Nominal,
            CostSource::Measured => CostSourceMarker::Measured,
            CostSource::Vendor => CostSourceMarker::Vendor,
        }
    }
}

impl Cost {
    pub const ZERO: Cost = Cost {
        micros: 0,
        ram_bytes: 0,
        tokens_in: 0,
        tokens_out: 0,
        usd_micros: 0,
        source: CostSourceMarker::Measured,
    };

    /// Add two costs, keeping the *weakest* provenance: measured + nominal is
    /// nominal, because the sum is no better evidenced than its weakest term.
    ///
    /// Named `plus` rather than `add` so that `+` and this agree on nothing by
    /// accident: the provenance rule is a policy, and an operator impl would hide
    /// it behind a `+` that reads like arithmetic.
    pub fn plus(self, other: Cost) -> Cost {
        Cost {
            micros: self.micros + other.micros,
            ram_bytes: self.ram_bytes.max(other.ram_bytes),
            tokens_in: self.tokens_in + other.tokens_in,
            tokens_out: self.tokens_out + other.tokens_out,
            usd_micros: self.usd_micros + other.usd_micros,
            source: weakest(self.source, other.source),
        }
    }

    pub fn from_profile(p: &CostProfile) -> Cost {
        p.as_cost()
    }
}

fn weakest(a: CostSourceMarker, b: CostSourceMarker) -> CostSourceMarker {
    let rank = |m: CostSourceMarker| match m {
        CostSourceMarker::Measured => 0,
        CostSourceMarker::Vendor => 1,
        CostSourceMarker::Nominal => 2,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

// ── The world, the ontology, the evidence ────────────────────────────────

/// The cell key format the rest of the crate already uses for a
/// `DOMAIN×MODE` pair: `"DOMAIN\u{0}MODE"`, matching
/// [`crate::quality`]'s penalties and [`crate::classify`]'s proposals, so one
/// string identifies a cell everywhere instead of three near-misses.
pub fn cell_key(domain: &str, mode: &str) -> String {
    format!("{domain}\u{0}{mode}")
}

/// A typed value in a [`WorldSlice`].
///
/// Deliberately small: a head's feature set is meant to be enumerable and
/// testable, and every variant has a canonical string form so a rule or a count
/// table can key on it without a bespoke serializer. `Text` is the escape hatch
/// — a distinction that can only be expressed as free text is one the cheap
/// rungs handle less well, and the synthesis step says so rather than guessing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureValue {
    Bool(bool),
    Int(i64),
    Real(f64),
    Text(String),
}

impl FeatureValue {
    /// Canonical string form, shared by rules, count tables and traces.
    pub fn key(&self) -> String {
        match self {
            FeatureValue::Bool(b) => b.to_string(),
            FeatureValue::Int(i) => i.to_string(),
            FeatureValue::Real(f) => format!("{f}"),
            FeatureValue::Text(t) => t.to_lowercase(),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FeatureValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            FeatureValue::Int(i) => Some(*i as f64),
            FeatureValue::Real(f) => Some(*f),
            FeatureValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            FeatureValue::Text(_) => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            FeatureValue::Text(t) => Some(t),
            _ => None,
        }
    }
}

/// Unstructured state in, one decision out.
///
/// `state` is the typed half (dotted keys, e.g. `sensor.vibration`), `text` the
/// unstructured half, and `embedding` what a classifier already computed —
/// passed in rather than recomputed, so a decision costs no embedding work of
/// its own.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldSlice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub state: BTreeMap<String, FeatureValue>,
    #[serde(default)]
    pub embedding: Vec<f32>,
    /// The semiotic cell this world was placed in, when it has been placed.
    /// Used as the routing context (the bandit keys on it) and as the region a
    /// head is scoped to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

impl WorldSlice {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn with_state(mut self, key: impl Into<String>, v: FeatureValue) -> Self {
        self.state.insert(key.into(), v);
        self
    }

    pub fn with_embedding(mut self, e: Vec<f32>) -> Self {
        self.embedding = e;
        self
    }

    pub fn with_cell(mut self, domain: impl Into<String>, mode: impl Into<String>) -> Self {
        self.cell = Some((domain.into(), mode.into()));
        self
    }

    /// `DOMAIN\u{0}MODE` for this world's cell, if it has one.
    pub fn cell_key(&self) -> Option<String> {
        self.cell.as_ref().map(|(d, m)| cell_key(d, m))
    }
}

/// An ontology entry, reduced to what a head needs to know about it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntryRef {
    pub name: String,
    pub domain: String,
    pub mode: String,
    /// The embedded text of the entry (name + hints), as
    /// [`crate::classify::EntrySeed`] builds it.
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub embedding: Vec<f32>,
}

impl EntryRef {
    pub fn cell_key(&self) -> String {
        cell_key(&self.domain, &self.mode)
    }
}

/// An invariant the world is expected to satisfy.
///
/// Heads may read these as features; more importantly, a hard violation is a
/// signal that a cheap head's region is no longer the region it was distilled
/// in, which is why they are part of the decision input rather than a comment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invariant {
    pub id: String,
    pub statement: String,
    /// State keys the invariant constrains. Empty = constrains the text.
    #[serde(default)]
    pub features: Vec<String>,
    /// Hard invariants refute a decision outright; soft ones demote it.
    pub hard: bool,
}

/// One edge of the semantic-delta map: which distinction moved, and how much.
///
/// This is the "where" in *which distinctions matter and where*. The map is an
/// input, not something this module computes: the engine already owns the
/// machinery that derives structural deltas ([`crate::delta_engine`],
/// [`crate::direction`], [`crate::map`]), and a head that recomputed it would be
/// a second opinion rather than a reuse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticDelta {
    /// Entry name or [`cell_key`] the change touches.
    pub target: String,
    /// Magnitude in `[0, 1]`. Higher = more load-bearing for this region.
    pub weight: f32,
    #[serde(default)]
    pub rationale: String,
}

impl SemanticDelta {
    pub fn new(target: impl Into<String>, weight: f32) -> Self {
        Self {
            target: target.into(),
            weight: weight.clamp(0.0, 1.0),
            rationale: String::new(),
        }
    }

    pub fn because(mut self, why: impl Into<String>) -> Self {
        self.rationale = why.into();
        self
    }
}

/// What the system already knows about the world it is about to decide on.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OntologySlice {
    #[serde(default)]
    pub entries: Vec<EntryRef>,
    #[serde(default)]
    pub invariants: Vec<Invariant>,
    #[serde(default)]
    pub deltas: Vec<SemanticDelta>,
}

impl OntologySlice {
    pub fn empty() -> Self {
        Self::default()
    }

    /// The priority of a region, in `[0, 1]`: the largest delta weight touching
    /// it. Zero means nothing is *known* to have moved there, which is the
    /// normal case and is not a reason to skip the region — only a reason to
    /// rank other regions first.
    pub fn delta_priority(&self, cell: Option<&str>, entries: &[String]) -> f32 {
        self.deltas
            .iter()
            .filter(|d| cell.is_some_and(|c| d.target == c) || entries.contains(&d.target))
            .map(|d| d.weight)
            .fold(0.0_f32, f32::max)
    }

    pub fn entry(&self, name: &str) -> Option<&EntryRef> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Hard invariants this world does not satisfy, judged by the presence of
    /// the state keys they constrain. The check is shallow on purpose: an
    /// invariant whose statement needs parsing is a hypothesis, and
    /// [`crate::hypothesis`] is where that lives.
    pub fn violated(&self, world: &WorldSlice) -> Vec<&Invariant> {
        self.invariants
            .iter()
            .filter(|inv| inv.hard && !inv.features.is_empty())
            .filter(|inv| inv.features.iter().all(|f| !world.state.contains_key(f)))
            .collect()
    }
}

/// What was observed about this world, as input to a decision.
///
/// Distinct from [`crate::hypothesis::Evidence`], which is a *claim's* support.
/// This one is the same fact addressed to a head: a labeled exemplar, a raw
/// observation, or a known counterexample — things a head can retrieve against,
/// not things it weighs as belief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadEvidence {
    pub id: String,
    pub kind: EvidenceKind,
    pub claim: String,
    /// Confidence in this evidence, `[0, 1]`.
    pub confidence: f32,
    /// The label this evidence asserts, when it asserts one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub features: BTreeMap<String, FeatureValue>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// What kind of thing a piece of [`HeadEvidence`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Something was seen. May carry no label.
    Observation,
    /// A validated outcome: the label is ground truth for its region.
    Label,
    /// A case known to contradict a candidate distinction. Used by the
    /// synthesis step to reject a split a control would have accepted.
    Counterexample,
}

impl HeadEvidence {
    pub fn labeled(
        id: impl Into<String>,
        claim: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: EvidenceKind::Label,
            claim: claim.into(),
            confidence: 1.0,
            label: Some(label.into()),
            features: BTreeMap::new(),
            source: "unspecified".into(),
            embedding: None,
        }
    }

    pub fn observation(id: impl Into<String>, claim: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: EvidenceKind::Observation,
            claim: claim.into(),
            confidence: 1.0,
            label: None,
            features: BTreeMap::new(),
            source: "unspecified".into(),
            embedding: None,
        }
    }

    pub fn from_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_confidence(mut self, c: f32) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self
    }
}

/// One decision request: world + ontology + evidence, owned so that a cascade
/// can hand the same request to every rung without rebuilding it per head.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DecisionInput {
    pub world: WorldSlice,
    pub ontology: OntologySlice,
    #[serde(default)]
    pub evidence: Vec<HeadEvidence>,
}

impl DecisionInput {
    pub fn new(world: WorldSlice, ontology: OntologySlice) -> Self {
        Self {
            world,
            ontology,
            evidence: Vec::new(),
        }
    }

    pub fn with_evidence(mut self, e: HeadEvidence) -> Self {
        self.evidence.push(e);
        self
    }

    /// Labeled exemplars only, in the order supplied — the retrieval slice a
    /// cheap head sees.
    pub fn labeled_evidence(&self) -> Vec<&HeadEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.kind == EvidenceKind::Label && e.label.is_some())
            .collect()
    }

    /// One labeled world, with its evidence attached — the shortest legal way
    /// to build a request.
    pub fn example(
        id: impl Into<String>,
        text: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        let id: String = id.into();
        let text: String = text.into();
        let e = HeadEvidence::labeled(format!("{id}::label"), text.clone(), label);
        Self::new(WorldSlice::new(id, text), OntologySlice::empty()).with_evidence(e)
    }

    pub fn cell(&self) -> Option<String> {
        self.world.cell_key()
    }
}

// ─ The decision ─────────────────────────────────────────────────────────

/// One label with its probability. `distribution` is a *vector* of these rather
/// than a map so that a head's answer stays ordered and comparable; ties are
/// broken at construction, never at read time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mass<O> {
    pub value: O,
    pub probability: f32,
}

impl<O> Mass<O> {
    pub fn new(value: O, probability: f32) -> Self {
        Self { value, probability }
    }
}

/// Where a decision came from. Not optional, and not free text: a decision
/// without provenance cannot be audited, and an unauditable cheap head is
/// indistinguishable from a lucky guess.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub head_id: String,
    pub kind: HeadKind,
    /// Version of the head, incremented on every refit.
    pub version: u32,
    /// The label-space contract this answer is valid in — the same string as
    /// [`HeadSpec::label_space`].
    pub contract: String,
    /// Human-readable derivation where one exists: the rule text, the pattern,
    /// the nearest exemplar. `None` for opaque rungs, and saying so is the
    /// point rather than a gap to paper over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation: Option<String>,
    /// Evidence ids consulted.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    /// Trace ids this head was distilled from, when it was distilled.
    #[serde(default)]
    pub trace_ids: Vec<String>,
    /// Id of the calibration gate that admitted this confidence, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibrated_by: Option<String>,
}

impl Provenance {
    pub fn head(id: impl Into<String>, kind: HeadKind, contract: impl Into<String>) -> Self {
        Self {
            head_id: id.into(),
            kind,
            version: 1,
            contract: contract.into(),
            derivation: None,
            evidence_ids: Vec::new(),
            trace_ids: Vec::new(),
            calibrated_by: None,
        }
    }

    pub fn because_of(mut self, derivation: impl Into<String>) -> Self {
        self.derivation = Some(derivation.into());
        self
    }

    pub fn citing(mut self, evidence_ids: Vec<String>) -> Self {
        self.evidence_ids = evidence_ids;
        self
    }
}

/// The answer: a typed value (or an explicit abstention), a distribution over
/// the declared label space, a confidence, provenance, and what it cost.
///
/// `confidence` is *the probability that `value` is correct*, which is the
/// quantity a selective gate can threshold. It is deliberately not the same
/// field as `distribution[0].probability`: for a rule head those coincide, for
/// a retrieval head they do not, and collapsing them would hide exactly the
/// miscalibration a head has to be audited for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision<O> {
    pub value: Option<O>,
    pub distribution: Vec<Mass<O>>,
    pub confidence: f32,
    /// What the head believed before calibration. Kept so the two can be
    /// compared — a head whose raw and calibrated confidences always agree is
    /// either perfect or not being calibrated at all.
    pub raw_confidence: f32,
    pub provenance: Provenance,
    pub cost: Cost,
}

impl<O: Clone> Decision<O> {
    pub fn answered(
        value: O,
        distribution: Vec<Mass<O>>,
        confidence: f32,
        provenance: Provenance,
        cost: Cost,
    ) -> Self {
        let raw = confidence;
        Self {
            value: Some(value),
            distribution,
            confidence: confidence.clamp(0.0, 1.0),
            raw_confidence: raw,
            provenance,
            cost,
        }
    }

    /// No opinion, with the head's own name on it. The router escalates; a
    /// caller that ignores the escalation has chosen to have no answer.
    pub fn abstain(provenance: Provenance, cost: Cost) -> Self {
        Self {
            value: None,
            distribution: Vec::new(),
            confidence: 0.0,
            raw_confidence: 0.0,
            provenance,
            cost,
        }
    }

    pub fn did_abstain(&self) -> bool {
        self.value.is_none()
    }

    pub fn top(&self) -> Option<&Mass<O>> {
        self.distribution.first()
    }

    /// Posterior mass between the top two labels. A single-label answer reports
    /// its own probability, which is the honest reading of a deterministic rung
    /// with nothing to compare against.
    pub fn margin(&self) -> f32 {
        match (self.distribution.first(), self.distribution.get(1)) {
            (Some(a), Some(b)) => a.probability - b.probability,
            (Some(a), None) => a.probability,
            _ => 0.0,
        }
    }

    /// Shannon entropy of the distribution in nats. `None` when there is no
    /// distribution — not `0.0`, because "no distribution" and "certain" are
    /// different states and only one of them is safe to route on.
    pub fn entropy(&self) -> Option<f32> {
        if self.distribution.is_empty() {
            return None;
        }
        Some(
            -self
                .distribution
                .iter()
                .filter(|m| m.probability > 0.0)
                .map(|m| m.probability * m.probability.ln())
                .sum::<f32>(),
        )
    }

    /// Re-stamp the calibrated confidence, keeping the raw one. This is what a
    /// [`calibration::ConformalGate`] does to an answer: it does not change the
    /// value, it changes what the value is allowed to claim.
    pub fn with_calibrated_confidence(mut self, c: f32, gate_id: impl Into<String>) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self.provenance.calibrated_by = Some(gate_id.into());
        self
    }

    pub fn map<T>(&self, f: impl FnOnce(&O) -> T) -> Option<T> {
        self.value.as_ref().map(f)
    }

    /// True when the answer is a value that the gate would accept.
    pub fn accepts(&self, gate: &calibration::ConformalGate) -> bool {
        !self.did_abstain() && gate.accepts(self.confidence)
    }
}

// ── Label spaces: the type in "type-safe" ────────────────────────────────

/// A closed label space a head can answer in.
///
/// Three obligations, all of them mechanical: a canonical string (`as_key`,
/// lowercase, used by count tables and traces), a total parser (`from_key`) so
/// an answer that arrives as text can be **rejected** rather than coerced, and
/// a declared cardinality used to check a remote schema against the binding's
/// limit. Nothing here is a heuristic.
pub trait DecisionLabel:
    Clone
    + PartialEq
    + std::fmt::Debug
    + Send
    + Sync
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
    + 'static
{
    fn as_key(&self) -> String;
    fn from_key(key: &str) -> Option<Self>;
    /// Declared size of the space, when it is finite and small. `None` for
    /// open spaces such as free text.
    fn cardinality() -> Option<usize>;
}

impl DecisionLabel for bool {
    fn as_key(&self) -> String {
        if *self {
            "true".into()
        } else {
            "false".into()
        }
    }
    fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "true" | "t" | "yes" | "y" | "1" => Some(true),
            "false" | "f" | "no" | "n" | "0" => Some(false),
            _ => None,
        }
    }
    fn cardinality() -> Option<usize> {
        Some(2)
    }
}

impl DecisionLabel for i64 {
    fn as_key(&self) -> String {
        self.to_string()
    }
    fn from_key(key: &str) -> Option<Self> {
        key.trim().parse().ok()
    }
    fn cardinality() -> Option<usize> {
        None
    }
}

impl DecisionLabel for String {
    fn as_key(&self) -> String {
        self.to_lowercase()
    }
    /// Open space: any non-empty string parses, and the head's [`HeadSpec`] —
    /// not this trait — is what closes it. An `OracleHead` therefore checks the
    /// parsed value against the spec's declared labels
    /// (`HeadSpec::declared_labels`) before answering.
    fn from_key(key: &str) -> Option<Self> {
        let k = key.trim();
        if k.is_empty() {
            None
        } else {
            Some(k.to_string())
        }
    }
    fn cardinality() -> Option<usize> {
        None
    }
}

/// Declare a closed enum label space with no hand-written `DecisionLabel`.
///
/// ```ignore
/// label_set! {
///     /// What a maintenance decision may answer.
///     pub enum Action { Repair, Observe, Escalate }
/// }
/// ```
///
/// Generates the enum (deriving the traits a label needs), `as_key` in
/// `snake_case`, a **total** `from_key` that returns `None` on anything else,
/// `ALL` in declaration order, and `cardinality()`. A head built over this type
/// cannot answer outside it, which is the whole claim of the contract.
#[macro_export]
macro_rules! label_set {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident { $($variant:ident),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        $vis enum $name { $($variant),+ }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// `snake_case` key, without pulling in a case-conversion crate.
            pub fn key(&self) -> String {
                let mut out = String::new();
                for (i, c) in format!("{:?}", self).chars().enumerate() {
                    if c.is_uppercase() {
                        if i > 0 {
                            out.push('_');
                        }
                        out.extend(c.to_lowercase());
                    } else {
                        out.push(c);
                    }
                }
                out
            }
        }

        impl $crate::system_one::DecisionLabel for $name {
            fn as_key(&self) -> String {
                self.key()
            }
            fn from_key(key: &str) -> Option<Self> {
                let k = key.trim().to_ascii_lowercase();
                Self::ALL.iter().find(|v| v.key() == k).cloned()
            }
            fn cardinality() -> Option<usize> {
                Some(Self::ALL.len())
            }
        }
    };
}

// ─ Head specification: what a distinction is compiled into ──────────────

/// How a head decides, in one word. `Exact` means the answer is a function of
/// the declared features alone and can be read off the rule; the further down
/// this list, the less a person can check by reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transparency {
    /// A declared predicate: fully readable.
    Exact,
    /// A literal/prefix/regex pattern list: readable.
    Pattern,
    /// Counting table: readable in principle, not in practice.
    Counted,
    /// Retrieval over exemplars: readable case by case.
    Retrieved,
    /// A model answered: opaque.
    Opaque,
    /// A person answered: readable but not reproducible.
    Human,
}

/// What a head is for, what it may read, and what it answers in.
///
/// This is the *contract*, and it is also the invalidation key: the feature
/// keys, cells and entry names here are what [`jit::DependencyIndex`] matches a
/// semantic delta against, so a head that reads something its spec never
/// declared is a head that will silently survive a change it should not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadSpec {
    /// The decision being made, e.g. `"maintenance.action"`. One decision type
    /// per spec; a head answers exactly one.
    pub decision_type: String,
    /// Canonical, ordered list of legal label keys.
    pub declared_labels: Vec<String>,
    /// Stable digest of `decision_type + declared_labels`, so two specs that
    /// disagree about the space cannot both be used under one contract.
    pub label_space: String,
    /// State keys the head is allowed to read.
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether the head reads `world.text`.
    pub text_view: bool,
    /// Whether the head reads `world.embedding`.
    pub embedding_view: bool,
    /// Cells this head is scoped to, as [`cell_key`]s. Empty = unscoped.
    #[serde(default)]
    pub cells: Vec<String>,
    /// Ontology entry names the head was distilled from or reads.
    #[serde(default)]
    pub entry_deps: Vec<String>,
    pub transparency: Transparency,
    /// Why this head exists — the distinction it compiles, in one line.
    #[serde(default)]
    pub rationale: String,
}

impl HeadSpec {
    /// Build a spec, computing `label_space` from the declared labels.
    pub fn new(
        decision_type: impl Into<String>,
        declared_labels: impl IntoIterator<Item = impl Into<String>>,
        transparency: Transparency,
    ) -> Self {
        let decision_type = decision_type.into();
        let mut declared_labels: Vec<String> = declared_labels
            .into_iter()
            .map(|l| l.into().to_lowercase())
            .collect();
        declared_labels.sort();
        declared_labels.dedup();
        let label_space = label_space_digest(&decision_type, &declared_labels);
        Self {
            decision_type,
            declared_labels,
            label_space,
            features: Vec::new(),
            text_view: false,
            embedding_view: false,
            cells: Vec::new(),
            entry_deps: Vec::new(),
            transparency,
            rationale: String::new(),
        }
    }

    pub fn with_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut l: Vec<String> = labels
            .into_iter()
            .map(|x| x.into().to_lowercase())
            .collect();
        l.sort();
        l.dedup();
        self.label_space = label_space_digest(&self.decision_type, &l);
        self.declared_labels = l;
        self
    }

    pub fn because(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = rationale.into();
        self
    }

    pub fn reading(mut self, features: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.features = features.into_iter().map(|f| f.into()).collect();
        self.features.sort();
        self.features.dedup();
        self
    }

    pub fn with_text(mut self) -> Self {
        self.text_view = true;
        self
    }

    pub fn with_embedding(mut self) -> Self {
        self.embedding_view = true;
        self
    }

    pub fn scoped_to(mut self, cells: impl IntoIterator<Item = String>) -> Self {
        self.cells = cells.into_iter().collect();
        self.cells.sort();
        self.cells.dedup();
        self
    }

    pub fn depending_on(mut self, entries: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.entry_deps = entries.into_iter().map(|e| e.into()).collect();
        self.entry_deps.sort();
        self.entry_deps.dedup();
        self
    }

    /// Whether this spec's scope admits the given world. An unscoped head
    /// admits everything — a deliberate (and rare) choice, not the default: the
    /// synthesis step scopes to the cell the traces came from.
    pub fn admits(&self, input: &DecisionInput) -> bool {
        if self.cells.is_empty() {
            return true;
        }
        input
            .cell()
            .map(|c| self.cells.contains(&c))
            .unwrap_or(false)
    }

    pub fn cardinality(&self) -> usize {
        self.declared_labels.len()
    }

    /// Whether this spec compiles the *same distinction* as `other`: same region,
    /// same views, same features.
    ///
    /// Used by the JIT so that a second synthesis pass over a region whose split is
    /// already compiled moves down the ladder instead of replacing the head with a
    /// near-identical one. Two heads that read the same thing in the same place are
    /// one head, whatever they are called.
    pub fn same_distinction_as(&self, other: &HeadSpec) -> bool {
        self.decision_type == other.decision_type
            && self.cells == other.cells
            && self.features == other.features
            && self.text_view == other.text_view
            && self.embedding_view == other.embedding_view
    }

    /// Whether `label` is inside the declared space. `true` for an open space
    /// with no declared labels, which is the honest answer: this spec does not
    /// close the space, so it cannot refuse a value for being outside it.
    pub fn admits_label(&self, label: &str) -> bool {
        self.declared_labels.is_empty() || self.declared_labels.contains(&label.to_lowercase())
    }
}

// ─ Training, i.e. the distillation step ─────────────────────────────────

/// One labeled observation, in the form a head is fit on.
///
/// `verdict` uses the crate's own [`crate::models::CoherenceRating`] rather than
/// a new enum: `Success` = the label was validated, `Failure` = it was refuted,
/// `Inert` = nobody judged it. An `Inert` example is *kept* — it is a trace, and
/// traces are the substrate — but it is down-weighted rather than silently
/// promoted to truth. That bridge is what lets [`crate::praxis`]'s verdicts and
/// [`crate::quality`]'s feedback train a head without a translation layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingExample<O> {
    pub input: DecisionInput,
    pub output: O,
    pub verdict: crate::models::CoherenceRating,
    /// The trace this example came from, for provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Weight in the fit, `[0, 1]`: derived from verdict and source reliability,
    /// never from the head being trained (that would be a closed loop).
    pub weight: f32,
}

impl<O: Clone> TrainingExample<O> {
    pub fn new(input: DecisionInput, output: O) -> Self {
        Self {
            input,
            output,
            verdict: crate::models::CoherenceRating::Success,
            trace_id: None,
            weight: 1.0,
        }
    }

    /// Weight from the verdict: validated 1.0, unjudged 0.5, refuted **kept out**
    /// of the positive set (a refuted label is a counterexample, and fitting on
    /// it is how a head learns to be wrong in a newly confident way).
    pub fn from_verdict(
        input: DecisionInput,
        output: O,
        verdict: crate::models::CoherenceRating,
    ) -> Self {
        let weight = match verdict {
            crate::models::CoherenceRating::Success => 1.0,
            crate::models::CoherenceRating::Inert => 0.5,
            crate::models::CoherenceRating::Failure => 0.0,
        };
        Self {
            input,
            output,
            verdict,
            trace_id: None,
            weight,
        }
    }

    pub fn usable(&self) -> bool {
        self.weight > 0.0
    }

    pub fn from_text(label: O, text: impl Into<String>) -> Self {
        Self::new(DecisionInput::example("ex", text, ""), label)
    }
}

/// What a fit did. A head that cannot be fit says so; it does not pretend to
/// have learned nothing successfully.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FitReport {
    /// The head learned from `n` examples in `micros`.
    Fitted { n: usize, micros: u64, note: String },
    /// The rung is not trainable (a rule, an oracle, a person). Not a failure.
    NotTrainable { why: String },
    /// The fit ran and the head declined to deploy: too few usable examples, or
    /// no signal above the control.
    Refused { why: String },
}

impl FitReport {
    pub fn fitted(n: usize, micros: u64, note: impl Into<String>) -> Self {
        FitReport::Fitted {
            n,
            micros,
            note: note.into(),
        }
    }

    pub fn not_trainable(why: impl Into<String>) -> Self {
        FitReport::NotTrainable { why: why.into() }
    }

    pub fn refused(why: impl Into<String>) -> Self {
        FitReport::Refused { why: why.into() }
    }

    pub fn is_deployable(&self) -> bool {
        matches!(self, FitReport::Fitted { .. })
    }
}

// ── The head traits ──────────────────────────────────────────────────────

/// A typed decision head. Implementors answer in exactly one label space;
/// the type parameter is what makes the contract checkable at compile time.
pub trait DecisionHead: Send + Sync {
    type Output: DecisionLabel;

    fn id(&self) -> &str;
    fn kind(&self) -> HeadKind;
    fn spec(&self) -> &HeadSpec;

    /// Answer, or abstain. Implementations must not panic on a malformed
    /// `DecisionInput`; an unreadable world is an abstention, not a crash.
    fn decide(&self, input: &DecisionInput) -> Decision<Self::Output>;

    /// Distil from labeled examples. Default: this rung is not trainable.
    fn fit(&mut self, _examples: &[TrainingExample<Self::Output>]) -> FitReport {
        FitReport::not_trainable(format!(
            "{} heads are declared, not fitted",
            self.kind().as_str()
        ))
    }

    /// Per-call cost to use *before* measurements exist.
    fn profile(&self) -> CostProfile {
        self.kind().nominal_profile()
    }
}

/// The object-safe view of a [`DecisionHead`].
///
/// Rust has no object-safe associated types, so the registry holds this and the
/// blanket impl below makes every typed head a `HeadFor` for free. This is the
/// only place the type parameter is erased, and the erasure is one-directional:
/// a [`Decision<O>`] built by a typed head is narrowed back to `O` by its
/// caller, never widened into a stringly-typed bag.
pub trait HeadFor<O>: Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> HeadKind;
    fn spec(&self) -> &HeadSpec;
    fn decide(&self, input: &DecisionInput) -> Decision<O>;
    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport;
    fn profile(&self) -> CostProfile;
}

impl<H, O> HeadFor<O> for H
where
    H: DecisionHead<Output = O>,
    O: DecisionLabel,
{
    fn id(&self) -> &str {
        DecisionHead::id(self)
    }
    fn kind(&self) -> HeadKind {
        DecisionHead::kind(self)
    }
    fn spec(&self) -> &HeadSpec {
        DecisionHead::spec(self)
    }
    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        DecisionHead::decide(self, input)
    }
    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        DecisionHead::fit(self, examples)
    }
    fn profile(&self) -> CostProfile {
        DecisionHead::profile(self)
    }
}

// ── The rung above: System-One oracles ───────────────────────────────────

/// TypeSafe's Jev binds a cardinality of up to 255 labels per decision.
/// <https://typesafe.ai/blog/introducing-system-one-models-and-jev>
///
/// Recorded here because it is a *binding constraint on head synthesis*, not a
/// curiosity: [`OracleSchema::validate`] refuses a spec whose label space the
/// binding cannot express, so the failure is caught when the head is proposed
/// rather than when it is first forced to answer.
pub const JEV_MAX_CARDINALITY: usize = 255;

/// The static schema a remote System-One oracle is bound to.
///
/// The whole advantage of the System-One shape is that the output space is
/// fixed in advance, which is what makes parallel sampling possible: N open
/// decisions share one query because the schema does not change per call. The
/// fields here are the ones that make that claim checkable locally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleSchema {
    pub name: String,
    pub kind: HeadKind,
    pub cardinality: usize,
    pub allows_multi_label: bool,
    /// Whether the oracle answers N decisions in one round trip. A Jev-class
    /// oracle does; a chat-shaped LLM adapter does not, and must say so.
    pub parallel_sampling: bool,
    /// Whether the oracle's own probabilities are calibrated, in the sense the
    /// vendor claims (RLCD-trained for Jev). Adapters that guess must set
    /// `false` and let [`calibration`] fix it locally.
    pub calibrated: bool,
}

impl OracleSchema {
    /// A Jev-class binding: static schema, parallel, calibrated.
    pub fn system_one(name: impl Into<String>, cardinality: usize) -> Self {
        Self {
            name: name.into(),
            kind: HeadKind::Jev,
            cardinality,
            allows_multi_label: false,
            parallel_sampling: true,
            calibrated: true,
        }
    }

    /// A chat-shaped LLM: one decision per call, uncalibrated by construction.
    pub fn chat(name: impl Into<String>, cardinality: usize) -> Self {
        Self {
            name: name.into(),
            kind: HeadKind::Llm,
            cardinality,
            allows_multi_label: false,
            parallel_sampling: false,
            calibrated: false,
        }
    }

    pub fn validate(&self, spec: &HeadSpec) -> Result<(), SchemaError> {
        if spec.cardinality() > self.cardinality {
            return Err(SchemaError::CardinalityExceedsBinding {
                declared: spec.cardinality(),
                bound: self.cardinality,
            });
        }
        if spec.cardinality() > JEV_MAX_CARDINALITY && self.kind == HeadKind::Jev {
            return Err(SchemaError::CardinalityExceedsBinding {
                declared: spec.cardinality(),
                bound: JEV_MAX_CARDINALITY,
            });
        }
        Ok(())
    }

    /// How many round trips `n` open decisions cost on this oracle. The
    /// difference between `n` and `1` is most of the System-One case, so it is
    /// computed rather than asserted.
    pub fn round_trips(&self, n: usize) -> usize {
        if n == 0 {
            0
        } else if self.parallel_sampling {
            1
        } else {
            n
        }
    }
}

/// Why a spec cannot be bound to an oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// The label space is larger than the binding expresses. For Jev the ceiling
    /// is 255.
    CardinalityExceedsBinding { declared: usize, bound: usize },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::CardinalityExceedsBinding { declared, bound } => write!(
                f,
                "label space has {declared} labels; this oracle binds {bound}"
            ),
        }
    }
}

impl std::error::Error for SchemaError {}

/// The strong rung, behind a trait.
///
/// `physis-core` implements this for an OpenAI-compatible endpoint only under
/// the `http` feature (`heads::HttpOracle`); a provider cascade can implement it
/// directly. Tests and the experiment implement it with a
/// deterministic stub ([`heads::StubOracle`]) so the loop is exercised without a
/// key or a network — the stub is labelled as a stand-in everywhere it appears,
/// and never as a model.
pub trait DecisionOracle<O: DecisionLabel>: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> HeadKind;
    fn schema(&self) -> OracleSchema;

    fn decide(&self, input: &DecisionInput, spec: &HeadSpec) -> Decision<O>;

    /// Answer several decisions. The default is sequential and says so: an
    /// oracle that batches must override this, because the *point* of the
    /// System-One shape is that N decisions are one query, and a sequential
    /// default would quietly throw that away.
    fn decide_batch(&self, inputs: &[DecisionInput], spec: &HeadSpec) -> Vec<Decision<O>> {
        inputs.iter().map(|i| self.decide(i, spec)).collect()
    }

    fn profile(&self) -> CostProfile {
        self.kind().nominal_profile()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    label_set! {
        /// A test label space.
        pub enum Triage { Repair, Observe, Escalate }
    }

    #[test]
    fn label_set_round_trips_and_refuses_anything_else() {
        assert_eq!(Triage::Repair.key(), "repair");
        assert_eq!(Triage::Escalate.key(), "escalate");
        assert_eq!(Triage::from_key("REPAIR"), Some(Triage::Repair));
        assert_eq!(Triage::from_key(" replace "), None);
        assert_eq!(Triage::ALL.len(), 3);
        assert_eq!(<Triage as DecisionLabel>::cardinality(), Some(3));
    }

    #[test]
    fn open_label_spaces_parse_but_do_not_close() {
        assert!(<String as DecisionLabel>::cardinality().is_none());
        assert_eq!(String::from_key("  "), None);
        let spec = HeadSpec::new("x.y", Vec::<String>::new(), Transparency::Opaque);
        assert!(spec.admits_label("anything"));
        let closed = spec.with_labels(["repair", "observe"]);
        assert!(closed.admits_label("Repair"));
        assert!(!closed.admits_label("run"));
        assert_eq!(closed.cardinality(), 2);
    }

    #[test]
    fn label_space_digest_separates_disagreeing_specs() {
        let a = HeadSpec::new("m.action", ["a", "b"], Transparency::Exact);
        let b = HeadSpec::new("m.action", ["a", "c"], Transparency::Exact);
        let c = HeadSpec::new("m.action", ["b", "a"], Transparency::Exact);
        assert_ne!(a.label_space, b.label_space);
        assert_eq!(
            a.label_space, c.label_space,
            "declaration order is not the contract"
        );
    }

    #[test]
    fn abstention_is_a_value_not_an_error() {
        let p = Provenance::head("rule-1", HeadKind::Rule, "m.action#deadbeef");
        let d: Decision<Triage> = Decision::abstain(p, Cost::ZERO);
        assert!(d.did_abstain());
        assert_eq!(d.confidence, 0.0);
        assert!(d.entropy().is_none(), "no distribution is not certainty");
        assert_eq!(d.margin(), 0.0);
    }

    #[test]
    fn answering_keeps_raw_and_calibrated_confidence_apart() {
        let p = Provenance::head("knn-1", HeadKind::Knn, "m.action#x");
        let dist = vec![
            Mass::new(Triage::Repair, 0.8),
            Mass::new(Triage::Observe, 0.2),
        ];
        let d = Decision::answered(Triage::Repair, dist, 0.8, p, Cost::ZERO);
        assert!((d.margin() - 0.6).abs() < 1e-6);
        let e = d.entropy().unwrap();
        let expect = (-(0.8f32 * 0.8f32.ln() + 0.2 * 0.2f32.ln())).abs();
        assert!((e - expect).abs() < 1e-5);
        let c = d.with_calibrated_confidence(0.63, "gate-spec");
        assert!((c.confidence - 0.63).abs() < 1e-6);
        assert!(
            (c.raw_confidence - 0.8).abs() < 1e-6,
            "raw confidence is evidence, not scratch space"
        );
        assert_eq!(c.provenance.calibrated_by.as_deref(), Some("gate-spec"));
    }

    #[test]
    fn cost_sums_keep_the_weakest_provenance() {
        let measured = Cost {
            micros: 3,
            ..Cost::ZERO
        };
        let nominal = Cost::from_profile(&CostProfile::local(5, 0));
        let sum = measured.plus(nominal);
        assert_eq!(sum.micros, 8);
        assert_eq!(sum.source, CostSourceMarker::Nominal);
    }

    #[test]
    fn remote_cost_is_nonzero_and_local_cost_is_free() {
        let jev = HeadKind::Jev.nominal_profile();
        let llm = HeadKind::Llm.nominal_profile();
        let rule = HeadKind::Rule.nominal_profile();
        assert!(jev.usd_micros_per_call() > 0);
        assert!(rule.usd_micros_per_call() == 0);
        assert!(
            llm.usd_micros_per_call() > jev.usd_micros_per_call(),
            "the frontier rung must be the more expensive one"
        );
        let c = Cost::from_profile(&llm);
        assert_eq!(c.tokens_in, pricing::REMOTE_DECISION_TOKENS_IN);
        assert_eq!(c.source, CostSourceMarker::Vendor);
    }

    #[test]
    fn feature_keys_are_the_shared_vocabulary() {
        assert_eq!(FeatureValue::Bool(true).key(), "true");
        assert_eq!(FeatureValue::Int(7).key(), "7");
        assert_eq!(FeatureValue::Text("Repair".into()).key(), "repair");
        assert_eq!(FeatureValue::Bool(true).as_f64(), Some(1.0));
        assert_eq!(FeatureValue::Text("x".into()).as_f64(), None);
    }

    #[test]
    fn cell_key_matches_the_crate_wide_format() {
        assert_eq!(cell_key("HEAL", "REST"), format!("HEAL\u{0}REST"));
        let w = WorldSlice::new("w1", "t").with_cell("HEAL", "REST");
        assert_eq!(w.cell_key().as_deref(), Some("HEAL\u{0}REST"));
    }

    #[test]
    fn hard_invariants_are_violated_by_missing_keys_only() {
        let o = OntologySlice {
            invariants: vec![
                Invariant {
                    id: "i1".into(),
                    statement: "every maintenance world carries a machine id".into(),
                    features: vec!["machine.id".into()],
                    hard: true,
                },
                Invariant {
                    id: "i2".into(),
                    statement: "soft: usually has a shift".into(),
                    features: vec!["shift".into()],
                    hard: false,
                },
            ],
            ..Default::default()
        };
        let empty = WorldSlice::new("w", "");
        assert_eq!(o.violated(&empty).len(), 1);
        let ok = WorldSlice::new("w", "").with_state("machine.id", FeatureValue::Text("m1".into()));
        assert!(o.violated(&ok).is_empty());
    }

    #[test]
    fn delta_priority_ranks_the_region_that_moved() {
        let o = OntologySlice {
            deltas: vec![
                SemanticDelta::new(cell_key("HEAL", "REST"), 0.9).because("mode axis re-anchored"),
                SemanticDelta::new("pump maintenance", 0.3),
            ],
            ..Default::default()
        };
        let hot = o.delta_priority(Some(&cell_key("HEAL", "REST")), &[]);
        assert!((hot - 0.9).abs() < 1e-6);
        let cold = o.delta_priority(Some(&cell_key("BOND", "WORK")), &[]);
        assert_eq!(cold, 0.0);
        let by_entry = o.delta_priority(None, &["pump maintenance".to_string()]);
        assert!((by_entry - 0.3).abs() < 1e-6);
    }

    #[test]
    fn an_oracle_refuses_a_label_space_it_cannot_bind() {
        let big = HeadSpec::new(
            "cell.route",
            (0..300).map(|i| format!("l{i}")),
            Transparency::Opaque,
        );
        let jev = OracleSchema::system_one("jev", JEV_MAX_CARDINALITY);
        assert!(matches!(
            jev.validate(&big),
            Err(SchemaError::CardinalityExceedsBinding {
                declared: 300,
                bound: 255
            })
        ));
        let small = HeadSpec::new("cell.route", ["a", "b"], Transparency::Opaque);
        assert!(jev.validate(&small).is_ok());
        assert_eq!(
            jev.round_trips(64),
            1,
            "a static schema answers in one query"
        );
        assert_eq!(OracleSchema::chat("gpt", 16).round_trips(64), 64);
    }

    #[test]
    fn training_examples_carry_the_verdict_into_the_weight() {
        use crate::models::CoherenceRating;
        let input = DecisionInput::example("w", "text", "repair");
        let good = TrainingExample::new(input.clone(), Triage::Repair);
        let unjudged =
            TrainingExample::from_verdict(input.clone(), Triage::Repair, CoherenceRating::Inert);
        let refuted =
            TrainingExample::from_verdict(input, Triage::Repair, CoherenceRating::Failure);
        assert_eq!(good.weight, 1.0);
        assert_eq!(unjudged.weight, 0.5);
        assert!(
            !refuted.usable(),
            "a refuted label must not be fitted as truth"
        );
    }

    #[test]
    fn spec_scope_is_the_invalidation_region() {
        let spec = HeadSpec::new("m.action", ["repair", "observe"], Transparency::Exact)
            .scoped_to([cell_key("HEAL", "REST")])
            .reading(["sensor.vibration"])
            .depending_on(["pump maintenance"]);
        let inside = DecisionInput::new(
            WorldSlice::new("w", "").with_cell("HEAL", "REST"),
            OntologySlice::empty(),
        );
        let outside = DecisionInput::new(
            WorldSlice::new("w", "").with_cell("BOND", "WORK"),
            OntologySlice::empty(),
        );
        assert!(spec.admits(&inside));
        assert!(!spec.admits(&outside));
        assert!(!spec.text_view && !spec.embedding_view);
    }
}

fn label_space_digest(decision_type: &str, labels: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(decision_type.as_bytes());
    h.update(b"\x1f");
    for l in labels {
        h.update(l.as_bytes());
        h.update(b"\x1f");
    }
    format!("{decision_type}#{:x}", h.finalize())
}
