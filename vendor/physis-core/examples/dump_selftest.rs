//! Dump the selftest fixture reports from physis-core as the C oracle.
//! Prints, for each scenario of the four anchored Rust tests, the full
//! SelftestReport as JSON plus the fused context and per-claim relevances
//! as exact f32 hex bits.

use physis_core::embed::RandomProjectionEmbedder;
use physis_core::ingest::{CandidateClaimPacket, ClaimPayload, ClaimSource, ExtractorInfo, RetrievalHit};
use physis_core::selftest::{run_selftest, Expect, SelftestProbe};
use physis_core::map::ObserverReserve;

fn retrieval_claim(object_id: &str, snippet: &str, score: f32) -> CandidateClaimPacket {
    CandidateClaimPacket {
        id: format!("claim:{object_id}"),
        intake_id: Some("i1".into()),
        source: ClaimSource {
            uri: "doc://observer".into(),
            span: Some("0..1".into()),
            kind: None,
        },
        extractor: ExtractorInfo::new("test-observer"),
        asserted_at: None,
        observed_at: None,
        raw_ref: Some("r".into()),
        confidence: Some(score),
        payload: ClaimPayload::Retrieval(RetrievalHit {
            object_id: object_id.into(),
            score,
            strategy: Some("vec".into()),
            snippet: Some(snippet.into()),
        }),
        lossy_fields: vec![],
        extras: Default::default(),
    }
}

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

fn f32hex(v: f32) -> String {
    format!("{:08x}", v.to_bits())
}

fn dump(tag: &str, rep: &physis_core::selftest::SelftestReport, ctx: &str) {
    println!("== {tag} ==");
    println!("ok {}", rep.ok);
    println!("passed {} total {}", rep.passed, rep.total);
    println!("corpus_documents {} floor {} floorhex {}", rep.corpus_documents, rep.floor, f32hex(rep.floor));
    for p in &rep.probes {
        println!(
            "probe {} baseline_absent {} answered {} forbidden_absent {} observer_selected {} passed {} engine_best {} engine_besthex {}",
            p.name, p.baseline_absent, p.answered, p.forbidden_absent, p.observer_selected, p.passed,
            match p.engine_best_score { Some(v) => f32hex(v), None => "none".into() },
            match p.engine_best_score { Some(v) => f32hex(v), None => "none".into() }
        );
        for (oid, rel) in &p.relevances {
            println!("rel {oid} {} {}", f32hex(*rel), rel);
        }
    }
    println!("--- context ---");
    println!("{ctx}");
    println!("--- end ---");
}

