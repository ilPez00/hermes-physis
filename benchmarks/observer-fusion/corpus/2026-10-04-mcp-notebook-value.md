# MCP notebook value implementation plan

> Execution stopped 2026-10-05: operator requests plans, loops and checks only. Isolated drafts remain uncommitted; no new release/host certification. See [current machinery plan](../commercial/MARVELOUS_MACHINERY_PLAN.md).
> **For agentic workers:** Execute task-by-task with bounded OpenCode workers.
> User-selected method: `opencode/space-bunny-free`, `--variant max`, `--pure`.

**Goal:** Deliver resumable notebook evidence, task prompts and editor setup.

**Architecture:** Existing notebook JSON stores references/goals, existing graph
resolves them, current source validates them. A small notebook product module
returns data; a separate workflow module owns static prompts/config rendering.
Server wires both into existing entitlement and protocol handling.

**Tech Stack:** Rust/serde/Syn2, standard MCP2025-06-18, Python wire tests, Git.

**Spec:** [design](../commercial/MCP_NOTEBOOK_VALUE_DESIGN.md).

## Global constraints

- Read-only MCP; twelve existing tools stay compatible, one additive notebook tool.
- One existing notebook store/graph. No cloud, paid providers or source writes.
- Exact limits/errors/budgets in spec; all omissions visible, unverified != pass.
- Workers use user-selected free model/max; no automatic provider fallback.
- Isolated worktrees, bounded packets, no secrets/pro credentials in provider inputs.
- Parent alone integrates, builds releases, runs broad gates and commits.

## Review focus

- Corrupt/missing/symlinked/mismatched notebook must not look empty or trusted.
- Deleted functions, duplicate names, malformed Rust and large files cannot look fresh.
- Unicode, huge notes, many refs and zero budgets preserve wire/content bounds.
- Unlicensed resources/tool data access must fail closed; public prompts disclose no data.
- Unsaved editor buffers differ from disk; setup output must use exact argv/root.

## Task1 — notebook reader and evidence packet (worker NB)

Files: create commercial/physis-context-mcp/src/notebook.rs; modify product
Cargo.toml/Cargo.lock/COMMERCIAL_ALLOWLIST.txt and main.rs only to declare module.
Consumes physis_notebook::{Notebook,Cell,validate_id,validate,notebook_path},
physis_query::{Snapshot,QueryEngine}, existing count_tokens/file_freshness and Syn2.
Produces `pub fn list(root:&Path,offset:usize,limit:usize)->Result<Value,Value>`
and `pub fn resume(root:&Path,snap:&Snapshot,id:&str,budget:usize)->Result<Value,Value>`.

- [ ] Write unit tests for strict read/errors, escaping symlinks, exact targets,
  fresh/stale/deleted/ambiguous/unparsed refs, notes as data, zero/positive bounds.
- [ ] Capture failing test before implementing each behavior. Parent can run
  filters if shell permission or dependency wiring prevents worker execution.
- [ ] Implement pure read module; no Notebook::load fallback or new persistence.
- [ ] Run `cargo test --manifest-path commercial/physis-context-mcp/Cargo.toml notebook`.
- [ ] Return HANDOFF.md + receipt with changed files, actual checks and uncertainty.

## Task2 — prompts and printable editor setup (worker FLOW, parallel with NB)

Files: create product src/workflows.rs and docs/NOTEBOOK.md, docs/EDITOR.md.
No edits to main.rs/server.rs/manifests. Parent handles those shared interfaces.
Produces `pub fn prompts_list()->Value`, `pub fn prompt_get(name:&str,args:&Value)
->Result<Value,Value>`, `pub fn setup_config(client:&str,binary:&Path,root:&Path)
->Result<Value,Value>`. Error Value contains status/message; prompts follow MCP.

- [ ] Unit-test three exact names, missing/invalid args, untrusted data boundaries,
  unknown client, command paths with spaces and correct VSCode/OpenCode formats.
- [ ] Implement static templates and JSON rendering, no file mutation.
- [ ] Describe notebook native commands verified from HUD, explicit refresh and
  unsaved-buffer limits; don't advertise already-tested VS Code GUI.
- [ ] Parent wires module then runs `cargo test ... workflows`.
- [ ] Return scoped handoff/receipt, no unrun-pass claims.

## Task3 — protocol integration and useful wire proof (parent + worker WIRE)

Parent files: product main.rs/server.rs/Cargo.toml/Cargo.lock/catalogue.snapshot.json;
source release/boundary/acceptance scripts, product README/CHANGELOG.
Worker WIRE creates scripts/commercial-notebook-check.py only (own tests/receipt).

- [ ] Write persistent-session wire checks against agreed interfaces before wiring:
  prompts/resource capabilities, licensed vs unlicensed reads, strict errors,
  fresh->stale->observe->resume, deleted symbol, restart, zero budget, huge notes,
  no source/store mutation, protocol recovery and resource/tool agreement.
- [ ] Capture old1.1.0 failure for missing notebook surface (real red evidence).
- [ ] Wire thirteen-tool catalogue, enum/numeric validation, protocol prompts/resources
  and setup command, version1.2.0. Regenerate reviewed catalogue; update tests deriving
  surface from snapshot rather than another hand-maintained tool count.
- [ ] Connect notebook wire checks to existing acceptance, release inputs/docs to package.
- [ ] Run product fmt/tests/strict clippy/boundary, existing value/PR checks and new checks.

## Task4 — fresh review, host loop and integrate

- [ ] Fresh OpenCode REVIEW worker: read-only source review, own HANDOFF/receipt only.
  Check five review-focus classes and price/value claims. Parent verifies findings.
- [ ] Parent fixes confirmed issues with regression evidence; do not blindly follow review.
- [ ] Build sealed1.2.0 archive with operator public verifier; use existing internal
  evaluation signed file, never read/share issuer seed or treat it as paid fulfillment.
- [ ] Run exact archive acceptance + notebook wire + existing value/PR regressions.
- [ ] Actual OpenCode/free/max resumes synthetic notebook and finds correct source/caller;
  compare native task, preserve failed attempts and provider-reported counts.
- [ ] Integrate only approved paths after detecting concurrent owned-file changes.
  Run `cargo test`, `cargo clippy --all-targets`, observe/diff/invariant/remember.
- [ ] Commit scoped source/docs/receipts, preserve unrelated changes, no push.

## Completion contract

Ship implemented/proved scope, actual command results and limits. Agent success is
reported evidence; parent gates certify integration. Host/VSCode/payment unrun states
remain UNVERIFIABLE. No expanding into a custom editor, general memory platform or
subscription service without buyer evidence.
