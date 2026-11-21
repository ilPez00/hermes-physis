# Observer fusion — follow-up plan (written 2026-11-20, post `df284e8`)

State at writing: the fusion chain is complete and measured. Every shipped
constant is traced to real-corpus measurement (RP floor 0.99 opt-in; semantic
floor 0.66 validated at n=22 relevant / 46 noise, zero contamination; auto
reserve; assembled-packet accounting; per-path pricing; relative gate measured
and rejected). Work-check on 2026-11-20: gates green, deployed binary
byte-identical to HEAD, 23/23 queries reproduce exactly.

## U1 — Ship the measurement harness (prerequisite for everything else)

The evidence behind every constant lives in `/tmp/real-fusion` — volatile,
already lost once. Move into the repo:

- `benchmarks/observer-fusion/measure.py` — the sweep harness (parameterised
  reserve/floor, `--rp` mode, per-query claims, phrase-presence checks).
- `benchmarks/observer-fusion/claims.py` or inline — the held-out + noise
  claim definitions (verbatim excerpts, provenance noted).
- `benchmarks/observer-fusion/results-n22.json` + `results-repro.json` — the
  recorded measurements backing the 0.66 default and the reproducibility
  claim.
- `benchmarks/observer-fusion/README.md` — how to run it (env vars, corpus
  path expectation, runtime, what each field means).

Acceptance: a fresh clone + the documented commands reproduce the recorded
numbers. The /tmp copy is then disposable.

## U2 — Benchmark doc header refresh

`benchmarks/observer-fusion-real-corpus-2026-11-20.md` opens with the original
n=3 setup ("0.6 semantic"). Prepend a current-verdict header (0.66, n=22/46,
zero contamination held across two broadenings, relative gate rejected) so a
reader gets the final state before the log. The log sections stay untouched —
they are the audit trail.

## U3 — Crate version 0.2.0 → 0.3.0

The chain added observer fusion, reserve sizing (None/Fixed/Auto), the
assembled-packet accounting fix, per-path reserve pricing, and
`ContextReport.engine_best_score`. Minor bump earned. Update Cargo.toml,
CHANGELOG.

## U4 — `physis-core selftest` (substantive; the engine audits itself)

A new subcommand that runs a built-in set of known-answer probes against a
caller-supplied corpus and prints a verdict receipt:

- Each probe: query + observer claim (verbatim excerpt of a held-out doc) +
  distinctive phrase + noise claims. Pass = phrase present, zero noise text.
- Prints per-probe receipts (gate decision, relevance, engine_best_score) and
  a summary; **exits nonzero if any should-pass probe defers or any noise
  text enters a packet** — usable as a gate in scripts/CI.
- Probes ship as an embedded default set (the n=22 subset that fits) and
  accept `--probes file.json` for caller-supplied sets.
- Red-first: a failing-probe test written against the current binary must
  fail before the exit-code wiring exists; mutation-verified (flip the floor
  constant in a test build → selftest must fail).

This institutionalises "a worker's success claim is not evidence" inside the
engine itself: the binary ships with its own falsification harness.

## U5+ — Emergent feature candidates (plan-only, not started)

- **Per-corpus floor calibration** (`context --calibrate-floor`): run the
  noise probes against the caller's actual corpus and report where the
  measured zero-noise gap sits. The 0.66 default is corpus-agnostic; this
  makes it corpus-aware. Needs a labelled probe set — design first.
- **Receipt drift watch**: diff `observer_gated` receipts across runs of the
  same query; a corpus edit that silently flips gate decisions becomes
  visible. Cheap once U1's harness is in-repo.
- **Relative diagnostic reporting** (gating was rejected in `8e602`): report
  `relevance / engine_best_score` in receipts as a diagnostic only, so
  callers can see when a claim is weaker than the engine's own evidence.

## U5 — Receipt drift watch (detailed spec)

**Goal:** a corpus edit that silently flips a gate decision becomes visible.

