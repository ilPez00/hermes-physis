//! The semantic JIT: turn a repeatedly-expensive judgement into a validated
//! cheap head, or decline to.
//!
//! ```text
//! novel task → expensive rung → traces + outcomes
//!            → recurring distinction?           (detect_recurrence)
//!            → which distinctions matter, where? (information gain, Δ priority)
//!            → cheapest rung that can express it (synthesize)
//!            → fit, then calibrate on a held-out slice (ConformalGate)
//!            → promote only if it does not materially degrade (bootstrap CI)
//!            → monitor; demote on drift; fall back when uncertain
//! ```
//!
//! Every stage is a pure function over data the stage before produced, so each
//! can be tested and reported on its own, and the whole loop runs with the strong
//! rung stubbed out ([`crate::system_one::heads::StubOracle`]).
//!
//! ## The four decisions this module makes, and the control against each
//!
//! | decision | control |
//! |---|---|
//! | is this distinction **recurring**? | minimum support, and a base-rate comparison |
//! | is this split **informative**? | information gain in bits against the label entropy of the region |
//! | is the head **safe**? | accepted accuracy on a *hold-out* slice, and a paired bootstrap against the strong rung |
//! | is it still safe **now**? | a disagreement window with a Wilson upper bound against a tolerance |
//!
//! ## Reuse, not rebuild
//!
//! - Traces are [`crate::observe::Observation`]s. That log is already
//!   append-only, already sequence-numbered, already tail-readable, and
//!   [`crate::observe::read_tail`] already documents why reading the whole log per
//!   tick is how an observer gets noticed and killed. Nothing here writes a second
//!   event store.
//! - Invalidation runs the crate's own propagation:
//!   [`crate::delta_engine::evaluate_mutation`] over a shadow frame, plus
//!   [`crate::delta_engine::EvaluationContext::depends_on_walk`] — the *declared*
//!   dependency closure, which is the choice that module already made for
//!   revision selection.
//! - Standing is [`crate::quality::QualityTracker`], reached through the router.
//!
//! ## What this module refuses to do
//!
//! It will not promote a head on training-set accuracy, will not calibrate on the
//! data it trained on, and will not deploy a head whose held-out accuracy drop has
//! a confidence interval reaching past the configured tolerance. Those three are
//! the same failure — deciding something looks right when it has not been made to
//! prove it — and each is a distinct code path here.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::delta_engine::{
    evaluate_mutation, EvaluationContext, OntologyDeltaReport, OntologyMutation,
};
use crate::models::{cosine_sim, CoherenceNode, CoherenceRating};
use crate::relation::TypedEdge;
use crate::system_one::calibration::{
    bootstrap_ci, paired_differences, CalibrationReport, ConfidenceOutcome, ConformalGate,
};
use crate::system_one::heads::{
    CountTableHead, Expr, KnnHead, LexicalHead, PatternHead, PrototypeHead, RuleHead,
};
use crate::system_one::router::{HeadRouter, RoutedDecision, RoutingPolicy};
use crate::system_one::{
    cell_key, DecisionInput, DecisionLabel, DecisionOracle, FeatureValue, HeadEvidence, HeadFor,
    HeadKind, HeadSpec, OntologySlice, TrainingExample, Transparency, WorldSlice,
};
use crate::tokenizer::Tokenizer;

/// One decision the system made, and how it turned out.
///
/// Serialized into an [`crate::observe::Observation`] whose `source` is
/// `system_one`, `subject` the decision type and `duration_ms` the cost — so a
/// decision trace is queryable with the crate's existing observation tools rather
/// than through a bespoke reader.
///
/// `O` needs no `Default`: the deserialize bound is written explicitly, because a
/// label space is a closed enum or a scalar and none of them invents a value for
/// a missing field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "O: serde::Deserialize<'de>"))]
pub struct DecisionTrace<O> {
    pub id: String,
    pub decision_type: String,
    pub at: DateTime<Utc>,
    /// `DOMAIN\u{0}MODE` when the world was placed in a cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
    pub world_id: String,
    pub text: String,
    #[serde(default)]
    pub state: BTreeMap<String, FeatureValue>,
    #[serde(default)]
    pub embedding: Vec<f32>,
    /// The evidence the request carried.
    ///
    /// Kept because an audit has to ask the strong rung *the same question the
    /// router asked*. A landmark case from the first version of this code: a
    /// trace rebuilt without its evidence was put to the oracle, which — missing
    /// the authority the request had carried — answered from the state alone and
    /// agreed with the very head it was supposed to be checking. The audit said
    /// `keep` all the way through a shift that had flipped the meaning of a
    /// field. Missing evidence is not a small omission; it changes the question.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<HeadEvidence>,
    pub head_id: String,
    pub kind: HeadKind,
    /// What was answered; `None` = abstained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<O>,
    pub confidence: f32,
    pub cost_micros: u64,
    /// The validated outcome, when one arrived. This is what the JIT learns from,
    /// and it is deliberately *not* `value`: a head's own answer is not evidence
    /// about the world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<O>,
    /// `Success` = label validated, `Failure` = refuted, `Inert` = unjudged.
    pub verdict: CoherenceRating,
}

/// `DOMAIN\u{0}MODE` back to a `(domain, mode)` pair, when it has that shape.
pub fn split_cell_key(key: &str) -> Option<(String, String)> {
    let (d, m) = key.split_once('\u{0}')?;
    if d.is_empty() || m.is_empty() {
        return None;
    }
    Some((d.to_string(), m.to_string()))
}

impl<O: DecisionLabel> DecisionTrace<O> {
    pub fn from_routed(
        input: &DecisionInput,
        routed: &RoutedDecision<O>,
        decision_type: &str,
    ) -> Self {
        let answered_by = routed
            .answered_by()
            .map(|s| s.to_string())
            .unwrap_or_else(|| routed.decision.provenance.head_id.clone());
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            decision_type: decision_type.to_string(),
            at: Utc::now(),
            cell: input.cell(),
            world_id: input.world.id.clone(),
            text: input.world.text.clone(),
            state: input.world.state.clone(),
            embedding: input.world.embedding.clone(),
            evidence: input.evidence.clone(),
            head_id: answered_by,
            kind: routed.decision.provenance.kind,
            value: routed.decision.value.clone(),
            confidence: routed.decision.confidence,
            cost_micros: routed.total_cost.micros,
            label: None,
            verdict: CoherenceRating::Inert,
        }
    }

    /// The request this trace was made against, with its label attached as
    /// evidence — the form a head is fit on.
    pub fn as_example(&self) -> Option<TrainingExample<O>> {
        let label = self.label.clone()?;
        let mut input = self.as_input()?;
        input.evidence.push(HeadEvidence {
            id: format!("{}::label", self.id),
            kind: crate::system_one::EvidenceKind::Label,
            claim: format!("validated in trace {}", self.id),
            confidence: 1.0,
            label: Some(label.as_key()),
            features: self.state.clone(),
            source: self.head_id.clone(),
            embedding: None,
        });
        Some(TrainingExample::from_verdict(input, label, self.verdict))
    }

    /// Project into the crate's observation log.
    pub fn to_observation(&self) -> crate::observe::Observation {
        crate::observe::Observation::new("system_one", self.decision_type.clone())
            .with_body(serde_json::to_string(self).unwrap_or_default())
            .with_duration(self.cost_micros / 1000)
            .by(self.head_id.clone())
    }

    /// Recover a trace from a body written by [`DecisionTrace::to_observation`].
    pub fn from_observation_body(body: &str) -> Option<Self> {
        serde_json::from_str(body).ok()
    }

    /// Rebuild the request this trace was made against — **including the evidence
    /// it carried**, because an audit that drops it asks a different question.
    pub fn as_input(&self) -> Option<DecisionInput> {
        let mut input = DecisionInput::new(
            WorldSlice {
                id: self.world_id.clone(),
                text: self.text.clone(),
                state: self.state.clone(),
                embedding: self.embedding.clone(),
                cell: self.cell.as_deref().and_then(split_cell_key),
                at: Some(self.at),
            },
            OntologySlice::empty(),
        );
        input.evidence = self.evidence.clone();
        Some(input)
    }
}

/// The trace log: in memory for reasoning, on disk through the crate's
/// observation log.
///
/// `max` bounds the in-memory window, for the reason
/// [`crate::observe::read_tail`] gives: a loop that re-reads everything it ever
/// saw does not survive a month of traffic. The durable record is the JSONL; the
/// window is what the JIT reasons over.
#[derive(Debug, Clone)]
pub struct TraceStore<O> {
    traces: Vec<DecisionTrace<O>>,
    max: usize,
}

impl<O: DecisionLabel> Default for TraceStore<O> {
    fn default() -> Self {
        Self::new(20_000)
    }
}

impl<O: DecisionLabel> TraceStore<O> {
    pub fn new(max: usize) -> Self {
        Self {
            traces: Vec::new(),
            max: max.max(1),
        }
    }

    pub fn record(&mut self, trace: DecisionTrace<O>) {
        self.traces.push(trace);
        if self.traces.len() > self.max {
            let drop = self.traces.len() - self.max;
            self.traces.drain(0..drop);
        }
    }

