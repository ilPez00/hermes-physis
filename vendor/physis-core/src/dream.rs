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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::delta_engine::{MutationOp, OntologyMutation};
use crate::epistemic::{EpistemicAuditTrail, EpistemicEventType};

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

        proposals.push(RetrospectiveProposal {
            subject_id: subject.to_string(),
            kind: ProposalKind::ReactivateHypothesis,
            anchor_time: retired_ev.assertion_time(),
            rationale: format!(
                "branch retired to {} at {}, but {} later observation(s) \
                 re-present the pattern within the lookback; the branch was \
                 retained (G7), never deleted",
                retired_ev.posterior_state.as_deref().unwrap_or("?"),
                retired_ev.assertion_time(),
                re_presented.len(),
            ),
            citing: re_presented.iter().map(|e| e.id.clone()).collect(),
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
