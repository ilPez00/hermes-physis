//! The interchangeable backends.
//!
//! Every type here implements [`crate::system_one::DecisionHead`] (or, for the
//! remote rungs, [`crate::system_one::DecisionOracle`] wrapped by
//! [`OracleHead`]). The rule for this file: **no backend may be a stub**. If a
//! rung cannot answer, it abstains and its [`crate::system_one::Provenance`]
//! says which rung it was and why. There is exactly one type in here that is not
//! a decision procedure — [`StubOracle`] — and it is named, documented and
//! labelled as a stand-in so a report can never mistake it for a model.
//!
//! Existing crate machinery is reused rather than re-derived:
//!
//! | rung | built on |
//! |---|---|
//! | [`CountTableHead`] | [`crate::ngram_table`] — backed-off counts, smoothing, pruning, support statistics |
//! | [`ModelHead`] | [`crate::model_provider`] — the interchangeable-model trait; `NgramDecoderModel` offline |
//! | [`LexicalHead`] | [`crate::tokenizer`] — one tokenizer for tables, BM25 and patterns |
//! | [`PrototypeHead`], [`KnnHead`] | [`crate::models::cosine_sim`], the similarity the grid classifier already scores with |
//! | [`OracleHead`] | [`crate::oracle`] under the `http` feature |

use std::collections::{BTreeMap, HashMap};

use crate::model_provider::ModelProvider;
use crate::models::cosine_sim;
use crate::ngram_table::{NGramTable, TableBuilder, TableConfig, TableKind};
use crate::system_one::{
    Cost, Decision, DecisionHead, DecisionInput, DecisionLabel, DecisionOracle, FeatureValue,
    FitReport, HeadKind, HeadSpec, Mass, OracleSchema, Provenance, SchemaError, TrainingExample,
};
use crate::tokenizer::{Tokenizer, WhitespaceTokenizer};

/// The tokenizer every lexical backend in this module shares. One tokenizer
/// means a term a pattern matches, a term BM25 scores and a token a count table
/// counts are the *same* term.
pub fn tokenizer() -> WhitespaceTokenizer {
    WhitespaceTokenizer::new(0)
}

/// Lower-cased text, for patterns and provenance strings.
fn lower(s: &str) -> String {
    s.to_lowercase()
}

// ─ Rule: deterministic predicates over typed state ──────────────────────

/// A predicate over a [`crate::system_one::WorldSlice`].
///
/// Small on purpose. Every variant is checkable by eye, which is what makes a
/// rule head auditable: a distinction that cannot be written here is one that
/// belongs on a counted or retrieved rung, where the cost of losing readability
/// is paid deliberately rather than by accident.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expr {
    /// Always true. Only ever produced by an explicitly unscoped rule.
    Always,
    /// `state[key] == value`, compared through [`FeatureValue::key`].
    Eq {
        key: String,
        value: FeatureValue,
    },
    /// `state[key]` is one of `values`.
    In {
        key: String,
        values: Vec<FeatureValue>,
    },
    /// Numeric comparison; `None` when the key is missing or non-numeric.
    Lt {
        key: String,
        bound: f64,
    },
    Gt {
        key: String,
        bound: f64,
    },
    /// `world.text` contains `needle` (lower-cased substring).
    TextContains {
        needle: String,
    },
    /// All children hold. Unknown stays unknown, it does not become false.
    All(Vec<Expr>),
    /// Any child holds.
    Any(Vec<Expr>),
    Not(Box<Expr>),
}

impl Expr {
    /// Evaluate against a world. `None` = the world does not carry the keys this
    /// predicate reads, which is *not* the same as `false`: a head whose
    /// predicate cannot be evaluated must abstain, not answer `no`.
    pub fn eval(&self, input: &DecisionInput) -> Option<bool> {
        match self {
            Expr::Always => Some(true),
            Expr::Eq { key, value } => input.world.state.get(key).map(|v| v.key() == value.key()),
            Expr::In { key, values } => input.world.state.get(key).map(|v| {
                let k = v.key();
                values.iter().any(|candidate| candidate.key() == k)
            }),
            Expr::Lt { key, bound } => input
                .world
                .state
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|x| x < *bound),
            Expr::Gt { key, bound } => input
                .world
                .state
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|x| x > *bound),
            Expr::TextContains { needle } => {
                Some(lower(&input.world.text).contains(&needle.to_lowercase()))
            }
            Expr::All(children) => {
                let mut unknown = false;
                for c in children {
                    match c.eval(input) {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => unknown = true,
                    }
                }
                if unknown {
                    None
                } else {
                    Some(true)
                }
            }
            Expr::Any(children) => {
                let mut unknown = false;
                for c in children {
                    match c.eval(input) {
                        Some(true) => return Some(true),
                        Some(false) => {}
                        None => unknown = true,
                    }
                }
                if unknown {
                    None
                } else {
                    Some(false)
                }
            }
            Expr::Not(inner) => inner.eval(input).map(|b| !b),
        }
    }

    /// Human-readable form, for [`Provenance::derivation`].
    pub fn render(&self) -> String {
        match self {
            Expr::Always => "always".to_string(),
            Expr::Eq { key, value } => format!("{key} == {}", value.key()),
            Expr::In { key, values } => format!(
                "{key} in [{}]",
                values
                    .iter()
                    .map(|v| v.key())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Lt { key, bound } => format!("{key} < {bound}"),
            Expr::Gt { key, bound } => format!("{key} > {bound}"),
            Expr::TextContains { needle } => format!("text contains \"{needle}\""),
            Expr::All(c) => c
                .iter()
                .map(|e| e.render())
                .collect::<Vec<_>>()
                .join(" and "),
            Expr::Any(c) => format!(
                "({})",
                c.iter()
                    .map(|e| e.render())
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
            Expr::Not(e) => format!("not({})", e.render()),
        }
    }
}

/// One learned or declared rule: *when this, answer that, and here is how often
/// it held*.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rule<O> {
    pub when: Expr,
    pub then: O,
    /// Examples that fell inside the predicate during the fit.
    pub support: usize,
    /// Share of those that carried `then`.
    pub precision: f32,
}

impl<O> Rule<O> {
    /// Confidence of this rule as an answer: its precision, **not 1.0**. A rule
    /// whose precision is 0.7 is a 0.7, and [`crate::system_one::calibration`]
    /// is what decides whether 0.7 is good enough to route on.
    pub fn confidence(&self) -> f32 {
        self.precision.clamp(0.0, 1.0)
    }
}

/// Knobs of [`RuleHead::learn`]. The defaults are deliberately conservative: a
/// rule that fires on three examples is a coincidence with nice syntax.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleConfig {
    pub min_support: usize,
    pub min_precision: f32,
    /// How far above the region's base rate a rule must sit.
    pub min_lift: f32,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            min_support: 5,
            min_precision: 0.8,
            min_lift: 0.2,
        }
    }
}

/// A declared predicate table. The cheapest rung, and the only one whose
/// [`crate::system_one::Transparency`] is [`crate::system_one::Transparency::Exact`].
#[derive(Debug, Clone)]
pub struct RuleHead<O> {
    id: String,
    spec: HeadSpec,
    rules: Vec<Rule<O>>,
}

impl<O: DecisionLabel> RuleHead<O> {
    pub fn new(id: impl Into<String>, spec: HeadSpec, rules: Vec<Rule<O>>) -> Self {
        Self {
            id: id.into(),
            spec,
            rules,
        }
    }

