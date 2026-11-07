//! Ontology-driven feedback loop — when the user says "this is bad/wrong",
//! capture the negative signal through the semiotic grid, store it as a
//! vector-space negative example, and bias future classification away from it.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::embed::VectorEmbed;
use crate::models::{cosine_sim, Score};

/// A recorded quality failure — what was wrong, where in the grid it landed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFailure {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub feedback: String,
    pub feedback_embedding: Vec<f32>,
    /// The DOMAIN×MODE cell that was the top hit ("DOMAIN\x00MODE").
    pub top_cell: String,
    pub top_score: f32,
    pub correct_domain: Option<String>,
    pub correct_embedding: Option<Vec<f32>>,
    pub severity: f32,
    pub resolved: bool,
}

impl QualityFailure {
    pub fn new(feedback: &str, embedding: Vec<f32>, top_cell: &str, top_score: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            feedback: feedback.to_string(),
            feedback_embedding: embedding,
            top_cell: top_cell.to_string(),
            top_score,
            correct_domain: None,
            correct_embedding: None,
            severity: 0.5,
            resolved: false,
        }
    }
}

/// Tracks quality feedback and applies penalty/gain to semiotic cells.
/// Also tracks per-agent penalty/boost state for agent judging/tracking.
pub struct QualityTracker {
    pub failures: Vec<QualityFailure>,
    /// Per-cell negative weight (cell_key → penalty 0.0–1.0).
    pub cell_penalties: HashMap<String, f32>,
    /// Per-cell positive weight (corrections).
    pub cell_boosts: HashMap<String, f32>,
    /// Per-agent negative weight (agent_id → penalty 0.0–1.0).
    pub agent_penalties: HashMap<String, f32>,
    /// Per-agent positive weight (agent_id → boost 0.0–1.0).
    pub agent_boosts: HashMap<String, f32>,
    embedder: Box<dyn VectorEmbed>,
}

impl QualityTracker {
    pub fn new(embedder: Box<dyn VectorEmbed>) -> Self {
        Self {
            failures: Vec::new(),
            cell_penalties: HashMap::new(),
            cell_boosts: HashMap::new(),
            agent_penalties: HashMap::new(),
            agent_boosts: HashMap::new(),
            embedder,
        }
    }