**Design:** extend `physis-core selftest` with `--baseline <report.json>`.
The baseline is a previous `selftest --json` output (the report IS the
baseline artifact — no second format). After the live run, diff per probe:

- decision flips: `baseline_absent` / `answered` / `forbidden_absent` /
  `passed` changing value;
- relevance drift: any claim's cosine moving more than 0.01 between runs
  (embedders are deterministic; a bigger move means the corpus or model
  changed under the probe).

To carry relevances, `SelftestProbeReport` gains
`relevances: BTreeMap<String, f32>` (serde-defaulted, from the fused run's
`observer_gated` receipts).

**Core is a pure function** `diff_selftest_reports(baseline, current) ->
Vec<SelftestDrift>` — trivially testable, no I/O. `SelftestDrift` names the
probe, the kind of drift (DecisionFlip { field, from, to } / RelevanceDrift
{ object_id, from, to }), and both values.

**CLI contract:** with `--baseline`, print `DRIFT` lines after the normal
report and **exit 1 on any drift** (drift is what the caller is asking
about; regenerating a baseline = rerun without `--baseline` and save the
`--json` output). Without `--baseline`, behaviour is unchanged.

**Red-first:** tests against the missing `diff_selftest_reports` (recorded
E0425), then: identical reports → empty; one flipped decision → one
DecisionFlip; relevance moved 0.02 → one RelevanceDrift; 0.005 → none.
Mutation: flip the epsilon comparison → relevance test fails → revert.

**E2E:** run selftest --json on the real corpus twice → no drift, exit 0;
then against a corpus with one doc's text edited → drift detected, exit 1.

## U6 — Per-corpus floor calibration (detailed spec)

**Goal:** the 0.66 default is corpus-agnostic; give callers a measured,
corpus-aware floor recommendation from THEIR corpus and THEIR probe set.

**Design:** new subcommand
`physis-core calibrate-floor --corpus <dir> --probes <file.json>` reusing
the U4 probes file (each probe: query + claim[0] relevant + rest noise).
For every probe, run one floor-0.0 context compile and collect labelled
relevances (label = `obj:noise` prefix convention, same as the harness).

