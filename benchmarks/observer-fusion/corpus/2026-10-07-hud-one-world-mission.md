# The mission, verbatim

This is the brief as received, kept verbatim so the plan can never drift from
what was asked. Nothing here is the author's interpretation; the plan that
uses it is `2026-10-07-hud-one-world.md`.

---

MISSION: turn `physis-hud` into the primary human interface to Physis and the
concrete realization of the "remake the computer" idea.

This is NOT an IDE redesign.

This is NOT a file manager + editor + notebook + terminal glued together.

This is NOT a dashboard with twenty panes.

The HUD must become:

    A NAVIGABLE VIEWPORT INTO ONE EVOLVING WORLD.

The governing spatial rule is:

    LEFT   →   CENTER   →   RIGHT

where:

    LEFT
    = where this object came from
    = containment
    = causes
    = previous focus
    = history
    = inputs

    CENTER
    = the current focused object
    = what the user is inspecting / editing / running / thinking about NOW

    RIGHT
    = where this object leads
    = outgoing relations
    = possible actions
    = consequences
    = results
    = future state

The user moves through reality horizontally.

Following an object should conceptually shift the world left:

    A → B → C

focus C:

    A → B     C → D → E

The identity remains stable.
The projection may change.

==================================================
CORE LAW
==================================================

THERE IS ONE FOCUS.

The HUD always has one current focused object.

That object may be:

- repository
- workspace
- directory
- file
- symbol
- expression
- package/crate
- relation
- revision
- delta
- diff
- notebook
- notebook cell
- query result
- task
- command run
- test
- verification result
- agent run
- process
- service
- socket
- machine
- primitive
- hypothesis
- evidence item
- arbitrary future world object

Everything else is arranged around that focus.

Do NOT create separate focus models for:

- editor
- graph
- notebook
- task runner
- search
- agent UI

There is ONE focus history.

There is ONE navigation model.

There is ONE world.

==================================================
CORE SPATIAL MODEL
==================================================

The HUD should conceptually render:

┌──────────────────────┬──────────────────────────────────┬──────────────────────────────┐
│ BEFORE / CONTEXT     │ FOCUS                            │ AFTER / POSSIBILITY          │
│                      │                                  │                              │
│ parents              │ current object                   │ outgoing relations           │
│ containers           │ current lens                     │ consequences                 │
│ callers              │ current content                  │ possible actions             │
│ previous revisions   │                                  │ callees                      │
│ previous focus       │                                  │ tests                        │
│ evidence             │                                  │ resulting state              │
└──────────────────────┴──────────────────────────────────┴──────────────────────────────┘

Plus:

TOP:
    workspace
    breadcrumbs
    revision
    magic textbox

BOTTOM:
    tasks
    command output
    diagnostics
    world state
    status

Adapt layout intelligently for terminal dimensions.

Do not hard-code three equal columns.
The focused object should receive most space when necessary.

==================================================
THE HUD IS NOT APP-CENTRIC
==================================================

Do NOT organize the user experience primarily around:

    File Manager
    Editor
    Notebook
    Terminal
    Graph
    AI Chat

Those capabilities exist, but they are NOT the primary abstraction.

Instead:

                ┌─────────────────────┐
    previous →  │   FOCUSED OBJECT    │  → related / resulting
                └─────────────────────┘
                          │
                          │ change projection
                          ▼
        source
        graph
        edit
        notebook
        diff
        context
        history
        execution
        cognition
        evidence

The OBJECT remains the same.
Only the LENS changes.

==================================================
LENSES
==================================================

Every focused object may expose applicable lenses.

Example lenses:

    source
    graph
    relations
    structure
    dependencies
    callers
    tests
    history
    diff
    context
    notebook
    edit
    run
    evidence
    cognition
    scale
    raw

Not every object supports every lens.

Lens switching must NOT change focus.

Example:

    focused:
        symbol:Runtime::write_object

    lens=source
        show definition

    lens=graph
        show bounded ego graph

    lens=history
        show revisions touching symbol

    lens=context
        show exactly what an agent receives

    lens=edit
        edit backing file/object

Same identity.
Different projection.

==================================================
NAVIGATION
==================================================