    /// Report a quality failure — analyze feedback through the grid, record it,
    /// and adjust cell penalties.
    pub fn report_failure(
        &mut self,
        feedback: &str,
        cell_centroids: &HashMap<String, Vec<f32>>,
        correct_domain: Option<&str>,
    ) -> &QualityFailure {
        let embedding = self.embedder.embed(feedback);

        let mut scored: Vec<(&str, f32)> = cell_centroids
            .iter()
            .map(|(key, centroid)| (key.as_str(), cosine_sim(&embedding, centroid)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (top_cell, top_score) = scored
            .first()
            .map(|(k, s)| (*k, *s))
            .unwrap_or(("UNKNOWN", 0.0));

        let severity = if top_score < 0.3 {
            0.8
        } else if top_score < 0.6 {
            0.5
        } else {
            0.3
        };

        let mut failure = QualityFailure::new(feedback, embedding, top_cell, top_score);

        if let Some(correct) = correct_domain {
            failure.correct_domain = Some(correct.to_string());
            let correct_emb = self.embedder.embed(correct);
            for (key, centroid) in cell_centroids {
                if cosine_sim(&correct_emb, centroid) > 0.6 {
                    let boost = self.cell_boosts.entry(key.clone()).or_insert(0.0);
                    *boost = (*boost + 0.1).min(1.0);
                }
            }
        }

        failure.severity = severity;
        self.failures.push(failure);

        let penalty = self
            .cell_penalties
            .entry(top_cell.to_string())
            .or_insert(0.0);
        *penalty = (*penalty + severity * 0.2).min(1.0);

        // Same cell flagged 3+ times → lock it.
        let same_cell_count = self
            .failures
            .iter()
            .filter(|f| f.top_cell == top_cell)
            .count();
        if same_cell_count >= 3 {
            *penalty = 1.0;
        }

        self.failures.last().unwrap()
    }

    /// Penalize a cell that is already known, by a caller-chosen severity.
    ///
    /// The structured counterpart of [`Self::report_success`]. `report_failure`
    /// exists for free-text complaints: it re-embeds the text to *find* which
    /// cell was wrong, and lets three of them lock a cell outright. A pin
    /// disagreement needs neither — the editor already told us both the wrong
    /// cell and the right one — and must not be able to lock anything, because
    /// a pin is locally authoritative but only one sample globally.
    ///
    /// Same `severity * 0.2` scale as a reported failure, so the two kinds of
    /// evidence accumulate in comparable units.
    pub fn penalize_cell(&mut self, cell: &str, severity: f32) {
        let severity = severity.clamp(0.0, 1.0);
        let penalty = self.cell_penalties.entry(cell.to_string()).or_insert(0.0);
        *penalty = (*penalty + severity * 0.2).min(1.0);
    }

    /// Report positive feedback — user confirmed the output is good.
    pub fn report_success(&mut self, cell: &str) {
        let boost = self.cell_boosts.entry(cell.to_string()).or_insert(0.0);
        *boost = (*boost + 0.15).min(1.0);
        if let Some(p) = self.cell_penalties.get_mut(cell) {
            *p = (*p - 0.1).max(0.0);
        }
    }

    /// Report positive feedback for an agent — agent confirmed output is good.
    /// Increases agent boost and decreases agent penalty.
    pub fn report_agent_success(&mut self, agent_id: &str) {
        let agent_boost = self.agent_boosts.entry(agent_id.to_string()).or_insert(0.0);
        *agent_boost = (*agent_boost + 0.1).min(1.0);
        let agent_penalty = self.agent_penalties.get_mut(agent_id);
        if let Some(p) = agent_penalty {
            *p = (*p - 0.05).max(0.0);
        }
    }

    /// Report agent failure — agent produced bad output / error.
    /// Increases agent penalty and decreases agent boost.
    pub fn report_agent_failure(&mut self, agent_id: &str) {
        let agent_penalty = self
            .agent_penalties
            .entry(agent_id.to_string())
            .or_insert(0.0);
        *agent_penalty = (*agent_penalty + 0.1).min(1.0);
        let agent_boost = self.agent_boosts.get_mut(agent_id);
        if let Some(b) = agent_boost {
            *b = (*b - 0.05).max(0.0);
        }
    }

    /// How this cell currently stands with the tracker: `1.0` is neutral,
    /// below is demoted by accumulated penalties, above is promoted by boosts.
    ///
    /// This is the multiplier `adjust_score` applies to live classification, so
    /// any surface that wants to *show* a cell's condition must read it from
    /// here rather than recombining penalties and boosts on its own — otherwise
    /// the display can say "healthy" about a cell the engine is demoting.
    pub fn cell_standing(&self, cell: &str) -> f32 {
        let penalty = self.cell_penalties.get(cell).copied().unwrap_or(0.0);
        let boost = self.cell_boosts.get(cell).copied().unwrap_or(0.0);
        1.0 - penalty + boost
    }

    /// Adjust a raw score by learned penalties and boosts.
    pub fn adjust_score(&self, cell: &str, raw_score: f32) -> f32 {
        (raw_score * self.cell_standing(cell)).clamp(0.0, 1.0)
    }

    /// Human-readable summary (used by the CLI).
    pub fn summary(&self) -> String {
        let mut out = String::new();
        out.push_str("=== Quality Tracker ===");
        out.push('\n');
        out.push_str(&format!("Total failures: {}\n", self.failures.len()));
        out.push_str(&format!(
            "Cells with penalties: {}\n",
            self.cell_penalties.len()
        ));
        out.push_str(&format!("Cells with boosts: {}\n", self.cell_boosts.len()));
        out.push('\n');

        if self.failures.is_empty() {
            out.push_str("No failures recorded.\n");
            return out;
        }

        out.push_str("--- Recent Failures ---\n");
        for f in self.failures.iter().rev().take(5) {
            out.push_str(&format!(
                "  [{}] cell={} score={:.3} sev={:.1} feedback={}\n",
                &f.id[..8],
                f.top_cell,
                f.top_score,
                f.severity,
                &f.feedback[..f.feedback.len().min(60)],
            ));
        }
        out.push('\n');

        out.push_str("--- Cell Penalties ---\n");
        let mut sorted: Vec<_> = self.cell_penalties.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (cell, penalty) in sorted.iter().take(10) {
            out.push_str(&format!("  {}: penalty={:.2}\n", cell, penalty));
        }

        out
    }

    /// Persist failures to a JSON file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.failures)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load failures from a JSON file.
    pub fn load(path: &Path) -> anyhow::Result<Vec<QualityFailure>> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    /// Load from disk, rebuilding penalties from the recorded failures.
    pub fn load_or_new(path: &Path) -> Self {
        let mut t = QualityTracker::new(Box::new(crate::embed::RandomProjectionEmbedder::new(384)));
        let failures = if path.exists() {
            Self::load(path).unwrap_or_default()
        } else {
            Vec::new()
        };
        for f in &failures {
            let penalty = t.cell_penalties.entry(f.top_cell.clone()).or_insert(0.0);
            *penalty = (*penalty + f.severity * 0.2).min(1.0);
        }
        t.failures = failures;
        t
    }
}

/// Reduce what is known about a cell to one 0..1 value a heatmap can colour.
///
/// The grid used to colour by entry `count`, which says how *big* a cell is and
/// nothing about how it is *doing* — a cell the engine demotes on every single
/// query rendered identically to a healthy cell of the same size. These are the
/// three things actually known per cell:
///
/// - `count` — entries filed here. `0` means **unknown**, not bad: nothing has
///   ever landed here, so there is no evidence either way.
/// - `standing` — [`QualityTracker::cell_standing`], the live multiplier the
///   classifier applies: `1.0` neutral, toward `0.0` demoted by reported
///   failures, above `1.0` promoted by confirmations (not bounded above).
/// - `mean_asserted` — mean human verdict (`-1.0`..`1.0`) over the nodes that
///   landed here, or `None` where nobody has judged any of them. Distinct from
///   `standing`: standing is how well the *classifier* is doing at this cell,
///   `mean_asserted` is how the *work filed under it* turned out. A cell can
///   classify perfectly and hold nothing but failures.
///
/// The fourth input, `confidence`, decides *how much a cell's own evidence has
/// to accumulate before the colour believes it* — see [`ConditionLens`].
///
/// Returns `0.0` (worst) .. `1.0` (best), with `0.5` meaning "nothing known".
/// Neutral is the middle of the range rather than the bottom because absent
/// evidence is not bad news: an unjudged cell has earned no verdict either way.
pub fn cell_condition(
    count: u32,
    standing: f32,
    mean_asserted: Option<f32>,
    confidence: f32,
) -> f32 {
    // Nothing has ever landed here: unknown, not bad.
    if count == 0 {
        return NEUTRAL_CONDITION;
    }

    // `standing` is unbounded above 1.0, so squash rather than clamp — with
    // `s / (s + 1)` a neutral cell sits at 0.5, a doubly-boosted one at 0.75,
    // and a fully-penalised one near 0, all still distinguishable. Clamping
    // would flatten every boosted cell onto the same top colour.
    let s = standing.max(0.0);
    let standing_health = s / (s + 1.0);

    // Two different questions, averaged only when both have an answer:
    // standing is how well the classifier does *at* this cell, mean_asserted is
    // how the work filed *under* it turned out.
    let health = match mean_asserted {
        Some(v) => 0.5 * standing_health + 0.5 * (v.clamp(-1.0, 1.0) + 1.0) / 2.0,
        None => standing_health,
    };

    // Shrink toward neutral by how much evidence the cell carries. `k` is the
    // number of entries needed before the cell's own reading outweighs the
    // neutral prior, so `confidence` slides between the two readings of
    // `count`: at 0.0 it is mere size and every cell reacts at full strength;
    // at 1.0 it is a confidence weight and thin cells barely move.
    let n = count as f32;
    let k = confidence.clamp(0.0, 1.0) * CONDITION_CONFIDENCE_SPAN;
    let trust = n / (n + k);
    NEUTRAL_CONDITION + (health - NEUTRAL_CONDITION) * trust
}

/// What [`cell_condition`] returns when it knows nothing — the middle of the
/// ramp, so "unjudged" reads as neither good nor bad.
pub const NEUTRAL_CONDITION: f32 = 0.5;

/// Entries needed to fully trust a cell's own evidence at `confidence == 1.0`.
///
/// 16 is chosen so the slider's far end is visibly slow without being inert: a
/// 2-entry cell then shows about a ninth of its raw reading, a 16-entry cell
/// half, a 48-entry cell three quarters.
pub const CONDITION_CONFIDENCE_SPAN: f32 = 16.0;

/// How much a cell's own evidence must accumulate before the heatmap believes
/// it — the slider behind [`cell_condition`]'s `confidence` argument.
///
/// `count` can be read two ways and both are defensible, so this is a knob
/// rather than a decision baked into the formula:
///
/// - [`Responsive`](ConditionLens::Responsive) treats `count` as mere size.
///   Every cell reacts at full strength, so one bad verdict in a 40-entry cell
///   moves the colour as hard as one in a 2-entry cell. New problems surface
///   immediately; the grid is twitchy and a single stray click is visible.
/// - [`Trusting`](ConditionLens::Trusting) treats `count` as a confidence
///   weight. Thin cells stay near neutral until they accumulate evidence, so
///   the grid is slow to alarm and its colours are worth acting on.
///
/// The default sits between them: a stray click is damped, a repeated problem
/// still colours through.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ConditionLens {
    /// Count is size. Full reaction at any population.
    Responsive,
    /// Balanced default — a few entries are damped, a dozen are believed.
    #[default]
    Balanced,
    /// Count is confidence. Thin cells hold near neutral.
    Trusting,
    /// An explicit position on the slider, `0.0` (responsive) ..= `1.0`
    /// (trusting).
    Custom(f32),
}

impl ConditionLens {
    /// The `confidence` value to hand [`cell_condition`].
    pub fn confidence(self) -> f32 {
        match self {
            ConditionLens::Responsive => 0.0,
            ConditionLens::Balanced => 0.35,
            ConditionLens::Trusting => 1.0,
            ConditionLens::Custom(v) => v.clamp(0.0, 1.0),
        }
    }

