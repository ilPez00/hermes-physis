# Changelog

## Unreleased

- **Semantic observer floor -> 0.66, the measured zero-contamination point**: broadening to 13 held-out queries / 28 noise samples shows the relevance ROC on real text is SHALLOW (relevant 0.5543-0.7774, noise to 0.6522 — domain-adjacent hard negatives). No floor gives both high recall and zero contamination: 0.60 = 12/13 relevant + 3 noise; 0.66 = 9/13 + 0. The default now buys zero contamination and pays in recall; callers tune --observer-floor to move along the measured curve.

- **Semantic observer floor 0.6 -> 0.65**: broadening the real corpus from 3 to 7 held-out queries revealed the relevant and noise bands OVERLAP at 0.6 — a noise claim (0.6385) was admitted with its text verbatim in a packet. 0.65 sits in the measured zero-noise gap (0.6385, 0.6536]: 6/7 held-out queries answerable, zero noise; the one deferral is the safe direction. Third fixture-calibrated margin this session that real-corpus measurement overturned.

- **RP embedder fusion is now opt-in** (default floor 0.9 -> 0.99): measured on the real corpus, random-projection relevance overlaps noise completely (relevant 0.84-0.94, noise 0.84-0.92) — the old default admitted real noise (0.9016) while deferring real relevant content (0.8848). A path that cannot keep its gate's promise ships disabled. The semantic default stays 0.6 (validated on fixtures and the real corpus). Callers opt in with --observer-floor.

- **observer reserve, auto-sized** (`3735f67` follow-up): `--observer-reserve auto` sizes the headroom from the gated-in claims themselves (pre-gate + join-overhead bound), measured on the real corpus: all three held-out queries answerable AND the corpus-covered control query keeps its answer (fixed-250 lost it). Plugin accepts `observer_reserve: "auto"` or an integer. The reserve also reaches code mode: node-search caps its retrieval through the shared `observer_reserve_tokens` helper (single source for both paths), with the packet budget kept at the caller's full budget so the reserved headroom is usable. The helper takes a caller-supplied pricer — each path reserves exactly what it spends (measured: code mode kept an extra engine result once the header over-reserve was removed). A measurement defect was found and closed at the source: the retriever budgeted bare chunk tokens while the assembled packet pays 3 tokens per `\n\n---\n\n` join — `TokenFixedRetriever::with_separator` now budgets the assembled string (opt-in; direct `retrieve` callers keep legacy accounting), so no packet can exceed its nominal budget.

Observer fusion stays opt-in — measured, not assumed.

- **The ablation the fusion contract demanded now runs.**
  `observer_eval`'s helpers had unit tests but no caller over the real
  pipeline. Three fixture ablations now drive RagCorpus →
  TokenFixedRetriever → compile (baseline vs observer-fused, same budget)
  and assert measured numbers: a sparse corpus where the observer supplies
  the only relevant knowledge (recall 0.0 → 1.0, the win case); a dense
  corpus with ample budget where the win rides in with both noise hits
  selected and utilization rising (the cost case); a zero-slack budget
  where fusion selects identically to the baseline and the observer claim
  is dropped OverBudget (the conservative case). A can-fail mutation
  (fusion silently dropping observer candidates) fails the win and
  conservative cases; a first mutation attempt (removing the redundant
  `observer_sourced` re-forcing in `compile_fused`) did not fire —
  attribution is doubly enforced via `candidates_from_retrieval_claims`.
- **The unsolved seam, measured — then closed.** With partial slack, an
  observer's advisory score (0.99) outranked verified engine evidence and
  displaced the weakest relevant engine chunk — recall fell 1.0 → 2/3
  (measured in `partial_slack_lets_an_advisory_score_displace_engine_evidence`).
  `compile_context` now tiers its ranking: observer-sourced candidates rank
  strictly below engine-verified evidence (an advisory score is not
  authority), so fusion is a pure superset — observers fill slack only, and
  the fused packet always contains the baseline packet. Red-first: the
  no-displacement test failed against the old ordering; neutralizing the
  tier in a can-fail mutation makes it fail again. The old tripwire test is
  superseded by the contract tests; the pre-existing
  `budget_and_receipts_are_deterministic` expectation was updated to the
  tier contract (an observer score-tie no longer wins against engine
  evidence).
