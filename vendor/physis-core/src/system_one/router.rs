//! Routing: which rung answers, and what that is allowed to cost.
//!
//! The router walks the ladder from cheapest to strongest, admits the first
//! answer that clears its gate, and never substitutes a guess for an abstention.
//! Three selection policies ship, because "which cheap head" is an empirical
//! question and the design should let it be asked rather than answered by
//! preference:
//!
//! - [`RoutingPolicy::CostOrdered`] — the default cascade (FrugalGPT-shaped:
//!   Chen, Zaharia & Zou 2023, arXiv:2305.05176). Deterministic and auditable.
//! - [`RoutingPolicy::ThompsonSampling`] — a Beta-Bernoulli contextual bandit
//!   keyed on the semiotic cell the world was placed in, after Thompson (1933)
//!   and Chapelle & Li, *An Empirical Evaluation of Thompson Sampling* (2011).
//!   The cell is the context because that is the region a head's competence is
//!   actually conditional on; a single global arm would average across regions
//!   and learn nothing usable.
//! - [`RoutingPolicy::Fixed`] — one named head only. This is the ablation arm;
//!   without it, "the ladder helps" cannot be separated from "the best single
//!   head helps".
//!
//! Cost is measured per head as it runs (an exponential moving average of wall
//! clock micros), so the ladder's order comes from this machine's numbers rather
//! than from the nominal table in
//! [`crate::system_one::HeadKind::nominal_profile`]. The nominal table is the
//! prior; a measurement is the evidence.

use std::collections::HashMap;
use std::time::Instant;

use crate::quality::QualityTracker;
use crate::system_one::calibration::ConformalGate;
use crate::system_one::{
    Cost, CostSourceMarker, Decision, DecisionInput, DecisionLabel, DecisionOracle, HeadFor,
    HeadKind, HeadSpec, Provenance,
};

/// Which answers are admissible.
///
/// A head's own gate is used when one has been calibrated for it; otherwise the
/// router's floor applies. Two levels rather than one, so a calibrated head can
/// be admitted at 0.55 while an uncalibrated one still needs 0.9.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptanceGate {
    /// Floor for heads with no calibrated gate of their own.
    pub min_confidence: f32,
    /// Per-head calibrated gates, keyed by head id.
    pub by_head: HashMap<String, ConformalGate>,
    /// Minimum standing, from the crate's own quality loop. A head the engine has
    /// been demoting is not accepted just because it is confident.
    pub min_standing: f32,
}

impl Default for AcceptanceGate {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            by_head: HashMap::new(),
            min_standing: 0.5,
        }
    }
}

impl AcceptanceGate {
    pub fn new(min_confidence: f32) -> Self {
        Self {
            min_confidence: min_confidence.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    pub fn with_gate(mut self, head_id: impl Into<String>, gate: ConformalGate) -> Self {
        self.by_head.insert(head_id.into(), gate);
        self
    }

    /// Admit an answer of this confidence from this head.
    pub fn admits(&self, head_id: &str, confidence: f32, standing: f32) -> bool {
        if standing < self.min_standing {
            return false;
        }
        match self.by_head.get(head_id) {
            Some(gate) => gate.accepts(confidence),
            None => confidence >= self.min_confidence,
        }
    }

    pub fn gate_for(&self, head_id: &str) -> Option<&ConformalGate> {
        self.by_head.get(head_id)
    }
}

/// How the router chooses among heads.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingPolicy {
    /// Cheapest admissible first.
    CostOrdered,
    /// Beta-Bernoulli Thompson sampling over per-cell competence, with the
    /// cheapest surviving arm taking the call.
    ThompsonSampling {
        /// A sampled success rate below this means "not here"; the walk continues
        /// to a stronger rung.
        floor: f32,
        seed: u64,
    },
    /// One head only — the ablation arm.
    Fixed(String),
}

impl RoutingPolicy {
    pub fn name(&self) -> String {
        match self {
            RoutingPolicy::CostOrdered => "cost-ordered".into(),
            RoutingPolicy::ThompsonSampling { .. } => "thompson".into(),
            RoutingPolicy::Fixed(id) => format!("fixed:{id}"),
        }
    }
}

/// Beta-Bernoulli posterior for one (context, head) arm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BetaPosterior {
    pub alpha: f64,
    pub beta: f64,
    pub draws: u64,
}

impl Default for BetaPosterior {
    fn default() -> Self {
        // Beta(1,1): no information. A head starts with the benefit of no doubt
        // and no credit, which is the only honest prior for a new rung.
        Self {
            alpha: 1.0,
            beta: 1.0,
            draws: 0,
        }
    }
}