    /// Parse a slider position or a named preset, for query strings and config.
    ///
    /// Accepts `responsive` / `balanced` / `trusting` or a bare number in
    /// `0.0..=1.0`. Anything else is `None` so a typo falls back to the default
    /// rather than silently pinning the grid to one extreme.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "responsive" => Some(ConditionLens::Responsive),
            "balanced" => Some(ConditionLens::Balanced),
            "trusting" => Some(ConditionLens::Trusting),
            other => other
                .parse::<f32>()
                .ok()
                .filter(|v| (0.0..=1.0).contains(v))
                .map(ConditionLens::Custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    fn make_tracker() -> QualityTracker {
        QualityTracker::new(Box::new(RandomProjectionEmbedder::new(64)))
    }

    #[test]
    fn failure_creates_record_and_penalty() {
        let mut t = make_tracker();
        let mut centroids = HashMap::new();
        centroids.insert("WORK\x00LIFT".to_string(), vec![0.5; 64]);
        let (top_cell, top_score) = {
            let f = t.report_failure("this classification is completely wrong", &centroids, None);
            (f.top_cell.clone(), f.top_score)
        };
        assert_eq!(t.failures.len(), 1);
        assert_eq!(top_cell, "WORK\x00LIFT");
        assert!(top_score.is_finite());
        assert!(*t.cell_penalties.get("WORK\x00LIFT").unwrap() > 0.0);
    }

    #[test]
    fn adjust_score_with_penalty_reduces() {
        let mut t = make_tracker();
        let mut centroids = HashMap::new();
        centroids.insert("WORK\x00LIFT".to_string(), vec![0.5; 64]);
        t.report_failure("wrong", &centroids, None);
        assert!(t.adjust_score("WORK\x00LIFT", 0.8) < 0.8);
    }

    #[test]
    fn adjust_score_with_boost_increases() {
        let mut t = make_tracker();
        t.cell_boosts.insert("WORK\x00LIFT".to_string(), 0.5);
        assert!(t.adjust_score("WORK\x00LIFT", 0.8) > 0.8);
    }

    #[test]
    fn penalize_cell_accumulates_but_never_locks() {
        let mut t = make_tracker();
        let cell = "FABRICATE\x00MAINTAIN";

        // One dissenting pin is a sample, not a verdict: it moves the penalty,
        // but only slightly.
        t.penalize_cell(cell, 0.25);
        let first = t.cell_penalties[cell];
        assert!(first > 0.0 && first < 0.1, "one dissent nudges: {first}");

        // Repetition is what makes it count.
        t.penalize_cell(cell, 0.25);
        assert!(t.cell_penalties[cell] > first, "dissent accumulates");

        // Unlike `report_failure`, no number of pins may lock a cell outright —
        // locking is reserved for typed complaints, which carry an explanation.
        for _ in 0..200 {
            t.penalize_cell(cell, 1.0);
        }
        assert!(t.cell_penalties[cell] <= 1.0, "penalty stays bounded");

        // Out-of-range severity is clamped rather than trusted.
        let mut t2 = make_tracker();
        t2.penalize_cell(cell, -5.0);
        assert_eq!(
            t2.cell_penalties[cell], 0.0,
            "negative severity cannot boost"
        );
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut t = make_tracker();
        let mut centroids = HashMap::new();
        centroids.insert("TEST\x00CELL".to_string(), vec![0.5; 64]);
        t.report_failure("test failure", &centroids, None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("failures.json");
        t.save(&path).unwrap();
        let loaded = QualityTracker::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].feedback, "test failure");
    }

