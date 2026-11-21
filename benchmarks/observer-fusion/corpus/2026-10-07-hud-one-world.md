# HUD as viewport into one world — plan, findings, and resume prompts

Mission: turn `physis-hud` into the primary human interface to Physis — one
focus, one trail, LEFT → CENTER → RIGHT, lenses over a stable object, and no
separate editor/graph/notebook/terminal applications.

The brief this implements is kept verbatim in
**`2026-10-07-hud-one-world-mission.md`**. Read that for *what* was asked;
read this for *what the code actually looks like*.

Status: **P1, P0 and P2 landed**. P3–P12 open. Six commits on
`feat/hud-spatial-shell` in the worktree `/home/gio/pw-hud-shell`, base
`97f0763`. A short status card lives in `docs/HUD-STATUS.md`.

---

## Part 0 — Start here

**Read in this order, and stop when you have what you need:**

1. This file, Part 1 (findings) and Part 6 (resume prompts). ~20 minutes.
2. The mission only if you need the *why*:
   `2026-10-07-hud-one-world-mission.md` (verbatim, 1715 lines — do not read
   it end to end; it is the reference, not the brief).
3. The code, in this order — the map is in §1.14:
   `focus.rs` (220 lines) → `object.rs` (the new one) → `layout.rs` (165) →
   `event.rs` (only `dispatch_command` and the final `match` in
   `handle_key`) → `views.rs` render arms → `app.rs` render loop.

**Environment setup for this worktree** — each of these is needed and none is
obvious:

```bash
cd /home/gio/pw-hud-shell
cp -a /home/gio/dev/physis-next/.physis-next/. .physis-next/   # the store
cargo build -p physis-cli      # else 2 research-loop cards are UNVERIFIABLE
```

The `.physis-next/` copy is the observation store. **It is gitignored, so a
fresh worktree has none** and every store-backed test (graph, scale, render)
fails with errors like `a search crate file exists` — environmental, not real.
Re-copy it after any rebase.

**Run this before you change anything:**

```bash
cargo fmt --all --check
cargo clippy -p physis-hud --all-targets -- -D warnings
cargo test -p physis-hud
bash scripts/research-loop.sh --gate      # currently INCOMPLETE — see Part 3
```

**Then rebase onto `main`** (5+ commits ahead, carrying World 2). Resume A
does exactly this and nothing else. Do it before a new phase, not after.

**House rules for this repo, which are not optional:**

- **Never `git add -A`.** `research/loops/results/*.jsonl` accumulates from
  other sessions' loop runs and is not yours to commit. Stage explicit paths.
- **Run gates against the committed state**, not the working tree. See §3.2 —
  I broke this once.
- **UNVERIFIABLE is not a pass.** The research loop enforces this on itself:
  a card that did not check stays in the denominator. "Skipped" and "tied" are
  different states (`docs/IMPOSSIBLE_COMPUTER.md` §2).
- **Another session may be editing the same clone.** `git status` immediately
  before any destructive step, not earlier in the turn.
- `main` carries ~50 unpushed commits by operator decision. **Do not push.**

---

## Part 1 — Findings

Everything here was measured in this tree, not assumed.

### 1.1 Repository state at baseline

- `main` at `97f0763`; the loop receipt for that tree reads
  `50 checked: 50 pass, 0 FAIL, 0 unverifiable`.
- Two other worktrees exist and both are live:
  `/home/gio/pw-docs` (`feat/doc-ranking`, clean, no upstream) and
  `/home/gio/pw-lang` (`feat/lang-coverage`, clean, tracks origin, merged).
- `main` carries a large unpushed history — 50 commits ahead of
  `origin/main` at the time of survey, 0 behind. `git merge-base
  --is-ancestor main origin/main` exits 0, so published ancestry is intact;
  the unpushed stack is an operator decision, not an accident
  (`d64d9a0` "push pending operator OK").
- Branches: `feat/doc-ranking`, `feat/lang-coverage` (merged),
  `mc05`, `mcp-receipt`, `native-loop-parity`, `task/solve-v01-demo`,
  `task/solve-v04`.
- A **second session was editing the main checkout during the survey** —
  `docs/STATUS.md`, `tasks/todo.md` and `tests/invariants/docs_freshness.rs`
  appeared dirty, then landed as `d3d76c4` mid-survey. Hence the worktree.

### 1.2 The root defect: two navigation axes

The mission's core law is *there is one focus, one trail, one navigation
model*. The HUD had two axes:

| Axis | Where | Shape |
|---|---|---|
| `Hud.tab: usize` | `app.rs:88` | index into 20 render arms, switched by `1-9`,`0`,`m` |
| `Hud.focus: FocusState` | `app.rs:132` | one `EntityId`, a `Lens`, a trail |

`TABS` (`app.rs:47-68`) and `TAB_TO_MODULE` (`modules/mod.rs:11-32`) both
enumerate the same 20 slots. The footer advertises `1-9,0 tabs`
(`layout.rs:54`). `views::render` (`views.rs:76-101`) dispatches on the
integer.

So the HUD was **a tabbed application with a focus system bolted on**. The 20
arms are the mission's "dashboard with twenty panes" and its
"do not build an editor app / notebook app / graph app". P12 retires them;
doing that before the replacement is proven would delete working functionality,
so P0 was made additive.

The second axis is load-bearing in ways that must be undone carefully:

- `event.rs` writes `hud.tab` from six places (`:edit` → 17, `:module` → N,
  `deps`/`context` → 2, `history` → 5, `:cog` → 19, `:nb` → 15).
- `l` cycles the lens and then **maps the lens onto a tab**
  (`event.rs:967-981`) — a six-arm match from `Lens` to tab index. This is
  the lens/tab coupling in its purest form, and it is why "lens switching must
  not change focus" was only half true: focus survives, but the *view* is
  decided by a hardcoded match on a 6-variant enum.
