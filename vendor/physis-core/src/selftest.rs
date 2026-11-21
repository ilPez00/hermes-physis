//! Selftest: the engine audits its own gated observer fusion against
//! caller-supplied known-answer probes.
//!
//! A probe pairs a query with an observer claim whose text the engine can
//! never retrieve (held-out), a distinctive phrase that must appear in the
//! fused packet, and forbidden phrases (noise text) that must not. The probe
//! also checks the phrase is absent from the *baseline* packet — a "held-out"
//! answer the engine already retrieves proves nothing.
//!
//! This institutionalises the project rule that a worker's success claim is
//! not evidence: the binary ships with its own falsification harness, and
//! `physis-core selftest` exits nonzero when a should-pass probe defers or
//! noise text enters a packet.

use serde::{Deserialize, Serialize};

/// The `--probes` file envelope: {"probes": [...]}.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelftestProbeFile {
    pub probes: Vec<SelftestProbe>,
}

/// What the shipped configuration is measured to do with this probe.
/// `answer`: the gate must admit the claim (phrase in packet). `defer`:
/// the gate must defer it (phrase absent) — the measured behaviour of the
/// shipped default for harder queries; a change either way is drift from
/// the measurement record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Expect {
    #[default]
    Answer,
    Defer,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelftestProbe {
    pub name: String,
    /// What the shipped configuration is measured to do. Default: answer.
    #[serde(default)]
    pub expect: Expect,
    pub query: String,
    /// Distinctive phrase from the held-out claim; must appear in the fused
    /// packet and be absent from the baseline packet.
    pub phrase: String,
    /// Noise text fragments; none may appear in the fused packet.
    #[serde(default)]
    pub forbidden: Vec<String>,
    /// Observer claims in the standard `--observer-claims` JSON format.
    /// Claim 0 is the relevant one; the rest are noise.
    pub claims: Vec<crate::ingest::CandidateClaimPacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelftestProbeReport {
    pub name: String,
    pub baseline_absent: bool,
    pub answered: bool,
    pub forbidden_absent: bool,
    pub observer_selected: usize,
    pub engine_best_score: Option<f32>,
    pub passed: bool,
    /// Raw cosine per claim from the fused run's gate receipts — the drift
    /// signal. `#[serde(default)]` keeps old reports parseable.
    #[serde(default)]
    pub relevances: std::collections::BTreeMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelftestReport {
    pub corpus_documents: usize,
    pub floor: f32,
    pub probes: Vec<SelftestProbeReport>,
    pub passed: usize,
    pub total: usize,
    pub ok: bool,
}

pub fn run_selftest(
    docs: &[(String, String)],
    embedder: &dyn crate::embed::VectorEmbed,
    probes: &[SelftestProbe],
    budget: usize,
    floor: f32,
    reserve: crate::map::ObserverReserve,
) -> anyhow::Result<SelftestReport> {
    anyhow::ensure!(!docs.is_empty(), "no corpus documents to selftest against");
    anyhow::ensure!(!probes.is_empty(), "no probes supplied");
    let mut reports = Vec::with_capacity(probes.len());
    for probe in probes {
        // Baseline: no observer claims. A probe whose phrase the engine
        // already retrieves is not a valid held-out probe.
        let baseline = crate::map::compile_context_with_observer(
            docs,
            embedder,
            &probe.query,
            budget,
            &[],
            floor,
            reserve,
        )?;
        let fused = crate::map::compile_context_with_observer(
            docs,
            embedder,
            &probe.query,
            budget,
            &probe.claims,
            floor,
            reserve,
        )?;
        let base_ctx = baseline.physis_context.to_lowercase();
        let fused_ctx = fused.physis_context.to_lowercase();
        let phrase = probe.phrase.to_lowercase();
        let baseline_absent = !base_ctx.contains(&phrase);
        let answered = fused_ctx.contains(&phrase);
        let forbidden_absent = probe
            .forbidden
            .iter()
            .all(|f| !fused_ctx.contains(&f.to_lowercase()));
        let passed = baseline_absent
            && forbidden_absent
            && match probe.expect {
                Expect::Answer => answered,
                Expect::Defer => !answered,
            };
        reports.push(SelftestProbeReport {
            name: probe.name.clone(),
            baseline_absent,
            answered,
            forbidden_absent,
            observer_selected: fused.observer_selected,
            engine_best_score: fused.engine_best_score,
            passed,
            relevances: fused
                .observer_gated
                .iter()
                .map(|r| (r.object_id.clone(), r.relevance.unwrap_or(0.0)))
                .collect(),
        });
    }
    let passed_count = reports.iter().filter(|r| r.passed).count();
    let total = reports.len();
    Ok(SelftestReport {
        corpus_documents: docs.len(),
        floor,
        probes: reports,
        passed: passed_count,
        total,
        ok: passed_count == total,
    })
}

/// One difference between a baseline selftest report and the live run.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SelftestDrift {
    /// A per-probe decision changed value.
    DecisionFlip {
        probe: String,
        field: &'static str,
        from: bool,
        to: bool,
    },
    /// A claim's cosine moved more than the drift tolerance.
    RelevanceDrift {
        probe: String,
        object_id: String,
        from: f32,
        to: f32,
    },
    /// The engine retrieval's own best chunk score moved beyond the
    /// tolerance — the corpus-side signal. Claim relevances are
    /// query↔claim cosines and cannot see corpus edits; this can.
    EngineBestDrift { probe: String, from: f32, to: f32 },
}

/// Cosine drift below this is embedder noise, not a corpus/model change.
pub const RELEVANCE_DRIFT_EPSILON: f32 = 0.01;

/// Diff a baseline selftest report against the live run. Pure function —
/// the CLI only prints and exits on its result.
pub fn diff_selftest_reports(
    baseline: &SelftestReport,
    current: &SelftestReport,
) -> Vec<SelftestDrift> {
    let mut drifts = Vec::new();
    for cur in &current.probes {
        let Some(base) = baseline.probes.iter().find(|p| p.name == cur.name) else {
            // A probe present now but absent from the baseline is new, not
            // drift — the baseline was taken with a different probe set.
            continue;
        };
        for (field, b, c) in [
            ("baseline_absent", base.baseline_absent, cur.baseline_absent),
            ("answered", base.answered, cur.answered),
            (
                "forbidden_absent",
                base.forbidden_absent,
                cur.forbidden_absent,
            ),
            ("passed", base.passed, cur.passed),
        ] {
            if b != c {
                drifts.push(SelftestDrift::DecisionFlip {
                    probe: cur.name.clone(),
                    field,
                    from: b,
                    to: c,
                });
            }
        }
        for (oid, cur_score) in &cur.relevances {
            match base.relevances.get(oid) {
                Some(base_score) if (base_score - cur_score).abs() > RELEVANCE_DRIFT_EPSILON => {
                    drifts.push(SelftestDrift::RelevanceDrift {
                        probe: cur.name.clone(),
                        object_id: oid.clone(),
                        from: *base_score,
                        to: *cur_score,
                    });
                }
                Some(_) => {}
                None => drifts.push(SelftestDrift::RelevanceDrift {
                    probe: cur.name.clone(),
                    object_id: oid.clone(),
                    from: f32::NAN,
                    to: *cur_score,
                }),
            }
        }
        // Corpus-side drift: claim relevances are query↔claim cosines and
        // are blind to corpus edits; the engine's best chunk score is not.
        if let (Some(b), Some(c)) = (base.engine_best_score, cur.engine_best_score) {
            if (b - c).abs() > RELEVANCE_DRIFT_EPSILON {
                drifts.push(SelftestDrift::EngineBestDrift {
                    probe: cur.name.clone(),
                    from: b,
                    to: c,
                });
            }
        }
        for oid in base.relevances.keys() {
            if !cur.relevances.contains_key(oid) {
                drifts.push(SelftestDrift::RelevanceDrift {
                    probe: cur.name.clone(),
                    object_id: oid.clone(),
                    from: base.relevances[oid],
                    to: f32::NAN,
                });
            }
        }
    }
    drifts
}

/// Per-corpus floor calibration: measured bands and an advisory floor
/// recommendation from the caller's own corpus and labelled probe set.
/// Every number here is a measurement receipt, never a directive — the
/// project rule is that advisory scores are not trusted, only read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationReport {
    /// (min, max, n) of the labelled relevant claims' cosines.
    pub relevant_band: (f32, f32, usize),
    /// (min, max, n) of the labelled noise claims' cosines.
    pub noise_band: (f32, f32, usize),
    /// The highest observed noise cosine.
    pub noise_ceiling: f32,
    /// noise_ceiling + 1e-3: the lowest floor with zero measured noise.
    pub minimal_zero_noise_floor: f32,
    /// noise_ceiling + 0.008 — the margin the shipped 0.66 default keeps
    /// over its own measured noise ceiling (0.6522). Advisory.
    pub recommended_floor: f32,
    /// Relevant claims admitted at the recommended floor.
    pub recall_at_recommended: (usize, usize),
    /// False when the bands overlap so badly that no floor separates them
    /// (e.g. the random-projection embedder on real text).
    pub separable: bool,
    /// (floor, relevant admitted, noise admitted) at fixed steps.
    pub roc: Vec<(f32, usize, usize)>,
}