impl BetaPosterior {
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    pub fn observe(&mut self, correct: bool) {
        if correct {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
        self.draws += 1;
    }
}

/// Per-cell bandit over head selection.
#[derive(Debug, Clone)]
pub struct Bandit {
    arms: HashMap<String, HashMap<String, BetaPosterior>>,
    rng: rand::rngs::StdRng,
}

impl Bandit {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            arms: HashMap::new(),
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    fn key(context: Option<&str>) -> String {
        context.unwrap_or("unscoped").to_string()
    }

    pub fn posterior(&self, context: Option<&str>, head_id: &str) -> BetaPosterior {
        self.arms
            .get(&Self::key(context))
            .and_then(|m| m.get(head_id))
            .copied()
            .unwrap_or_default()
    }

    /// Sample the arm's success rate. Deterministic given the seed and the
    /// observation order, which is what lets a benchmark reproduce a run.
    pub fn sample(&mut self, context: Option<&str>, head_id: &str) -> f64 {
        let p = self.posterior(context, head_id);
        let x = sample_gamma(&mut self.rng, p.alpha);
        let y = sample_gamma(&mut self.rng, p.beta);
        (x / (x + y)).clamp(0.0, 1.0)
    }

    pub fn observe(&mut self, context: Option<&str>, head_id: &str, correct: bool) {
        self.arms
            .entry(Self::key(context))
            .or_default()
            .entry(head_id.to_string())
            .or_default()
            .observe(correct);
    }

    /// Contexts with any observation, for reporting.
    pub fn contexts(&self) -> Vec<String> {
        let mut v: Vec<String> = self.arms.keys().cloned().collect();
        v.sort();
        v
    }
}

/// Gamma variate: Marsaglia & Tsang (2000) for shape ≥ 1, with Johnk's boost
/// below 1. Written out rather than depended on, because a bandit is a dozen
/// lines and a distributions crate for a dozen lines is the wrong dependency for
/// an engine. The algorithm is credited, not invented.
fn sample_gamma(rng: &mut rand::rngs::StdRng, shape: f64) -> f64 {
    use rand::Rng;
    if shape < 1.0 {
        let u: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
        return sample_gamma(rng, shape + 1.0) * u.powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x: f64 = rng.gen::<f64>() * 2.0 - 1.0;
        let y: f64 = rng.gen::<f64>();
        let v = 1.0 + c * x;
        if v <= 0.0 {
            continue;
        }
        let v = v * v * v;
        let u: f64 = rng.gen::<f64>();
        if u < 1.0 - 0.0331 * x.powi(4) {
            return d * v;
        }
        if y.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

/// One rung's turn at the question.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Attempt {
    pub head_id: String,
    pub kind: HeadKind,
    pub answered: bool,
    pub accepted: bool,
    pub confidence: f32,
    pub cost_micros: u64,
    pub note: String,
}

/// The router's answer, plus everything that happened on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutedDecision<O> {
    pub decision: Decision<O>,
    pub attempts: Vec<Attempt>,
    /// True when the ladder ran out and the strongest configured rung took it.
    pub escalated: bool,
    pub total_cost: Cost,
    pub policy: String,
}

impl<O: Clone> RoutedDecision<O> {
    pub fn value(&self) -> Option<&O> {
        self.decision.value.as_ref()
    }

    /// Which rung answered.
    pub fn answered_by(&self) -> Option<&str> {
        self.attempts
            .iter()
            .find(|a| a.accepted)
            .map(|a| a.head_id.as_str())
    }

    pub fn cost_micros(&self) -> u64 {
        self.total_cost.micros
    }
}

/// The ladder.
pub struct HeadRouter<O> {
    heads: Vec<Box<dyn HeadFor<O>>>,
    gate: AcceptanceGate,
    policy: RoutingPolicy,
    /// Measured micros per head, exponential moving average.
    measured: HashMap<String, f64>,
    bandit: Bandit,
    quality: Option<Box<QualityTracker>>,
    /// The label-space contract every head in this router must share.
    contract: Option<String>,
}

impl<O: DecisionLabel> HeadRouter<O> {
    pub fn new(policy: RoutingPolicy, gate: AcceptanceGate) -> Self {
        let seed = match &policy {
            RoutingPolicy::ThompsonSampling { seed, .. } => *seed,
            _ => 7,
        };
        Self {
            heads: Vec::new(),
            gate,
            policy,
            measured: HashMap::new(),
            bandit: Bandit::new(seed),
            quality: None,
            contract: None,
        }
    }

