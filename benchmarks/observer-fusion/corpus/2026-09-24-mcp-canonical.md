# Plan — Canonical Physis MCP (2026-09-24)

## Research findings (initial state)

- Current MCP: `apps/physis-cli/src/mcp.rs` (220 lines). NDJSON JSON-RPC 2.0,
  protocol `2024-11-05`, tools: initialize/ping/tools list+call, 11 tools,
  results flattened to `items.join("\n")` text. No resources, no arg validation
  beyond dispatch, no health/capabilities.
- Old canonical surface: `PROLIVE /home/gio/physis_pro/src/ported/system_mcp.rs`
  (274 lines) — 13 tools (`physis_capabilities inspect find pack read history
  remember predict runs usage list tasks task`), **argument validation against
  inputSchema**, **structured JSON results**, audit hook (`system.mcp.call`).
  Backed by a Workspace with its own notes/nodes store; physis-next has the
  equivalent primitives: `physis-store` (append-only Observation/Event JSONL),
  `physis-runtime` (snapshot/diff/architecture), `physis-search` (unified RRF),
  `physis-query` (context packets, graph), `physis-registry` (modules+adapters).
- Runtime primitives verified: `Observation {seq, at, source, subject, body,
  produced_by}`; `Runtime::observe/load/diff/architecture/search/context`;
  `SearchRequest{mode,limit,kinds}`; `RelationType` graph; task worktrees
  (`physis-task::new_task/verify_task`); config (`RuntimeConfig::save_dirty`).
- Protocol: no hard blocker found for `2025-06-18`; the old server negotiates
  NDJSON stdio the same way. Upgrading is a constant + resources/tools capability
  advertisement.

## Architecture chosen

New crate `crates/physis-mcp` (thin protocol projection; no domain logic):

```
crates/physis-mcp/
  src/lib.rs        // façade: serve(), Server struct
  src/protocol.rs   // JSON-RPC dispatch, 2025-06-18, capabilities, batching-out
  src/schema.rs     // typed response structs (serde) — the structured-output contract
  src/tools.rs      // tool catalogue + arg validation (ported from old serve) + dispatch arms
  src/resources.rs  // physis:// URI scheme: list/read/templates
  src/transport.rs  // stdio NDJSON loop (same transport as today + old serve)
  src/tests.rs + tests/
apps/physis-cli/src/mcp.rs  // becomes a 10-line shim: Cmd::Serve -> physis_mcp::serve (old file's logic MOVED, not deleted)
```

Tool dispatch rule: `MCP arm -> typed service call (physis_agent / runtime /
registry / store)`. Every tool returns `{structured: <typed JSON>, text: <human>,
is_error: bool}` via `schema::ToolOutcome`.

## Slices (each = implement + test + gates + commit)

- **P0.1 foundation (this session's first commit):** crate skeleton, protocol
  2025-06-18, arg validation, structured outputs for the existing 11 tools,
  `physis.health`, `physis.capabilities` (old), memory trio ported
  (`physis.history/remember/predict` over the Observation log with
  actor/outcome/evidence and "reported ≠ certified" semantics), file-bound
  `physis.read`/`physis.find`/`physis.pack` mappings, compatibility matrix.
- **P0.2 information surface:** object API over kernel entities/relations
  (`object.list/get/relations` read-only first; create/update/delete behind
  observation-append semantics, destructive annotated), `physis.resources`
  (workspace/current, invariants, config, object/<id>, module/<id>), nav tools
  over `neighbors` (pin/relate need a store-backed relation log — design first).
- **P0.3 cognition:** reasoning tools over `physis-reasoning`
  (hypothesis.list/get/propose/evidence/compare, contradictions, coherence,
  predict) — hypotheses live in a new `hypotheses.jsonl` store log; delta tools
  over `physis-delta` + `Runtime::diff` (`delta.since/compare/affected`).
- **P0.4 development:** task.* over `physis-task` (create/list/get/verify
  first; accept/reject explicit, never auto-merge), test/invariant API
  (`test.for`, `invariant.check` — bounded run via the action layer), action
  layer (`actions.list/get/execute` over registered commands only), adapters.*
  over `available_adapters()`, delegate.plan/run/result.
- **P0.5 self-description:** config.* over `physis-config` (get/schema/diff/
  reload; set uses `save_dirty`), module.* over registry
  (list/get/capabilities/health; enable/disable persists a disabled-set file).
- **P1 (explicitly out of scope until P0 stable):** subscriptions/events,
  long-running operations, HTTP transport, Aion bridge.

## Security model (enforced from P0.1)

- Tool definitions carry `access: readonly|mutating|destructive|external`.
- `physis.read`: workspace-root confined, symlink-resolved prefix check, byte cap.
- `delegate`: registry-resolved ids only (already enforced; keep).
- `remember`: appends observations; never asserts facts (kernel rule).
- Unknown method/tool/adapter fail closed with named knowns.
- No git mutation outside `physis-task` worktrees; no merge operations at all.

## Compatibility matrix (old system serve -> canonical)

| old | new | status | parity notes |
|---|---|---|---|
| physis_capabilities | physis.capabilities | P0.1 | workspace stats from Runtime |
| physis_inspect | physis.inspect | P0.1 | architecture() inventory |
| physis_find | physis.find | P0.1 | SearchService (was lexical BM25; now RRF, method reported) |
| physis_pack | physis.pack | P0.1 | ContextPacket w/ token budget + truncation |
| physis_read | physis.read | P0.1 | workspace-confined byte-capped file read |
| physis_history | physis.history | P0.1 | observation-log query (actor/at/body structured) |
| physis_remember | physis.remember | P0.1 | Observation append, outcome enum, "reported evidence" |
| physis_predict | physis.predict | P0.1 | prior matching observations + naive failure rate |
| physis_tasks / physis_task | physis.task.list/get | P0.4 | worktrees + task.json |
| physis_list | physis.object.list | P0.2 | entity inventory |
| physis_runs / physis_usage | (no equivalent store yet) | P0.4 (action layer) | explicit unavailable error naming the gap until then |
| 11 existing tools | unchanged names, structured payloads added | P0.1 | no capability loss |

## Testing plan

- protocol tests: initialize/notifications/ping/tools list+call/resources
  list+read/malformed JSON/unknown method/unknown tool/invalid args/unknown
  adapter (fail-closed).
- capability tests per domain as domains land.
- security tests: read escape (`../`, absolute, symlink) rejected; delegate
  arbitrary bin rejected; remember never mutates snapshot.
- integration: spawn real stdio server (bin), drive initialize→tools/list→
  resources/read→search→context→remember→history over the pipe.
- acceptance test (end of program): fixture workspace + fake adapter, full loop
  per task §27.

## Gates

`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings &&
cargo test --workspace --all-targets` (default + `--features quarry` for
clippy), on a clean tree, every commit.