Navigation must feel browser-like and spatial.

Required operations:

    follow
    back
    forward
    parent
    child
    previous revision
    next revision
    expand relation
    collapse relation
    change lens

Maintain one navigation trail.

Conceptually:

    trail: A → B → C → D

If current focus is D:

    LEFT:
        C
        B
        A

    CENTER:
        D

    RIGHT:
        outgoing relations from D

Following E:

    A → B → C → D → E

Back:

    A → B → C → D

Forward:

    E

Do not discard navigation context when changing view.

==================================================
BREADCRUMBS
==================================================

Always make spatial position legible.

Examples:

    physis-next
      › crate:physis-runtime
      › file:src/lib.rs
      › symbol:Runtime::write_object

or:

    workspace
      › notebook:Architecture
      › cell:12
      › symbol:DecisionEngine

Breadcrumb components are navigable.

==================================================
UNIVERSAL OPEN
==================================================

Implement a universal conceptual operation:

    open(object)

The system decides which projection is most appropriate.

Examples:

    open(directory)
        → children/navigation

    open(file)
        → file view

    open(symbol)
        → definition + relations

    open(diff)
        → diff lens

    open(task)
        → task execution/result

    open(revision)
        → revision delta

    open(notebook)
        → notebook cells

    open(process)
        → process inspector

    open(unknown_entity)
        → generic fields + relations

This avoids object-type-specific navigation plumbing.

==================================================
WORLD / FILE BROWSER
==================================================

The filesystem must appear as one projection of the world.

Do NOT maintain a conceptual split between:

    filesystem browser

and:

    semantic graph browser

A file is a world object.

A directory is a world object.

A symbol is a world object.

A service is a world object.

A notebook cell is a world object/reference-bearing object.

The left-side navigator should be capable of showing:

- directory tree
- package tree
- symbol tree
- recent objects
- pinned objects
- notebooks
- tasks
- search results
- revision history

Selecting anything changes focus.

==================================================
GRAPH VIEW
==================================================

The graph should become a real navigation system.

Never attempt to render the whole repository.

Default:

    bounded ego graph around current focus

Support:

- incoming edges
- outgoing edges
- relation filtering
- expand one hop
- collapse branch
- follow edge
- inspect edge provenance
- change abstraction level
- switch graph lens
- open source
- pin object
- add object to notebook

Possible graph lenses:

    structure
    callers
    dependencies
    tests
    history
    runtime
    evidence
    causality

Focused node must always be visually obvious.

==================================================
LEFT → CENTER → RIGHT SEMANTICS
==================================================

Use the horizontal rule consistently.

CODE:

    caller
      → function
      → callee

FILES:

    directory
      → file
      → symbol

EXECUTION:

    intent/context
      → action
      → receipt/result

VERIFICATION:

    candidate
      → check
      → evidence

EDITING:

    old revision
      → working object
      → proposed delta

GIT:

    previous revision
      → current revision
      → next revision

REASONING:

    evidence
      → hypothesis
      → prediction

AGENT WORK:

    context
      → agent run
      → proposed delta

NOTEBOOK:

    referenced objects
      → current cell
      → references/results

SYSTEMS:

    machine
      → service
      → process/socket

SEARCH:

    query
      → result
      → neighborhood

Try to preserve this directionality whenever it is semantically coherent.

==================================================
EDITOR
==================================================

Editing is a lens over a focused writable object.

Do NOT launch a separate editor world.

Example:

LEFT:
    containing file
    previous revision
    related symbol

CENTER:
    editable source

RIGHT:
    callers
    tests
    expected affected relations
    possible actions

Requirements:

- line numbers
- cursor movement
- insert/delete
- selection
- undo
- redo
- search
- replace
- go-to-line
- save
- new file
- save-as where useful
- dirty indicator
- stale-write protection
- diff before save
- syntax highlighting where available

Saving MUST follow:

    edit
      → canonical write
      → observation
      → revision
      → delta
      → world update

The editor never becomes a second source of truth.

==================================================
CODE NAVIGATION
==================================================

From source:

    go definition
    callers
    callees
    references
    tests
    containing module
    containing crate
    history
    diff
    related evidence

These actions move/change Physis focus.

