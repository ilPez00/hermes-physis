# Plan — public story + documentation alignment

Date: 2026-09-28. Goal: make the repository tell the true story — a working
observe → world → lens → context → receipt → agent loop, with the closing
edge (action → delta → re-observe) honestly marked as the next wiring target.

## Verified ground truth (source of every claim in the docs)

Every capability claim in README/STATUS/DEMO was verified against source +
a live run on this checkout (2026-09-28, 2052 entities / 3628 relations):

| Capability | Status | Proof |
|---|---|---|
| observe/query/context/compile/diff/architecture/search/navigate/modules/agent/task/serve | working prototype | `apps/physis-cli/src/main.rs:23–133`, live runs |
| context compiler (candidates/RRF/dedup/hard budget/receipts) | working | `physis-search/src/context.rs`, live compile 658/1600 tok, 41 objects |
| semantic leg + honesty probes + disk memo cache | working, `embed-http` feature-gated; offline default labelled | `physis-projection/src/embed.rs` |
| MCP server, 42 tools, stdio+HTTP | working prototype | `crates/physis-mcp` |
| HUD (15 tabs, focus/lenses, magic textbox, 2-phase agent launch, live settings, `--snapshot`) | working prototype | `apps/physis-hud/src/main.rs` |
| agent delegation (opencode/hermes/jev-review, bounded 60 s) | working prototype | `physis_agent::delegate` |
| deterministic notebook ask | working | `physis-search/notebook.rs` |
| full notebook (cells/persistence), editor, world-object mutation, post-agent delta ingestion | missing → NEXT horizon | — |
| scale verdict B; router KILLED (R1); P2 narrowed claim; P10 negative | research, in-repo | `research/papers/PORTFOLIO.md` |

## Deliverables (one commit each)

1. `docs/plans/2026-09-28-public-story-plan.md` — this file.
2. `README.md` rewrite — one-liner, live demo transcript, Why Physis, loop
   diagram, honest status, self-observation, falsification culture.
3. `docs/STATUS.md` — capability matrix, PRODUCT/INFRA/RESEARCH/VISION split.
4. `docs/DEMO.md` — 5-act demo, CLI + HUD paths, Act V honest about the gap.
5. `docs/HALF_WIRED.md` — 11 rows, fixed 7-field format, no percentages.
6. `PLAN.md` — horizons (NOW/NEXT/THEN×3/LONG TERM); packet history appendix.
7. `ARCHITECTURE.md` — contradiction fixes only (loop diagram up, story intro).

## Gates

- every command in README/DEMO run against this repo, output pasted real;
- `cargo fmt --check` + clippy `-D warnings` clean tree per commit;
- relative-link grep check;
- push `origin main`, no force.

## GitHub description (primary)

> A software world model and context compiler: turns a codebase into typed
> objects, relations and provenance receipts, then compiles the smallest
> relevant context for humans and AI agents.

Topics: `rust` `ai-agents` `mcp` `code-intelligence` `context-engineering`
`developer-tools` `semantic-search` `knowledge-graph`