    pub fn rules(&self) -> &[Rule<O>] {
        &self.rules
    }

    /// Learn a rule set from labeled examples.
    ///
    /// For every declared feature key, one rule per observed value, kept when it
    /// carries enough support **and** beats the base rate of its own region by
    /// `min_lift`. The base-rate comparison is the control: without it, a feature
    /// that predicts the majority label everywhere yields a rule for the majority
    /// label and nothing else, which looks like learning and is not.
    pub fn learn(
        id: impl Into<String>,
        spec: HeadSpec,
        examples: &[TrainingExample<O>],
        cfg: &RuleConfig,
    ) -> RuleHead<O> {
        let usable: Vec<&TrainingExample<O>> = examples.iter().filter(|e| e.usable()).collect();
        let mut rules: Vec<Rule<O>> = Vec::new();
        if usable.is_empty() {
            return RuleHead::new(id, spec, rules);
        }
        let base = base_rates(&usable);

        for key in &spec.features {
            let mut by_value: BTreeMap<String, HashMap<String, (usize, O)>> = BTreeMap::new();
            for e in &usable {
                let Some(v) = e.input.world.state.get(key) else {
                    continue;
                };
                let bucket = by_value.entry(v.key()).or_default();
                let entry = bucket
                    .entry(e.output.as_key())
                    .or_insert((0, e.output.clone()));
                entry.0 += 1;
            }
            for (value, outcomes) in by_value {
                let support = outcomes.values().map(|(n, _)| *n).sum::<usize>();
                if support < cfg.min_support {
                    continue;
                }
                let (best_label, best_n) = outcomes
                    .values()
                    .max_by_key(|(n, _)| *n)
                    .map(|(n, o)| (o.clone(), *n))
                    .expect("outcomes is non-empty");
                let precision = best_n as f32 / support as f32;
                let prior = base.get(&best_label.as_key()).copied().unwrap_or(0.0);
                if precision >= cfg.min_precision && precision >= prior + cfg.min_lift {
                    rules.push(Rule {
                        when: Expr::Eq {
                            key: key.clone(),
                            value: parse_like(&value, &usable, key),
                        },
                        then: best_label,
                        support,
                        precision,
                    });
                }
            }
        }

        // Higher precision first, so a narrow rule is not shadowed by a broad one
        // built from a different key.
        rules.sort_by(|a, b| {
            b.precision
                .partial_cmp(&a.precision)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.support.cmp(&a.support))
                .then(a.when.render().cmp(&b.when.render()))
        });
        RuleHead::new(id, spec, rules)
    }
}

/// Restore the typed form of a value key when it round-trips, else keep it as
/// text. `Eq` compares through [`FeatureValue::key`] anyway, so this only affects
/// how the rule reads in a log.
fn parse_like<O>(key_str: &str, examples: &[&TrainingExample<O>], key: &str) -> FeatureValue {
    for e in examples {
        if let Some(v) = e.input.world.state.get(key) {
            if v.key() == key_str {
                return v.clone();
            }
        }
    }
    FeatureValue::Text(key_str.to_string())
}

fn base_rates<O: DecisionLabel>(examples: &[&TrainingExample<O>]) -> HashMap<String, f32> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in examples {
        *counts.entry(e.output.as_key()).or_insert(0) += 1;
    }
    let n = examples.len() as f32;
    counts.into_iter().map(|(k, c)| (k, c as f32 / n)).collect()
}

impl<O: DecisionLabel> DecisionHead for RuleHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Rule
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Rule, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if !self.spec.admits(input) {
            return Decision::abstain(provenance, cost);
        }
        let mut distribution: Vec<Mass<O>> = Vec::new();
        let mut fired: Vec<String> = Vec::new();
        let mut best_precision = 0.0f32;
        for rule in &self.rules {
            if rule.when.eval(input) == Some(true) {
                fired.push(format!("{} → {}", rule.when.render(), rule.then.as_key()));
                let p = rule.confidence();
                match distribution.iter_mut().find(|m| m.value == rule.then) {
                    Some(m) => m.probability += p,
                    None => distribution.push(Mass::new(rule.then.clone(), p)),
                }
            }
        }
        if distribution.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        // The confidence is the **strongest fired rule's precision**, not the
        // normalized share: a single rule whose precision is 0.7 must report 0.7,
        // and normalizing one mass to 1.0 would silently promote it to certainty.
        // The distribution stays normalized because that is what a distribution is.
        let winning = distribution
            .iter()
            .max_by(|a, b| {
                a.probability
                    .partial_cmp(&b.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|m| m.value.clone())
            .expect("distribution is non-empty");
        for rule in &self.rules {
            if rule.then == winning && rule.when.eval(input) == Some(true) {
                best_precision = best_precision.max(rule.confidence());
            }
        }
        let total: f32 = distribution.iter().map(|m| m.probability).sum();
        for m in distribution.iter_mut() {
            m.probability /= total.max(1e-8);
        }
        distribution.sort_by(|a, b| {
            b.probability
                .partial_cmp(&a.probability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top = distribution[0].clone();
        // A fired rule's precision is the evidence; the posterior share is what
        // the distribution is for. Only when no rule carries a precision does the
        // posterior stand in as the confidence.
        let confidence = if best_precision > 0.0 {
            best_precision
        } else {
            top.probability
        };
        Decision::answered(
            top.value,
            distribution,
            confidence,
            provenance.because_of(format!(
                "{} rule(s) fired: {}",
                fired.len(),
                fired.join("; ")
            )),
            cost,
        )
    }

    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        let started = std::time::Instant::now();
        let learned = RuleHead::learn(
            &self.id,
            self.spec.clone(),
            examples,
            &RuleConfig::default(),
        );
        if learned.rules.is_empty() {
            return FitReport::refused("no feature split cleared support/precision/lift");
        }
        let n = learned.rules.len();
        self.rules = learned.rules;
        FitReport::fitted(
            n,
            started.elapsed().as_micros() as u64,
            "rules learned from feature splits that beat the base rate",
        )
    }
}

// ─ Pattern: lexical cues, no model ──────────────────────────────────────

/// How a pattern matches.
///
/// `Literal` and `Prefix` are dependency-free and cover most operational cues
/// ("vibration", "leak ", "err-"). `Regex` is available behind the
/// **`head-regex`** feature, which pulls in the `regex` crate (MIT/Apache-2.0)
/// rather than re-implementing a regex engine here — a hand-rolled matcher would
/// be exactly the kind of rebuild this design forbids. Without the feature the
/// [`PatternHead`] still works; a [`PatternKind::Regex`] rule is reported as
/// unavailable rather than silently matching nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    /// Case-insensitive substring.
    Literal,
    /// Case-insensitive prefix of the text.
    Prefix,
    /// Regular expression (requires the `head-regex` feature).
    Regex,
}

impl PatternKind {
    pub fn available(self) -> bool {
        match self {
            PatternKind::Literal | PatternKind::Prefix => true,
            PatternKind::Regex => cfg!(feature = "head-regex"),
        }
    }
}

/// One pattern and the label it asserts.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PatternRule<O> {
    pub kind: PatternKind,
    pub pattern: String,
    pub then: O,
    /// Examples the pattern matched during the fit.
    pub support: usize,
    pub precision: f32,
}

