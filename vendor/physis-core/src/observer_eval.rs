//! Deterministic evaluation stubs for observer-assisted vs baseline flows.
//!
//! Pure, in-crate helpers over in-memory fixtures (no IO, no network, no models).
//! These support evidence-gated comparisons (counts, proxies, yield) but do not
//! themselves claim wins — measurements must be interpreted against baselines
//! and construction-matched nulls.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::compile::CompiledContext;
use crate::gate::{
    GateDecision, GateOutcome, LessonGateResult, RelationGateResult, ResolutionGateResult,
};

/// Count resolution-gate outcomes (accept/defer/reject/abstain).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionOutcomeCounts {
    #[serde(default)]
    pub accept: usize,
    #[serde(default)]
    pub defer: usize,
    #[serde(default)]
    pub reject: usize,
    #[serde(default)]
    pub abstain: usize,
    #[serde(default)]
    pub contradictions: usize,
}

/// Tally outcomes of equivalence/resolution gate results.
pub fn resolution_outcomes(results: &[ResolutionGateResult]) -> ResolutionOutcomeCounts {
    let mut c = ResolutionOutcomeCounts::default();
    for r in results {
        match r.decision {
            GateDecision::AcceptCandidate => c.accept += 1,
            GateDecision::Defer => c.defer += 1,
            GateDecision::Reject => c.reject += 1,
            GateDecision::Abstain => c.abstain += 1,
        }
        if r.contradiction.is_some() {
            c.contradictions += 1;
        }
    }
    c
}

/// Count relation-gate outcomes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimpleOutcomeCounts {
    #[serde(default)]
    pub accept: usize,
    #[serde(default)]
    pub defer: usize,
    #[serde(default)]
    pub reject: usize,
    #[serde(default)]
    pub abstain: usize,
}

/// Tally outcomes of relation gate results.
pub fn relation_outcomes(results: &[RelationGateResult]) -> SimpleOutcomeCounts {
    let mut c = SimpleOutcomeCounts::default();
    for r in results {
        match r.decision {
            GateDecision::AcceptCandidate => c.accept += 1,
            GateDecision::Defer => c.defer += 1,
            GateDecision::Reject => c.reject += 1,
            GateDecision::Abstain => c.abstain += 1,
        }
    }
    c
}

/// Tally outcomes of lesson gate results.
pub fn lesson_outcomes(results: &[LessonGateResult]) -> SimpleOutcomeCounts {
    let mut c = SimpleOutcomeCounts::default();
    for r in results {
        match r.decision {
            GateDecision::AcceptCandidate => c.accept += 1,
            GateDecision::Defer => c.defer += 1,
            GateDecision::Reject => c.reject += 1,
            GateDecision::Abstain => c.abstain += 1,
        }
    }
    c
}

/// Tally outcomes of grounding gate results (mapped candidate vs unmapped
/// discovery/coverage defer etc.), for deterministic observer/grounding
/// ablation accounting. Counts/proxies only — never a truth verdict.
pub fn grounding_outcomes(results: &[crate::gate::GroundingGateResult]) -> SimpleOutcomeCounts {
    let mut c = SimpleOutcomeCounts::default();
    for r in results {
        match r.decision {
            GateDecision::AcceptCandidate => c.accept += 1,
            GateDecision::Defer => c.defer += 1,
            GateDecision::Reject => c.reject += 1,
            GateDecision::Abstain => c.abstain += 1,
        }
    }
    c
}

/// Recall proxy: fraction of `relevant_ids` present in selected packet ids.
/// Returns 1.0 when `relevant_ids` is empty (vacuous), so callers should
/// check emptiness separately when that matters.
pub fn context_recall_proxy(packet: &CompiledContext, relevant_ids: &HashSet<String>) -> f32 {
    if relevant_ids.is_empty() {
        return 1.0;
    }
    let selected: HashSet<&str> = packet.selected.iter().map(|s| s.id.as_str()).collect();
    let hit = relevant_ids
        .iter()
        .filter(|id| selected.contains(id.as_str()))
        .count();
    hit as f32 / relevant_ids.len() as f32
}

/// How many selected items originated from an observer (unverified).
pub fn observer_selected_count(packet: &CompiledContext) -> usize {
    packet
        .selected
        .iter()
        .filter(|s| s.observer_sourced)
        .count()
}

/// Budget utilization ratio recorded in the packet (heuristic units).
pub fn utilization(packet: &CompiledContext) -> f32 {
    packet.stats.utilization
}

/// Deterministic comparison between a baseline and an observer-fused compile.
///
/// `relevant_ids`, when provided, yields a recall-proxy delta
/// (fused - baseline). Positive delta means the fused packet selected more of
/// the relevant set under the same budget; this is a proxy, not a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompileComparison {
    pub baseline_selected: usize,
    pub fused_selected: usize,
    pub baseline_observer_selected: usize,
    pub fused_observer_selected: usize,
    pub baseline_utilization: f32,
    pub fused_utilization: f32,
    pub recall_proxy_baseline: Option<f32>,
    pub recall_proxy_fused: Option<f32>,
    pub recall_proxy_delta: Option<f32>,
}

/// Compare two compiled packets (typically engine-only baseline vs fused).
pub fn compare_compiles(
    baseline: &CompiledContext,
    fused: &CompiledContext,
    relevant_ids: Option<&HashSet<String>>,
) -> CompileComparison {
    let (rb, rf, rd) = if let Some(rel) = relevant_ids {
        let b = context_recall_proxy(baseline, rel);
        let f = context_recall_proxy(fused, rel);
        (Some(b), Some(f), Some(f - b))
    } else {
        (None, None, None)
    };
    CompileComparison {
        baseline_selected: baseline.stats.selected_count,
        fused_selected: fused.stats.selected_count,
        baseline_observer_selected: observer_selected_count(baseline),
        fused_observer_selected: observer_selected_count(fused),
        baseline_utilization: baseline.stats.utilization,
        fused_utilization: fused.stats.utilization,
        recall_proxy_baseline: rb,
        recall_proxy_fused: rf,
        recall_proxy_delta: rd,
    }
}

/// Count how many lesson-gate results are both accepted and shadow-stable
/// (robustness probe reported no degradation). A coarse survival proxy.
pub fn stable_accepted_lessons(results: &[LessonGateResult]) -> usize {
    results
        .iter()
        .filter(|r| {
            r.decision == GateDecision::AcceptCandidate && r.robustness_stable == Some(true)
        })
        .count()
}

/// Count contradictions surfaced by resolution gates (conflict candidates).
pub fn resolution_contradiction_yield(results: &[ResolutionGateResult]) -> usize {
    results.iter().filter(|r| r.contradiction.is_some()).count()
}

/// Fraction of selected packet items that originated from an observer.
/// Returns 0.0 when nothing is selected.
pub fn observer_selection_ratio(packet: &CompiledContext) -> f32 {
    if packet.stats.selected_count == 0 {
        return 0.0;
    }
    observer_selected_count(packet) as f32 / packet.stats.selected_count as f32
}

/// Deterministic set-based precision/recall over string ids (generic).
///
/// - precision = |pred ∩ relevant| / |pred|  (1.0 if pred empty and relevant empty else 0.0 if pred empty)
/// - recall    = |pred ∩ relevant| / |relevant| (1.0 if relevant empty)
///
/// Returns `(precision, recall, f1, true_positive_count)`.
pub fn precision_recall_f1(
    predicted: &HashSet<String>,
    relevant: &HashSet<String>,
) -> (f32, f32, f32, usize) {
    let tp = predicted.intersection(relevant).count();
    let precision = if predicted.is_empty() {
        if relevant.is_empty() {
            1.0
        } else {
            0.0
        }
    } else {
        tp as f32 / predicted.len() as f32
    };
    let recall = if relevant.is_empty() {
        1.0
    } else {
        tp as f32 / relevant.len() as f32
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    (precision, recall, f1, tp)
}

/// Build a predicted-id set from relation-gate results whose decision is
/// `AcceptCandidate` (using proposed edge evidence/provenance-neutral identity
/// `source->target` as a comparable key). Useful as a coarse relation-yield
/// proxy in fixture benchmarks with matching keys.
pub fn accepted_relation_keys(results: &[RelationGateResult]) -> HashSet<String> {
    results
        .iter()
        .filter(|r| r.decision == GateDecision::AcceptCandidate)
        .map(|r| {
            format!(
                "{}->{}",
                r.proposed_edge.source_id, r.proposed_edge.target_id
            )
        })
        .collect()
}

/// Canonical undirected pair key (sorted, deterministic) for equivalence
/// false-merge/split scoring over labeled corpora.
pub fn canonical_pair_key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{}={}", a, b)
    } else {
        format!("{}={}", b, a)
    }
}