    /// An empty cell has no evidence, so it must not read as "bad" — that would
    /// paint 40-odd uncovered intersections alarm-red and bury the real ones.
    #[test]
    fn an_unpopulated_cell_reads_as_unknown_not_bad() {
        for lens in [
            ConditionLens::Responsive,
            ConditionLens::Balanced,
            ConditionLens::Trusting,
        ] {
            let c = cell_condition(0, 0.1, Some(-1.0), lens.confidence());
            assert_eq!(
                c, NEUTRAL_CONDITION,
                "{lens:?} must call an empty cell unknown"
            );
        }
    }

    /// The point of the whole change: two cells of equal size, one being
    /// demoted on every live query, must not render alike.
    #[test]
    fn a_demoted_cell_reads_worse_than_a_healthy_cell_of_the_same_size() {
        let conf = ConditionLens::Responsive.confidence();
        let healthy = cell_condition(10, 1.0, None, conf);
        let demoted = cell_condition(10, 0.2, None, conf);
        assert!(
            demoted < healthy,
            "demoted {demoted} should read worse than healthy {healthy}"
        );
    }

    /// Boosts push standing above 1.0 without bound; squashing keeps them
    /// ordered instead of flattening every boosted cell onto one colour.
    #[test]
    fn boosted_standing_stays_distinguishable_above_neutral() {
        let conf = ConditionLens::Responsive.confidence();
        let neutral = cell_condition(10, 1.0, None, conf);
        let boosted = cell_condition(10, 2.0, None, conf);
        let very_boosted = cell_condition(10, 6.0, None, conf);
        assert!(
            neutral < boosted && boosted < very_boosted,
            "boost must stay ordered"
        );
        assert!(very_boosted <= 1.0, "condition must stay within the ramp");
    }

