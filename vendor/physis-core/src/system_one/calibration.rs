//! Calibration and selective prediction for decision heads.
//!
//! A head that says `0.9` must be right nine times in ten *in the region it is
//! deployed in*, or its confidence cannot be routed on. This module is the
//! machinery that makes that true locally, with no assumption about the head,
//! the label space, or the embedder behind it.
//!
//! ## What is implemented, and from where
//!
//! - **Split conformal / selective risk control** — a threshold on confidence
//!   chosen so that *accepted* accuracy clears `1 − α` on held-out data:
//!   [`conformal_quantile`], [`ConformalGate`]. The idea and its finite-sample
//!   caveat are from Angelopoulos & Bates, *A Gentle Introduction to Conformal
//!   Prediction and Distribution-Free Uncertainty Quantification* (2021,
//!   arXiv:2107.07511) and Geifman & El-Yaniv, *Selective Classification for
//!   Deep Neural Networks* (ICML 2017). The implementation is a scan over the
//!   calibration set's nonconformity scores — no library is vendored, and none
//!   is needed for a scan.
//! - **Expected Calibration Error** and **Brier score** — the two standard
//!   scalar summaries (Guo et al. 2017, *On Calibration of Modern Neural
//!   Networks*; Brier 1950). Equal-width bins, reported with the bin count, so a
//!   reader can see how coarse the estimate is.
//! - **Wilson interval** — for a proportion from few samples, which is the
//!   normal case when a deployment has run for a day (Wilson 1927).
//! - **Paired bootstrap** — for "did the cheap head change the answer set?",
//!   resampling *items*, never arms, because both rungs answered the same items
//!   and a two-sample test would throw that pairing away.
//!
//! ## What is deliberately not implemented
//!
//! Temperature scaling and isotonic regression re-map a head's probabilities
//! rather than gate them. They are the right tools when a head's *distribution*
//! is trusted and only its scale is wrong, and the wrong tool here: the ladder's
//! question is "should this answer be used at all", which a threshold answers
//! and a rescaling does not. A head whose probabilities are wanted downstream
//! can be rescaled by its owner; the gate in front of it stays a gate.
//!
//! ## The honest caveat, stated where it applies
//!
//! Split conformal's guarantee is *marginal* and holds under exchangeability
//! between calibration and deployment. Traces from a live system are **not**
//! exchangeable with a deployment that starts after a semantic delta moves —
//! which is exactly why [`crate::system_one::jit`] invalidates and re-calibrates
//! heads by region instead of trusting one global gate.

use serde::{Deserialize, Serialize};

/// One calibration observation: the confidence a head claimed, and whether it
/// was right.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceOutcome {
    pub confidence: f32,
    pub correct: bool,
}

impl ConfidenceOutcome {
    pub fn new(confidence: f32, correct: bool) -> Self {
        Self {
            confidence: confidence.clamp(0.0, 1.0),
            correct,
        }
    }
}

/// The largest-coverage threshold whose accepted set meets the target accuracy.
///
/// The threshold is chosen over *values*, not ranks: τ is admissible when the
/// set `{conf ≥ τ}` itself meets `1 − α` with at least `min_accepted` items. A
/// top-k scan would lie here, because two items sharing the same confidence are
/// admitted together — a prefix can look clean while the value-set it implies is
/// not.
///
/// Returns `1.0` when no threshold clears the target — meaning *accept nothing*,
/// which is the correct answer rather than a failure: a head that cannot reach
/// the target anywhere must not be deployed anywhere.
///
/// `min_accepted` guards the degenerate solution "accept the one item that was
/// right": a threshold admitting fewer than `min_accepted` items is not evidence
/// of anything, so it is not allowed to be the operating point.
pub fn conformal_quantile(
    observations: &[ConfidenceOutcome],
    alpha: f32,
    min_accepted: usize,
) -> f32 {
    let target = 1.0 - alpha.clamp(0.0, 1.0);
    if observations.is_empty() {
        return 1.0;
    }
    let mut uniq: Vec<f32> = observations.iter().map(|o| o.confidence).collect();
    uniq.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    uniq.dedup();
    let mut best: Option<(f32, usize)> = None; // (tau, admitted)
    for t in uniq {
        let (n, acc) = accepted_accuracy(observations, t);
        if n >= min_accepted.max(1) && acc >= target {
            match best {
                Some((_, bn)) if bn >= n => {}
                _ => best = Some((t, n)),
            }
        }
    }
    best.map(|(t, _)| t).unwrap_or(1.0)
}

/// A calibrated admission rule for one head in one region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConformalGate {
    pub id: String,
    /// Target error rate among accepted answers: accepted accuracy ≥ `1 − α`.
    pub alpha: f32,
    /// Confidence at or above which an answer is admitted.
    pub tau: f32,
    /// Calibration sample size. Zero means "no gate was calibrated", which the
    /// gate reports rather than hides: `tau = 1.0`, nothing admitted.
    pub n: usize,
    /// Which method produced `tau`.
    pub method: String,
}