    pub fn len(&self) -> usize {
        self.traces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    pub fn all(&self) -> &[DecisionTrace<O>] {
        &self.traces
    }

    /// Attach an outcome to a trace. Returns whether the trace was found: an
    /// outcome for an unknown trace is refused rather than attached to whatever
    /// is nearby.
    pub fn judge(&mut self, trace_id: &str, label: O, verdict: CoherenceRating) -> bool {
        match self.traces.iter_mut().find(|t| t.id == trace_id) {
            Some(t) => {
                t.label = Some(label);
                t.verdict = verdict;
                true
            }
            None => false,
        }
    }

    /// Labeled examples for a decision type, oldest first — chronological order
    /// is what makes the train/hold-out split honest.
    pub fn examples(&self, decision_type: &str) -> Vec<TrainingExample<O>> {
        self.traces
            .iter()
            .filter(|t| t.decision_type == decision_type)
            .filter(|t| t.verdict != CoherenceRating::Failure)
            .filter_map(|t| t.as_example())
            .collect()
    }

    /// Labeled examples for one region: the slice a head scoped to a cell is fit
    /// on.
    pub fn examples_in(&self, decision_type: &str, cell: Option<&str>) -> Vec<TrainingExample<O>> {
        self.traces
            .iter()
            .filter(|t| t.decision_type == decision_type && t.cell.as_deref() == cell)
            .filter(|t| t.verdict != CoherenceRating::Failure)
            .filter_map(|t| t.as_example())
            .collect()
    }

    /// Traces with a label, for a decision type.
    pub fn labeled(&self, decision_type: &str) -> Vec<&DecisionTrace<O>> {
        self.traces
            .iter()
            .filter(|t| t.decision_type == decision_type && t.label.is_some())
            .collect()
    }

    /// Append traces to the observation log, assigning sequence numbers.
    pub fn flush_to_observations(
        &self,
        path: &std::path::Path,
        since: usize,
    ) -> anyhow::Result<(u64, u64)> {
        let mut obs: Vec<crate::observe::Observation> = self
            .traces
            .iter()
            .skip(since)
            .map(|t| t.to_observation())
            .collect();
        crate::observe::append(path, &mut obs)
    }

    /// Read traces back out of an observation log.
    pub fn from_observations(
        observations: &[crate::observe::Observation],
    ) -> Vec<DecisionTrace<O>> {
        observations
            .iter()
            .filter(|o| o.source == "system_one")
            .filter_map(|o| DecisionTrace::from_observation_body(&o.body))
            .collect()
    }
}

// ── Recurrence: is this distinction coming back, and what splits it? ────

/// One feature value's record inside a recurrence: which label it carried, how
/// often, and how much that beat the region's base rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitValue<O> {
    pub value: String,
    pub dominant: O,
    pub count: usize,
    pub share: f32,
    pub lift: f32,
}

/// The informative split a recurrence offered, if any.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split<O> {
    /// A state key, or `"text"` for a token split.
    pub key: String,
    /// Whether the split is over a typed feature or over a token's presence.
    pub over_text: bool,
    /// Information gain in bits: `H(label) − H(label | split)`.
    pub gain_bits: f64,
    /// Values seen, strongest first.
    pub values: Vec<SplitValue<O>>,
}

impl<O: Clone> Split<O> {
    /// The predicate a rule head would carry for this split's dominant value.
    pub fn rule_expr(&self) -> Expr {
        let top = &self.values[0];
        if self.over_text {
            Expr::TextContains {
                needle: top.value.clone(),
            }
        } else {
            Expr::Eq {
                key: self.key.clone(),
                value: FeatureValue::Text(top.value.clone()),
            }
        }
    }

    /// Precision of the dominant value: the share of its examples that carry
    /// `dominant`.
    pub fn precision(&self) -> f32 {
        self.values[0].share
    }

    /// Coverage: the dominant value's share of the region.
    pub fn coverage(&self) -> f32 {
        let total: usize = self.values.iter().map(|v| v.count).sum();
        if total == 0 {
            0.0
        } else {
            self.values[0].count as f32 / total as f32
        }
    }
}

/// A distinction the system keeps making in one region, and what would compile it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recurrence<O> {
    pub decision_type: String,
    /// `DOMAIN\u{0}MODE` the recurrence lives in, when the worlds were placed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
    /// How many labeled traces back this.
    pub support: usize,
    /// Label histogram, descending.
    pub label_distribution: Vec<(O, usize)>,
    pub dominant: O,
    pub dominant_share: f32,
    /// Entropy of the label histogram in bits: how much a split could remove at
    /// most, and therefore the ceiling on `gain_bits`.
    pub label_entropy_bits: f64,
    /// Best typed-feature split, if any cleared the gain floor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_split: Option<Split<O>>,
    /// Best token split, if any cleared the gain floor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_split: Option<Split<O>>,
    /// Mean pairwise cosine between label centroids, in `[0, 1]`: how separable
    /// the labels are in embedding space. `0` when no trace carried a vector.
    pub separability: f32,
    /// How load-bearing this region is, from the ontology's semantic-delta map.
    /// `0` means nothing known moved; it never means "do not look".
    pub delta_priority: f32,
}

impl<O: PartialEq + Clone> Recurrence<O> {
    /// The best split available, whichever kind won.
    pub fn best_split(&self) -> Option<&Split<O>> {
        match (&self.feature_split, &self.text_split) {
            (Some(f), Some(t)) => Some(if f.gain_bits >= t.gain_bits { f } else { t }),
            (Some(f), None) => Some(f),
            (None, Some(t)) => Some(t),
            (None, None) => None,
        }
    }

    /// Ordering key for the JIT: a region that moved matters more than one that
    /// did not, and a strong split matters more than a weak one.
    pub fn priority(&self) -> f64 {
        let gain = self.best_split().map(|s| s.gain_bits).unwrap_or(0.0);
        gain * (1.0 + f64::from(self.delta_priority)) * (self.support as f64).ln().max(0.1)
    }
}

/// Knobs of [`detect_recurrence`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecurrenceConfig {
    /// Minimum labeled traces before a distinction counts as recurring.
    pub min_support: usize,
    /// Minimum information gain, in bits, for a split to be reported at all.
    pub min_gain_bits: f64,
    /// Distinct values a categorical feature may have and still be searched.
    /// Above this the feature is an identifier, not a distinction.
    pub max_distinct_values: usize,
    /// How many of the most common tokens to test as text splits.
    pub max_text_terms: usize,
}

impl Default for RecurrenceConfig {
    fn default() -> Self {
        Self {
            min_support: 20,
            min_gain_bits: 0.05,
            max_distinct_values: 24,
            max_text_terms: 200,
        }
    }
}

/// Entropy in bits of a count histogram.
fn entropy_bits(counts: &HashMap<String, usize>, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let n = total as f64;
    -counts
        .values()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = *c as f64 / n;
            p * p.log2()
        })
        .sum::<f64>()
}

/// Find the distinctions the system keeps drawing, grouped by decision type and
/// region.
///
/// Grouping by `(decision_type, cell)` rather than by decision type alone is the
/// "where" in *which distinctions matter and where*: one decision type answered
/// differently in two cells is two problems, and a head distilled from the union
/// would be worse in both.
///
/// `ontology` supplies the semantic-delta weight, so a region whose content moved
/// ranks above an equally common region whose content did not. Nothing here reads
/// an embedding model: `separability` is computed from the vectors the traces
/// already carry, which is what keeps the stage free of a second opinion about
/// the space.
pub fn detect_recurrence<O: DecisionLabel>(
    traces: &[DecisionTrace<O>],
    ontology: &OntologySlice,
    cfg: &RecurrenceConfig,
) -> Vec<Recurrence<O>> {
    let mut groups: BTreeMap<(String, Option<String>), Vec<&DecisionTrace<O>>> = BTreeMap::new();
    for t in traces.iter().filter(|t| t.label.is_some()) {
        groups
            .entry((t.decision_type.clone(), t.cell.clone()))
            .or_default()
            .push(t);
    }

    let mut out: Vec<Recurrence<O>> = Vec::new();
    for ((decision_type, cell), group) in groups {
        if group.len() < cfg.min_support {
            continue;
        }
        let total = group.len();
        let mut counts: HashMap<String, (usize, O)> = HashMap::new();
        for t in &group {
            let label = t.label.clone().expect("filtered to labeled");
            let e = counts.entry(label.as_key()).or_insert((0, label.clone()));
            e.0 += 1;
        }
        let mut label_distribution: Vec<(O, usize)> =
            counts.values().map(|(n, l)| (l.clone(), *n)).collect();
        label_distribution.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.as_key().cmp(&b.0.as_key())));
        let dominant = label_distribution[0].0.clone();
        let dominant_share = label_distribution[0].1 as f32 / total as f32;
        let label_entropy = entropy_bits(
            &counts
                .iter()
                .map(|(k, (n, _))| (k.clone(), *n))
                .collect::<HashMap<String, usize>>(),
            total,
        );

        let feature_split = best_feature_split(&group, label_entropy, cfg);
        let text_split = best_text_split(&group, label_entropy, cfg);
        let separability = label_separability(&group);

        out.push(Recurrence {
            decision_type,
            cell: cell.clone(),
            support: total,
            label_distribution,
            dominant,
            dominant_share,
            label_entropy_bits: label_entropy,
            feature_split,
            text_split,
            separability,
            delta_priority: ontology.delta_priority(cell.as_deref(), &[]),
        });
    }

    // Strongest, most load-bearing regions first.
    out.sort_by(|a, b| {
        b.priority()
            .partial_cmp(&a.priority())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.decision_type.cmp(&b.decision_type))
    });
    out
}