    /// Attach the crate's quality tracker, so a head's standing is the same state
    /// the rest of the engine already maintains rather than a private score kept
    /// in this module.
    pub fn with_quality_tracker(mut self, tracker: QualityTracker) -> Self {
        self.quality = Some(Box::new(tracker));
        self
    }

    /// Deploy a head.
    ///
    /// Refuses — and says why — if it answers in a different label space than the
    /// heads already installed: two contracts in one ladder is a bug that would
    /// otherwise surface as a mysterious accuracy loss. A head of the same kind
    /// replaces its predecessor, because two rules for one decision *are* one
    /// rule set.
    pub fn deploy(&mut self, head: Box<dyn HeadFor<O>>) -> Result<(), String> {
        let space = head.spec().label_space.clone();
        match &self.contract {
            Some(existing) if *existing != space => {
                return Err(format!(
                    "head '{}' answers in '{}' but this ladder is bound to '{}'",
                    head.id(),
                    space,
                    existing
                ))
            }
            Some(_) => {}
            None => self.contract = Some(space),
        }
        self.heads
            .retain(|h| h.id() != head.id() && h.kind() != head.kind());
        self.heads.push(head);
        self.sort_by_cost();
        Ok(())
    }

    pub fn heads(&self) -> Vec<(&str, HeadKind)> {
        self.heads.iter().map(|h| (h.id(), h.kind())).collect()
    }

    pub fn head_count(&self) -> usize {
        self.heads.len()
    }

    pub fn has_head(&self, id: &str) -> bool {
        self.heads.iter().any(|h| h.id() == id)
    }

    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }

    pub fn gate(&self) -> &AcceptanceGate {
        &self.gate
    }

    pub fn gate_mut(&mut self) -> &mut AcceptanceGate {
        &mut self.gate
    }

    pub fn set_policy(&mut self, policy: RoutingPolicy) {
        self.policy = policy;
        self.sort_by_cost();
    }

    pub fn policy(&self) -> &RoutingPolicy {
        &self.policy
    }

    /// Remove a head — how a demotion is actually enforced.
    pub fn retire(&mut self, head_id: &str) -> bool {
        let before = self.heads.len();
        self.heads.retain(|h| h.id() != head_id);
        self.heads.len() != before
    }

    /// Current cost estimate for a head: the measurement when there is one, the
    /// nominal profile otherwise.
    pub fn cost_of(&self, head: &dyn HeadFor<O>) -> u64 {
        match self.measured.get(head.id()) {
            Some(m) => *m as u64,
            None => head.profile().micros,
        }
    }

    /// Measured micros for a head, when it has been used at least once.
    pub fn measured_micros(&self, head_id: &str) -> Option<u64> {
        self.measured.get(head_id).map(|m| *m as u64)
    }

    fn sort_by_cost(&mut self) {
        let measured = self.measured.clone();
        self.heads.sort_by_key(|h| {
            measured
                .get(h.id())
                .map(|m| *m as u64)
                .unwrap_or_else(|| h.profile().micros)
        });
    }

    fn record_cost(&mut self, head_id: &str, micros: u64) {
        // EMA with alpha = 0.3: responsive to a real change, not to one slow call.
        let entry = self
            .measured
            .entry(head_id.to_string())
            .or_insert(micros as f64);
        *entry = 0.7 * *entry + 0.3 * micros as f64;
    }

    fn standing(&self, head_id: &str) -> f32 {
        match &self.quality {
            Some(q) => {
                let penalty = q.agent_penalties.get(head_id).copied().unwrap_or(0.0);
                let boost = q.agent_boosts.get(head_id).copied().unwrap_or(0.0);
                1.0 - penalty + boost
            }
            None => 1.0,
        }
    }

    pub fn standing_of(&self, head_id: &str) -> f32 {
        self.standing(head_id)
    }

    /// Report a right or wrong answer from a head: the crate's own quality loop,
    /// keyed on the head as an agent, plus the bandit's posterior.
    pub fn record_outcome(&mut self, context: Option<&str>, head_id: &str, correct: bool) {
        if let Some(q) = &mut self.quality {
            if correct {
                q.report_agent_success(head_id);
            } else {
                q.report_agent_failure(head_id);
            }
        }
        self.bandit.observe(context, head_id, correct);
    }

    pub fn bandit(&self) -> &Bandit {
        &self.bandit
    }

