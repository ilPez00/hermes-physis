//! Acting, recorded — and checked against what is already believed.
//!
//! ## The gap this closes
//!
//! The survey in `computer-remake-research/` found that acting and remembering
//! are never the same system. Systems that **act** (open-interpreter, ragfs
//! `.ops/`) accumulate no belief; systems that **remember** (recall,
//! localsearch, ActivityWatch) cannot act. ragfs comes closest — it acts through
//! `.ops/` and audits through `.safety/` — but its memory is an index, not a
//! state of belief.
//!
//! The consequence, stated in `gaps.md` as Gap 8:
//!
//! > **No system in the survey can notice that an action it took contradicted
//! > something it already held.**
//!
//! That is the specific failure the brief calls *catching agentic mistakes*, and
//! it requires action and belief to live in one substrate.
//!
//! ## What this does, and what it deliberately does not
//!
//! It is **not** a sandbox and not a permission system. `sh -c` already runs
//! commands; the contribution here is that running one:
//!
//! 1. records the intent as an observation *before* the command runs, so an
//!    action that hangs or kills the process is still in the record;
//! 2. records the outcome with its real duration;
//! 3. **recalls what is already believed about this** and puts it in front of
//!    the operator — especially claims already `Contradicted`, which is the
//!    machine saying *you established this does not work*.
//!
//! Step 3 is the whole point. Steps 1 and 2 are bookkeeping that make it
//! possible.
//!
//! ## Honest limits
//!
//! Relevance is cosine over the command text against claim statements. That is
//! a weak matcher and it is the same lexical-versus-semantic caveat as
//! everywhere else in this crate: under the random-projection fallback it
//! matches strings, not meaning. It surfaces candidates for a person to read; it
//! does not decide anything, and it will miss a contradiction phrased
//! differently from the command that triggers it.

use crate::embed::VectorEmbed;
use serde::{Deserialize, Serialize};

/// Something already believed that bears on what is about to be done.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bearing {
    /// Short display id (prefix of the hypothesis id).
    pub id: String,
    /// Full hypothesis id (for support/affected snapshot lookup). Empty in older records.
    #[serde(default)]
    pub hypothesis_id: String,
    pub status: String,
    pub statement: String,
    pub relevance: f32,
    /// Unresolved predictions riding on this claim.
    pub open_predictions: usize,
}

impl Bearing {
    /// Does this deserve to stop someone?
    ///
    /// A `Contradicted` claim is the machine saying *this was tried and it did
    /// not work*. An unresolved prediction is a promise that was never scored.
    /// Both are worth reading before acting; a merely-related Candidate is not.
    pub fn is_warning(&self) -> bool {
        self.status == "Contradicted" || self.status == "Failed" || self.open_predictions > 0
    }
}

/// What happened, and what was already known about it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acted {
    pub command: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout_lines: usize,
    pub stderr_tail: String,
    /// Claims that bear on this command, most relevant first.
    pub bearing: Vec<Bearing>,
    /// Sequence numbers of the observations this produced.
    pub observed: Vec<u64>,
}

impl Acted {
    pub fn warnings(&self) -> Vec<&Bearing> {
        self.bearing.iter().filter(|b| b.is_warning()).collect()
    }

    /// Deterministic "why supported / affected now" snapshots for all
    /// bearings, in relevance order. Read-only diagnostic (no verdict/
    /// certification); bearings with no resolvable hypothesis id (e.g. legacy
    /// records) yield `None`.
    pub fn support_snapshots(
        &self,
        core: &crate::core::PhysisCore,
        recent_event_limit: usize,
    ) -> Vec<(Bearing, Option<crate::support::SupportSnapshot>)> {
        self.bearing
            .iter()
            .map(|b| {
                let snap = if b.hypothesis_id.is_empty() {
                    None
                } else {
                    core.support_snapshot(&b.hypothesis_id, recent_event_limit)
                };
                (b.clone(), snap)
            })
            .collect()
    }

    /// Deterministic "why supported / affected now" snapshots for warning
    /// bearings (claims already contradicted/failed or with unscored predictions).
    /// Read-only diagnostic (no verdict/certification); bearings with no
    /// resolvable hypothesis id (e.g. legacy records) yield `None`.
    pub fn warning_support_snapshots(
        &self,
        core: &crate::core::PhysisCore,
        recent_event_limit: usize,
    ) -> Vec<(Bearing, Option<crate::support::SupportSnapshot>)> {
        self.warnings()
            .into_iter()
            .map(|b| {
                let snap = if b.hypothesis_id.is_empty() {
                    None
                } else {
                    core.support_snapshot(&b.hypothesis_id, recent_event_limit)
                };
                (b.clone(), snap)
            })
            .collect()
    }