- `post-command tab checks` (`event.rs:501-526`) recompile context on
  `tab == 2` and recompute diff on `tab == 3`.

### 1.3 What the focus already got right

More than expected, and worth not rebuilding:

- `FocusState` (`focus.rs:107-148`) already had `set` / `clear` / `back` /
  `cycle_lens`, with the invariant *"lens cycle must not move focus"* already
  asserted at `focus.rs:267`.
- The graph pane already had a working **navigable** cursor: `j`/`k` move
  `graph_sel`, `Enter` focuses the selected neighbour **through the one focus
  system** (`event.rs:1025-1040`). Rows come from a single derivation
  (`app.rs:1098 graph_rows`) shared by the pane and the cursor, with a comment
  recording that a filtered row and a selected row must not be able to
  disagree.
- The scale lens already had a `scale_zoom` **projection id kept deliberately
  separate from `focus.current`** (`app.rs:167-171`) — the mission's "scale
  changes are projections, do not create fake entities" was already decided.

So the architecture was not absent. It was **incomplete in one direction** —
navigation had no forward, no parent, no breadcrumbs — and **wrong in one
other**: `tab` was a second focus model wearing a projection's clothes.

### 1.4 What the focus got wrong

- **The trail was backward-only.** `back()` popped `trail` and overwrote
  `current` with no record of where it had been. `b` was a retreat with no
  return.
- **No containment navigation.** Nothing could walk from a symbol to its file
  to its crate. The mission's LEFT region ("parents, containers, callers,
  previous revisions, previous focus, history") had no source.
- **No breadcrumbs.** Spatial position was in the *header*
  (`app.rs:389 header()` prints focus id, kind, state, revision) but it is the
  focused object only, with no containment chain — so "where am I" had no
  actionable answer.

### 1.5 A pre-existing name collision

`focus::Lens` — the six-variant enum at `focus.rs:12-39`.
`lens::Lens` — a projection *trait* at `lens.rs:51`.

Different concepts, same name, same crate. P4 touches both, so the rename has
to happen there and it is the first task in that phase.

### 1.6 The lens set is half the mission's list

`Lens::ALL` (`focus.rs:31-38`) has six: **graph, relations, deps, history,
context, scale**.

Present: graph ✓, relations ✓, history ✓, context ✓.
Present in a different shape: dependencies (`deps`).
**Absent: source, structure, callers, tests, diff, notebook, edit, run,
evidence, cognition, raw.**

`deps`/`context` both route to tab 2, and `graph`/`relations` both route to
tab 8 (`event.rs:971-977`) — so the six lenses already collapse onto four
distinct views. The enum is already closer to a view selector than a
projection model.

### 1.7 There is no harness in the HUD

This is the largest single gap, and the substrate already exists unused.

`event.rs:233` matches:

```rust
"run" | "write" | "delete" | "execute" => format!(...)
```

…into the `UNSUPPORTED_VERBS` refusal (recorded in `tasks/todo.md` as card
**L07**, "Stop unreachable textbox promises"). So `:run` is a truthful refusal
today. The mission's steps 19–25 of the dogfood test — run a test, inspect
stdout/stderr/exit/duration, follow evidence, invoke an agent — are
**entirely unavailable**.

The substrate, verified present and CLI-only:

| Thing | Where |
|---|---|
| `ExecRequest` (intent, caps, target, argv, incoming mode, executors) | `crates/physis-exec/src/lib.rs:40` |
| `plan()` — deterministic routing, spawns nothing, fails closed | `lib.rs:127` |
| `run_plan` / `run_plan_with` (injected env, so tests never mutate globals) | `lib.rs:235`, `lib.rs:244` |
| `RunReport` | `lib.rs:218` |
| `ExecOutcome { argv, exit_code, stdout, stderr, timed_out }` | `mirrord.rs:292` |
| `ExecOutcome::verdict()` → `pass`/`fail` from exit status and timeout only | `mirrord.rs:307` |

`grep -rn physis_exec apps/physis-hud/src` → **no hits**. Only
`apps/physis-cli` uses it (`main.rs:1623-1673`, `render.rs:234-452`).

So P7 is wiring plus a `Run` object model, not new execution machinery. The
one thing that must be built from nothing: **background execution**, because
`tests/frame_budget.rs` fails at 16 ms if a pane blocks.

### 1.8 The editor is real but short

`crates/physis-editor/src/lib.rs` — 523 lines, `Editor` at :41.

Present: `open` (:77), `undo` (:112), `insert_char` (:128), `backspace`
(:149), `move_cursor_char` (:170), `move_line` (:195), `diff` (:252),
`discard` (:276), `reload` (:288), `stage_or_save` (:314), `report` (:382).

**Absent: redo, selection, search, replace, go-to-line, new file, save-as,
syntax highlighting.** `Hud.editor.status`, `buffer` and `insert_mode` are
view state; canonical content stays on disk and every write goes through
`Runtime::write_object` (`app.rs:182-185`) — the "editor is never a second
source of truth" rule is already honoured and must stay.

`handle_key` (`lib.rs:470-519`) consumes: `Esc`, `Enter`, `i`, `Backspace`
(insert), any char (insert), arrows, `d` (discard), `u` (undo). **Everything
else falls through to the global handler**, which is what made `Ctrl-f` and
`p` safe to bind globally — worth re-checking when adding keys.

### 1.9 The notebook is a pin list, not a thinking environment

`crates/physis-notebook/src/lib.rs` — 485 lines, `Notebook` at :95.
`MAX_CELLS = 64` (:33), `MAX_NOTE_CHARS = 2000` (:38).

Present: `load`/`save`/`save_with` (:202-241), `pin` / `pin_with_baseline`
(:255-259) with observed-fingerprint baselines for staleness (:278),
`matching_cells` (:297), `add_note` (:312), `goal` / `set_goal` / `point_goal`
/ `scope_goal` (:327-394), `pin_note` (:420), `remove` (:459), and
`validate` (:123). Persists under `.physis-next/notebooks/`.

