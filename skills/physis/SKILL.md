---
name: physis
description: Use Physis tools to gather evidence-backed code context, causal chains, dependencies and structural diffs instead of guessing.
---

# Physis skill

Use the physis tools when a task involves understanding or changing a codebase.

Rules:
- Run `physis_observe` first on a repo that may have changed.
- Use `physis_context` with a concrete task and a small budget instead of pasting files.
- Use `physis_why` for provenance and gaps; treat UNKNOWN as real missing evidence.
- Use `physis_diff` before claiming a change is safe.
- If the engine reports an unsupported command, state that limitation plainly.
- Do not invent evidence that Physis did not return.