    pub fn render(&self) -> String {
        let mut o = String::new();
        let w = self.warnings();
        if !w.is_empty() {
            o.push_str("── ALREADY ESTABLISHED ──\n");
            for b in &w {
                o.push_str(&format!(
                    "  [{}] {:<13} rel {:.3}{}\n      {}\n",
                    b.id,
                    b.status,
                    b.relevance,
                    if b.open_predictions > 0 {
                        format!("   {} unscored", b.open_predictions)
                    } else {
                        String::new()
                    },
                    b.statement.chars().take(88).collect::<String>()
                ));
            }
            o.push('\n');
        }
        o.push_str(&format!(
            "── RAN ──\n{}\nexit {} · {:.1}s · {} line(s) out\n",
            self.command,
            self.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into()),
            self.duration_ms as f64 / 1000.0,
            self.stdout_lines
        ));
        if !self.stderr_tail.trim().is_empty() {
            o.push_str(&format!("stderr: {}\n", self.stderr_tail.trim()));
        }
        o.push_str(&format!(
            "\nrecorded as observation(s) {}\n",
            self.observed
                .iter()
                .map(|s| format!("#{s}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        o
    }
}

/// Record an acted command/outcome into the core epistemic audit as append-only
/// observation events (intent + outcome). This is a record of what was
/// attempted/observed, not a claim of verified truth, certification, or domain
/// success beyond the captured exit/duration/stderr tail.
///
/// Returns the ids of the recorded audit events (2 when both recorded).
pub fn record_acted_in_audit(core: &mut crate::core::PhysisCore, acted: &Acted) -> Vec<String> {
    let mut ids = Vec::with_capacity(2);

    let intent = crate::epistemic::EpistemicEvent::new(
        crate::epistemic::EpistemicEventType::ObservationIngested,
        format!("action:{}", acted.command),
        format!("action intent: {}", acted.command),
    )
    .with_source("act");
    ids.push(intent.id.clone());
    core.record_epistemic_event(intent);

    let mut outcome_desc = format!(
        "action outcome: exit {} · {}ms · {} line(s) out",
        acted
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".into()),
        acted.duration_ms,
        acted.stdout_lines
    );
    if !acted.stderr_tail.trim().is_empty() {
        outcome_desc.push_str(&format!(" · stderr: {}", acted.stderr_tail.trim()));
    }
    let mut outcome = crate::epistemic::EpistemicEvent::new(
        crate::epistemic::EpistemicEventType::ObservationIngested,
        format!("action:{}", acted.command),
        outcome_desc,
    )
    .with_source("act");
    // record runtime as metric (ms) for observability; not a truth verdict
    outcome.metric_value = Some(acted.duration_ms as f32);
    ids.push(outcome.id.clone());
    core.record_epistemic_event(outcome);

    ids
}

/// Claims bearing on `command`, most relevant first.
pub fn bearing_on(
    core: &crate::core::PhysisCore,
    command: &str,
    embedder: &dyn VectorEmbed,
    top: usize,
) -> Vec<Bearing> {
    let q = embedder.embed(command);
    let mut v: Vec<Bearing> = core
        .hypotheses
        .values()
        .map(|h| Bearing {
            id: h.id.chars().take(8).collect(),
            hypothesis_id: h.id.clone(),
            status: format!("{:?}", h.status),
            statement: h.statement.clone(),
            relevance: crate::models::cosine_sim(&q, &h.embedding),
            open_predictions: h
                .predictions
                .iter()
                .filter(|p| p.observed_at.is_none())
                .count(),
        })
        .filter(|b| b.relevance.is_finite())
        .collect();
    v.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.id.cmp(&b.id))
    });
    v.truncate(top);
    v
}

