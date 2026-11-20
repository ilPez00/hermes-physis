//! Temporal dream — retrospective proposals over the time machine.
//!
//! The time machine (G3) replays the past; the dream pass asks what the
//! past would change *now*. Retired branches were retained, never deleted
//! (G7 invalidate-don't-delete), so they can be **re-proposed**:
//!
//! - a superseded/failed/inert hypothesis whose pattern re-presents itself
//!   in later observations → [`ProposalKind::ReactivateHypothesis`];
//! - a severed DependsOn connection whose target has since re-confirmed →
//!   [`ProposalKind::RestoreConnection`];
//! - a connection whose target keeps contradicting →
//!   [`ProposalKind::RetireConnection`].
//!
//! Dreams are **proposals only**. A dream takes the trail by shared
//! reference and cannot mutate it; every [`RetrospectiveProposal`] cites
//! the historical event/mutation ids that motivated it; nothing applies
//! without an explicit commit by the caller. Many things already happened,
//! and may re-present themselves — the dream is how the past speaks.
//!
//! Recurrence analysis is order-aware (runs/temporal separation vs
//! interleaved) so sustained re-presentation differs from coincidental noise;
//! stability analysis over the trail surfaces non-monotonic flip-flops and
//! overshoot/relax patterns as inspectable flags.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::delta_engine::{MutationOp, OntologyMutation};
use crate::epistemic::{EpistemicAuditTrail, EpistemicEvent, EpistemicEventType};
use crate::hypothesis::HypothesisStatus;

/// Contradictions a connection's target may accumulate before the dream
/// proposes retiring the connection.
pub const RETIRE_AFTER_CONTRADICTIONS: usize = 2;

/// What kind of edit the dream proposes for an old branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalKind {
    /// Re-activate a retained hypothesis whose pattern re-presented itself.
    ReactivateHypothesis,
    /// Re-establish a DependsOn connection that was severed, whose target
    /// has since re-confirmed.
    RestoreConnection { to: String },
    /// Sever a connection whose target keeps contradicting.
    RetireConnection { to: String },
}

/// Order-aware recurrence evidence for a re-presentation (additive, inspectable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurrenceEvidence {
    pub observations: usize,
    /// Distinct active bins in the lookback window.
    pub active_bins: usize,
    pub total_bins: usize,
    /// Runs-test z over the binary active/inactive bin sequence (negative =
    /// temporally clumped/sustained; near 0/positive = interleaved/sporadic).
    pub runs_z: f64,
    /// Recency: seconds between newest re-presentation and `now`.
    pub recency_secs: i64,
    /// A coarse qualitative label for callers that do not read runs_z.
    pub character: String,
}

/// One retrospective proposal. Proposals cite history; they never write it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrospectiveProposal {
    /// The hypothesis (or node) the proposed edit bears on.
    pub subject_id: String,
    pub kind: ProposalKind,
    /// The replay point the proposal is anchored to: the retirement or
    /// sever time of the branch it would edit.
    pub anchor_time: chrono::DateTime<chrono::Utc>,
    pub rationale: String,
    /// Ids of the historical events / mutations that motivated this.
    pub citing: Vec<String>,
    /// Recurrence strength/evidence for reactivation proposals (additive).
    #[serde(default)]
    pub recurrence: Option<RecurrenceEvidence>,
}

/// Inspectable instability/oscillation flag over assertion-ordered history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StabilityFlag {
    pub subject_id: String,
    /// e.g. "flip_flop", "oscillation", "overshoot_relax"
    pub kind: String,
    /// Number of status transitions observed.
    pub transitions: usize,
    /// Direction changes in the status trajectory (flip-flops).
    pub flips: usize,
    /// Short human-readable summary.
    pub summary: String,
    /// Event ids participating in the flagged behaviour.
    pub citing: Vec<String>,
}

fn parse_status(s: &str) -> Option<HypothesisStatus> {
    match s.to_lowercase().as_str() {
        "candidate" => Some(HypothesisStatus::Candidate),
        "supported" => Some(HypothesisStatus::Supported),
        "contradicted" => Some(HypothesisStatus::Contradicted),
        "confirmed" => Some(HypothesisStatus::Confirmed),
        "inert" => Some(HypothesisStatus::Inert),
        "failed" => Some(HypothesisStatus::Failed),
        "superseded" => Some(HypothesisStatus::Superseded),
        "isolated" => Some(HypothesisStatus::Isolated),
        "certified" => Some(HypothesisStatus::Certified),
        _ => None,
    }
}

