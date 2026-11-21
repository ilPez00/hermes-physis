# Plan — HUD → module-centric impossible-computer architecture

Date: 2026-09-28. Basis: mission brief (rebuild-the-computer), reality report
below (measured on this checkout), quarry plans `2026-09-19-impossible-computer.md`
and `PH-024-hud-one-surface.md`, context/HUD/scale plan `2026-09-27`.

## Prime directive

DO NOT DELETE FEATURES. DO NOT build parallel implementations of anything
Physis already has. One world graph + one universal focus + one TUI shell +
modular projections. The graph is the substrate from which applications are
projected. Zero capability loss (`LOST == 0` in `docs/FEATURE_LEDGER.json`).

## Ground truth (measured on this checkout, 2026-09-28)

| Fact | Where |
|---|---|
| HUD is one 1146-line `main.rs`, 15 integer tabs, render arms `match hud.tab` (lines 536–820), keys 841–1090 | `apps/physis-hud/src/main.rs` |
| Focus + trail + 5-lens cycle, tested; lens cycle still rewrites `hud.tab` (958–970) | `apps/physis-hud/src/focus.rs` |
| EntityLens projection, identity round-trip invariant | `apps/physis-hud/src/lens.rs` |
| Total textbox parser: Focus/Verb/Agent/Filter/Shell, mutation labels | `apps/physis-hud/src/textbox.rs` |
| Two-phase agent preview/launch, bounded 60 s, clipped | `main.rs:208–247`, `1005–1020` |
| Deterministic notebook ask (`pack_token_budgeted` + `ask_prompt`) | `physis-search/src/notebook.rs`, `main.rs:1028–1065` |
| Context compiler: candidates/reasons/RRF/dedup/hard budget | `physis-search/src/context.rs` |
| Embedder stack + honesty probes + disk memo cache | `physis-projection/src/embed.rs` |
| Settings live edit (e/s/r/d over `RuntimeConfig::save_dirty`) | `main.rs:789–915`, `1096–1107` |
| Static `ModuleRegistry` (descriptors + adapter availability); does NOT drive HUD routing | `physis-registry/src/lib.rs` |
| `Runtime` owns canonical mutation paths: `write_object` and shared `ingest_delegate_result`, alongside search/bm25/compile_context/context/answer/diff/architecture | `physis-runtime/src/lib.rs` |
| Ledger: 53 rows, 0 LOST | `docs/FEATURE_LEDGER.json` |
| Former `history_lens` flake fixed: invariant now chooses an observed `CHANGED_IN` source inside the bounded Git window | `tests/invariants/source_graph.rs` |

Decisions confirmed: session scope = Phases A–I; editor write via new
`Runtime::write_object` (fs write + re-observe + DiffReport + observation),
reused later by CLI/MCP — never a HUD-private store.

## Invariants (hold in every packet)

- **I1 Focus stability.** Switching modules must not change the focused
  object. `FocusState.current` is mutated only by explicit focus actions.
- **I2 No draw-loop work.** No `compile_context`, no fs scan, no model call
  inside a render arm. Expensive results are cached behind explicit keys.
- **I3 One canonical system.** No second compiler/search/registry/store/
  action system. Modules extend; they do not fork.
- **I4 Confirmation for mutation.** Every mutating verb uses the two-Enter
  pending pattern (same key twice) — the pattern already proven for
  `/agent` and settings staging.
- **I5 Offline-first.** Every remote-dependent surface prints
  `unavailable: <reason>`; everything else works with network off.
- **I6 Parity before removal.** An old route dies only after its replacement
  module's parity snapshot passes.
- **I7 Ledger.** No row reaches LOST. Ported rows flip only on green gate.

## Gate (every packet)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Plus per-packet snapshot/parity commands as listed. The former
`history_lens` state-dependent test is fixed; workspace failures are no
longer waived by that exception. Commit only on a clean tree.

## Snapshot goldens

Before Phase A, capture the golden set and check it in
(`apps/physis-hud/tests/goldens/`):

```bash
for v in overview modules settings graph entity:file:apps/physis-hud/src/main.rs \
         notebook:context compiler context:ContextPacket \
         agent:opencode improve the budget; do
  cargo run -p physis-hud -- --snapshot --view="$v" > "goldens/$(echo $v | tr ':/ ' '___').txt"
done
```