/// Run `command`, recording intent before and outcome after.
///
/// The intent observation is written first and on purpose: a command that hangs,
/// is killed, or takes the machine down still leaves a record that it was
/// attempted. A log that only records completions cannot explain a crash.
pub fn run(command: &str, bearing: Vec<Bearing>, log: &std::path::Path) -> anyhow::Result<Acted> {
    let mut intent = crate::observe::Observation::new("action", command)
        .with_body("intent")
        .by("act");
    let (first, _) = crate::observe::append(log, std::slice::from_mut(&mut intent))?;

    let started = std::time::Instant::now();
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;
    let duration_ms = started.elapsed().as_millis() as u64;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let stderr_tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ");

    let mut outcome = crate::observe::Observation::new("action", command)
        .with_body(format!(
            "exit {} · {} out / {} err line(s){}",
            out.status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into()),
            stdout.lines().count(),
            stderr.lines().count(),
            if stderr_tail.is_empty() {
                String::new()
            } else {
                format!(" · {stderr_tail}")
            }
        ))
        .by("act");
    outcome.duration_ms = Some(duration_ms);
    let (second, _) = crate::observe::append(log, std::slice::from_mut(&mut outcome))?;

    Ok(Acted {
        command: command.to_string(),
        exit_code: out.status.code(),
        duration_ms,
        stdout_lines: stdout.lines().count(),
        stderr_tail,
        bearing,
        observed: vec![first, second],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;
    use crate::hypothesis::{Evidence, Hypothesis};

    fn tmp(n: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("physis-act-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("o.jsonl")
    }

    /// An action that hangs or kills the process must still leave a record that
    /// it was attempted. A log of completions only cannot explain a crash.
    #[test]
    fn intent_is_recorded_before_the_command_runs() {
        let log = tmp("intent");
        let a = run("true", vec![], &log).unwrap();
        let all = crate::observe::read(&log).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].body, "intent", "intent must be written first");
        assert!(all[1].body.starts_with("exit 0"));
        assert!(
            all[1].duration_ms.is_some(),
            "the outcome carries the runtime"
        );
        assert_eq!(a.observed, vec![1, 2]);
    }

    /// The point of the module: a claim already refuted must be put in front of
    /// the operator, and a merely-related candidate must not.
    #[test]
    fn a_refuted_claim_is_a_warning_and_a_candidate_is_not() {
        let refuted = Bearing {
            id: "aaaaaaaa".into(),
            hypothesis_id: "aaaaaaaa-full".into(),
            status: "Contradicted".into(),
            statement: "this approach fails".into(),
            relevance: 0.9,
            open_predictions: 0,
        };
        let candidate = Bearing {
            id: "bbbbbbbb".into(),
            hypothesis_id: "bbbbbbbb-full".into(),
            status: "Candidate".into(),
            statement: "might work".into(),
            relevance: 0.9,
            open_predictions: 0,
        };
        let unscored = Bearing {
            open_predictions: 1,
            ..candidate.clone()
        };
        assert!(refuted.is_warning());
        assert!(
            !candidate.is_warning(),
            "relatedness alone must not interrupt"
        );
        assert!(unscored.is_warning(), "an unkept promise is worth reading");
    }

    /// Relevance must actually rank: a claim about the command's subject should
    /// outrank an unrelated one.
    #[test]
    fn bearing_ranks_by_relevance() {
        let e = RandomProjectionEmbedder::new(128);
        let mut core = crate::core::PhysisCore::new();
        for text in [
            "cargo test fails on this crate",
            "the kitchen needs painting",
        ] {
            let mut h = Hypothesis::new(text, e.embed(text));
            h.add_supporting_evidence(Evidence::supports("x", "y"));
            core.hypotheses.insert(h.id.clone(), h);
        }
        let b = bearing_on(&core, "cargo test", &e, 2);
        assert_eq!(b.len(), 2);
        assert!(
            b[0].statement.contains("cargo"),
            "the related claim must rank first, got {:?}",
            b[0].statement
        );
        assert!(!b[0].hypothesis_id.is_empty());
        // full id enables support/affected "why supported now" lookup
        assert!(core.support_snapshot(&b[0].hypothesis_id, 5).is_some());
    }

    /// Warnings are a filter over bearing, not a separate list that can drift.
    #[test]
    fn warnings_are_a_subset_of_bearing() {
        let a = Acted {
            command: "x".into(),
            exit_code: Some(0),
            duration_ms: 1,
            stdout_lines: 0,
            stderr_tail: String::new(),
            bearing: vec![
                Bearing {
                    id: "a".into(),
                    hypothesis_id: "a-full".into(),
                    status: "Contradicted".into(),
                    statement: "s".into(),
                    relevance: 0.5,
                    open_predictions: 0,
                },
                Bearing {
                    id: "b".into(),
                    hypothesis_id: "b-full".into(),
                    status: "Supported".into(),
                    statement: "t".into(),
                    relevance: 0.4,
                    open_predictions: 0,
                },
            ],
            observed: vec![1, 2],
        };
        assert_eq!(a.warnings().len(), 1);
        assert!(a.render().contains("ALREADY ESTABLISHED"));
        assert!(a.render().contains("recorded as observation(s) #1, #2"));
    }

    /// Warning bearings can be resolved to deterministic "why supported now"
    /// snapshots (read-only diagnostic) via their full hypothesis ids, and all
    /// bearings can be resolved in relevance order (legacy/unresolvable -> None).
    #[test]
    fn bearing_support_snapshots_resolve_hypotheses() {
        let e = RandomProjectionEmbedder::new(128);
        let mut core = crate::core::PhysisCore::new();
        let mut contradicted = Hypothesis::new(
            "cargo test fails on this crate",
            e.embed("cargo test fails"),
        );
        contradicted.add_supporting_evidence(Evidence::supports("ci", "failed run"));
        contradicted.status = crate::hypothesis::HypothesisStatus::Contradicted;
        let hid = core.register_hypothesis(contradicted);
        let mut ok = Hypothesis::new("the kitchen needs painting", e.embed("kitchen painting"));
        ok.add_supporting_evidence(Evidence::supports("note", "walls chipped"));
        core.register_hypothesis(ok);

        let bearing = bearing_on(&core, "cargo test", &e, 2);
        let acted = Acted {
            command: "cargo test".into(),
            exit_code: Some(1),
            duration_ms: 10,
            stdout_lines: 0,
            stderr_tail: String::new(),
            bearing: bearing.clone(),
            observed: vec![],
        };

        let warn_snaps = acted.warning_support_snapshots(&core, 5);
        assert_eq!(warn_snaps.len(), 1);
        assert_eq!(warn_snaps[0].0.hypothesis_id, hid);
        let snap = warn_snaps[0]
            .1
            .as_ref()
            .expect("snapshot for known warning");
        assert_eq!(snap.hypothesis_id, hid);
        assert!(snap.contradiction_pressure);

        let all = acted.support_snapshots(&core, 5);
        assert_eq!(all.len(), bearing.len());
        assert!(all
            .iter()
            .any(|(b, s)| b.hypothesis_id == hid && s.is_some()));
        for (b, s) in &all {
            if !b.hypothesis_id.is_empty() {
                assert!(
                    s.is_some(),
                    "known bearing must resolve: {}",
                    b.hypothesis_id
                );
            }
        }

        let mut legacy_bearing = bearing[0].clone();
        legacy_bearing.hypothesis_id.clear();
        let acted_legacy = Acted {
            bearing: vec![legacy_bearing],
            ..acted
        };
        let legacy = acted_legacy.support_snapshots(&core, 5);
        assert_eq!(legacy.len(), 1);
        assert!(legacy[0].1.is_none());
    }

    /// Full hypothesis id is carried for support/why-supported lookups while
    /// older serialized bearings (without it) remain readable.
    #[test]
    fn bearing_keeps_full_hypothesis_id_and_deserializes_legacy_shape() {
        let e = RandomProjectionEmbedder::new(64);
        let mut core = crate::core::PhysisCore::new();
        let mut h = Hypothesis::new("cargo test fails", e.embed("cargo test fails"));
        h.add_supporting_evidence(Evidence::supports("ci", "failed"));
        let hid = core.register_hypothesis(h);
        let b = bearing_on(&core, "cargo test", &e, 1);
        assert_eq!(b[0].hypothesis_id, hid);

        let legacy = r#"{"id":"abcdefgh","status":"Candidate","statement":"s","relevance":0.5,"open_predictions":0}"#;
        let de: Bearing = serde_json::from_str(legacy).unwrap();
        assert_eq!(de.id, "abcdefgh");
        assert!(de.hypothesis_id.is_empty());
    }

    /// Recording an acted outcome into core audit is append-only and
    /// observational (no hypotheses/edges/contradictions created).
    #[test]
    fn record_acted_in_audit_appends_observation_events() {
        let mut core = crate::core::PhysisCore::new();
        let before = core.epistemic_audit.events.len();
        let a = Acted {
            command: "echo hi".into(),
            exit_code: Some(0),
            duration_ms: 7,
            stdout_lines: 1,
            stderr_tail: String::new(),
            bearing: vec![],
            observed: vec![1, 2],
        };
        let ids = record_acted_in_audit(&mut core, &a);
        assert_eq!(ids.len(), 2);
        assert_eq!(core.epistemic_audit.events.len(), before + 2);
        assert!(core.hypotheses.is_empty());
        assert!(core.edges.is_empty());
        assert!(core.contradictions.is_empty());
        assert!(core
            .epistemic_audit
            .events
            .iter()
            .all(|e| e.event_type == crate::epistemic::EpistemicEventType::ObservationIngested));
    }
}