Do not build hidden editor-only symbol navigation state.

==================================================
NOTEBOOK
==================================================

Notebook is another projection over the same world.

It must become a usable thinking environment.

Cell types may include:

- markdown
- plain text
- object reference
- query
- code
- command
- result
- task
- snapshot
- evidence

Object references should be real links:

    [[symbol:...]]
    [[file:...]]
    [[revision:...]]
    [[task:...]]
    [[service:...]]

They should be navigable.

Required:

- create notebook
- rename
- add cell
- edit cell
- delete cell
- reorder cells
- execute applicable cell
- pin focus into cell
- reference arbitrary world object
- ask over selected cells
- compile context from selected cells
- follow referenced object
- indicate stale references

The notebook must NOT become a disconnected document silo.

==================================================
NOTEBOOK SPATIAL RULE
==================================================

For a focused notebook cell:

LEFT:
    referenced inputs
    previous cells
    source objects

CENTER:
    current cell

RIGHT:
    referenced outputs
    resulting object
    executed result
    following cells

A notebook therefore participates in the same horizontal machine.

==================================================
HARNESS
==================================================

Create a general execution substrate.

Do NOT think "terminal first".

Think:

    structured action runner

The harness should execute:

- commands
- tests
- builds
- scripts
- physis solve
- agents
- MCP actions
- project commands
- selected safe system operations
- long-running processes where appropriate

A run should become a first-class object.

Example:

Run {
    id
    kind
    command_or_action
    cwd
    environment
    capabilities
    input_focus
    context_ref
    started_at
    state
    stdout
    stderr
    exit_status
    duration
    changed_files
    resulting_revision
    evidence
}

==================================================
HARNESS SPATIAL MODEL
==================================================

A run naturally renders:

LEFT:
    intent
    focus
    context
    inputs

CENTER:
    running action
    stdout
    stderr
    progress

RIGHT:
    result
    exit status
    produced files
    evidence
    delta
    new world revision

Example:

    CONTEXT
       → cargo test
       → PASS / evidence

or:

    task request
       → agent
       → proposed delta

This must use the same focus/navigation model.

==================================================
HARNESS REQUIREMENTS
==================================================

Support:

- execute
- cancel
- rerun
- scroll output
- command history
- cwd awareness
- stdout/stderr distinction
- exit code
- duration
- copy output into notebook
- attach output as evidence
- navigate changed files
- navigate resulting revision
- re-observe after mutating runs

Do NOT begin with a full terminal emulator.

PTY support can come later.

Structured execution first.

==================================================
TASK MODEL
==================================================

Unify:

- command runs
- tests
- builds
- agent tasks
- solve attempts
- scripts
- external actions

under one Task/Run abstraction.

Activity strip/panel shows:

    RUNNING
    PASSED
    FAILED
    CANCELLED
    BLOCKED

Selecting one makes it the current focus.

Then the same HUD machinery handles it.

==================================================
AGENTS
==================================================

Agents are tools inside the computer.

They do not own the computer.

Desired flow:

    focus object
      → inspect context
      → invoke agent
      → agent becomes Run
      → inspect output
      → inspect delta
      → verify
      → accept / reject
      → changed world

No dominant "AI chat application".

A model is a capability provider.

The world remains primary.

==================================================
SOLVE
==================================================

Integrate `physis solve` as another first-class action.

Spatially:

LEFT:
    goal
    context
    constraints
    current world

CENTER:
    attempt
    candidate
    verification progress

RIGHT:
    counterexample
    evidence
    verified delta
    resulting revision

If World 2 introduces structured counterexamples and receipts, the HUD should
render them generically rather than inventing a solve-specific application.

==================================================
CONTEXT
==================================================

`:context` on any appropriate focus should show EXACTLY what an agent would
receive.

Expose:

- selected objects
- retrieval legs
- scores
- reasons
- graph neighborhood
- dropped objects
- token budget
- assembled packet
- embedder/backend
- semantic honesty label

Then:

    :agent ...
    :solve ...

should use that packet unless explicitly overridden.

The user should always be able to answer:

    WHAT DOES THE MACHINE THINK IS RELEVANT RIGHT NOW?