**Absent: multiple notebooks, cell edit, cell reorder, typed `[[kind:id]]`
references, cell execution, stale-reference *indication* in the surface.**
Staleness is *tracked* (baselines) and the HUD even reports it on follow
(`event.rs:709-715`, "stale reference: {id}"), which is the right instinct on
a smaller surface than the mission wants.

### 1.10 There is no session state file

`.physis-next/` contains exactly: `constraints.jsonl`, `embed-cache`,
`interview-cache`, `notebooks/`, `observations.jsonl`.

No focus, no trail, no lens, no pins, no layout, no recent runs. P10 is
greenfield, and the mission's constraint is explicit: **one inspectable file,
no opaque project database, never auto-resume a destructive or running
action.**

### 1.11 The world will get richer underneath us

`docs/plans/2026-10-07-world-model-computer.md` (on `main` via `913b2a1`) is
the concurrent workstream: 8 slices — frozen benchmark, two-stage Rust call
representation (`MAY_CALL`), structured `Counterexample`, proof-carrying
`ChangeReceipt`, multi-candidate repair, static+dynamic evidence fusion
(`CALLED`/`COVERED_BY`/`FAILED_IN`), **world schemas as data** (EntityType /
RelationType / Lens / Action / Invariant), model amplification bench.

Verified **not present** at this branch's base:

- no `MAY_CALL` anywhere in `physis-kernel` or `physis-query`
  (`grep -rn 'MayCall\|MAY_CALL'` → no hits)
- no `Counterexample` struct in `physis-task/src/solve.rs`
- no `ChangeReceipt` struct anywhere
- `research/benchmark/` contains only `corpus/` — no `oracle.json`, no
  `manifest.json`, no `bench-world-fidelity.py`

`RelationType` (`crates/physis-kernel/src/relation.rs:16-46`) has 18
variants: DependsOn, Produces, DerivedFrom, Observes, Supports, Contradicts,
Contains, Defines, Calls, Implements, Imports, ChangedIn, Runs, Owns,
Executes, WorksIn, Listens. `SolveReport` with `delta_score` does exist
(`solve.rs:324-337`).

Two consequences:

1. **Slice 7 overlaps the mission's generic-renderer requirement.** That is
   the open decision in Part 4.
2. `main` was mid-slice-2 during this work — uncommitted edits to
   `kernel/relation.rs`, `query/engine.rs`, `discover/scan.rs`,
   `discover/Cargo.toml`, `Cargo.lock`. Those files are exactly the
   `MAY_CALL` / call-resolution work.

### 1.12 Test and gate surface

Existing HUD tests, all in-crate and fast:

| File | Lines | Covers |
|---|---|---|
| `tests/shell.rs` | 419 | autorefresh, terminal guard, lens keys, editor save receipt |
| `tests/notebook.rs` | 801 | pins, goals, staleness |
| `tests/graph.rs` | 575 | graph rows, Enter-follows, edge filter, direction |
| `tests/render.rs` | 409 | header, filtered counts, whole-frame no-panic |
| `tests/textbox.rs` | 351 | verb grammar, agent preview |
| `tests/editor.rs` | 289 | buffer ops |
| `tests/scale.rs` | 266 | zoom, projections |
| `tests/document_work.rs` | 256 | document-level work |
| `tests/architecture.rs` | 43 | the architecture memo follows the world |
| `tests/frame_budget.rs` | 98 | **separate binary on purpose** |

`frame_budget.rs` is worth reading before adding panes. Its header explains
why it is isolated: `cargo test` shares a process across four rendering tests,
so timing measured alongside them reads contention as much as pane code — the
same pane measured 1.5 ms alone and 29.5 ms with siblings. Budget 16 ms,
tabs `0..18`, two frame sizes, median of 20, and a failure prints its ambient
inputs (profile, cpus, samples, empty-frame time) because "a time-based gate
reads ambient state".

Adding LEFT/RIGHT widens that loop. **Prefer deterministic cache assertions
over new timing assertions** — a new wall-clock gate on this machine is a
flake generator.

