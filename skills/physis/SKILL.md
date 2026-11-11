---
name: physis
description: Use Physis for evidence-backed code context, structural search, token-budgeted context packets and verified recipe replay instead of guessing.
---

# Physis skill

Use these tools when a task involves understanding, changing or explaining a
codebase, or when the same verified workflow will run again.

## Decision policy

Use Physis when it offers better evidence, less context, or reusable verified
execution. Otherwise use ordinary tools.

Physis earns its call when one of these holds:

- You have not already read the code: the graph finds call sites, owners and
  neighbours that grep misses.
- You need a bounded packet under a token budget (`physis_context`), not a
  pile of files.
- A verified procedure already covers the task, so reasoning again is waste.

Use ordinary tools when:

- You already know where to look and one `search_files` or `read_file` answers.
- The repo was never observed, or the index is empty or stale — observe first,
  or just grep.
- The task is a one-off with no evidence gap and no reuse.

Never call Physis because it is installed. A call that returns nothing useful
is a cost, not a ritual.

## Order of operations

1. `physis_engine_status` if anything looks off (missing engine, zero nodes).
2. `physis_observe` on the repo before querying it. Re-run after large changes.
   `physis_grid` shows the current axes/shape and the active embedder.
   `physis_ingest` feeds the engine from fs/git (default) and, only when the
   operator enables them, terminal/agent/process/browser;
   `physis_observed` reads back what was logged, filtering out buckets the
   operator has not enabled.
3. `physis_query` for "where is X" questions; `physis_context` when you need a
   packet under a token budget (compiles straight from the corpus);
   `physis_snapshot` for graph health; `physis_chain` for a gap/drift overview.
4. `physis_classify` / `physis_diff` when the question is about the character
   of a file or whether a change altered it.
5. Check reuse before reasoning: `physis_recipe_lookup` is read-only and says
   whether an approved recipe exists for the request, what it depends on, and
   whether those dependencies are still tracked. If it says replayable, ask the
   operator to use `/physis replay <prompt>`; the model never calls replay or
   record, both are operator-only.
6. With `auto_learn` on, a session that passed `verify_cmd` becomes a candidate.
   Read it with `physis_candidates`. Only the operator can approve a candidate,
   using `/physis approve <id>`; approval makes the workflow replayable.

## Retrieval discipline

- Compare before trusting. For a "where is X" question, run the ordinary search
  you would have run anyway and check the graph's answer against it. When they
  disagree, say which you trust and why.
- Inspect uncertainty before editing. Query results carry scores and sources; a
  weak best hit, or a packet assembled from unrelated files, means stop, widen
  the query, or read the file directly.
- Measure the packet. Code mode reports `tokens` against `budget_reported`;
  docs mode adds `baseline_tokens`, `physis_tokens` and `compression_ratio`.
  If the packet is not smaller than reading the files, you bought latency for
  nothing — say so and fall back.
- An empty or zero-node result is a fact about the index, not about the code.
  Report it and re-observe; never narrate a guess as a finding.

## Reuse boundary: when not to think again

Two kinds of work, and telling them apart is the point of this skill:

- Fresh reasoning — no verified recipe exists, or its inputs changed. Observe,
  retrieve, edit, verify. Do not replay.
- Verified procedure — an approved workflow exists and its captured inputs
  still match. Do not reason again; replay it. Re-deriving a verified procedure
  costs tokens and drifts from the verified path.

The test is `physis_recipe_lookup` and the replay answer. `blocked: true` is not
a failure to work around: it is the signal that this is now fresh reasoning.
Repeat the task, then ask the operator to approve a new candidate.

Corollary: never replay to save time on a task whose inputs fall outside the
captured fingerprint. Freshness covers captured edited files up to 1 MiB and
nothing else; a blocked answer is the engine saying the world moved.

## Rules

- Never invent evidence Physis did not return. If the index is empty or the
  object is absent, say so and run `physis_observe` or widen the query.
- A replay that answers `blocked: true` is the correct outcome: the inputs
  changed since verification or evidence is missing. Repeat the task and request
  operator approval; do not force it.
  Freshness only covers captured edited files up to 1 MiB and can be empty;
  untracked dependencies are not checked.
  A candidate that fails at approval is the same signal: the world moved.
- Never approve a candidate you have not read: its steps are shell commands
  that will run again unattended.
- `terminal`, `agent`, `process` and `browser` ingestion sources are opt-in
  and refused unless the operator enables them in
  `plugins.entries.physis.settings.ingest_sources`. Never ask for them to be
  enabled and never work around a refusal.
- `physis_why` returns evidence around a label, not a causal story.
- The bundled embedder is semantic (BGE-small, CLS pooling) and on by default;
  the ontology cache makes calls fast after the first build. This does not make
  host commands network-free, sandboxed, or deterministic.
- When a classification is a tie or the best entry is weak, the engine may add
  a learned ontology entry (`Ontology expanded: ...`). Treat it as a generic
  placeholder, not a certified category.
- `replant_demo` is the fastest proof the engine works; use it before wiring a
  custom registry.
- replant uses the operator-configured `replant_tools` registry or the
  plugin-generated learned registry. A model cannot supply a registry path.
- Approval and replay are operator-only. A same-user model can modify the
  registry and approval manifest together; an on-disk checksum is not
  authentication.
- If a command is unsupported, report the limitation plainly instead of
  substituting a different tool that looks similar.
