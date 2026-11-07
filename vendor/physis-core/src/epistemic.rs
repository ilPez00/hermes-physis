//! Epistemic Audit Trail — replayable timeline of beliefs, observations, and revisions.
//!
//! Answers:
//!   - "What did Physis believe at time T?"
//!   - "What evidence caused the belief to change?"
//!   - "Which hypotheses evolved, were contradicted, or were superseded?"

use serde::{Deserialize, Serialize};

use crate::hypothesis::HypothesisStatus;
use crate::models::Score;
use crate::temporal::TemporalValidity;

/// Types of epistemic events recorded on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EpistemicEventType {
    /// Initial assertion or baseline belief.
    BeliefFormed,
    /// New empirical observation ingested.
    ObservationIngested,
    /// Conflict/contradiction detected between two claims.
    ContradictionDetected,
    /// New candidate hypothesis generated.
    HypothesisGenerated,
    /// Prediction formulated by a hypothesis.
    PredictionFormulated,
    /// Outcome observed for a prior prediction.
    OutcomeObserved,
    /// Fitness score adjusted based on evidence or outcomes.
    FitnessShifted,
    /// Status transition of a hypothesis (e.g. Supported -> Contradicted).
    StatusTransition,
    /// Discovery of an ontology gap requiring conceptual extension.
    OntologyGapDiscovered,
    /// Promotion of a candidate ontology domain to active status.
    OntologyDomainPromoted,
}

/// A single step in the epistemic audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpistemicEvent {
    /// Unique event ID.
    pub id: String,
    /// When this event occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Type of epistemic change.
    pub event_type: EpistemicEventType,
    /// ID of the subject entity (hypothesis, node, observation, contradiction).
    pub subject_id: String,
    /// Human-readable explanation of what occurred.
    pub description: String,
    /// Prior state or status (if applicable).
    #[serde(default)]
    pub prior_state: Option<String>,
    /// Posterior state or status (if applicable).
    #[serde(default)]
    pub posterior_state: Option<String>,
    /// Source or trigger of this change (file, operator, sensor, feedback ID).
    #[serde(default)]
    pub source: Option<String>,
    /// Confidence or fitness value at this instant.
    #[serde(default)]
    pub metric_value: Option<Score>,
    /// G3: when the fact was *asserted by its source* (episode reference
    /// time), distinct from `timestamp` (arrival / transaction time). None
    /// on pre-G3 events — replay then treats arrival as assertion.
    #[serde(default)]
    pub asserted_at: Option<chrono::DateTime<chrono::Utc>>,
    /// G4: id of the intake episode recorded by this event — the per-link
    /// intake id the trail carries so an evidence citation resolves to the
    /// concrete ingest that produced it.
    #[serde(default)]
    pub intake_id: Option<String>,
}

impl EpistemicEvent {
    pub fn new(
        event_type: EpistemicEventType,
        subject_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_type,
            subject_id: subject_id.into(),
            description: description.into(),
            prior_state: None,
            posterior_state: None,
            source: None,
            metric_value: None,
            asserted_at: None,
            intake_id: None,
        }
    }

    /// G3: stamp the episode reference time (when the source asserted the
    /// fact), as opposed to `timestamp` (when the engine received it).
    pub fn with_asserted_at(mut self, asserted_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.asserted_at = Some(asserted_at);
        self
    }

    /// G3: the replay clock for this event — assertion time when known,
    /// arrival otherwise (pre-G3 events). Replay is ordered by this key, so
    /// an out-of-order intake replays to the same state as an in-order one.
    pub fn assertion_time(&self) -> chrono::DateTime<chrono::Utc> {
        self.asserted_at.unwrap_or(self.timestamp)
    }

    pub fn with_transition(
        mut self,
        prior: impl Into<String>,
        posterior: impl Into<String>,
    ) -> Self {
        self.prior_state = Some(prior.into());
        self.posterior_state = Some(posterior.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_metric(mut self, metric: Score) -> Self {
        self.metric_value = Some(metric);
        self
    }

    /// G4: stamp which intake episode this event recorded.
    pub fn with_intake_id(mut self, intake_id: impl Into<String>) -> Self {
        self.intake_id = Some(intake_id.into());
        self
    }
}

/// The full epistemic audit trail of the engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpistemicAuditTrail {
    pub events: Vec<EpistemicEvent>,
    /// G3: per-stream intake watermarks (see [`HighWaterMark`]).
    #[serde(default)]
    pub watermarks: Vec<HighWaterMark>,
}

/// G3: per-stream intake watermark — the two clocks of the trail.
///
/// `last_arrival` is the transaction clock (wall time of the latest intake
/// for this stream); `last_episode_valid_at` is the episode reference clock
/// (the newest assertion time seen). A stream that delivers an episode
/// older than the mark is *late* — the receipt says so, and replay is
/// ordered by assertion time, so the late episode still lands where it
/// belongs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighWaterMark {
    pub stream_id: String,
    pub last_arrival: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub last_episode_valid_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// G3: what [`EpistemicAuditTrail::note_intake`] reports for one intake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntakeReceipt {
    pub stream_id: String,
    /// True when this episode's assertion time predates the mark's.
    pub late: bool,
    /// The mark after advancing.
    pub mark: HighWaterMark,
}