fn status_rank(st: HypothesisStatus) -> i32 {
    // Higher = healthier/more committed, for detecting non-monotonic movement.
    match st {
        HypothesisStatus::Failed => 0,
        HypothesisStatus::Contradicted => 1,
        HypothesisStatus::Isolated => 2,
        HypothesisStatus::Inert => 3,
        HypothesisStatus::Superseded => 4,
        HypothesisStatus::Candidate => 5,
        HypothesisStatus::Supported => 6,
        HypothesisStatus::Confirmed => 7,
        HypothesisStatus::Certified => 8,
    }
}

fn recurrence_evidence(
    events: &[&EpistemicEvent],
    now: chrono::DateTime<chrono::Utc>,
    window_start: chrono::DateTime<chrono::Utc>,
) -> Option<RecurrenceEvidence> {
    if events.is_empty() {
        return None;
    }
    let total_bins = 8usize;
    let window_secs = (now - window_start).num_seconds().max(1);
    let mut bins = vec![false; total_bins];
    for ev in events {
        let t = ev.assertion_time();
        let offset = (t - window_start).num_seconds().clamp(0, window_secs - 1);
        let idx = ((offset as i64 * total_bins as i64) / window_secs)
            .clamp(0, (total_bins - 1) as i64) as usize;
        bins[idx] = true;
    }
    let active_bins = bins.iter().filter(|b| **b).count();
    let z = crate::becoming::runs_z(&bins);
    let newest = events
        .iter()
        .map(|e| e.assertion_time())
        .max()
        .unwrap_or(now);
    let recency_secs = (now - newest).num_seconds().max(0);
    let character = if active_bins <= 1 {
        "isolated".to_string()
    } else if z <= -1.0 {
        "sustained_clumped".to_string()
    } else if z >= 1.0 {
        "interleaved_sporadic".to_string()
    } else {
        "mixed".to_string()
    };
    Some(RecurrenceEvidence {
        observations: events.len(),
        active_bins,
        total_bins,
        runs_z: z,
        recency_secs,
        character,
    })
}

/// Replay the trail in assertion order (G3) and propose edits to retired
/// branches. Deterministic: the "now" anchor is the newest assertion time
/// in the trail, never the wall clock; proposals come back sorted by
/// subject and anchor.
///
/// `mutations` is the delta engine's append-only mutation log — the record
/// of historical `RelationSevered` / `RelationCreated` operations whose
/// wisdom the dream may now second-guess.
pub fn dream_over_history(
    trail: &EpistemicAuditTrail,
    mutations: &[OntologyMutation],
    lookback: chrono::Duration,
) -> Vec<RetrospectiveProposal> {
    let mut proposals = Vec::new();

    // Assertion-ordered history per subject (the G3 replay discipline).
    let mut by_subject: HashMap<&str, Vec<&crate::epistemic::EpistemicEvent>> = HashMap::new();
    for ev in &trail.events {
        by_subject
            .entry(ev.subject_id.as_str())
            .or_default()
            .push(ev);
    }
    for events in by_subject.values_mut() {
        events.sort_by_key(|e| e.assertion_time());
    }

    // Deterministic "now": the newest assertion in the trail.
    let Some(now) = trail.events.iter().map(|e| e.assertion_time()).max() else {
        return proposals;
    };

    // ── 1. Reactivate: a retained branch whose pattern re-presented ────
    for (subject, events) in &by_subject {
        let Some(retired_ev) = events.iter().rev().find(|e| {
            matches!(
                e.posterior_state.as_deref(),
                Some("superseded") | Some("failed") | Some("inert")
            )
        }) else {
            continue;
        };

        // Re-presentation: later observations naming the same subject,
        // within the lookback window.
        let re_presented: Vec<&crate::epistemic::EpistemicEvent> = events
            .iter()
            .copied()
            .filter(|e| {
                e.event_type == EpistemicEventType::ObservationIngested
                    && e.assertion_time() > retired_ev.assertion_time()
                    && now - e.assertion_time() <= lookback
            })
            .collect();
        if re_presented.is_empty() {
            continue;
        }

        let window_start = if retired_ev.assertion_time() > now - lookback {
            retired_ev.assertion_time()
        } else {
            now - lookback
        };
        let rec = recurrence_evidence(&re_presented, now, window_start);
        let (count, character, z, recency) = rec
            .as_ref()
            .map(|r| {
                (
                    r.observations,
                    r.character.as_str(),
                    r.runs_z,
                    r.recency_secs,
                )
            })
            .unwrap_or((re_presented.len(), "unknown", 0.0, 0));

        proposals.push(RetrospectiveProposal {
            subject_id: subject.to_string(),
            kind: ProposalKind::ReactivateHypothesis,
            anchor_time: retired_ev.assertion_time(),
            rationale: format!(
                "branch retired to {} at {}, but {} later observation(s) \
                 re-present the pattern within the lookback (recurrence: {}, \
                 runs_z={:.2}, recency={}s); the branch was retained (G7), \
                 never deleted",
                retired_ev.posterior_state.as_deref().unwrap_or("?"),
                retired_ev.assertion_time(),
                count,
                character,
                z,
                recency,
            ),
            citing: re_presented.iter().map(|e| e.id.clone()).collect(),
            recurrence: rec,
        });
    }

    // ── 2/3. Connections: restore what re-confirmed, retire what rots ──
    for m in mutations {
        match &m.operation {
            MutationOp::RelationSevered { target_id } => {
                // Did the target re-confirm after the sever?
                let re_confirmed = trail.events.iter().find(|e| {
                    e.subject_id == *target_id
                        && e.assertion_time() >= m.timestamp
                        && matches!(
                            e.posterior_state.as_deref(),
                            Some("supported") | Some("confirmed")
                        )
                });
                if let Some(ev) = re_confirmed {
                    proposals.push(RetrospectiveProposal {
                        subject_id: m.target_node_id.clone(),
                        kind: ProposalKind::RestoreConnection {
                            to: target_id.clone(),
                        },
                        anchor_time: m.timestamp,
                        rationale: format!(
                            "connection severed at {}, but target '{}' \
                             re-confirmed at {} (event {}); the edge may \
                             belong back",
                            m.timestamp,
                            target_id,
                            ev.assertion_time(),
                            ev.id,
                        ),
                        citing: vec![m.id.clone(), ev.id.clone()],
                        recurrence: None,
                    });
                }
            }
            MutationOp::RelationCreated { target_id, .. } => {
                let contradictions = trail
                    .events
                    .iter()
                    .filter(|e| {
                        (e.subject_id == *target_id || e.subject_id == m.target_node_id)
                            && e.assertion_time() >= m.timestamp
                            && e.event_type == EpistemicEventType::ContradictionDetected
                    })
                    .count();
                if contradictions >= RETIRE_AFTER_CONTRADICTIONS {
                    proposals.push(RetrospectiveProposal {
                        subject_id: m.target_node_id.clone(),
                        kind: ProposalKind::RetireConnection {
                            to: target_id.clone(),
                        },
                        anchor_time: m.timestamp,
                        rationale: format!(
                            "target '{}' accumulated {} contradiction(s) \
                             since the connection was created at {}",
                            target_id, contradictions, m.timestamp,
                        ),
                        citing: vec![m.id.clone()],
                        recurrence: None,
                    });
                }
            }
            _ => {}
        }
    }

    // Deterministic order regardless of HashMap iteration.
    proposals.sort_by(|a, b| {
        (&a.subject_id, a.anchor_time, &a.rationale).cmp(&(
            &b.subject_id,
            b.anchor_time,
            &b.rationale,
        ))
    });
    proposals
}

