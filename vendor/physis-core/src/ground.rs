//! Common ground — the conceptual state a human and a machine both work on.
//!
//! ## What this is for
//!
//! A person and an agent working on the same problem normally share nothing but
//! prose. Each re-derives what the other already established, each states
//! conclusions without the evidence that produced them, and neither can tell
//! later which claims were actually tested. Fluent output substitutes for
//! evidence because there is nowhere else for evidence to live.
//!
//! This module renders the place it lives. Not a report *about* the work — the
//! **state of the work**: what is currently believed, what it rests on, what
//! contradicts it, what was predicted and never scored, and what changed when.
//!
//! Both sides write to the same store through the same commands
//! (`hypothesis create/evidence/transition`, `contradiction record`), so the
//! view is shared by construction rather than by synchronisation.
//!
//! ## What it deliberately does not do
//!
//! - **No model is involved.** Nothing here embeds, generates, ranks or scores
//!   with a network. It reads persisted state and arranges it. Physis's claim
//!   is to be the common ground, not another voice in the conversation.
//! - **It does not resolve contradictions.** Open ones are shown open, with
//!   both parties and both sources. A view that quietly picked a side would be
//!   destroying the one signal a disagreement carries.
//! - **It does not summarise.** Every line is a record that exists, rendered.
//!
//! ## The three questions it answers
//!
//! 1. **What do we believe, and how hard did it earn that?** Claims by standing,
//!    with evidence counts — supporting *and* contradicting, never netted into
//!    one number.
//! 2. **What are we still disagreeing about?** Open contradictions, both sides.
//! 3. **What did we promise to check and never check?** Open predictions with
//!    their age. This is the section that makes the state honest: a claim with
//!    an unresolved prediction from three weeks ago is not settled, however
//!    confident its wording.

use crate::core::PhysisCore;
use crate::hypothesis::HypothesisStatus;
use serde::{Deserialize, Serialize};

/// One claim in the shared state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub status: String,
    pub fitness: f32,
    /// Supporting and contradicting counts, kept apart on purpose: a claim with
    /// 9 for and 8 against is not the same object as one with 1 for and 0
    /// against, and a single net number erases exactly that difference.
    ///
    /// Excludes `obs:` evidence, which is provenance rather than corroboration
    /// and appears under [`Claim::grounded_in`].
    pub for_count: usize,
    pub against_count: usize,
    /// Predictions made and never resolved.
    pub open_predictions: usize,
    /// Observations this claim cites, resolved back into the append-only log.
    ///
    /// A claim grounded in a recorded observation is a different object from an
    /// assertion someone typed: its evidence can be re-read rather than
    /// trusted. Empty means the claim rests on something outside the log, which
    /// is allowed and worth seeing.
    pub grounded_in: Vec<String>,
}

/// A disagreement that has not been resolved, shown as a disagreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenDisagreement {
    pub id: String,
    pub claim_a: String,
    pub source_a: String,
    pub claim_b: String,
    pub source_b: String,
}

/// A promise to check something, still outstanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unscored {
    pub claim_id: String,
    pub statement: String,
    pub prediction: String,
    pub days_open: i64,
}

/// The whole shared state in one value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ground {
    pub claims: Vec<Claim>,
    pub disagreements: Vec<OpenDisagreement>,
    pub unscored: Vec<Unscored>,
    pub events: usize,
    pub nodes: usize,
}

fn rank(s: &HypothesisStatus) -> u8 {
    // Strongest standing first, and Contradicted ABOVE Candidate: a claim that
    // was tested and lost is more informative than one never tested at all.
    match s {
        HypothesisStatus::Certified => 0,
        HypothesisStatus::Confirmed => 1,
        HypothesisStatus::Supported => 2,
        HypothesisStatus::Contradicted => 3,
        HypothesisStatus::Failed => 4,
        HypothesisStatus::Superseded => 5,
        HypothesisStatus::Isolated => 6,
        HypothesisStatus::Inert => 7,
        HypothesisStatus::Candidate => 8,
    }
}

