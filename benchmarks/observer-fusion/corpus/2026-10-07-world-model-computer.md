# World-model computer — the 2026-10-07 programme

Current execution order: [audit amendment with bounded loops and pseudocode](2026-10-07-physis-audit-execution.md).
This programme retains its original baseline and research rationale. Subsequent
source already contains cross-file resolution and structured `Counterexample`
objects; do not reassign its P1/P2/P3 from historical descriptions alone. The
18-pair frozen benchmark establishes fixture performance, not general caller
completeness. A comprehensive receipt verifier remains a proposed capability.

Mission: evolve Physis from a code-context tool into a trustworthy,
inspectable world-model computer. Baseline: `d64d9a0` (the
physis-computer 1.2.0 staging commit; HEAD when this plan was
written). `physis-computer` stays a derived/public product, never
the source of truth.

The loop this programme strengthens:

    OBSERVE → WORLD → FOCUS/LENS → CONTEXT → PROPOSE → VERIFY
    → COUNTEREXAMPLE → REPAIR → MINIMAL VALID DELTA → RE-OBSERVE
    → MEMORY

## Governing rule (unchanged, restated because it is the point)

    unique resolution → relation
    ambiguous resolution → explicit unresolved candidate
    impossible resolution → no relation

Never guess. A derived result never silently becomes canonical
truth. Physis prefers "I cannot establish this relation" over "I
generated a plausible relation."

## Why now — the measured weakness

The licensed N=32 benchmark (`research/product/results.json`,
2026-10-06) measured **caller recall 0.132** on this repository.
Root cause, stated in the scanner's own doc comment
(`crates/physis-discover/src/scan.rs:450-470`): method calls were
not represented at all until the same-file recovery, and
cross-file / cross-crate method resolution does not exist. Rust
is idiomatically method-call-shaped, so the CALLS graph under-reports
blast radius, and `physis.delta.affected` says "0 callers" for
methods called from other files.

The fix is not "guess harder". It is two-stage representation:

1. **Discovery** emits direct resolved `CALLS` edges where resolution
   is certain, and **unresolved call candidate observations** for
   every other call site, carrying enough information for later
   world-level resolution: caller entity, method/function name,
   receiver text, source file, module/package/crate, import/use
   information, candidate namespace, language, source span,
   provenance, confidence/resolution status.