    /// The slider's whole purpose: the same thin, badly-doing cell reads
    /// alarming at one end and reserved at the other.
    #[test]
    fn the_confidence_slider_changes_how_loudly_a_thin_cell_speaks() {
        let responsive = cell_condition(2, 0.1, Some(-1.0), ConditionLens::Responsive.confidence());
        let balanced = cell_condition(2, 0.1, Some(-1.0), ConditionLens::Balanced.confidence());
        let trusting = cell_condition(2, 0.1, Some(-1.0), ConditionLens::Trusting.confidence());

        assert!(responsive < balanced, "responsive must react hardest");
        assert!(balanced < trusting, "trusting must hold nearest neutral");
        assert!(
            trusting > 0.4,
            "two bad entries must not alarm the trusting lens (got {trusting})"
        );
        assert!(
            responsive < 0.2,
            "the responsive lens must show the raw reading (got {responsive})"
        );
    }

    /// Under the trusting lens the *same* verdict ratio colours through once a
    /// cell is populated — evidence accumulates rather than being ignored.
    #[test]
    fn trusting_lens_believes_a_cell_once_it_has_enough_entries() {
        let conf = ConditionLens::Trusting.confidence();
        let thin = cell_condition(2, 0.1, Some(-1.0), conf);
        let thick = cell_condition(64, 0.1, Some(-1.0), conf);
        assert!(
            thick < thin,
            "more evidence must move the colour further from neutral"
        );
        assert!(
            thick < 0.2,
            "a well-evidenced bad cell must still read bad (got {thick})"
        );
    }