impl ConformalGate {
    /// A gate that admits nothing until it has been calibrated.
    pub fn uncalibrated(id: impl Into<String>, alpha: f32) -> Self {
        Self {
            id: id.into(),
            alpha,
            tau: 1.0,
            n: 0,
            method: "uncalibrated".to_string(),
        }
    }

    /// Calibrate on held-out observations. Chronological order is the caller's
    /// job: passing training items here is how an optimistic gate is built, and
    /// no amount of arithmetic downstream can undo it.
    pub fn calibrate(
        id: impl Into<String>,
        observations: &[ConfidenceOutcome],
        alpha: f32,
        min_accepted: usize,
    ) -> Self {
        Self {
            id: id.into(),
            alpha,
            tau: conformal_quantile(observations, alpha, min_accepted),
            n: observations.len(),
            method: "split-conformal-selective".to_string(),
        }
    }

    pub fn calibrated(&self) -> bool {
        self.n > 0
    }

    /// Admit a confidence at or above the operating point, once the gate has
    /// been calibrated. `n == 0` admits nothing: no data is not a licence to
    /// guess.
    pub fn accepts(&self, confidence: f32) -> bool {
        self.n > 0 && confidence >= self.tau
    }

    /// Whether a refit moved the operating point enough to matter. Lets the JIT
    /// loop say *why* it re-deployed a head instead of silently swapping the
    /// threshold under a running deployment.
    pub fn materially_differs_from(&self, other: &ConformalGate, epsilon: f32) -> bool {
        (self.tau - other.tau).abs() > epsilon
    }
}

/// Everything measurable about one head on one held-out set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationReport {
    pub head_id: String,
    pub n: usize,
    pub alpha: f32,
    /// Fraction of items the head answered at all (as opposed to abstaining).
    pub answered: f32,
    /// Accuracy over everything it answered.
    pub accuracy: f32,
    /// Fraction admitted by the gate.
    pub coverage: f32,
    /// Accuracy over admitted items — the number that must clear `1 − α`.
    pub accepted_accuracy: f32,
    pub mean_confidence: f32,
    /// Expected calibration error over equal-width bins.
    pub ece: f32,
    pub ece_bins: usize,
    pub brier: f32,
    pub tau: f32,
    pub method: String,
}

impl CalibrationReport {
    /// Score a head's answers against the gate that will front them.
    ///
    /// `answers` is `(confidence_or_None, correct)`: `None` is an abstention,
    /// counted in `answered` but never in `accuracy` — averaging an abstention
    /// in as a wrong answer would punish a head for the honesty the design asks
    /// of it.
    pub fn evaluate(
        head_id: impl Into<String>,
        answers: &[(Option<f32>, bool)],
        gate: &ConformalGate,
    ) -> Self {
        let n = answers.len();
        let pairs: Vec<ConfidenceOutcome> = answers
            .iter()
            .filter_map(|(c, correct)| c.map(|c| ConfidenceOutcome::new(c, *correct)))
            .collect();
        let answered_rate = if n == 0 {
            0.0
        } else {
            pairs.len() as f32 / n as f32
        };
        let accuracy = if pairs.is_empty() {
            0.0
        } else {
            pairs.iter().filter(|p| p.correct).count() as f32 / pairs.len() as f32
        };
        let admitted: Vec<&ConfidenceOutcome> = pairs
            .iter()
            .filter(|p| gate.accepts(p.confidence))
            .collect();
        let coverage = if n == 0 {
            0.0
        } else {
            admitted.len() as f32 / n as f32
        };
        let accepted_accuracy = if admitted.is_empty() {
            0.0
        } else {
            admitted.iter().filter(|p| p.correct).count() as f32 / admitted.len() as f32
        };
        let mean_confidence = if pairs.is_empty() {
            0.0
        } else {
            pairs.iter().map(|p| p.confidence).sum::<f32>() / pairs.len() as f32
        };
        let (ece_v, bins) = ece(&pairs, 10);

        Self {
            head_id: head_id.into(),
            n,
            alpha: gate.alpha,
            answered: answered_rate,
            accuracy,
            coverage,
            accepted_accuracy,
            mean_confidence,
            ece: ece_v,
            ece_bins: bins,
            brier: brier(&pairs),
            tau: gate.tau,
            method: gate.method.clone(),
        }
    }

    /// Whether the accepted set met the target on this hold-out.
    pub fn met_target(&self) -> bool {
        self.coverage > 0.0 && self.accepted_accuracy >= 1.0 - self.alpha
    }