/// A text-cue head: literal/prefix/regex patterns over `world.text`.
///
/// Trained or declared. Training keeps the tokens that discriminate a label from
/// the base rate — the same control [`RuleHead::learn`] applies to state keys —
/// as literals. A caller can hand-build regex rules through
/// [`PatternHead::new`] when a person knows the cue and no trace has shown it yet.
#[derive(Debug, Clone)]
pub struct PatternHead<O> {
    id: String,
    spec: HeadSpec,
    rules: Vec<PatternRule<O>>,
    #[cfg(feature = "head-regex")]
    compiled: Vec<(usize, regex::Regex)>,
}

impl<O: DecisionLabel> PatternHead<O> {
    /// Declare patterns by hand. Regex rules are compiled by
    /// `with_compiled` (which exists only with the `head-regex` feature) when it is on; a rule
    /// whose kind is unavailable is kept but reports as unmatchable rather than
    /// being silently dropped.
    pub fn new(id: impl Into<String>, spec: HeadSpec, rules: Vec<PatternRule<O>>) -> Self {
        Self {
            id: id.into(),
            spec,
            rules,
            #[cfg(feature = "head-regex")]
            compiled: Vec::new(),
        }
    }

    #[cfg(feature = "head-regex")]
    pub fn with_compiled(mut self) -> Self {
        self.recompile();
        self
    }

    pub fn rules(&self) -> &[PatternRule<O>] {
        &self.rules
    }

    fn recompile(&mut self) {
        #[cfg(feature = "head-regex")]
        {
            self.compiled = self
                .rules
                .iter()
                .enumerate()
                .filter(|(_, r)| r.kind == PatternKind::Regex)
                .filter_map(|(i, r)| {
                    regex::Regex::new(&format!("(?i){}", r.pattern))
                        .ok()
                        .map(|re| (i, re))
                })
                .collect();
        }
    }

    /// Learn literal patterns from the tokens that separate labels.
    ///
    /// For each token present in at least `min_support` examples, compare the
    /// label distribution of the examples containing it against the base rate. A
    /// token that does not beat the base rate by `min_lift` is a word, not a cue.
    pub fn learn(
        id: impl Into<String>,
        spec: HeadSpec,
        examples: &[TrainingExample<O>],
        min_support: usize,
        min_precision: f32,
        min_lift: f32,
    ) -> Self {
        let tok = tokenizer();
        let usable: Vec<&TrainingExample<O>> = examples.iter().filter(|e| e.usable()).collect();
        let base = base_rates(&usable);
        let mut seen: BTreeMap<String, HashMap<String, usize>> = BTreeMap::new();
        for e in &usable {
            let mut toks: Vec<String> = tok.encode(&e.input.world.text);
            toks.sort();
            toks.dedup();
            for t in toks {
                *seen
                    .entry(t)
                    .or_default()
                    .entry(e.output.as_key())
                    .or_insert(0) += 1;
            }
        }
        let mut rules = Vec::new();
        for (token, counts) in seen {
            let support: usize = counts.values().sum();
            if support < min_support {
                continue;
            }
            let (label, hits) = counts
                .iter()
                .max_by_key(|(_, c)| **c)
                .map(|(l, c)| (l.clone(), *c))
                .expect("counts is non-empty");
            let precision = hits as f32 / support as f32;
            let prior = base.get(&label).copied().unwrap_or(0.0);
            if precision >= min_precision && precision >= prior + min_lift {
                let Some(then) = O::from_key(&label) else {
                    continue;
                };
                rules.push(PatternRule {
                    kind: PatternKind::Literal,
                    pattern: token,
                    then,
                    support,
                    precision,
                });
            }
        }
        rules.sort_by(|a, b| {
            b.precision
                .partial_cmp(&a.precision)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.support.cmp(&a.support))
                .then(a.pattern.cmp(&b.pattern))
        });
        PatternHead::new(id, spec, rules)
    }

    fn matches(&self, index: usize, rule: &PatternRule<O>, text_lower: &str) -> bool {
        match rule.kind {
            PatternKind::Literal => text_lower.contains(&rule.pattern.to_lowercase()),
            PatternKind::Prefix => text_lower.starts_with(&rule.pattern.to_lowercase()),
            PatternKind::Regex => {
                #[cfg(feature = "head-regex")]
                {
                    self.compiled
                        .iter()
                        .find(|(i, _)| *i == index)
                        .map(|(_, re)| re.is_match(text_lower))
                        .unwrap_or(false)
                }
                #[cfg(not(feature = "head-regex"))]
                {
                    let _ = (index, text_lower);
                    false
                }
            }
        }
    }
}

