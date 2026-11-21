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

## Execution order

U1 → U2 → U3 as one housekeeping commit; U4 as its own measured unit with
red-first tests; U5+ stay plan-only until called.

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