==================================================
MAGIC TEXTBOX
==================================================

The textbox becomes the universal interaction entrypoint.

Examples:

    timeout

        search world

    Runtime::write_object

        search/focus symbol

    :open src/client.rs

        focus object

    :edit

        switch current focus to edit lens

    :graph callers

        graph lens / relation filter

    :context

        context lens

    :run cargo test

        create Run

    :solve fix timeout handling

        create solve Run

    :agent opencode improve this

        create agent Run

    :note

        add current focus to notebook

    :back

    :forward

    :parent

Use autocomplete for:

- commands
- paths
- symbols
- object ids
- relations
- notebooks
- recent objects

Keep syntax small.

==================================================
SEARCH
==================================================

One search system over:

- files
- paths
- file contents
- symbols
- graph entities
- relations
- notebooks
- tasks
- history
- primitive corpus

Each result says WHY it matched.

Example:

    retry

    symbol:src/client.rs::retry
        exact symbol

    file:src/retry.rs
        path

    primitive:design.retry-backoff
        BM25

    notebook:timeouts/cell:7
        text

    revision:abc123
        changed entity

Selecting result changes focus.

==================================================
GENERIC OBJECT RENDERING
==================================================

New future world object types should not require a new application.

Every object should be renderable generically from:

- identity
- kind
- fields
- incoming relations
- outgoing relations
- provenance
- history
- available lenses
- available actions

Specialized renderers may improve important types, but generic rendering
must always work.

This is essential.

World 2 may introduce new:

- relations
- entity types
- uncertainty objects
- evidence
- counterexamples
- receipts

The HUD must remain usable without bespoke UI work.

==================================================
FILESYSTEM MUTATION
==================================================

Support ordinary work:

- create file
- create directory
- rename
- move
- copy
- delete
- restore where possible

Mutations must go through safe Physis/runtime paths.

Every mutation produces world evidence/revision information where appropriate.

No direct invisible filesystem state changes.

==================================================
WORKSPACE
==================================================

A workspace consists of:

- roots
- current world
- current focus
- navigation trail
- open/pinned objects
- notebooks
- running/recent tasks
- layout state
- current lens

Persist UI/session state separately from canonical world state.

Use an inspectable file under `.physis-next/`.

Do not create an opaque project database for UI state.

==================================================
SESSION RESTORATION
==================================================

On restart restore, where safe:

- focus
- history
- lens
- editor buffer reference
- notebook
- pins
- recent tasks
- layout

Do NOT silently restart destructive/running actions.

==================================================
LAYOUT
==================================================

The default desktop should visually embody time/causality/navigation.

Recommended conceptual shape:

┌───────────────────────────────────────────────────────────────────────────────────────────┐
│ PHYSIS | workspace | breadcrumb | revision | search / command                           │
├──────────────────────┬───────────────────────────────────────────┬────────────────────────┤
│ LEFT                 │ CENTER                                    │ RIGHT                  │
│                      │                                           │                        │
│ history              │ FOCUSED OBJECT                            │ relations              │
│ parents              │                                           │ consequences           │
│ containers           │ active lens                               │ actions                │
│ callers              │ source / graph / edit / note / run        │ results                │
│ references           │                                           │ tests                  │
│ inputs               │                                           │ next objects           │
├──────────────────────┴───────────────────────────────────────────┴────────────────────────┤
│ ACTIVITY / RUNS / DIAGNOSTICS                                                           │
├───────────────────────────────────────────────────────────────────────────────────────────┤
│ status | world revision | dirty | backend | semantic label | tasks                        │
└───────────────────────────────────────────────────────────────────────────────────────────┘

The central region may consume 60–80% width for editor/notebook/run output.

Left and right may collapse.

On small terminals degrade gracefully.

==================================================
HORIZONTAL SHIFT
==================================================

Following an object should visually feel like the world moved.

Optional implementation:

- retain previous focus briefly in LEFT
- new focus expands in CENTER
- populate RIGHT with its neighborhood

No excessive animation is required.

The conceptual motion matters more than literal animation.

Navigation should feel:

    world flows left
    attention stays center

==================================================
PINS
==================================================

Pins are anchors against movement.

A pinned object remains quickly available while focus moves elsewhere.