impl<O: DecisionLabel> DecisionHead for PatternHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Pattern
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Pattern, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if !self.spec.admits(input) {
            return Decision::abstain(provenance, cost);
        }
        let text_lower = lower(&input.world.text);
        let mut distribution: Vec<Mass<O>> = Vec::new();
        let mut fired: Vec<String> = Vec::new();
        let mut best_precision = 0.0f32;
        for (i, rule) in self.rules.iter().enumerate() {
            if self.matches(i, rule, &text_lower) {
                fired.push(format!("{} → {}", rule.pattern, rule.then.as_key()));
                match distribution.iter_mut().find(|m| m.value == rule.then) {
                    Some(m) => m.probability += rule.precision,
                    None => distribution.push(Mass::new(rule.then.clone(), rule.precision)),
                }
            }
        }
        if distribution.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let winning = distribution
            .iter()
            .max_by(|a, b| {
                a.probability
                    .partial_cmp(&b.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|m| m.value.clone())
            .expect("distribution is non-empty");
        for (i, rule) in self.rules.iter().enumerate() {
            if rule.then == winning && self.matches(i, rule, &text_lower) {
                best_precision = best_precision.max(rule.precision.clamp(0.0, 1.0));
            }
        }
        let total: f32 = distribution.iter().map(|m| m.probability).sum();
        for m in distribution.iter_mut() {
            m.probability /= total.max(1e-8);
        }
        distribution.sort_by(|a, b| {
            b.probability
                .partial_cmp(&a.probability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top = distribution[0].clone();
        let confidence = if best_precision > 0.0 {
            best_precision
        } else {
            top.probability
        };
        Decision::answered(
            top.value,
            distribution,
            confidence,
            provenance.because_of(format!(
                "{} pattern(s) matched: {}",
                fired.len(),
                fired.join(", ")
            )),
            cost,
        )
    }

    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        let started = std::time::Instant::now();
        let learned = PatternHead::learn(&self.id, self.spec.clone(), examples, 5, 0.8, 0.2);
        if learned.rules.is_empty() {
            return FitReport::refused("no token cleared support/precision/lift");
        }
        let n = learned.rules.len();
        self.rules = learned.rules;
        self.recompile();
        FitReport::fitted(
            n,
            started.elapsed().as_micros() as u64,
            "literal patterns learned from discriminating tokens",
        )
    }
}

// ─ Shared scoring: a distribution from label scores ─────────────────────

/// Softmax over `(label, score)` pairs, ordered descending.
///
/// Shared by every retrieval rung so that "how did a head turn scores into a
/// distribution" has one answer in this crate rather than four. `temperature`
/// controls how sharply the rung discriminates; it is a declared parameter of the
/// head and never something the router tunes silently.
fn to_distribution<O: DecisionLabel>(scored: Vec<(O, f32)>, temperature: f32) -> Vec<Mass<O>> {
    if scored.is_empty() {
        return Vec::new();
    }
    let t = temperature.max(1e-3);
    let max = scored
        .iter()
        .map(|(_, s)| *s)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut total = 0.0f32;
    let mut out: Vec<Mass<O>> = Vec::with_capacity(scored.len());
    for (value, score) in scored {
        let e = ((score - max) / t).exp();
        total += e;
        out.push(Mass::new(value, e));
    }
    for m in out.iter_mut() {
        m.probability /= total.max(1e-12);
    }
    out.sort_by(|a, b| {
        b.probability
            .partial_cmp(&a.probability)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.value.as_key().cmp(&b.value.as_key()))
    });
    out
}

// ── Count table: the crate's own n-gram machinery as a head ──────────────

/// A decision head over [`crate::ngram_table::BackoffTable`].
///
/// Training appends the label key as the final token of each example's token
/// sequence, so the table's conditional — `P(next | context)` with
/// stupid-backoff and add-k smoothing — is read as *`P(label | last tokens of the
/// text)`*. Nothing new is counted here: the builder, the pruning, the vocabulary
/// cap, the smoothing constants, the manifest and its serialization are the ones
/// [`crate::ngram_table`] already ships and tests. This head contributes the
/// reading of that table as a label space, plus the posterior over labels.
#[derive(Clone)]
pub struct CountTableHead<O> {
    id: String,
    spec: HeadSpec,
    table: Option<std::sync::Arc<dyn NGramTable>>,
    labels: Vec<O>,
    /// How much of the text is used as context; matches the table's `max_order`.
    order: usize,
    temperature: f32,
    fitted_from: usize,
}

impl<O: DecisionLabel> CountTableHead<O> {
    /// An unfitted head: it abstains until [`DecisionHead::fit`] runs.
    pub fn new(id: impl Into<String>, spec: HeadSpec, order: u8) -> Self {
        Self {
            id: id.into(),
            spec,
            table: None,
            labels: Vec::new(),
            order: order.max(1) as usize,
            temperature: 0.05,
            fitted_from: 0,
        }
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }

    /// Adopt an already-built table (e.g. one loaded from disk with
    /// [`crate::ngram_table::load`]) instead of fitting a new one. `labels` is the
    /// declared space; the table must have been built with those label keys as
    /// final tokens.
    pub fn from_table(
        id: impl Into<String>,
        spec: HeadSpec,
        table: std::sync::Arc<dyn NGramTable>,
        labels: Vec<O>,
    ) -> Self {
        let order = (table.manifest().max_order as usize).max(1);
        Self {
            id: id.into(),
            spec,
            table: Some(table),
            labels,
            order,
            temperature: 0.05,
            fitted_from: 0,
        }
    }

    fn build(
        examples: &[TrainingExample<O>],
        cfg: TableConfig,
        order: usize,
    ) -> Option<std::sync::Arc<dyn NGramTable>> {
        let tok = tokenizer();
        let usable: Vec<&TrainingExample<O>> = examples.iter().filter(|e| e.usable()).collect();
        if usable.is_empty() {
            return None;
        }
        let mut builder = TableBuilder::new(cfg);
        for e in &usable {
            let mut seq = tok.encode(&e.input.world.text);
            seq.push(e.output.as_key());
            builder.push_sequence(&seq);
        }
        // The fingerprint is structural, not a content hash: same examples in any
        // order produce the same table, so the identity of the fit is the count
        // and the order rather than a byte hash of the corpus.
        let fingerprint = format!("system-one:{}:{}", usable.len(), order);
        let (table, _bytes) = builder.finish(&tok, &fingerprint);
        Some(std::sync::Arc::new(table))
    }
}

impl<O: DecisionLabel> DecisionHead for CountTableHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::CountTable
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::CountTable, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        let Some(table) = &self.table else {
            return Decision::abstain(provenance, cost);
        };
        if !self.spec.admits(input) || self.labels.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let tok = tokenizer();
        let tokens = tok.encode(&input.world.text);
        let lo = tokens
            .len()
            .saturating_sub(self.order.saturating_sub(1).max(1));
        let context = &tokens[lo..];
        let mut scored: Vec<(O, f32)> = Vec::with_capacity(self.labels.len());
        let mut observed = false;
        for label in &self.labels {
            let p = table.probability(context, &label.as_key());
            if p > 0.0 {
                observed = true;
            }
            scored.push((label.clone(), p.ln().max(-30.0)));
        }
        if !observed {
            // Nothing in this context was ever counted: the table says nothing
            // about this world, which is an abstention and not a uniform guess.
            return Decision::abstain(provenance, cost);
        }
        let dist = to_distribution(scored, self.temperature);
        let top = dist[0].clone();
        Decision::answered(
            top.value,
            dist,
            top.probability,
            provenance.because_of(format!(
                "backoff table, {} token(s) of context, {} training example(s)",
                context.len(),
                self.fitted_from
            )),
            cost,
        )
    }

    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        let started = std::time::Instant::now();
        let usable = examples.iter().filter(|e| e.usable()).count();
        if usable < 4 {
            return FitReport::refused("fewer than 4 usable examples; a table would be a lookup");
        }
        let cfg = TableConfig {
            table_id: format!("system-one-{}", self.spec.decision_type),
            kind: TableKind::Lexical,
            max_order: self.order as u8,
            min_count: 1,
            max_vocabulary: 20_000,
            smoothing_addk: 0.5,
            backoff_alpha: 0.4,
            license: "CC0-1.0".into(),
        };
        let Some(table) = CountTableHead::build(examples, cfg, self.order) else {
            return FitReport::refused("no usable examples after weighting");
        };
        let mut labels: Vec<O> = Vec::new();
        for e in examples.iter().filter(|e| e.usable()) {
            if !labels.iter().any(|l| l.as_key() == e.output.as_key()) {
                labels.push(e.output.clone());
            }
        }
        labels.sort_by_key(|l| l.as_key());
        self.table = Some(table);
        self.labels = labels;
        self.fitted_from = usable;
        FitReport::fitted(
            usable,
            started.elapsed().as_micros() as u64,
            "backed-off token→label table via crate::ngram_table",
        )
    }
}

// ── Lexical: BM25 over labeled exemplars ────────────────────────────────

/// A BM25 head over labeled exemplars.
///
/// **Algorithm credit:** Robertson & Zaragoza, *The Probabilistic Relevance
/// Framework: BM25 and Beyond* (2009), with the standard `k1 = 1.2`, `b = 0.75`
/// parametrization. It is written here rather than pulled from a crate because it
/// is a closed-form sum over term frequencies, and a dependency for twelve lines
/// of arithmetic is the wrong trade for an engine crate.
///
/// One document per *label*: its exemplars concatenated. Per-label documents
/// rather than per-example ones keep a well-populated label from winning on
/// volume alone.
#[derive(Debug, Clone)]
pub struct LexicalHead<O> {
    id: String,
    spec: HeadSpec,
    docs: BTreeMap<String, Vec<String>>,
    labels: Vec<O>,
    idf: HashMap<String, f32>,
    avg_len: f32,
    k1: f32,
    b: f32,
    temperature: f32,
    fitted_from: usize,
}

impl<O: DecisionLabel> LexicalHead<O> {
    pub fn new(id: impl Into<String>, spec: HeadSpec) -> Self {
        Self {
            id: id.into(),
            spec,
            docs: BTreeMap::new(),
            labels: Vec::new(),
            idf: HashMap::new(),
            avg_len: 1.0,
            k1: 1.2,
            b: 0.75,
            temperature: 1.0,
            fitted_from: 0,
        }
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }

    /// Build the index from labeled examples.
    pub fn learn(id: impl Into<String>, spec: HeadSpec, examples: &[TrainingExample<O>]) -> Self {
        let tok = tokenizer();
        let mut docs: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut labels: Vec<O> = Vec::new();
        let mut n = 0usize;
        for e in examples.iter().filter(|e| e.usable()) {
            let key = e.output.as_key();
            if !labels.iter().any(|l| l.as_key() == key) {
                labels.push(e.output.clone());
            }
            docs.entry(key)
                .or_default()
                .extend(tok.encode(&e.input.world.text));
            n += 1;
        }
        labels.sort_by_key(|l| l.as_key());
        let mut head = LexicalHead::new(id, spec);
        head.avg_len = if docs.is_empty() {
            1.0
        } else {
            docs.values().map(|d| d.len()).sum::<usize>() as f32 / docs.len() as f32
        };
        head.idf = idf_over(&docs);
        head.docs = docs;
        head.labels = labels;
        head.fitted_from = n;
        head
    }

    /// BM25 score of `query` against the label document `key`, plus the terms
    /// that contributed — the terms are what makes the provenance readable.
    fn bm25(&self, query: &[String], key: &str) -> (f32, Vec<String>) {
        let Some(doc) = self.docs.get(key) else {
            return (0.0, Vec::new());
        };
        let len = doc.len().max(1) as f32;
        let mut score = 0.0f32;
        let mut terms: Vec<String> = Vec::new();
        for term in query {
            let tf = doc.iter().filter(|t| *t == term).count() as f32;
            if tf == 0.0 {
                continue;
            }
            let idf = self.idf.get(term).copied().unwrap_or(0.0);
            if idf <= 0.0 {
                continue;
            }
            let denom = tf + self.k1 * (1.0 - self.b + self.b * len / self.avg_len.max(1e-6));
            score += idf * (tf * (self.k1 + 1.0)) / denom;
            terms.push(term.clone());
        }
        terms.sort();
        terms.dedup();
        (score, terms)
    }
}

fn idf_over(docs: &BTreeMap<String, Vec<String>>) -> HashMap<String, f32> {
    let n = docs.len() as f32;
    let mut df: HashMap<String, usize> = HashMap::new();
    for doc in docs.values() {
        let mut seen: Vec<&String> = doc.iter().collect();
        seen.sort();
        seen.dedup();
        for t in seen {
            *df.entry(t.clone()).or_insert(0) += 1;
        }
    }
    df.into_iter()
        .map(|(t, d)| {
            let idf = (1.0 + (n - d as f32 + 0.5) / (d as f32 + 0.5)).ln();
            (t, idf)
        })
        .collect()
}

impl<O: DecisionLabel> DecisionHead for LexicalHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Lexical
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Lexical, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if !self.spec.admits(input) {
            return Decision::abstain(provenance, cost);
        }
        if self.docs.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let query = tokenizer().encode(&input.world.text);
        if query.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let mut scored: Vec<(O, f32)> = Vec::with_capacity(self.labels.len());
        let mut best_terms: Vec<String> = Vec::new();
        let mut any_signal = false;
        for label in &self.labels {
            let (score, terms) = self.bm25(&query, &label.as_key());
            if score > 0.0 {
                any_signal = true;
                if terms.len() > best_terms.len() {
                    best_terms = terms;
                }
            }
            scored.push((label.clone(), score));
        }
        if !any_signal {
            return Decision::abstain(provenance, cost);
        }
        let dist = to_distribution(scored, self.temperature);
        let top = dist[0].clone();
        Decision::answered(
            top.value,
            dist,
            top.probability,
            provenance.because_of(format!(
                "BM25 over {} label document(s) from {} example(s); terms [{}]",
                self.docs.len(),
                self.fitted_from,
                best_terms.join(", ")
            )),
            cost,
        )
    }

    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        let started = std::time::Instant::now();
        let usable = examples.iter().filter(|e| e.usable()).count();
        if usable < 4 {
            return FitReport::refused("fewer than 4 usable examples; BM25 would be a lookup");
        }
        let learnt = LexicalHead::learn(&self.id, self.spec.clone(), examples);
        self.docs = learnt.docs;
        self.labels = learnt.labels;
        self.idf = learnt.idf;
        self.avg_len = learnt.avg_len;
        self.fitted_from = learnt.fitted_from;
        FitReport::fitted(
            usable,
            started.elapsed().as_micros() as u64,
            "BM25 index over per-label documents",
        )
    }
}

// ── Prototype and k-NN: retrieval in embedding space ────────────────────

/// Nearest label centroid, with the same similarity the grid classifier scores
/// cells with ([`cosine_sim`]).
///
/// The rung that matters when the distinction is semantic rather than lexical:
/// two phrasings of the same case land near the same centroid. It compares
/// *means*, so it is robust to a few odd exemplars and blind to a multimodal
/// label — which is what [`KnnHead`] is for.
#[derive(Debug, Clone)]
pub struct PrototypeHead<O> {
    id: String,
    spec: HeadSpec,
    centroids: Vec<(O, Vec<f32>)>,
    /// Below this cosine the head abstains instead of choosing the least-bad
    /// label. A nearest-neighbour answer always exists; a *supported* one may not.
    min_cosine: f32,
    temperature: f32,
}

impl<O: DecisionLabel> PrototypeHead<O> {
    pub fn new(id: impl Into<String>, spec: HeadSpec) -> Self {
        Self {
            id: id.into(),
            spec,
            centroids: Vec::new(),
            min_cosine: 0.05,
            temperature: 0.1,
        }
    }

    pub fn with_min_cosine(mut self, c: f32) -> Self {
        self.min_cosine = c;
        self
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }

    pub fn learn(id: impl Into<String>, spec: HeadSpec, examples: &[TrainingExample<O>]) -> Self {
        let mut sums: BTreeMap<String, (O, Vec<f32>, usize)> = BTreeMap::new();
        for e in examples.iter().filter(|e| e.usable()) {
            let emb = &e.input.world.embedding;
            if emb.is_empty() {
                continue;
            }
            let entry = sums
                .entry(e.output.as_key())
                .or_insert_with(|| (e.output.clone(), vec![0.0; emb.len()], 0));
            if entry.1.len() != emb.len() {
                continue; // mixed embedder dimensions: skip rather than pad
            }
            for (i, v) in emb.iter().enumerate() {
                entry.1[i] += v;
            }
            entry.2 += 1;
        }
        let mut centroids: Vec<(O, Vec<f32>)> = sums
            .into_values()
            .filter(|(_, _, n)| *n > 0)
            .map(|(label, sums, n)| {
                let mut c: Vec<f32> = sums.iter().map(|v| v / n as f32).collect();
                let norm: f32 = c.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
                c.iter_mut().for_each(|x| *x /= norm);
                (label, c)
            })
            .collect();
        centroids.sort_by_key(|(l, _)| l.as_key());
        let mut head = PrototypeHead::new(id, spec);
        head.centroids = centroids;
        head
    }
}