fn split_from_counts<O: DecisionLabel>(
    key: &str,
    over_text: bool,
    buckets: BTreeMap<String, HashMap<String, (usize, O)>>,
    label_base: &HashMap<String, f32>,
    label_entropy: f64,
) -> Option<Split<O>> {
    let total: usize = buckets
        .values()
        .map(|b| b.values().map(|(n, _)| *n).sum::<usize>())
        .sum();
    if total == 0 {
        return None;
    }
    // H(label | split) = Σ_v (n_v / n) H(label | v)
    let conditional: f64 = buckets
        .values()
        .map(|b| {
            let n_v: usize = b.values().map(|(n, _)| *n).sum();
            let h = entropy_bits(&b.iter().map(|(k, (n, _))| (k.clone(), *n)).collect(), n_v);
            (n_v as f64 / total as f64) * h
        })
        .sum();
    let gain = (label_entropy - conditional).max(0.0);

    let mut values: Vec<SplitValue<O>> = buckets
        .into_iter()
        .map(|(value, b)| {
            let n: usize = b.values().map(|(c, _)| *c).sum();
            let (dominant, hits) = b
                .values()
                .max_by_key(|(c, _)| *c)
                .map(|(c, l)| (l.clone(), *c))
                .expect("bucket is non-empty");
            let share = hits as f32 / n as f32;
            let prior = label_base.get(&dominant.as_key()).copied().unwrap_or(0.0);
            SplitValue {
                value,
                dominant,
                count: n,
                share,
                lift: share - prior,
            }
        })
        .collect();
    values.sort_by(|a, b| {
        b.share
            .partial_cmp(&a.share)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.count.cmp(&a.count))
            .then(a.value.cmp(&b.value))
    });

    Some(Split {
        key: key.to_string(),
        over_text,
        gain_bits: gain,
        values,
    })
}

fn label_base<O: DecisionLabel>(group: &[&DecisionTrace<O>]) -> HashMap<String, f32> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for t in group {
        if let Some(l) = &t.label {
            *counts.entry(l.as_key()).or_insert(0) += 1;
        }
    }
    let n = group.len() as f32;
    counts.into_iter().map(|(k, c)| (k, c as f32 / n)).collect()
}

fn best_feature_split<O: DecisionLabel>(
    group: &[&DecisionTrace<O>],
    label_entropy: f64,
    cfg: &RecurrenceConfig,
) -> Option<Split<O>> {
    let mut keys: HashSet<&str> = HashSet::new();
    for t in group {
        for k in t.state.keys() {
            keys.insert(k.as_str());
        }
    }
    let base = label_base(group);
    let mut best: Option<Split<O>> = None;
    for key in keys {
        let mut buckets: BTreeMap<String, HashMap<String, (usize, O)>> = BTreeMap::new();
        for t in group {
            let Some(v) = t.state.get(key) else { continue };
            let Some(label) = &t.label else { continue };
            let e = buckets
                .entry(v.key())
                .or_default()
                .entry(label.as_key())
                .or_insert((0, label.clone()));
            e.0 += 1;
        }
        if buckets.len() < 2 || buckets.len() > cfg.max_distinct_values {
            continue;
        }
        let Some(split) = split_from_counts(key, false, buckets, &base, label_entropy) else {
            continue;
        };
        if split.gain_bits < cfg.min_gain_bits {
            continue;
        }
        let better = match &best {
            Some(b) => split.gain_bits > b.gain_bits,
            None => true,
        };
        if better {
            best = Some(split);
        }
    }
    best
}

fn best_text_split<O: DecisionLabel>(
    group: &[&DecisionTrace<O>],
    label_entropy: f64,
    cfg: &RecurrenceConfig,
) -> Option<Split<O>> {
    let tok = crate::system_one::heads::tokenizer();
    // Document frequency over traces, so a token that appears in every trace is
    // not a split; the top `max_text_terms` by frequency are the candidates.
    let mut df: HashMap<String, usize> = HashMap::new();
    for t in group {
        let mut toks: Vec<String> = tok.encode(&t.text);
        toks.sort();
        toks.dedup();
        for w in toks {
            *df.entry(w).or_insert(0) += 1;
        }
    }
    let mut candidates: Vec<(usize, String)> = df
        .into_iter()
        .filter(|(_, n)| *n >= 2 && *n < group.len())
        .map(|(w, n)| (n, w))
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    candidates.truncate(cfg.max_text_terms);

    let base = label_base(group);
    let mut best: Option<Split<O>> = None;
    for (_, term) in candidates {
        let mut buckets: BTreeMap<String, HashMap<String, (usize, O)>> = BTreeMap::new();
        for t in group {
            let Some(label) = &t.label else { continue };
            let present = t.text.to_lowercase().contains(&term);
            let bucket = buckets
                .entry(if present {
                    term.clone()
                } else {
                    "else".to_string()
                })
                .or_default();
            let e = bucket.entry(label.as_key()).or_insert((0, label.clone()));
            e.0 += 1;
        }
        let Some(mut split) = split_from_counts("text", true, buckets, &base, label_entropy) else {
            continue;
        };
        // The "else" bucket is a complement, not a cue: keeping it as the
        // dominant value would produce a rule that fires on the absence of a
        // word, which no one wrote down as a distinction.
        split.values.retain(|v| v.value != "else");
        if split.values.is_empty() {
            continue;
        }
        if split.gain_bits < cfg.min_gain_bits {
            continue;
        }
        let better = match &best {
            Some(b) => split.gain_bits > b.gain_bits,
            None => true,
        };
        if better {
            best = Some(split);
        }
    }
    best
}

/// Mean pairwise cosine between per-label centroids, mapped to `[0, 1]`.
///
/// One number with one job: telling [`synthesize`] whether an embedding-space
/// rung is worth trying. A trace set with no vectors returns `0.0`, which means
/// "not measurable", and no prototype rung is proposed on the strength of it.
fn label_separability<O: DecisionLabel>(group: &[&DecisionTrace<O>]) -> f32 {
    let mut sums: BTreeMap<String, (Vec<f32>, usize)> = BTreeMap::new();
    for t in group {
        if t.embedding.is_empty() {
            continue;
        }
        let Some(label) = &t.label else { continue };
        let entry = sums
            .entry(label.as_key())
            .or_insert_with(|| (vec![0.0; t.embedding.len()], 0));
        if entry.0.len() != t.embedding.len() {
            continue;
        }
        for (i, v) in t.embedding.iter().enumerate() {
            entry.0[i] += v;
        }
        entry.1 += 1;
    }
    let centroids: Vec<Vec<f32>> = sums
        .into_values()
        .filter(|(_, n)| *n > 0)
        .map(|(mut sums, n)| {
            let norm: f32 = sums.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
            for x in sums.iter_mut() {
                *x = (*x / n as f32) / norm;
            }
            sums
        })
        .collect();
    if centroids.len() < 2 {
        return 0.0;
    }
    let mut total = 0.0f32;
    let mut pairs = 0usize;
    for i in 0..centroids.len() {
        for j in (i + 1)..centroids.len() {
            // Cosine in [-1, 1] mapped to [0, 1]; orthogonal label means 0.5.
            total += (cosine_sim(&centroids[i], &centroids[j]) + 1.0) / 2.0;
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        total / pairs as f32
    }
}

// ─ Synthesis: the cheapest rung that can express the distinction ───────

/// Knobs of [`synthesize`]. Defaults are the ones the crate ships with; every one
/// of them is a *control*, and lowering one is a decision a benchmark should be
/// able to see.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthesisConfig {
    /// Gain floor for preferring a typed rule over a counted rung.
    pub rule_gain_bits: f64,
    /// Precision floor for the rule's dominant value.
    pub rule_precision: f32,
    /// Coverage floor: a rule covering 1% of the region is a curiosity.
    pub rule_coverage: f32,
    /// Separability floor for proposing an embedding-space rung, in `[0, 1]`.
    pub prototype_separability: f32,
    /// Minimum examples before any head is proposed.
    pub min_examples: usize,
    /// Fraction of examples held out for calibration and promotion. The
    /// chronological tail, never a random sample — the future is what the gate
    /// has to hold on.
    pub holdout_fraction: f32,
    /// Gain floor for proposing anything at all.
    pub min_gain_bits: f64,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            rule_gain_bits: 0.3,
            rule_precision: 0.85,
            rule_coverage: 0.1,
            prototype_separability: 0.62,
            min_examples: 24,
            holdout_fraction: 0.3,
            min_gain_bits: 0.05,
        }
    }
}