    /// Walk the ladder for one decision.
    ///
    /// `oracle` is the strongest configured rung, tried only after every head has
    /// abstained or been refused by the gate. If it is `None` and nothing
    /// accepted, the answer is an abstention — which is a result, not an error.
    pub fn route(
        &mut self,
        input: &DecisionInput,
        oracle: Option<(&dyn DecisionOracle<O>, &HeadSpec)>,
    ) -> RoutedDecision<O> {
        let context = input.cell();
        let mut attempts: Vec<Attempt> = Vec::new();
        let mut total = Cost::ZERO;
        let mut last: Option<Decision<O>> = None;

        for i in 0..self.heads.len() {
            let head_id = self.heads[i].id().to_string();
            let kind = self.heads[i].kind();
            let allowed = match &self.policy {
                RoutingPolicy::Fixed(id) => id == &head_id,
                _ => true,
            };
            if !allowed {
                continue;
            }
            if let RoutingPolicy::ThompsonSampling { floor, .. } = &self.policy {
                let floor = *floor;
                let draws = self.bandit.posterior(context.as_deref(), &head_id).draws;
                let sample = self.bandit.sample(context.as_deref(), &head_id);
                // Explore an unmeasured arm once rather than refusing it forever;
                // after that the posterior has to justify the trial.
                if draws > 0 && sample < floor as f64 {
                    attempts.push(Attempt {
                        head_id,
                        kind,
                        answered: false,
                        accepted: false,
                        confidence: 0.0,
                        cost_micros: 0,
                        note: format!("bandit sampled {sample:.2} < floor {floor:.2}"),
                    });
                    continue;
                }
            }

            let standing = self.standing(&head_id);
            let started = Instant::now();
            let mut decision = self.heads[i].decide(input);
            let micros = started.elapsed().as_micros() as u64;
            self.record_cost(&head_id, micros);
            decision.cost = Cost {
                micros,
                source: CostSourceMarker::Measured,
                ..decision.cost
            };
            total = total.plus(decision.cost);
            let answered = !decision.did_abstain();
            let accepted = answered && self.gate.admits(&head_id, decision.confidence, standing);
            let note = decision.provenance.derivation.clone().unwrap_or_else(|| {
                if answered {
                    "answered".to_string()
                } else {
                    "abstained".to_string()
                }
            });
            attempts.push(Attempt {
                head_id: head_id.clone(),
                kind,
                answered,
                accepted,
                confidence: decision.confidence,
                cost_micros: micros,
                note,
            });
            if accepted {
                return RoutedDecision {
                    decision,
                    attempts,
                    escalated: false,
                    total_cost: total,
                    policy: self.policy.name(),
                };
            }
            last = Some(decision);
        }

        // The strong rung, only now.
        let mut escalated = false;
        if let Some((oracle, spec)) = oracle {
            let started = Instant::now();
            let mut decision = oracle.decide(input, spec);
            let micros = started.elapsed().as_micros() as u64;
            let profile_cost = oracle.profile();
            let billing = Cost::from_profile(&profile_cost);
            // A local stand-in answers in microseconds and cannot measure a
            // network round trip, so for a token-billed rung the *profile* is the
            // floor: reporting the local timing as the cost of a remote call
            // would make the ledger claim a saving that no deployment got.
            let cost = Cost {
                micros: if oracle.kind().bills_tokens() {
                    billing.micros.max(micros)
                } else if micros > 0 {
                    micros
                } else {
                    billing.micros
                },
                ..billing
            };
            decision.cost = cost;
            total = total.plus(cost);
            escalated = true;
            attempts.push(Attempt {
                head_id: oracle.name().to_string(),
                kind: oracle.kind(),
                answered: !decision.did_abstain(),
                accepted: !decision.did_abstain(),
                confidence: decision.confidence,
                cost_micros: cost.micros,
                note: "strong rung".to_string(),
            });
            last = Some(decision);
        }

        let decision = last.unwrap_or_else(|| {
            Decision::abstain(
                Provenance::head(
                    "none",
                    HeadKind::Llm,
                    self.contract.clone().unwrap_or_default(),
                ),
                Cost::ZERO,
            )
        });
        RoutedDecision {
            decision,
            attempts,
            escalated,
            total_cost: total,
            policy: self.policy.name(),
        }
    }

    /// Answer a batch. A System-One oracle answers all of them in one round trip
    /// when its schema says so; the heads are still walked one at a time, because
    /// they are local and the round trip is the thing worth amortising.
    pub fn route_batch(
        &mut self,
        inputs: &[DecisionInput],
        oracle: Option<(&dyn DecisionOracle<O>, &HeadSpec)>,
    ) -> Vec<RoutedDecision<O>> {
        inputs.iter().map(|i| self.route(i, oracle)).collect()
    }
}