impl<O: DecisionLabel> DecisionHead for PrototypeHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Prototype
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Prototype, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if !self.spec.admits(input) || self.centroids.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let emb = &input.world.embedding;
        if emb.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let mut scored: Vec<(O, f32)> = Vec::with_capacity(self.centroids.len());
        for (label, centroid) in &self.centroids {
            if centroid.len() != emb.len() {
                continue;
            }
            scored.push((label.clone(), cosine_sim(emb, centroid)));
        }
        if scored.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let best = scored
            .iter()
            .map(|(_, s)| *s)
            .fold(f32::NEG_INFINITY, f32::max);
        if best < self.min_cosine {
            return Decision::abstain(provenance, cost);
        }
        let dist = to_distribution(scored, self.temperature);
        let top = dist[0].clone();
        Decision::answered(
            top.value,
            dist,
            top.probability,
            provenance.because_of(format!(
                "nearest of {} label centroid(s), cosine {:.3}",
                self.centroids.len(),
                best
            )),
            cost,
        )
    }

    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        let started = std::time::Instant::now();
        let with_vectors = examples
            .iter()
            .filter(|e| e.usable() && !e.input.world.embedding.is_empty())
            .count();
        if with_vectors < 4 {
            return FitReport::refused(
                "fewer than 4 usable examples carrying an embedding; nothing to average",
            );
        }
        let learnt = PrototypeHead::learn(&self.id, self.spec.clone(), examples);
        if learnt.centroids.is_empty() {
            return FitReport::refused("no label produced a centroid");
        }
        let n = learnt.centroids.len();
        self.centroids = learnt.centroids;
        FitReport::fitted(
            n,
            started.elapsed().as_micros() as u64,
            "one normalised centroid per label",
        )
    }
}

/// Weighted k-NN over remembered exemplars.
///
/// The rung between [`PrototypeHead`] and a model: it costs a linear scan and it
/// handles a label that lives in several places at once, which a centroid
/// cannot. `k` is declared; `k = 1` is legitimate when the traces are clean and
/// abstention is cheap.
#[derive(Debug, Clone)]
pub struct KnnHead<O> {
    id: String,
    spec: HeadSpec,
    memory: Vec<(Vec<f32>, O, f32)>,
    k: usize,
    min_similarity: f32,
    temperature: f32,
}

impl<O: DecisionLabel> KnnHead<O> {
    pub fn new(id: impl Into<String>, spec: HeadSpec, k: usize) -> Self {
        Self {
            id: id.into(),
            spec,
            memory: Vec::new(),
            k: k.max(1),
            min_similarity: 0.0,
            temperature: 0.05,
        }
    }

    pub fn with_min_similarity(mut self, s: f32) -> Self {
        self.min_similarity = s;
        self
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }

    pub fn memory_len(&self) -> usize {
        self.memory.len()
    }

    fn remember(&mut self, e: &TrainingExample<O>) {
        if !e.usable() || e.input.world.embedding.is_empty() {
            return;
        }
        self.memory
            .push((e.input.world.embedding.clone(), e.output.clone(), e.weight));
    }
}

impl<O: DecisionLabel> DecisionHead for KnnHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Knn
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Knn, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if !self.spec.admits(input) || self.memory.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let emb = &input.world.embedding;
        if emb.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let mut neighbours: Vec<(f32, &O, f32)> = self
            .memory
            .iter()
            .filter(|(v, _, _)| v.len() == emb.len())
            .map(|(v, label, w)| (cosine_sim(emb, v), label, *w))
            .collect();
        neighbours.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.as_key().cmp(&b.1.as_key()))
        });
        if neighbours.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let best = neighbours[0].0;
        if best < self.min_similarity {
            return Decision::abstain(provenance, cost);
        }
        neighbours.truncate(self.k);
        let mut scored: Vec<(O, f32)> = Vec::new();
        for (sim, label, weight) in neighbours {
            match scored
                .iter_mut()
                .find(|(l, _)| l.as_key() == label.as_key())
            {
                Some((_, s)) => *s += sim.max(0.0) * weight,
                None => scored.push((label.clone(), sim.max(0.0) * weight)),
            }
        }
        let dist = to_distribution(scored, self.temperature);
        let top = dist[0].clone();
        Decision::answered(
            top.value,
            dist,
            top.probability,
            provenance.because_of(format!(
                "{}-NN over {} remembered case(s), nearest cosine {:.3}",
                self.k,
                self.memory.len(),
                best
            )),
            cost,
        )
    }

    fn fit(&mut self, examples: &[TrainingExample<O>]) -> FitReport {
        let started = std::time::Instant::now();
        self.memory.clear();
        for e in examples {
            self.remember(e);
        }
        if self.memory.is_empty() {
            return FitReport::refused("no usable example carried an embedding");
        }
        // Deterministic order keeps two runs of the same fit byte-comparable in
        // any report that serializes the head's state.
        self.memory.sort_by(|a, b| {
            a.1.as_key()
                .cmp(&b.1.as_key())
                .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });
        let n = self.memory.len();
        FitReport::fitted(
            n,
            started.elapsed().as_micros() as u64,
            "exemplar memory, one vector per case",
        )
    }
}

// ─ Model-scored: an SLM or an n-gram decoder as a head ─────────────────

/// A head that asks a [`crate::model_provider::ModelProvider`] to score each
/// candidate label.
///
/// This is *LLM-as-classifier by log-probability*, expressed against the crate's
/// own interchangeable-model trait, so the same head runs over
/// [`crate::model_provider::NgramDecoderModel`] offline (deterministic, no
/// weights, no key) and over a real small model when one is installed. Nothing
/// here knows which provider it has: [`ModelHead::provider_id`] reports it for
/// provenance, and `HeadKind::Ngram` is the honest rung name either way.
pub struct ModelHead<O> {
    id: String,
    spec: HeadSpec,
    provider: Box<dyn ModelProvider>,
    labels: Vec<O>,
    temperature: f32,
}

impl<O: DecisionLabel> ModelHead<O> {
    pub fn new(
        id: impl Into<String>,
        spec: HeadSpec,
        provider: Box<dyn ModelProvider>,
        labels: Vec<O>,
    ) -> Self {
        Self {
            id: id.into(),
            spec,
            provider,
            labels,
            temperature: 1.0,
        }
    }

    pub fn provider_id(&self) -> &str {
        &self.provider.metadata().id
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }

    /// The string a provider scores. One function, so every rung that scores a
    /// candidate uses the same prompt shape — a head that improvised its own
    /// would make two models incomparable for no reason.
    fn probe(text: &str, label: &str) -> String {
        format!("{text} => {label}")
    }
}

impl<O: DecisionLabel> DecisionHead for ModelHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Ngram
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn profile(&self) -> crate::system_one::CostProfile {
        // The provider reports its own memory estimate; latency stays nominal,
        // because a head cannot time a provider it did not build.
        let mut p = HeadKind::Ngram.nominal_profile();
        p.ram_bytes = self.provider.memory_requirements_mb() * 1024 * 1024;
        p
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Ngram, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if !self.spec.admits(input) || self.labels.is_empty() {
            return Decision::abstain(provenance, cost);
        }
        let mut scored: Vec<(O, f32)> = Vec::with_capacity(self.labels.len());
        for label in &self.labels {
            match self
                .provider
                .score(&ModelHead::<O>::probe(&input.world.text, &label.as_key()))
            {
                Ok(s) if s.is_finite() => scored.push((label.clone(), s)),
                // A provider that cannot score says so; the head does not turn
                // that into a zero and let another label win by default.
                _ => return Decision::abstain(provenance, cost),
            }
        }
        let dist = to_distribution(scored, self.temperature);
        let top = dist[0].clone();
        Decision::answered(
            top.value,
            dist,
            top.probability,
            provenance.because_of(format!(
                "scored by model '{}' ({})",
                self.provider.metadata().id,
                self.provider.metadata().architecture
            )),
            cost,
        )
    }
}