- **The noise-admission cost now has a measured answer: the retrieval-hit
  relevance gate.** Retrieval hits fell into the catch-all `gate_claim`
  (provenance only — zero relevance signal), and the fusion path never
  consulted gates at all. `gate_retrieval_hit` (in `gate.rs`) gates an
  observer retrieval hit against the query: provenance/attribution first
  (defer/reject with named causes), then query↔snippet cosine similarity
  against a caller-supplied floor — below the floor defers ("the observer
  may know something; this packet is not the place for it"), and the
  advisory score is recorded but can never raise the decision.
  `compile_from_retrieval_with_gated_observer_claims` is the fusion path
  that uses it: only accepted hits become candidates (still attributed,
  still ranked below engine evidence), and the gate results are returned
  alongside the packet so exclusions are visible, never silent.
  Measured on the dense-corpus fixture: the additive object (~0.93
  relevance) is admitted, both noise hits (0.892/0.858) are deferred with
  the floor named, observer_selected falls 3 → 1, and the recall win
  (+1/7) survives without the noise riding along. Can-fail verified:
  bypassing the gate makes the test fail (3 selected, not 1).
  **Embedder-dependence, measured and documented:** with the crate's
  random-projection embedder, same-language text clusters in a narrow band
  (relevant 0.934/0.917 vs topical noise 0.892/0.858), so an absolute floor
  separates only coarse overlap; with a semantic embedder (minilm via the
  `embed-onnx` feature) the floor separates topical relevance. The floor is
  a caller policy parameter for exactly this reason.
- **Production wiring: the gate is now callable from the CLI.**
  `map::compile_context_with_observer` runs the unchanged engine pipeline
  and appends accepted observer retrieval-hit blocks to the packet only
  while token budget remains (observer content fills slack, never
  displaces engine-selected text); every claim produces an
  `ObserverGateReceipt` (decision, included, relevance, reasons) carried
  on `ContextReport.observer_gated` (serde-defaulted, old JSON still
  parses). The `context` CLI command takes `--observer-claims <json>`
  (a file of candidate-claim packets) and `--observer-floor <f>`
  (default 0.9), prints gate receipts in text mode, and serializes them
  in JSON mode. Measured live: relevant hit accepted and in packet,
  noise deferred with the floor named. Can-fail verified: bypassing the
  gate in the map path makes the admit test fail (3 selected, not 1).
  The compile.rs gated-fusion path remains the packet-level API; the map
  path is the CLI-level one — both share `gate_retrieval_hit`, the
  measured, tested piece.
- **Plugin wiring: the whole path is agent-reachable.** `physis_context`
  (docs mode) accepts `observer_claims` (inline candidate-claim packets;
  passed to the engine via a cleaned-up temp file) and `observer_floor`;
  `observer_gated`/`observer_selected` are surfaced in the tool result.
  Validation: non-list claims and out-of-range floors are rejected with
  named errors (tests).
- **The floor default is embedder-aware — a real defect caught by wiring
  the path end-to-end.** The CLI's fixed 0.9 default was calibrated on the
  random-projection embedder. Through the plugin the engine loads the
  semantic bge-small model (`PHYSIS_MODEL_DIR`), where measured separation
  is relevant 0.891/0.644 vs noise 0.375–0.445 — the 0.9 default deferred
  a genuinely relevant hit. `context` now takes `--observer-floor` as an
  override and defaults by embedder kind (`default_observer_floor`):
  0.9 random-projection, 0.6 semantic. End-to-end through the plugin with
  bge-small and no floor: both relevant hits admitted with labelled
  blocks, all noise deferred, budget kept.
- **Code mode is agent-reachable too.** `node-search` gains the same
  `--observer-claims`/`--observer-floor` surface: accepted observer hits
  join the ranked results strictly below engine evidence (the packet tier
  via `compile_from_retrieval_with_gated_observer_claims` — its first
  production caller), filling only the token budget the engine selection
  left; receipts print in the same OBSERVER format. Output without claims
  is byte-identical to before. The plugin's code mode passes the claims
  through (same validation/temp-file handling) and parses the receipt
  lines into `observer_gated`/`observer_selected`. End-to-end through the
  plugin with bge-small: 2 engine + 1 observer result, noise deferred at
  the semantic floor (0.445 < 0.6).
- **Verdict: default-off stands, for a different reason.** The recall
  argument is gone — fusion can no longer lose recall, and unused budget
  slack is the only thing observer items consume. What remains is
  epistemic: an ample-slack packet picks up unverified observer content
  (noise included, measured) with no relevance filter before the budget.
  The gated fusion path is the relevance filter, but it requires an
  embedder and a floor policy, so it is opt-in by construction. Callers
  who cannot tolerate unverified content keep fusion off; callers who can
  choose between ungated fusion (strictly more recall, noise included)
  and gated fusion (relevance-filtered, embedder required).

Skill: the operating manual now teaches selectivity and the reuse boundary.

- **Decision policy.** Use Physis when it offers better evidence, less context,
  or reusable verified execution; otherwise use ordinary tools. The skill lists
  what earns a call (unread code, a bounded packet, an existing verified
  procedure) and what does not (a one-lookup answer, an empty or stale index, a
  one-off with no reuse), and states plainly that a call returning nothing
  useful is a cost, not a ritual.
- **Retrieval discipline.** Compare the graph's answer against the ordinary
  search you would have run anyway; inspect scores and sources before editing;
  measure the packet (`tokens`/`budget_reported` in code mode,
  `baseline_tokens`/`physis_tokens`/`compression_ratio` in docs mode) and fall
  back when it is not smaller than reading the files.
- **Reuse boundary.** Fresh reasoning (no verified recipe, or changed inputs)
  must not be replayed; a verified procedure with matching captured inputs must
  not be re-derived. `replant_show` and the replay verdict are the test, and
  `blocked: true` is the signal to switch from reuse to reasoning, not a
  failure to work around.

Integration: the plugin now acts as an evidence substrate, not only a tool the
agent may consult.

- **Hooks resolve the session's directory, not the gateway's.** `pre_llm_call`
  (`pre_context`), `post_tool_call` (`auto_observe`) and `pre_verify` derived
  the repository from the process cwd. Hermes runs plugins in the gateway
  process, whose cwd is the install root shared by every session and is never
  `chdir`-ed, so whenever the session's workspace was anywhere else the hooks
  targeted the wrong repository: `pre_context` never injected and `auto_observe`
  scanned the install tree. They now read `PluginContext.session_cwd()` (with a
  process-cwd fallback for standalone calls), and a relative tool `path`
  resolves against it too.
- **One graph per repository.** The store key is the canonical repository root
  (nearest `.git`/`.hg`/`.svn`, else the path), so a `post_tool_call` re-index
  of `src/` lands in the same graph the repository root queries. Previously the
  hook keyed the store on the edited file's parent, so an edit silently created
  a second store the repository-wide queries could not see. `physis_observe`
  now reports `repo_root` and `store`.
- **Verification runs from the right directory, and a timeout is not a pass.**
  `pre_verify` used the first changed file's parent as the working directory and
  let `subprocess.TimeoutExpired` fall into the bare `except`, so a hung check
  finished the turn as if it had passed. It now runs from the changed files'
  repository root, uses a separate `verify_timeout` setting (default 25s), and
  reports a timeout as a failure. `OSError` is reported too.
- **Opt-in pre-reasoning context.** `pre_context: true` makes `pre_llm_call`
  inject a small retrieval packet into the user message before the model
  reasons. It reads an existing, fresh index only; it skips a stale or missing
  store, weak retrieval (`pre_context_min_score`), a missing engine, or a
  session with no repository; it never ingests a source; and it does not repeat
  an identical packet within a session.
- **Read-only recipe lookup.** `physis_recipe_lookup(prompt)` tells the model
  whether an approved recipe exists, which fingerprint files its steps depend
  on, whether they still exist, and whether the recorded inputs still match.
  It maps to replant's `show`, never `run`, and cannot approve anything.
- **Agent-in-the-loop benchmark.** `benchmarks/agent_loop/` runs 20 tasks
  (fresh / grep-negative-control / reuse-probe) under three `HERMES_HOME`
  configurations (plain, skill, auto) and reports verified outcomes per token,
  cost, latency and tool mentions. It is a harness, not a result: the full
  60-run comparison has not been executed here.

## 1.7.1 — 2026-11-11

Catalog admission fix: sensitive ingestion is opt-in.

### Sensitive sources are opt-in

- `physis_ingest` refuses `terminal`, `agent`, `process`, `browser` and `all`
  unless the operator lists the source in
  `plugins.entries.physis.settings.ingest_sources`. The default is
  `["fs", "git"]`; the refusal happens in the tool before any engine runs, so
  a prompt-injected call cannot pull shell history, another agent's
  transcripts, process command lines or browser history into the provider
  context.
- `physis_observed` refuses a disabled source by name and filters disabled
  buckets out of an unfiltered read, so a sensitive observation recorded
  before the gate cannot be read back through it.
- `plugin.yaml` discloses the four sources, what each reads, and the setting
  that enables them.

### Fixes

- `bin/SHA256SUMS` now matches the shipped `replant` binary (the line was
  stale).
- Plugin state moved from `~/.physis` to Hermes' profile data dir
  (`~/.hermes/plugin-data/physis`), so separate profiles keep separate state.

## 1.7.0 — 2026-11-10

The release the catalog entry points at. Everything below was already in the
tree; this version number exists so the pin and the manifest agree.

### Security boundary

- **Model-facing raw execution removed.** A model can no longer name an
  executable or trigger a raw command: the learned registry maps every step to
  the bundled Python running a fixed wrapper (`stepN`, `verify`).
- **Approval is an operator action.** Turning a session into a replayable
  recipe needs `/physis approve <candidate-id>`; the approve verb is deliberately
  not a registered model tool, and candidates live in the plugin profile
  directory, never in the repository.
- **State that decides what runs is never model-supplied.** Registry, store and
  candidate paths come from operator config only.
- **Fail closed on ambiguity.** A stale or changed input blocks replay instead of
  guessing: `status: blocked`, with the changed paths named.

### Learning from verified work

- `auto_learn` turns a session's shell steps and edited files into a workflow
  candidate, but only after the operator's own `verify_cmd` exits zero. A recipe
  without a recorded intent is not written (there would be nothing to match it
  against later).
- Candidates are content-addressed and re-validated on load; a tampered or
  repo-planted candidate is not approvable.

### Replay

- Replay re-executes the real actions and the independent checks; it never
  returns a cached success. `provider_calls` is `0` on a replay by construction,
  and the demo test asserts it.
- Replant fingerprint limit raised to 256 MiB so learned candidates work under
  large interpreters.
- `physis_trace` exposes replay diagnostics (ported ReasonTrace) without
  widening the execution surface.

### Honesty fixes

- Security and provenance claims in the docs were qualified to match what the
  code does: approved commands run with the configured user's filesystem and
  network access; Python entry points need an interpreter; replay freshness
  fingerprints captured files, not arbitrary dependencies.
- `physis-core` rebuilt for host glibc 2.41 compatibility.

### Bundled engines

`bin/physis-core` and `bin/replant` are built by the `build-binaries` workflow
from this commit and carry GitHub build-provenance attestations, declared in the
catalog entry under `attested_binaries`. Verify with:

```sh
gh attestation verify bin/physis-core --repo ilPez00/hermes-physis \
  --source-digest <this commit> --predicate-type https://slsa.dev/provenance/v1
```