Golden-mismatch = regression unless the plan for that packet says the view
changed deliberately (then regenerate + record why in the packet receipt).

---

# Packets

Conventional commits, one packet ≈ one or two commits. Packet header format
follows the PH-024 convention (id, gate, depends).

---

## PIM-A — extract the HUD shell (no behavior change)

```
id: PIM-A
depends: []
gate: gate + golden parity diff + cargo test -p physis-hud
```

Move render bodies verbatim. No trait yet, no renames of user-visible keys.

Target file shape:

```text
apps/physis-hud/src/
    main.rs        // arg parsing, snapshot views, startup only (< 250 lines)
    app.rs         // Hud struct + load + run loop (moved from main.rs)
    event.rs       // key dispatch (moved verbatim from main.rs:841–1090)
    layout.rs      // frame Layout splits + footer
    focus.rs       // unchanged
    lens.rs        // unchanged
    textbox.rs     // unchanged
    views.rs       // the 15 render arms as free fns: fn view_search(hud,&mut Frame,Rect) etc.
```

Pseudocode of the extraction (mechanical, bodies unchanged):

```rust
// views.rs — one fn per arm, signature shared
pub fn render(tab: usize, hud: &mut Hud, area: Rect, f: &mut Frame) {
    match tab {
        0 => overview(hud, area, f),
        1 => search(hud, area, f),
        // ... 14 arms, bodies copied from main.rs:537–819
        _ => help_placeholder(hud, area, f),
    }
}
```

`stage_cfg_edit`, `agent_preview`, `agent_launch`, snapshot helpers move
into `app.rs` (or small private modules) with their tests.

Gates:
- `diff <(old snapshot out) <(new snapshot out)` empty for every golden view.
- New test: `mod view_parity { /* renders each view fn against a 100x30
  TestTerminal buffer and asserts non-empty + title present */ }`.

---

## PIM-B — module registry routing

```
id: PIM-B
depends: PIM-A
gate: gate + goldens + new registry tests
```

Introduce the module trait (smaller than the brief draft — state lives in
`Hud`, so modules are capabilities over shared state, matching the
extraction style):

```rust
// modules/mod.rs
pub trait HudModule {
    fn id(&self) -> &'static str;        // "search", "editor", ...
    fn title(&self) -> &'static str;
    /// Key ownership: return true if this key was consumed.
    fn handle(&mut self, hud: &mut Hud, key: KeyCode) -> bool { let _ = (hud, key); false }
    fn render(&mut self, hud: &mut Hud, area: Rect, f: &mut Frame);
    fn snapshot(&self, hud: &Hud) -> String;   // CI text frame
}
```

`views.rs` render fns become thin module impls wrapping the same bodies.
`modules/registry.rs`:

```rust
pub struct ModuleRegistry { modules: Vec<Box<dyn HudModule>> }
impl ModuleRegistry {
    pub fn new() -> Self { Self { modules: vec![
        Box::new(Overview), Box::new(Search), /* ... all 15, order = old TABS order */ ] } }
    pub fn len(&self) -> usize;
    pub fn by_index(&self, i: usize) -> Option<&mut Box<dyn HudModule>>;
    pub fn by_id(&self, id: &str) -> Option<&mut Box<dyn HudModule>>;
    pub fn ids(&self) -> Vec<&'static str>;
}
```

- `Hud.tab` stays a `usize` (position in registry) — numeric keys `1-9,0`
  keep their exact old meaning. `TABS` const derived from `registry.ids()`.
- Settings key ownership moves from the hardcoded `hud.tab == 14` block
  (main.rs:846–915) to `Settings::handle` — same behavior, routed by trait.
- HUD Modules view (old tab 13) renders the HUD registry merged with
  `physis_registry` availability (the `•/o` markers already there).

Tests:
- `old_tab_maps_to_module`: index i of `TABS` == `registry.by_index(i).id()`.
- `focus_survives_module_switch`: set focus, switch to every module, assert
  focus unchanged (mirrors focus.rs lens test at module granularity).
- `settings_keys_owned_by_settings_module`: `e`/`s`/`r`/`d` reach Settings
  only when it is active.

---

## PIM-C — graph-centered shell + universal focus

```
id: PIM-C
depends: PIM-B
gate: gate + updated goldens (recorded change) + narrow-degradation test
```

Graph becomes the default surface; the tab bar is replaced by a breadcrumb.

Layout (`layout.rs`):