/// Precision/recall/F1 over undirected pair sets (false-merge/split proxy).
///
/// `predicted_pairs`/`gold_pairs` should contain canonical keys
/// (see [`canonical_pair_key`]). Returns `(precision, recall, f1, tp)`.
pub fn pair_precision_recall_f1(
    predicted_pairs: &HashSet<String>,
    gold_pairs: &HashSet<String>,
) -> (f32, f32, f32, usize) {
    precision_recall_f1(predicted_pairs, gold_pairs)
}

/// Build canonical predicted pair keys from equivalence gate results that were
/// accepted as candidate resolutions (proposal-only, never a merge).
pub fn accepted_equivalence_pair_keys(results: &[ResolutionGateResult]) -> HashSet<String> {
    results
        .iter()
        .filter(|r| r.decision == GateDecision::AcceptCandidate)
        .filter_map(|r| r.pair.as_ref().map(|(l, r)| canonical_pair_key(l, r)))
        .collect()
}

/// Tally decisions over typed gate outcomes (dispatch/assessment results).
pub fn gate_outcome_counts(outcomes: &[GateOutcome]) -> SimpleOutcomeCounts {
    let mut c = SimpleOutcomeCounts::default();
    for o in outcomes {
        match o.decision() {
            GateDecision::AcceptCandidate => c.accept += 1,
            GateDecision::Defer => c.defer += 1,
            GateDecision::Reject => c.reject += 1,
            GateDecision::Abstain => c.abstain += 1,
        }
    }
    c
}

/// Tally decisions over assessed intakes (intake + gate outcome pairs).
pub fn assessed_outcome_counts(assessed: &[crate::gate::AssessedIntake]) -> SimpleOutcomeCounts {
    let mut c = SimpleOutcomeCounts::default();
    for a in assessed {
        match a.outcome.decision() {
            GateDecision::AcceptCandidate => c.accept += 1,
            GateDecision::Defer => c.defer += 1,
            GateDecision::Reject => c.reject += 1,
            GateDecision::Abstain => c.abstain += 1,
        }
    }
    c
}

/// Per-kind outcome tallies for typed gate outcomes (dispatch/assessment
/// results). Deterministic fixture accounting (counts/proxies only, never a
/// verdict): relation/equivalence/lesson/grounding/claim buckets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOutcomeCountsByKind {
    #[serde(default)]
    pub relation: SimpleOutcomeCounts,
    #[serde(default)]
    pub equivalence: SimpleOutcomeCounts,
    #[serde(default)]
    pub lesson: SimpleOutcomeCounts,
    #[serde(default)]
    pub grounding: SimpleOutcomeCounts,
    /// Conservative top-level claim outcomes (Entity/Retrieval/Opaque payloads).
    #[serde(default)]
    pub claim: SimpleOutcomeCounts,
}

fn bump(counts: &mut SimpleOutcomeCounts, decision: GateDecision) {
    match decision {
        GateDecision::AcceptCandidate => counts.accept += 1,
        GateDecision::Defer => counts.defer += 1,
        GateDecision::Reject => counts.reject += 1,
        GateDecision::Abstain => counts.abstain += 1,
    }
}

/// Tally decisions over typed gate outcomes, broken down by outcome kind.
pub fn gate_outcomes_by_kind(outcomes: &[GateOutcome]) -> GateOutcomeCountsByKind {
    let mut out = GateOutcomeCountsByKind::default();
    for o in outcomes {
        match o {
            GateOutcome::Relation(r) => bump(&mut out.relation, r.decision),
            GateOutcome::Equivalence(r) => bump(&mut out.equivalence, r.decision),
            GateOutcome::Lesson(r) => bump(&mut out.lesson, r.decision),
            GateOutcome::Grounding(r) => bump(&mut out.grounding, r.decision),
            GateOutcome::Claim(r) => bump(&mut out.claim, r.decision),
        }
    }
    out
}

/// Tally decisions over assessed intakes, broken down by gate outcome kind.
pub fn assessed_outcomes_by_kind(
    assessed: &[crate::gate::AssessedIntake],
) -> GateOutcomeCountsByKind {
    let mut out = GateOutcomeCountsByKind::default();
    for a in assessed {
        match &a.outcome {
            GateOutcome::Relation(r) => bump(&mut out.relation, r.decision),
            GateOutcome::Equivalence(r) => bump(&mut out.equivalence, r.decision),
            GateOutcome::Lesson(r) => bump(&mut out.lesson, r.decision),
            GateOutcome::Grounding(r) => bump(&mut out.grounding, r.decision),
            GateOutcome::Claim(r) => bump(&mut out.claim, r.decision),
        }
    }
    out
}

/// Composed, deterministic summary of gate/assessed outcomes for a fixture or
/// batch (counts/proxies only, never a verdict). Serializable for harness
/// consumption/evidence capture.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateAssessmentSummary {
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub accept: usize,
    #[serde(default)]
    pub defer: usize,
    #[serde(default)]
    pub reject: usize,
    #[serde(default)]
    pub abstain: usize,
    /// Contradiction candidates surfaced by equivalence assessments
    /// (proposal-only, never a merge).
    #[serde(default)]
    pub equivalence_contradictions: usize,
    #[serde(default)]
    pub by_kind: GateOutcomeCountsByKind,
    /// Per-kind reason-category tallies (counts/proxies only), when provided.
    /// Omitted when absent to keep older fixture records parseable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_counts: Option<GateOutcomeReasonCountsByKind>,
}

fn sum_simple(c: &SimpleOutcomeCounts) -> usize {
    c.accept + c.defer + c.reject + c.abstain
}

fn fold_simple(total: &mut SimpleOutcomeCounts, part: &SimpleOutcomeCounts) {
    total.accept += part.accept;
    total.defer += part.defer;
    total.reject += part.reject;
    total.abstain += part.abstain;
}

fn gate_outcome_contradiction_count(outcome: &GateOutcome) -> usize {
    match outcome {
        GateOutcome::Equivalence(r) if r.contradiction.is_some() => 1,
        _ => 0,
    }
}

fn summary_from_kind_totals(
    by_kind: GateOutcomeCountsByKind,
    equivalence_contradictions: usize,
    reason_counts: Option<GateOutcomeReasonCountsByKind>,
) -> GateAssessmentSummary {
    let mut totals = SimpleOutcomeCounts::default();
    fold_simple(&mut totals, &by_kind.relation);
    fold_simple(&mut totals, &by_kind.equivalence);
    fold_simple(&mut totals, &by_kind.lesson);
    fold_simple(&mut totals, &by_kind.grounding);
    fold_simple(&mut totals, &by_kind.claim);
    GateAssessmentSummary {
        total: sum_simple(&totals),
        accept: totals.accept,
        defer: totals.defer,
        reject: totals.reject,
        abstain: totals.abstain,
        equivalence_contradictions,
        by_kind,
        reason_counts,
    }
}

/// Summarize typed gate outcomes (dispatch results) as counts/proxies only.
pub fn summarize_gate_outcomes(outcomes: &[GateOutcome]) -> GateAssessmentSummary {
    summary_from_kind_totals(
        gate_outcomes_by_kind(outcomes),
        outcomes.iter().map(gate_outcome_contradiction_count).sum(),
        Some(gate_outcome_reason_counts_by_kind(outcomes)),
    )
}

/// Summarize assessed intakes (intake+outcome pairs) as counts/proxies only.
pub fn summarize_assessed(assessed: &[crate::gate::AssessedIntake]) -> GateAssessmentSummary {
    summary_from_kind_totals(
        assessed_outcomes_by_kind(assessed),
        assessed_contradiction_count(assessed),
        Some(assessed_reason_counts_by_kind(assessed)),
    )
}