Repo gates: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings` (license crate excluded in the repo's own
gate), `cargo test --workspace --all-targets`, and
`scripts/research-loop.sh [--gate] [--repair] [--learn]`.

The research loop derives its cards from the justfile, the binary's `--help`,
the MCP `def()` catalogue and the demo filenames, so a new verb becomes a card
with no list to update. Its own rule, enforced on itself: **UNVERIFIABLE is
not a pass** — it is counted in the denominator and never folded into the pass
column. That rule is why a fresh worktree reports 2 unverifiable rather than
12 pass.

### 1.13 The textbox already parses a shell escape it refuses to run

`crates/physis-hud/src/textbox.rs` is the mission's "universal interaction
entrypoint", and it is further along than expected.

`Command` (`textbox.rs:22-39`) has six variants: `Empty`, `Focus(String)`,
`Verb { verb, target }`, `Agent { id, task }`, `Filter(String)`, and
**`Shell(String)`** — `$cmd`, parsed and labelled, with execution policy
left to the call site.

What the call site does with it (`event.rs`):

```rust
textbox::Command::Shell(c) => {
    hud.magic_status = Some(format!("shell disabled by policy (`{c}` not run)"));
}
```

So the parse hook for the harness exists and is **refused by policy**. P7 does
not need to invent a shell surface — it needs to decide the policy, and
`$cmd` is already the grammar.

Three verb tables (`textbox.rs:48-82`), and this discipline is worth keeping:

- `READ_VERBS` — 7: `deps`, `history`, `context`, `tests`, `zoom`, `module`,
  `cog`
- `MUTATING_VERBS` — 1: `edit`
- `UNSUPPORTED_VERBS` — 4: `run`, `write`, `delete`, `execute`
- `INTERVIEW_VERBS` — 8, `PALETTE` — 7 entries, all interview

The comment on `open`/`show` is the design rule in miniature: *"the dispatcher
never handled them, so suggesting them was a lie"*. Those verbs were
**deleted from the grammar** rather than left as suggestions. Any verb P3/P7
adds must have a live dispatch arm in the same commit, or it does not go in.

#### Mission verbs vs what actually exists

| Mission asks for | Actual |
|---|---|
| `:open src/client.rs` | **absent.** `open`/`show` were deliberately removed as lies |
| `:back` `:forward` `:parent` | **absent as verbs.** Keys `b` / Ctrl-f / `p` exist since `e989fc9` |
| `:edit` | ✅ `event.rs:120` — but refuses non-`file:` ids ("editor v1 edits files") |
| `:graph callers` | **absent.** `:deps` and `:context` exist and both route to tab 2 |
| `:context` | ✅ |
| `:run cargo test` | **refused** — `UNSUPPORTED_VERBS`, `event.rs:233` |
| `:solve fix timeout handling` | **absent from the textbox.** solve is CLI-only; `:cog` is a different thing |
| `:agent opencode improve this` | `/agent <id> <task>` — different signature (`event.rs`) |
| `:note` | `:nb add <text>` — different name |
| autocomplete for commands/paths/symbols/ids | **absent** |

An unknown verb answers with the full live verb list (`event.rs`), so the
grammar is discoverable — that is a good pattern to extend rather than
replace.

#### One more dead end worth knowing about

```rust
v if textbox::is_mutating(v) => format!("mutation `{v}` requires confirm (type Enter again to run)")
```

A **confirm gate for mutating verbs exists, and no mutation follows it** —
a second Enter changes nothing. That is the shape P9 has to complete: the
confirmation policy is already the right shape, the executor is missing.

### 1.14 The MCP surface is far ahead of the HUD — this changes P7

`crates/physis-mcp` defines **59 named `physis.*` tools** (62 defs including
three unnamed; the research loop reads the catalogue and reports "62 defs, 62
dispatched, 0 unroutable").

The relevant discovery: **the operations area already has a harness-shaped
surface the HUD never touches.** `crates/physis-mcp/src/tools/areas/ops.rs`
owns, via a `handles()` predicate derived from its own match arms so it
cannot drift:

```
physis.action.execute | physis.action.run | physis.aion.list-nodes
| physis.aion.node-run | physis.aion.node-status | physis.delegate
| physis.exec | physis.test.run
```

And the read side already covers the mission's generic-object requirement:
`physis.object.get`, `physis.object.relations`, `physis.object.list`,
`physis.object.patch`, `physics.nav.query`, `physics.navigate`,
`physics.dependencies`, `physics.tests_for`, `physics.diff`,
`physics.history`, `physics.changes`, plus the whole
`physis.hypothesis.*` family.

**So the capabilities behind P7 and P8 exist at the MCP boundary. The HUD is
behind its own MCP.** That reframes P7 from "build a harness" to "adopt the
existing action surface in the HUD's one-focus model" — and it means the HUD
does not need to be the first caller, so the semantics are already exercised.

This is also the mission's own instruction: *"Use runtime facade
capabilities. If a view function begins implementing business logic, stop and
extract it."* The facade is already richer than the view.

### 1.15 File map — so you need not read 5,716 lines

`apps/physis-hud/src/`, all in-crate tests inline:

| File | Lines | Read it when |
|---|---|---|
| `app.rs` | 1758 | Anything about `Hud` state or the render loop. `TABS` :47, `struct Hud` :84, `graph_rows` :1098, `Hud::render` :467 |
| `views.rs` | 934 | Render arms. `render_left` (new) :~14, `render` dispatch :76 |
| `event.rs` | ~1180 | `dispatch_command` :108, key `match` :~850 |
| `textbox.rs` | 371 | Verb grammar. `Command` :22, verb tables :48-82 |
| `focus.rs` | 220 | **Start here.** `Lens` :12, `ALL` :32, `FocusState` :107, containment fns :~150 |
| `lens.rs` | 247 | The projection *trait* `Lens` :51 — collides by name with `focus::Lens` |
| `layout.rs` | 165 | `spatial_split`, `body_split`, footer |
| `interview.rs`, `modules/cognition.rs` | — | Feature-specific, unrelated to the mission |

Test files: `tests/shell.rs` 419, `tests/notebook.rs` 801, `tests/graph.rs`
575, `tests/render.rs` 409, `tests/textbox.rs` 351, `tests/editor.rs` 289,
`tests/scale.rs` 266, `tests/document_work.rs` 256, `tests/architecture.rs`
43, and `tests/frame_budget.rs` 98 **in its own binary on purpose** — read its
header before adding panes.

### 1.16 Findings that changed the plan

Five things I would have got wrong by planning from the mission text alone:

1. **The `l` → tab match at `event.rs:967-981` is the real P4 problem, not the
   lens enum.** Adding lenses without addressing the match just grows a
   hardcoded coupling. And thirteen lenses on one cycling key is a worse
   control than the six it replaces.
2. **The harness is wiring, not construction — and the wiring already exists
   twice.** `physis_exec` is unused by the HUD (§1.7), *and* the MCP already
   exposes `physis.action.run`, `physis.action.execute`, `physis.test.run`
   and `physis.exec` (§1.14). P7 is a `Run` type in the HUD's focus model over
   an existing, already-exercised surface — plus background execution, which
   is the only piece genuinely built from nothing.
3. **The textbox is further along than the mission implies, and stricter than
   it needs to be** (§1.13). `$cmd` is parsed and refused; `open`/`show` were
   deleted rather than left as lying suggestions; and a confirm gate for
   mutations already exists with no mutation behind it. P3/P7/P9 should extend
   these tables, not invent a parallel grammar.
4. **The architecture was already mostly right.** A navigable graph cursor, a
   focus invariant with a test, projection ids kept out of canonical state,
   the editor writing through the runtime — all already there. The programme
   is closer to *completing and ordering* the design than inventing it. The
   risk is therefore not "too little exists" but "the tab axis gets deleted
   before its replacement carries the load".
5. **Search already focuses.** `Command::Focus` calls `hud.focus.set` on the
   top hit, so dogfood steps 2-3 work today — worth knowing before assuming the
   whole acceptance test is greenfield.

### 1.17 Known unknowns — never surveyed

Stated so nobody re-derives them by accident, and so nobody assumes they were
checked:

- **`views.rs` render arms** for history, diff, tasks, settings, cognition were
  not read. Only `render_left` and the `render` dispatch. If a phase needs to
  know what a pane shows, read the arm then — do not guess from the tab label.
- **MCP ↔ HUD parity was counted, not compared.** §1.14 lists the tool names;
  nobody has checked whether `physis.object.get` and `object.rs` render the
  same thing, which is a real duplication risk worth resolving in P2's
  follow-through.
- **`views.rs` `render_left` line numbers** drift whenever anything is inserted
  above them; §1.15 says "~14" for that reason.
- **The 30-step dogfood test was never executed.** The mission requires it and
  Resume F is the first attempt. Until then no claim about it is evidence.
- **`physis-workspace.toml`** lists seven federated "quarries" (physis-core,
  physis-pro, physis-global, physis-impossible, physis_wow, physis-skill) with
  `owns` organ lists. `AGENTS.md` says **physis-pro is retired** and this tree
  is the only Physis, so the ledger's organ ownership is historical. Do not
  treat it as current architecture.
- **`.physis-capabilities`** is a `capability | write path | read path` ledger
  checked by `physis-check capabilities`, where a capability counts only if
  both halves are live. If P7/P9 adds a capability, it belongs in that file.
- **`justfile` targets** were listed (11) but not read: `ci`,
  `commercial-release`, `quarry-check`, `snapshot-smoke`, `cheat-sheet`,
  `experiment-mirrord`, `versions`, `stage-computer`, `unified-release`.

---

## Part 2 — What landed

### P1 — one focus, one trail (`e989fc9`)

`FocusState` gained a **forward stack**. `back` pushes the focus it left,
`forward` pops it, and following a new object clears it — the future changed
with the present.

- Bound to **Ctrl-f**. This needed a modifier guard on the bare-`f`
  clear-focus arm: both match `Char('f')`, the clear arm comes first, so
  without the guard Ctrl-f cleared focus. Found by a test, not by reading.
- `parent_of` / `breadcrumb_trail` / `focus_parent` resolve containment from
  the world. Containers are matched by predicate **name** through
  `["DEFINES", "CONTAINS", "WORKS_IN", "EXECUTES"]` rather than an exhaustive
  enum match, so a relation a future world adds reaches breadcrumbs with no
  code change here, and an unknown predicate is honestly "not a container"
  rather than a guess.
- `DEFINES` outranks `CONTAINS`, so a symbol's *file* beats the crate that also
  contains it. Ties resolve by id — a breadcrumb that flaps between renders is
  worse than none.
- A containment cycle ends on the repeat; the hop bound is a backstop, not the
  loop guard.
- An object with no container **stays focused and says so**. Navigation that
  found nowhere to go never clears focus.
- Keys that do nothing now report it: `b` at the start of the trail, Ctrl-f
  with nothing ahead, `p` on an orphan.

`p` goes through `focus.set`, so the trail keeps the child and `b` returns.

### P0 — LEFT · CENTER · RIGHT (`56eedb9`)

The shell drew `[graph | module]`, which reads as two applications side by
side. The same two render arms are now placed in the mission's order:

- **LEFT** — new pane `views::render_left`. Containment breadcrumb root-first,
  then the trail most-recent-first with the forward stack after it. Derived
  from the focus alone, so it needed no new state. The focused node is marked
  `●` and coloured; ancestors are dimmed.
- **CENTER** — the module pane: the focused object under its active lens. Every
  existing tab arm, unchanged.
- **RIGHT** — the graph pane: the object's neighbourhood with the working
  `j`/`k`/Enter cursor. Only its `Rect` moved.

Widths are not equal thirds: **CENTER 45%, LEFT 22%, RIGHT 33%**.
`spatial_split` tiles the body with no gap and no overlap, asserted — a layout
that silently dropped a column would *hide the focused object* rather than
fail. `body_split` keeps its narrow and fullscreen collapse, so editor and
notebook still take the whole body on a small terminal.

Nothing deleted. The tab arms are untouched behind it, deliberately.

### P2 — the generic object renderer (`97c8ecd`, `62ca678`)

`apps/physis-hud/src/object.rs`, plus `--view=object:<kind>:<key>`.

`match kind { "file" => … }` would be the natural way to write this and it
would be the bug: the first new kind would be unopenable until somebody wrote
an arm. So the renderer is driven only by what the snapshot carries:

- Every predicate goes through `as_str`. The kind is printed, never switched
  on.
- Tests pin the claim on a **`counterexample`** entity and a **`CONTRADICTS`**
  edge — both deliberately foreign to the HUD.
- Uncertain edges say so on the row (`confidence 0.40`). The mission forbids
  assuming every relation is settled.
- Provenance is part of the object, not debug output: edge observation numbers,
  entity evidence, world revision. The revision renders even with no evidence,
  since an object still answers "which world is this from".
- Flat text reads in the spatial order: identity → what points at it → what it
  points at.
- A focus pointing at something the world no longer holds renders `MISSING` as
  itself rather than panicking or showing an empty pane.
- Evidence bounded at 8 with the true count and the remainder stated, so a
  thousand-evidence object cannot blow the frame budget.
- Pure, no ratatui: a new object shape is proven renderable in a unit test
  without a terminal.

---

## Part 3 — Verification, honestly

Green at `62ca678`, **checked against HEAD, not only the working tree**:

- `cargo fmt --all --check` → exit 0 (verified by stashing and re-running at
  the commit)
- `cargo clippy -p physis-hud --all-targets -- -D warnings` → exit 0
- `cargo test -p physis-hud` → 197 lib + 9 + 4 + 1, all pass
- `just snapshot-smoke` → exit 0
- `scripts/research-loop.sh` static+surface → **49 of 50 cards checked, all
  pass, 0 unverifiable**, after `cargo build -p physis-cli` so the
  binary-dependent cards could run at all.

  The fiftieth card is `static/binary-freshness/release`, present in the
  baseline receipt and **absent** here: the worktree has only a debug
  binary, so the runner had no release build to check. Confirmed by diffing
  the two receipts rather than by trusting the totals. That is a skip, and a
  skip is not a pass — close it with `cargo build --release -p physis-cli` if
  the release freshness matters.

**UNVERIFIED: the full `--gate` on a binary-present tree.** One run reported
**12 pass, 0 FAIL, 2 UNVERIFIABLE** — no `target/debug/physis` in a fresh
worktree. The rerun after building the binary was **aborted by the operator,
not completed**. `gate/fmt`, `gate/fmt-product`, `gate/ci` (clippy +
workspace tests, 276 s), `gate/invariants`, `gate/snapshot-smoke`,
`gate/mcp-e2e` and the five demos passed in that first run. So the gate is
*substantially* green and *formally* incomplete. Do not report it as passed.

### A defect in my own process, recorded so it is not repeated

Commit `97c8ecd` shipped an **unformatted `focus.rs`**: `cargo fmt --all` ran
*after* the commit and I staged only `object.rs`, `lib.rs`, `app.rs`. So
`cargo fmt --all --check` failed at that commit — verified by stashing and
re-running against HEAD. The gate I had reported green was green on the
working tree, not on the commit.

**Rule that follows: run gates against the committed state.** A green
working tree proves nothing about what you are about to hand over.

---

## Part 4 — Open decisions

**Should the lens system bind to World 2's slice-7 "world schemas as data", or
stay independent?**

Recommendation: **stay independent.** Slice 7 is itself P7 of a programme
that has not started past slice 2, so binding to it serialises the HUD behind
unstarted world work. `object.rs` and the name-matched containment list are
both deliberately shaped so schema-declared lenses can be added *alongside*
them later without a rewrite.

Cost of being wrong: a lens list that later needs a migration. Bounded, one
enum.

**When does P12 retire the tabs?** After the dogfood test passes end to end.
Not before. `TAB_TO_MODULE` stays as a compatibility map in the meantime.

---

## Part 5 — Remaining phases

Ordered by dependency, not by the mission's numbering. Each names the phase it
unblocks, because doing these out of order is what produces rework.

| # | Phase | What | Unblocks | Where it starts |
|---|---|---|---|---|
| 1 | **P4** | Lens normalisation. **First** resolve the `focus::Lens` / `lens::Lens` name collision. Then add source, structure, callers, tests, diff, notebook, edit, run, evidence, cognition. **The `l` → tab match at `event.rs:967-981` is the actual work**; 13 lenses on one cycling key needs a decision, and a labelled picker beats an unpredictable cycle. | P5, P6, P8 | `focus.rs:12-39`, `lens.rs:51`, `event.rs:967-981` |
| 2 | **P7** | **`Run` object model + harness in the HUD.** Highest value remaining, and mostly adoption: the surface exists at `physis_exec` and at the MCP ops area. Background execution is the one piece built from nothing, and the frame budget fails without it. | P8, P9, P12 | `event.rs:233`, `textbox.rs:56`; `physis_exec::{plan, run_plan_with}`, `ExecOutcome`, `verdict()`; `mcp/src/tools/areas/ops.rs` |
| 3 | **P3** | Unified world navigator: directories, files, symbols, notebooks, tasks and history as *projections of one snapshot*, plus pins and a real `:open`. | P12 | The LEFT pane (`views::render_left`); `graph_rows` `app.rs:1098` |
| 4 | **P5** | Editor: redo, selection, search, replace, go-to-line, new file, save-as. Every write stays through `Runtime::write_object`. | dogfood 8-11 | `crates/physis-editor/src/lib.rs:77-382` |
| 5 | **P6** | Notebook: navigable typed `[[kind:id]]` references, multiple notebooks, cell edit/reorder, stale-reference indication, cell execution. | dogfood 15-18 | `crates/physis-notebook/src/lib.rs:33-485` |
| 6 | **P9** | Filesystem mutation: create, rename, move, copy, delete, restore, each through runtime write paths producing world evidence. Complete the confirm gate that already exists. | dogfood 19+ | `event.rs` mutating arm; `.physis-capabilities` must gain the capability |
| 7 | **P8** | Solve and agents through the same Run/evidence/delta model. No solve-specific UI. | dogfood 22-25 | `physis_task::solve`; `/agent` already previews then launches and reports a delta |
| 8 | **P10** | Session restoration. One inspectable file under `.physis-next/`. Never auto-resume a destructive or running action — a restored `RUNNING` state comes back as interrupted and says so. | dogfood 29-30 | Nothing exists; §1.10 enumerates `.physis-next/` |
| 9 | **P11** | Projection cache keyed by (world revision, focus, lens). Deterministic cache assertions preferred over new wall-clock gates. | — | `tests/frame_budget.rs` |
| 10 | **P12** | Retire the tab/app metaphor — **only after the dogfood test passes end to end.** | — | `TABS` `app.rs:47`, `TAB_TO_MODULE` `modules/mod.rs:11`, `views.rs:76-101`, footer `layout.rs:54` |

**Why P4 before P7, against the mission's own numbering.** P7 introduces `Run`
and `run` as lenses and actions; if the lens list is still a six-variant enum
mapped by a hardcoded match, the Run object lands behind a view that cannot
address it. P4 is small and unblocks three later phases.

**Cross-cutting, before P7:** `Hud` is a **~50-field flat mutable bag**
(`app.rs:84-193`). Run registry, pins and session state landing there compound
the coupling. The mission's own rule — *"if a view function begins implementing
business logic, stop and extract it"* — argues for splitting state out first.
The precedent exists: `focus.rs`, `layout.rs`, `textbox.rs` and the new
`object.rs` are all pure modules with inline tests and no `Hud` dependency,
and `lib.rs` documents the one-way dependency direction with an invariant
enforcing it (`tests/invariants/hud_modules.rs`).

---

## Part 6 — Resume prompts

Each is self-contained: goal, files, verification. **A, then B, C, D, E, F** —
the order is dependency order (Part 5), not the mission's numbering. F is the
acceptance test and is worthless before the phases it measures.

### Resume A — close the verification gap and rebase (do this before any phase)

> In `/home/gio/pw-hud-shell` on branch `feat/hud-spatial-shell`: run
> `cargo build -p physis-cli` then `bash scripts/research-loop.sh --gate` to
> completion and report the final line verbatim. The previous `--gate` was
> aborted, so the full gate on a binary-present tree is UNVERIFIED. Then
> rebase onto `main` — it has moved 5 commits ahead with World 2's `MAY_CALL`
> and call-resolution work uncommitted in `kernel/relation.rs`,
> `query/engine.rs` and `discover/scan.rs`. Re-copy `.physis-next/` from
> `/home/gio/dev/physis-next` if graph/scale/render tests start failing: a
> rebase leaves the copied observation store stale and those failures are
> environmental, not real. Re-run the full gate after the rebase. Fix what the
> rebase breaks; do not redesign. Never `git add -A` — `research/loops/results/`
> receipts are another session's output.

### Resume B — P4, lens normalisation (the first *phase*)

> In `/home/gio/pw-hud-shell` on `feat/hud-spatial-shell`. Read
> `docs/plans/2026-10-07-hud-one-world.md` §1.5, §1.6 and §1.16 first — they
> are why this phase is not what it looks like.
>
> Step 1, mechanical: `focus::Lens` (six-variant enum, `focus.rs:12-39`) and
> `lens::Lens` (a projection trait, `lens.rs:51`) are different concepts with
> the same name in the same crate. Rename the trait to `Projection` and fix its
> impls and tests. Do this alone in its own commit; it touches nothing
> behavioural.
>
> Step 2, the actual work: add the lenses the mission names and the HUD lacks
> — source, structure, callers, tests, diff, notebook, edit, run, evidence,
> cognition. The blocker is **not** the enum, it is the hardcoded
> `Lens -> tab` match at `event.rs:967-981`, which currently collapses six
> lenses onto four views (deps and context both go to tab 2; graph and
> relations both go to tab 8). Extending the enum without addressing that match
> just grows a coupling.
>
> Step 3, a decision you must make and write down: with thirteen lenses, what
> does `l` cycle through? An unpredictable thirteen-step cycle is a worse
> control than the six it replaces. A labelled picker (`:lens <name>`, plus
> the active lens in the status line) is probably right, and it needs to be
> discoverable from `:help` and the unknown-verb answer at `event.rs`.
>
> Invariants to preserve, with the tests that already guard them:
> `focus.rs:267` — switching lens must not move focus; `focus.rs:32` `ALL`
> totality — a variant missing from `ALL` must fail. Extend both.
>
> When adding verbs to the textbox, follow the rule in `textbox.rs`: a verb
> goes into `READ_VERBS`/`MUTATING_VERBS` only with a live dispatch arm in the
> same commit. `open`/`show` were deleted from the grammar precisely because
> they had none.
>
> Verify: `cargo fmt --all --check`; `cargo clippy -p physis-hud --all-targets
> -- -D warnings`; `cargo test -p physis-hud`; `just snapshot-smoke`;
> `apps/physis-hud/tests/frame_budget.rs` — whose tab loop must now cover the
> new lenses, and whose header explains why it is its own binary (read it
> before changing it). Then `bash scripts/research-loop.sh --gate`.


### Resume C — P3, the unified world navigator

> In `/home/gio/pw-hud-shell`: turn the LEFT pane into one navigator over the
> world rather than a graph viewer beside a file manager. A file, a directory,
> a crate and a symbol are already entities with containment edges, so `p` and
> `breadcrumb_trail` already reach them — extend the LEFT pane to offer the
> mission's sources (directory tree, package tree, symbol tree, recent, pinned,
> notebooks, tasks, search results, revision history) as *projections of the
> same snapshot*, all selecting through the one focus system.
>
> Add pins: a `Vec<EntityId>` in its own pure module (not on `Hud` — see the
> cross-cutting note in Part 5), `:pin` to add, rows in the LEFT pane. Pins are
> shortcuts back into the same world, not a second sidebar universe.
>
> Add **`:open <path>`**, the mission's universal open, resolving anything the
> user can type — a path, a symbol name, a full `kind:key` — to an `EntityId`,
> focusing it through `focus.set`. Report honestly when it is not in the world
> rather than guessing a near match. Note that `open`/`show` were **removed**
> from the textbox grammar in an earlier commit because they had no dispatch
> arm; this adds the arm first this time, and `textbox.rs`'s rule is that a
> verb and its arm land in the same commit.
>
> Also worth folding in here: **autocomplete** in the magic box (commands,
> paths, symbols, object ids, relations, notebooks, recent objects). It is
> mission-named, absent today, and it is textbox work rather than a phase of
> its own.
>
> Tests: universal open over file / directory / symbol / unknown-id, pins
> surviving focus movement, breadcrumbs stable across a pin toggle, and an
> unknown `:open` target reporting rather than resolving to something else.
>
> Verify: `cargo fmt --all --check`; `cargo clippy -p physis-hud --all-targets
> -- -D warnings`; `cargo test -p physis-hud`; `just snapshot-smoke`;
> `apps/physis-hud/tests/frame_budget.rs`.


### Resume D — P7, the harness and the `Run` object (biggest single win)

> In `/home/gio/pw-hud-shell`. Read §1.7, §1.13, §1.14 and §1.16 first — this
> phase is mostly adoption, not construction, and planning it as construction
> would duplicate a surface that already exists.
>
> What exists and is unused by the HUD:
> - `physis_exec::{ExecRequest, plan, run_plan_with}` and
>   `ExecOutcome { argv, exit_code, stdout, stderr, timed_out }` with
>   `verdict()` (`crates/physis-exec/src/lib.rs:40,127,244`,
>   `crates/physis-exec/src/mirrord.rs:292-307`). `grep -rn physis_exec
>   apps/physis-hud/src` returns **nothing**.
> - An MCP ops area already exposing `physis.action.run`,
>   `physis.action.execute`, `physis.test.run` and `physis.exec`
>   (`crates/physis-mcp/src/tools/areas/ops.rs`, its `handles()` predicate is
>   derived from its own match arms). **The HUD is behind its own MCP.** Decide
>   deliberately whether the HUD calls the facade directly or goes through the
>   MCP area, and write the reason down — the mission says "use runtime facade
>   capabilities", and picking without a reason is how two harnesses appear.
> - A **disabled shell escape**: `textbox::Command::Shell` parses `$cmd` and
>   `event.rs` answers "shell disabled by policy (`{c}` not run)". The grammar
>   hook exists; P7 decides the policy.
>
> What is genuinely new and must be built:
> 1. A `Run` type with the mission's fields — id, kind, command_or_action,
>    cwd, environment, capabilities, input_focus, context_ref, started_at,
>    state, stdout, stderr, exit_status, duration, changed_files,
>    resulting_revision, evidence.
> 2. **Background execution.** A run must not block the draw loop or
>    `apps/physis-hud/tests/frame_budget.rs` fails at 16 ms. Run on a thread and
>    poll, and write a test that proves a running job does not stall a frame.
> 3. A `Run` must be focusable through the existing one-focus system and
>    renderable through `object.rs` — **not** through a bespoke pane. The
>    mission's rejection test applies: does this create another application?
> 4. The BOTTOM activity strip: RUNNING / PASSED / FAILED / CANCELLED /
>    BLOCKED. Selecting one focuses it.
>
> Support: execute, cancel, rerun, bounded output, command history, cwd
> awareness, stdout/stderr distinction, exit code, duration, copy output into
> the notebook, attach output as evidence, navigate changed files and the
> resulting revision, re-observe after a mutating run.
>
> Scope discipline: remove `run` from `UNSUPPORTED_VERBS` **only**. Leave
> `write`, `delete`, `execute` refused until P9. Do not build a terminal
> emulator; structured execution first, PTY later.
>
> Cross-cutting, do this first or the `Hud` struct gets worse: `Hud` is a
> ~50-field flat mutable bag (`app.rs:84-193`). Put the run registry in its own
> pure module like `focus.rs` and `object.rs`, with inline tests and no `Hud`
> dependency, rather than adding a dozen more fields.
>
> Verify: fmt, clippy, `cargo test -p physis-hud`,
> `bash scripts/research-loop.sh --gate`, plus tests for harness success,
> failure and cancellation, and the frame-stall test above.


### Resume E — P10, session restoration

> In `/home/gio/pw-hud-shell`: persist HUD session state so focus, trail,
> lens, pins, notebook and recent-run metadata survive a restart. No UI state
> file exists — `.physis-next/` holds only `constraints.jsonl`,
> `embed-cache`, `interview-cache`, `notebooks` and `observations.jsonl`. The
> mission forbids an opaque project database: one inspectable file under
> `.physis-next/`, kept separate from canonical world state. Restore focus,
> trail, lens, editor buffer reference, notebook, pins, recent run metadata and
> layout. **Do not silently restart destructive or still-running actions** — a
> restored `RUNNING` state must come back as interrupted and say so. Verify: a
> test that writes state, reloads, and asserts focus and pins came back, plus
> one that a restored in-flight run reads as interrupted rather than as a pass.

### Resume F — the dogfood test

> In `/home/gio/pw-hud-shell`: run the mission's 30-step dogfood acceptance test
> against the current HUD without leaving `physis-hud`, and report which steps
> pass, which fail, and which are UNVERIFIED — with the phase that would close
> each gap. Steps that cannot work today and why: step 2's
> `Runtime::write_object` does not exist in this world's entity keys (verify
> against a real symbol id before calling it a defect); step 5 (source lens)
> needs P4; step 19 (run a test from the harness) needs P7; step 23 (invoke an
> agent) needs the `:agent` delta inspection; step 30 (session restore) needs
> P10. Do not paper over a gap — by the mission's own rule a step requiring an
> external editor, file manager, graph viewer or plain shell is a FAIL, and it
> names the phase that closes it. Then run the mission's 10-step non-code test
> (machine → service → process/socket → provenance → history → pin → notebook →
> harness read → evidence → back) and report whether the same
> OBJECT+FOCUS+LENS+ACTION architecture carries it with no service-specific
> dashboard.

---

## Part 7 — Non-goals, restated because they are the point

No new chat UI. No decorative panes. No terminal emulator. No second state
store, no second graph, no bypass of the `Runtime` canonical write path. No
`MAY_CALL` → `CALLS` promotion. No model confidence as world truth. No models
in the kernel. No silhouette of an IDE with panes rearranged.

The rejection test before any new UI concept: **does this create another
application?** If yes, rework it. Build OBJECT + FOCUS + LENS + ACTION +
HISTORY.

And the repo's own rule, which applies to all of the above: **UNVERIFIABLE is
not a pass.** A card that did not check is counted in the denominator and
never folded into the pass column.