```text
row 0   breadcrumb: ◈ physis · focus: file:src/foo.rs · lens: graph · trail 2 · tok 512/1600
row 1.. body:   wide terminal (≥100 cols): [graph | active module] split
                narrow (<100) or fullscreen-toggle: active module only
row -3  magic textbox (existing)
row -1  status/footer (existing key hints + token session total, Phase H)
```

- Body split: graph pane 40% / module pane 60% when both visible.
- `Tab`/`BackTab` cycle modules; `M` opens the module palette (j/k + Enter).
- Fullscreen toggle key `F` (uppercase; `f` stays clear-focus).
- Graph pane = EntityLens rows when focused (reuse lens.rs), raw edge table
  otherwise (current tab 8 body). Enter on a graph row focuses that row's
  target — rows already carry `kind:key` identity (lens.rs invariant test).
- New textbox verbs routed in `event.rs`:

```rust
"edit"     => open_editor(hud, focus_or_target)   // Phase D; until then "unavailable: editor module lands in PIM-D"
"notebook" => hud.switch_to("notebook")
"actions"  => hud.switch_to("actions")           // Phase G placeholder
```

- `/module <id>` = `Command::Filter` special-cased in `event.rs`? No —
  extend `textbox::Command` with `Module(String)`:

```rust
// textbox.rs
pub const MODULE_IDS: &[&str] = &["graph","search","context","editor","notebook","diff",
    "history","tasks","agents","architecture","invariants","relations","ontologies",
    "live","tools","modules","settings","help"];
"/module editor" => Command::Module("editor".into())
// suggestions("/mo") => ["/module editor", ...] prefix-filtered as today
```

Tests:
- narrow terminal renders single pane (buffer width 60).
- `Tab` cycles; focus unchanged through the whole cycle (I1).
- graph row Enter focuses the row's target identity.
- `suggestions("/mod")` deterministic + prefix-filtered.

---

## PIM-D — editor module + `Runtime::write_object`

```
id: PIM-D
depends: PIM-C
gate: gate + editor loop test + goldens
```

### D1 — canonical write API (`crates/physis-runtime/src/lib.rs`)

```rust
impl Runtime {
    /// Resolve a file entity id ("file:apps/x/src/main.rs") to an absolute
    /// path inside the workspace root. Refuses paths that escape root
    /// (symlink-resolved), refuses non-file kinds.
    pub fn resolve_file(&self, id: &EntityId) -> anyhow::Result<PathBuf>;

    /// Read the canonical content of a file object.
    pub fn read_object(&self, id: &EntityId) -> anyhow::Result<String>;

    /// Canonical write: validate → write file → re-observe the tree →
    /// append an observation (`source: "hud:editor.write"`) → return the
    /// DiffReport of before/after re-observation. The ONLY write path.
    pub fn write_object(&self, id: &EntityId, content: &str) -> anyhow::Result<WriteReceipt>;
}

pub struct WriteReceipt {
    pub id: EntityId,
    pub bytes_written: usize,
    pub delta: DiffReport,        // rt.diff("HEAD", "worktree")-shaped stat
    pub observations_added: usize,
}
```

Path-escape rule mirrors the store's data-dir resolution pattern; write is
atomic (tmp file + rename in the same dir).

### D2 — editor module (`modules/editor.rs`)

State (module-owned, per I3 canonical content stays on disk):

```rust
pub struct Editor {
    pub target: Option<EntityId>,
    pub buffer: String,
    pub dirty: bool,
    pub sel: usize,               // cursor line
    pub pending_write: bool,      // two-Enter confirm
    pub last_receipt: Option<WriteReceipt>,
    pub status: String,
}
```

Keys: `↑/↓` move line, `e` insert-mode (chars/Backspace/Enter/Esc like the
settings edit buffer), `d` diff preview (buffer vs disk, unified-diff lines
in the pane), `s` first Enter arms `pending_write` + shows receipt preview,
second Enter executes `rt.write_object`, then reloads the Runtime snapshot
(the exact reload path the Live tab already uses, main.rs:918–925) so the
graph updates from the observed delta; `u` reverts buffer to disk.

Dirty indicator in the module title: `editor · file:… *DIRTY*`.

The edit loop, end to end (this is the §20 success loop segment):

```text
select graph object → open editor (:edit) → modify buffer → d diff preview
→ s confirm (two-Enter) → rt.write_object → observation appended
→ snapshot reload → graph/relations show the delta → status shows receipt
```

