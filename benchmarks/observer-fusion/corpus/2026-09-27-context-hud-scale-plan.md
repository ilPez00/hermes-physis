# Implementation plan — context compilation, HUD focus/lenses, scale experiment

Date: 2026-09-27. Basis: mission briefs (token-saving + HUD; fractal/scale
hypothesis). Engine landed at `9178698` (`physis-search/src/context.rs`).

## Ground truth (verified by inspection)

- `physis-search/context.rs`: `compile_context` — candidates with per-leg
  scores + reasons, RRF fusion, jaccard dedup, hard budget, `ContextStats`.
  16 tests green, bench table `#[ignore]`d. **No consumers yet.**
- `physis-runtime::Runtime` already depends on `physis-search`; has
  `search/bm25_search/context/answer/diff/architecture`.
- HUD `main.rs` (~860 lines): 15 tabs, integer tab index, `focus_entity` used
  only by Relations tab (tab 8). Search/Context tabs disconnected from focus.
- MCP: 41 tools via `crates/physis-mcp`; `physis.context` exists (old packet).
- Scale experiment: not started. physis-core has transform-analogy/fBm
  experiments but nothing on graph coarse-graining.

## Slices (each lands with tests, clippy -D warnings clean)

### S1 — Compiler reaches callers (runtime + CLI + MCP)
1. `Runtime::compile_context(query, budget, focus: Option<&str>) -> CompiledContext`
   delegating to `physis_search::compile_context` with a
   `Bm25Projection::build(&self.snap)` index + `QueryEngine`.
2. CLI: `physis compile <query> [--budget N] [--focus ID] [--path P] [--json]`
   printing the stats block + selected rows with reasons.
3. MCP: new tool `physis.context_stats` (args `{query, budget?, focus?}`)
   returning stats + selected list + reasons. Old `physis.context` untouched
   (non-destruction).
4. Tests: runtime integration test (fixture snapshot), MCP tool arm test.

### S2 — HUD focus state (one world, many lenses)
1. New field `focus: Option<physis_kernel::EntityId>` on `Hud` (replaces
   `focus_entity`'s role gradually — Relations tab keeps working; `focus`
   becomes the universal focus).
2. `l` key cycles lens: `graph → relations → deps → history → context` while
   `focus` is `Some`. Focus survives; only the render arm changes. Lens shown
   in a breadcrumb line: `focus: file:src/x.rs · lens: relations · [l] next`.
3. Setting focus: Enter on a search result focuses it (replaces current
   `search_results`-only behavior); Relations tab `f` still clears focus.
4. Test: focus survives lens switch (state-level unit test on a tiny Hud-like
   struct extracted to `apps/physis-hud/src/focus.rs`).

### S3 — Context/token-economy lens
1. When lens == `context` (or old tab 2 gains a stats block): call
   `hud.rt.compile_context(focus-or-query, hud.ctx_budget, focus)` ONCE per
   focus/query change, cache `CompiledContext` in Hud (never in draw loop).
2. Render: budget bar (`█████░░░ utilization`), candidate/selected/dropped
   tokens, reduction %, strategies, truncated flag, then per-candidate rows:
   `[kind] id — reason` (selected) / `  dropped — reason` (top 6).
3. Keys in this lens: `+`/`-` adjust budget (recompile on change), `r`
   recompile. Snapshot view `--view=context:<query>` for CI.
4. Tests: snapshot renders stats; budget change recompiles (unit test on cache
   logic); no per-frame recompile (assert cache hit count in test via small
   wrapper).

### S4 — Magic textbox (pure parser first)
1. New module `apps/physis-hud/src/textbox.rs`:
   ```rust
   pub enum Command {
       Focus(String),          // bare text: search + focus best hit
       Verb { verb: String, target: Option<String> },  // :verb [target]
       Agent { id: String, task: String },             // @id task
       Filter(String),         // /text (relations filter)
       Raw(String),            // fallback → existing search behavior
   }
   pub fn parse(input: &str) -> Command
   ```
   Deterministic, total (never panics), unit-tested (grammar table incl.
   edge cases: empty, whitespace, unknown verb, `@` without id).
2. Wire into HUD: Enter in the magic box dispatches Command; nav/read-only vs
   mutation distinction: `:write/:execute` require a confirmation line render
   before acting (v1: show "confirm with Enter again" state).
3. Palette: on empty input, show top-20 entity ids matching last focus kind +
   verb list. Suggestions from snapshot only — no model.

### S5 — Agent handoff preview
1. `@opencode <task>` (or CLI `physis delegate --preview`) compiles context
   for the task via `compile_context`, renders the exact handoff: task, agent
   id, selected objects + token count, budget, omitted count — and only on
   confirm calls `physis_agent::delegate`.
2. Test: preview-only path never calls delegate (unit test with a guard
   closure); preview text contains budget + selected count.

### S6 — Scale experiment (physis-projection, no new crate)
1. `crates/physis-projection/src/scale.rs`:
   - `Level = File | Crate | Layer` (from actual evidence: file ids +
     `package: X dep-> Y` cargo observations).
   - `lift(snap, level) -> LiftedGraph`: map each file→its crate via
     `package` membership; merge edges file-level DEPENDS_ON → crate-level;
     each lifted edge carries `support: usize` (number of lower-scale edges).
     Provenance: `Lifted` — never collapsed into Observed.
   - `stability(level_a, level_b) -> f32`: direction-consistency of the
     relation across levels (fraction of lifted edges preserving direction).
2. Negative controls (must score below real): shuffled containment
   assignment, randomized edge direction, degree-matched random graph.
3. Signature: `ScaleSignature { edge_counts: Vec<(level, usize, support)>,
   direction_stability: Vec<(level_pair, f32)> }`.
4. Tests: lift on fixture; control scores strictly below real hierarchy;
   determinism.

### S7 — Compression + reconstruction + verdict
1. `compress(graph, level) -> (primitive_repr, bindings, exceptions)` —
   group lifted edges by (predicate, direction) with instance bindings.
2. `reconstruct(primitive, bindings) -> edges`: precision/recall vs original.
3. Token comparison at matched budgets: raw edges vs primitive+bindings
   vs mixed (invariant + exceptions). Printed in an `#[ignore]`d bench test.
4. Verdict in docs: A/B/C/D per the mission's rubric, scoped claims only.

### S8 — Docs, receipts, ledger, gates
1. Migration receipt `migrations/receipts/` (or repo convention) for
   `context.rs` + `scale.rs` transplants (source repo/commit/what/why/tests).
2. `docs/FEATURE_LEDGER.md/json` entries.
3. `cargo fmt --check`, clippy `--workspace --all-targets -D warnings`,
   `cargo test --workspace` (known flake: `history_lens…`), HUD snapshot tests,
   bench tables. Dogfood: `physis compile "ContextPacket" --path .` +
   HUD snapshots on physis-next itself.

## Order
S1 → S2 → S3 → S4 → S5 (HUD/compiler spine), then S6 → S7 (scale), then S8.
Commit per slice.