/// A head the JIT proposes, before it has been fit or validated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadCandidate<O> {
    pub spec: HeadSpec,
    pub kind: HeadKind,
    pub transparency: Transparency,
    /// Traces behind the recurrence.
    pub support: usize,
    /// Projected per-call cost, from [`HeadKind::nominal_profile`].
    pub projected: crate::system_one::CostProfile,
    pub rationale: String,
    /// The distinction this compiles, in the JIT's own words — the line a person
    /// reads before trusting anything downstream of it.
    pub distinction: String,
    /// The region's majority label: what a head reverts to when nothing fires.
    pub dominant: O,
}

fn recurrence_feature_keys<O: DecisionLabel>(recurrence: &Recurrence<O>) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    if let Some(s) = &recurrence.feature_split {
        keys.push(s.key.clone());
    }
    keys
}

/// Choose the cheapest rung that can express the split.
///
/// The order is not a preference; it is the ladder. Each step down is taken only
/// when the step above cannot express what the traces show, and the *reason* is
/// carried in the candidate so a report can say "prototype because the split is
/// in the space, not in the words".
///
/// `None` means **no cheap rung can express this distinction** — the honest
/// outcome for a semantic judgement with no stable lexical or featural correlate,
/// and the case that must keep costing what it costs.
pub fn synthesize<O: DecisionLabel>(
    recurrence: &Recurrence<O>,
    cfg: &SynthesisConfig,
) -> Option<HeadCandidate<O>> {
    synthesize_avoiding(recurrence, cfg, &[])
}

/// [`synthesize`], skipping distinctions that are already compiled.
///
/// A region can host more than one distinction — a feature split *and* a lexical
/// cue, say — and a loop that re-derived the same head every tick would never
/// find the second. `taken` is the set of specs already deployed; the ladder is
/// walked past any rung that would compile one of them again.
pub fn synthesize_avoiding<O: DecisionLabel>(
    recurrence: &Recurrence<O>,
    cfg: &SynthesisConfig,
    taken: &[HeadSpec],
) -> Option<HeadCandidate<O>> {
    candidate_ladder(recurrence, cfg)
        .into_iter()
        .find(|c| !taken.iter().any(|t| t.same_distinction_as(&c.spec)))
}

/// The rungs, in ladder order, that could express this recurrence.
fn candidate_ladder<O: DecisionLabel>(
    recurrence: &Recurrence<O>,
    cfg: &SynthesisConfig,
) -> Vec<HeadCandidate<O>> {
    let mut out: Vec<HeadCandidate<O>> = Vec::new();
    if recurrence.support < cfg.min_examples {
        return out;
    }
    let labels: Vec<String> = recurrence
        .label_distribution
        .iter()
        .map(|(l, _)| l.as_key())
        .collect();
    if labels.len() < 2 {
        // One label, forever: that is not a decision, it is a constant. A rule
        // stating it would look like intelligence and behave like a `const`.
        return out;
    }
    let cells: Vec<String> = recurrence.cell.clone().into_iter().collect();
    let base_spec = |kind: Transparency| {
        HeadSpec::new(&recurrence.decision_type, labels.clone(), kind).scoped_to(cells.clone())
    };
    let feature_keys = recurrence_feature_keys(recurrence);

    if let Some(s) = recurrence.best_split() {
        if !s.over_text
            && s.gain_bits >= cfg.rule_gain_bits
            && s.precision() >= cfg.rule_precision
            && s.coverage() >= cfg.rule_coverage
        {
            out.push(HeadCandidate {
                spec: base_spec(Transparency::Exact).reading([s.key.clone()]),
                kind: HeadKind::Rule,
                transparency: Transparency::Exact,
                support: recurrence.support,
                projected: HeadKind::Rule.nominal_profile(),
                distinction: format!("{} decides {}", s.key, recurrence.decision_type),
                rationale: format!(
                    "typed feature '{}' separates {} at gain {:.3} bits, precision {:.2}, coverage {:.2}",
                    s.key,
                    s.values[0].dominant.as_key(),
                    s.gain_bits,
                    s.precision(),
                    s.coverage()
                ),
                dominant: recurrence.dominant.clone(),
            });
        }
        if s.over_text && s.gain_bits >= cfg.rule_gain_bits && s.precision() >= cfg.rule_precision {
            out.push(HeadCandidate {
                spec: base_spec(Transparency::Pattern).with_text(),
                kind: HeadKind::Pattern,
                transparency: Transparency::Pattern,
                support: recurrence.support,
                projected: HeadKind::Pattern.nominal_profile(),
                distinction: format!("a lexical cue decides {}", recurrence.decision_type),
                rationale: format!(
                    "token '{}' carries gain {:.3} bits at precision {:.2}",
                    s.values[0].value,
                    s.gain_bits,
                    s.precision()
                ),
                dominant: recurrence.dominant.clone(),
            });
        }
        if !s.over_text && s.gain_bits >= cfg.min_gain_bits {
            out.push(HeadCandidate {
                spec: base_spec(Transparency::Counted)
                    .reading(feature_keys.clone())
                    .with_text(),
                kind: HeadKind::CountTable,
                transparency: Transparency::Counted,
                support: recurrence.support,
                projected: HeadKind::CountTable.nominal_profile(),
                distinction: format!("token and feature counts decide {}", recurrence.decision_type),
                rationale: format!(
                    "feature '{}' carries gain {:.3} bits — real, but below the rule floor ({:.2} precision, {:.2} coverage), so it is counted rather than declared",
                    s.key,
                    s.gain_bits,
                    s.precision(),
                    s.coverage()
                ),
                dominant: recurrence.dominant.clone(),
            });
        }
    }

    // No typed or lexical split strong enough: is the space itself separable?
    if recurrence.separability >= cfg.prototype_separability {
        out.push(HeadCandidate {
            spec: base_spec(Transparency::Retrieved)
                .with_embedding()
                .reading(feature_keys),
            kind: HeadKind::Prototype,
            transparency: Transparency::Retrieved,
            support: recurrence.support,
            projected: HeadKind::Prototype.nominal_profile(),
            distinction: format!("the embedding neighbourhood decides {}", recurrence.decision_type),
            rationale: format!(
                "no feature or token split cleared the floors, but label centroids are separable (mean mapped pairwise cosine {:.3}); an embedding rung is the cheapest thing that can express it",
                recurrence.separability
            ),
            dominant: recurrence.dominant.clone(),
        });
    }

    if let Some(lexical) = lexical_candidate(recurrence, labels, cells) {
        out.push(lexical);
    }
    out
}

/// The lexical fallback, split out so the ladder above stays readable.
fn lexical_candidate<O: DecisionLabel>(
    recurrence: &Recurrence<O>,
    labels: Vec<String>,
    cells: Vec<String>,
) -> Option<HeadCandidate<O>> {
    recurrence.text_split.as_ref()?;
    Some(HeadCandidate {
        spec: HeadSpec::new(&recurrence.decision_type, labels, Transparency::Retrieved)
            .scoped_to(cells)
            .with_text(),
        kind: HeadKind::Lexical,
        transparency: Transparency::Retrieved,
        support: recurrence.support,
        projected: HeadKind::Lexical.nominal_profile(),
        distinction: format!("lexical overlap decides {}", recurrence.decision_type),
        rationale:
            "lexical signal exists but is weak; BM25 over the region's exemplars is still cheaper than the strong rung"
                .to_string(),
        dominant: recurrence.dominant.clone(),
    })
}

// ─ Instantiation, calibration, promotion ───────────────────────────────

/// Build and fit the concrete head a candidate names.
///
/// Split from [`synthesize`] so that "what kind of head would express this" and
/// "does the fit work" are separate answers: a synthesis that is right and a fit
/// that fails is a different report from a distinction nothing can express.
pub fn instantiate<O: DecisionLabel>(
    candidate: &HeadCandidate<O>,
    examples: &[TrainingExample<O>],
    id: impl Into<String>,
) -> (Box<dyn HeadFor<O>>, crate::system_one::FitReport) {
    let id = id.into();
    let spec = candidate.spec.clone();
    match candidate.kind {
        HeadKind::Rule => {
            let mut head = RuleHead::new(id, spec, Vec::new());
            let report = head.fit(examples);
            (Box::new(head), report)
        }
        HeadKind::Pattern => {
            let mut head = PatternHead::new(id, spec, Vec::new());
            let report = head.fit(examples);
            (Box::new(head), report)
        }
        HeadKind::CountTable => {
            let mut head = CountTableHead::new(id, spec, 3);
            let report = head.fit(examples);
            (Box::new(head), report)
        }
        HeadKind::Lexical => {
            let mut head = LexicalHead::new(id, spec);
            let report = head.fit(examples);
            (Box::new(head), report)
        }
        HeadKind::Prototype => {
            let mut head = PrototypeHead::new(id, spec);
            let report = head.fit(examples);
            (Box::new(head), report)
        }
        HeadKind::Knn => {
            let mut head = KnnHead::new(id, spec, 5);
            let report = head.fit(examples);
            (Box::new(head), report)
        }
        // Rungs that answer by calling something else are never synthesized: a
        // cheap head that delegates to the expensive rung is a way of paying the
        // expensive price with extra steps.
        other => {
            let head = KnnHead::new(id, spec, 5);
            (
                Box::new(head),
                crate::system_one::FitReport::refused(format!(
                    "{} is not a rung the JIT synthesizes",
                    other.as_str()
                )),
            )
        }
    }
}