/// Count contradiction candidates surfaced by assessed equivalence outcomes.
///
/// A contradiction candidate indicates conflict detected by the resolution gate
/// (proposal-only, never a merge). Other outcome kinds contribute zero.
pub fn assessed_contradiction_count(assessed: &[crate::gate::AssessedIntake]) -> usize {
    assessed
        .iter()
        .filter(|a| matches!(&a.outcome, GateOutcome::Equivalence(r) if r.contradiction.is_some()))
        .count()
}

/// Coarse diagnostic categories for gate reason strings (counts/proxies only,
/// never a verdict). Classification is deterministic and best-effort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCategory {
    Provenance,
    LossyCitation,
    Extractor,
    Endpoints,
    Conflict,
    Ambiguous,
    BelowRelevanceFloor,
    Unsupported,
    Unstable,
    OutOfRange,
    Advisory,
    Other,
}

/// Classify a gate reason string into a coarse diagnostic category.
/// Deterministic, counts/proxies only; unknown strings map to `Other`.
pub fn classify_gate_reason(reason: &str) -> ReasonCategory {
    let r = reason;
    if r.contains("missing provenance") || r.contains("provenance anchor") {
        return ReasonCategory::Provenance;
    }
    if r.contains("no source span") || r.contains("lossy citation") {
        return ReasonCategory::LossyCitation;
    }
    if r.contains("missing extractor id") {
        return ReasonCategory::Extractor;
    }
    if r.contains("advisory score out of range") {
        return ReasonCategory::OutOfRange;
    }
    if r.contains("below floor") {
        return ReasonCategory::BelowRelevanceFloor;
    }
    if r.contains("conflict surfaced") {
        return ReasonCategory::Conflict;
    }
    if r.contains("ambiguous/unknown similarity") || r.contains("endpoints not both known") {
        return ReasonCategory::Ambiguous;
    }
    if r.contains("relation endpoints missing")
        || r.contains("invalid equivalence endpoints")
        || r.contains("no object id")
        || r.contains("empty lesson statement")
        || r.contains("empty concept")
    {
        return ReasonCategory::Endpoints;
    }
    if r.contains("no testable prediction")
        || r.contains("no cited episodes")
        || r.contains("weak grounding")
        || r.contains("unmapped")
        || r.contains("opaque payload")
        || r.contains("no cell/mapped_type")
        || r.contains("no snippet to check relevance against")
    {
        return ReasonCategory::Unsupported;
    }
    if r.contains("history stability flags") || r.contains("robustness_stable=false") {
        return ReasonCategory::Unstable;
    }
    if r.contains("advisory_confidence")
        || r.contains("advisory_similarity")
        || r.contains("extractor confidence is advisory")
        || r.contains("payload_kind=")
        || r.contains("pair_analysed")
        || r.contains("cites ")
        || r.contains("testable prediction present")
        || r.contains("suggests candidate equivalence")
        || r.contains("matches known label")
        || r.contains("shadow robustness_stable=true")
        || r.contains("robustness probe skipped")
        || r.contains("deferred: the observer may know something")
        || (r.contains("relevance=") && r.contains(">= floor"))
    {
        return ReasonCategory::Advisory;
    }
    ReasonCategory::Other
}

/// Per-kind reason-category tallies (counts/proxies only, never a verdict).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOutcomeReasonCountsByKind {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub relation: BTreeMap<ReasonCategory, usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub equivalence: BTreeMap<ReasonCategory, usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lesson: BTreeMap<ReasonCategory, usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub grounding: BTreeMap<ReasonCategory, usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub claim: BTreeMap<ReasonCategory, usize>,
}

fn tally_reasons(map: &mut BTreeMap<ReasonCategory, usize>, reasons: &[String]) {
    for r in reasons {
        *map.entry(classify_gate_reason(r)).or_insert(0) += 1;
    }
}

fn tally_outcome_reasons(counts: &mut GateOutcomeReasonCountsByKind, outcome: &GateOutcome) {
    match outcome {
        GateOutcome::Relation(r) => tally_reasons(&mut counts.relation, &r.reasons),
        GateOutcome::Equivalence(r) => tally_reasons(&mut counts.equivalence, &r.reasons),
        GateOutcome::Lesson(r) => tally_reasons(&mut counts.lesson, &r.reasons),
        GateOutcome::Grounding(r) => tally_reasons(&mut counts.grounding, &r.reasons),
        GateOutcome::Claim(r) => tally_reasons(&mut counts.claim, &r.reasons),
    }
}

pub fn gate_outcome_reason_counts_by_kind(
    outcomes: &[GateOutcome],
) -> GateOutcomeReasonCountsByKind {
    let mut out = GateOutcomeReasonCountsByKind::default();
    for o in outcomes {
        tally_outcome_reasons(&mut out, o);
    }
    out
}

pub fn assessed_reason_counts_by_kind(
    assessed: &[crate::gate::AssessedIntake],
) -> GateOutcomeReasonCountsByKind {
    let mut out = GateOutcomeReasonCountsByKind::default();
    for a in assessed {
        tally_outcome_reasons(&mut out, &a.outcome);
    }
    out
}

/// Deterministic, fixture-scoped ablation record for harness consumption.
/// Counts/proxies only, never a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObserverFixtureReport {
    pub compile: CompileComparison,
    pub baseline_observer_selection_ratio: f32,
    pub fused_observer_selection_ratio: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateAssessmentSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_counts: Option<GateOutcomeReasonCountsByKind>,
}

pub fn build_observer_fixture_report(
    baseline: &CompiledContext,
    fused: &CompiledContext,
    relevant_ids: Option<&HashSet<String>>,
    gate: Option<GateAssessmentSummary>,
) -> ObserverFixtureReport {
    ObserverFixtureReport {
        compile: compare_compiles(baseline, fused, relevant_ids),
        baseline_observer_selection_ratio: observer_selection_ratio(baseline),
        fused_observer_selection_ratio: observer_selection_ratio(fused),
        gate,
        reason_counts: None,
    }
}

pub fn observer_fixture_report_from_outcomes(
    baseline: &CompiledContext,
    fused: &CompiledContext,
    relevant_ids: Option<&HashSet<String>>,
    outcomes: &[GateOutcome],
) -> ObserverFixtureReport {
    let gate = summarize_gate_outcomes(outcomes);
    ObserverFixtureReport {
        reason_counts: gate.reason_counts.clone(),
        ..build_observer_fixture_report(baseline, fused, relevant_ids, Some(gate))
    }
}