fn main() {
    // JSON mode for the parity tool: the six selftest scenarios as one
    // JSON document, floats as exact f32 hex bits — and nothing else on
    // stdout, so the receipt is machine-parseable.
    if std::env::args().any(|a| a == "--json") {
        let embedder = RandomProjectionEmbedder::new(64);
        let docs = fixture_docs();
        let probe = fixture_probe();
        let mut probe3 = fixture_probe();
        probe3.phrase = "bench harness".into();
        let mut probe4 = fixture_probe();
        probe4.expect = Expect::Defer;
        let scenarios: Vec<(&str, &SelftestProbe, usize, f32)> = vec![
            ("floor0.9", &probe, 400, 0.9),
            ("floor0.5", &probe, 400, 0.5),
            ("floor0.99", &probe, 400, 0.99),
            ("phrase-covered", &probe3, 400, 0.9),
            ("defer-floor0.9", &probe4, 400, 0.9),
            ("defer-floor0.99", &probe4, 400, 0.99),
        ];
        let docs_json: Vec<serde_json::Value> = scenarios.iter().map(|(tag, p, budget, floor)| {
            let rep = run_selftest(&docs, &embedder, std::slice::from_ref(*p), *budget, *floor, ObserverReserve::Auto).unwrap();
            serde_json::json!({"tag": tag, "report": report_json(&rep)})
        }).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "kind": "selftest_reports",
            "scenarios": docs_json
        })).unwrap());
        return;
    }
    let embedder = RandomProjectionEmbedder::new(64);
    let docs = fixture_docs();

    // scenario 1a: floor 0.9 (separating floor)
    let rep = run_selftest(&docs, &embedder, std::slice::from_ref(&fixture_probe()), 400, 0.9, ObserverReserve::Auto).unwrap();
    // recompute the fused context for display
    let probe = fixture_probe();
    let claims: Vec<CandidateClaimPacket> = probe.claims.clone();
    let fused = physis_core::map::compile_context_with_observer(&docs, &embedder, &probe.query, 400, &claims, 0.9, ObserverReserve::Auto).unwrap();
    dump("floor0.9", &rep, &fused.physis_context);

    // scenario 1b: floor 0.5 (noise leaks)
    let rep = run_selftest(&docs, &embedder, std::slice::from_ref(&fixture_probe()), 400, 0.5, ObserverReserve::Auto).unwrap();
    let fused = physis_core::map::compile_context_with_observer(&docs, &embedder, &probe.query, 400, &claims, 0.5, ObserverReserve::Auto).unwrap();
    dump("floor0.5", &rep, &fused.physis_context);

    // scenario 2: floor 0.99 (gate defers a should-pass probe)
    let rep = run_selftest(&docs, &embedder, std::slice::from_ref(&fixture_probe()), 400, 0.99, ObserverReserve::Auto).unwrap();
    let fused = physis_core::map::compile_context_with_observer(&docs, &embedder, &probe.query, 400, &claims, 0.99, ObserverReserve::Auto).unwrap();
    dump("floor0.99", &rep, &fused.physis_context);

    // scenario 3: phrase the baseline already retrieves
    let mut probe3 = fixture_probe();
    probe3.phrase = "bench harness".into();
    let rep = run_selftest(&docs, &embedder, std::slice::from_ref(&probe3), 400, 0.9, ObserverReserve::Auto).unwrap();
    let fused = physis_core::map::compile_context_with_observer(&docs, &embedder, &probe3.query, 400, &claims, 0.9, ObserverReserve::Auto).unwrap();
    dump("phrase-covered", &rep, &fused.physis_context);

    // scenario 4: expect defer, floors 0.9 and 0.99
    let mut probe4 = fixture_probe();
    probe4.expect = Expect::Defer;
    let rep = run_selftest(&docs, &embedder, std::slice::from_ref(&probe4), 400, 0.9, ObserverReserve::Auto).unwrap();
    dump("defer-floor0.9", &rep, "");
    let rep = run_selftest(&docs, &embedder, std::slice::from_ref(&probe4), 400, 0.99, ObserverReserve::Auto).unwrap();
    dump("defer-floor0.99", &rep, "");

    // tight-budget scenarios that exercise the reserve trade
    dump_ctx("tight-b64-auto", 64, 0.9, ObserverReserve::Auto, 0);
    dump_ctx("tight-b64-none", 64, 0.9, ObserverReserve::None, 0);
    dump_ctx("tight-b64-fixed40", 64, 0.9, ObserverReserve::Fixed(40), 40);

    // auto_reserve_sizes_the_trade_exactly_and_avoids_starving_the_engine
    let long_doc = "relay calibration requires the bench harness and a reference meter \
two pass verification catches swapped contacts and the reference meter readings \
are logged each run for the calibration record";
    let other_doc = "shift handover notes are filed weekly and the coolant pump filter \
is replaced every quarter while the invoice reconciliation runs";
    let docs2 = vec![
        ("relay.md".to_string(), long_doc.to_string()),
        ("other.md".to_string(), other_doc.to_string()),
    ];
    let claims2 = vec![
        retrieval_claim(
            "obj:relay-procedure",
            "relay calibration bench harness reference meter",
            0.9,
        ),
        retrieval_claim("obj:noise-menu", "cafeteria lunch menu rotates daily", 0.1),
    ];
    for (tag, reserve) in [("auto-b40", ObserverReserve::Auto), ("fixed40-b40", ObserverReserve::Fixed(40))] {
        let r = physis_core::map::compile_context_with_observer(&docs2, &embedder, "relay calibration bench harness", 40, &claims2, 0.9, reserve).unwrap();
        println!("== {tag} ==");
        println!("observer_selected {} physis_tokens {}", r.observer_selected, r.physis_tokens);
        println!("contains_harness_meter {}", r.physis_context.contains("bench harness and a reference meter"));
        println!("contains_observer {}", r.physis_context.contains("observer: obj:relay-procedure"));
        println!("--- context ---");
        println!("{}", r.physis_context);
        println!("--- end ---");
    }
    // reserve pricing: auto with both claims vs accepted only
    let a2 = physis_core::map::observer_reserve_tokens(&embedder, "relay calibration bench harness", &claims2, 0.9, ObserverReserve::Auto, |c, s| physis_core::rag::count_tokens(&format!("

--- observer: {} ({}) ---
{}", match &c.payload { physis_core::ingest::ClaimPayload::Retrieval(h) => h.object_id.clone(), _ => c.id.clone() }, c.extractor.id, s.trim())));
    let a1 = physis_core::map::observer_reserve_tokens(&embedder, "relay calibration bench harness", &claims2[..1], 0.9, ObserverReserve::Auto, |c, s| physis_core::rag::count_tokens(&format!("

--- observer: {} ({}) ---
{}", match &c.payload { physis_core::ingest::ClaimPayload::Retrieval(h) => h.object_id.clone(), _ => c.id.clone() }, c.extractor.id, s.trim())));
    let n0 = physis_core::map::observer_reserve_tokens(&embedder, "relay calibration", &claims2, 0.9, ObserverReserve::None, |c, s| physis_core::rag::count_tokens(&format!("

--- observer: {} ({}) ---
{}", match &c.payload { physis_core::ingest::ClaimPayload::Retrieval(h) => h.object_id.clone(), _ => c.id.clone() }, c.extractor.id, s.trim())));
    let f7 = physis_core::map::observer_reserve_tokens(&embedder, "relay calibration", &claims2, 0.9, ObserverReserve::Fixed(7), |c, s| physis_core::rag::count_tokens(&format!("

--- observer: {} ({}) ---
{}", match &c.payload { physis_core::ingest::ClaimPayload::Retrieval(h) => h.object_id.clone(), _ => c.id.clone() }, c.extractor.id, s.trim())));
    println!("== reserve-pricing ==");
    println!("auto2 {a2} auto1 {a1} none {n0} fixed7 {f7}");

    // split_document: a document larger than the budget must be chunked
    // into #partN pieces or it can never be selected
    let big = (1..=30).map(|i| format!("word{} filler{} token{}", i, i, i)).collect::<Vec<_>>().join(" and ");
    let docs3 = vec![("big.md".to_string(), big.clone()), ("small.md".to_string(), "tiny note".to_string())];
    let r3 = physis_core::map::compile_context_with_observer(&docs3, &embedder, "word7 filler7", 64, &[], 0.9, ObserverReserve::None).unwrap();
    println!("== split-chunk ==");
    println!("big_tokens {} physis_tokens {} source_documents {}", physis_core::rag::count_tokens(&big), r3.physis_tokens, r3.source_documents);
    println!("context_has_part {}", r3.physis_context.contains("#part"));
    println!("--- context ---");
    println!("{}", r3.physis_context);
    println!("--- end ---");
}
// appended: tight-budget scenarios that exercise the reserve trade
fn dump_ctx(tag: &str, budget: usize, floor: f32, reserve: ObserverReserve, fixed: usize) {
    let embedder = RandomProjectionEmbedder::new(64);
    let docs = fixture_docs();
    let probe = fixture_probe();
    let claims: Vec<CandidateClaimPacket> = probe.claims.clone();
    let fused = physis_core::map::compile_context_with_observer(&docs, &embedder, &probe.query, budget, &claims, floor, reserve).unwrap();
    let base = physis_core::map::compile_context_with_observer(&docs, &embedder, &probe.query, budget, &[], floor, reserve).unwrap();
    println!("== {tag} ==");
    println!("baseline_tokens {} physis_tokens {} observer_selected {} engine_best {} engine_besthex {}",
        base.baseline_tokens, fused.physis_tokens, fused.observer_selected,
        match fused.engine_best_score { Some(v) => f32hex(v), None => "none".into() },
        match fused.engine_best_score { Some(v) => f32hex(v), None => "none".into() });
    println!("repeats {} differences {} contradictions {}", fused.repeats, fused.differences, fused.contradictions);
    println!("clusters {:?}", fused.clusters);
    for r in &fused.observer_gated {
        println!("receipt {} decision {:?} included {} rel {} relhex {} engrel {} engrelhex {}",
            r.object_id, r.decision, r.included,
            match r.relevance { Some(v) => f32hex(v), None => "none".into() },
            match r.relevance { Some(v) => f32hex(v), None => "none".into() },
            match r.engine_relative { Some(v) => f32hex(v), None => "none".into() },
            match r.engine_relative { Some(v) => f32hex(v), None => "none".into() });
    }
    println!("--- context ---");
    println!("{}", fused.physis_context);
    println!("--- end ---");
}

// at the end of main:

// appended: JSON helpers for the parity tool (serde_json builds the
// document; floats go in as exact f32 hex-bit strings)
fn report_json(rep: &physis_core::selftest::SelftestReport) -> serde_json::Value {
    let probes: Vec<serde_json::Value> = rep
        .probes
        .iter()
        .map(|p| {
            let rels: Vec<serde_json::Value> = p
                .relevances
                .iter()
                .map(|(oid, r)| {
                    serde_json::json!({
                        "object_id": oid,
                        "hex": format!("{:08x}", r.to_bits())
                    })
                })
                .collect();
            serde_json::json!({
                "name": p.name,
                "baseline_absent": p.baseline_absent,
                "answered": p.answered,
                "forbidden_absent": p.forbidden_absent,
                "observer_selected": p.observer_selected,
                "passed": p.passed,
                "engine_best_hex": p.engine_best_score
                    .map(|v| format!("{:08x}", v.to_bits())),
                "relevances": rels
            })
        })
        .collect();
    serde_json::json!({
        "corpus_documents": rep.corpus_documents,
        "floor_hex": format!("{:08x}", rep.floor.to_bits()),
        "passed": rep.passed,
        "total": rep.total,
        "ok": rep.ok,
        "probes": probes
    })
}