    /// Population alone is not health: the responsive lens must ignore size
    /// entirely, or it reintroduces the headcount colouring being replaced.
    #[test]
    fn responsive_lens_ignores_population_entirely() {
        let conf = ConditionLens::Responsive.confidence();
        let small = cell_condition(1, 0.4, Some(0.2), conf);
        let large = cell_condition(400, 0.4, Some(0.2), conf);
        assert!(
            (small - large).abs() < 1e-6,
            "size must not shift the responsive reading"
        );
    }

    #[test]
    fn lens_parses_presets_and_slider_positions_and_rejects_junk() {
        assert_eq!(
            ConditionLens::parse("trusting"),
            Some(ConditionLens::Trusting)
        );
        assert_eq!(
            ConditionLens::parse("  Balanced "),
            Some(ConditionLens::Balanced)
        );
        assert_eq!(
            ConditionLens::parse("0.25"),
            Some(ConditionLens::Custom(0.25))
        );
        assert_eq!(ConditionLens::parse("0"), Some(ConditionLens::Custom(0.0)));
        // Out of range and nonsense both fall back to the default rather than
        // silently pinning the grid to an extreme.
        assert_eq!(ConditionLens::parse("1.5"), None);
        assert_eq!(ConditionLens::parse("-1"), None);
        assert_eq!(ConditionLens::parse("banana"), None);
    }

    /// Whatever the inputs, the result has to be a colourable 0..1.
    #[test]
    fn condition_stays_within_the_ramp_for_extreme_inputs() {
        for count in [0u32, 1, 7, 1_000] {
            for standing in [0.0f32, 0.5, 1.0, 25.0] {
                for verdict in [
                    None,
                    Some(-1.0),
                    Some(0.0),
                    Some(1.0),
                    Some(-9.0),
                    Some(9.0),
                ] {
                    for conf in [0.0f32, 0.35, 1.0] {
                        let c = cell_condition(count, standing, verdict, conf);
                        assert!(
                            (0.0..=1.0).contains(&c),
                            "condition {c} out of range for \
                             count={count} standing={standing} verdict={verdict:?} conf={conf}"
                        );
                    }
                }
            }
        }
    }
}

// ── Context-dependent fitness ───────────────────────────────────────
//
// Global penalties destroy an interpretation everywhere. Physis must learn
// that an interpretation can work in context A and fail in context B.
//
// Contexts include: domain, machine, process, time, environment, operator,
// material, state, task.

/// Context tags for a fitness record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FitnessContext {
    pub domain: Option<String>,
    pub machine: Option<String>,
    pub process: Option<String>,
    pub environment: Option<String>,
    pub operator: Option<String>,
    pub material: Option<String>,
    pub state: Option<String>,
    pub task: Option<String>,
}

