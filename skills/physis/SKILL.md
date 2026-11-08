---
name: physis
description: Use Physis for evidence-backed code context, structural search, token-budgeted context packets and verified recipe replay instead of guessing.
---

# Physis skill

Use these tools when a task involves understanding, changing or explaining a
codebase, or when the same verified workflow will run again.

## Order of operations

1. `physis_engine_status` if anything looks off (missing engine, zero nodes).
2. `physis_observe` on the repo before querying it. Re-run after large changes.
   `physis_grid` shows the current axes/shape and the active embedder.
   `physis_ingest` feeds the engine from fs/git/terminal/agent/process/browser;
   `physis_observed` reads back what was logged.
3. `physis_query` for "where is X" questions; `physis_context` when you need a
   packet under a token budget (compiles straight from the corpus);
   `physis_snapshot` for graph health; `physis_chain` for a gap/drift overview.
4. `physis_classify` / `physis_diff` when the question is about the character
   of a file or whether a change altered it.
5. `replant_run` before re-planning a recurring task; `replant_record` only
   after a recipe's actions AND independent checks succeeded. Pass
   `artifacts=[...]` so a later replay is refused when those inputs changed.
   `/physis replay <prompt>` repeats a recorded recipe without a model turn.
6. With `auto_learn` on, a session that passed `verify_cmd` becomes a candidate.
   Read it with `physis_candidates` and record only the ones worth keeping:
   `replant_record(candidate=<id>)` or `/physis approve <id>`. Approving is the
   only thing that makes a learned workflow replayable.

## Rules

- Never invent evidence Physis did not return. If the index is empty or the
  object is absent, say so and run `physis_observe` or widen the query.
- A replay that answers `blocked: true` is the correct outcome: the inputs
  changed since verification. Re-verify with `replant_record`; do not force it.
  A candidate that fails at approval is the same signal: the world moved.
- Never approve a candidate you have not read: its steps are shell commands
  that will run again unattended.
- `physis_why` returns evidence around a label, not a causal story.
- The bundled embedder is semantic (BGE-small, CLS pooling) and on by default; the ontology
  cache makes calls fast after the first build.
- When a classification is a tie or the best entry is weak, the engine may add
  a learned ontology entry (`Ontology expanded: ...`). Treat it as a generic
  placeholder, not a certified category.
- `replant_demo` is the fastest proof the engine works; use it before wiring a
  custom registry.
- replant tools require an operator-owned registry (`tools=...`); a model may
  name a tool, never choose an executable.
- If a command is unsupported, report the limitation plainly instead of
  substituting a different tool that looks similar.
