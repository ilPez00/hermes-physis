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
3. `physis_query` for "where is X" questions; `physis_context` when you need a
   packet under a token budget; `physis_snapshot` for graph health.
4. `physis_classify` / `physis_diff` when the question is about the character
   of a file or whether a change altered it.
5. `replant_run` before re-planning a recurring task; `replant_record` only
   after a recipe's actions AND independent checks succeeded.

## Rules

- Never invent evidence Physis did not return. If the index is empty or the
  object is absent, say so and run `physis_observe` or widen the query.
- `physis_why` returns evidence around a label, not a causal story.
- The bundled embedder is lexical (random projection). Treat similarity as
  "shares tokens/structure", not "means the same thing".
- `replant_demo` is the fastest proof the engine works; use it before wiring a
  custom registry.
- replant tools require an operator-owned registry (`tools=...`); a model may
  name a tool, never choose an executable.
- If a command is unsupported, report the limitation plainly instead of
  substituting a different tool that looks similar.