Tests:
- `editor_loop_roundtrip` (temp workspace): write_object → observation
  count +1 → reload → entity/edge delta present → read_object returns
  written content.
- `write_refuses_path_escape`: `file:../outside.rs` and symlinked escape
  both error, nothing written.
- `write_requires_two_enters`: single `s` never mutates the file.
- `editor_compiles_affected_view_off_draw_loop`: affected-relations pane is
  compiled once per save (cache key = (id, revision)), not per frame (I2).

---

## PIM-E — notebook module

```
id: PIM-E
depends: PIM-C (D not required)
gate: gate + notebook reload test + budget test
```

Notebook = projection over references to world objects. Persistence: one
JSON file per notebook under `.physis-next/notebooks/<name>.json`. No
second DB; cells hold references and short notes, never content blobs.

```rust
// modules/notebook.rs
pub enum Cell {
    Ref    { id: EntityId, note: Option<String> },   // pinned world object
    Query  { text: String },                          // live search projection
    Note   { text: String },
}
pub struct Notebook { pub name: String, pub cells: Vec<Cell> }

impl Notebook {
    pub fn load(path: &Path) -> anyhow::Result<Self>;
    pub fn save(&self, path: &Path) -> anyhow::Result<()>;   // atomic write
}
```

HUD keys: `n` new cell of focused object (`Ref`), `j/k` select cell,
`Enter` focus the cell's object, `x` delete cell (two-Enter), `a` ask
(deterministic: collect cell objects → hits → `pack_token_budgeted` →
`ask_prompt`, the exact Q7 path), `c` send selection to the context
compiler (`compile_context` with the cell object as focus, budget from
`ctx_budget`).

Ask path never calls a model — it renders the citation prompt and labels
degradation exactly like the current search-tab ask (I5).

Tests:
- `notebook_survives_reload`: save → new Notebook::load → cells equal.
- `notebook_ask_bounded`: rendered ask tokens ≤ budget for any cell count.
- `ref_cells_address_canonical_objects`: every Ref id resolves in the
  snapshot or the cell renders `stale: <id>` (never silently dropped).

---

## PIM-F — remaining tabs → modules + route retirement

```
id: PIM-F
depends: B (mechanically), D/E for the two new ones
gate: gate + per-module parity snapshots
```

Every old arm becomes a module in `modules/`: `overview.rs search.rs
context.rs diff.rs tasks.rs history.rs architecture.rs invariants.rs
relations.rs ontologies.rs live.rs tools.rs help.rs settings.rs` (already
done for graph/editor/notebook). Parity = per-module `snapshot()` compared
against the golden captured for the old tab arm. Old integer-tab routing is
kept as the compatibility layer (it just indexes the registry); nothing is
deleted until all 18 module snapshots pass.

Deliverable: `docs/HUD_FEATURE_COVERAGE.md` regenerated from the registry:
every old tab maps to a module id, every snapshot view name still resolves.

---

## PIM-G — agents/tasks/actions as graph objects

```
id: PIM-G
depends: F
gate: gate + graph-object test + offline degradation test
```

G1 — discovery/runtime emits graph objects (extend `physis-runtime`
observe post-pass, no new crate):

```rust
// after observing: for each registered adapter (physis_registry::available_adapters)
//   entity "agent:<id>"  label = adapter id
//   edge agent:<id> --OBSERVES--> nothing yet; edges wired to tasks below
// for each dir under worktrees/:
//   entity "task:<name>", edge task:<name> --DerivedFrom--> workspace root
//   edge agent:<available-adapter> --Produces--> task:<name> only when an
//   observation with source "physis:delegate" names both (provenance, not guesswork)
```

G2 — actions module (`modules/actions.rs`): "what can I do to this
object?" — a static, kind-keyed table rendered for the focus. Read-only
this packet; verbs route to existing machinery or print
`unavailable: <reason>`.

```rust
fn actions_for(kind: &str, focus: &EntityId) -> Vec<Row> {
    match kind {
        "file"   => rows!["read","edit (:edit)","diff (:diff)","history (:history)",
                          "tests (:tests)","deps (:deps)"],
        "agent"  => rows!["inspect","call (/agent <id> <task>)","handoff (preview)"],
        "task"   => rows!["inspect","open worktree path"],
        _        => rows!["inspect","relations","history"],
    }
}
```