impl EpistemicAuditTrail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, event: EpistemicEvent) {
        self.events.push(event);
    }

    /// G3: note an intake on `stream_id`, advancing the per-stream
    /// watermark. Returns a receipt that flags a *late* episode (assertion
    /// time older than the mark's). Late arrivals are expected, not errors —
    /// replay is ordered by assertion time, so they still land where they
    /// belong. Neither clock moves backwards.
    pub fn note_intake(
        &mut self,
        stream_id: impl Into<String>,
        arrival: chrono::DateTime<chrono::Utc>,
        episode_valid_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> IntakeReceipt {
        let stream_id = stream_id.into();
        let mut late = false;
        let mark = match self
            .watermarks
            .iter_mut()
            .find(|w| w.stream_id == stream_id)
        {
            Some(w) => {
                if let (Some(prev), Some(v)) = (w.last_episode_valid_at, episode_valid_at) {
                    if v < prev {
                        late = true;
                    }
                }
                w.last_arrival = w.last_arrival.max(arrival);
                if w.last_episode_valid_at
                    .is_none_or(|prev| episode_valid_at.is_some_and(|v| v > prev))
                {
                    w.last_episode_valid_at = episode_valid_at;
                }
                w.clone()
            }
            None => {
                let w = HighWaterMark {
                    stream_id: stream_id.clone(),
                    last_arrival: arrival,
                    last_episode_valid_at: episode_valid_at,
                };
                self.watermarks.push(w.clone());
                w
            }
        };
        IntakeReceipt {
            stream_id,
            late,
            mark,
        }
    }

    /// Retrieve all events relevant to a specific entity ID in chronological order.
    pub fn history_for(&self, subject_id: &str) -> Vec<&EpistemicEvent> {
        self.events
            .iter()
            .filter(|e| e.subject_id == subject_id)
            .collect()
    }

    pub fn reconstruct_status_at(
        &self,
        subject_id: &str,
        when: chrono::DateTime<chrono::Utc>,
    ) -> Option<HypothesisStatus> {
        // G3: replay in *assertion* order, not arrival order — late
        // (backfilled) evidence lands where it belongs, so an out-of-order
        // intake replays to the same state as an in-order one. Ties keep
        // arrival order (stable sort). Pre-G3 events (no `asserted_at`)
        // replay by arrival, unchanged.
        let mut relevant: Vec<&EpistemicEvent> = self
            .events
            .iter()
            .filter(|ev| ev.subject_id == subject_id && ev.assertion_time() <= when)
            .collect();
        relevant.sort_by_key(|ev| ev.assertion_time());

        let mut last_status: Option<HypothesisStatus> = None;
        for ev in relevant {
            if ev.event_type == EpistemicEventType::HypothesisGenerated {
                last_status = Some(HypothesisStatus::Candidate);
            }
            if let Some(ref post) = ev.posterior_state {
                match post.to_lowercase().as_str() {
                    "candidate" => last_status = Some(HypothesisStatus::Candidate),
                    "supported" => last_status = Some(HypothesisStatus::Supported),
                    "contradicted" => last_status = Some(HypothesisStatus::Contradicted),
                    "confirmed" => last_status = Some(HypothesisStatus::Confirmed),
                    "inert" => last_status = Some(HypothesisStatus::Inert),
                    "failed" => last_status = Some(HypothesisStatus::Failed),
                    "superseded" => last_status = Some(HypothesisStatus::Superseded),
                    "isolated" => last_status = Some(HypothesisStatus::Isolated),
                    "certified" => last_status = Some(HypothesisStatus::Certified),
                    _ => {}
                }
            }
        }
        last_status
    }

    /// G6: a point-in-time query — the audited status of `subject_id` at
    /// `when`, gated by `validity` (which carries the system-invalidation
    /// `expired_at` leg). The audit trail is the authority for *what was
    /// believed*; the validity window is the authority for *whether that
    /// belief was visible at T*. They are allowed to disagree exactly on
    /// purpose: history keeps superseded and invalidated states, the
    /// validity surface does not.
    ///
    /// `None` therefore means one of two things: the subject had no
    /// audited status at `when`, or the status existed but the validity
    /// window (or the system invalidation) closed it. To tell them apart,
    /// call [`EpistemicAuditTrail::reconstruct_status_at`] — the trail
    /// still shows the state that the surface no longer exposes.
    pub fn point_in_time_status_at(
        &self,
        subject_id: &str,
        when: chrono::DateTime<chrono::Utc>,
        validity: &TemporalValidity,
    ) -> Option<HypothesisStatus> {
        if !validity.is_valid_at(when) {
            return None;
        }
        self.reconstruct_status_at(subject_id, when)
    }

    /// Format chronological summary for audit reporting.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        for (i, ev) in self.events.iter().enumerate() {
            let t = ev.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
            lines.push(format!(
                "t{i} [{t}] [{:?}]: {} (subject: {})",
                ev.event_type, ev.description, ev.subject_id
            ));
        }
        lines.join("\n")
    }
}