    pub fn line(&self) -> String {
        format!(
            "{}: n={} answered={:.3} acc={:.3} tau={:.3} cov={:.3} acc@cov={:.3} ece={:.3} brier={:.3}",
            self.head_id,
            self.n,
            self.answered,
            self.accuracy,
            self.tau,
            self.coverage,
            self.accepted_accuracy,
            self.ece,
            self.brier
        )
    }
}

/// Accuracy over items a gate at `tau` would admit: `(admitted, accuracy)`.
pub fn accepted_accuracy(observations: &[ConfidenceOutcome], tau: f32) -> (usize, f32) {
    let admitted: Vec<&ConfidenceOutcome> = observations
        .iter()
        .filter(|o| o.confidence >= tau)
        .collect();
    if admitted.is_empty() {
        return (0, 0.0);
    }
    let hits = admitted.iter().filter(|o| o.correct).count();
    (admitted.len(), hits as f32 / admitted.len() as f32)
}

/// Expected calibration error over `bins` equal-width confidence bins, plus the
/// bin count actually used — a reader needs to know how coarse the estimate is.
pub fn ece(observations: &[ConfidenceOutcome], bins: usize) -> (f32, usize) {
    if observations.is_empty() || bins == 0 {
        return (0.0, bins);
    }
    let mut total = 0.0f32;
    let edge = 1.0 / bins as f32;
    for b in 0..bins {
        let lo = b as f32 * edge;
        let hi = if b + 1 == bins {
            1.0 + 1e-6
        } else {
            (b + 1) as f32 * edge
        };
        let in_bin: Vec<&ConfidenceOutcome> = observations
            .iter()
            .filter(|o| o.confidence >= lo && o.confidence < hi)
            .collect();
        if in_bin.is_empty() {
            continue;
        }
        let conf = in_bin.iter().map(|o| o.confidence).sum::<f32>() / in_bin.len() as f32;
        let acc = in_bin.iter().filter(|o| o.correct).count() as f32 / in_bin.len() as f32;
        total += (in_bin.len() as f32 / observations.len() as f32) * (acc - conf).abs();
    }
    (total, bins)
}

/// Mean squared error of the confidence against the outcome.
pub fn brier(observations: &[ConfidenceOutcome]) -> f32 {
    if observations.is_empty() {
        return 0.0;
    }
    observations
        .iter()
        .map(|o| {
            let y = if o.correct { 1.0 } else { 0.0 };
            (o.confidence - y).powi(2)
        })
        .sum::<f32>()
        / observations.len() as f32
}

/// Wilson score interval for a proportion. `z = 1.96` gives 95%.
pub fn wilson_interval(successes: usize, n: usize, z: f32) -> (f32, f32) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let n_f = n as f32;
    let p = successes as f32 / n_f;
    let denom = 1.0 + z * z / n_f;
    let center = p + z * z / (2.0 * n_f);
    let spread = z * ((p * (1.0 - p) / n_f + z * z / (4.0 * n_f * n_f)).sqrt());
    (
        ((center - spread) / denom).clamp(0.0, 1.0),
        ((center + spread) / denom).clamp(0.0, 1.0),
    )
}

/// Paired bootstrap over per-item differences, returning `(lo, hi, mean)`.
///
/// Takes per-item differences between two arms that answered the *same* items.
/// Resampling items (not arms) is what keeps the pairing: a two-sample test
/// would report a variance the experiment never had.
pub fn bootstrap_ci(differences: &[f32], iters: usize, seed: u64) -> (f32, f32, f32) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    if differences.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mean = differences.iter().sum::<f32>() / differences.len() as f32;
    let mut rng = StdRng::seed_from_u64(seed);
    let mut means = Vec::with_capacity(iters);
    for _ in 0..iters {
        let mut acc = 0.0f32;
        for _ in 0..differences.len() {
            let i: usize = rng.gen_range(0..differences.len());
            acc += differences[i];
        }
        means.push(acc / differences.len() as f32);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q = |p: f32| {
        let idx = ((means.len() - 1) as f32 * p).round() as usize;
        means[idx]
    };
    (q(0.025), q(0.975), mean)
}