Mutating verbs keep the two-Enter confirm (I4). Shell stays disabled by
policy (textbox already labels `$` Shell; HUD answers "shell disabled by
policy").

Tests:
- `agent_task_entities_present`: observe a fixture with worktrees/ →
  agent:* + task:* entities and edges exist in the snapshot.
- `actions_table_covers_focus_kinds`: for every kind present in the
  snapshot, `actions_for` returns ≥1 row (no silent empty).
- `offline_degradation_labels`: with no adapters on PATH, agent surfaces
  print `unavailable: no adapter binary on PATH …` (message already exists
  in `agent_launch`; assert it, and add the same label style to embedder /
  semantic surfaces).

---

## PIM-H — token economy instrumentation

```
id: PIM-H
depends: F (independent of G)
gate: gate + measurement doc
```

- `physis-search::context` gains a tiny callback-free side channel:
  `CompiledContext.stats` already carries everything; instrumentation =
  call sites append one observation per compile:

```rust
// runtime, inside compile_context (single choke point — no per-module copies)
self.record_observation("physis:compile", &format!(
    "q={:?} focus={:?} budget={} selected={}/{} tok strategies={} embedder={} semantic={} took_ms={}",
    query, focus, budget, st.selected_tokens, st.budget,
    st.strategies.join("+"), label, semantic, ms));
```

- HUD status bar shows session totals (selected tokens, reduction %, compiles).
- Measurement pass: run the §20 loop on this repo (dogfood), then
  `docs/recon/2026-09-28-module-token-accounting.md` with the numbers:
  compiles, tokens selected vs dropped, reduction by module, elapsed.
- Fractal compression receipts (§11): expose the scale experiment's
  primitive+bindings+exceptions rows in the context lens **only** when
  reconstruction gates pass — behind a flag, raw evidence stays addressable.
  No new compression this packet; wiring only.

---

## PIM-I — docs + ledger + receipts

```
id: PIM-I
depends: A–H
gate: gate + ledger check test
```

- Flip ledger rows: notebook cells, editor/write path, actions, graph
  objects → PORTED/ADAPTED with receipts under `migrations/receipts/`
  (source = quarry docs, destination = module paths).
- New test `tests/invariants/hud_coverage.rs`:

```rust
#[test]
fn ledger_has_no_lost_rows() {
    let txt = std::fs::read_to_string("docs/FEATURE_LEDGER.json").unwrap();
    assert!(!txt.contains("\"LOST\""), "a capability was lost");
}
#[test]
fn every_hud_tab_has_a_module_and_snapshot() { /* registry ids vs coverage doc */ }
```

- Update `ARCHITECTURE.md` (module diagram), `PLAN.md` (packet table),
  HUD help text, README quick start (`physis-hud` keys).

---

# Test matrix (final state)

| Area | Test | Packet |
|---|---|---|
| Shell | golden snapshot parity (all views) | A,F |
| Registry | old-tab↔module mapping, ids total | B |
| Focus | survives module switch (I1) | B,C |
| Textbox | `Module` grammar, suggestions | C |
| Layout | narrow degradation, fullscreen toggle | C |
| Editor | loop roundtrip, path escape refused, two-Enter write, no draw-loop compile | D |
| Notebook | reload survival, ask bounded, stale refs labelled | E |
| Graph objects | agent/task entities+edges, actions coverage | G |
| Offline | degradation labels on agent/embedder surfaces | G |
| Budget | context ≤ budget (existing), notebook ask ≤ budget | E,H |
| Ledger | LOST == 0, coverage doc complete | I |
| Regression | all current workspace tests stay green, including history-lens traversal | all |

# Success criterion (acceptance, not aesthetics)

```text
launch physis-hud → world graph default
→ navigate project → crate → file → symbol (Enter on rows)
→ focus one object
→ :edit → modify → diff preview → confirm → save
→ observation + delta visible, graph refreshed
→ Tab to History / Notebook / Context — focus unchanged each time
→ notebook pins the object; ask bounded; compile shows the exact packet
→ /agent opencode … previews the bounded packet before launch
→ kill network: everything above except the agent launch still works,
   unavailable surfaces print their reason
```

Highest-leverage next step after I: crystallization (§12) — agent outputs
becoming relations/claims, and the L0–L6 routing measurement, both of which
consume the instrumentation PIM-H leaves behind.
