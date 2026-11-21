# ws14 observer-eval hardening — merge rollup

**Date:** 2026-11-21  
**Repo:** `hermes-physis` (`main`)  
**Scope:** additive/default-preserving observer-eval hardening; counts/proxies only (no verdict/certification/merge).

## Findings

- `d3a659d` added deterministic `GateOutcomeCountsByKind::merge` and `GateAssessmentSummary::merge` for multi-fixture harness rollups.
- Merge rollup test needed stronger coverage for reason-count merge semantics and contradiction proxy accumulation.

## Plan

- [x] Verify baseline: observer_eval/lib tests green.
- [x] Strengthen merge test: remove dead `PhysisCore` allocation; assert equivalence contradiction proxy rollup; assert `None+Some` and `Some+Some` reason-count merge; assert decision tally behavior follows outcome decision (counts/proxies only).
- [x] Run fmt + observer_eval + lib tests.
- [x] Commit and push only this work.

## Verification

- `cargo fmt -- src/observer_eval.rs`
- `cargo test --lib observer_eval` — **28 passed**
- `cargo test --lib` — **400 passed**

## Landed

- `d3a659d` `feat(observer-eval): add deterministic summary rollup merge`
- `7bdbe3d` `test(observer-eval): harden summary merge rollup coverage`

## Constraints

- Counts/proxies only; never verdict/certification/merge.
- Additive/default-preserving; optional/serde-default fields remain backcompat-safe.
- Unrelated dirty/untracked files left untouched.