Use pins for:

- important source
- active task
- reference document
- hypothesis
- service
- notebook
- revision

Do not let pins become another app/sidebar universe.

They are shortcuts back into the same world.

==================================================
ABSTRACTION / SCALE
==================================================

The left→right model should coexist with scale.

A user should move:

    system
      → package
      → file
      → symbol
      → expression

or back out.

Scale changes must remain projections over the world.

Do not create fake graph entities solely for UI scale.

==================================================
HISTORY
==================================================

History is not just Git.

For an object expose:

- focus history
- world revisions
- file revisions
- observed changes
- actions performed
- tasks affecting it
- agent edits
- evidence over time

History should be navigable as objects.

==================================================
PROVENANCE
==================================================

Everything important should answer:

    HOW DO YOU KNOW THIS?

For relations, fields, results and decisions expose provenance/evidence where
available.

Examples:

    CALLS
        static scanner
        file span
        revision

    CALLED
        runtime observation
        test run
        timestamp

    context candidate
        BM25
        graph relation
        score

    agent delta
        run id
        model
        receipt

Provenance is a lens/action, not buried debug output.

==================================================
KEYBOARD-FIRST
==================================================

Keyboard navigation is primary.

But avoid dozens of invisible modal shortcuts.

Prioritize:

    movement
    follow
    back
    forward
    lens
    search
    command
    edit
    run
    pin

Everything else can live in:

    command palette
    contextual action list
    :help

Show context-sensitive available actions.

==================================================
MOUSE
==================================================

Only after keyboard architecture is correct.

Optional:

- select object
- follow edge
- scroll
- focus region
- resize panel

Do not let mouse functionality distort the model.

==================================================
PERFORMANCE
==================================================

The HUD must feel like a computer, not a report generator.

Target ordinary interaction <16ms where realistic.

Rules:

- never traverse whole graph each frame
- cache projections keyed by world revision + focus + lens
- incremental recomputation
- background expensive work
- bounded outputs
- never block rendering on network/model calls

Preserve existing HUD frame-budget tests.

Add performance tests for:

- 10k+ entity world
- large directory
- large source file
- dense local graph
- large notebook
- large run output
- long focus history

==================================================
ARCHITECTURE
==================================================

HUD owns presentation and navigation.

HUD does NOT own:

- graph semantics
- search implementation
- writes
- discovery
- context compilation
- solve
- verification
- agent logic
- task policy
- notebook persistence rules
- canonical world state

Use runtime facade capabilities.

If a view function begins implementing business logic, stop and extract it.

==================================================
CONCURRENT DEVELOPMENT SAFETY
==================================================

Another branch/workstream is modifying World 2:

- call discovery
- unresolved candidates
- graph fidelity
- verification
- structured counterexamples
- receipts
- world semantics

Therefore:

- minimize modifications to world/discovery/reasoning crates
- use additive facade interfaces
- isolate HUD commits
- merge/rebase often
- generic object renderer first
- do not hardcode current relation set
- do not assume every relation is certain
- render certainty/provenance metadata if supplied

HUD must become MORE robust as the world becomes richer.

==================================================
IMPLEMENTATION ORDER
==================================================

P0 — establish spatial shell

Build persistent:

    LEFT
    CENTER
    RIGHT
    TOP COMMAND
    BOTTOM ACTIVITY

without deleting current functionality.

P1 — one-focus architecture

Unify focus across existing panes/modules.

Implement:

    follow
    back
    forward
    parent
    breadcrumbs
    open(object)

P2 — generic object renderer

Ensure arbitrary entity types can be displayed and traversed.

P3 — unified graph/file navigation

Directories, files, symbols and graph objects become one navigable system.

P4 — lens system

Normalize:

    source
    graph
    relations
    history
    context
    diff
    edit
    notebook
    run

around stable focus.

P5 — editor

Make daily-use editing possible.

P6 — notebook

Make graph-native notebook complete enough for actual work.

P7 — harness

Structured execution + Task/Run object model.

P8 — solve/agent integration

Render them through the same Run/evidence/delta model.

P9 — filesystem operations

Safe project mutation from HUD.

P10 — workspace/session restoration

P11 — performance + polish

