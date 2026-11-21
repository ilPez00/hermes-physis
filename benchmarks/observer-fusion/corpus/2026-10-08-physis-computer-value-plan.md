# Physis as a learning, responsive computer — execution plan

> **For workers:** planning only. Execute one assigned card through the existing
> [worker protocol](../../tasks/agent-loops.md). Use `executing-plans` when
> implementation is assigned. Delegate only when explicitly assigned; use the
> existing registry and handoff mechanism. This document starts no agents.

**Goal:** develop Physis into a general computer that interprets user intent,
executes inspectable programs, learns scoped rules from checked experience and
stays responsive, using the smallest model that meets measured requirements;
fund that direction through independently useful distribution and paid outcomes.

**Architecture:** preserve the append-only observation store and canonical object
IDs. Compose existing runtime, graph, context, editor, notebook, tasks and checks
through CLI, native MCP and HUD. Physis owns executable state, contracts, checks,
history and learned procedures; the replaceable model translates intent and
proposes hypotheses within those contracts. Keep commercial Context MCP small
and separate. Learned primitives earn reuse through discriminating checks,
repeated confirmation and counterexamples; symbolic learning and behavioral
models are central architectural work, independent of first-sale timing.

**Tech stack:** existing Rust workspace, stdlib, current dependencies, native
MCP, ratatui HUD, Bash/Python release and evidence scripts. No new platform,
database, orchestration service, model backend or dependency is assumed.

**Spec:** operator mission in this conversation; architectural and audit evidence
in [audit execution plan](2026-10-07-physis-audit-execution.md); commercial
[value contract](../commercial/VALUE_PROPOSITION.md) and
[first-revenue experiment](../commercial/FAST_PATH.md). This amendment supplies
current priorities. Historical cards and receipts remain intact.
The supplied **“PHYSIS — THE IMPOSSIBLE MACHINE”** mission is a design input for
the parallel implementation lane. Its instruction to implement is not this
assignment: update the plan, packets, loops and gates, then commit planning only.

## Global constraints

- This assignment writes and commits planning documents only. No product
  implementation, checkout switch, build, publication, outbound message, charge
  or licence issuance. The requested commit contains only owned planning changes.
- One world, many lenses. Reuse shared mechanisms before adding another path.
- One founder; 40h/week; €0 new upfront spend. Model availability is a constraint,
  not a reason to buy a new service automatically.
- No verified paid customers. Gift recipients, stars and compliments are not sales.
- Existing offer: €49 founding licence for the first 25 licences, €79 standard;
  perpetual named-developer v1.x. Offer changes require their own concrete review.
- Supported artifact proof currently covers Linux x86_64. Other platforms remain
  unverified until exercised on those platforms.
- Keep engine/research source private. Public Computer means free executable
  distribution; do not describe the private engine as open source.
- Commercial Context MCP remains its existing 12-tool coding product. Native MCP
  is the broader world interface. No cognition, CA or research dependency added
  to the paid package without a demonstrated customer requirement and boundary gate.
- Never replace UNKNOWN with a guessed edge, guessed effect or asserted PASS.
- Receipts bind source, executable, root, world revision, checks and environment.
  UNVERIFIABLE, NOT MEASURED and timeout remain visible in denominators.
- Preserve concurrent work. Worker writes at most five allowed files per card;
  split a card before expanding scope. No destructive cleanup of another lane.
- Existing publication authorizations and receipts are history. This planning
  amendment does not initiate another release or customer campaign.

## Review focus

1. **Anonymous download:** a private GitHub release must not be sold or presented
   as an anonymously accessible artifact. V01/V02 test the actual delivery host.
2. **Concurrent edit / stale world:** a check on revision A cannot certify B;
   changed source invalidates prepared writes and pinned evidence. V06/V12 test it.
3. **Program meaning:** two ports sharing a type do not imply matching targets,
   units or effects. V09/V11 reject semantically wrong compositions.
4. **Whole interaction cost:** smaller display text can increase model usage or
   lose evidence. V07 keeps schemas, history, retries and losing rows.
5. **Buyer reality:** free-native overlap, unsupported languages, missing tools,
   failed delivery and non-use must affect the offer. V05/V08 record these cases.
6. **Living computer:** slow observation, inference and execution cannot block
   navigation or cancellation. V16a/V16b prove process ownership and interaction;
   V17 measures the minimum model capability instead of assuming it.

## 1. Current evidence and changed priorities

Snapshot inspected 2026-10-08. Re-pin before execution; another session is active.
During final document verification, concurrent edits appeared in
`crates/physis-discover/src/scan.rs` (shell-text indexing),
`crates/physis-agent/src/lib.rs` (`adapter:` ID normalization), and the OpenCode
dogfood section appended to `tasks/plan.md`. They were preserved. Their session
reports are historical evidence to review, not executable checks run by this
planning turn; neither source edit closes the inspected process/learning gaps.

| Surface | Verified state | Consequence |
|---|---|---|
| Shared checkout | Initially `feat/compute-primitives` at `23f3fd4a`; during review another session moved to `audit/aud-wip` at `c1000159`, committing earlier audit documents; new value plan and generated `.bin` remain untracked | Main checkout is actively shared. Pin every read/assignment; do not infer source identity from the first status check. |
| GitHub engine main | Live API returned `a9a5510e7414c1f078bb4d2015f6dea9cd48791b`; local main/tracking ref agree | Old “81 commits unpublished” snapshot no longer describes main. New docs followed the release. |
| Compute branch | `23f3fd4a` pushed; not ancestor of current main | Arithmetic, unit conversion and date difference need separate review/integration. Not inside released 1.2.0. |
| Engine visibility | GitHub API: `private=true` | Its release is a qualification/handoff location, not public customer delivery. |
| Engine v1.2.0 | Tag source `3b9dd51d`; new CLI/HUD and commercial archives published | Preserve this tag and qualified bytes. Do not rebuild them from the newer feature branch and silently reuse the identity. |
| Public Computer v1.2.0 | Live API still returns old archive SHA `f167239a…`, published 2026-10-07 | Public installation still misses later fixes. Publishing the private release did not close this gap. |
| Commercial delivery | AUD-05 exact website patch prepared; application/deployment not proven | Anonymous hosted download and actual buyer fulfilment are the remaining sales gates. |
| Native loop / HUD | AUD-09/AUD-11 source and receipts exist; focus, navigation, notebook references and canonical writes are implemented | Qualify the existing journey. Do not rebuild the same navigation or task loop. |
| Compact native MCP | `f994813c`, opt-in `PHYSIS_MCP_COMPACT=1`; structured payload unchanged | Measure what the host actually sends to the model before claiming token savings. |
| Primitive programs | `PrimitiveInvocation.args` still `BTreeMap<String,String>`; textbox pipeline calls `answer_traced` separately | Editable/rendered programs are not yet a general executable dataflow system. |
| Learning lifecycle | `CompiledRule`, `Quarantine`, replay counters and eligibility filter exist; checked call sites for `record_replay` are tests; runtime comment names nonexistent `record_compiled_replay` | Scaffolding is substantial; production confirmation/persistence loop must be wired and checked before learning claims. |
| Typed model choice | `GenerationContext`, `GuidedAction::validate`, `resolve_exact`, `GuidedModel` and deterministic `MockGuidedModel` exist in `physis-reasoning::guided`; no calls found in runtime/HUD/CLI/MCP source | Useful existing seam, not a working small-model integration. V17 must prove the actual routed choice and resulting action. |
| Notebook identity / persistence | `Cell::Note { text }` has no independent stable ID; `Cell::object_id()` returns `None` for notes; notebook JSON saves atomically outside `Runtime::write_object`, which accepts only `file:` IDs | Notes are not yet ordinary addressable world objects. V19 must preserve native notebook content and derive world objects through one canonical runtime adapter. |
| Notebook intent claim | `answer_traced`'s `WriteNote` branch returns `notebook:note-added`; source searches find no persistence call on that branch, while its test asserts the string | Routing prose is not an executed notebook write. V19 must refuse a completion claim without persisted bytes, reload and evidence; the existing HUD `:nb` path is separate. |
| HUD responsiveness | `Hud::run` invokes auto-observation and key dispatch synchronously; `/agent` calls `agent_launch` → `physis_agent::delegate` synchronously | A 60-second delegate bound does not imply responsive input or cancellation. V16b must move long work outside the input/render path. |
| Process execution | Agent runner waits before reading stdout/stderr, clips only after unbounded reads and kills direct child; task runner drains concurrently/bounds retained output, but also kills only direct child; neither inspected runner has cancellation/process-group cleanup | Reuse the better runner in their common dependency, then prove bounded output, root ownership and descendant cancellation. Do not claim current task-tree cancellation works. |
| Customer traction | User confirms no verified paid customers | First-sale proof precedes team subscriptions, enterprise controls and acquisition scaling. |

Qualified private-release archive hashes, from AUD-14 and verified GitHub metadata:

```text
physis-1.2.0-linux-x86_64.tar.gz
fd442aab86edc9893f9afe69d74d5ef994c0f9303f39fea414cda99a3e1653e6

physis-context-1.2.0-linux-x86_64.tar.gz
960d344a893b97989248004419f6083b2bda57319354c4737224874c4da818fd

public Computer's older physis-1.2.0-linux-x86_64.tar.gz
f167239a683496d82a6d0ff33ec8cc8150872872fda4c313915c285d1eb04a52
```

AUD-14 reports downloaded-byte acceptance 15/15 and value checks 10/10 using an
existing production-signed operator gift. This planning review inspected the
receipt and release identity; it did not rerun those executable gates. Existing
qualified commercial artifact is no longer merely a stale 1.1.1 build. Buyer
delivery, payment and retention still have no completed proof.

The old same-task host result is a warning: Physis returned 2,907 tool bytes
against 8,619, but consumed 13,000 provider-reported tokens against 6,278 and took
21.302s against 18.448s. The current five-task outcome pilot finished 2/5 with
Physis versus 1/5 control; its shared successful task took 741.8s versus 263.5s.
Neither establishes a general speed or token-saving claim. Sources:
[value contract](../commercial/VALUE_PROPOSITION.md),
[pilot report](../../research/product/task-outcome-report-2026-10-07.md).
Counts of tests added by each agent are not independent acceptance quality.

Reachability checks were also exercised against this source. `physis-check calls`
over broad `crates`/`apps` roots counted `.physis-next/observations.jsonl` as live
uses and returned false PASS for `GuidedModel` and `record_replay`. Repeating over
explicit Rust source directories returned zero uses outside their declaring
files; `run_textbox_program` has three actual HUD uses. Generated observations
remain useful evidence, but cannot establish executable reachability. V00 receipts
must state search scope and distinguish source calls from quoted history.

## 2. What “remake the computer” must mean operationally

The durable unit is an object plus observed evidence and revision. Human and agent
see the same identity; each surface presents the evidence differently.

```text
files / packages / host / processes / user notes
  → observe → append-only observations
  → derived world: entities, relations, projections, indexes
  → query / impact / bounded context
  → persistent focus + graph / source / notebook / native MCP / CLI
  → explicit proposed action or isolated task
  → canonical write / registered delegate / executed checks
  → re-observe → structural delta + revision-bound outcome
  → inspectable history / memory → next intent
```

Three closures matter more than more tabs:

1. **Identity closure:** search, graph, source, notebook and agent packet preserve
   root + canonical ID + revision. A missing object renders missing.
2. **Action closure:** proposed operation names its target and scope; execution
   returns actual writes/process status; re-observation updates the same world.
3. **Evidence closure:** “done” requires declared checks and desired-state evidence
   at the resulting revision. An agent's prose is retained as a claim.

### Universal representation machine: philosophical contract

Physis makes a thing, its representations and its available transformations
continuous for the human and agent. Meaning is expressed through identity,
relations, constraints, provenance and operations; representation changes do
not silently change the represented thing. The graph organizes navigation and
composition while native files, notes, processes and devices keep their semantics.

Universality means the interface can admit new domains through registered
observers/capabilities. It does not mean a single format completely describes
every domain, every transformation is possible, or observed structure establishes
causality. Preserve units, time, identity, authority, uncertainty and domain rules.
Lossy projections name their omissions. Historical reconstruction, prediction,
simulation and real execution remain distinguishable.

“One world; zero parallel truths” means one route from authoritative domain state
to observed evidence and derived views. A notebook JSON file can remain its native
content authority; its graph projection and observation history do not become
independently writable notebook replicas. Remembered prose remains a claim.
View preferences, operation definitions and reusable procedures should themselves
be inspectable; an explicit correction can change them without erasing history.

| Fundamental objective | Realizable foundation today | Missing proof / boundary |
|---|---|---|
| Addressable | canonical `EntityId`, observations, generic HUD renderer | Stable notebook cell identity and world ingestion; an unimplemented adapter remains unavailable |
| Composable | `CogPrimitive` ports, programs, capability registry, guided constraints | V09/V10 real bindings; V18 semantic applicability; types alone do not prove meaningful composition |
| Inspectable | provenance, graph relations, focus/history, traces, context receipts | Show available operations and actual executed definition; opaque delegate internals stay opaque |
| Consequences visible | canonical file writes, task outcomes, re-observation/deltas | V19 notebook writes, V12 desired-state proof, V16 cancellation and partial-effect receipts |
| Improves through use | compiled rules/quarantine, history, primitive overlays | V14 checked production confirmation, V13 reuse and measured transfer; replay hits are not learning proof |

The twelve verbs in the other mission are a vocabulary to expose where an
implementation and verification method exist, not twelve new mandatory APIs.
Reuse `WorldView`/`WorldWriter`/`WorldDiffer`, primitives, tasks and execution
routes. Unsupported simulate/replay/transform operations carry an explicit gap.
Do not add an all-object enum, an application-pair dispatch table or a second
universal protocol alongside the runtime facade.

### Two horizons and the next vertical slice

**North star:** idea → object → relationships → operation → consequences → learned
capability, within the same world, functional with all models disconnected.
**Next deliverable:** a note has stable identity; select two notes, discover
applicable operations, inspect shared explicit references/differences, preview
and persist one authorized reference/edit, verify it, and return to the same
note/history. Same-note selection deduplicates identity and exposes history;
self-comparison uses explicit revision operands. No generated synthesis required.

LEFT shows origins; CENTER shows focus; RIGHT shows possible transformations;
BOTTOM accepts intention. These are semantic directions, adapting to available
space. Preserve the existing shell, focus/history and useful lenses. Multi-object
selection is an operand set alongside one focus, not another navigation system.
Switching lenses preserves identity/selection; it never performs an action.

Generic composition is `selected objects × context → applicable registered
operations`. Validate root/revisions, operand roles, units, cardinality,
preconditions, effects, authority and available checks. Exact text differences,
explicit references, supported typed contradictions and proposed associations
are separate results. Notebook-wide pins cannot be attributed to every note.