/// What a head did on a slice, for the promotion decision.
///
/// `ladder_accuracy` is the number that matters: the head answers when its gate
/// admits, and the request escalates otherwise — which is how the head will
/// actually be deployed. Scoring a region-scoped head on the whole slice as if it
/// had to answer every item would fail every head whose region is narrower than
/// the slice, and the deployment it describes is not the one on offer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SliceScore {
    /// Answers the head was willing to give.
    pub answered: usize,
    /// Of those, how many the gate admitted.
    pub accepted: usize,
    /// Correctness of the deployed ladder: the head's answer where admitted, the
    /// strong rung's answer where not.
    pub ladder_accuracy: f32,
    /// Correctness of the head's own admitted answers.
    pub accepted_accuracy: f32,
    /// Share of the slice the head took.
    pub coverage: f32,
}

impl SliceScore {
    /// Kept for the promotion reason string, which names the ladder explicitly.
    pub fn end_to_end_accuracy(&self) -> f32 {
        self.ladder_accuracy
    }
}

/// A point on the accuracy × cost frontier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParetoPoint {
    pub label: String,
    pub accuracy: f32,
    pub cost_micros: u64,
    pub coverage: f32,
}

impl ParetoPoint {
    /// Dominates `other` when it is at least as accurate, at most as costly, and
    /// strictly better on one of the two.
    pub fn dominates(&self, other: &ParetoPoint) -> bool {
        let acc_ok = self.accuracy >= other.accuracy;
        let cost_ok = self.cost_micros <= other.cost_micros;
        acc_ok
            && cost_ok
            && (self.accuracy > other.accuracy || self.cost_micros < other.cost_micros)
    }
}

/// The non-dominated subset of `points`, ordered by cost ascending.
pub fn pareto_frontier(points: &[ParetoPoint]) -> Vec<ParetoPoint> {
    let mut out: Vec<ParetoPoint> = points
        .iter()
        .filter(|p| !points.iter().any(|q| q.dominates(p)))
        .cloned()
        .collect();
    out.sort_by_key(|p| (p.cost_micros, p.label.clone()));
    out
}

/// The bar a candidate head has to clear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PromotionPolicy {
    /// Maximum tolerated drop in end-to-end accuracy versus the strong rung, as an
    /// absolute fraction. The "materially degrading" threshold, stated once.
    pub max_accuracy_drop: f32,
    /// The paired-bootstrap lower bound must not be worse than this.
    pub max_ci_worst_case: f32,
    /// Minimum accepted accuracy the gate must demonstrate.
    pub min_accepted_accuracy: f32,
    /// Minimum examples in the hold-out slice for the comparison to mean anything.
    pub min_holdout: usize,
    /// Bootstrap resamples.
    pub bootstrap_iters: usize,
    pub seed: u64,
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self {
            max_accuracy_drop: 0.02,
            max_ci_worst_case: 0.05,
            min_accepted_accuracy: 0.9,
            min_holdout: 20,
            bootstrap_iters: 1_000,
            seed: 11,
        }
    }
}

/// The promotion decision, with the evidence that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Promotion<O> {
    pub head_id: String,
    pub kind: HeadKind,
    pub accepted: bool,
    /// Why — in the JIT's own words, never a bare boolean.
    pub reason: String,
    pub spec: HeadSpec,
    pub candidate: Option<HeadCandidate<O>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<ConformalGate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<CalibrationReport>,
    pub head_slice: SliceScore,
    pub baseline_slice: SliceScore,
    /// Paired bootstrap `(lo, hi, mean)` on `head − baseline` per item.
    pub delta_ci: (f32, f32, f32),
    pub frontier: Vec<ParetoPoint>,
}

/// Score a head over a slice, item by item, **as deployed in a ladder**.
///
/// `baseline_correct[i]` is what the strong rung would have got on item `i`, so
/// an item the head does not take is scored at the strong rung's accuracy rather
/// than as a failure. The returned per-item vector is therefore the ladder's
/// outcome, which is what the paired bootstrap must compare against the baseline.
fn score_slice<O: DecisionLabel>(
    head: &dyn HeadFor<O>,
    items: &[TrainingExample<O>],
    gate: &ConformalGate,
    baseline_correct: &[bool],
) -> (SliceScore, Vec<bool>) {
    let mut ladder_by_item: Vec<bool> = Vec::with_capacity(items.len());
    let mut answered = 0usize;
    let mut accepted = 0usize;
    let mut accepted_correct = 0usize;
    for (i, e) in items.iter().enumerate() {
        let d = head.decide(&e.input);
        let head_correct = d.value.as_ref().map(|v| v == &e.output).unwrap_or(false);
        if !d.did_abstain() {
            answered += 1;
        }
        let admitted = !d.did_abstain() && gate.accepts(d.confidence);
        if admitted {
            accepted += 1;
            if head_correct {
                accepted_correct += 1;
            }
            ladder_by_item.push(head_correct);
        } else {
            ladder_by_item.push(baseline_correct.get(i).copied().unwrap_or(false));
        }
    }
    let n = items.len().max(1);
    let score = SliceScore {
        answered,
        accepted,
        ladder_accuracy: ladder_by_item.iter().filter(|c| **c).count() as f32 / n as f32,
        accepted_accuracy: if accepted == 0 {
            0.0
        } else {
            accepted_correct as f32 / accepted as f32
        },
        coverage: accepted as f32 / n as f32,
    };
    (score, ladder_by_item)
}

/// The bar a strong-rung reference sets, over the same items.
fn baseline_score(baseline_correct: &[bool]) -> SliceScore {
    let hits = baseline_correct.iter().filter(|c| **c).count();
    let n = baseline_correct.len();
    let acc = if n == 0 { 0.0 } else { hits as f32 / n as f32 };
    SliceScore {
        answered: n,
        accepted: n,
        ladder_accuracy: acc,
        accepted_accuracy: acc,
        coverage: if n == 0 { 0.0 } else { 1.0 },
    }
}

/// Decide whether a fitted head may be deployed.
///
/// The comparison is against the *strong rung's* answers on the same hold-out
/// items, not against a training statistic: "materially degrading task
/// performance" is defined as the paired accuracy difference, and the decision is
/// made on the lower bound of its bootstrap interval rather than on its mean. A
/// head that is as good on average but occasionally catastrophic does not pass
/// the tail test and should not be deployed.
pub fn evaluate_promotion<O: DecisionLabel>(
    head: &dyn HeadFor<O>,
    head_id: &str,
    holdout: &[TrainingExample<O>],
    baseline_correct: &[bool],
    gate: &ConformalGate,
    policy: &PromotionPolicy,
    candidate: Option<HeadCandidate<O>>,
) -> Promotion<O> {
    let (head_slice, head_correct) = score_slice(head, holdout, gate, baseline_correct);
    let baseline_slice = baseline_score(baseline_correct);
    let delta_ci = if head_correct.len() == baseline_correct.len() {
        bootstrap_ci(
            &paired_differences(&head_correct, baseline_correct),
            policy.bootstrap_iters,
            policy.seed,
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    let frontier = pareto_frontier(&[
        ParetoPoint {
            label: head_id.to_string(),
            accuracy: head_slice.ladder_accuracy,
            cost_micros: head.profile().micros,
            coverage: head_slice.coverage,
        },
        ParetoPoint {
            label: "strong-rung".to_string(),
            accuracy: baseline_slice.ladder_accuracy,
            cost_micros: HeadKind::Llm.nominal_profile().micros,
            coverage: 1.0,
        },
    ]);

    let (accepted, reason) = if holdout.len() < policy.min_holdout {
        (
            false,
            format!(
                "hold-out slice has {} item(s); {} required before a promotion can mean anything",
                holdout.len(),
                policy.min_holdout
            ),
        )
    } else if head_slice.accepted == 0 {
        (
            false,
            "head answered nothing the gate would admit on the hold-out slice".to_string(),
        )
    } else if head_slice.accepted_accuracy < policy.min_accepted_accuracy {
        (
            false,
            format!(
                "accepted accuracy {:.1}% below the required {:.1}%",
                head_slice.accepted_accuracy * 100.0,
                policy.min_accepted_accuracy * 100.0
            ),
        )
    } else if head_slice.ladder_accuracy + policy.max_accuracy_drop < baseline_slice.ladder_accuracy
    {
        (
            false,
            format!(
                "end-to-end accuracy {:.3} is more than {:.3} below the strong rung's {:.3}",
                head_slice.ladder_accuracy,
                policy.max_accuracy_drop,
                baseline_slice.ladder_accuracy
            ),
        )
    } else if delta_ci.0 < -policy.max_ci_worst_case {
        (
            false,
            format!(
                "paired bootstrap lower bound {:.3} is worse than the tolerated {:.3} — the mean may be fine and the tail is not",
                delta_ci.0, -policy.max_ci_worst_case
            ),
        )
    } else {
        (
            true,
            format!(
                "end-to-end {:.3} vs {:.3} strong-rung (mean {:+.3}, 95% CI [{:+.3}, {:+.3}]), accepted accuracy {:.3} at coverage {:.3}",
                head_slice.ladder_accuracy,
                baseline_slice.ladder_accuracy,
                delta_ci.2,
                delta_ci.0,
                delta_ci.1,
                head_slice.accepted_accuracy,
                head_slice.coverage
            ),
        )
    };

    Promotion {
        head_id: head_id.to_string(),
        kind: head.kind(),
        accepted,
        reason,
        spec: head.spec().clone(),
        candidate,
        gate: Some(gate.clone()),
        calibration: None,
        head_slice,
        baseline_slice,
        delta_ci,
        frontier,
    }
}

// ── Drift: is the head still safe now? ────────────────────────────────────

/// One audited answer: did the cheap rung agree with the strong rung on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit {
    /// `true` = the head agreed with the stronger rung (or with a judge).
    pub agreed: bool,
    /// Whether the head's own confidence was below the audit trigger. Disagreement
    /// on a case the head was sure about is a different signal from disagreement
    /// on a case it hedged on.
    pub was_uncertain: bool,
}

/// The verdict on a disagreement window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriftSignal {
    pub head_id: String,
    pub window: usize,
    pub audited: usize,
    pub disagreements: usize,
    /// Observed disagreement rate.
    pub rate: f32,
    /// Upper bound of the Wilson 95% interval on the rate.
    pub wilson_hi: f32,
    /// The tolerance the window is judged against.
    pub tolerance: f32,
    /// False = keep; true = retire this head and fall back.
    pub demote: bool,
    pub reason: String,
}