2. **The world** (`physis-query`'s fold) resolves candidates against
   the whole workspace — same-file, cross-file, cross-module,
   imported, inherent-impl, trait (when uniquely resolvable),
   crate-qualified, module-qualified — and emits either a `CALLS`
   edge (certain), a `MAY_CALL` edge to the real candidate symbols
   (ambiguous), or nothing (impossible; the candidate observation
   remains in the log as the record that the call exists).

`MAY_CALL` is never promoted to `CALLS` silently. Impact queries
(`used_by`) keep their current certain-edge semantics; `MAY_CALL`
edges are visible through `physis.object.relations` and the
benchmark's candidate metrics, always distinguishable by predicate.

## The slices

| Slice | Priority | What lands |
|---|---|---|
| 1 | P0 | Frozen benchmark: corpus, oracle, target list, hashes, metrics, BEFORE/AFTER |
| 2 | P1+P2 | Two-stage Rust call representation + world-level resolution |
| 3 | P3 | Structured `Counterexample` objects in `physis solve` |
| 4 | P4 | Proof-carrying `ChangeReceipt` + `physis verify-receipt` |
| 5 | P5 | Multi-candidate minimal verified repair with deterministic ranking |
| 6 | P6 | Minimal static+dynamic evidence fusion (`CALLED`/`COVERED_BY`/`FAILED_IN`) |
| 7 | P7 | Declarative world-schema PoC (one toy domain: local services) |
| 8 | P8 | Small-model amplification bench harness |

Each slice is one commit (or a small commit stack), with tests, and
each is measured against the frozen benchmark from slice 1.

## Slice 1 — frozen benchmark (P0)

The current benchmark samples from a moving repository. A benchmark
that moves cannot prove anything.

Lands:

- `research/benchmark/corpus/` — a small, fixed Rust corpus with a
  known call graph: same-file free functions, same-file methods,
  cross-file functions, cross-module functions, imported functions,
  inherent impl methods, trait methods, crate-qualified calls,
  module-qualified calls, plus deliberately ambiguous cases (a method
  name defined in two impls) and impossible cases (external crate
  calls, macros).
- `research/benchmark/oracle.json` — hand-authored ground truth:
  for each target symbol, the exact set of callers. The oracle is
  authored from the corpus source, not derived from Physis.
- `research/benchmark/manifest.json` — target list + sha256 of every
  corpus file + oracle hash. The harness fails if the corpus or
  oracle changes without the manifest being regenerated.
- `scripts/bench-world-fidelity.py` — observes the corpus into a
  throwaway `PHYSIS_STORE_DIR`, runs the impact query for every
  target, and computes: caller precision, caller recall, F1, invented
  edges, unresolved-but-correct candidates, resolution coverage,
  impact latency, context size. Writes
  `research/benchmark/results-<tag>.json`; fails on corpus/oracle
  hash mismatch.
- BEFORE numbers recorded against the pre-change engine.

Metrics are defined so that precision can never exceed 1 by
invention: an invented edge (a CALLS edge the oracle does not
contain) counts against precision; an unresolved-but-correct
candidate (a MAY_CALL or candidate observation pointing at a real
oracle caller) counts toward recall's "known-but-not-certain"
bucket, never toward certain recall.

## Slice 2 — two-stage Rust call representation (P1+P2)

**Discovery (`physis-discover/src/scan.rs`):**

- The definition list used for call resolution becomes AST-derived
  (`syn`), fixing the line-scanner false-uniqueness residual
  documented at `scan.rs:463-470` (a one-line `impl A { pub fn
  n(..) .. }` was invisible to the line scanner, so `n` could look
  unique when it was not). Definition *observations* stay
  line-based — identity and the HUD golden do not move; only the
  uniqueness count becomes honest.
- Every call site the scanner cannot resolve with certainty emits a
  candidate observation: `rel|MAY_CALL|{file}::{caller}|{callee}`
  with a JSON body carrying kind (fn/method/assoc), receiver text,
  qualifier, impl owner (for `self` receivers), line, language,
  span, provenance, status.
- Direct `rel|CALLS|` emission is unchanged for the cases that are
  certain today (same-file unique free functions and methods).

**Kernel (`physis-kernel/src/relation.rs`):** `MayCall` joins
`RelationType` (`MAY_CALL`), so the fold can parse candidate
observations instead of silently mapping them to `DEPENDS_ON`.

**World (`physis-query/src/engine.rs`):**

- `rel|MAY_CALL|` observations fold into a candidate queue with
  their parsed metadata — no target entity is created for a bare
  candidate name (a bare method name is not an entity).
- The second pass (`resolve_pending_calls`, extended) resolves:
  - bare names through the file's `IMPORTS` edges (imported
    functions: `use crate::foo::bar;` then `bar()`);
  - type-qualified associated calls (`Foo::new()`) to the unique
    method `new` on the workspace type `Foo`;
  - module-qualified calls (`mod::fn()`) through the workspace's
    module declarations;
  - `self`/`Self`-receiver method calls through the caller's impl
    owner;
  - locally-typed receivers (`let x: Foo = …; x.bar()`) to the
    unique `Foo` method.
- Ambiguity (two or more real candidate symbols) materializes
  `MAY_CALL` edges to each real candidate — visible, never
  promoted. Impossibility (external crate, macro, no candidate)
  materializes nothing; the observation remains.
- `used_by` (impact) keeps certain-edge semantics: `CALLS`,
  `DEPENDS_ON`, `DEFINES`, `DERIVED_FROM`. `MAY_CALL` rows surface
  through `physis.object.relations`, where every edge is already
  predicate-labelled.

**Known remaining failure cases (stated, not smoothed over):**