| Mission area | Existing owner / plan route | Claim boundary |
|---|---|---|
| Universal objects, protocol, composition | runtime facade, primitive contracts; V18/V19/V09/V10 | New kinds need an adapter and verified affordances; no automatic universal semantics |
| Spatial HUD, direct manipulation, lenses, textbox | existing HUD; V06/V20/V16/V17 | Deterministic keyboard slice first; core operation requires no model |
| Living/executable notebook, note × note | existing notebook JSON; V19/V20; V12 for actions | Stable note IDs, explicit references, byte-level persistence and evidence closure |
| Time, counterfactuals, desired state | existing history/diff/task/solve/exec; V12/V21 | Reconstruction is labelled; impact is not causality; simulation requires a real model |
| Structural/procedural/operational/interaction learning | V14/V13/V15/V07; existing config and observations | Separate learned rule, measured strategy and explicit preference; repetition alone grants none |
| Full working environment | V06/V16/V20/V21 plus existing editor/host/task modules | Preserve external editor/terminal routes; HUD raw shell is currently disabled, not a full terminal |
| Remote/physical/communications adapters | current host/exec primitives, later named adapter packets | Explicit authorization, freshness/offline semantics and real checks before control claims |
| Zenith controller | §10 mechanisms inside Physis | Earlier operator clarification still applies: no second controller/world required |
| Safety, demonstrations and measurement | §7 packet loops, §8 gates, A–E matrix below | Actual bytes/outcomes and interactive proof; missing evidence stays UNVERIFIABLE |

External `physis-pro` remains retired. A `physis-core` organ is relevant only
through this workspace's actual linked dependency/feature boundary; do not import
a separate project's roadmap as the current machine. Source/call paths outrank
`PLAN.md` or phase-log claims that contradict later source or receipts.

For several hours of real use, the HUD must let a developer recover orientation,
edit safely, hand off a task, inspect a process/check, cancel it, compare changes
and return to their note without losing focus. Use existing modules below; repair
only failed steps found in V06. Keep an external editor/terminal escape reachable
while the internal path earns trust. Shell output is evidence, not a second world.

| Capability | Existing seam | Acceptance / remaining proof | Priority |
|---|---|---|---|
| Spatial graph and persistent focus | HUD `focus.rs`, `object.rs`, `layout.rs`, `app.rs` | Graph → source → context → back/forward preserves canonical ID; no focus reset on lens switch | V06, high |
| Object inspection / browsing | HUD `object.rs`, `views.rs`; query snapshot | Unknown kind renders identity/provenance; deleted target renders missing | V06, high |
| Edit and diff | `physis-editor::Buffer`, `Runtime::write_object` | Preview → save → observed delta; concurrent edit causes conflict or explicit reload, never silent overwrite | V06/V12, extreme |
| Notebook | `physis-notebook::Cell::Ref { observed }`, `Goal` | Pin, reopen, stale indicator, explicit refresh; pinned fingerprint survives restart | V06, high |
| Agent / task / checks | `physis-task`, registry, HUD task/shell views | Isolated task remains separate until reviewed; actual check exits/revisions visible; cancel terminates the launched task tree | V06/V12, high |
| Settings / search / textbox | HUD `textbox.rs`, `event.rs`, existing config | One visible focus, searchable actions, contextual keys; unsupported intent returns an honest available route | V04/V06, high |
| Host world | runtime host observation and existing views | One source-to-process investigation with explicit freshness/permissions; no claim that all host effects are observed | After V06, medium |
| Executable editable programs | cognition `Program`, runtime primitives | Same program rendered, validated and executed; step output binds next input | V09/V10, core lane |
| Deterministic composition | registry, typed ports, program IR | Bound target/predicate goal; bounded search; no guessed effects or arbitrary shell synthesis | V11, conditional |
| Primitive learning / confirmation | existing traces, `CompiledRule`, quarantine, primitive overlays | Proposed behavior → distinct checks → held-out confirmation → visible confidence/evidence → explicit promotion; counterexample withdraws eligibility | V14a/V14c/V14b, core lane |
| Behavioral learning / optional CA | same evidence and primitive contracts | One scoped behavioral model wins on held-out probes without replacing observation store | V15, bounded core experiment |
| Responsiveness / cancellation | HUD input loop; existing agent/task runners | Navigation stays responsive during owned background work; cancellation leaves an honest receipt and no owned descendants | V16a/V16b, core lane |
| Minimum viable language model | reasoning `ConstraintCompiler`, `GenerationContext`, `GuidedModel`; registry | Deterministic route first; finite validated choices; cheapest available model must pass frozen interpretation/execution cases | V17, after execution closure |

Keep every useful lens available. Make modules context-sensitive destinations,
not twenty competing top-level starting points. A new HUD shell or graph store
is not required for this acceptance.

### General computer: intent, programs, learning and attention

Latest operator direction makes the general computer the primary objective.
Commercial readiness funds and tests a useful subset; it does not determine the
engine's scope or postpone learning until a sale. The development target is a
machine that can acquire new capabilities through the same objects, contracts
and execution path, without a separate agent product for every domain.

```text
user intent + current focus + preferences + existing authority
  → exact deterministic route, or bounded model interpretation
  → explicit goal: target, desired state, constraints and check obligations
  → registered primitive / inspected program / scoped learned procedure
  → background execution with visible progress and cancellation
  → actual observations + checks + desired-state verdict
  → same world, focus, notebook and history
  → candidate generalization → experiments → scoped reuse or counterexample
```

The small model may resolve language, ask a useful question, rank permitted
choices and propose a candidate rule. It cannot supply missing observations,
assert success, grant authority or manufacture a capability. Exact commands and
eligible learned procedures should remain usable with the model unavailable.
Start from `guided.rs`'s existing finite choices and validation, not a second
natural-language router. “Smallest” means minimum actual resource/cost use among
models passing V17; parameter count or a free catalogue entry proves nothing.

Keep four meanings distinct in existing observations/contracts: **observed
fact**, **predictive hypothesis**, **user preference/goal**, and **authority**.
The user's will chooses desired outcomes and permitted tradeoffs. It does not
change a measurement or make an impossible predicate true. An infeasible goal
returns the missing capability, constraint conflict or unknown input, with an
inspectable trace. No new ontology service or separate preference database.