impl DriftSignal {
    pub fn line(&self) -> String {
        format!(
            "{}: {}/{} disagreed (rate {:.3}, interval hi {:.3}) vs {:.3} tolerance — {}",
            self.head_id,
            self.disagreements,
            self.audited,
            self.rate,
            self.wilson_hi,
            self.tolerance,
            if self.demote { "DEMOTE" } else { "keep" }
        )
    }
}

/// Judge a window of audits against a tolerance.
///
/// The decision is on the **observed disagreement rate**, and it needs
/// `min_audits` before it counts at all. The Wilson interval is reported beside
/// it for the opposite purpose: to say how little a small window proves. Using
/// the upper bound as the trigger would be a bug rather than a caution — at
/// eleven audits with *no* disagreement the upper bound is still 0.26, so any
/// tolerance below that would demote a healthy head on no evidence, which is
/// precisely the failure this window exists to avoid. A discovered instance of
/// that is in `experiment54`'s drift episode: the first version demoted a head
/// that had never disagreed.
pub fn detect_drift(
    head_id: impl Into<String>,
    audits: &[Audit],
    tolerance: f32,
    min_audits: usize,
) -> DriftSignal {
    let head_id = head_id.into();
    let audited = audits.len();
    let disagreements = audits.iter().filter(|a| !a.agreed).count();
    let rate = if audited == 0 {
        0.0
    } else {
        disagreements as f32 / audited as f32
    };
    let (lo, hi) = crate::system_one::calibration::wilson_interval(disagreements, audited, 1.96);
    let (demote, reason) = if audited < min_audits {
        (
            false,
            format!(
                "{audited} audit(s)/{} required: the window is not evidence either way (interval [{lo:.3}, {hi:.3}])",
                min_audits
            ),
        )
    } else if rate > tolerance {
        (
            true,
            format!(
                "observed disagreement {rate:.3} exceeds the {tolerance:.3} tolerance over {audited} audits (interval [{lo:.3}, {hi:.3}])"
            ),
        )
    } else {
        (
            false,
            format!(
                "observed disagreement {rate:.3} is within the {tolerance:.3} tolerance over {audited} audits (interval [{lo:.3}, {hi:.3}])"
            ),
        )
    };
    DriftSignal {
        head_id,
        window: audits.len(),
        audited,
        disagreements,
        rate,
        wilson_hi: hi,
        tolerance,
        demote,
        reason,
    }
}

// ── Dependency index: only recompute what the delta moved ───────────────

/// What a deployed head depends on: the subset of a [`HeadSpec`] the
/// invalidation machinery reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadDependency {
    pub head_id: String,
    pub decision_type: String,
    pub features: Vec<String>,
    /// `DOMAIN\u{0}MODE` cell keys.
    pub cells: Vec<String>,
    /// Ontology entry names.
    pub entry_deps: Vec<String>,
    pub text_view: bool,
}

impl HeadDependency {
    pub fn from_spec(head_id: impl Into<String>, spec: &HeadSpec) -> Self {
        Self {
            head_id: head_id.into(),
            decision_type: spec.decision_type.clone(),
            features: spec.features.clone(),
            cells: spec.cells.clone(),
            entry_deps: spec.entry_deps.clone(),
            text_view: spec.text_view,
        }
    }
}

/// Which deployed heads a mutation touches.
///
/// The index is the Rete-like half of this module: a head's declared inputs are
/// compiled into per-token indexes at registration, so a delta consults the
/// index instead of re-evaluating every head. The edges the index matches
/// against come from the crate's own propagation (see [`plan_recompute`]), not
/// from a parallel bookkeeping of the graph.
#[derive(Debug, Clone, Default)]
pub struct DependencyIndex {
    deps: Vec<HeadDependency>,
    by_cell: HashMap<String, Vec<String>>,
    by_entry: HashMap<String, Vec<String>>,
    by_feature: HashMap<String, Vec<String>>,
}

impl DependencyIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, dep: HeadDependency) {
        for cell in &dep.cells {
            self.by_cell
                .entry(cell.clone())
                .or_default()
                .push(dep.head_id.clone());
        }
        for entry in &dep.entry_deps {
            self.by_entry
                .entry(entry.clone())
                .or_default()
                .push(dep.head_id.clone());
        }
        for feature in &dep.features {
            self.by_feature
                .entry(feature.clone())
                .or_default()
                .push(dep.head_id.clone());
        }
        self.deps.retain(|d| d.head_id != dep.head_id);
        self.deps.push(dep);
    }

    pub fn deregister(&mut self, head_id: &str) {
        self.deps.retain(|d| d.head_id != head_id);
        for map in [&mut self.by_cell, &mut self.by_entry, &mut self.by_feature] {
            for ids in map.values_mut() {
                ids.retain(|id| id != head_id);
            }
        }
    }

    pub fn heads(&self) -> Vec<&str> {
        self.deps.iter().map(|d| d.head_id.as_str()).collect()
    }

    fn dependents(
        &self,
        node: &CoherenceNode,
        label_of: &HashMap<String, String>,
    ) -> HashSet<String> {
        let mut out: HashSet<String> = HashSet::new();
        // An affected node is addressed three ways: by its label (an ontology
        // entry name or a file path), by its pinned cell, and by the feature
        // keys any head claims on it.
        if let Some(label) = &node.label {
            if let Some(ids) = self.by_entry.get(label) {
                out.extend(ids.iter().cloned());
            }
            if let Some(entry) = label_of.get(label) {
                if let Some(ids) = self.by_entry.get(entry) {
                    out.extend(ids.iter().cloned());
                }
            }
            // A text-view head reads every labeled node: the label changed is
            // enough to qualify.
            for d in &self.deps {
                if d.text_view {
                    out.insert(d.head_id.clone());
                }
            }
        }
        if let Some((d, m)) = &node.cell_pin {
            if let Some(ids) = self.by_cell.get(&cell_key(d, m)) {
                out.extend(ids.iter().cloned());
            }
        }
        out
    }

    /// The heads a set of affected node ids touches.
    pub fn invalidate_nodes(
        &self,
        affected: &[String],
        nodes: &[CoherenceNode],
        label_of: &HashMap<String, String>,
    ) -> InvalidationPlan {
        let by_id: HashMap<&str, &CoherenceNode> =
            nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut dirty: HashSet<String> = HashSet::new();
        for id in affected {
            if let Some(node) = by_id.get(id.as_str()) {
                dirty.extend(self.dependents(node, label_of));
            }
        }
        let mut dirty: Vec<String> = dirty.into_iter().collect();
        dirty.sort();
        let skipped: Vec<String> = self
            .deps
            .iter()
            .map(|d| d.head_id.clone())
            .filter(|id| !dirty.contains(id))
            .collect();
        InvalidationPlan {
            dirty_heads: dirty,
            skipped_heads: skipped.clone(),
            recompute_fraction: if self.deps.is_empty() {
                0.0
            } else {
                (self.deps.len() - skipped.len()) as f32 / self.deps.len() as f32
            },
            affected_nodes: affected.len(),
            reason: format!(
                "mutation reached {} node(s); {} head(s) depend on that region",
                affected.len(),
                self.deps.len() - skipped.len()
            ),
        }
    }
}

/// What to redo after a mutation, and what may be left alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidationPlan {
    /// Heads that must be re-fit / re-calibrated before they answer again.
    pub dirty_heads: Vec<String>,
    /// Heads whose declared inputs the delta did not reach.
    pub skipped_heads: Vec<String>,
    /// Share of deployed heads dirtied.
    pub recompute_fraction: f32,
    pub affected_nodes: usize,
    pub reason: String,
}