impl FitnessContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compact key for maps: sorted `key=value` pairs joined by `;`.
    pub fn key(&self) -> String {
        let mut parts = Vec::new();
        if let Some(v) = &self.domain {
            parts.push(format!("domain={v}"));
        }
        if let Some(v) = &self.machine {
            parts.push(format!("machine={v}"));
        }
        if let Some(v) = &self.process {
            parts.push(format!("process={v}"));
        }
        if let Some(v) = &self.environment {
            parts.push(format!("env={v}"));
        }
        if let Some(v) = &self.operator {
            parts.push(format!("operator={v}"));
        }
        if let Some(v) = &self.material {
            parts.push(format!("material={v}"));
        }
        if let Some(v) = &self.state {
            parts.push(format!("state={v}"));
        }
        if let Some(v) = &self.task {
            parts.push(format!("task={v}"));
        }
        parts.sort();
        parts.join(";")
    }
}

/// A fitness record scoped to a context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessRecord {
    pub interpretation: String,
    pub context: FitnessContext,
    pub successes: usize,
    pub failures: usize,
    pub fitness: Score,
}

impl FitnessRecord {
    pub fn new(interpretation: impl Into<String>, context: FitnessContext) -> Self {
        let mut rec = Self {
            interpretation: interpretation.into(),
            context,
            successes: 0,
            failures: 0,
            fitness: 0.5,
        };
        rec.recompute();
        rec
    }

    pub fn record_success(&mut self) {
        self.successes += 1;
        self.recompute();
    }

    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.recompute();
    }

    fn recompute(&mut self) {
        let total = self.successes + self.failures;
        if total == 0 {
            self.fitness = 0.5;
            return;
        }
        self.fitness = self.successes as Score / total as Score;
    }
}

/// Context-dependent quality tracker.
///
/// Extends `QualityTracker` with per-context fitness. Instead of:
///
///   fitness(interpretation) = bad everywhere
///
/// we track:
///
///   fitness(interpretation, context) = good in A, bad in B
///
/// This prevents simplistic global rules from destroying valid interpretations.
pub struct ContextualQualityTracker {
    pub global_failures: Vec<QualityFailure>,
    pub global_penalties: HashMap<String, f32>,
    pub global_boosts: HashMap<String, f32>,
    pub fitness_records: HashMap<String, FitnessRecord>,
    embedder: Box<dyn VectorEmbed>,
}

impl ContextualQualityTracker {
    pub fn new(embedder: Box<dyn VectorEmbed>) -> Self {
        Self {
            global_failures: Vec::new(),
            global_penalties: HashMap::new(),
            global_boosts: HashMap::new(),
            fitness_records: HashMap::new(),
            embedder,
        }
    }