// ─ Oracle-backed: the strong rung, and its honest stand-in ──────────────

/// Wraps a [`DecisionOracle`] as a head, checking the schema binding first.
///
/// The check is the point: a System-One oracle has a *static* schema, so a spec
/// whose label space the binding cannot express is refused when the head is
/// constructed rather than when it is first forced to answer.
pub struct OracleHead<O> {
    id: String,
    spec: HeadSpec,
    oracle: Box<dyn DecisionOracle<O>>,
}

impl<O: DecisionLabel> OracleHead<O> {
    pub fn new(
        id: impl Into<String>,
        spec: HeadSpec,
        oracle: Box<dyn DecisionOracle<O>>,
    ) -> Result<Self, SchemaError> {
        oracle.schema().validate(&spec)?;
        Ok(Self {
            id: id.into(),
            spec,
            oracle,
        })
    }

    pub fn schema(&self) -> OracleSchema {
        self.oracle.schema()
    }

    pub fn oracle_name(&self) -> &str {
        self.oracle.name()
    }
}

impl<O: DecisionLabel> DecisionHead for OracleHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        self.oracle.kind()
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn profile(&self) -> crate::system_one::CostProfile {
        self.oracle.profile()
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let mut d = self.oracle.decide(input, &self.spec);
        // The answer must be inside the declared space. For an enum label the
        // type guarantees it; for `String` it does not, and this is where the
        // open space gets closed.
        if let Some(v) = d.value.as_ref() {
            if !self.spec.admits_label(&v.as_key()) {
                let provenance = Provenance::head(&self.id, self.kind(), &self.spec.label_space);
                return Decision::abstain(
                    provenance.because_of(format!(
                        "oracle answered '{}', outside the declared label space",
                        v.as_key()
                    )),
                    d.cost,
                );
            }
        }
        d.provenance.head_id = format!("{}::{}", self.id, d.provenance.head_id);
        d.provenance.contract = self.spec.label_space.clone();
        d
    }
}

/// A **stand-in** oracle for tests and experiments: a declared rule plus noise.
///
/// This is not a model and must never be reported as one. It exists because the
/// loop in [`crate::system_one::jit`] has to be exercised end to end without a
/// key, a network, or a claim about intelligence — and because a benchmark of the
/// *ladder* needs an arm that is strong, expensive and exactly reproducible.
/// [`DecisionOracle::name`] returns `"stub"` so nothing can confuse the two.
///
/// `noise` is the probability that an answer is replaced by a uniformly drawn
/// wrong label. The schema's `calibrated` flag is `false`: an injected error
/// model is a declared property of this stand-in, not evidence that a model is
/// calibrated.
pub struct StubOracle<O> {
    name: String,
    kind: HeadKind,
    judge: Box<dyn Fn(&DecisionInput) -> O + Send + Sync>,
    labels: Vec<O>,
    noise: f32,
    schema: OracleSchema,
    rng: std::sync::Mutex<rand::rngs::StdRng>,
}

impl<O: DecisionLabel> StubOracle<O> {
    /// Build a stub around a deterministic judge. `seed` fixes the noise.
    pub fn new(
        name: impl Into<String>,
        kind: HeadKind,
        labels: Vec<O>,
        judge: Box<dyn Fn(&DecisionInput) -> O + Send + Sync>,
        noise: f32,
        seed: u64,
    ) -> Self {
        use rand::SeedableRng;
        let mut schema = if kind == HeadKind::Jev {
            OracleSchema::system_one(name.into(), labels.len())
        } else {
            OracleSchema::chat(name.into(), labels.len())
        };
        schema.calibrated = false;
        let name = match kind {
            HeadKind::Jev => "stub-system-one",
            _ => "stub-chat",
        };
        Self {
            name: name.to_string(),
            kind,
            judge,
            labels,
            noise: noise.clamp(0.0, 1.0),
            schema,
            rng: std::sync::Mutex::new(rand::rngs::StdRng::seed_from_u64(seed)),
        }
    }

    /// The noise actually in force — printed by any report that uses the stub.
    pub fn noise(&self) -> f32 {
        self.noise
    }

    /// The judge's answer, before noise: the ground truth this stub encodes.
    pub fn exact(&self, input: &DecisionInput) -> O {
        (self.judge)(input)
    }
}

impl<O: DecisionLabel> DecisionOracle<O> for StubOracle<O> {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> HeadKind {
        self.kind
    }

    fn schema(&self) -> OracleSchema {
        self.schema.clone()
    }

    fn decide(&self, input: &DecisionInput, spec: &HeadSpec) -> Decision<O> {
        use rand::Rng;
        let provenance = Provenance::head(&self.name, self.kind, &spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        let exact = (self.judge)(input);
        let mut chosen = exact.clone();
        let mut wrong = false;
        if self.noise > 0.0 && self.labels.len() > 1 {
            let mut rng = self.rng.lock().expect("stub rng");
            if rng.gen::<f32>() < self.noise {
                let alternatives: Vec<&O> = self
                    .labels
                    .iter()
                    .filter(|l| l.as_key() != exact.as_key())
                    .collect();
                let pick = rng.gen_range(0..alternatives.len());
                chosen = alternatives[pick].clone();
                wrong = true;
            }
        }
        let confidence = 1.0 - self.noise;
        let dist: Vec<Mass<O>> = if wrong {
            vec![
                Mass::new(chosen.clone(), confidence),
                Mass::new(exact, 1.0 - confidence),
            ]
        } else {
            vec![Mass::new(chosen.clone(), confidence)]
        };
        Decision::answered(
            chosen,
            dist,
            confidence,
            provenance.because_of(if self.noise > 0.0 {
                "stub oracle (stand-in, not a model) with declared noise".to_string()
            } else {
                "stub oracle (stand-in, not a model), noise 0".to_string()
            }),
            cost,
        )
    }
}

// ── Human: queued, answered, learned from ───────────────────────────────

/// A question waiting for a person.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingAsk {
    pub id: String,
    pub world_id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
    pub head_id: String,
    /// Why this case was escalated: the cheapest rung's own words.
    pub reason: String,
}

/// The human rung: abstains while it queues, and answers from what people said.
///
/// Two jobs, kept apart on purpose:
///
/// 1. **Queue** — every abstention is recorded as a [`PendingAsk`], so the
///    expensive resource is spent only where the cheap rungs had nothing to say.
///    That is uncertainty sampling with the uncertainty made explicit rather than
///    inferred from a margin.
/// 2. **Learn** — labelled answers are distilled into a [`CountTableHead`] over
///    the same spec, so the second time this case appears it costs nothing. The
///    head reports `HeadKind::Human` even once its answers come from the
///    distilled table, because *the label* came from a person and a report must
///    be able to tell that apart from a model's opinion.
pub struct HumanQueueHead<O> {
    id: String,
    spec: HeadSpec,
    inner: Option<CountTableHead<O>>,
    pending: Vec<(PendingAsk, DecisionInput)>,
    answers: Vec<TrainingExample<O>>,
    min_labels: usize,
}