/// Per-item difference between two arms, as `f32` for [`bootstrap_ci`].
pub fn paired_differences(a_correct: &[bool], b_correct: &[bool]) -> Vec<f32> {
    a_correct
        .iter()
        .zip(b_correct.iter())
        .map(|(a, b)| (*a as i32 - *b as i32) as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(pairs: &[(f32, bool)]) -> Vec<ConfidenceOutcome> {
        pairs
            .iter()
            .map(|(c, ok)| ConfidenceOutcome::new(*c, *ok))
            .collect()
    }

    #[test]
    fn an_empty_calibration_set_admits_nothing() {
        let g = ConformalGate::calibrate("h", &[], 0.1, 1);
        assert_eq!(g.tau, 1.0);
        assert!(!g.calibrated(), "no data is not a licence to guess");
        assert!(!g.accepts(1.0));
    }

    #[test]
    fn conformal_picks_the_largest_coverage_that_meets_the_target() {
        // Everything at >= 0.8 is right; below that the record is mixed. With
        // alpha = 0.1 the gate admits the confident block and stops before
        // accuracy falls under 0.9.
        let data = obs(&[
            (0.95, true),
            (0.9, true),
            (0.85, true),
            (0.8, true),
            (0.6, true),
            (0.6, false),
            (0.5, false),
            (0.4, false),
        ]);
        let t = conformal_quantile(&data, 0.1, 2);
        assert!((t - 0.8).abs() < 1e-6, "got tau={t}");
        let (n, acc) = accepted_accuracy(&data, t);
        assert_eq!(n, 4);
        assert!((acc - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_head_that_never_clears_the_target_gets_a_gate_that_admits_nothing() {
        let data = obs(&[(0.9, false), (0.8, false), (0.7, true), (0.6, false)]);
        assert_eq!(conformal_quantile(&data, 0.1, 2), 1.0);
        assert_eq!(accepted_accuracy(&data, 1.0).0, 0);
    }

    #[test]
    fn min_accepted_blocks_the_single_lucky_item() {
        // Only one item is right and it is the most confident one. Without a
        // floor, that one item would "prove" 100% accepted accuracy.
        let data = obs(&[(0.99, true), (0.5, false), (0.4, false), (0.3, false)]);
        assert_eq!(conformal_quantile(&data, 0.1, 1), 0.99);
        assert_eq!(
            conformal_quantile(&data, 0.1, 10),
            1.0,
            "a gate admitting fewer than min_accepted is not evidence"
        );
    }

    #[test]
    fn ece_is_zero_for_a_perfectly_calibrated_pair() {
        let data = obs(&[(0.5, true), (0.5, false)]);
        let (v, bins) = ece(&data, 10);
        assert_eq!(bins, 10);
        assert!(v < 1e-6, "50% confident, 50% right, ece={v}");
        let biased = obs(&[(0.9, true), (0.9, false)]);
        assert!(ece(&biased, 10).0 > 0.3);
    }

    #[test]
    fn brier_matches_the_closed_form() {
        assert!(brier(&obs(&[(1.0, true), (0.0, false)])) < 1e-6);
        assert!((brier(&obs(&[(0.5, true), (0.5, true)])) - 0.25).abs() < 1e-6);
        assert_eq!(brier(&[]), 0.0);
    }

    #[test]
    fn report_counts_abstentions_as_not_answered() {
        let gate = ConformalGate {
            id: "g".into(),
            alpha: 0.1,
            tau: 0.5,
            n: 100,
            method: "test".to_string(),
        };
        let answers = [(Some(0.9), true), (None, false), (Some(0.7), true)];
        let r = CalibrationReport::evaluate("h", &answers, &gate);
        assert_eq!(r.n, 3);
        assert!(r.answered < 0.67, "one abstention out of three");
        assert!(
            (r.accuracy - 1.0).abs() < 1e-6,
            "abstentions must not be averaged in as errors"
        );
        assert!((r.coverage - 2.0 / 3.0).abs() < 1e-6);
        assert!(r.met_target());
    }

    #[test]
    fn wilson_interval_brackets_the_estimate_and_tightens_with_n() {
        let (lo, hi) = wilson_interval(9, 10, 1.96);
        assert!(lo < 0.9 && hi > 0.9);
        let (lo2, hi2) = wilson_interval(900, 1000, 1.96);
        assert!(hi2 - lo2 < hi - lo, "more data, tighter interval");
        assert_eq!(wilson_interval(0, 0, 1.96), (0.0, 1.0));
    }

    #[test]
    fn paired_bootstrap_is_deterministic_and_centred_on_the_mean() {
        let diffs = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let a = bootstrap_ci(&diffs, 1000, 7);
        assert_eq!(a, bootstrap_ci(&diffs, 1000, 7), "same seed, same answer");
        assert!((a.2 - 0.5).abs() < 1e-6);
        assert!(a.0 < a.2 && a.2 < a.1);
        assert_eq!(bootstrap_ci(&[], 10, 1), (0.0, 0.0, 0.0));
    }

    #[test]
    fn paired_differences_keep_the_pairing() {
        assert_eq!(
            paired_differences(&[true, false, true], &[false, false, true]),
            vec![1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn recalibration_is_visible_as_a_material_change() {
        let a = ConformalGate::calibrate("h", &obs(&[(0.9, true), (0.8, true)]), 0.1, 1);
        let b = ConformalGate::calibrate("h", &obs(&[(0.5, true), (0.4, true)]), 0.1, 1);
        assert!(a.materially_differs_from(&b, 0.05));
        assert!(!a.materially_differs_from(&a.clone(), 0.05));
    }
}