impl InvalidationPlan {
    pub fn line(&self) -> String {
        format!(
            "dirty [{}]; skipped [{}]; recompute {:.0}% over {} affected node(s)",
            self.dirty_heads.join(", "),
            self.skipped_heads.join(", "),
            self.recompute_fraction * 100.0,
            self.affected_nodes
        )
    }
}

/// Compute which deployed heads one mutation reaches.
///
/// The propagation is the crate's own: an [`EvaluationContext`] over the base
/// nodes, [`evaluate_mutation`] for the shadow wave, and
/// [`EvaluationContext::depends_on_walk`] for the declared-dependency closure —
/// both reports are intersected with the [`DependencyIndex`], and only the
/// intersection is dirty.
///
/// `label_of` maps a node's label to the ontology entry name it was placed
/// from, when that mapping is known; it may be empty, in which case labels match
/// entry names directly.
pub fn plan_recompute(
    nodes: &[CoherenceNode],
    edges: &[TypedEdge],
    mutation: OntologyMutation,
    index: &DependencyIndex,
    label_of: &HashMap<String, String>,
) -> (OntologyDeltaReport, InvalidationPlan) {
    let mut ctx = EvaluationContext::from_base(nodes, &[], edges);
    let report = evaluate_mutation(&mut ctx, mutation);
    let mut affected: HashSet<String> = report
        .affected_nodes
        .iter()
        .map(|n| n.node_id.clone())
        .collect();
    // The revision walk is already on the report when the engine ran one; it is
    // the *declared* dependency half of the propagation, which a breadth-only
    // list of coherence deltas cannot substitute for.
    if let Some(walk) = &report.revision_walk {
        for step in walk.steps.iter().chain(walk.dependents.iter()) {
            affected.insert(step.node_id.clone());
        }
    }
    let affected: Vec<String> = affected.into_iter().collect();
    let plan = index.invalidate_nodes(&affected, nodes, label_of);
    (report, plan)
}

// ── The loop: SemanticJit ─────────────────────────────────────────────────

/// Knobs of [`SemanticJit`].
#[derive(Debug, Clone)]
pub struct JitConfig {
    pub recurrence: RecurrenceConfig,
    pub synthesis: SynthesisConfig,
    pub promotion: PromotionPolicy,
    /// Target accepted-error rate for the conformal gate.
    pub gate_alpha: f32,
    /// Minimum calibration items before a gate is a gate.
    pub min_calibration: usize,
    /// Consecutive failed ticks before the decision type is quarantined (kept on
    /// the expensive rung for `quarantine_ticks`, then retried).
    pub failure_tolerance: usize,
    pub quarantine_ticks: usize,
    /// Audit trigger: re-ask the strong rung when the head's confidence falls
    /// below this. Between `0` and `1`; `0` disables auditing.
    pub audit_below_confidence: f32,
    /// Drift tolerance: admissible disagreement rate with the strong rung.
    pub drift_tolerance: f32,
    pub drift_min_audits: usize,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            recurrence: RecurrenceConfig::default(),
            synthesis: SynthesisConfig::default(),
            promotion: PromotionPolicy::default(),
            gate_alpha: 0.1,
            min_calibration: 10,
            failure_tolerance: 3,
            quarantine_ticks: 5,
            audit_below_confidence: 0.6,
            drift_tolerance: 0.15,
            drift_min_audits: 10,
        }
    }
}

/// One learning episode: what the loop did and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JitOutcome<O> {
    pub decision_type: String,
    pub cell: Option<String>,
    /// What happened, in order.
    pub actions: Vec<String>,
    pub candidates: usize,
    pub promotions: Vec<Promotion<O>>,
    /// Heads retired this tick, with reasons.
    pub retired: Vec<(String, String)>,
    pub quarantined: bool,
}

impl<O> JitOutcome<O> {
    pub fn new(decision_type: impl Into<String>, cell: Option<String>) -> Self {
        Self {
            decision_type: decision_type.into(),
            cell,
            actions: Vec::new(),
            candidates: 0,
            promotions: Vec::new(),
            retired: Vec::new(),
            quarantined: false,
        }
    }

    fn note(&mut self, action: impl Into<String>) {
        self.actions.push(action.into());
    }
}

/// Amounts to report on the ledger.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JitStats {
    pub decisions: u64,
    pub oracle_calls: u64,
    pub oracle_micros: u64,
    pub oracle_usd_micros: u64,
    pub head_calls: u64,
    pub head_micros: u64,
    pub promotions: u64,
    pub rejections: u64,
    pub demotions: u64,
    pub quarantines: u64,
}

impl JitStats {
    /// Share of decisions that never touched the strong rung.
    pub fn oracle_avoidance(&self) -> f32 {
        if self.decisions == 0 {
            0.0
        } else {
            (self.decisions - self.oracle_calls) as f32 / self.decisions as f32
        }
    }
}

/// The loop itself: a trace store, a router, a dependency index, per-head
/// calibration gates, and a ledger.
///
/// One `SemanticJit` serves one decision type. Sharing one struct across decision
/// types would mix label contracts, and the router already refuses that — the
/// type is the boundary rather than a comment.
pub struct SemanticJit<O> {
    pub decision_type: String,
    pub store: TraceStore<O>,
    pub router: HeadRouter<O>,
    pub index: DependencyIndex,
    pub gates: HashMap<String, ConformalGate>,
    pub config: JitConfig,
    stats: JitStats,
    audits: HashMap<String, Vec<Audit>>,
    consecutive_failures: usize,
    quarantined_until: usize,
    tick: usize,
    labels: Vec<O>,
    /// Deployed heads and the specs they answer in, so a later tick walks past a
    /// distinction that is already compiled instead of rebuilding it, and so that
    /// retiring a head frees its distinction for the next derivation.
    deployed_specs: Vec<(String, HeadSpec)>,
}