    /// Report a failure with context.
    pub fn report_failure_context(
        &mut self,
        feedback: &str,
        cell_centroids: &HashMap<String, Vec<f32>>,
        correct_domain: Option<&str>,
        context: FitnessContext,
    ) -> &QualityFailure {
        let embedding = self.embedder.embed(feedback);

        let mut scored: Vec<(&str, f32)> = cell_centroids
            .iter()
            .map(|(key, centroid)| (key.as_str(), cosine_sim(&embedding, centroid)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (top_cell, top_score) = scored
            .first()
            .map(|(k, s)| (*k, *s))
            .unwrap_or(("UNKNOWN", 0.0));
        let severity = if top_score < 0.3 {
            0.8
        } else if top_score < 0.6 {
            0.5
        } else {
            0.3
        };

        let mut failure = QualityFailure::new(feedback, embedding, top_cell, top_score);
        failure.severity = severity;

        if let Some(correct) = correct_domain {
            failure.correct_domain = Some(correct.to_string());
            let correct_emb = self.embedder.embed(correct);
            for (key, centroid) in cell_centroids {
                if cosine_sim(&correct_emb, centroid) > 0.6 {
                    let boost = self.global_boosts.entry(key.clone()).or_insert(0.0);
                    *boost = (*boost + 0.1).min(1.0);
                }
            }
        }

        self.global_failures.push(failure);

        let penalty = self
            .global_penalties
            .entry(top_cell.to_string())
            .or_insert(0.0);
        *penalty = (*penalty + severity * 0.2).min(1.0);

        // Context-dependent fitness update
        let ctx_key = format!("{top_cell}:{}", context.key());
        let rec = self
            .fitness_records
            .entry(ctx_key)
            .or_insert_with(|| FitnessRecord::new(top_cell.to_string(), context));
        rec.record_failure();

        self.global_failures.last().unwrap()
    }

    /// Report a failure for an explicit interpretation/cell with context.
    pub fn report_failure_for_cell_context(
        &mut self,
        cell: &str,
        _feedback: &str,
        context: FitnessContext,
    ) {
        let penalty = self.global_penalties.entry(cell.to_string()).or_insert(0.0);
        *penalty = (*penalty + 0.15).min(1.0);
        if let Some(b) = self.global_boosts.get_mut(cell) {
            *b = (*b - 0.1).max(0.0);
        }

        let ctx_key = format!("{cell}:{}", context.key());
        let rec = self
            .fitness_records
            .entry(ctx_key)
            .or_insert_with(|| FitnessRecord::new(cell.to_string(), context));
        rec.record_failure();
    }

    /// Report a success with context.
    pub fn report_success_context(&mut self, cell: &str, context: FitnessContext) {
        let boost = self.global_boosts.entry(cell.to_string()).or_insert(0.0);
        *boost = (*boost + 0.15).min(1.0);
        if let Some(p) = self.global_penalties.get_mut(cell) {
            *p = (*p - 0.1).max(0.0);
        }

        let ctx_key = format!("{cell}:{}", context.key());
        let rec = self
            .fitness_records
            .entry(ctx_key)
            .or_insert_with(|| FitnessRecord::new(cell.to_string(), context));
        rec.record_success();
    }

    /// Get fitness for a specific interpretation and context. Returns None if no data.
    pub fn fitness_for(&self, interpretation: &str, context: &FitnessContext) -> Option<Score> {
        let key = format!("{interpretation}:{}", context.key());
        self.fitness_records.get(&key).map(|r| r.fitness)
    }

    /// Get fitness for an interpretation across all contexts.
    pub fn fitness_for_interpretation(&self, interpretation: &str) -> Vec<&FitnessRecord> {
        self.fitness_records
            .values()
            .filter(|r| r.interpretation == interpretation)
            .collect()
    }

    /// Adjust a raw score, blending global and contextual fitness.
    pub fn adjust_score_contextual(
        &self,
        cell: &str,
        raw_score: Score,
        context: &FitnessContext,
    ) -> Score {
        let global_standing = {
            let penalty = self.global_penalties.get(cell).copied().unwrap_or(0.0);
            let boost = self.global_boosts.get(cell).copied().unwrap_or(0.0);
            1.0 - penalty + boost
        };

        let contextual = self.fitness_for(cell, context).unwrap_or(0.5);
        let blended = 0.7 * global_standing + 0.3 * contextual;
        (raw_score * blended).clamp(0.0, 1.0)
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        out.push_str("=== Contextual Quality Tracker ===\n");
        out.push_str(&format!(
            "Global failures: {}\n",
            self.global_failures.len()
        ));
        out.push_str(&format!(
            "Context records: {}\n",
            self.fitness_records.len()
        ));
        out.push('\n');

        for (key, rec) in self.fitness_records.iter().take(10) {
            out.push_str(&format!(
                "  {} : fitness={:.2} ({} success / {} failure)\n",
                key, rec.fitness, rec.successes, rec.failures
            ));
        }
        out
    }
}