P12 — remove obsolete tab/app metaphors where no longer useful

Compatibility may remain temporarily, but the final UX must not depend on
tab proliferation.

==================================================
DOGFOOD ACCEPTANCE TEST
==================================================

Perform this ENTIRE workflow without leaving `physis-hud`.

1. launch HUD in physis-next

2. search:
       Runtime::write_object

3. focus the symbol

4. inspect:
       callers
       callees
       tests
       provenance

5. switch to source lens

6. switch to graph lens

7. return to source without losing focus

8. enter edit lens

9. change a comment/doc line

10. inspect proposed diff

11. save

12. observe:
       new revision
       world delta
       changed relations

13. navigate to resulting revision

14. navigate back to symbol

15. create/open notebook

16. insert current symbol reference

17. write a note

18. follow symbol link from notebook

19. run relevant test from harness

20. inspect:
       stdout
       stderr
       exit code
       duration
       resulting evidence

21. follow test/evidence back to source

22. inspect current context packet

23. invoke an agent on a tiny related change

24. inspect:
       context
       agent run
       proposed delta
       verification
       receipt

25. accept or reject deliberately

26. navigate backwards through focus trail

27. pin two useful objects

28. quit

29. reopen HUD

30. recover:
       focus
       useful history
       pins
       notebook
       recent task metadata

FAIL if normal interaction requires:

- external editor
- external file manager
- external graph viewer
- another agent frontend
- separate notebook app
- ordinary shell for a task the harness should support

==================================================
NON-CODE ACCEPTANCE TEST
==================================================

Prove the HUD is a computer, not an IDE.

Using existing local-host observations:

1. focus machine
2. follow to service
3. follow to process/socket
4. inspect provenance
5. inspect history
6. pin object
7. add to notebook
8. execute a safe relevant action/read through harness
9. inspect evidence/result
10. navigate back

No service-specific dashboard.

The same:

    OBJECT
    + FOCUS
    + LENS
    + ACTION

architecture must work.

==================================================
TESTS
==================================================

Add deterministic tests for:

- focus transition
- horizontal navigation state
- back/forward
- breadcrumbs
- parent navigation
- universal open
- lens switching preserving focus
- arbitrary entity rendering
- graph relation traversal
- file navigation
- editor save → world revision
- stale editor refusal
- notebook reference navigation
- harness success
- harness failure
- cancellation
- agent run display
- solve run display
- context packet rendering
- receipt navigation
- session restoration
- unknown relation type
- unknown entity type
- small terminal
- long output truncation
- frame budget

Preserve:

    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    physis-check all

Report anything that cannot run as UNVERIFIED.

==================================================
DESIGN REJECTION TEST
==================================================

Before implementing any major UI concept ask:

    DOES THIS CREATE ANOTHER APPLICATION?

If yes, reject/rework it.

Do not build:

    editor app
    notebook app
    terminal app
    graph app
    git app
    AI app

Build:

    OBJECT
      +
    FOCUS
      +
    LENS
      +
    ACTION
      +
    HISTORY

==================================================
END STATE
==================================================

The user runs:

    physis-hud .

They enter one world.

They can:

- browse files
- browse directories
- browse symbols
- browse graph relations
- inspect provenance
- inspect history
- read code
- edit code
- create files
- delete/move/rename
- inspect diffs
- run commands
- run tests
- inspect processes
- invoke agents
- invoke solve
- inspect model context
- inspect cognition
- inspect verification
- write notebooks
- link notes to real objects
- follow revisions
- inspect evidence
- operate non-code objects

without conceptually switching applications.

The machine should continuously feel like:

    PAST / CAUSE / CONTAINER
               ↓
             LEFT

          CURRENT REALITY
               ↓
             CENTER

    CONSEQUENCE / RELATION / POSSIBILITY
               ↓
             RIGHT

And navigation should continuously transform:

    A → B → C

into:

    A → B     C → D → E

into:

    B → C     D → F → G

as the user moves.

Files, programs, notes, agents, processes and history are not separate
universes.

They are objects in one changing world.

The HUD is the viewport through which that world is inhabited.

That is the "remake the computer" objective.

Build toward that, not toward a better IDE.