/// Analyse assertion-ordered history for non-monotonic status behaviour:
/// flip-flops, oscillations, and overshoot-then-relax patterns. Read-only and
/// deterministic; emits inspectable flags (no mutation, no auto-commit).
pub fn analyse_stability(trail: &EpistemicAuditTrail) -> Vec<StabilityFlag> {
    let mut by_subject: HashMap<&str, Vec<&EpistemicEvent>> = HashMap::new();
    for ev in &trail.events {
        by_subject
            .entry(ev.subject_id.as_str())
            .or_default()
            .push(ev);
    }
    let mut flags = Vec::new();
    for (subject, events) in &mut by_subject {
        events.sort_by_key(|e| e.assertion_time());
        // Collect status-bearing transitions in order.
        let mut seq: Vec<(HypothesisStatus, &EpistemicEvent)> = Vec::new();
        for ev in events.iter() {
            if ev.event_type == EpistemicEventType::HypothesisGenerated {
                seq.push((HypothesisStatus::Candidate, *ev));
                continue;
            }
            if let Some(ref post) = ev.posterior_state {
                if let Some(st) = parse_status(post) {
                    if seq.last().map(|(s, _)| *s) != Some(st) {
                        seq.push((st, *ev));
                    }
                }
            }
        }
        if seq.len() < 3 {
            continue;
        }
        let transitions = seq.len().saturating_sub(1);
        let mut flips = 0usize;
        let mut prev_dir: i32 = 0;
        for w in seq.windows(2) {
            let d = status_rank(w[1].0) - status_rank(w[0].0);
            if d != 0 {
                let dir = d.signum();
                if prev_dir != 0 && dir != prev_dir {
                    flips += 1;
                }
                prev_dir = dir;
            }
        }
        if flips == 0 {
            continue;
        }

        // Overshoot-relax: rises then falls (or falls then rises) with a peak/trough.
        let ranks: Vec<i32> = seq.iter().map(|(s, _)| status_rank(*s)).collect();
        let mut has_peak = false;
        let mut has_trough = false;
        for i in 1..ranks.len().saturating_sub(1) {
            if ranks[i] > ranks[i - 1] && ranks[i] > ranks[i + 1] {
                has_peak = true;
            }
            if ranks[i] < ranks[i - 1] && ranks[i] < ranks[i + 1] {
                has_trough = true;
            }
        }
        let kind = if flips >= 2 {
            "oscillation"
        } else if has_peak || has_trough {
            "overshoot_relax"
        } else {
            "flip_flop"
        };
        let citing: Vec<String> = seq.iter().map(|(_, e)| e.id.clone()).collect();
        flags.push(StabilityFlag {
            subject_id: subject.to_string(),
            kind: kind.to_string(),
            transitions,
            flips,
            summary: format!(
                "{}: {} status transitions, {} direction flips (pattern: {})",
                subject, transitions, flips, kind
            ),
            citing,
        });
    }
    flags.sort_by(|a, b| (&a.subject_id, &a.kind).cmp(&(&b.subject_id, &b.kind)));
    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epistemic::{EpistemicAuditTrail, EpistemicEvent, EpistemicEventType};
    use chrono::{Duration, TimeZone, Utc};

    fn obs(
        trail: &mut EpistemicAuditTrail,
        subject: &str,
        ts: chrono::DateTime<chrono::Utc>,
        id: &str,
    ) {
        let mut ev = EpistemicEvent::new(EpistemicEventType::ObservationIngested, subject, "obs");
        ev.id = id.to_string();
        ev.timestamp = ts;
        ev.asserted_at = Some(ts);
        trail.record(ev);
    }

    fn status(
        trail: &mut EpistemicAuditTrail,
        subject: &str,
        ts: chrono::DateTime<chrono::Utc>,
        id: &str,
        post: &str,
    ) {
        let mut ev = EpistemicEvent::new(EpistemicEventType::StatusTransition, subject, "st")
            .with_transition("prior", post);
        ev.id = id.to_string();
        ev.timestamp = ts;
        ev.asserted_at = Some(ts);
        trail.record(ev);
    }

    #[test]
    fn recurrence_evidence_marks_sustained_reappearance() {
        let mut trail = EpistemicAuditTrail::new();
        let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        status(&mut trail, "h1", t0, "s1", "failed");
        // sustained: observations spread across multiple bins after retirement
        for i in 1..=6 {
            obs(
                &mut trail,
                "h1",
                t0 + Duration::hours(i * 6),
                &format!("o{i}"),
            );
        }
        let props = dream_over_history(&trail, &[], Duration::days(2));
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].kind, ProposalKind::ReactivateHypothesis);
        let rec = props[0].recurrence.as_ref().expect("recurrence evidence");
        assert!(rec.observations >= 3);
        assert!(rec.active_bins >= 2);
        assert!(
            props[0].rationale.contains("recurrence:"),
            "rationale should include recurrence: {}",
            props[0].rationale
        );
    }

    #[test]
    fn analyse_stability_flags_oscillation_and_overshoot() {
        let mut trail = EpistemicAuditTrail::new();
        let t0 = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        // Candidate -> Supported -> Contradicted -> Supported -> Contradicted (oscillation/flips)
        let mut gen = EpistemicEvent::new(EpistemicEventType::HypothesisGenerated, "h2", "gen");
        gen.id = "g".into();
        gen.timestamp = t0;
        gen.asserted_at = Some(t0);
        trail.record(gen);
        status(&mut trail, "h2", t0 + Duration::hours(1), "t1", "supported");
        status(
            &mut trail,
            "h2",
            t0 + Duration::hours(2),
            "t2",
            "contradicted",
        );
        status(&mut trail, "h2", t0 + Duration::hours(3), "t3", "supported");
        status(
            &mut trail,
            "h2",
            t0 + Duration::hours(4),
            "t4",
            "contradicted",
        );

        let flags = analyse_stability(&trail);
        assert!(!flags.is_empty(), "expected instability flags");
        let f = &flags[0];
        assert_eq!(f.subject_id, "h2");
        assert!(f.flips >= 2 || f.kind == "oscillation" || f.kind == "overshoot_relax");
    }

    #[test]
    fn analyse_stability_is_deterministic_and_empty_for_monotonic() {
        let mut trail = EpistemicAuditTrail::new();
        let t0 = Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();
        let mut gen = EpistemicEvent::new(EpistemicEventType::HypothesisGenerated, "h3", "gen");
        gen.id = "g".into();
        gen.timestamp = t0;
        gen.asserted_at = Some(t0);
        trail.record(gen);
        status(&mut trail, "h3", t0 + Duration::hours(1), "t1", "supported");
        status(&mut trail, "h3", t0 + Duration::hours(2), "t2", "confirmed");
        assert!(analyse_stability(&trail).is_empty());
        let a = analyse_stability(&trail);
        let b = analyse_stability(&trail);
        assert_eq!(a, b);
    }
}