- Receiver types that require data-flow analysis (field access
  chains, `match` bindings, generic instantiation) are not inferred;
  those calls stay candidates.
- Trait-method dispatch where two impls provide the same method
  name stays ambiguous by design.
- Macros and `dyn` dispatch are not modelled.

## Slice 3 — structured counterexamples (P3)

`physis-task/src/solve.rs` gains `Counterexample { goal,
attempt_revision, verification_level, check_name, command,
exit_status, bounded_output, affected_entities, violated_constraints,
evidence_refs }`. Every failed check in every attempt becomes one.
The next editor prompt receives structured constraints derived from
**all** previous counterexamples, not the previous attempt's prose
stderr. Stop-at-first-failure is kept: the ladder never runs an
expensive rung a lower rung already disproved.

## Slice 4 — proof-carrying change receipt (P4)

`ChangeReceipt` (JSON, deterministic key order, hashes where
appropriate, replayable commands, no prose required to establish
correctness) is emitted by every successful solve: goal, world
revisions, focus entities, context hash/sources, assumptions,
constraints, candidate id, proposed actions, filesystem and world
delta, checks declared/reached/passed/failed/unreached, invariants,
counterexamples seen, repairs attempted, blast radius before/after,
delta score, backend, latency, cost, evidence refs.

`physis verify-receipt <receipt>` independently re-runs what can
still be verified and reports one of: `VERIFIED`, `STALE WORLD`,
`CHECK FAILED`, `EVIDENCE MISSING`, `UNREPLAYABLE`.

## Slice 5 — multi-candidate repair (P5)

`solve` optionally generates/evaluates N candidate repairs (not N
identical model calls: candidates come from distinct prompts or
distinct repair strategies). Ranking is deterministic evidence:
verification completeness > invariant satisfaction > absence of new
unresolved inconsistencies > blast radius > delta score > touched
objects/files > public-interface preservation > cost/latency. A
candidate that fails verification can never outrank one that passes.
The ranking receipt is exposed.

## Slice 6 — static + dynamic evidence (P6)

One ingestion path: a machine-readable test trace (JSONL of
`{test, status, covered_symbols}` or a coverage map). Facts land as
observations and fold to `CALLED` (runtime-observed execution),
`COVERED_BY` (observed test execution), `FAILED_IN` (observed
failing execution). `CALLS` stays statically established; `MAY_CALL`
stays a static candidate; runtime facts never overwrite either.
Impact and solve prefer certain+observed > certain > candidate,
while preserving all three.

## Slice 7 — world schemas as data (P7)

A small declarative schema (EntityType, RelationType, Lens, Action,
Invariant) compiled into ordinary bounded world operations. Schemas
are projections/configuration, not kernel ontology truth; malformed
schemas cannot corrupt existing state; schema-defined actions still
go through capability checks and receipts. One toy domain: local
services/processes, because the host adapter already observes them.
Proves the same HUD/query/action machinery operates on it without a
bespoke dashboard.

## Slice 8 — small-model amplification bench (P8)

A reproducible A/B/C/D harness (small model + raw repo, small model
+ Physis, stronger model + raw repo, stronger model + Physis) on the
same frozen tasks, measuring task success, verification success,
retries, context tokens, changed lines, wrong files touched, time,
cost where measurable, unresolved constraints, hallucinated
entities/files/symbols. The primary question is whether a weaker
model over a high-quality Physis world approaches or exceeds a
stronger model over raw context. If it does not, the harness says
so. Without metered model access on this box, the harness lands
with its protocol and its outcome reported UNVERIFIED.

## Non-goals (carried from the mission)