pub fn observer_fixture_report_from_assessed(
    baseline: &CompiledContext,
    fused: &CompiledContext,
    relevant_ids: Option<&HashSet<String>>,
    assessed: &[crate::gate::AssessedIntake],
) -> ObserverFixtureReport {
    let gate = summarize_assessed(assessed);
    ObserverFixtureReport {
        reason_counts: gate.reason_counts.clone(),
        ..build_observer_fixture_report(baseline, fused, relevant_ids, Some(gate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile_context, CompileCandidate, CompileOptions, EmbedderInfo};
    use crate::embed::VectorEmbed;
    use crate::gate::{self, GateOutcome};
    use crate::ingest::{
        CandidateClaimPacket, ClaimPayload, ClaimSource, ConsolidatedLesson, EquivalenceProposal,
        ExtractorInfo, ProposedRelation,
    };
    use crate::relation::RelationType;

    fn claim(payload: ClaimPayload) -> CandidateClaimPacket {
        CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i".into()),
            source: ClaimSource {
                uri: "d".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("x"),
            asserted_at: None,
            observed_at: None,
            raw_ref: Some("r".into()),
            confidence: Some(0.5),
            payload,
            lossy_fields: vec![],
            extras: Default::default(),
        }
        .normalize()
    }

    #[test]
    fn outcome_tallies_are_deterministic() {
        let core = crate::core::PhysisCore::new();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let r = gate::gate_relation(&claim(ClaimPayload::Relation(rel.clone())), &rel);
        let rc = relation_outcomes(&[r]);
        assert_eq!(rc.accept, 1);

        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.9),
            rationale: None,
        };
        let er = gate::gate_equivalence(&core, &claim(ClaimPayload::Relation(rel)), &eq);
        let ec = resolution_outcomes(&[er]);
        assert_eq!(ec.abstain, 1);

        let lesson = ConsolidatedLesson {
            statement: "x".into(),
            prediction: None,
            episodes: vec![],
        };
        let lr = gate::gate_lesson(&core, &claim(ClaimPayload::Lesson(lesson.clone())), &lesson);
        let lc = lesson_outcomes(&[lr]);
        assert_eq!(lc.defer, 1);
    }

    #[test]
    fn context_recall_proxy_counts_selected_relevant() {
        let cands = vec![
            CompileCandidate {
                id: "a".into(),
                kind: "k".into(),
                text: "a".into(),
                tokens: 1,
                score: 1.0,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: false,
                source: None,
            },
            CompileCandidate {
                id: "b".into(),
                kind: "k".into(),
                text: "b".into(),
                tokens: 1,
                score: 0.5,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: true,
                source: None,
            },
        ];
        let pkt = compile_context(
            "q",
            cands,
            &CompileOptions {
                budget: 10,
                ..Default::default()
            },
            EmbedderInfo::default(),
        );
        let rel: HashSet<String> = ["a".to_string(), "c".to_string()].into_iter().collect();
        assert!((context_recall_proxy(&pkt, &rel) - 0.5).abs() < 1e-6);
        assert_eq!(observer_selected_count(&pkt), 1);
    }

    #[test]
    fn compile_comparison_reports_recall_delta() {
        let base_cands = vec![CompileCandidate {
            id: "a".into(),
            kind: "k".into(),
            text: "a".into(),
            tokens: 1,
            score: 1.0,
            strategies: vec![],
            reasons: vec![],
            observer_sourced: false,
            source: None,
        }];
        let fused_cands = vec![
            CompileCandidate {
                id: "a".into(),
                kind: "k".into(),
                text: "a".into(),
                tokens: 1,
                score: 1.0,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: false,
                source: None,
            },
            CompileCandidate {
                id: "obs".into(),
                kind: "k".into(),
                text: "o".into(),
                tokens: 1,
                score: 0.9,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: true,
                source: None,
            },
        ];
        let opts = CompileOptions {
            budget: 10,
            ..Default::default()
        };
        let base = compile_context("q", base_cands, &opts, EmbedderInfo::default());
        let fused = compile_context("q", fused_cands, &opts, EmbedderInfo::default());
        let rel: HashSet<String> = ["a".to_string(), "obs".to_string()].into_iter().collect();
        let cmp = compare_compiles(&base, &fused, Some(&rel));
        assert_eq!(cmp.recall_proxy_baseline, Some(0.5));
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        assert_eq!(cmp.recall_proxy_delta, Some(0.5));
        assert_eq!(cmp.fused_observer_selected, 1);
    }

    #[test]
    fn precision_recall_and_observer_ratio() {
        let pred: HashSet<String> = ["a", "b", "c"].into_iter().map(String::from).collect();
        let rel: HashSet<String> = ["b", "c", "d"].into_iter().map(String::from).collect();
        let (p, r, f1, tp) = precision_recall_f1(&pred, &rel);
        assert_eq!(tp, 2);
        assert!((p - 2.0 / 3.0).abs() < 1e-6);
        assert!((r - 2.0 / 3.0).abs() < 1e-6);
        assert!(f1 > 0.0);

        let cands = vec![
            CompileCandidate {
                id: "a".into(),
                kind: "k".into(),
                text: "a".into(),
                tokens: 1,
                score: 1.0,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: false,
                source: None,
            },
            CompileCandidate {
                id: "b".into(),
                kind: "k".into(),
                text: "b".into(),
                tokens: 1,
                score: 0.5,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: true,
                source: None,
            },
        ];
        let pkt = compile_context(
            "q",
            cands,
            &CompileOptions {
                budget: 10,
                ..Default::default()
            },
            EmbedderInfo::default(),
        );
        assert!((observer_selection_ratio(&pkt) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn pair_keys_and_equivalence_scoring_are_deterministic() {
        assert_eq!(canonical_pair_key("b", "a"), "a=b");
        assert_eq!(canonical_pair_key("a", "b"), "a=b");
        let pred: HashSet<String> = ["a=b", "c=d"].into_iter().map(String::from).collect();
        let gold: HashSet<String> = ["a=b", "c=e"].into_iter().map(String::from).collect();
        let (p, r, _f1, tp) = pair_precision_recall_f1(&pred, &gold);
        assert_eq!(tp, 1);
        assert!((p - 0.5).abs() < 1e-6);
        assert!((r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gate_outcome_counts_tallies_dispatch_results() {
        let core = crate::core::PhysisCore::new();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let r = gate::gate_relation(&claim(ClaimPayload::Relation(rel.clone())), &rel);
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.9),
            rationale: None,
        };
        let er = gate::gate_equivalence(&core, &claim(ClaimPayload::Relation(rel)), &eq);
        let outcomes = vec![GateOutcome::Relation(r), GateOutcome::Equivalence(er)];
        let c = gate_outcome_counts(&outcomes);
        assert_eq!(c.accept, 1);
        assert_eq!(c.abstain, 1);

        // Same tally over assessed intakes.
        let mut core2 = crate::core::PhysisCore::new();
        let claims = vec![claim(ClaimPayload::Relation(ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        }))];
        let assessed = crate::gate::assess_claims_with_intake(
            &mut core2,
            &claims,
            &[],
            "obs",
            chrono::Utc::now(),
        );
        let ac = assessed_outcome_counts(&assessed);
        assert_eq!(ac.accept, 1);
        assert_eq!(assessed[0].decision(), GateDecision::AcceptCandidate);
        assert_eq!(assessed_contradiction_count(&assessed), 0);
    }

    #[test]
    fn assessed_contradiction_count_surfaces_equivalence_conflicts() {
        let mut core = crate::core::PhysisCore::new();
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.1), // low agreement -> conflict surfaced, no merge
            rationale: None,
        };
        let claims = vec![claim(ClaimPayload::Equivalence(eq))];
        let assessed = crate::gate::assess_claims_with_intake(
            &mut core,
            &claims,
            &[],
            "obs",
            chrono::Utc::now(),
        );
        assert_eq!(assessed.len(), 1);
        // May abstain or reject depending on resolution path, but never a merge;
        // contradiction is surfaced as candidate when conflict is detected.
        assert!(matches!(assessed[0].outcome, GateOutcome::Equivalence(_)));
        let n = assessed_contradiction_count(&assessed);
        assert!(n <= 1);
        // Resolution/equivalence outcomes never certify merges (proposal-only).
        assert_ne!(assessed[0].decision(), GateDecision::AcceptCandidate);
    }

    #[test]
    fn grounding_outcome_tallies_mapped_vs_unmapped() {
        use crate::ingest::GroundedConcept;
        let known = vec!["MEMORY".to_string(), "IO".to_string()];
        let ok = gate::assess_grounding(
            &claim(ClaimPayload::Grounding(GroundedConcept {
                concept: "cache".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            })),
            &GroundedConcept {
                concept: "cache".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            },
            &known,
        );
        let unk = gate::assess_grounding(
            &claim(ClaimPayload::Grounding(GroundedConcept {
                concept: "mystery".into(),
                cell: Some("NOWHERE".into()),
                mapped_type: None,
            })),
            &GroundedConcept {
                concept: "mystery".into(),
                cell: Some("NOWHERE".into()),
                mapped_type: None,
            },
            &known,
        );
        let empty = gate::assess_grounding(
            &claim(ClaimPayload::Grounding(GroundedConcept {
                concept: "".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            })),
            &GroundedConcept {
                concept: "".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            },
            &known,
        );
        let c = grounding_outcomes(&[ok, unk, empty]);
        assert_eq!(c.accept, 1); // mapped to known label -> candidate proposal
        assert_eq!(c.defer, 1); // unmapped -> discovery/coverage defer
        assert_eq!(c.reject, 1); // empty concept
    }

    #[test]
    fn gate_outcomes_by_kind_tallies_mixed_dispatch_and_assessed() {
        use crate::ingest::{EntityMention, GroundedConcept};
        let core = crate::core::PhysisCore::new();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let r = gate::gate_relation(&claim(ClaimPayload::Relation(rel.clone())), &rel);
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.9),
            rationale: None,
        };
        let er = gate::gate_equivalence(&core, &claim(ClaimPayload::Equivalence(eq.clone())), &eq);
        let lesson = ConsolidatedLesson {
            statement: "x".into(),
            prediction: None,
            episodes: vec![],
        };
        let lr = gate::gate_lesson(&core, &claim(ClaimPayload::Lesson(lesson.clone())), &lesson);
        let known = vec!["MEMORY".to_string()];
        let gr = gate::assess_grounding(
            &claim(ClaimPayload::Grounding(GroundedConcept {
                concept: "cache".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            })),
            &GroundedConcept {
                concept: "cache".into(),
                cell: Some("MEMORY".into()),
                mapped_type: None,
            },
            &known,
        );
        let cr = gate::gate_claim(&claim(ClaimPayload::Entity(EntityMention {
            label: "e".into(),
            refers_to: None,
            concept: None,
        })));
        let outcomes = vec![
            GateOutcome::Relation(r),
            GateOutcome::Equivalence(er),
            GateOutcome::Lesson(lr),
            GateOutcome::Grounding(gr),
            GateOutcome::Claim(cr),
        ];
        let by = gate_outcomes_by_kind(&outcomes);
        assert_eq!(by.relation.accept, 1);
        assert_eq!(by.equivalence.abstain, 1);
        assert_eq!(by.lesson.defer, 1);
        assert_eq!(by.grounding.accept, 1);
        assert_eq!(by.claim.accept, 1);

        // assessed intake path produces the same per-kind structure
        let mut core2 = crate::core::PhysisCore::new();
        let claims = vec![
            claim(ClaimPayload::Relation(ProposedRelation {
                subject: "a".into(),
                predicate: "depends_on".into(),
                relation_type: Some(RelationType::DependsOn),
                object: "b".into(),
                fact: None,
            })),
            claim(ClaimPayload::Entity(EntityMention {
                label: "e".into(),
                refers_to: None,
                concept: None,
            })),
        ];
        let assessed = crate::gate::assess_claims_with_intake(
            &mut core2,
            &claims,
            &[],
            "obs",
            chrono::Utc::now(),
        );
        let aby = assessed_outcomes_by_kind(&assessed);
        assert_eq!(aby.relation.accept, 1);
        assert_eq!(aby.claim.accept, 1);
        assert_eq!(aby.equivalence, SimpleOutcomeCounts::default());
        assert_eq!(aby.lesson, SimpleOutcomeCounts::default());
        assert_eq!(aby.grounding, SimpleOutcomeCounts::default());
    }

    #[test]
    fn gate_assessment_summaries_compose_counts_and_contradictions() {
        use crate::ingest::EntityMention;
        let core = crate::core::PhysisCore::new();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let r = gate::gate_relation(&claim(ClaimPayload::Relation(rel.clone())), &rel);
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.1), // conflict surfaced as candidate contradiction, no merge
            rationale: None,
        };
        let er = gate::gate_equivalence(&core, &claim(ClaimPayload::Equivalence(eq.clone())), &eq);
        let cr = gate::gate_claim(&claim(ClaimPayload::Entity(EntityMention {
            label: "e".into(),
            refers_to: None,
            concept: None,
        })));
        let outcomes = vec![
            GateOutcome::Relation(r),
            GateOutcome::Equivalence(er),
            GateOutcome::Claim(cr),
        ];
        let sum = summarize_gate_outcomes(&outcomes);
        assert_eq!(sum.total, 3);
        assert_eq!(sum.by_kind.relation.accept, 1);
        assert_eq!(sum.by_kind.claim.accept, 1);
        assert_eq!(sum.accept + sum.defer + sum.reject + sum.abstain, sum.total);
        assert!(
            sum.equivalence_contradictions <= 1,
            "equivalence path is proposal-only and may surface at most one contradiction candidate here"
        );
        // no certified merge implied
        assert_ne!(sum.by_kind.equivalence.accept, 1);

        let mut core2 = crate::core::PhysisCore::new();
        let claims = vec![
            claim(ClaimPayload::Relation(ProposedRelation {
                subject: "a".into(),
                predicate: "depends_on".into(),
                relation_type: Some(RelationType::DependsOn),
                object: "b".into(),
                fact: None,
            })),
            claim(ClaimPayload::Entity(EntityMention {
                label: "e".into(),
                refers_to: None,
                concept: None,
            })),
        ];
        let assessed = crate::gate::assess_claims_with_intake(
            &mut core2,
            &claims,
            &[],
            "obs",
            chrono::Utc::now(),
        );
        let asum = summarize_assessed(&assessed);
        assert_eq!(asum.total, 2);
        assert_eq!(asum.by_kind.relation.accept, 1);
        assert_eq!(asum.by_kind.claim.accept, 1);
        assert_eq!(asum.equivalence_contradictions, 0);
        assert_eq!(
            asum.accept + asum.defer + asum.reject + asum.abstain,
            asum.total
        );
        // serializable for harness capture (counts/proxies only)
        let j = serde_json::to_string(&asum).unwrap();
        assert!(j.contains("\"total\":2"));
    }

    /// A retrieval-hit claim from an observer, attributed to a named extractor
    /// with a real source uri, carrying `object_id`/`snippet`.
    fn retrieval_claim(object_id: &str, snippet: &str, score: f32) -> CandidateClaimPacket {
        use crate::ingest::RetrievalHit;
        claim(ClaimPayload::Retrieval(RetrievalHit {
            object_id: object_id.into(),
            score,
            strategy: Some("vec".into()),
            snippet: Some(snippet.into()),
        }))
    }

    /// The ablation the crate contract demands before fusion may be trusted:
    /// the *real* pipeline (RagCorpus → TokenFixedRetriever → compile),
    /// core-only baseline vs observer-fused packet under the same budget,
    /// compared with [`compare_compiles`]. The observer knows one relevant
    /// object the local corpus never named — the one case fusion can help.
    #[test]
    fn fusion_ablation_over_real_retrieval_observer_adds_recall() {
        use crate::rag::{RagCorpus, TokenFixedRetriever};
        use crate::RandomProjectionEmbedder;

        let embedder = RandomProjectionEmbedder::new(64);
        let docs = [
            "the spindle bearing lubrication schedule is monthly",
            "relay calibration requires the bench harness and a reference meter",
            "shift handover notes are filed at the end of each week",
            "the coolant pump filter is replaced every quarter",
        ];
        let texts: Vec<String> = docs.iter().map(|d| d.to_string()).collect();
        let corpus = RagCorpus::build(&texts, &embedder);
        let query_emb = embedder.embed("relay calibration bench harness");
        let retrieval = TokenFixedRetriever::new(400, 4).retrieve(&query_emb, &corpus);
        assert!(!retrieval.chunks.is_empty(), "the corpus must retrieve");

        // The observer surfaces a relay-procedure object the corpus never named.
        let claims = vec![retrieval_claim(
            "obj:relay-procedure",
            "relay calibration: bench harness, reference meter, two-pass verification",
            0.9,
        )];
        let tokens_for = |c: &CandidateClaimPacket, h: &crate::ingest::RetrievalHit| {
            let _ = c;
            crate::rag::count_tokens(h.snippet.as_deref().unwrap_or(""))
        };

        let base = crate::compile::compile_from_retrieval(
            "relay calibration",
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let fused = crate::compile::compile_from_retrieval_with_observer_claims(
            "relay calibration",
            &retrieval,
            &["engine"],
            &claims,
            EmbedderInfo::default(),
            tokens_for,
        );

        // Same budget, both within it — the comparison is construction-matched.
        assert_eq!(base.stats.budget, fused.stats.budget);
        assert!(base.stats.selected_tokens <= base.stats.budget);
        assert!(fused.stats.selected_tokens <= fused.stats.budget);

        // Ample budget: fusion adds the observer object and keeps every engine
        // selection. Losing an engine chunk to an observer claim would be a
        // displacement, and this fixture forbids it.
        let base_ids: HashSet<&str> = base.selected.iter().map(|s| s.id.as_str()).collect();
        let fused_ids: HashSet<&str> = fused.selected.iter().map(|s| s.id.as_str()).collect();
        assert!(
            base_ids.is_subset(&fused_ids),
            "engine selections must survive fusion"
        );
        assert_eq!(observer_selected_count(&fused), 1);
        assert_eq!(observer_selected_count(&base), 0);

        // Measured recall over a relevant set the baseline cannot fully cover.
        let mut relevant: HashSet<String> = base_ids.iter().map(|s| s.to_string()).collect();
        relevant.insert("obj:relay-procedure".to_string());
        let cmp = compare_compiles(&base, &fused, Some(&relevant));
        assert_eq!(cmp.baseline_observer_selected, 0);
        assert_eq!(cmp.fused_observer_selected, 1);
        // The relevant set is every engine selection plus the observer-only
        // object, so the baseline sits at n/(n+1) and fusion closes the gap.
        let n = base_ids.len() as f32;
        assert_eq!(cmp.recall_proxy_baseline, Some(n / (n + 1.0)));
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        // f32 subtraction rounds: 1.0 - 0.8 is 0.19999999, not 0.2.
        let delta = cmp.recall_proxy_delta.unwrap();
        assert!((delta - 1.0 / (n + 1.0)).abs() < 1e-6, "delta was {delta}");
    }

    /// The conservative half of the same contract: when the budget cannot fit
    /// the observer's contribution, fusion must not displace engine evidence —
    /// the fused packet selects exactly what the baseline selected, and the
    /// recall delta is zero, not negative.
    #[test]
    fn fusion_is_conservative_when_the_budget_cannot_fit_the_observer() {
        use crate::rag::{RagCorpus, TokenFixedRetriever};
        use crate::RandomProjectionEmbedder;

        let embedder = RandomProjectionEmbedder::new(64);
        let texts = vec!["alpha".to_string(), "beta".to_string()];
        let corpus = RagCorpus::build(&texts, &embedder);
        let query_emb = embedder.embed("alpha");
        // Two one-token docs, budget exactly their total: the engine fills it.
        let retrieval = TokenFixedRetriever::new(2, 2).retrieve(&query_emb, &corpus);
        assert_eq!(retrieval.chunks.len(), 2);
        assert_eq!(retrieval.total_tokens, 2);

        let claims = vec![retrieval_claim("obj:hidden", "hidden relay note", 0.9)];
        let tokens_for = |c: &CandidateClaimPacket, h: &crate::ingest::RetrievalHit| {
            let _ = c;
            crate::rag::count_tokens(h.snippet.as_deref().unwrap_or(""))
        };

        let base = crate::compile::compile_from_retrieval(
            "alpha",
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let fused = crate::compile::compile_from_retrieval_with_observer_claims(
            "alpha",
            &retrieval,
            &["engine"],
            &claims,
            EmbedderInfo::default(),
            tokens_for,
        );

        let base_ids: HashSet<String> = base.selected.iter().map(|s| s.id.clone()).collect();
        let fused_ids: HashSet<String> = fused.selected.iter().map(|s| s.id.clone()).collect();
        assert_eq!(base_ids, fused_ids, "a full budget must select identically");
        assert_eq!(observer_selected_count(&fused), 0);
        assert!(fused
            .dropped
            .iter()
            .any(|d| d.id == "obj:hidden" && d.reason == crate::compile::DropReason::OverBudget));

        let mut relevant: HashSet<String> = base_ids.clone();
        relevant.insert("obj:hidden".to_string());
        let cmp = compare_compiles(&base, &fused, Some(&relevant));
        assert_eq!(cmp.recall_proxy_baseline, cmp.recall_proxy_fused);
        assert_eq!(cmp.recall_proxy_delta, Some(0.0));
    }

    /// Regime 1 of the realistic ablation: the engine's corpus does not cover
    /// the query at all, and the observer supplies the only relevant knowledge.
    /// This is the one case fusion can win, and it does — measured, not assumed.
    #[test]
    fn sparse_corpus_observer_supplies_the_only_relevant_knowledge() {
        use crate::rag::{RagCorpus, TokenFixedRetriever};
        use crate::RandomProjectionEmbedder;

        let embedder = RandomProjectionEmbedder::new(64);
        let docs = [
            "spindle bearing lubrication schedule is monthly",
            "coolant pump filter replacement every quarter",
            "shift handover notes filed weekly",
            "invoice reconciliation runs on the first business day",
            "warehouse picking routes optimized each morning",
            "fire extinguisher inspection tags renewed yearly",
        ];
        let texts: Vec<String> = docs.iter().map(|d| d.to_string()).collect();
        let corpus = RagCorpus::build(&texts, &embedder);
        let query_emb = embedder.embed("deploy rollback procedure smoke checks");
        let retrieval = TokenFixedRetriever::new(400, 6).retrieve(&query_emb, &corpus);
        assert!(!retrieval.chunks.is_empty());

        let claims = vec![retrieval_claim(
            "obj:rollback-runbook",
            "deploy rollback: revert to last green build then rerun smoke checks",
            0.9,
        )];
        let tokens_for = |c: &CandidateClaimPacket, h: &crate::ingest::RetrievalHit| {
            let _ = c;
            crate::rag::count_tokens(h.snippet.as_deref().unwrap_or(""))
        };
        let base = crate::compile::compile_from_retrieval(
            "deploy rollback",
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let fused = crate::compile::compile_from_retrieval_with_observer_claims(
            "deploy rollback",
            &retrieval,
            &["engine"],
            &claims,
            EmbedderInfo::default(),
            tokens_for,
        );

        let relevant: HashSet<String> = ["obj:rollback-runbook".to_string()].into();
        let cmp = compare_compiles(&base, &fused, Some(&relevant));
        assert_eq!(cmp.recall_proxy_baseline, Some(0.0));
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        assert_eq!(cmp.recall_proxy_delta, Some(1.0));
        assert_eq!(observer_selected_count(&fused), 1);
    }

    /// Regime 2: the engine covers the query, the budget is ample, and the
    /// observer adds one relevant object plus two noise hits. The recall win
    /// is real but the noise rides along — measured here as the cost side of
    /// the same number.
    #[test]
    fn ample_budget_dense_corpus_noise_rides_with_the_win() {
        use crate::rag::{RagCorpus, TokenFixedRetriever};
        use crate::RandomProjectionEmbedder;

        let embedder = RandomProjectionEmbedder::new(64);
        let docs = [
            "relay calibration requires the bench harness and a reference meter",
            "relay calibration two pass verification catches swapped contacts",
            "spindle bearing lubrication schedule is monthly",
            "coolant pump filter replacement every quarter",
            "shift handover notes filed weekly",
            "invoice reconciliation runs on the first business day",
        ];
        let texts: Vec<String> = docs.iter().map(|d| d.to_string()).collect();
        let corpus = RagCorpus::build(&texts, &embedder);
        let query_emb = embedder.embed("relay calibration bench harness");
        let retrieval = TokenFixedRetriever::new(400, 6).retrieve(&query_emb, &corpus);
        assert!(retrieval.chunks.len() >= 2, "both relay docs must retrieve");

        let claims = vec![
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
        ];
        let tokens_for = |c: &CandidateClaimPacket, h: &crate::ingest::RetrievalHit| {
            let _ = c;
            crate::rag::count_tokens(h.snippet.as_deref().unwrap_or(""))
        };
        let base = crate::compile::compile_from_retrieval(
            "relay calibration",
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let fused = crate::compile::compile_from_retrieval_with_observer_claims(
            "relay calibration",
            &retrieval,
            &["engine"],
            &claims,
            EmbedderInfo::default(),
            tokens_for,
        );

        let base_ids: HashSet<String> = base.selected.iter().map(|s| s.id.clone()).collect();
        let mut relevant = base_ids.clone();
        relevant.insert("obj:relay-procedure".to_string());
        let cmp = compare_compiles(&base, &fused, Some(&relevant));

        // The win: the additive object is selected and the recall gap closes.
        assert_eq!(cmp.fused_observer_selected, 3);
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        let n = base_ids.len() as f32;
        let delta = cmp.recall_proxy_delta.unwrap();
        assert!((delta - 1.0 / (n + 1.0)).abs() < 1e-6, "delta was {delta}");

        // The cost, same packet: both noise hits were selected too — the
        // compiler ranks by score but nothing filters an observer's noise
        // before the budget does.
        let fused_ids: HashSet<&str> = fused.selected.iter().map(|s| s.id.as_str()).collect();
        assert!(fused_ids.contains("obj:noise-menu"));
        assert!(fused_ids.contains("obj:noise-weather"));
        assert!(utilization(&fused) > utilization(&base));
    }

    /// The gated fusion path answers the noise-admission cost measured in
    /// regime 2: each observer retrieval hit is gated against the query by
    /// cosine relevance before it can become a candidate. Measured with this
    /// embedder, the additive object scores ~0.93 against the query and the
    /// noise hits 0.892/0.858 — a floor of 0.9 admits the additive object and
    /// defers both noise hits with named causes. The recall win survives; the
    /// noise does not ride along. Gate exclusions are returned, never silent.
    #[test]
    fn gated_fusion_admits_the_relevant_hit_and_defers_noise_with_named_causes() {
        use crate::rag::{RagCorpus, TokenFixedRetriever};
        use crate::RandomProjectionEmbedder;

        let embedder = RandomProjectionEmbedder::new(64);
        let docs = [
            "relay calibration requires the bench harness and a reference meter",
            "relay calibration two pass verification catches swapped contacts",
            "spindle bearing lubrication schedule is monthly",
            "coolant pump filter replacement every quarter",
            "shift handover notes filed weekly",
            "invoice reconciliation runs on the first business day",
        ];
        let texts: Vec<String> = docs.iter().map(|d| d.to_string()).collect();
        let corpus = RagCorpus::build(&texts, &embedder);
        let query = "relay calibration bench harness";
        let query_emb = embedder.embed(query);
        let retrieval = TokenFixedRetriever::new(400, 6).retrieve(&query_emb, &corpus);
        assert!(retrieval.chunks.len() >= 2, "both relay docs must retrieve");

        let claims = vec![
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
        ];
        let base = crate::compile::compile_from_retrieval(
            query,
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let (fused, gated) = crate::compile::compile_from_retrieval_with_gated_observer_claims(
            query,
            &retrieval,
            &["engine"],
            &claims,
            &embedder,
            0.9,
        );

        // Gate receipts: exactly one accept (the additive object), two defers
        // with the floor named in the reasons.
        assert_eq!(gated.len(), 3);
        assert_eq!(
            gated[0].decision,
            crate::gate::GateDecision::AcceptCandidate
        );
        for g in &gated[1..] {
            assert_eq!(g.decision, crate::gate::GateDecision::Defer);
            assert!(g.reasons.iter().any(|r| r.contains("below floor")));
            assert!(g.relevance.unwrap() < 0.9);
        }

        // The packet: only the gated-in observer item, noise excluded — the
        // recall win survives without the noise riding along.
        assert_eq!(observer_selected_count(&fused), 1);
        let fused_ids: HashSet<&str> = fused.selected.iter().map(|s| s.id.as_str()).collect();
        assert!(fused_ids.contains("obj:relay-procedure"));
        assert!(!fused_ids.contains("obj:noise-menu"));
        assert!(!fused_ids.contains("obj:noise-weather"));

        let base_ids: HashSet<String> = base.selected.iter().map(|s| s.id.clone()).collect();
        let mut relevant = base_ids.clone();
        relevant.insert("obj:relay-procedure".to_string());
        let cmp = compare_compiles(&base, &fused, Some(&relevant));
        // The baseline cannot see the observer-only object: recall 6/7. The
        // gated fusion closes the gap (7/7) without the noise riding along.
        let n = base_ids.len() as f32;
        assert_eq!(cmp.recall_proxy_baseline, Some(n / (n + 1.0)));
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        let delta = cmp.recall_proxy_delta.unwrap();
        assert!((delta - 1.0 / (n + 1.0)).abs() < 1e-6, "delta was {delta}");
        assert!(fused_ids.is_superset(&base_ids.iter().map(|s| s.as_str()).collect()));
    }

    /// The contract the displacement tripwire exposed: an observer's advisory
    /// score must never outrank verified engine evidence. Under partial slack
    /// the observer item may only fill what the engine did not take — here the
    /// slack (1 token) cannot fit the snippet, so the fused packet must select
    /// exactly what the baseline selected and lose nothing.
    #[test]
    fn advisory_observer_scores_must_not_displace_engine_evidence() {
        use crate::rag::{RetrievalResult, RetrievedChunk};

        let chunk = |id: usize, score: f32| RetrievedChunk {
            id,
            text: format!("doc {id} relay calibration evidence"),
            score,
            tokens: 3,
        };
        let retrieval = RetrievalResult {
            chunks: vec![chunk(0, 0.9), chunk(1, 0.8), chunk(2, 0.7)],
            total_tokens: 9,
            budget: 10,
            truncated: false,
        };
        let claims = vec![retrieval_claim("obj:shiny", "one two three four", 0.99)];
        let tokens_for = |c: &CandidateClaimPacket, h: &crate::ingest::RetrievalHit| {
            let _ = c;
            crate::rag::count_tokens(h.snippet.as_deref().unwrap_or(""))
        };
        let base = crate::compile::compile_from_retrieval(
            "relay calibration",
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let fused = crate::compile::compile_from_retrieval_with_observer_claims(
            "relay calibration",
            &retrieval,
            &["engine"],
            &claims,
            EmbedderInfo::default(),
            tokens_for,
        );

        let base_ids: HashSet<&str> = base.selected.iter().map(|s| s.id.as_str()).collect();
        let fused_ids: HashSet<&str> = fused.selected.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(base_ids.len(), 3);
        assert_eq!(
            base_ids, fused_ids,
            "no engine chunk may lose its slot to an advisory score"
        );
        assert_eq!(observer_selected_count(&fused), 0);
        assert!(fused
            .dropped
            .iter()
            .any(|d| d.id == "obj:shiny" && d.reason == crate::compile::DropReason::OverBudget));

        let relevant: HashSet<String> = base_ids.iter().map(|s| s.to_string()).collect();
        let cmp = compare_compiles(&base, &fused, Some(&relevant));
        assert_eq!(cmp.recall_proxy_baseline, Some(1.0));
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        assert_eq!(cmp.recall_proxy_delta, Some(0.0));
    }

    /// The other half of the same contract: when slack CAN fit the observer
    /// item, fusion is a pure superset — the observer fills leftover budget
    /// after every engine chunk is placed, and recall only rises.
    #[test]
    fn observer_fills_slack_without_displacing() {
        use crate::rag::{RetrievalResult, RetrievedChunk};

        let chunk = |id: usize, score: f32| RetrievedChunk {
            id,
            text: format!("doc {id} relay calibration evidence"),
            score,
            tokens: 3,
        };
        let retrieval = RetrievalResult {
            chunks: vec![chunk(0, 0.9), chunk(1, 0.8), chunk(2, 0.7)],
            total_tokens: 9,
            budget: 13,
            truncated: false,
        };
        let claims = vec![retrieval_claim(
            "obj:slack-fill",
            "one two three four",
            0.99,
        )];
        let tokens_for = |c: &CandidateClaimPacket, h: &crate::ingest::RetrievalHit| {
            let _ = c;
            crate::rag::count_tokens(h.snippet.as_deref().unwrap_or(""))
        };
        let base = crate::compile::compile_from_retrieval(
            "relay calibration",
            &retrieval,
            &["engine"],
            EmbedderInfo::default(),
        );
        let fused = crate::compile::compile_from_retrieval_with_observer_claims(
            "relay calibration",
            &retrieval,
            &["engine"],
            &claims,
            EmbedderInfo::default(),
            tokens_for,
        );

        let base_ids: HashSet<&str> = base.selected.iter().map(|s| s.id.as_str()).collect();
        let fused_ids: HashSet<&str> = fused.selected.iter().map(|s| s.id.as_str()).collect();
        assert!(base_ids.is_subset(&fused_ids));
        assert_eq!(observer_selected_count(&fused), 1);

        let mut relevant: HashSet<String> = base_ids.iter().map(|s| s.to_string()).collect();
        relevant.insert("obj:slack-fill".to_string());
        let cmp = compare_compiles(&base, &fused, Some(&relevant));
        assert_eq!(cmp.recall_proxy_baseline, Some(3.0 / 4.0));
        assert_eq!(cmp.recall_proxy_fused, Some(1.0));
        assert!(cmp.recall_proxy_delta.unwrap() > 0.0);
    }

    #[test]
    fn observer_fixture_report_composes_compile_gate_ratios_and_reasons() {
        use crate::ingest::EntityMention;
        let base_cands = vec![CompileCandidate {
            id: "a".into(),
            kind: "k".into(),
            text: "a".into(),
            tokens: 1,
            score: 1.0,
            strategies: vec![],
            reasons: vec![],
            observer_sourced: false,
            source: None,
        }];
        let fused_cands = vec![
            CompileCandidate {
                id: "a".into(),
                kind: "k".into(),
                text: "a".into(),
                tokens: 1,
                score: 1.0,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: false,
                source: None,
            },
            CompileCandidate {
                id: "obs".into(),
                kind: "k".into(),
                text: "o".into(),
                tokens: 1,
                score: 0.9,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: true,
                source: None,
            },
        ];
        let opts = CompileOptions {
            budget: 10,
            ..Default::default()
        };
        let base = compile_context("q", base_cands, &opts, EmbedderInfo::default());
        let fused = compile_context("q", fused_cands, &opts, EmbedderInfo::default());
        let relevant: HashSet<String> = ["a".to_string(), "obs".to_string()].into_iter().collect();
        let core = crate::core::PhysisCore::new();
        let rel = ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        };
        let r = gate::gate_relation(&claim(ClaimPayload::Relation(rel.clone())), &rel);
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.1),
            rationale: None,
        };
        let er = gate::gate_equivalence(&core, &claim(ClaimPayload::Equivalence(eq.clone())), &eq);
        let cr = gate::gate_claim(&claim(ClaimPayload::Entity(EntityMention {
            label: "e".into(),
            refers_to: None,
            concept: None,
        })));
        let report = observer_fixture_report_from_outcomes(
            &base,
            &fused,
            Some(&relevant),
            &[
                GateOutcome::Relation(r),
                GateOutcome::Equivalence(er),
                GateOutcome::Claim(cr),
            ],
        );
        assert_eq!(report.compile.recall_proxy_delta, Some(0.5));
        assert_eq!(report.gate.as_ref().unwrap().total, 3);
        assert!(report.gate.as_ref().unwrap().equivalence_contradictions <= 1);
        assert!(!report.reason_counts.as_ref().unwrap().relation.is_empty());
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
        assert!(v.get("reason_counts").is_some());
    }

    #[test]
    fn observer_fixture_report_noise_baseline_records_ratios_without_recall_gain() {
        let base_cands = vec![CompileCandidate {
            id: "a".into(),
            kind: "k".into(),
            text: "a".into(),
            tokens: 1,
            score: 1.0,
            strategies: vec![],
            reasons: vec![],
            observer_sourced: false,
            source: None,
        }];
        let fused_cands = vec![
            CompileCandidate {
                id: "a".into(),
                kind: "k".into(),
                text: "a".into(),
                tokens: 1,
                score: 1.0,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: false,
                source: None,
            },
            CompileCandidate {
                id: "obs-noise".into(),
                kind: "k".into(),
                text: "n".into(),
                tokens: 1,
                score: 0.2,
                strategies: vec![],
                reasons: vec![],
                observer_sourced: true,
                source: None,
            },
        ];
        let opts = CompileOptions {
            budget: 10,
            ..Default::default()
        };
        let base = compile_context("q", base_cands, &opts, EmbedderInfo::default());
        let fused = compile_context("q", fused_cands, &opts, EmbedderInfo::default());
        let relevant: HashSet<String> = ["a".to_string()].into_iter().collect();
        let report = build_observer_fixture_report(&base, &fused, Some(&relevant), None);
        assert_eq!(report.compile.recall_proxy_delta, Some(0.0));
        assert!(report.fused_observer_selection_ratio > 0.0);
        assert!(report.gate.is_none());
        assert!(report.reason_counts.is_none());
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
        assert!(v.get("gate").is_none() && v.get("reason_counts").is_none());
    }

    #[test]
    fn observer_fixture_report_from_assessed_summarizes_intake_outcomes() {
        let base_cands = vec![CompileCandidate {
            id: "a".into(),
            kind: "k".into(),
            text: "a".into(),
            tokens: 1,
            score: 1.0,
            strategies: vec![],
            reasons: vec![],
            observer_sourced: false,
            source: None,
        }];
        let opts = CompileOptions {
            budget: 10,
            ..Default::default()
        };
        let base = compile_context("q", base_cands.clone(), &opts, EmbedderInfo::default());
        let fused = compile_context("q", base_cands, &opts, EmbedderInfo::default());
        let mut core = crate::core::PhysisCore::new();
        let claims = vec![claim(ClaimPayload::Relation(ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        }))];
        let assessed = crate::gate::assess_claims_with_intake(
            &mut core,
            &claims,
            &[],
            "obs",
            chrono::Utc::now(),
        );
        let report = observer_fixture_report_from_assessed(&base, &fused, None, &assessed);
        assert_eq!(report.gate.as_ref().unwrap().total, 1);
        assert!(report
            .reason_counts
            .as_ref()
            .unwrap()
            .relation
            .contains_key(&ReasonCategory::Advisory));
    }

    #[test]
    fn gate_reason_categories_are_deterministic_counts_only() {
        use crate::ingest::{EntityMention, GroundedConcept};
        let core = crate::core::PhysisCore::new();
        let mut missing_prov = claim(ClaimPayload::Relation(ProposedRelation {
            subject: "a".into(),
            predicate: "depends_on".into(),
            relation_type: Some(RelationType::DependsOn),
            object: "b".into(),
            fact: None,
        }));
        missing_prov.source.uri.clear();
        missing_prov.raw_ref = None;
        let rel = gate::gate_relation(
            &missing_prov,
            &ProposedRelation {
                subject: "a".into(),
                predicate: "depends_on".into(),
                relation_type: Some(RelationType::DependsOn),
                object: "b".into(),
                fact: None,
            },
        );
        assert_eq!(rel.decision, GateDecision::Defer);
        let eq = EquivalenceProposal {
            left: "n1".into(),
            right: "n2".into(),
            similarity: Some(0.1),
            rationale: None,
        };
        let er = gate::gate_equivalence(&core, &claim(ClaimPayload::Equivalence(eq.clone())), &eq);
        let lesson = ConsolidatedLesson {
            statement: "x".into(),
            prediction: None,
            episodes: vec![],
        };
        let lr = gate::gate_lesson(&core, &claim(ClaimPayload::Lesson(lesson.clone())), &lesson);
        let gr = gate::assess_grounding(
            &claim(ClaimPayload::Grounding(GroundedConcept {
                concept: "mystery".into(),
                cell: Some("NOWHERE".into()),
                mapped_type: None,
            })),
            &GroundedConcept {
                concept: "mystery".into(),
                cell: Some("NOWHERE".into()),
                mapped_type: None,
            },
            &["MEMORY".to_string()],
        );
        let cr = gate::gate_claim(&claim(ClaimPayload::Entity(EntityMention {
            label: "e".into(),
            refers_to: None,
            concept: None,
        })));
        let outcomes = vec![
            GateOutcome::Relation(rel),
            GateOutcome::Equivalence(er),
            GateOutcome::Lesson(lr),
            GateOutcome::Grounding(gr),
            GateOutcome::Claim(cr),
        ];
        let counts = gate_outcome_reason_counts_by_kind(&outcomes);
        assert_eq!(counts, gate_outcome_reason_counts_by_kind(&outcomes));
        assert!(
            counts
                .relation
                .get(&ReasonCategory::Provenance)
                .copied()
                .unwrap_or(0)
                >= 1
        );
        assert_eq!(
            classify_gate_reason("novel reason text"),
            ReasonCategory::Other
        );
    }
}
