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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelftestProbe {
    pub name: String,
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

#[derive(Debug, Clone, Serialize)]
pub struct SelftestProbeReport {
    pub name: String,
    pub baseline_absent: bool,
    pub answered: bool,
    pub forbidden_absent: bool,
    pub observer_selected: usize,
    pub engine_best_score: Option<f32>,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
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
        let passed = baseline_absent && answered && forbidden_absent;
        reports.push(SelftestProbeReport {
            name: probe.name.clone(),
            baseline_absent,
            answered,
            forbidden_absent,
            observer_selected: fused.observer_selected,
            engine_best_score: fused.engine_best_score,
            passed,
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
}