For “discover rules of nature,” begin with observable rules of the machine and
then admit real sensor/simulator measurements through registered observers.
V15 is the first behavioral-learning experiment; its filesystem/MCP fixture is
explicitly computer behavior, not a discovery of physics. A later physical rule
requires measured quantities/units, clock or sensor calibration, intervention
and reset assumptions, noise/error bounds, independent checks, held-out probes
and counterexamples. Simulated data stays labelled simulated. A causal claim
requires evidence that distinguishes relevant competing explanations; passive
correlation and repeated success alone do not establish it. Observable models
can be useful without recovering unique internals; see the existing
[active model-learning reference](https://www.sws.cs.ru.nl/publications/papers/fvaan/lncs10k.pdf).

Learning must produce a visible object: implementation/program, scope, inputs,
expected effects, checks, supporting observations, distinct cases, repeats,
unknowns, counterexamples, revision and measured costs. Deterministic decisions,
model proposals and failures all get inspectable provenance. Surface engine errors
instead of silently replacing them with an unexplained choice score. Confidence
expresses its declared evidence meaning, never an all-domain reliability promise.

“Capable of doing anything” is the horizon of an extensible computer, not a
present acceptance claim. A finite observer/action set cannot execute unavailable
capabilities or infer unobserved facts. Expansion follows a concrete loop:
named user goal → explicit capability gap → existing registered mechanism or
bounded new adapter/primitive → checked execution → same-world integration.
Preserve ordinary editor/terminal access while these routes earn coverage.

### Responsiveness is part of correctness

Proposed targets on a recorded reference machine and fixture, not measured facts:

- Input-to-visible navigation feedback p95 ≤100ms; initial action acknowledgement
  ≤250ms. Record load, terminal and fixture; retain missed targets.
- A 30-second check/delegate or re-observation never blocks focus, typing, history
  or cancellation. One long-running operation is sufficient for the first proof.
- Cancellation acknowledgement ≤250ms, owned process-group cleanup ≤2s; inability
  to prove cleanup is UNVERIFIABLE. Cancellation is not undo; completed writes
  remain visible and require ordinary re-observation/rollback.
- Keep the last coherent world visible with a freshness badge during work.
  Apply background results only when root, request and revision still match;
  later work cannot overwrite a newer focus, edit or notebook pin.
- Store evidence before retiring a task. A restart can show its last status and
  reconcile UNKNOWN with actual process/filesystem state. Restart recovery is not
  permission to repeat an action, nor a claim that arbitrary process resume exists.

Use HUD state plus stdlib channels/threads and the existing bounded executor.
Do not add a job service, event bus or orchestration database. V16a/V16b make one
owned task and observation path correct before introducing parallel writers.

First complete Computer proof: ask about a focused object → inspect the exact
lookup/callers/context program → execute it → pin evidence → teach one scoped
procedure → verify varied and held-out cases → reuse with no model call → change
its relevant input → expose the stale assumption/counterexample → revise and
recheck. Run one slow check during the journey and keep navigating/cancelling.
The proof may begin as explicit commands; natural phrasing joins only after V17.
This demonstrates a learning computer without pretending universal competence.

## 3. Surprise within 60 seconds

Working promise: **“Change a file. Watch its world change.”**

Use the existing `timeout-client` fixture and qualified demo. Show a real loop,
then let the viewer try their own supported repository. Exact counts are derived
from the installed executable; never hard-code a polished transcript as proof.

| Time | Visible event | Evidence |
|---|---|---|
| 0–8s | Focus request symbol; ask who calls it | Exact ID, source, root and freshness |
| 8–20s | Follow one caller; pin the target in notebook; show agent context | Same target and bounded source packet; a caller opens to its actual code |
| 20–34s | Preview and apply the existing scripted `timeout` → `deadline` edit | Label scripted edit; actual source diff and canonical-write receipt |
| 34–48s | World changes; pin becomes stale; inspect delta and executed test | Real relation/source changes; process exit; structural check separately labelled |
| 48–60s | Return through history; reopen evidence; invite local reproduction | Same world/revision chain; one explicit next command |

If the existing demo does not expose a notebook step, show it in HUD with the same
fixture; do not add a separate demonstration-only notebook. If this exceeds 60s,
cut presentation, not checks. Initial cut gets at most four engineering hours.

Second demonstration: **“Teach the computer a small rule; try to prove it wrong.”**
Propose a small registered primitive/rule, inspect its checks and held-out cases,
replay a duplicate without increasing confidence, then show a counterexample
withdrawing eligibility. Start with current graph primitives; pure compute joins
after V03. V14a/V14b make the learning real and inspectable. Do not present a manual
edit or seeded replay counter as autonomous learning.

No hired production, paid advertising or telemetry SDK. Consented viewer notes
and public reproducible fixtures suffice. The free binary remains independently
useful: local structural exploration, bounded context, knowledge/quiz and HUD.
The learning demonstration becomes the next Computer proof after V14 closure:
teach once → inspect its rule/checks → reuse without model calls → change the
conditions → watch abstention/counterexample → revise and recheck. Record actual
inference/check costs and keep navigation usable throughout. This is a proposed
demonstration, not present executable evidence.

## 4. Economics: prove a painful outcome, then repeat it

Offer hypotheses, ranked by present evidence and founder effort:

| Rank | Opportunity | Why now / stop condition | Value category |
|---|---|---|---|
| 1 | Supported Context MCP licence on one real change-impact investigation | Existing small product/artifact; ask for payment after a useful result. Stop expanding features when buyers cannot name a recurring pain. | Near-term revenue |
| 2 | €750 scoped codebase investigation, at most five delivery hours | Test willingness to pay for evidence and expert help before software demand is clear. Do not deliver unpaid custom engineering or sell through the individual-licence SKU. | Revenue + learning |
| 3 | Reproducible free Computer demonstration and local trial | Distribution/credibility already mostly built. Viewer trial and real task matter more than views/stars. | Adoption + credibility |
| 4 | Measured agent context/impact workflow | Can become a moat if it improves accepted work on external repositories. Smaller packets alone do not establish value. | Retention + long-term moat |
| 5 | Privacy/local enterprise, research, sponsorship and portfolio | Preserve evidence; pursue when a named buyer or sponsor requests it. No enterprise feature programme funded by imagined demand. | Conditional credibility / moat |

Strongest paid workflow: resolve an exact symbol → see observed callers and their
source → compile a bounded change packet → expose unresolved cases → inspect the
post-change structural delta. Secondary workflows: unfamiliar Rust workspace
orientation; exact impact excluding string/prefix decoys; resuming with revision-
bound evidence. Language coverage claims must match paid executable tests.

Free native MCP overlaps paid capabilities. Disclose that honestly. Paid value
must be the maintained small integration, reliable onboarding and supported
outcome, not artificial deprivation of the free Computer. If qualified users
prefer free native and decline support, record that as a failed licence hypothesis.

Service scope: one repository/revision, one agreed investigation, written evidence
and coverage limits, a reproducible check packet, one 30-minute walkthrough;
maximum five delivery hours. No unlimited implementation, guaranteed safety or
unbounded support. Price, tax treatment, payment and refund terms need a separately
reviewed offer before money changes hands.

Illustrative **gross receipts**, not forecasts or profit:

| Scenario | Gross | Practical constraint |
|---|---:|---|
| 10 founding licences | €490 | Small support burden essential |
| 10 founding licences + 3 scoped investigations | €2,740 | Up to 15 delivery hours plus sales, support and costs |
| 25 founding licences + 3 investigations | €3,475 | Founder discount exhausted; demand still unproven |
| 1,000 standard licences | €79,000 one-time | Needs repeatable acquisition/delivery; creates future support obligations |

Track collected cash minus refunds, payment/tax obligations and incremental costs
separately from economic contribution after founder time. €750/5 = €150 per
delivery hour before acquisition, support, costs and taxes. A finite service can
fund proof; wealth would require distribution and repeated product demand.

Customer sequence: qualified conversation → real supported task → verified useful
result → voluntary paid offer → confirmed payment → delivered valid licence →
day-7 repeat use. Payment without activation is not product validation. Three
paying users who repeat an investigation are the next decision milestone, not a
promise that revenue arrives within a week.

## 5. Execution order and budgets

```text
V00 pin reality
 ├─ V01 public Computer delivery ─ V04 demo ─ V05 viewers ─┐
 ├─ V02 commercial website/delivery ──────────────────────┼─ V08 buyers/retention
 ├─ V03 compute branch qualification (only compute needs it)
 ├─ V06 existing HUD journey ─ V09 bindings ─ V10 executor ─┬─ V11 planner
 ├─ V07 cost diagnostic                                  ├─ V12 action proof
 └─ V16a owned process execution ─ V16b responsive HUD    └─ V17 bounded language
V00 ─ V14a behavioral checks ─┐
V00 ─ V14c evidence/score ────┴─ V14b persisted confirmation ─ V15 black-box probe
V10 + V14b ─ V13 confirmed program reuse
V16b needs V06's pinned HUD fixture; V17 needs V07 accounting and V06 focus proof.
V00 ─ V18 shared operation contract ──────────┐
V00 ─ V19 addressable notebook / persistence ├─ V20 notebook composition slice
V06 ────────────────────────────────────────┘
V18 + V20 ─ V21 code/host parity and A–E demonstration qualification
```

Arrows express dependencies, not permission to spawn workers. Sales and the first
Computer demo do not depend on V09–V17. Large symbolic-composition work is not a
release prerequisite. V11 and V12 are separate: explicit execution needs no planner.
Existing primitives make the first two-step program possible without the compute
branch. V15's fixture/experiment design can start at V00; promotion waits for V14b.
V18–V21 are the parallel Impossible Machine lane's common acceptance map, not a
second implementation queue. Match its actual work to these contracts first;
reproduce a remaining gap before assigning a duplicate patch. V09/V10 remain
the program-composition owners; V18 describes/discovers their operations.

### NOW — next two hours, once execution is assigned

1. V00: 20min capture and review exact source/artifact state.
2. V18/V19: 60min seal notebook identity/write and shared-operation packets;
   reconcile the parallel lane's actual diff first. Source-write scope empty for
   these initial INVESTIGATE/DESIGN packets. V09/V10 remain the owners when a
   composed program is required; do not assign a second interpreter.
3. V14a: 40min seal/reproduce the wrong-result-with-correct-prose counterexample.
   This is the first learning correctness slice, not another learning framework.

Those are timeboxes, not claims full builds/deployment finish in two hours. Stop a
slow gate at its declared timeout, retain output and resume it deliberately.

### NEXT — next working day

- Finish V06's pinned journey, then one reviewed V09/V10 read-only execution slice.
- Complete V14a's exact behavioral check, with failure and negative-control proof.
- Start V16a only after reproducing the output/deadline/descendant problem in an
  isolated fixture. Do not hide a blocking runner inside a background thread.
- Reserve the delivery slot for V01/V02 independent-install gates and the existing
  V04 demonstration; release only the reviewed candidate. This is useful proof
  while the learning demonstration is still being developed.
- V07: three paired diagnostic tasks; record full costs and unchanged correctness.
- Prepare V05/V08 target list and honest offer. Do not promise every card in one day.

### THEN — next seven days

Latest operator steering supersedes the earlier 12h/20h/8h split. Allocate
**20h engineering, 12h customer discovery/delivery, 8h support/evidence**:

| Engineering lane | Hours | First deliverable |
|---|---:|---|
| Existing delivery/demo | 4 | V01/V02 reviewed handoffs and truthful existing V04 run; unresolved external deployment stays explicit |
| Executable, responsive computer | 8 | V06 proof, first V09/V10 slice, first reproduced V16a/V16b fix; choose by failed acceptance |
| Primitive/behavioral learning | 4 | At least 2h V14a; remaining time V14c or V15 fixture/design, not product claims |
| Small-model/cost qualification | 4 | V07 accounting baseline and frozen V17 cases; no provider integration before execution is real |

These total 40h including customer work and support/evidence. Timeboxes fund
progress; they do not promise all cards finish in a week. Partial work ends with
a concrete receipt and ready next slice. Ten buyer demos remain the experiment
target, not a guarantee within 12h; five independent Computer viewers can overlap
the same consented sessions without counting gifts as sales. Cut polish before
the core learning slot. Inaccessible customers leave an acquisition finding.
Within the same 8h computer slot, prefer the first V18/V19/V20 notebook continuity
gap if the parallel lane has not already closed it. It replaces another optional
slice; it does not add hours. Do not promise program execution, notebook migration
and background cancellation all finish in this allocation.

### LATER — conditionally funded

Continue the funded core sequence rather than moving it behind a sale: finish
V09/V10, V16a/V16b, V14c/V14b, then V17 and V13 reusable programs. V11 starts at a
concrete unsolved composition; V12 starts when explicit actions need composed
checks. V15 receives one two-hour behavioral-learning probe plus its recorded
counterexamples; promote only through V14b. Broader physical models follow a named
experiment with actual measurements and a baseline. Team/enterprise pricing,
broader platforms, large-repository studies and publications require their own
evidence gate. Preserve research; postpone unsupported product claims.

## 6. Executable work cards

All source cards use: reproduce → smallest patch → focused check → runtime/wire
check when relevant → receipt → review. Documentation/qualification cards need no
new mirrored unit tests. Hours are upper planning ranges; long cards split into
two-hour worker assignments before dispatch.

### V00 — seal present reality

**Goal:** one reliable baseline for every later worker. **Dependencies:** none.
**Difficulty:** XS, 20–40min. **Leverage:** extreme.
**Allowed outputs:** `tasks/handoffs/V00/EVIDENCE.md` (new),
`tasks/handoffs/V00/RECEIPT.json` (new). Read refs/worktrees and existing receipts.

- [ ] Capture branch, HEAD, status, live GitHub main, worktree/branch inventory,
  tag targets and exact artifact hashes. Record private/public host distinction.
- [ ] Classify existing AUD-04/14 proofs as artifact qualification; AUD-05/12 as
  delivery/buyer evidence still needing closure. Preserve historical results.
- [ ] Seal source and write allowlist before any worker patch. Acceptance: a
  reviewer reconstructs each candidate without using whichever binary is on PATH.
- [ ] Reachability scans use explicit source directories and inspect real callers;
  generated observations, receipts and documentation cannot count as executable
  calls. Preserve the broad-scan false PASS as a tooling finding, not product proof.

### V01 — close public Computer distribution

**Goal:** anonymous users receive truthful, qualified Computer bytes.
**Dependencies:** V00. **Difficulty:** S, 1–3h. **Leverage:** extreme.
**Likely files:** `scripts/stage-physis-computer.sh`, `scripts/open-release.sh`,
`physis-computer/README.md`, `physis-computer/docs/CHANGELOG.md`,
`tasks/handoffs/V01/RECEIPT.md` (new). Installer/TOUR repairs, if required, get a
separate bounded follow-up card; existing installer fixes are not reimplemented.

- [ ] Compare only the explicit staging allowlist/corpus to public repository.
  Verify stager resolves the release's actual `CARGO_TARGET_DIR`; it currently
  names the default path while open-release permits an override.
- [ ] Choose concrete release identity: preferred immutable next version for a
  replacement payload, with shared version guard updated through its existing
  workflow. Do not silently replace public v1.2.0 assets. Alternative: preserve
  old release and publish qualified 1.2.0 bytes through a separately named,
  documented handoff pending a new version; do not call that an updated v1.2.0.
- [ ] Qualify public candidate before publication: no engine/research/commercial
  source; staged CLI/HUD hashes equal corresponding archive executables; required
  corpus and checksums present. Missing qualified binaries fail closed. After
  authorized publication, anonymous download → outer/inner hashes → install →
  version → demo from outside checkout. Missing checksum/corrupt file refuses.

**Gate:** existing open release, staging leak gate, `just versions`, demo e2e and
installer controls. Checksum detects corruption; without a delivered verified
signature do not claim signed provenance. Record actual commands/hash receipts.

### V02 — finish commercial delivery seam

**Goal:** customer-facing version/URL/hash/install instructions deliver the exact
qualified commercial artifact. **Dependencies:** V00 and existing AUD-14 bytes.
**Difficulty:** S, 1–2h plus deployment availability. **Leverage:** extreme.
**Source handoff:** `tasks/handoffs/AUD-05/SEAM-DIFF.md` (read-only).
**External allowlist, separately authorized checkout:**
`/home/gio/praxis_webapp/client/src/features/physis/PhysisLandingPage.tsx`,
`/home/gio/praxis_webapp/client/src/features/physis/PhysisDeliveredPage.tsx`,
`/home/gio/praxis_webapp/client/src/features/physis/PhysisLandingPage.test.tsx`.
**Receipt:** `tasks/handoffs/V02/RECEIPT.md` (new).

- [ ] Apply the prepared three-file seam; mirror qualified archive/checksums to
  the existing `praxisweb.xyz/downloads/` delivery route through authorized deploy.
  Do not substitute a private GitHub URL on customer pages.
- [ ] Run the existing landing-page assertion and sales/entitlement tests. Probe
  anonymously from a fresh environment; 200 alone is insufficient: hash the body,
  install, verify version and an existing consented entitlement, run exact-byte
  supplied acceptance/value/client checks without rebuilding the executable.
- [ ] Acceptance: both pages agree with delivered version/hash; invalid licence
  fails; existing valid licence survives; missing hosted file blocks the offer.
  Paid checkout → webhook → issuance → delivery remains V08's separate proof.

### V03 — qualify compute branch without expanding scope

**Goal:** decide whether three std-only primitives merit main integration.
**Dependencies:** V00. **Difficulty:** S, 1–2h. **Leverage:** medium.
**Review files:** `crates/physis-cognition/src/compute.rs`,
`crates/physis-cognition/src/cogprim.rs`, `crates/physis-cognition/src/program.rs`,
`crates/physis-cognition/src/lib.rs`; receipt `tasks/handoffs/V03/RECEIPT.md` (new).
These paths refer to `23f3fd4a`; `compute.rs` does not exist on current main.
Inspect that pinned tree or isolated candidate, not a racing shared checkout.

- [ ] Review `23f3fd4a` against released source: precedence, right-associative
  powers, non-finite/domain behavior, affine temperatures, invalid dates/leap days,
  input bounds and attribution. Preserve branch until review passes.
- [ ] Run `cargo test -p physis-cognition`, runtime/MCP affected tests, clippy/fmt
  and primitive CLI probes. Session-reported passes are baseline evidence, not
  current candidate proof. Observe whether textbox executes these primitives;
  primitive CLI reachability does not imply textbox reachability.
- [ ] Acceptance: real outputs and failures documented; no CAS/currency/physics
  promise. Integrate on an isolated candidate after review, without rewriting the
  released v1.2.0 identity. Current broad composer remains V09/V10.

### V04 — make one honest, reproducible 60-second demonstration

**Goal:** present the same-object loop without narration-heavy setup.
**Dependencies:** V00; V01 for public reproduction. **Difficulty:** S, ≤4h total.
**Leverage:** high. **Likely files:** `physis-computer/README.md`,
`physis-computer/docs/TOUR.md`, `apps/physis-cli/src/demo.rs`,
`apps/physis-cli/tests/demo_e2e.rs`, `tasks/handoffs/V04/RECEIPT.md` (new).

- [ ] Run current demo from extracted qualified bytes. Record actual counts,
  test availability, elapsed times and source IDs. First prepare transcript/video
  using existing mechanisms; patch demo only for a reproduced blocker.
- [ ] Present source → callers/context → scripted canonical edit → observed
  delta → executed check → history. Graph-preserving wrong result must fail;
  missing Cargo reports UNVERIFIABLE and never aggregate PASS.
- [ ] Acceptance: fresh Linux user needs one documented install path and one
  starting command; 60-second presentation has inspectable output, fixture path
  and actual backend labels. Installation time is reported separately, not edited
  out of a “first useful result within 60 seconds” claim.

**Gate:** `just demo-e2e`, extracted-binary run and existing negative controls.

### V05 — five-viewer shock test

**Goal:** distinguish surprise from confusion and applause. **Dependencies:**
V04/V01. **Difficulty:** S, 2h founder time per round. **Leverage:** high.
**Files:** `tasks/handoffs/V05/VIEWERS.md`, `tasks/handoffs/V05/RECEIPT.md` (new).

- [ ] With authorized, consented contact, show one cut to five technically capable
  target users. Ask them to describe what changed before explaining the mechanism.
- [ ] Record denominators: understood, independently installed, first useful
  result, attempted real repo task, supported platform/language, confusion/failure.
- [ ] Decision thresholds: 4/5 understand shared-object change, 3/5 independently
  install, 2/5 attempt their own task. These are small-sample engineering triggers,
  not market conversion estimates. At most two rounds; fix one observed obstacle
  between rounds. If no real task follows, change problem/story before adding UI.

### V06 — prove the current HUD can sustain a work session

**Goal:** validate existing loop, select one actual friction fix. **Dependencies:**
V00, qualified HUD and the existing `timeout-client` fixture; public demo publication
is not required. **Difficulty:** S, 1–2h evidence, follow-up fix
separately scoped. **Leverage:** high.
**Likely files if a reproduced fix is needed:** `apps/physis-hud/src/event.rs`,
`apps/physis-hud/src/app.rs`, `apps/physis-hud/src/tests/document_work.rs`,
`apps/physis-hud/src/tests/shell.rs`, `tasks/handoffs/V06/RECEIPT.md` (new).
Read focus/object/layout/editor/notebook modules; do not automatically rewrite them.

- [ ] Fresh disposable observed repo, 120×36 and 80×24: search → focus → graph →
  source → pin → back/forward → edit preview → save → check → delta → reopen pin.
  Repeat with deleted target, out-of-band edit and failed process/check.
- [ ] Run a real 20-minute keyboard session, then one several-hour session when
  adoption justifies it. Record orientation recovery, key discovery, resize,
  unsaved edits, process cancellation and history; snapshots cannot prove key feel.
- [ ] Acceptance: same ID/root across lenses and agent context; stale pin visible;
  unchanged notes survive restart; exact exit/check revision shown; no false done
  state. A gap creates one named ≤5-file follow-up card, not a generic redesign.

**Gate:** `cargo test -p physis-hud`, `just snapshot-smoke`, affected editor/notebook
tests and real terminal journey. Construct/observe fixture explicitly; failing
unobserved fresh-worktree tests must be attributed to setup before product claims.

### V07 — measure full agent cost, then change one cause

**Goal:** choose an efficiency change from actual host costs at equal correctness.
**Dependencies:** V00; qualified commercial/native executable as applicable.
**Difficulty:** M, 2–4h diagnostic; existing larger rubric separately budgeted.
**Leverage:** high if a real bottleneck exists.
**Allowed files:** `scripts/commercial-agent-host-check.py`,
`scripts/mcp-surface-cost.sh`, `tasks/handoffs/V07/MEASUREMENT.md`,
`tasks/handoffs/V07/RECEIPT.json` (new). Read native compact/protocol and the
existing M4 rubric; source optimization gets a separate sealed card after diagnosis.

- [ ] Three matched diagnostic tasks with explicit oracle and negative controls:
  exact caller versus prefix/string decoy; freshness after edit/re-observe; context
  and impact across files. Pin model ID actually served, host, tools, source,
  root/store, executable, limits and arm order. Randomize/counterbalance order.
- [ ] Extend existing host receipt only where missing: all provider usage/cost
  events, schemas/history/retry/fallback totals, wire bytes, elapsed time, setup
  and acceptance. A free provider is allowed; reported zero cost is not a
  correctness requirement for a later paid-provider comparison. Do not inspect
  hidden reasoning; separately reported reasoning-token counts suffice.
- [ ] Compare baseline, ordinary MCP, then compact/profile on selected diagnostic
  cases. Retain structured data and text actually seen by the host. If both enter
  model context, count both. Pick one repair: fewer repeated evidence packets,
  smaller default surface, exact-target context, truthful polling or display.

**Acceptance:** missing usage = NOT MEASURED, never zero; timeouts/retries included;
no invented callers or stale-current claims; no critical correctness regression.
Three cases diagnose, not market. Public TokenSave promotion uses existing M4
gate: ≥16/20 main, 8/8 hidden, no category regression, ≥12 paired-PASS; complete
usage, ≥20% median token reduction and non-worse aggregate totals on all pairs
and paired-PASS subset. Do not silently exceed its 56-turn/4h run cap. Separate
accepted-change study is needed for “faster coding” claims.

### V08 — first paid use and day-7 retention

**Goal:** verified payment, delivery and repeated useful work. **Dependencies:**
V02; V04/V05 supply optional inbound; V07 diagnostic informs honest claims.
**Difficulty:** M, repeated ≤2h founder sessions; within the 12h/week customer allocation.
**Leverage:** extreme.
**Files:** `docs/commercial/LISTING.md`, `docs/commercial/FIRST_SALE.md`,
`tasks/handoffs/V08/FUNNEL.md`, `tasks/handoffs/V08/RECEIPT.md` (new).

- [ ] Prepare ten qualified demonstrations: supported environment, recurring
  investigation pain, real repository/task, consent. Do not count a spectator
  as a buyer. Show free native overlap and the maintained commercial alternative.
- [ ] Offer current licence after a useful result; offer the separate scoped
  investigation only if expert help is requested. Record wins, declines, pending
  outcomes, price objections and founder/support time. Outbound contact and
  payment operations require the applicable explicit assignment.
- [ ] Verify existing paid flow on an actual authorized sale: price/product,
  payment confirmation, idempotent webhook, production licence, email/download,
  anonymous correct artifact, install/activation and one useful task. Day 7: one
  repeated task and reason to retain/abandon. Never fabricate sales or issue a
  customer licence just to make the report green.

**Decision:** no purchase after ten qualified demos → reassess buyer/problem/offer
before expanding the commercial offer. Purchases but no repeat use → one demonstrated
activation/retention repair. Three paid repeat users → test repeatable acquisition;
recurring team product only after explicit repeated team demand. This is an
operational experiment, not a statistically established conversion threshold.

### V09 — output bindings with semantic scope

**Goal:** one primitive consumes another's real output safely. **Dependencies:**
V00 and V06 identity/focus proof; V03 only if compute primitives enter the slice.
Exact lookup → callers → context is the first concrete composition need.
**Difficulty:** M, 3–5h; split contract/replay into ≤2h assignments.
**Leverage:** high, core.
**Files:** `crates/physis-cognition/src/program.rs`,
`crates/physis-cognition/src/cogprim.rs`, `crates/physis-cognition/src/lib.rs`,
`crates/physis-runtime/src/primitives.rs`, `tasks/handoffs/V09/RECEIPT.md` (new).

**Proposed minimal interfaces:** extend `PrimitiveInvocation` argument values to
`ArgBinding::Literal(String)` or `ArgBinding::Output { step, port }`; deserialize
old string arguments as literals. Add named typed results without removing legacy
`PrimitiveOutcome.values`. Typed result must distinguish entity ID, list, scalar,
unit and truth status; no universal value algebra beyond the first proven workflow.

- [ ] Baseline-failing case: exact-target lookup output binds callers' target.
  Unknown output port, forward reference, cycle, missing target and cardinality
  mismatch refuse execution. Old saved literal programs load unchanged.
- [ ] Carry repository identity/world revision with object-bearing values; pure
  scalar result has declared units. Different entity or unit cannot satisfy the
  consumer just because both values are strings. Implement in existing IR/results.
- [ ] Acceptance: literal one-step behavior preserved; bound two-step program
  passes; ambiguous lookup abstains instead of choosing first result; edited
  primitive revision invalidates previous binding assumptions.

**Gate:** cognition tests, runtime primitive tests, fmt/clippy; no new planner yet.

### V10 — execute the exact program shown to the user

**Goal:** close render/execute gap with a read-only interpreter. **Dependencies:**
V09. **Difficulty:** M, 4–6h in ≤2h slices. **Leverage:** high, core.
**Files:** `crates/physis-runtime/src/cognition.rs`,
`crates/physis-runtime/src/primitives.rs`, `crates/physis-cognition/src/trace.rs`,
`apps/physis-hud/src/modules/cognition.rs`, `tasks/handoffs/V10/RECEIPT.md` (new).

**Proposed interface:** `Runtime::execute_program(&Program, &ProgramLimits)
-> anyhow::Result<CognitionTrace>`. `ProgramLimits` contains `max_steps: usize`,
`max_results_per_step: usize`, `deadline_ms: u64`, `context_budget: u32`,
`expected_revision: Option<String>` and `read_only: bool`. Extend existing trace
with ordered step outcomes, program/primitive fingerprints and start/end world
revisions. Semantic unresolved outcomes stay explicit in trace; invalid programs
and operational failures return errors. This API is proposed, not present source.

- [ ] First wire explicit read-only program through `Runtime::run_primitive`;
  do not reimplement `CogPrimitive::execute`, GraphView or cascade in HUD. Keep
  existing query behavior via adapter until equivalence tests pass.
- [ ] Route textbox execution to this shared path for registered supported
  invocations. On compute candidate, `calc`, `convert`, `date_diff` return executed
  values, not rendered intent. Unsupported/cascade/action programs retain an
  explicit unresolved/pending state instead of performing a hidden second plan.
- [ ] Acceptance: displayed edited program fingerprint equals executed one;
  modifying a binding changes result; missing primitive, zero budget, step failure,
  cancellation and invalid revision stop. No implicit model calls or writes.

**Limits:** initially ≤8 steps, ≤32 results per step, explicit overall deadline;
source-context token budget uses existing accounting and cannot be equated with
step count. Pure compute must have existing parser/input bounds. Gate runtime,
cognition, affected HUD tests and one real textbox/CLI-equivalence probe.

### V11 — smallest deterministic composition for one unsolved goal

**Goal:** compose only when explicit programs cannot meet a repeated user need.
**Dependencies:** V10 and two recorded failed manual goals of the same supported
kind. **Difficulty:** M, 4–6h. **Leverage:** medium until proven.
**Files:** `crates/physis-cognition/src/program.rs`,
`crates/physis-cognition/src/registry.rs`, `crates/physis-cognition/src/cogprim.rs`,
`crates/physis-runtime/src/cognition.rs`, `tasks/handoffs/V11/RECEIPT.md` (new).

- [ ] Define one structured goal: root/revision, target, desired predicate,
  available inputs and allowed effects. Add only the corresponding explicit
  contracts to existing primitives. A Bool-returning primitive does not prove
  arbitrary goals; callers of A cannot satisfy a question about B.
- [ ] Deterministic bounded backward search: depth ≤4, candidate fanout ≤3,
  expanded nodes ≤32, plan ≤8 steps, cycle detection, lexical stable tie-break;
  order by existing cost class. Cost class is a heuristic, not currency or tokens.
- [ ] Replay candidate through V10 and independently evaluate the goal. Unknown
  effects, unsupported goal, missing evidence and conflicting scope abstain with
  the smallest gap. Acceptance includes cheaper wrong-target producer rejected,
  unsatisfiable goal and budget exhaustion. No arbitrary shell synthesis or model
  inventing a plan. No broad GOAP/STRIPS/logic engine dependency.

### V12 — explicit program actions preserve the canonical loop

**Goal:** composed explicit actions obtain the same evidence as existing tasks.
**Dependencies:** V10 and V06 canonical/task journey proof; V11 not required.
**Difficulty:** M, 4–6h. **Leverage:** high after repeated use.
**Files:** `crates/physis-runtime/src/cognition.rs`,
`crates/physis-runtime/src/lib.rs`, `crates/physis-task/src/lib.rs`,
`crates/physis-mcp/tests/e2e.rs`, `tasks/handoffs/V12/RECEIPT.md` (new).

- [ ] One explicit supported action calls existing canonical write or registered
  isolated task. Seal target/files/checks/revision and existing authority before
  execution; retain task proposal separately from an applied main-tree change.
- [ ] Run declared checks on changed bytes, re-observe, inspect structural delta
  and desired-state predicate, then record outcome at matching revision. External
  writes during checks invalidate success until rechecked.
- [ ] Acceptance: no declaration/check = UNKNOWN or unverified, process exit0
  alone cannot establish goal; failed check stays failed; delegate “done” prose
  cannot override actual filesystem/check evidence; cancelled task cannot merge.
  Main-tree merge/write follows existing concrete review/authorization path.

**Gate:** task/runtime tests, native e2e, `scripts/demo-native-loop.sh` and affected
invariants. Do not add native action features to the commercial tool catalogue.

### V13 — reuse one proven program without stale truth

**Goal:** repeated successful work reuses structure and refreshes evidence.
**Dependencies:** V10/V14b, and ≥3 successful distinct confirmed cases of one
program; V12 if mutable. Duplicated replays do not satisfy this dependency.
**Difficulty:** S, ≤2h. **Leverage:** medium, conditional.
**Files:** `crates/physis-cognition/src/compile.rs`,
`crates/physis-cognition/src/program.rs`, `crates/physis-runtime/src/primitives.rs`,
`tasks/handoffs/V13/RECEIPT.md` (new).

- [ ] Use existing quarantine/replay and append-only primitive overlay; retain
  program, applicability scope, primitive revisions and check obligations.
- [ ] Replay on fresh evidence; changed root/contract/input rejects applicability.
  Acceptance: held-out counterexample invalidates chunk; rollback appends history;
  no old check result reused as present truth. Remembered prose stays lexical
  retrieval, not a new causal rule.

### V14a — primitive learning requires behavioral checks

**Goal:** replace repetition-as-proof with explicit evidence for one learned
primitive/rule. **Dependencies:** V00; no paid-customer prerequisite.
**Difficulty:** S, first slice ≤2h; further contract slice separately assigned.
**Leverage:** high for architecture/credibility.
**Files:** `crates/physis-cognition/src/cogprim.rs`,
`crates/physis-runtime/src/primitives.rs`, `tasks/handoffs/V14a/RECEIPT.md` (new).

- [ ] First reproduce wrong-result acceptance with one small discriminating
  fixture: a primitive produces the expected note but wrong output. Current
  `save_primitive_edit` checks note substrings; semantic confirmation must not.
- [ ] Extend existing example/check contract with exact output or behavioral
  assertions, keeping old example files readable. Legacy note-only checks remain
  smoke evidence; they cannot certify learned semantic correctness. Candidate
  itself must not supply its sole oracle. Assert real relation/value, negative
  case, boundary and held-out case from an operator-owned fixture.
- [ ] Acceptance: exact values/relations and correct abstention are checkable;
  wrong output with correct prose fails; invalid definition appends no edit;
  old example files remain readable. Note-only checks are visibly weaker.

**Gate:** focused cognition/runtime primitive tests plus actual existing
`physis primitive edit/run` path. No confidence UI or learner database in this card.

### V14c — distinct confirmations, scoped score and withdrawal

**Goal:** repeated execution cannot manufacture certainty.
**Dependencies:** V00; can proceed independently of V14a in separately assigned
work, with serialized runtime file ownership. **Difficulty:** M, 2–4h split at
evidence contract / derived gate. **Leverage:** high.
**Files:** `crates/physis-cognition/src/compile.rs`,
`crates/physis-cognition/src/deterministic.rs`,
`crates/physis-runtime/src/cognition.rs`, `tasks/handoffs/V14c/RECEIPT.md` (new).

- [ ] Baseline-reproduce duplicate replay inflation in existing Quarantine.
  Add version/scoped evidence identity and repetition/distinct-case separation;
  backward-compatible old counter-only records have UNKNOWN evidence breadth.
- [ ] Derive score and eligibility from real case outcomes, never selection
  confidence or compiled-backend hits. Unknown required checks block promotion.
  Include held-out/control coverage and preserve failures. A duplicate that fails
  invalidates that case's success. Apply the confidence rules below.
- [ ] Key merge/reload by candidate + contract/implementation version + scope;
  existing pattern+candidate keys cannot merge different claims. Match explicit
  applicability guards, not only substring keyword. Source trace digest cannot
  recover missing conditions: require captured state/inputs and reject incomplete
  candidates. Acceptance covers same keyword/different target and scope changes.

**Proposed minimal additions:** a serde-compatible validation record under existing
`CompiledRule`/primitive provenance, not a new learning database. Separate
`repetition_count`, `distinct_case_count`, `held_out_count`, `failure_count`,
`unknown_count`, `confidence_source`, `confidence_score` and applicability scope.
Use existing observation bodies for full check receipts; counters are derived.

**Acceptance:** 100 identical passing executions increase reproducibility count,
not evidence score; a wrong result with correct prose fails; unknown outcome does
not count as success; changing implementation/contract starts a new version's
evidence; one in-scope counterexample withdraws eligibility. No new learning loop
is advertised until V14b makes production persistence reachable.

**Gate:** cognition/runtime tests including all `CompiledRule` construction sites;
seeded counter-only observations never masquerade as confirmed new records.

### V14b — wire propose → confirm → persist → reuse → challenge

**Goal:** first user-reachable, revision-bound primitive learning loop.
**Dependencies:** V14a/V14c; V10 only for learning composed programs, not single
registered primitive/rule candidates. **Difficulty:** M, 4–6h in ≤2h slices.
**Leverage:** high. **Files:** `crates/physis-runtime/src/cognition.rs`,
`crates/physis-runtime/src/primitives.rs`, `apps/physis-cli/src/cmd_primitive.rs`,
`apps/physis-hud/src/modules/cognition.rs`, `tasks/handoffs/V14b/RECEIPT.md` (new).

- [ ] First wire runtime confirmation through proposed
  `physis primitive confirm <id> --cases <path> --path <root> [--validate]`.
  Without `--validate`, append confirmation evidence only; explicit validation
  still requires all pilot checks. A pending definition lives as a candidate
  observation, not an active primitive edit. Candidate carries input/output
  contract, scope and source trace references. Use existing quarantine and
  append-only overlays/observations; no eval/JIT or parallel primitive registry.
- [ ] Persist actual checked replay outcome through a shared runtime method,
  then reload and derive current eligibility. Implement the missing confirmation
  writer rather than incrementing a HUD badge. Success requires actual check and
  target predicate; a selection or compiled-backend hit is not success.
- [ ] Surface candidate state, evidence counts, uncertainty, last counterexample
  and “why eligible” in the existing cognition module. Acceptance: restart retains
  check history/eligibility; duplicate invocation adds repetition only; edited
  scope/primitive invalidates evidence; failed/unknown observation blocks required
  checks; explicit validated read-only candidate can earn reuse, then lose it.

**Authority:** preserve existing explicit `Quarantine::validate` boundary. Models
may propose candidates and discriminating tests under assigned authority; existing
registered mechanisms execute them. Model-proposed tests do not validate their
own output alone. Confidence never grants permission to mutate a host or merge.

**Proposed runtime seam:** `confirm_primitive(&mut self, id: &str, cases: &Path,
validate: bool) -> anyhow::Result<ConfirmationReport>`. Report carries candidate
version/scope, check observation IDs, repetition/distinct/held-out/failure/unknown
counts, optional score and lifecycle status. Use existing observation append,
definition validation and quarantine gate. JSON confirmation file names fixed
inputs, fixture state and exact expected outcomes; it contains no arbitrary
shell commands. Replay of an isolated action remains V12, not this command.

### V15 — behavior-derived structure, one bounded learning experiment

**Goal:** decide if a learned behavioral model adds value beyond an explicit
contract/table. **Dependencies:** V00 and a recorded repeated unknown; V14b before
production promotion. Fixture/probe design does not wait for a sale or V14b.
**Difficulty:** S, ≤2h for the first probe. **Leverage:** medium, core evidence.
**Files:** `crates/physis-cognition/src/deterministic.rs`,
`crates/physis-cognition/src/compile.rs`, `crates/physis-mcp/tests/e2e.rs`,
`tasks/handoffs/V15/RECEIPT.md` (new). Prefer research receipt only initially.

- [ ] Probe an existing isolated MCP freshness fixture: create → observe → query
  → edit without observe → query → re-observe → query. Record inputs/outputs,
  resets, revisions, applicability and counterexamples, including old stale-session
  behavior. Use static/source evidence alongside black-box observations.
- [ ] Compare a finite behavioral hypothesis against simple table/prior cache on
  held-out sequences. Unknown input stays UNKNOWN; a counterexample must revise
  or reject hypothesis. Passing finite probes is not equivalence proof or recovery
  of unique internal/causal structure.
- [ ] Keep observation, hypothesis and desired user outcome distinguishable.
  Record experiment/reset assumptions and measurement error. Learned freshness
  behavior cannot be presented as a physical law. A simulator is labelled as such;
  any physical claim needs real registered measurements and calibration.
- [ ] Store a versioned model/primitive with provenance in existing observations
  and overlays. CA is an optional model for genuinely local propagation rules,
  not replacement for the canonical store or universal primitive format. Promote
  only if it solves a measured need more simply than the explicit contract.

Current engine already has symbols, rules and deterministic execution: a limited
symbolic layer. General symbolic planning/learning requires these closures; a CA
representation alone establishes none of them. No general AI claim comes from V15.

### V16a — one bounded, owned process path

**Goal:** launched work cannot wedge output capture, escape its root silently or
outlive a claimed cancellation. **Dependencies:** V00 and an isolated reproduction
of each inspected runner risk. **Difficulty:** M, 4–6h, assigned as ≤2h slices.
**Leverage:** extreme for a responsive computer.
**Files:** `crates/physis-runtime/src/lib.rs`,
`crates/physis-runtime/src/process.rs` (new only if extracting the shared helper),
`crates/physis-task/src/solve.rs`, `crates/physis-agent/src/lib.rs`,
`tasks/handoffs/V16a/RECEIPT.md` (new).

- [ ] Reproduce in temporary fixtures: output larger than pipe capacity on both
  streams; timeout; cwd-sensitive command; child spawning a descendant that keeps
  a pipe open; cancellation. Record baseline failures versus unverified risks.
- [ ] Reuse the task runner's concurrent drain and bounded retained output. Agent
  and task both depend on runtime, so move the minimum helper to that existing
  common dependency if sharing is needed; preserve their public wrappers/results.
  Runtime must not depend on task/agent. No new executor crate or external service.
- [ ] Add explicit cwd, monotonic deadline and cooperative cancellation to the
  actual shared path. Preserve output limits while draining excess. A stdlib
  cancellation flag must be observed by the wait loop, not merely hide UI output.
  Use the supported Linux process-group/native termination mechanism; record
  platform limits. Cancellation is not proof that arbitrary escaped processes
  are contained. If cleanup cannot be established, return UNVERIFIABLE.
- [ ] Acceptance: noisy process completes or times out without blocked pipe joins;
  output stays bounded; exact root is in receipt; cancellation/deadline removes
  owned fixture descendants within 2s and never reports success. Refuse unsupported
  adapter IDs. Ordinary task/delegate exits and timeout reporting stay compatible.

**Gate:** focused runtime/agent/task runner tests, native delegate wire check,
fmt/clippy. CLI/MCP/HUD must reach the same corrected runner before declaring
cross-surface cancellation. No concurrent actions added in this card.

### V16b — responsive HUD during observation and execution

**Goal:** keep the same focus and evidence usable while work runs.
**Dependencies:** V06 pinned HUD fixture; V16a before delegate cancellation claims.
**Difficulty:** M, 4–6h in ≤2h slices. **Leverage:** extreme.
**Files:** `apps/physis-hud/src/app.rs`, `apps/physis-hud/src/event.rs`,
`apps/physis-hud/src/tests/shell.rs`, `apps/physis-hud/src/tests/render.rs`,
`tasks/handoffs/V16b/RECEIPT.md` (new).

- [ ] Measure key-to-feedback while a 30-second fixture delegate runs and while
  observation processes a recorded repository. Keep benchmark load/terminal and
  event timings in receipt; current synchronous source is a risk, not measured
  latency. Existing `frame_budget` only measures rendering, not blocked key handling.
- [ ] Keep one running request in existing HUD state: ID/root/revision, start/deadline,
  cancellation handle and result channel. Use stdlib thread/channel for slow work;
  input/render owns focus and coherent snapshot. Do not hold a runtime mutex across
  slow work. Start with one active operation and one coalesced refresh request.
- [ ] Apply a completed result only to its matching root/request/revision; retain
  stale-result evidence without replacing newer edits/focus. Show last coherent
  world, freshness and actual running/elapsed state. Commit the observed fingerprint
  only after successful observation; failed observation must remain retryable.
- [ ] Cancellation uses V16a's actual process handle/flag. For pure observation,
  stop scheduling further work and reject obsolete results; distinguish requested
  cancellation from proven completion. Persist the task outcome through the existing
  task/observation path before removing the running indicator.
- [ ] Acceptance on the recorded reference fixture: navigation p95 ≤100ms,
  acknowledgement/cancel feedback ≤250ms, owned process cleanup ≤2s. While work
  runs, type, change lenses, navigate back/forward and inspect a pinned note.
  Stale completion cannot overwrite a newer edit; failed refresh retries; terminal
  restore remains correct. Report misses rather than silently relaxing thresholds.

**Gate:** affected HUD tests, existing frame/snapshot gates and one real terminal
journey. These are proposed acceptance targets, not measured current performance.
Crash-time reconciliation may show UNKNOWN; arbitrary checkpoint/resume is later.

### V17 — smallest model that actually operates the computer

**Goal:** measure the lowest-resource available language interpreter that chooses
valid actions and achieves the supported goals. **Dependencies:** V06 identity
proof, V10 real execution and V07 whole-interaction accounting. Read-only fixture
design can precede V10; interactive inference must satisfy V16b's scheduling rules.
**Difficulty:** M, 4–6h initial routed pilot, split into ≤2h assignments.
**Leverage:** high after execution and confirmation exist.
**Files:** `crates/physis-reasoning/src/guided.rs`,
`crates/physis-runtime/src/cognition.rs`, `crates/physis-registry/src/lib.rs`,
`research/product/minimum-model-cases.json` (new),
`tasks/handoffs/V17/RECEIPT.md` (new).

- [ ] Freeze 12 acceptance cases and six withheld phrasing/context cases before
  choosing a model. Cover graph lookup/impact/context, file/host inspection,
  notebook references, task status, ambiguity, unavailable capability, changed
  revision and unsupported write authority. Arithmetic joins only after V03.
  Include non-code machine tasks; do not claim these cover every domain.
- [ ] Reuse `ConstraintCompiler::compile`, `GenerationContext`, exact resolution
  and `GuidedAction::validate`; build them from the current runtime slice/focus.
  Exact deterministic requests bypass the model. Ambiguity permits clarification;
  a language score cannot supply a missing target or waive a check.
- [ ] Use one already available registered/configured model or harness, constrained
  to the existing choice schema/grammar and resolved canonical object handles.
  Reject illegal verb/target combinations, unexpected effects and invalid output.
  Inspect existing callers before adapting a provider. No new provider dependency
  or model download is assumed; absent affordable backend is UNVERIFIABLE.
- [ ] Route the chosen registered operation through V10/existing runtime, compare
  actual goal evidence, and record model identity/version, hardware/provider,
  prompt/output/usage, retries, latency and UNKNOWNs. The mock is a deterministic
  baseline, never evidence that a real weak language model works.
- [ ] Proposed pilot gate: ≥17/18 independently correct intent-and-result outcomes,
  including ≥5/6 held-out cases; zero wrong-target mutations, authority violations
  or false PASS. Clarification/abstention counts as correct only on predeclared
  ambiguous/unsupported cases. Everything else stays in denominator.
- [ ] Among passing candidates choose minimum measured total cost/resources within
  declared latency bounds. If none pass, narrow the supported language or record
  the capability gap; do not inflate results through a hidden stronger-model fallback.
  Unsupported thinking may be delegated explicitly with its own usage receipt.

**Gate:** guided/runtime tests and actual routed case receipts, not schema-only
tests. Learning should improve the engine's reusable procedures without requiring
the chosen model's weights to change. This pilot proves a bounded capability;
whole-agent token-saving claims still require V07's frozen promotion experiment.

### V18 — discover operations through the existing semantic contract

**Goal:** one descriptor/request/preview contract makes supported operations
discoverable across object kinds and interfaces. **Dependencies:** V00; V09/V10
only for executing composed programs. **Difficulty:** M, 4–6h in sealed slices.
**Leverage:** extreme for interface coherence.
**Read/change candidates:** `crates/physis-runtime/src/facade.rs`,
`crates/physis-runtime/src/primitives.rs`, `crates/physis-cognition/src/cogprim.rs`,
`crates/physis-reasoning/src/guided.rs`, `tasks/handoffs/V18/RECEIPT.md` (new).

- [ ] First packet is DESIGN: map current `WorldView`, `WorldWriter`, primitive
  ports, guided constraints and `ExecRequest`/`RoutePlan`. Seal one operation
  descriptor/request/preview format by extending this seam. Unknown precondition,
  effect or verification support remains explicit, not a guessed affordance.
- [ ] Descriptor fields: operation/implementation version, operand roles and
  cardinality, typed output/bindings, root/revision, preconditions, declared
  effects, existing authority requirements, verification obligations, cost/bounds
  and evidence. Execution-environment `Capability` is not user authorization.
  Reuse V09's binding/result semantics rather than defining another value algebra.
  Declare inspection, calculation, reversible mutation, potentially irreversible
  mutation, external execution and privileged/physical effects as applicable;
  reversibility requires a real restoration method, not a reassuring label.
- [ ] Proposed runtime entry points: `available_operations(selection)`,
  `preview_operation(request)` and `execute_operation(preview, limits)` over
  these envelopes. These names are proposed, not existing callable APIs. Resolve
  implementations through existing primitives/writes/tasks/exec; no independent
  operation interpreter. Read-only generic operations are the first implementation.
- [ ] Freeze tests `operation_selection_deduplicates_identity`,
  `operation_rejects_wrong_role_root_revision_units`,
  `operation_preview_has_no_domain_effects` and
  `unknown_kind_keeps_inspection_without_invented_actions` as proposed tests.
  Pure preview performs no object mutation, process execution or remote request.
- [ ] IMPLEMENT packets may begin only after G0/G1/G2 contract review. A descriptor
  advertising an operation must have a registered implementation and a real
  check. Unsupported verbs are unavailable with reasons. Reject table entries
  keyed to every application pair; derive applicability from roles/facets/constraints.

**Acceptance:** known file/symbol inspection is unchanged; a new registered facet
can reuse an operation without a new HUD branch; duplicate selection is unary;
wrong-scope compatible types are rejected; rendering/listing/preview is inert.
**Gate:** cognition/guided/runtime tests, actual discovery/preview route, G1–G3.
The initial contract can be reviewed independently of notebook migration.

### V19 — make notebook cells addressable without parallel truth

**Goal:** a saved note has stable identity, revision and world evidence; notebook
intent cannot claim a write it did not perform. **Dependencies:** V00; V18 before
generic operation exposure. **Difficulty:** M, 4–8h in independently reviewed slices.
**Leverage:** extreme for thought → reality continuity.
**Read/change candidates:** `crates/physis-notebook/src/lib.rs`,
`crates/physis-notebook/src/tests.rs`, `crates/physis-runtime/src/cognition.rs`,
`crates/physis-query/src/engine.rs`, `tasks/handoffs/V19/RECEIPT.md` (new).
Runtime facade wiring/manifests and existing HUD/CLI consumers are read dependencies;
seal separate ≤5-file compatibility packets if source changes are required there.

- [ ] DESIGN packet seals persisted cell identity and migration. Identity survives
  text edit, reorder, restart and insertion; deleted IDs are not reused. A cell's
  own identity is distinct from its referenced target. Two equal texts remain two
  different objects. Root + ID identifies the object; revision identifies its state.
- [ ] Preserve existing notebook JSON as content authority and current cell APIs
  until compatibility migration is reviewed. Derive objects/explicit references
  into the same snapshot/observation history. No standalone notebook graph/store.
  Preserve old unkeyed cells; malformed JSON must be reported and retained rather
  than silently replaced by an empty notebook and saved over.
- [ ] Route mutation through one runtime-owned notebook adapter using the existing
  native validation/atomic save. If an existing in-workspace dependency is needed,
  its manifest/lockfile/caller changes require an explicit compatibility packet;
  no copied JSON writer in HUD, CLI or MCP. Preserve partial-write evidence if
  content persisted but observation append failed; do not return full success.
- [ ] Freeze proposed tests `note_id_survives_edit_reorder_restart`,
  `identical_note_text_has_distinct_ids`, `invalid_notebook_preserves_bytes`,
  `note_intent_requires_persistence_and_reload`,
  `notebook_append_failure_is_partial` and `stale_note_preview_refuses_overwrite`.
  First negative fixture exercises the existing `notebook:note-added` prose path.
- [ ] Persist → reload exact native bytes → re-observe/derive same note IDs →
  inspect revision/history/reference freshness. Legacy `:nb` behavior remains
  accessible; new object history never silently rebaselines a pinned source claim.

**Acceptance:** real saved bytes, reloaded text, same note identity and evidence
exist; failed/unexecuted note creation reports pending/error rather than added.
Existing ordinary notes/pins/goals load unchanged. **Gate:** notebook/query/runtime
tests, genuine notebook intent route and G1/G4/G5. Helper/string tests alone fail.

### V20 — one model-free notebook composition journey

**Goal:** select → discover → compare → preview → transform → verify → return
inside the current HUD, using V18/V19. **Dependencies:** V06/V18/V19; V16b before
claiming responsiveness during long execution. **Difficulty:** M, 4–6h in ≤2h slices.
**Leverage:** high, first convincing interface proof.
**Read/change candidates:** `apps/physis-hud/src/focus.rs`,
`apps/physis-hud/src/event.rs`, `apps/physis-hud/src/views.rs`,
`apps/physis-hud/src/tests/notebook.rs`, `tasks/handoffs/V20/RECEIPT.md` (new).
Seal a separate small shell-state packet if `app.rs` needs a change; no sixth file
silently added to an assignment.

- [ ] Keep one focus/history and a deduplicated operand selection by root + ID.
  Lens switches preserve both. Revision operands are explicit for self-comparison;
  duplicate current rows do not masquerade as two objects.
- [ ] Available operations come from V18, not a notebook-specific pairing menu.
  First result is exact text difference/shared explicit references; unsupported
  claim contradiction, inferred association or synthesis remains labelled/unavailable.
- [ ] Preview one reference/edit operation with exact targets, base revisions,
  changed bytes, declared effects and checks. Cancel keeps bytes unchanged.
  Execute only through V19/V18; verify saved bytes/reload/world delta; place the
  resulting evidence reference in the originating note without erasing old pins.
- [ ] Freeze proposed tests `selection_survives_lens_and_back_navigation`,
  `same_note_selection_is_one_identity`, `shared_references_are_note_scoped`,
  `cancelled_preview_preserves_notebook` and
  `notebook_operation_closes_world_and_history`. Validate actual keyboard events,
  persistence and restart, not just rendered strings.
- [ ] Real terminal proof: models disconnected; choose two notes, compare, preview,
  cancel once, apply once, follow evidence and return. Use existing spatial shell;
  context keys expose operations. No new top-level tab or compulsory fixed columns.

**Acceptance:** demonstrations A/C's supported subset passes; unchanged keyboard
routes remain available; three recorded journeys complete with zero wrong-target
changes or false completion. Measure interaction count, elapsed time and recovery;
no performance improvement claimed without a baseline. **Gate:** HUD/notebook/
runtime tests, snapshots, real journey and G6/G7.

### V21 — prove cross-domain generalization and five demonstrations

**Goal:** the same operation contract works for notes, code and supported local
host objects without a new domain workflow. **Dependencies:** V18/V20; V12 for
composed actions, V14b/V13 for demonstration E. **Difficulty:** M, 4–6h initial
qualification; unsupported capabilities become separately scoped packets.
**Leverage:** high credibility; architectural generalization gate.
**Read/change candidates:** `crates/physis-mcp/tests/e2e.rs`,
`tests/invariants/hud_modules.rs`, `tests/invariants/action_receipt.rs`,
`scripts/demo-impossible-computer.sh`, `tasks/handoffs/V21/RECEIPT.md` (new).
CLI/HUD dispatch and host adapter are read dependencies; demonstrated missing
connections are assigned to their existing owner, not reimplemented in this gate.

- [ ] Compare canonical selections, operation IDs/versions, preview targets,
  constraints, actual effects and evidence across CLI/native MCP/HUD; formatting
  can differ. Each adapter invokes the shared semantic path. Existing commercial
  tools are outside this expanded interface surface.
- [ ] Qualify A–E using the matrix below. No seeded telemetry counter, prose
  completion marker or graph dependency substitutes for actual outcome evidence.
  Model-free A/B/C use deterministic supported instructions.
- [ ] Host D uses an owned disposable process/service only. Bind machine boot,
  process start/identity and freshness; preview has no effect; execute an already
  supported authorized operation, then observe real state. Missing operation,
  privileges or visibility yields UNVERIFIABLE, not universal-control claims.
- [ ] Counterfactuals disclose assumptions: structural impact, recorded-state
  reconstruction and isolated experiment have separate labels. No general
  simulation engine added. E checks unseen inputs and measures setup/check costs
  as well as savings; interaction preferences require explicit acceptance/undo.
- [ ] Proposed negative check `surface_done_cannot_close_unproven_goal`: a passing
  derived census/loop decision cannot certify a missing desired-state predicate,
  untested notebook operation or physical outcome. Preserve the census result
  separately; terminal review reads the original goal and applicable proof coverage.

**Acceptance:** semantic parity, native history, negative controls and complete
denominators. D/E may remain unverified while A/C graduate; report each status.
**Gate:** G1–G9 applicable rows, affected full integration gates, exact-byte demo
before public promotion. Publish only separately authorized qualified artifacts.

### Confidence: repetition is evidence, with a declared meaning

Do not merge these different quantities into an unexplained float:

- **Selection score:** current cascade's preference for an option. Not measured
  probability that the proposed behavior is correct.
- **Reproducibility:** repeated executions under the same relevant conditions
  agree. Useful for flaky behavior; one hundred duplicates remain one case.
- **Validation support:** distinct scoped cases with independently checked
  outcomes, including correct abstention and correct negative cases.
- **Predictive calibration:** how reported scores align with held-out outcomes
  across many candidates. NOT MEASURED until such data exists.

For a small explicit `confidence_score`, use a conservative validation-support
index rather than self-reported model certainty. Initial proposal: Wilson lower
bound with `z=1.96` on eligible distinct, fully observed case outcomes, accompanied
by numerator/denominator/scope. Duplicate replay cannot increase `n`. Example:
3/3 → about 0.44, 10/10 → 0.72, 100/100 → 0.96. This prevents tiny all-pass samples
from displaying 1.0. Formula from
[NIST's binomial proportion interval](https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm).

```text
n = unique_fully_observed_cases
if n == 0: confidence_score = UNKNOWN
else:
    p = successes / n; z = 1.96
    confidence_score = (p + z*z/(2*n)
        - z*sqrt(p*(1-p)/n + z*z/(4*n*n))) / (1 + z*z/n)
```

For designed/adaptive/correlated probes this is an explicitly labelled support
index, **not** a calibrated truth probability or valid population confidence
statement. Statistical interpretation requires its sampling assumptions; dedup
alone does not establish independence. Report family/context coverage and unknown
cases separately. Pending required checks block promotion even when the displayed
known-case score is high. No fabricated decimal precision beyond two places.

Initial pilot promotion: explicit validation, supported read-only scope, at least
three distinct cases, a negative/control case and a held-out case, no unresolved
required checks or in-scope counterexample, plus existing configured thresholds.
This permits a labelled constrained pilot, not automatic broad deployment. Broad
read-only reuse additionally requires a declared held-out test distribution,
coverage of relevant contexts and proposed support lower bound ≥0.80. At zero
failures that takes about 16 distinct eligible cases; numeric threshold remains
an engineering policy, not proof. Mutation authority stays separate at every score.

Behavioral scope matters more than an average: “callers in this observed Rust
module after re-observation” may earn support; “all repository callers always”
cannot inherit it. Counterexample revises scope or candidate as a new version,
preserving the old failure. Do not silently exclude failures to improve the score.

Growth path: confirmed primitive → checked composition → explicit applicability
rule → bounded symbolic planning → richer behavioral models. Input/output queries
and counterexamples can learn observable state-machine behavior under stated
reset/abstraction assumptions; finite checks do not recover unique internals.
This direction matches
[active model-learning research](https://www.sws.cs.ru.nl/publications/papers/fvaan/lncs10k.pdf)
and uses Physis's source evidence to reduce blind probing. Build the first confirmed
unit now; grow expressive power from demonstrated gaps.

## 7. Bounded loops and pseudocode

Pseudocode specifies behavior for authorized execution. Reuse existing scripts,
handoff files, registry, tasks and receipts; do not implement a new orchestrator.

### Packet contract: one claim, one owner, bounded proof

Packets are written under existing `tasks/handoffs/<card>/` only when assigned;
this plan creates no placeholder handoffs and starts no worker. Reuse
[agent-loops](../../tasks/agent-loops.md), not a second packet generator.
Each packet must contain the following sealed fields; omitted fields are a gate
failure, never permission for the worker to invent scope.

```yaml
packet_id: <card>/<slice>/<attempt>
kind: INVESTIGATE | DESIGN | IMPLEMENT | VERIFY | REVIEW
owner: <existing worker/lane; actual harness/model if known>
baseline: {commit: <sha>, branch: <name>, root: <absolute>, dirty_digest: <hash>}
artifact: {binary: <absolute-or-NOT_REQUIRED>, hash: <sha-or-NOT_REQUIRED>}
world: {store: <isolated>, revision: <value-or-UNKNOWN>, observation_watermark: <seq>}
goal: <one user-visible capability and its desired-state predicate>
objects: <exact root/ID/revision operands; role assignments and missing facets>
existing_owner: <shared function and actual source callers>
interfaces: <consumed/produced fields and version; proposed versus existing>
read_scope: <bounded source locations; source beats prose/receipts>
write_allowlist: <at most five exact files, including handoff; empty source scope for DESIGN>
effects_and_authority: <permitted effects and existing authorization; no inferred grants>
checks: <ID, exact argv/cwd, deadline, oracle, expected predicate, positive/negative fixtures>
gates: <applicable G0-G9; unavailable gates and reason>
limits: {source_context_tokens: 800, summary_tokens: 800, turns: 8, attempts: 2, worker_minutes: 20}
stop: <acceptance, scope conflict, failed precondition, budget or missing oracle>
receipt: <source/bytes/checks/outcomes/cost/unknowns/counterexamples; raw evidence path>
```

800-token source/summary targets bound the supplied excerpts and returned summary;
measure assembled packet and actual model usage separately. If source cannot fit,
split the question or supply exact read locations, never truncate away the oracle.
Compiled gate execution can exceed a worker's 20min only as a separately assigned
verification packet with its own deadline. Waiting yields progress at ≤60s intervals.
No hidden budget increase or retry with a stronger model.

INVESTIGATE produces a reproduction and owner, not a patch. DESIGN produces a
reviewable compatibility/operation contract, not source changes. IMPLEMENT starts
only when that contract, file scope and independent oracle are sealed. Each source
slice has a meaningful failing baseline and passing result; read-only qualification
needs no artificial failure or mirrored test. VERIFY uses the exact candidate
bytes in a fresh fixture. REVIEW decides ACCEPT / REJECT / NEEDS EVIDENCE.

**First notebook packet — V19 / INVESTIGATE:** read notebook `Cell`/load/save,
runtime `answer_traced` WriteNote branch, HUD `save_notebook` and snapshot ingestion.
Source-write scope empty; allowed output is the assigned V19 evidence handoff.
Use an isolated store and supported exact binary to request a note, then inspect
notebook bytes/reload/world entities. Record claimed completion versus actual
persistence, plus positive control through the existing `:nb` save path. If binary
does not match source, classify stale artifact before accusing current source.
Outcome seals the smallest identity/write compatibility packet; no new notebook.

**First composition packet — V18 / DESIGN:** read runtime facade, primitive ports,
guided `ObjectRef`/constraints and current HUD focus. Source-write scope empty;
allowed output is its contract/evidence handoff. Supply one file/symbol selection,
same-object duplicate, different-root operand and unknown kind. Produce descriptor,
request and preview examples plus named independent predicates. Reuse V09 types.
Reject any operation whose implementation, effect or check is missing; record the
gap instead of adding a persuasive fake affordance. Reviewer seals the first
read-only implementation packet only after G0 and G1–G3's test contracts are
reviewed. Their executable results remain pending until implementation is tested.

### Parallel-lane reconciliation and continuation gate

The other agent's mission is valuable input, not authority to overwrite this
checkout or automatically dispatch this queue. Before every assignment:

```text
pin_current_source_and_active_worktrees()
derive_capability_gap_from_source_and_actual_receipt()
if parallel_lane_claims_the_same_object_contract_or_write_owner:
    inspect_its_exact_diff_and_candidate_checks()
    if accepted: reuse_result_and_close_duplicate_card_as_SUPERSEDED()
    else: retain_gap_without_reimplementing_behind_its_back()
seal_one_owner_root_allowlist_oracle_and_limits()
if scope_overlaps_active_writer: choose_independent_ready_packet_or_hold_this_packet()
```

One canonical implementation owner per seam; presentation/read-only review can
proceed independently. “Continuous” means continuously deciding from fresh
evidence, not running forever or ignoring human interruption.
During this review, the parallel lane added uncommitted
`crates/physis-cognition/src/loop_decide.rs` and native MCP wiring. Its receipt
tally/decision is a potential existing control mechanism to qualify and reuse;
this planning turn did not run its tests. A `declare-done` result for all cards
that ran certifies only that receipt's declared scope. It cannot establish the
universal mission or missing semantic checks; retain the terminal goal gate below.

```text
after_each_receipt_or_user_correction:
    census = derive_current_capability_checks_from_existing_catalogues()
    classify_PASS_FAIL_UNVERIFIABLE_UNATTRIBUTED_and_missing_requirements()
    if source_or_artifact_changed: invalidate_only_dependent_old_proofs()
    if assigned_goal_has_all_applicable_proofs_and_terminal_review_accepts: DONE
    else if missing_oracle_authority_or_external_state: NEEDS_EVIDENCE
    else if bounded_repair_ready: assign_one_existing_owner_packet()
    else if recurring_unknown_has_discriminating_probe: propose_one_learning_packet()
    else: retain_open_gap_and_stop_this_attempt()
```

Continue/repair decisions cannot clear failed checks. A derived surface census
is reachability evidence, not proof of semantics. Registry discovery must name
new operations automatically; no hand-maintained “everything passed” list.

### Coordinator / small-worker / stronger-review loop

```text
while founder_budget_remains:
    reality = pin_source_artifacts_roots_and_dirty_state()
    card = first_ready_highest_value_card_with_required_authority()
    if no card: record_missing_external_input(); stop
    seal(card, <=5_write_files, acceptance, negative_control, deadlines)
    if no repro_or_verifiable_need: close_as_obsolete_or_NOT_MEASURED(); continue
    packet = exact_ids + bounded_context + failing_command + allowed_patch_scope
    worker = existing_registered_harness_if_delegation_was_assigned_else_current_worker
    for attempt in [1, 2]:
        run_one_card(worker, packet, isolated_path, max_turns=8, deadline<=20min)
        assert_diff_inside_allowlist_and_no_shared_checkout_edits()
        run_operator_owned_acceptance_on_result_and_negative_control()
        if independent_checks_pass: break
        classify_product_defect_environment_failure_or_unattributed_race()
        packet = same_card + short_counterexample + new_fingerprint
    if not accepted: retain_receipt_and_gap(); stop_this_card
    reviewer_checks_exact_diff_receipts_claims_and_dependencies()
    only_reviewed_authorized_candidate_can_integrate()
    reobserve_affected_isolated_world(); inspect_delta(); record_outcome()
```

Use small agents for bounded investigation/patches with deterministic acceptance;
use the stronger reviewer for semantic contracts, release/licence boundaries,
benchmark validity and final candidate. No endless free-model swarm. Provider
unavailability is UNVERIFIABLE. Eight turns/20min cap is not permission to ignore
the separate four-hour benchmark or founder budgets. Exceeding scope creates a
new card; it never silently expands the current packet.

### Shared-object interaction loop

```text
focus = (repo_identity, canonical_id)
while user_session_active:
    render_current_lens(focus, revision, freshness, available_actions)
    event = read_user_event()
    if switch_lens: keep(focus)
    if follow_object: push_navigation_history(); focus = selected_canonical_id
    if back_or_forward: restore_identity_then_render_current_or_missing_object()
    if pin: save_existing_notebook_ref(id, observed_fingerprint)
    if request_agent_context: compile_same_focus_bounded_packet_with_revision()
    if propose_action: preview_target_diff_checks_and_existing_authority()
    if authorized_execute: run_existing_action_loop()
    refresh_derived_views_and_show_stale_pins_without_rewriting_their_history()
```

### Universal selection → composition → representation loop

```text
selection = deduplicate_by_root_and_canonical_identity(displayed_operands)
keep_one_persistent_focus_and_its_navigation_history()
state_refs = explicit_revision_operands_if_comparing_history()
facets = resolve_authoritative_or_observed_facets(selection, state_refs)
operations = registered_contracts_matching_roles_facets_constraints_and_authority()
display_applicable_operations_and_rejected_or_unavailable_reasons()
if lens_switch: change_projection_only(); preserve_identity_selection_and_history()
if choose_operation:
    request = seal_operands_revisions_implementation_inputs_checks_and_effects()
    preview = shared_runtime_preview(request)
    verify_preview_did_not_apply_domain_effects()
    if cancelled: keep_objects_unchanged_and_return_to_focus()
    else if authorized:
        recheck_preconditions_and_native_revisions()
        execute_shared_existing_path(); observe_actual_consequences(); run_checks()
        insert_actual_evidence_reference_into_originating_note_if_requested()
        retain_original_claim_and_revision_when_source_changes()
```

Compatible shape alone cannot establish relevance or causality. Two notes sharing
text can be compared exactly; a contradiction needs supported claim semantics.
User-created relations are labelled authored. Inferred associations stay proposed.
Native JSON/process/filesystem state is observed through its adapter, not overwritten
by a lens or reconstructed historical display.

### Desired-state reconciliation, bounded and inspectable

```text
require(explicit_goal_scope_checks_existing_authority_and_limits)
for reconciliation_attempt in [1, 2]:
    current = observe_registered_target_with_freshness_and_measurement_limits()
    if goal_and_required_invariants_proven(current): retain_receipt(); stop_PASS
    if unknown_required_state: stop_UNVERIFIABLE_with_missing_observation()
    proposal = existing_task_solve_or_explicit_program_under_registered_contracts()
    if no_supported_operation: stop_UNKNOWN_with_capability_gap()
    expose_target_preconditions_effects_checks_and_cost()
    if cancelled_or_authority_missing: stop_without_new_effects()
    execute_existing_path_then_reobserve_and_check_at_resulting_revision()
    if counterexample: preserve_it_and_replan_only_this_bounded_goal()
if no_proven_goal: retain_FAIL_or_UNVERIFIABLE_and_stop()
```

“Keep this service running” requires an explicit observation interval, allowed
outage and checks during transformation. Before/after running snapshots cannot
prove continuous availability. If the available observer cannot establish the
invariant, report that limitation rather than changing the requested meaning.

### Intent → inspected execution → same-world outcome

```text
on_user_intent(text, focus):
    base = pin_root_world_revision_focus_and_existing_authority()
    context = existing_constraint_compiler(current_world_slice(base), text)
    choice = existing_exact_resolution(context, text)
    if no_unique_choice:
        if ambiguity_needs_user_fact: ask_one_bounded_clarification(); return
        if no_available_capability: return UNKNOWN_with_gap_and_evidence()
        choice = configured_small_model.choose(context, text, bounded_usage_deadline)
    reject_unregistered_choice_wrong_target_wrong_effect_or_changed_revision()
    goal = explicit_target_desired_state_constraints_and_checks(choice)
    program = explicit_or_applicable_confirmed_program(goal)
    if no_program_and_bounded_planner_enabled: program = plan_under_contracts(goal)
    if no_verified_route: return UNKNOWN_with_missing_contract_or_observation()
    expose_program_bindings_checks_cost_and_authority()
    task = enqueue_one_owned_existing_execution(program, base, limits)
    show_running_receipt_without_waiting_on_input_thread()
    on_completion: reobserve_check_goal_and_record_same_world_outcome(task)
```

Model choice is advisory. Validate at execution time as well as selection time;
the object/revision may change between them. Reuse program structure, not cached
success. A new user instruction can alter the goal or cancel work; it cannot
retroactively turn a failed measurement into success.

### Responsive foreground / bounded background loop

```text
foreground_loop:
    process_input_and_focus_navigation_without_blocking_on_work()
    drain_bounded_result_messages()
    for message:
        retain_result_receipt()
        if matches_active_request_root_and_revision: apply_coherent_result()
        else: label_stale_without_overwriting_newer_state()
    if cancel_requested:
        signal_existing_owned_runner(); show_cancel_requested()
    if refresh_needed_and_idle: launch_one_refresh(); coalesce_further_requests()
    draw_last_coherent_world_focus_progress_and_freshness()

background_operation:
    use_existing_checked_program_or_bounded_process_runner()
    enforce_monotonic_deadline_output_limit_and_cancellation()
    check_owned_cleanup_and_record_actual_status()
    reobserve_after_actual_effects_even_if_cancelled()
    send_revision_bound_result(); never_mutate_foreground_focus()
```

Initially one active operation avoids competing canonical writers. Observation
may finish after cancellation; its obsolete result stays evidence, not a new
foreground snapshot. Long model calls use the same attention/cancellation rule.

### Program / bounded-planner loop

```text
compile_or_load_program(intent, registered_primitive_revisions)
validate_bindings_targets_units_scope_and_limits()
if explicit_supported_program: execute_it
else if supported_structured_goal_and_V11_enabled:
    agenda = [goal]; seen = {}; expanded = 0
    while agenda and expanded < 32:
        pop_best_by_declared_cost_then_stable_id()
        reject_cycles_depth>4_and_wrong_scope_or_effect()
        expand_at_most_3_registered_contract_matching_candidates()
        expanded += 1
        if grounded_plan_found: replay_then_check_goal(); stop
    if no grounded_plan: return UNKNOWN_with_missing_contract_or_input()

for step in validated_program.steps[0..max8]:
    stop_if_cancelled_deadline_or_revision_precondition_changed()
    args = literals_or_validated_prior_output_ports()
    outcome = existing_runtime_run_primitive(step.id, args)
    retain_named_results_backend_cost_and_evidence_references()
    if error_or_unknown_required_result: stop_with_receipt()
return_same_program_fingerprint_and_actual_trace()
```

Semantic contract validation precedes search. `cost_class` must never be displayed
as measured money savings. Learned suggestions do not bypass authority or checks.

### Canonical action / task closure loop

```text
require(explicit_target_scope_declared_checks_and_existing_authority)
base = pin_root_source_world_and_input_hashes()
preview_action_or_create_existing_isolated_task(base)
if source_changed_since_preview: return CONFLICT
execute_existing_canonical_write_or_registered_delegate()
checks = run_declared_checks_on_exact_changed_bytes_with_deadlines()
rt = reobserve_same_root_or_isolated_task_root()
delta = compare_structural_evidence(base, rt)
if checked_bytes_changed: verdict = UNKNOWN_revision_mismatch
else if any_check_failed: verdict = FAIL
else if any_check_unavailable_or_goal_unverifiable: verdict = UNVERIFIABLE
else if checks_pass_and_desired_predicate_proven: verdict = PASS
else: verdict = FAIL_or_UNKNOWN_with_reason
record_revision_bound_outcome_delta_and_claims()
retain_existing_remember_outcome_when_authorized_and_root_verified()
present_reviewable_candidate(); never_auto_merge_from_delegate_prose()
```

### Token/cost loop

```text
freeze_tasks_oracles_model_host_limits_and_primary_metric()
for matched_task in declared_corpus:
    run_arms_in_counterbalanced_order_on_isolated_identical_roots()
    for arm in [baseline, physis]:
        ledger = provider_usage_events_for_every_attempt_and_retry()
        keep_tool_schema_history_evidence_fallback_latency_setup_and_acceptance()
        total = provider_total_if_complete_else_NOT_MEASURED
        never_add_overlapping_reasoning_or_cached_subfields_twice()
    retain_pair_even_if_one_arm_fails_or_times_out()
report_all_pairs_and_paired_PASS_subset_separately()
if completeness_or_correctness_gate_fails: no_savings_claim()
else apply_existing_M4_primary_metric_thresholds()
change_one_measured_cause(); rerun_only_affected_diagnostic_then_required_gate()
```

A smaller repeated packet still costs input tokens when retransmitted. Delta-only
evidence needs acknowledged matching base/root/revision and otherwise sends a
complete bounded packet. Tool aliases do not justify hiding source or coverage.

### Surprise / paid activation / retention loop

```text
for presentation_round in [1, 2]:
    require_truthful_demo_and_anonymous_qualified_download()
    show_to_5_new_consented_target_viewers()
    record_understood_independent_install_and_real_task_denominators()
    if 4_understand_and_3_install_and_2_real_tasks: break
    improve_only_largest_observed_obstacle_within_engineering_budget()
if no_real_tasks_after_round2: stop_optional_polish(); revisit_problem()

for qualified_buyer in first_10_authorized_demonstrations:
    agree_one_verifiable_supported_task()
    compare_current_workflow_with_Physis_and_disclose_free_overlap()
    record_result_time_coverage_support_and_declined_or_pending_offer()
    if voluntary_purchase:
        verify_payment_delivery_licence_activation_and_useful_result()
        day7_record_repeat_task_or_abandonment_reason()
if 0_purchases: change_buyer_problem_or_offer_before_new_features()
elif purchases_but_no_repeat: repair_one_observed_retention_failure()
elif >=3_paid_repeat_users: test_repeatable_acquisition_with_existing_spend()
```

### Primitive learning / confirmation / confidence loop

```text
require(one_supported_learning_goal_and_bounded_assignment)
candidate = propose_from_traces_or_explicit_user_definition()
freeze_contract_scope_implementation_version_and_check_oracles()
store_in_existing_quarantine(); candidates_never_execute_unreviewed_writes()
for attempt in [1, 2]:
    for case in distinct_examples_boundaries_negative_and_held_out_checks:
        run_registered_implementation_in_isolated_replayable_fixture()
        outcome = independently_check_exact_result_and_desired_predicate()
        append_version_bound_check_receipt()
        deduplicate_by_case_identity_without_erasing_failed_repetitions()
        recompute_counts_support_score_and_eligibility()
        if in_scope_counterexample:
            withdraw_eligibility(); retain_counterexample(); break
    if required_checks_pass_and_explicit_validation_and_pilot_scope:
        make_read_only_pilot_eligible(); break
    if correction_needed: propose_one_scoped_new_version_then_reset_its_evidence()
    else: retain_UNKNOWN_or_REJECTED(); stop
on_future_use:
    verify_applicability_versions_freshness_and_authority()
    execute_existing_primitive_or_program_path()
    record_checked_outcome_not_backend_hit()
    repeated_same_case_updates_reproducibility_not_breadth()
    counterexample_immediately_suspends_future_eligibility()
```

Learning can save future model calls by reusing verified structure. Every
proposal/check/retry still enters V07's total-cost ledger; compile/replay latency
`saved_ms` is milliseconds, not measured tokens. First learning proof must show
both evidence improvement and its actual checking cost.

### Behavioral research loop

```text
require_selected_unknown_problem_and_two_hour_research_allowance()
define_input_output_abstraction_reset_assumptions_and_scope()
candidate = simplest_existing_table_or_finite_transition_hypothesis()
for discriminating_probe in bounded_probe_set:
    observe_actual_behavior_and_record_provenance()
    if contradicts(candidate): retain_counterexample(); revise_or_reject()
replay_on_held_out_sequences_and_compare_simple_baseline()
if no_incremental_useful_prediction: preserve_receipt(); do_not_promote()
else quarantine_versioned_model_and_require_future_counterexample_checks()
```

### Rule discovery → prediction → user-directed action

```text
require(named_observable_problem, registered_measurements, bounded_experiment)
scope = quantities_units_inputs_reset_noise_calibration_and_versions()
goal = user_desired_predicate_and_constraints()  # distinct from measurements
baseline = simplest_existing_explicit_table_or_predictor(scope)
hypotheses = <=3_scoped_candidates_from_rules_traces_or_bounded_model_proposal()
freeze_independent_checks_and_held_out_probes_before_candidate_selection()
while experiment_budget_remains_and_unresolved_candidate_difference:
    probe = smallest_authorized_intervention_that_distinguishes_predictions()
    prediction = record_before_observation(hypotheses, probe)
    observation = registered_measurement(probe, calibration)
    append_actual_outcome_noise_unknowns_and_counterexamples()
    reject_or_version_candidates_contradicted_within_declared_tolerance()
    repeated_probe_updates_reproducibility_not_distinct_coverage()
evaluate_frozen_held_out_error_and_failure_against_baseline()
if no_useful_incremental_prediction: retain_findings_without_promotion(); stop
candidate = quarantine_best_scoped_model_with_checks_and_limitations()
require_existing_confirmation_and_authority_before_using_it_for_actions()
on_new_evidence: recheck_applicability_and_withdraw_on_counterexample()
```

For deterministic contracts compare exact predicates; for noisy physical models
predeclare error metrics/tolerance and report residuals, calibration and competing
explanations. Binary primitive support and predictive uncertainty are different
measurements. The user's goal selects among feasible checked actions; failed
predictions update the model. A new observer/sensor joins through the same registry
and observation store only when a concrete experiment requires it.

## 8. Integration, release and review gates

### G0–G9: explicit evidence before graduation

Gate statuses are PASS / FAIL / UNVERIFIABLE / UNATTRIBUTED; applicability is
declared before execution. NOT_APPLICABLE needs an explicit reason and cannot
prove the excluded capability. A design can be accepted for implementation while
its executable gates remain unverified. Record exact source/artifact/world/check
versions; any relevant change invalidates dependent proofs. All required checks
must pass; no averaging away a veto or relabelling a failed check as continued.

| Gate | Required evidence / negative control | Existing or proposed check route |
|---|---|---|
| G0 — reality, ownership, scope | pinned source/dirty fingerprint/binary/root/store; one lane owns each seam; candidate diff fits ≤5-file assignment | `git status --short`, `git rev-parse HEAD`, exact caller/source read, registry, packet comparison; generated logs never count as callers |
| G1 — identity and representation | identity survives lens/edit/reorder/restart; same object deduplicates; equal text does not merge distinct notes; missing object/facet stays missing; historical revisions remain explicit | V19 notebook/query tests, V20 focus tests; parse/reload actual bytes, inspect source evidence; malformed notebook cannot become a silent empty save |
| G2 — operations and effects | implementation/roles/units/root/revisions/preconditions/checks/authority match; preview changes no domain bytes/state and executes no process/remote effect; cancel is inert | V18 tests with filesystem/process/network negative controls; current facade/primitives/guided contracts; required environmental capabilities do not grant authority |
| G3 — executable meaning | rendered/previewed fingerprint equals executed program; outputs bind correct operands; ambiguous/unsupported/mismatched inputs refuse; no hidden alternative plan | V09/V10 runtime/cognition proofs; V18 descriptor checks; unknown returns or missing effects cannot satisfy desired predicate |
| G4 — actual consequences and recovery | canonical/native mutation, actual saved bytes/process state, re-observation and delta agree; stale preview/conflicting edit refused; partial append/cancellation remains visible | V12/V19 task/notebook/runtime checks; existing mutation/action-receipt invariants; rollback is tested where supported, unavailable undo stays unavailable |
| G5 — verification and honesty | independent oracle checks intended resulting state/revision; zero false PASS; missing/failed/timeout/stale observations remain visible; ordinary before/after samples cannot prove continuous invariants | existing task/solve checks, packet-specific expected predicates and negative controls; filtered test must report ≥1 actual matching test, not exit0 with zero tests |
| G6 — cross-surface parity | same object/operation/version/preview/effects/check obligations across HUD/CLI/native MCP; presentation may differ; commercial boundary preserved | V21 native wire/CLI/HUD traces; actual dispatched operations, `just mcp-e2e`, `just snapshot-smoke`; schema/export alone cannot pass |
| G7 — human continuity and responsiveness | models disconnected for core A/B/C; real keyboard journey, stable focus/history/selection, visible errors, cancellation/recovery; measure interactions/time versus baseline | V06/V20 terminal journey, affected HUD tests; V16 input/progress/cleanup checks. Latency targets remain proposals until measured on recorded machine/load |
| G8 — learning and transfer | candidate quarantined; varied/held-out independent checks; repeats deduplicated; counterexample withdraws; applicability/version guards; setup/check/retry costs retained; explicit preferences undoable | V14/V13/V15 actual replay receipts; V07 total-cost ledger; V17 model pilot separately. Cached answer, seeded counter or repeated lens use is insufficient |
| G9 — distribution and paid proof | exact downloaded bytes/version/source, immutable artifacts, anonymous access, signed licence/closure/privacy/IP gates as applicable; paid claim needs payment/activation/reuse | existing release/boundary/staging/version gates plus V01/V02/V08; native/public/commercial qualifications remain separate |

**Gate execution order:** provenance/scope → semantic invariant tests → real
runtime path → actual CLI/MCP/keyboard path → relevant regressions → independent
review → separately authorized integration/publication. Checks attach to the
actual capability, not how impressive its transcript looks. Source-changing
packets run the relevant `cargo test -p <actual-package>` and existing gates;
the coordinator supplies literal commands, fixtures, deadlines and expected
predicates in each sealed packet. Proposed test names in cards do not yet exist.

### Five demonstrations: independent acceptance matrix

| Demo | Minimum complete proof | Owner / prerequisites | Current claim boundary |
|---|---|---|---|
| A — knowledge becomes action | note/cell selection → typed supported intention → exact target/program/preview → authorized native write → byte/reload/world verification → evidence reference in original note | V18/V19/V20; V09/V10 if composed, V12 if action task | Current note-intent prose does not pass; initial deterministic instruction is allowed |
| B — programming without switching | graph focus → actual callers/deps/history/tests → explicit change → structural preview → canonical write → executed checks → changed revision/history | V06/V09/V10/V12; existing native loop/editor | Existing mechanisms are substantial; whole keyboard journey needs evidence |
| C — objects reveal affordances | two stable note IDs → generic available operations → exact differences/shared explicit references → approved edit/reference/view → native persistence/provenance; same-note and wrong-root controls | V18/V19/V20 | First universal slice; no inferred contradiction/synthesis sold as fact |
| D — machine becomes visible | owned disposable host target → observed executable/machine/state → inert preview → supported authorized action → real re-observation/verification | V21, existing host/exec; V16 for long/cancellable work | Host observation exists; full HUD terminal and universal service control are not established |
| E — computer improves | typed successful traces on distinct cases → versioned procedure → quarantine → varied/held-out checks → explicit promotion → fresh-evidence replay → measured total savings/transfer and counterexample | V14/V13/V07; V10 for composed program | Production confirmation missing; seeded replay counters/prose completion cannot pass |

Every demo receipt includes source and binary hash, root/store/revision, input
objects/roles, executed operation versions, native before/after bytes/state,
checks/oracles, negative controls, omissions, latency/resources and replay steps.
Models disconnected is an exercised condition, not a claim inferred from a flag.
Record five independent statuses; passing A/C never certifies D/E. First public
story may remain the already qualified B fixture while C/E are still developed.

### Integration and release sequence

1. Start from current main in an isolated candidate. Review compute separately;
   do not merge historical branches wholesale or check out main in another
   session's dirty primary directory. Compare remaining branch capabilities,
   preserving stranded useful work until accepted or explicitly postponed.
2. Focused code cards: relevant crate tests, fmt, clippy, runtime/wire behavior.
   Candidate integration: `cargo fmt --all -- --check`, `just ci`, license tests,
   affected invariants, `just mcp-e2e`, `just demo-e2e`, `just snapshot-smoke`,
   `bash scripts/check-commercial-boundary.sh`, `just versions`, applicable
   existing research-loop gates. Test counts derived from output, not fixed prose.
3. Prepare observed HUD fixtures intentionally. Environment/setup/permissions
   failure gets its own attribution. Dirty failing tree is UNATTRIBUTED until
   reproduced on pinned source. No shared `PHYSIS_STORE_DIR` across unrelated
   tests; release/build outputs outside observed roots.
4. Commercial artifacts use production **public verifier** and existing release
   workflow. No private seed needed for build/review. Test keys never ship; use
   existing consented operator entitlement for qualification. Real paid issuance
   belongs to the existing authorized fulfilment path, with idempotency checks.
5. Cut immutable version only after candidate qualification; keep engine/public
   repositories and sales delivery as distinct hosts. Preview exact filenames,
   hashes, source/BUILD provenance, staged allowlist, checksums and page changes.
6. After separately authorized publication/deployment, anonymously download the
   actual customer path and exercise those bytes. A rebuilt local binary cannot
   certify the download. Record version/hash/URL, root and limitations together.

Stronger-model review packet: goal/card, ≤5-file diff, source and artifact hashes,
one baseline failure, changed behavior, independent acceptance and negative
control, exact limits, remaining UNKNOWNs, published claims implicated, founder
time/cost. Reviewer decides ACCEPT / REJECT / NEEDS EVIDENCE. Rejection produces
one bounded correction; repeated unsupported redesign proposals are not execution.

This plan's validation is document/source consistency, paths, dependency order,
budgets and diff hygiene. It does not claim new executable tests were run.
Document check: 25 unique complete cards, exact file scopes, this amendment's
local links/source paths, pinned compute-branch path, balanced fences, 40h allocation
and a dependency graph derived from card fields were verified. G0–G9 and five
independent demo rows are present; final list contains exactly five actions.
Historical unrelated task links were excluded from this amendment's link check.
Shared `tasks/plan.md`'s concurrently appended dogfood text remains outside the
owned commit; its existing whitespace warning was left untouched. Product builds,
acceptance, latency, model qualification and predictive calibration were not run.

## 9. Keep / improve / postpone

**Keep:** append-only evidence; canonical IDs/writes; graph as shared navigation;
existing native coding loop; notebook references; registry-gated delegates;
small commercial tool surface; useful research with honest status.

**Improve next:** exact inspected program execution; actual behavioral confirmation;
owned process cancellation and responsive HUD; minimum-model routed proof. In the
bounded delivery lane, finish anonymous delivery, truthful demonstration and first
useful paid task. Measure full-host cost before advertising savings.

**Stop:** marketing shorter bytes as saved tokens; equating structural predicates
with executed tests; treating gifted licences as sales; duplicating pipelines;
mass launch before download proof; features chosen without buyer/user evidence.

**Develop deliberately:** primitive learning, varied confirmation checks,
inspectable confidence and counterexample-driven withdrawal; behavioral models
through discriminating experiments. Broad symbolic learning is an engine objective
now, built from these reusable units rather than postponed until revenue.

**Postpone:** universal CA storage, an unscoped all-purpose learner, complete CAS,
enterprise control plane, subscription SKU, new UI framework and every-domain
feature programme. Preserve experiments; graduate individual results when useful.

## 10. Zenith mechanisms inside Physis

Operator clarification: integrate Zenith's learning/control features into Physis;
the request is not to run Zenith as an external replacement harness.
Local clone reviewed at `/home/gio/dev/zenith`, clean commit
`a8d9b5786f81e70e73cce7bb164f5f83da84a4b5`.

Zenith's implemented strengths are task-DAG validation, assertion ownership,
separate validator attempts, gates that reject missing verdicts or any dissent,
attention/replanning decisions, durable Markdown skills/memory and terminal gap
review. Its continuous improvement principally changes procedures and orchestration.
The skill loader reads `SKILL.md`; it is not an executable primitive inducer with
calibrated confidence. Source: `zenith_harness/{assets,storage,coordinator,
controller,task_validation}.py` and bundled orchestrator/validator prompts.

Adapt the mechanisms through existing Physis ownership:

| Zenith mechanism | Physis owner | Minimum integration / acceptance | Cards |
|---|---|---|---|
| Atomic contract assertions mapped to fewer work tasks | `physis-task` dossiers/declared checks, existing task cards | Stable assertion IDs, exactly one active work owner, per-assertion evidence; modifying contract invalidates affected proof | V12, coordinator loop |
| Independent validators + AND gate | task outcomes, registry delegates, existing check runner | Missing verdict or any applicable dissent blocks success; receipt binds assertion, root, checked revision and candidate bytes | V12/V14a |
| Attention and adaptive replanning | existing solve constraints, task cards and outcomes | Diagnose earliest wrong artifact; revise contract/oracle/procedure before another identical retry; bounded attempts retain failure | Coordinator loop, V14b |
| Reusable skills from discovered failures | observation history + primitive/program provenance | Model/user can propose a compact procedure with scope and evidence; tested executable subset earns primitive/program reuse; prose remains guidance | V14a/V14c/V14b, then V13 |
| Terminal requirement-gap review | task/history evidence, registered stronger reviewer | Review original intent against actual resulting world; all-green task labels alone cannot close mission; unknowns remain visible | V06/V12, review gate |
| Resume and role-specific context | observation/task logs, context compiler, registry | Reconstruct current root/revision, unresolved assertions and next bounded card; no transcript replay or duplicate task store | V07/V12 |

Two learning layers share provenance, with different evidence requirements:

1. **Procedural learning:** a failed investigation yields a better test recipe,
   command sequence or worker instruction. Preserve this as bounded guidance;
   check whether future use actually improves an independently accepted outcome.
2. **Primitive learning:** a candidate has machine-readable inputs, outputs,
   applicability and a registered executable implementation/program. V14 confirms
   behavior on varied/held-out cases, records scoped support and counterexamples,
   and controls eligibility. A generated skill is only a candidate for this layer.

```text
observe failure or successful trace
  → propose changed procedure / primitive / program
  → freeze contract, scope and independent oracle
  → isolated execution + discriminating validation
  → append checked outcomes and derive confidence
  → explicit validation + eligible scoped reuse
  → new use produces evidence
  → counterexample suspends reuse and revises candidate
```

Keep this inside the existing world: candidates, assertions, checks and outcomes
are inspectable objects/evidence; active definitions use existing primitive
overlays. HUD and native MCP expose the same IDs and receipts. Human-authored
procedures can remain Markdown exports; a second `.zenith` mission database or
parallel scheduler is unnecessary. Commercial Context MCP surface remains small.

Strengthen three source-level limitations during adaptation: Zenith defaults to
shared-workspace concurrent workers; validator verdict schema contains booleans
without mandatory revision/hash evidence; runner forces full-access/no-approval
Codex settings. Preserve Physis isolation, canonical writes, bounded runner and
existing authority instead. Its `continue` decision can clear failed work/gates;
Physis must distinguish an explicitly accepted risk from a passed assertion.

Zenith's published eight-task cost comparison is motivation, not a Physis token
claim. No provider-usage ledger or hard task token/currency budget was found in
the reviewed runtime; V07 and existing time/attempt budgets still apply. Source
syntax parsed for 17 files; pytest/runtime tests were not run because the clone's
environment lacks pytest, FastMCP and Jinja2. No source was imported or changed.

First learning integration slice remains V14a's exact behavioral oracle, followed by V14c
evidence/eligibility and V14b production confirmation. Add assertion-level gates
to V12's existing task loop; learning composed procedures waits for V09/V10's
real output bindings/executor. This produces verified learning inside Physis
without making the Computer depend on a second orchestration system.

## 11. Top five moves

1. **Seal one shared notebook/operation packet:** V00/V18/V19; inspect the parallel
   lane's current diff, reproduce identity/persistence gaps and agree the existing
   owner, contract, negative controls and smallest slice. Avoid duplicated work.
2. **Prove the model-free notebook journey:** V20; select → discover → compare →
   preview → authorized transform → verify → return. V09/V10 execute any composed
   program shown; same identities, native bytes and evidence must persist.
3. **Keep it responsive and cancellable:** V16a/V16b; one owned process/observation
   path, bounded output, real cancellation, stable focus and stale-result guards.
4. **Make learning inspectable and falsifiable:** V14a/V14c/V14b, then V15; checked
   distinct cases, repetition, held-out probes, scoped confidence and withdrawal.
   V13 reuses procedures; V07/V17 measure whole cost and the smallest passing model.
5. **Qualify generalization, distribute proof and test paid demand:** V21 A–E
   independent gates; bounded V01/V02/V04/V05/V08 delivery lane. Anonymous exact-byte
   download, independent viewers and actual paid/repeated use; focused paid MCP.