/// Read the shared state out of a loaded core. No model, no network.
///
/// `observations` is the append-only log; claims that cite it are resolved so
/// the view shows which beliefs are anchored to something the machine actually
/// saw. Pass an empty slice to render the state without that resolution.
pub fn read(
    core: &PhysisCore,
    observations: &[crate::observe::Observation],
    now: chrono::DateTime<chrono::Utc>,
) -> Ground {
    let mut claims: Vec<Claim> = core
        .hypotheses
        .values()
        .map(|h| Claim {
            id: h.id.chars().take(8).collect(),
            statement: h.statement.clone(),
            status: format!("{:?}", h.status),
            fitness: h.fitness,
            // `obs:` evidence is PROVENANCE, not corroboration — it is what the
            // claim is about, not a reason to believe it. It is rendered on its
            // own line as `grounded in`, so counting it here too would show a
            // freshly asserted claim as already corroborated. Same conflation
            // `observe::promote` avoids at the status level, avoided again at
            // the display level.
            for_count: h
                .supporting_evidence
                .iter()
                .filter(|e| !e.source.starts_with("obs:"))
                .count(),
            against_count: h
                .contradicting_evidence
                .iter()
                .filter(|e| !e.source.starts_with("obs:"))
                .count(),
            open_predictions: h
                .predictions
                .iter()
                .filter(|p| p.observed_at.is_none())
                .count(),
            grounded_in: crate::observe::cited_by(h, observations)
                .iter()
                .map(|o| format!("#{} {} {}", o.seq, o.source, o.subject))
                .collect(),
        })
        .collect();
    // Deterministic: standing, then fitness, then id. Same state renders the
    // same way for whoever opens it.
    claims.sort_by(|a, b| {
        let (ra, rb) = (
            core.hypotheses
                .values()
                .find(|h| h.id.starts_with(&a.id))
                .map(|h| rank(&h.status))
                .unwrap_or(9),
            core.hypotheses
                .values()
                .find(|h| h.id.starts_with(&b.id))
                .map(|h| rank(&h.status))
                .unwrap_or(9),
        );
        ra.cmp(&rb)
            .then(
                b.fitness
                    .partial_cmp(&a.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.id.cmp(&b.id))
    });

    let disagreements = core
        .contradictions
        .iter()
        .filter(|c| matches!(c.resolution, crate::contradiction::ResolutionStatus::Open))
        .map(|c| OpenDisagreement {
            id: c.id.chars().take(8).collect(),
            claim_a: c.claim_a.claim.clone(),
            source_a: c.claim_a.source.clone(),
            claim_b: c.claim_b.claim.clone(),
            source_b: c.claim_b.source.clone(),
        })
        .collect();

    let mut unscored: Vec<Unscored> = Vec::new();
    for h in core.hypotheses.values() {
        for p in h.predictions.iter().filter(|p| p.observed_at.is_none()) {
            unscored.push(Unscored {
                claim_id: h.id.chars().take(8).collect(),
                statement: h.statement.chars().take(70).collect(),
                prediction: p.statement.clone(),
                days_open: (now - p.made_at).num_days(),
            });
        }
    }
    // Oldest first: the longer a promise goes unscored, the more it matters.
    unscored.sort_by(|a, b| {
        b.days_open
            .cmp(&a.days_open)
            .then(a.claim_id.cmp(&b.claim_id))
    });

    Ground {
        claims,
        disagreements,
        unscored,
        events: core.epistemic_audit.events.len(),
        nodes: core.nodes.len(),
    }
}

impl Ground {
    pub fn render(&self) -> String {
        let mut o = String::new();
        o.push_str("── COMMON GROUND ──\n");
        o.push_str(&format!(
            "{} claim(s) · {} open disagreement(s) · {} unscored prediction(s) · {} recorded event(s)\n\n",
            self.claims.len(), self.disagreements.len(), self.unscored.len(), self.events
        ));

        if self.claims.is_empty() {
            o.push_str("No claims yet. Nothing has been asserted, so nothing can be wrong.\n");
            o.push_str("  physis-core hypothesis create \"<claim>\" --confidence 0.6\n\n");
        } else {
            o.push_str("WHAT WE BELIEVE — evidence for and against, never netted\n");
            for c in &self.claims {
                o.push_str(&format!(
                    "  [{}] {:<13} fit {:.3}   +{} / -{}{}\n      {}\n",
                    c.id,
                    c.status,
                    c.fitness,
                    c.for_count,
                    c.against_count,
                    if c.open_predictions > 0 {
                        format!("   {} unscored", c.open_predictions)
                    } else {
                        String::new()
                    },
                    c.statement.chars().take(92).collect::<String>()
                ));
                for g in &c.grounded_in {
                    o.push_str(&format!(
                        "      ↳ grounded in {}\n",
                        g.chars().take(86).collect::<String>()
                    ));
                }
            }
            o.push('\n');
        }

        if !self.disagreements.is_empty() {
            o.push_str("WHAT WE STILL DISAGREE ABOUT — both sides kept\n");
            for d in &self.disagreements {
                o.push_str(&format!(
                    "  [{}]\n    A  {}\n       ({})\n    B  {}\n       ({})\n",
                    d.id, d.claim_a, d.source_a, d.claim_b, d.source_b
                ));
            }
            o.push('\n');
        }

        if self.unscored.is_empty() {
            o.push_str("NOTHING UNSCORED — every prediction made has been resolved.\n");
        } else {
            o.push_str("WHAT WE PROMISED TO CHECK AND DID NOT — oldest first\n");
            for u in &self.unscored {
                o.push_str(&format!(
                    "  [{}] {:>4}d  {}\n           on: {}\n",
                    u.claim_id, u.days_open, u.prediction, u.statement
                ));
            }
            o.push_str("\n  A claim with an unresolved prediction is not settled, however\n");
            o.push_str("  confident its wording.  physis-core hypothesis resolve <id> <n> \"<what happened>\"\n");
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::Hypothesis;

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }

    /// Evidence counts must never be netted into one number. A claim fought to
    /// a standstill and a claim never challenged are different objects, and a
    /// single figure erases exactly that difference.
    #[test]
    fn evidence_for_and_against_stay_separate() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("contested", vec![]);
        h.add_supporting_evidence(crate::hypothesis::Evidence::supports("a", "x"));
        h.add_contradicting_evidence(crate::hypothesis::Evidence::contradicts("b", "y"));
        core.hypotheses.insert(h.id.clone(), h);
        let g = read(&core, &[], now());
        assert_eq!(g.claims[0].for_count, 1);
        assert_eq!(g.claims[0].against_count, 1);
        let r = g.render();
        assert!(r.contains("+1 / -1"), "counts must reach the output: {r}");
    }

    /// An empty state must say so plainly rather than render an empty frame
    /// that looks like a finished report.
    #[test]
    fn an_empty_ground_says_nothing_has_been_asserted() {
        let g = read(&PhysisCore::new(), &[], now());
        assert!(g.claims.is_empty());
        assert!(g.render().contains("nothing can be wrong"));
    }

    /// The unscored section is the one that makes the state honest, so it must
    /// appear whenever a prediction is outstanding.
    #[test]
    fn an_open_prediction_is_surfaced_with_its_age() {
        let mut core = PhysisCore::new();
        let mut h = Hypothesis::new("predicts things", vec![]);
        h.add_prediction(crate::hypothesis::Prediction::new("the gate will pass"));
        core.hypotheses.insert(h.id.clone(), h);
        let g = read(&core, &[], now() + chrono::Duration::days(9));
        assert_eq!(g.unscored.len(), 1);
        assert_eq!(g.unscored[0].days_open, 9);
        assert!(g.render().contains("PROMISED TO CHECK"));
    }

    /// The loop closed: a claim promoted from an observation must show, in the
    /// shared view, which observation it rests on. Without this the claim path
    /// exists in the library and is invisible to the person reading the state —
    /// which is how `promote` and `cited_by` shipped with zero call sites.
    #[test]
    fn a_claim_shows_the_observation_it_rests_on() {
        let mut o = crate::observe::Observation::new("git", "abc123").with_body("fix the null");
        o.seq = 7;
        let h = crate::observe::promote(&o, "the null was invalid", vec![0.1]);

        let mut core = PhysisCore::new();
        core.hypotheses.insert(h.id.clone(), h);

        let g = read(&core, std::slice::from_ref(&o), now());
        assert_eq!(g.claims[0].grounded_in.len(), 1);
        assert!(g.claims[0].grounded_in[0].contains("#7"));
        assert!(
            g.render().contains("grounded in"),
            "it must reach the output"
        );

        // And a claim resting on nothing in the log says so by absence, not by
        // a fabricated citation.
        let g2 = read(&core, &[], now());
        assert!(g2.claims[0].grounded_in.is_empty());
        assert!(!g2.render().contains("grounded in"));
    }

    /// A freshly promoted observation must read as *asserted*, not *corroborated*:
    /// Candidate status, zero evidence counted, and the citation shown
    /// separately. Getting this wrong fills the shared state with claims that
    /// look established because someone pressed a button.
    #[test]
    fn provenance_is_not_counted_as_corroboration() {
        let mut o = crate::observe::Observation::new("git", "deadbeef");
        o.seq = 3;
        let h = crate::observe::promote(&o, "this commit caused the regression", vec![0.0]);
        let mut core = PhysisCore::new();
        core.hypotheses.insert(h.id.clone(), h);

        let g = read(&core, std::slice::from_ref(&o), now());
        assert_eq!(g.claims[0].status, "Candidate");
        assert_eq!(
            g.claims[0].for_count, 0,
            "provenance must not count as support"
        );
        assert_eq!(g.claims[0].against_count, 0);
        assert_eq!(
            g.claims[0].grounded_in.len(),
            1,
            "but it must still be shown"
        );
        let r = g.render();
        assert!(r.contains("+0 / -0"));
        assert!(r.contains("grounded in"));
    }

    /// Rendering is deterministic: same state, same bytes, for whoever opens it.
    #[test]
    fn the_view_is_deterministic() {
        let mut core = PhysisCore::new();
        for s in ["alpha", "beta", "gamma"] {
            let h = Hypothesis::new(s, vec![]);
            core.hypotheses.insert(h.id.clone(), h);
        }
        let t = now();
        assert_eq!(read(&core, &[], t).render(), read(&core, &[], t).render());
    }
}