/// Compute the calibration from labelled cosines. Pure function.
pub fn calibrate_from_relevances(
    relevant: &[f32],
    noise: &[f32],
) -> anyhow::Result<CalibrationReport> {
    anyhow::ensure!(!relevant.is_empty(), "no labelled relevant claims");
    anyhow::ensure!(!noise.is_empty(), "no labelled noise claims");
    let band = |v: &[f32]| {
        let min = v.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        (min, max)
    };
    let (rel_min, rel_max) = band(relevant);
    let (noi_min, noi_max) = band(noise);
    let noise_ceiling = noi_max;
    let minimal_zero_noise_floor = noise_ceiling + 1e-3;
    // The shipped 0.66 default keeps 0.0078 of margin over its own measured
    // noise ceiling (0.6522 on the n22 corpus). 0.008 is that convention.
    let recommended_floor = noise_ceiling + 0.008;
    let separable = rel_max > noise_ceiling;
    let count_at = |v: &[f32], floor: f32| v.iter().filter(|&&x| x >= floor).count();
    let roc = (50..=100)
        .map(|i| {
            let f = i as f32 / 100.0;
            (f, count_at(relevant, f), count_at(noise, f))
        })
        .collect();
    Ok(CalibrationReport {
        relevant_band: (rel_min, rel_max, relevant.len()),
        noise_band: (noi_min, noi_max, noise.len()),
        noise_ceiling,
        minimal_zero_noise_floor,
        recommended_floor,
        recall_at_recommended: (count_at(relevant, recommended_floor), relevant.len()),
        separable,
        roc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    /// Same shape as the test helpers in map.rs/observer_eval.rs: a retrieval
    /// claim packet with the given object id, snippet and score.
    fn retrieval_claim(
        object_id: &str,
        snippet: &str,
        score: f32,
    ) -> crate::ingest::CandidateClaimPacket {
        crate::ingest::CandidateClaimPacket {
            id: format!("claim:{object_id}"),
            intake_id: Some("i1".into()),
            source: crate::ingest::ClaimSource {
                uri: "doc://observer".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: crate::ingest::ExtractorInfo::new("test-observer"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(score),
            payload: crate::ingest::ClaimPayload::Retrieval(crate::ingest::RetrievalHit {
                object_id: object_id.into(),
                score,
                strategy: Some("vec".into()),
                snippet: Some(snippet.into()),
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        }
    }

    // Same fixture corpus and claim family as
    // `context_fusion_admits_the_relevant_observer_hit_and_defers_noise`
    // (map.rs), whose measured RP relevances are: additive snippet ~0.93,
    // noise 0.892/0.858 — floor 0.9 separates them, floor 0.5 does not.
    fn fixture_docs() -> Vec<(String, String)> {
        [
            "relay.md",
            "relay calibration requires the bench harness and a reference meter",
            "verify.md",
            "relay calibration two pass verification catches swapped contacts",
            "other.md",
            "shift handover notes filed weekly",
        ]
        .chunks(2)
        .map(|c| (c[0].to_string(), c[1].to_string()))
        .collect()
    }

    fn fixture_probe() -> SelftestProbe {
        SelftestProbe {
            name: "relay-roster".into(),
            expect: Expect::Answer,
            query: "relay calibration bench harness".into(),
            phrase: "harness, reference meter, two-pass".into(),
            forbidden: vec!["cafeteria lunch menu".into()],
            claims: vec![
                retrieval_claim(
                    "obj:relay-procedure",
                    "relay calibration: bench harness, reference meter, two-pass verification",
                    0.9,
                ),
                retrieval_claim("obj:noise-menu", "cafeteria lunch menu rotates daily", 0.1),
                retrieval_claim(
                    "obj:noise-weather",
                    "the courtyard fountain runs from nine to five",
                    0.1,
                ),
            ],
        }
    }

    #[test]
    fn selftest_passes_at_the_separating_floor_and_fails_when_noise_leaks() {
        let embedder = RandomProjectionEmbedder::new(64);
        let docs = fixture_docs();

        // Floor 0.9 (the fixture's measured separating floor): the relevant
        // claim is admitted, both noise claims defer, the phrase is present,
        // the forbidden text is absent, and the baseline packet cannot answer.
        let good = run_selftest(
            &docs,
            &embedder,
            std::slice::from_ref(&fixture_probe()),
            400,
            0.9,
            crate::map::ObserverReserve::Auto,
        )
        .unwrap();
        assert_eq!(good.total, 1);
        assert!(good.ok, "selftest must pass at the separating floor");
        let p = &good.probes[0];
        assert!(p.baseline_absent, "held-out phrase must be absent baseline");
        assert!(p.answered, "the gate must admit the relevant claim");
        assert!(
            p.forbidden_absent,
            "noise text must not reach the packet at floor 0.9"
        );

        // Floor 0.5: the noise claims clear the floor too; with auto reserve
        // their text enters the packet — the selftest must FAIL, not pass.
        let bad = run_selftest(
            &docs,
            &embedder,
            std::slice::from_ref(&fixture_probe()),
            400,
            0.5,
            crate::map::ObserverReserve::Auto,
        )
        .unwrap();
        assert!(
            !bad.ok,
            "selftest must fail when noise text enters a packet"
        );
        let p = &bad.probes[0];
        assert!(
            !p.forbidden_absent,
            "noise text is in the packet at floor 0.5"
        );
        assert!(!p.passed);
    }

    #[test]
    fn selftest_fails_when_the_gate_defers_a_should_pass_probe() {
        // Floor 0.99 under the RP embedder: nothing is admitted, the probe's
        // phrase never reaches the packet — the selftest must report the
        // failure rather than pass silently.
        let embedder = RandomProjectionEmbedder::new(64);
        let report = run_selftest(
            &fixture_docs(),
            &embedder,
            std::slice::from_ref(&fixture_probe()),
            400,
            0.99,
            crate::map::ObserverReserve::Auto,
        )
        .unwrap();
        assert!(!report.ok, "a deferred should-pass probe is a failure");
        assert!(!report.probes[0].answered);
        assert!(!report.probes[0].passed);
    }

    #[test]
    fn selftest_rejects_a_probe_whose_answer_the_engine_already_retrieves() {
        // A "held-out" phrase that the baseline packet already contains proves
        // nothing; the probe must be marked failed (baseline_absent false).
        let embedder = RandomProjectionEmbedder::new(64);
        let mut probe = fixture_probe();
        probe.phrase = "bench harness".into(); // present in corpus doc relay.md
        let report = run_selftest(
            &fixture_docs(),
            &embedder,
            std::slice::from_ref(&probe),
            400,
            0.9,
            crate::map::ObserverReserve::Auto,
        )
        .unwrap();
        assert!(!report.ok, "a corpus-covered probe is not a valid selftest");
        assert!(!report.probes[0].baseline_absent);
        assert!(!report.probes[0].passed);
    }

    fn report_with(name: &str, answered: bool, relevances: &[(&str, f32)]) -> SelftestReport {
        SelftestReport {
            corpus_documents: 3,
            floor: 0.9,
            probes: vec![SelftestProbeReport {
                name: name.into(),
                baseline_absent: true,
                answered,
                forbidden_absent: true,
                observer_selected: usize::from(answered),
                engine_best_score: Some(0.7),
                passed: answered,
                relevances: relevances
                    .iter()
                    .map(|(k, v)| (k.to_string(), *v))
                    .collect(),
            }],
            passed: usize::from(answered),
            total: 1,
            ok: answered,
        }
    }

    #[test]
    fn calibration_places_the_floor_in_the_measured_gap() {
        let rep = calibrate_from_relevances(&[0.70, 0.55], &[0.65, 0.40]).unwrap();
        assert!((rep.noise_ceiling - 0.65).abs() < 1e-6);
        assert!((rep.minimal_zero_noise_floor - 0.651).abs() < 1e-6);
        assert!((rep.recommended_floor - 0.658).abs() < 1e-6);
        assert!(rep.separable);
        // at the recommendation, 1/2 relevant admitted, 0 noise
        assert_eq!(rep.recall_at_recommended, (1, 2));
    }

    #[test]
    fn calibration_reports_inseparable_bands_instead_of_nonsense() {
        // noise ceiling above the relevant maximum: no floor separates them
        let rep = calibrate_from_relevances(&[0.60, 0.55], &[0.65, 0.40]).unwrap();
        assert!(!rep.separable);
    }

    #[test]
    fn calibration_roc_counts_admissions_at_each_step() {
        let rep = calibrate_from_relevances(&[0.70, 0.55], &[0.65, 0.40]).unwrap();
        let at = |f: f32| {
            rep.roc
                .iter()
                .find(|(fl, _, _)| (*fl - f).abs() < 1e-6)
                .unwrap()
        };
        let (_, rel, noi) = at(0.60);
        assert_eq!((*rel, *noi), (1, 1));
        let (_, rel, noi) = at(0.66);
        assert_eq!((*rel, *noi), (1, 0));
    }

    #[test]
    fn defer_expected_probes_pass_when_the_gate_defers_and_fail_when_it_answers() {
        let embedder = RandomProjectionEmbedder::new(64);
        let mut probe = fixture_probe();
        probe.expect = Expect::Defer;
        // Floor 0.9: the gate admits the claim — the probe expected a defer,
        // so this is a failure (behaviour changed from the measurement).
        let rep = run_selftest(
            &fixture_docs(),
            &embedder,
            std::slice::from_ref(&probe),
            400,
            0.9,
            crate::map::ObserverReserve::Auto,
        )
        .unwrap();
        assert!(!rep.ok, "answered a defer-expected probe: behaviour drift");
        assert!(!rep.probes[0].passed);
        // Floor 0.99: the gate defers — matches the expectation.
        let rep = run_selftest(
            &fixture_docs(),
            &embedder,
            std::slice::from_ref(&probe),
            400,
            0.99,
            crate::map::ObserverReserve::Auto,
        )
        .unwrap();
        assert!(rep.ok, "deferred as expected: matches the measurement");
        assert!(rep.probes[0].passed);
        assert!(!rep.probes[0].answered);
    }

    #[test]
    fn drift_diff_is_empty_for_identical_reports() {
        let a = report_with("p", true, &[("obj:x", 0.72)]);
        let b = report_with("p", true, &[("obj:x", 0.72)]);
        assert!(diff_selftest_reports(&a, &b).is_empty());
    }

    #[test]
    fn drift_diff_reports_decision_flips_and_relevance_moves() {
        let base = report_with("p", true, &[("obj:x", 0.72)]);
        // decision flip: `passed` is derived from the three decisions, so it
        // co-flips with `answered` — both must be reported.
        let flipped = report_with("p", false, &[("obj:x", 0.72)]);
        let d = diff_selftest_reports(&base, &flipped);
        assert_eq!(d.len(), 2, "answered + derived passed flip: {d:?}");
        assert!(d
            .iter()
            .all(|x| matches!(x, SelftestDrift::DecisionFlip { .. })));
        assert!(d.iter().any(|x| matches!(
            x,
            SelftestDrift::DecisionFlip {
                field: "answered",
                ..
            }
        )));
        // relevance move beyond the epsilon
        let moved = report_with("p", true, &[("obj:x", 0.75)]);
        let d = diff_selftest_reports(&base, &moved);
        assert_eq!(d.len(), 1, "one relevance drift: {d:?}");
        assert!(
            matches!(&d[0], SelftestDrift::RelevanceDrift { from, to, .. }
            if (*from - 0.72).abs() < 1e-6 && (*to - 0.75).abs() < 1e-6)
        );
        // a 0.005 move is embedder noise, not drift
        let tiny = report_with("p", true, &[("obj:x", 0.725)]);
        assert!(diff_selftest_reports(&base, &tiny).is_empty());
    }

    #[test]
    fn drift_diff_sees_corpus_edits_through_engine_best() {
        // Claim relevances are query<->claim cosines: corpus edits do not
        // move them. The engine's best chunk score is the corpus signal.
        let mut base = report_with("p", true, &[("obj:x", 0.72)]);
        base.probes[0].engine_best_score = Some(0.70);
        let mut edited = report_with("p", true, &[("obj:x", 0.72)]);
        edited.probes[0].engine_best_score = Some(0.75);
        let d = diff_selftest_reports(&base, &edited);
        assert_eq!(d.len(), 1, "engine-best move is drift: {d:?}");
        assert!(
            matches!(&d[0], SelftestDrift::EngineBestDrift { from, to, .. }
            if (*from - 0.70).abs() < 1e-6 && (*to - 0.75).abs() < 1e-6)
        );
        // within epsilon: not drift
        edited.probes[0].engine_best_score = Some(0.705);
        assert!(diff_selftest_reports(&base, &edited).is_empty());
    }

    #[test]
    fn drift_diff_reports_a_claim_that_appeared_or_vanished() {
        let base = report_with("p", true, &[("obj:x", 0.72)]);
        let gained = report_with("p", true, &[("obj:x", 0.72), ("obj:y", 0.60)]);
        let d = diff_selftest_reports(&base, &gained);
        assert_eq!(d.len(), 1, "new claim is drift: {d:?}");
        let lost = report_with("p", true, &[]);
        let d = diff_selftest_reports(&base, &lost);
        assert_eq!(d.len(), 1, "vanished claim is drift: {d:?}");
    }
}