impl<O: DecisionLabel> HumanQueueHead<O> {
    /// `min_labels` is how many human answers must accumulate before the
    /// distilled table is allowed to answer at all.
    pub fn new(id: impl Into<String>, spec: HeadSpec, min_labels: usize) -> Self {
        Self {
            id: id.into(),
            spec,
            inner: None,
            pending: Vec::new(),
            answers: Vec::new(),
            min_labels: min_labels.max(1),
        }
    }

    pub fn pending(&self) -> Vec<&PendingAsk> {
        self.pending.iter().map(|(a, _)| a).collect()
    }

    pub fn answered_count(&self) -> usize {
        self.answers.len()
    }

    /// Record a person's answer to a queued ask. Returns `true` if the ask was
    /// outstanding; an answer for an unknown ask is refused rather than silently
    /// attached to the wrong case.
    pub fn answer(&mut self, ask_id: &str, label: O) -> bool {
        let Some(pos) = self.pending.iter().position(|(a, _)| a.id == ask_id) else {
            return false;
        };
        let (ask, input) = self.pending.remove(pos);
        let mut example = TrainingExample::new(input, label);
        example.trace_id = Some(ask.id);
        self.answers.push(example);
        true
    }

    /// Queue a case for a person, returning the ask id. Separate from `decide`
    /// because `decide` may not mutate: mutation belongs to the loop, not to the
    /// question.
    pub fn enqueue(&mut self, input: &DecisionInput, reason: impl Into<String>) -> String {
        let id = format!(
            "{}-ask-{}",
            self.id,
            self.pending.len() + self.answers.len()
        );
        self.pending.push((
            PendingAsk {
                id: id.clone(),
                world_id: input.world.id.clone(),
                text: input.world.text.clone(),
                cell: input.cell(),
                head_id: self.id.clone(),
                reason: reason.into(),
            },
            input.clone(),
        ));
        id
    }

    /// Distil the human answers into a table. Called by the JIT loop once enough
    /// answers exist; it returns what happened, so a report can say "still
    /// waiting" instead of implying an answer arrived.
    pub fn distil(&mut self) -> FitReport {
        if self.answers.len() < self.min_labels {
            return FitReport::refused(format!(
                "{} human answer(s); {} required before a table may answer",
                self.answers.len(),
                self.min_labels
            ));
        }
        let mut head = CountTableHead::new(format!("{}-distilled", self.id), self.spec.clone(), 3);
        let report = head.fit(&self.answers);
        if report.is_deployable() {
            self.inner = Some(head);
        }
        report
    }
}

impl<O: DecisionLabel> DecisionHead for HumanQueueHead<O> {
    type Output = O;

    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        HeadKind::Human
    }

    fn spec(&self) -> &HeadSpec {
        &self.spec
    }

    fn profile(&self) -> crate::system_one::CostProfile {
        // The distilled table's cost is local; a queued ask costs a person's
        // time. The profile reports the human rung, which is what the router's
        // cost model is protecting.
        let mut p = HeadKind::Human.nominal_profile();
        if self.inner.is_some() {
            p.micros = 50;
        }
        p
    }

    fn decide(&self, input: &DecisionInput) -> Decision<O> {
        let provenance = Provenance::head(&self.id, HeadKind::Human, &self.spec.label_space);
        let cost = Cost::from_profile(&self.profile());
        if let Some(inner) = &self.inner {
            let mut d = DecisionHead::decide(inner, input);
            let derivation = d.provenance.derivation.clone().unwrap_or_default();
            d.provenance = provenance.because_of(format!(
                "distilled from {} answered ask(s); {derivation}",
                self.answers.len()
            ));
            d.cost = cost;
            return d;
        }
        Decision::abstain(provenance.because_of("awaiting a person"), cost)
    }
}

// ── Remote: the `http` leg of the crate's own oracle module ─────────────

/// A [`DecisionOracle`] over an OpenAI-compatible endpoint, reusing
/// [`crate::oracle`].
///
/// Only compiled with the `http` feature. It is deliberately a *chat* binding
/// ([`OracleSchema::chat`]: one decision per round trip, `calibrated: false`)
/// unless the endpoint is a real System-One service, in which case the caller
/// passes [`HeadKind::Jev`] and the schema says so. Reporting a chat model as
/// parallel-sampling and calibrated is the exact mistake this type refuses to
/// make on the deployment's behalf.
#[cfg(feature = "http")]
pub struct HttpOracle<O> {
    id: String,
    kind: HeadKind,
    schema: OracleSchema,
    config: crate::oracle::OracleConfig,
    key: String,
    max_tokens: u32,
    _labels: std::marker::PhantomData<O>,
}

#[cfg(feature = "http")]
impl<O: DecisionLabel> HttpOracle<O> {
    /// `None` when no endpoint/key is configured — the caller then has no remote
    /// rung and the ladder ends where it ends. Nothing is faked.
    pub fn from_env(kind: HeadKind, cardinality: usize) -> Option<Self> {
        let (config, key) = crate::oracle::from_env()?;
        let schema = if kind == HeadKind::Jev {
            OracleSchema::system_one("http-system-one", cardinality)
        } else {
            OracleSchema::chat(config.model.clone(), cardinality)
        };
        Some(Self {
            id: format!("http-{}", kind.as_str()),
            kind,
            schema,
            config,
            key,
            max_tokens: 32,
            _labels: std::marker::PhantomData,
        })
    }

    /// The prompt contract: the declared labels, verbatim, and an instruction to
    /// answer with exactly one of them. Parsing is strict — a reply outside the
    /// space is an abstention, not a best-effort match.
    fn prompt(spec: &HeadSpec, text: &str) -> String {
        format!(
            "Choose exactly one label for the state below.\nLabels: {}\nAnswer with the label alone.\n\nState:\n{}",
            spec.declared_labels.join(", "),
            text
        )
    }
}

#[cfg(feature = "http")]
impl<O: DecisionLabel> DecisionOracle<O> for HttpOracle<O> {
    fn name(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> HeadKind {
        self.kind
    }

    fn schema(&self) -> OracleSchema {
        self.schema.clone()
    }

    fn decide(&self, input: &DecisionInput, spec: &HeadSpec) -> Decision<O> {
        let provenance = Provenance::head(&self.id, self.kind, &spec.label_space)
            .because_of(format!("{} via {}", self.config.model, self.config.url));
        let mut cost = Cost::from_profile(&self.profile());
        let prompt = HttpOracle::<O>::prompt(spec, &input.world.text);
        match crate::oracle::complete(
            &self.config,
            &self.key,
            "You are a classifier.",
            &prompt,
            self.max_tokens,
        ) {
            Ok((reply, latency_ms)) => {
                cost.micros = (latency_ms * 1000.0) as u64;
                let answer = reply.trim().trim_matches('"').to_ascii_lowercase();
                match O::from_key(&answer) {
                    Some(v) if spec.admits_label(&v.as_key()) => {
                        let dist = vec![Mass::new(v.clone(), 1.0)];
                        Decision::answered(v, dist, 1.0, provenance, cost)
                    }
                    _ => Decision::abstain(
                        provenance.because_of(format!(
                            "reply '{answer}' is not one of the declared labels"
                        )),
                        cost,
                    ),
                }
            }
            Err(e) => {
                Decision::abstain(provenance.because_of(format!("endpoint error: {e}")), cost)
            }
        }
    }
}