No new chat UI, no decorative HUD panes, no autonomous multi-agent
swarms, no hundreds of primitives, no increased agent autonomy
without stronger verification, no silent MAY_CALL→CALLS promotion,
no model confidence as world truth, no models in the kernel, no
second state store, no second graph, no bypass of the Runtime
canonical write path, no replacement of the
observation/event/revision architecture, no demo optimization before
benchmark correctness.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets
--exclude physis-license -- -D warnings`, `cargo test --workspace
--all-targets --exclude physis-license`, and the frozen benchmark.
Any slice that cannot run its gate reports BLOCKED/UNVERIFIED
explicitly. Completion is not claimed on inspection.

## Dogfooding

Every slice runs the loop on this repository itself: observe, focus,
blast radius, bounded context, implement, ladder, re-observe, delta,
receipt. Wherever Physis fails to model its own codebase, that is
first-class product evidence and is recorded in the benchmark output,
not worked around.

## Status — end of 2026-10-07 (handoff)

Landed and measured:

- **P0 frozen benchmark** — `research/benchmark/` (corpus, oracle,
  manifest) + `scripts/bench-world-fidelity.py`. The harness fails
  closed on corpus/oracle hash drift, measures caller precision,
  recall, F1, invented edges, unresolved-but-correct candidates,
  withheld-external violations, latency and context size, and pins
  the exact binary in each receipt (`engine.cli_sha256`).
  - BEFORE (binary built from the clean pre-fix checkout, the
    worktree at `a86b98b`): **recall 0.278, F1 0.435, 5/18 pairs**.
  - AFTER (`6fc40cc`): **recall 1.000, F1 1.000, 18/18 pairs,
    0 invented edges, 0 withheld-external violations**.
  - Compare: `python3 scripts/bench-world-fidelity.py --compare
    research/benchmark/results-head.json research/benchmark/results-after.json`.
- **P1+P2 two-stage call representation** — commit `904592c`
  ("fix(calls): resolve cross-file callers, or say UNKNOWN"):
  AST-derived uniqueness, `rel|MAY_CALL|` candidate observations
  with JSON metadata, kernel `MayCall` variant, world-level
  resolution in `resolve_pending_calls` (import scope, OWNED_BY
  impl census, crate/module/type-qualified paths), import-path
  phantom entities excluded from the name index, `&dyn Trait`
  receivers recorded as the trait, trait-impl methods owned under
  the trait. Gates: fmt, clippy, full `cargo test` green.
- **Operational fix** — `skip_dir` now skips `.target`: run
  receipts under `research/product/runs/*/` carry full cargo build
  trees, which spent the walk's whole 4000-file budget and made
  the self-scan report a handful of packages (`truncated: true`).
  Bootstrap tests green again.

Next, in order (each slice = one commit + tests + benchmark):

1. **P3** — `Counterexample` struct in `crates/physis-task/src/
   solve.rs`; `build_editor_prompt` (solve.rs:708 at plan-writing
   time) receives constraints from **all** counterexamples, not
   the last attempt's stderr. Design in this doc, slice 3.
2. **P4** — `ChangeReceipt` + `physis verify-receipt` verb in
   `apps/physis-cli/src/main.rs`. Design in this doc, slice 4.
3. **P5** — multi-candidate repair with deterministic ranking.
4. **P6** — static+dynamic evidence fusion (`CALLED`/`COVERED_BY`/
   `FAILED_IN`).
5. **P7** — declarative world-schema PoC (local services domain).
6. **P8** — small-model amplification bench. Without metered model
   access on this box it lands with its protocol and reports
   UNVERIFIED.

Handoff notes:

- The P1+P2 implementation was written by a concurrent agent in
  this same checkout; the frozen benchmark is what verified it.
  A pre-fix binary lives in the worktree `/tmp/physis-head`
  (commit `a86b98b`, own target dir `/tmp/physis-head-target`)
  for A/B measurement; remove it when done.
- `tasks/todo.md` carries uncommitted changes from the loop
  tooling; `research/product/runs/` holds untracked run receipts
  (deliberately kept, never committed — they contain `.target`
  build trees).
- Gates at handoff: `cargo fmt --check` clean, `cargo clippy
  --workspace --all-targets --exclude physis-license -- -D
  warnings` clean, `cargo test --workspace --all-targets --exclude
  physis-license` fully green, benchmark 18/18.