impl<O: DecisionLabel> SemanticJit<O> {
    /// `labels` is the declared answer space of this decision type.
    pub fn new(decision_type: impl Into<String>, labels: Vec<O>) -> Self {
        Self {
            decision_type: decision_type.into(),
            store: TraceStore::default(),
            router: HeadRouter::new(
                RoutingPolicy::CostOrdered,
                crate::system_one::router::AcceptanceGate::default(),
            ),
            index: DependencyIndex::new(),
            gates: HashMap::new(),
            config: JitConfig::default(),
            stats: JitStats::default(),
            audits: HashMap::new(),
            consecutive_failures: 0,
            quarantined_until: 0,
            tick: 0,
            labels,
            deployed_specs: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: JitConfig) -> Self {
        self.config = config;
        self
    }

    pub fn stats(&self) -> &JitStats {
        &self.stats
    }

    pub fn labels(&self) -> &[O] {
        &self.labels
    }

    pub fn deployed_heads(&self) -> Vec<(&str, HeadKind)> {
        self.router.heads()
    }

    /// Answer through the ladder, recording the trace and the cost on the ledger.
    /// `oracle` is the strong rung, asked only when the ladder has nothing the
    /// gate will admit.
    pub fn route_and_record(
        &mut self,
        input: &DecisionInput,
        oracle: Option<(&dyn DecisionOracle<O>, &HeadSpec)>,
    ) -> RoutedDecision<O> {
        let routed = self.router.route(input, oracle);
        self.stats.decisions += 1;
        for a in &routed.attempts {
            if a.kind.bills_tokens() {
                self.stats.oracle_calls += 1;
                self.stats.oracle_micros += a.cost_micros;
                self.stats.oracle_usd_micros += a.kind.nominal_profile().usd_micros_per_call();
            } else if a.note.starts_with("bandit sampled") {
                // skipped, not run: no cost, no evidence
            } else if a.answered {
                self.stats.head_calls += 1;
                self.stats.head_micros += a.cost_micros;
            }
        }
        let trace = DecisionTrace::from_routed(input, &routed, &self.decision_type);
        self.store.record(trace);
        routed
    }

    /// Attach the outcome of a decision to its trace, by trace id. Returns
    /// whether the trace existed.
    pub fn judge(&mut self, trace_id: &str, label: O, verdict: CoherenceRating) -> bool {
        self.store.judge(trace_id, label, verdict)
    }

    pub fn audit_window(&self, head_id: &str) -> &[Audit] {
        self.audits.get(head_id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Ask the strong rung for the truth the given trace's answer should be
    /// compared against — the sample-based audit of deployed heads.
    pub fn audit_trace(
        &mut self,
        trace: &DecisionTrace<O>,
        oracle: &dyn DecisionOracle<O>,
        spec: &HeadSpec,
    ) -> bool {
        let Some(input) = trace.as_input() else {
            return false;
        };
        let strong = oracle.decide(&input, spec);
        let head_value = trace.value.as_ref().map(|v| v.as_key());
        let strong_value = strong.value.as_ref().map(|v| v.as_key());
        let agreed = head_value == strong_value && strong_value.is_some();
        let was_uncertain = trace.confidence < self.config.audit_below_confidence;
        self.audits
            .entry(trace.head_id.clone())
            .or_default()
            .push(Audit {
                agreed,
                was_uncertain,
            });
        // The outcome feeds the bandit and the quality loop, so an audited head
        // is steered by evidence rather than left to its priors.
        self.router
            .record_outcome(trace.cell.as_deref(), &trace.head_id, agreed);
        agreed
    }

    /// Run one learning episode: detect recurrence, synthesize, fit, calibrate,
    /// promote or refuse — and quarantine the decision type if it keeps failing.
    pub fn tick(&mut self, ontology: &OntologySlice) -> JitOutcome<O> {
        self.tick += 1;
        let mut outcome = JitOutcome::new(&self.decision_type, None);
        if self.tick <= self.quarantined_until {
            outcome.note(format!(
                "quarantined for {} more tick(s): expensive rung only",
                self.quarantined_until - self.tick
            ));
            outcome.quarantined = true;
            return outcome;
        }

        let recurrences = detect_recurrence(self.store.all(), ontology, &self.config.recurrence);
        if recurrences.is_empty() {
            outcome.note(format!(
                "no recurrence over {} trace(s): nothing to compile",
                self.store.len()
            ));
            return outcome;
        }
        outcome.note(format!("{} recurring distinction(s)", recurrences.len()));

        let mut progressed = false;
        for recurrence in recurrences.into_iter().take(3) {
            outcome.cell = recurrence.cell.clone();
            let live: Vec<HeadSpec> = self
                .deployed_specs
                .iter()
                .map(|(_, spec)| spec.clone())
                .collect();
            let Some(candidate) = synthesize_avoiding(&recurrence, &self.config.synthesis, &live)
            else {
                outcome.note(format!(
                    "region {}: recurring, but no cheap rung expresses it — stays expensive",
                    recurrence
                        .cell
                        .clone()
                        .unwrap_or_else(|| "unscoped".to_string())
                ));
                continue;
            };
            outcome.candidates += 1;
            outcome.note(format!(
                "candidate: {} — {}",
                candidate.kind.as_str(),
                candidate.rationale
            ));
            let promotion = self.try_candidate(candidate, &recurrence);
            outcome.note(format!(
                "promotion {}: {}",
                promotion.head_id, promotion.accepted
            ));
            if promotion.accepted {
                outcome.promotions.push(promotion);
                progressed = true;
            } else {
                outcome.note(promotion.reason.clone());
                self.stats.rejections += 1;
            }
        }

        if progressed {
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures += 1;
            if self.consecutive_failures >= self.config.failure_tolerance {
                self.quarantined_until = self.tick + self.config.quarantine_ticks;
                self.stats.quarantines += 1;
                outcome.note(format!(
                    "{} consecutive failed tick(s): quarantined for {} ticks",
                    self.consecutive_failures, self.config.quarantine_ticks
                ));
                outcome.quarantined = true;
                self.consecutive_failures = 0;
            }
        }
        outcome
    }

    /// Fit one candidate on the train slice, calibrate its gate on the hold-out
    /// slice, and decide whether it may be deployed.
    fn try_candidate(
        &mut self,
        candidate: HeadCandidate<O>,
        recurrence: &Recurrence<O>,
    ) -> Promotion<O> {
        let head_id = format!(
            "{}-{}-{:04}",
            candidate.kind.as_str(),
            self.decision_type,
            self.tick
        );
        let examples: Vec<TrainingExample<O>> = self
            .store
            .examples_in(&recurrence.decision_type, recurrence.cell.as_deref());

        let empty_slice = || SliceScore {
            answered: 0,
            accepted: 0,
            ladder_accuracy: 0.0,
            accepted_accuracy: 0.0,
            coverage: 0.0,
        };

        // Chronological split: fit on the early window, validate on the tail —
        // the tail is the future as far as the gate is concerned.
        let n = examples.len();
        let holdout = (n as f32 * self.config.synthesis.holdout_fraction).round() as usize;
        let holdout = holdout.max(1).min(n.saturating_sub(1));
        let split_at = n - holdout;
        let (train, tail) = examples.split_at(split_at);

        let (head, fit_report) = instantiate(&candidate, train, &head_id);
        if !fit_report.is_deployable() {
            return Promotion {
                head_id,
                kind: candidate.kind,
                accepted: false,
                reason: format!("fit refused: {fit_report:?}"),
                spec: candidate.spec.clone(),
                candidate: Some(candidate),
                gate: None,
                calibration: None,
                head_slice: empty_slice(),
                baseline_slice: baseline_score(&[]),
                delta_ci: (0.0, 0.0, 0.0),
                frontier: Vec::new(),
            };
        }

        let gate = calibrate_over(&head_id, head.as_ref(), tail, self.config.gate_alpha);
        let answers = calibration_pairs(head.as_ref(), tail);
        let baseline_correct = self.baseline_correctness(tail);
        let mut promotion = evaluate_promotion(
            head.as_ref(),
            &head_id,
            tail,
            &baseline_correct,
            &gate,
            &self.config.promotion,
            Some(candidate.clone()),
        );
        promotion.calibration = Some(CalibrationReport::evaluate(&head_id, &answers, &gate));
        if promotion.accepted {
            // Deploy: register in the router, the index, and the gate map. The
            // gate travels with the head id, so retiring the head retires the
            // gate with it.
            let spec = head.spec().clone();
            if let Err(e) = self.router.deploy(head) {
                promotion.accepted = false;
                promotion.reason = format!("deploy refused by the ladder: {e}");
                return promotion;
            }
            self.index
                .register(crate::system_one::jit::HeadDependency::from_spec(
                    &head_id, &spec,
                ));
            self.gates.insert(head_id.clone(), gate.clone());
            self.router.gate_mut().by_head.insert(head_id.clone(), gate);
            self.deployed_specs.push((head_id, spec));
            self.stats.promotions += 1;
        }
        promotion
    }

    /// The strong-rung comparison over the hold-out slice.
    ///
    /// A trace's label *is* the validated outcome the strong rung produced, so the
    /// baseline is right wherever a trace was validated. This is not a second
    /// opinion about the world; it is the record the oracle leg left behind, and
    /// the paired bootstrap compares the head against exactly that.
    fn baseline_correctness(&self, tail: &[TrainingExample<O>]) -> Vec<bool> {
        tail.iter()
            .map(|e| e.verdict != CoherenceRating::Failure)
            .collect()
    }

    /// Judge a disagreement window for one deployed head and retire it when the
    /// tolerance is exceeded.
    pub fn check_drift(&mut self, head_id: &str) -> Option<DriftSignal> {
        let audits: Vec<Audit> = self.audit_window(head_id).to_vec();
        let signal = detect_drift(
            head_id,
            &audits,
            self.config.drift_tolerance,
            self.config.drift_min_audits,
        );
        if signal.demote {
            self.router.retire(head_id);
            self.index.deregister(head_id);
            self.gates.remove(head_id);
            // A retired head's distinction is available again: the next tick may
            // re-derive it from fresher traces, which is the point of retiring.
            self.deployed_specs.retain(|(id, _)| id != head_id);
            self.stats.demotions += 1;
        }
        Some(signal)
    }

    /// Invalidate deployed heads after an ontology mutation, retiring only the
    /// dirty ones: the "only recompute what moved" contract, enforced by
    /// construction rather than by audit.
    pub fn invalidate_after(
        &mut self,
        nodes: &[CoherenceNode],
        edges: &[TypedEdge],
        mutation: OntologyMutation,
        label_of: &HashMap<String, String>,
    ) -> InvalidationPlan {
        let (_report, plan) = plan_recompute(nodes, edges, mutation, &self.index, label_of);
        for head_id in &plan.dirty_heads {
            self.router.retire(head_id);
            self.index.deregister(head_id);
            self.gates.remove(head_id);
            // A retired head's distinction is available again: the next tick may
            // re-derive it from fresher traces, which is the point of retiring.
            self.deployed_specs.retain(|(id, _)| id != head_id);
            self.stats.demotions += 1;
        }
        plan
    }
}

/// Calibrate one head's gate on the hold-out answers it gives.
///
/// The observations are `(head confidence, was it right)` — *head* confidence,
/// not example weight: the gate thresholds what the head says, so calibrating on
/// anything else would calibrate a hypothetical head.
fn calibrate_over<O: DecisionLabel>(
    head_id: &str,
    head: &dyn HeadFor<O>,
    tail: &[TrainingExample<O>],
    alpha: f32,
) -> ConformalGate {
    let observations: Vec<ConfidenceOutcome> = tail
        .iter()
        .map(|e| {
            let d = head.decide(&e.input);
            let correct = d.value.as_ref().map(|v| v == &e.output).unwrap_or(false);
            ConfidenceOutcome::new(d.confidence, correct)
        })
        .collect();
    ConformalGate::calibrate(format!("{head_id}-gate"), &observations, alpha, 2)
}

/// `(Some(confidence), correct)` answers over the hold-out slice: `None` for an
/// abstention, which [`CalibrationReport::evaluate`] counts in `answered` and
/// refuses to average in as an error.
fn calibration_pairs<O: DecisionLabel>(
    head: &dyn HeadFor<O>,
    tail: &[TrainingExample<O>],
) -> Vec<(Option<f32>, bool)> {
    tail.iter()
        .map(|e| {
            let d = head.decide(&e.input);
            let correct = d.value.as_ref().map(|v| v == &e.output).unwrap_or(false);
            (d.value.as_ref().map(|_| d.confidence), correct)
        })
        .collect()
}