**Core is a pure function** `calibrate_from_relevances(relevant: &[f32],
noise: &[f32]) -> CalibrationReport` with fields: relevant band
(min/max/n), noise band (min/max/n), `noise_ceiling`, `minimal_zero_noise_floor`
(noise_ceiling + 1e-3), `recommended_floor` (noise_ceiling + 0.008 — the
shipped default's measured margin over its own noise ceiling), recall at
the minimal floor and at the recommendation, and the full ROC table at
fixed steps. All numbers are **advisory measurements, never directives** —
the output says so on every render, per the project rule that advisory
scores are never trusted.

**CLI contract:** prints the bands, the ROC table, the recommendation, and
the explicit caveat; `--json` for the full report. Exit 0 always (this is
a measurement, not a gate).

**Red-first:** tests against the missing `calibrate_from_relevances`:
fixture relevant [0.70, 0.55] / noise [0.65, 0.40] → noise_ceiling 0.65,
recommended 0.658, minimal floor admits 1/2 relevant with 0 noise; noise
above relevant (inverted bands) → report says no separating floor exists
(`separable: false`) rather than recommending nonsense. Mutation on the
margin constant → test fails → revert.

**E2E:** run against the real corpus with `selftest-probes.json` (bge-small):
expect noise ceiling ≈ 0.6522, recommendation ≈ 0.66, recall at
recommendation 4/5 — matching the recorded n22 measurement. The e2e IS the
cross-check that the shipped 0.66 is what calibration would recommend.

## U7 — Relative diagnostic reporting (detailed spec)

**Goal:** the relative gate was rejected for *gating* (`8e602`) — but the
ratio is still diagnostic information: a claim weaker than the engine's own
best chunk is probably not adding anything. Make it visible without letting
it decide.

**Design:** `ObserverGateReceipt` gains `engine_relative: Option<f32>`
(serde-defaulted) = relevance / engine_best_score, filled in
`compile_context_with_observer` after the engine retrieval is known. The
receipt already carries `relevance`; the ratio needs no new plumbing beyond
the field and one fill-in pass.

**Semantics documented on the field:** ratio > 1 means the claim outscores
the engine's own best chunk for this query; < 1 means the engine already
retrieved something more on-topic than this claim. Advisory only — the gate
decision logic is untouched (that was measured and rejected).

**Red-first:** test that a fixture report's receipt carries
engine_relative ≈ relevance / engine_best_score (recorded E0425/E0609 on
the missing field), then implement. Mutation: fill with a constant → test
fails → revert.

**E2E:** one real-corpus context run; check the JSON receipts carry
sensible ratios (relevant claim > 1, noise < 1 on the n22 data).

## Execution order

U5 → U6 → U7, one commit each, red-first each, gates each, push each.
U1–U4 are done (see execution record below).

## Execution record (2026-11-20)

- U1 done (`bee01e7`): harness in-repo, acceptance 23/23 reproduce from the
  new location.
- U2 done (`bee01e7`): verdict header prepended; log sections untouched.
- U3 done (`bee01e7`): crate 0.2.0 → 0.3.0.
- U4 done: `physis-core selftest` shipped — `SelftestProbe{File}`,
  `run_selftest`, `cmd_selftest` (defaults: embedder-aware floor, auto
  reserve; `--json`; exit 1 on any failure). Red-first: three tests written
  against the missing API (recorded E0425 ×3), then implemented; mutation
  `passed = true` → 3/3 fail → reverted → 3/3 green. E2E on the real corpus
  with `selftest-probes.json` (5 probes from the n22 set): 5/5 PASS exit 0;
  negative control at floor 0.99 → 0/5, exit 1. Probes file shipped at
  `benchmarks/observer-fusion/selftest-probes.json`.
- Note: the sync bot committed its own in-progress work as `25c477a` (which
  also carries this unit's one-line `pub mod selftest;` registration); an
  earlier injection that slipped into `bee01e7` was reverted in `e6c014e`.
- U5 done (`d166958`): `selftest --baseline` drift watch. Design correction
  found by the e2e itself: claim relevances are query↔claim cosines and are
  structurally blind to corpus edits — the corpus-side signal is
  `EngineBestDrift` on `engine_best_score`. E2E: same corpus → no drift,
  exit 0; doc removed → EngineBestDrift 0.6314→0.6058, exit 1.
- U6 done (`da1a43f`): `calibrate-floor`. E2E cross-check: noise ceiling
  0.6522 (the recorded n22 value) and recommendation 0.6602 — calibration
  independently reproduces the shipped 0.66 from the 5-probe subset.
- U5.5 done (this commit): probe `expect: answer|defer` — selftest now
  verifies the engine MATCHES its measurement record rather than only
  that it answers. The shipped `selftest-probes.json` grew from 5 probes
  to the full 22 (14 expect-answer, 8 expect-defer from the recorded n22
  run); selftest passes 22/22 exit 0 against the deployed binary, and
  `calibrate-floor` on the full set reproduces the entire n22 record
  (bands 0.5543–0.8299, ceiling 0.6522, recommendation 0.6602, recall
  14/22) through the shipped tooling. Red-first: the expect test was
  written against the missing enum (recorded E0063), then implemented;
  mutation `Defer => answered` → fail → revert → green. Note: the sync
  bot transiently reverted the probes file mid-verification; the final
  state is the 22-probe version.
- U7 done: `ObserverGateReceipt.engine_relative` (advisory, serde-defaulted,
  gate untouched). E2E on the invariants query: relevant claim 1.106 (>1 —
  outscores the engine's own best chunk), noise 0.928/0.840. Process note:
  the implementation landed before the test this time (order slipped);
  the test was added immediately after and mutation-verified (constant
  1.234 → fail → revert → green).
