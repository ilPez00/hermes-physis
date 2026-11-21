# Physis audit execution plan — bounded loops and review gates

**Status:** execution plan. Existing concurrent AUD-01/02 edits and receipts
are reviewed against their card acceptance below. Any further implementation,
integration, license issuance, payment tests, pushes or publication requires a
separate assignment within the user's authorized scope. Pseudocode specifies
behavior; it is not a request to build another orchestration framework.

**Goal:** earn the first verified customer outcome, using an inspectable free
Computer demonstration to attract qualified users and a small supported Context
MCP/service offer to test willingness to pay. Preserve one world and persistent
focus; fund wider machine development from evidence of repeated use.

**User update, 2026-10-07:** no verified paid customers yet; make the free Computer
surprising enough to earn a trial. Immediate scope is a bounded demonstration and
paid-delivery proof, not the entire platform queue. The concrete 60-second story,
offer, experiment thresholds and cash arithmetic live in
[FAST_PATH](../commercial/FAST_PATH.md). This document retains implementation
detail; workers receive one card, never all of it as their assignment.

**Architecture:** deterministic engine → native CLI/MCP/HUD lenses; separately
bounded commercial Context MCP; derived free Computer distribution; research
with explicit graduation gates. Reuse Runtime, the append-only store, solve,
registry adapters, the existing research runner and existing worker protocols.

**Stack:** existing Rust workspace/standalone commercial crate, Bash/Python
release and evidence scripts, ratatui HUD. No new dependency or runner planned.

**Inputs:** the 2026-10-07 inspection, current source, exact executable/archive
evidence, [existing plan](../../tasks/plan.md),
[worker protocol](../../tasks/agent-loops.md),
[MCP protocol](../../tasks/mcp-agent-loops.md), and
[research-loop design](2026-10-02-research-loop-dumb-agents.md).
Task status belongs in [todo.md](../../tasks/todo.md), not in model prose.

**Self-review correction:** the first version checked formatting and links but
put native/free infrastructure ahead of the commercial critical path. That was
insufficient strategic verification. The revised sequence allows a short free
demo experiment alongside paid proof; neither waits for a HUD rebuild or broader
benchmark programme.

## 1. Baseline and authority

**Historical checkpoint captured 2026-10-08:** local `main` was
`8e633257cb79f887a9b1b6996c0345d856809be3`; live GitHub and `origin/main` were
`79284a58a511645c3431f72d7cbe8a100ac62dcf`. They shared a merge base; local was
14 commits ahead and zero behind. Do not push the whole range by default. The
range touches 55 files (+6,948/−410): native MCP, actual demo checks and fixture,
world-fidelity harness, native loop, Computer installer/docs, HUD focus/object
views, handoffs, and research receipts. These are substantive source and proof
changes, grouped by capability below; this is not an empty or doc-only delta.

The earlier published `9d0ebaf6` is an ancestor of this checkpoint's origin/main,
followed there by one commit. Attribution correction: OpenCode session
`ses_eea5716c0ffeeOBr1f2uU5RL4m` contains an explicit operator instruction at
2026-10-07 16:58:04 UTC to push the reviewed range and publish Computer. That
precedes the push. Calling it an inspection-boundary violation was wrong; it
was a separately authorized operation alongside this read-only audit. Preserve
the event and its authorization, and make no corrective ref changes.
`origin/archive/main-2026-10-04` still resolves to `1ffd1253`.

That checkpoint's primary checkout was dirty. Its read-only status showed modified
`crates/physis-mcp/src/lib.rs`, `crates/physis-mcp/src/protocol.rs`, this plan,
and `tasks/todo.md`; untracked `crates/physis-mcp/src/compact.rs`, a symbolic
composition plan/recon, and `research/product/runs/.bin/`. The symbolic MCP work
is not part of the 14 committed local-only changes or the AUD-13 gate receipt.
Keep it outside release qualification until independently reviewed and tested.

The 2026-10-07 rows immediately below are the **79284 audit baseline**, not all
current defects. AUD-01, AUD-03/03a, AUD-06/09, AUD-11 and the other committed
changes supersede some of those observations. Current status and remaining
gates follow after the table. Recheck every executable/artifact before release.

- `research/product/runs/.bin/`
- `research/product/runs/task-outcome-20261007T100058Z.jsonl`

During this review, concurrent dirty edits appeared in
`crates/physis-mcp/src/tests.rs`, `crates/physis-mcp/src/tools/areas/workspace.rs`,
`scripts/open-release.sh` and `scripts/stage-physis-computer.sh`. They target
AUD-01/02 and are outside this documentation review's ownership. Preserve them,
recapture their fingerprints and review their proofs before assigning duplicate
work. The table below describes the committed audit baseline, not a claim that
those pending patches are absent or accepted.

Worker receipts now exist under `tasks/handoffs/AUD-01/` and `AUD-02/`.
They prove only their recorded assertions. AUD-02's default-directory staging
proof does not cover a custom `CARGO_TARGET_DIR`: the assembler honors that
override while the pending stager still hardcodes `target/open-release/`.
Its receipt also defers extraction/comparison of a newly assembled archive.
Keep those acceptance rows open despite a worker's aggregate PASS or checked box.

A release candidate still needs a clean isolated source tree because the current
publisher requires one. Never clean the primary checkout to satisfy that rule.

| Verified finding | Execution consequence | Source/evidence owner |
|---|---|---|
| The old 81-commit divergence is resolved; audit-final local/GitHub HEADs agree. | No wholesale push, history surgery or orphan-root recovery task. | Git refs/worktrees; root `AGENTS.md` |
| Native catalogue has 62 definitions; default advertisement hides two deprecated tools, yielding 60. | Derive counts from the current catalogue and real `tools/list`; retain compatibility. | `crates/physis-mcp/src/tools.rs` |
| Native `observe` discards the returned Runtime and does not apply `max_files`. | Repair one handler; mutable Runtime dispatch already exists. | `crates/physis-mcp/src/tools/areas/workspace.rs` |
| Commercial manifest/lock are now consistent at 1.2.0; a separate `target/release` binary remains 1.1.1. | Qualify identified archive bytes, not whichever binary is on PATH. | Commercial manifests, archive `BUILD.txt`, executable hash/version |
| Exact commercial 1.2.0 archive passed 15 acceptance checks and 10 value checks; production entitlement provenance, real host and whole-gate isolation remained unverified. | Preserve those proofs without declaring customer delivery ready. | `scripts/commercial-acceptance.sh`, `commercial-value-check.py` |
| Public Computer documents/corpus match staging, but the attached v1.2.0 archive contains older executables. | Repair assembly/provenance before another public sync. | Public assets versus staged/extracted binary hashes |
| Open release builds in `target/open-release/...`; public stager reads `target/release/...`. | Stage the exact qualified build. Fail if required binaries are missing. | `scripts/open-release.sh`, `stage-physis-computer.sh` |
| Open archive copies the engine README; public text promises signatures that are not delivered. | Ship the Computer README and narrow integrity claims to actual checksums. | Open assembler and `physis-computer/` documents |
| Caller fixture has 11 files and 18 oracle pairs; the stored 1.000 scores prove that fixture, not general repository accuracy. | Correct harness orientation/provenance, then add independent holdouts. | `scripts/bench-world-fidelity.py`, `research/benchmark/` |
| Counterexample objects already exist in solve. A comprehensive ChangeReceipt/verifier is not established. | Do not reimplement Counterexample or advertise a future receipt verifier. | `crates/physis-task/src/solve.rs`, kernel receipt types |
| `feat/hud-spatial-shell` contains navigation/object/spatial work absent from main. | Review/reuse that branch before adding parallel HUD mechanisms. | Branch `65a6b6e8` at audit; recapture before assignment |

Workspace tests, HUD tests, fmt, clippy, commercial tests and boundary checks
passed during inspection. Initial socket-binding failures were sandbox permission
failures and passed on an authorized rerun. Historical passes are baseline
evidence, not acceptance of a future patch. A default commercial contract probe
failed because `dist/physis-context` was absent; the release script already passes
its explicit binary path to that probe. Do not assign a nonexistent release bug.

### Authority order

1. Executed behavior, with source/build/artifact identity and an assertion.
2. Current source and contract tests at the assigned baseline.
3. Current manifests, catalogues, release assets and live delivery URLs.
4. Documentation, task status and old receipts, with their dates and scope.
5. Model explanation and recalled memory: leads to investigate, never proof.

Older plans retain useful research. Their unchecked items must be reproduced
before reassignment. This amendment supersedes their immediate execution order,
not their history or unfulfilled acceptance requirements.

### Findings from checking this plan

| Priority | Finding and evidence | Disposition |
|---|---|---|
| P0 for the old source; fixed in local code | Pre-fix binary `3c3bd2f9…` printed `tests: PASS` from a CALLS-edge predicate; its generated fixture failed Cargo with `E0283` at `src/client.rs:29`. Commit `2a8bbce8` now makes the fixture compile, executes tests before/after, uses a behavioral assertion and distinguishes structural/project/unavailable results; task notes record 4/4 demo e2e including wrong-result and missing-Cargo controls. | Source fix has a worker result and committed code. Do not treat the older v1.2.0 archive or pre-fix binary as qualified. Re-run from exact rebuilt bytes before launch. |
| P1 | Earlier demo counted four callers while ignored e2e expected three. | Commit `2a8bbce8` corrects the fixture/oracle and reports 4/4 e2e. Retain the real example caller. |
| P1 — still open | Stager uses `target/open-release/...`, while assembler permits `CARGO_TARGET_DIR` override. A build to a custom directory can leave stale default binaries to stage. | Receipt proves only the default path and defers a fresh archive extraction. Test custom-directory selection plus archive/stage hash equality before closing AUD-02. |
| P1 | First schedule funded infrastructure before an actual buyer workflow. AUD-12 also unnecessarily depended on the fixture benchmark. | Corrected schedule runs customer discovery immediately; benchmark repair gates benchmark claims only. No paid customer is verified. |
| P1 | Coding-loop pseudocode returned a new-worktree result before explicitly executing its checks. Worker scope was checked only after mutation. | Both corrected below: checks run at the actual edited root; scope is checked before and after a patch. |
| P1 | Existing FAST_PATH reported missing sales configuration. Read-only `/api/sales/state` now reports Creem/signer/webhook/email true; 1.1.1 download returns HTTP 200, 1.2.0 returns 404. | Update current runbook; configuration is not paid fulfillment evidence. User confirms no verified paid customers. |
| P2 | Existing development plan says 25 × €49 + 3 × €750 = €3,500 and repeats retired 50 × €50 founder pricing. | Correct to €3,475 and current 25-license founder allocation. Gross bookings remain distinct from profit. |

Fresh demo receipt: `/tmp/physis-plan-review-demo-wi2u4jrg/demo.json`; tested CLI
SHA256 `3c3bd2f930076e30bc695d4691376147c74dacf631c2cca2999014b304ce0111`.
It observed 84 entities/117 relations, four callers/three test relations, changed
`timeout` to `deadline`, recorded two added/two removed structural edges and
passed seven graph assertions. An independent source read confirmed the rename
and different revisions. These results establish the structural loop only.
The MCP dispatcher (`physis mcp physis.object.get`) and a HUD graph snapshot
over that fixture both expose the same request object and changed deadline
relation. This proves shared data access through those entry points; it does
not prove an interactive HUD journey or a real agent-host session.
`cargo test --offline` on the generated project exited 101 with the code error
above; no sandbox/network failure was involved.

Current local commercial archive SHA256 is
`15a488475f7f6f7913c65c693b655f609c55c06b413790e3a0617597eb985760`;
its BUILD names binary SHA256
`b0e663250dc25e742172f1f4ad32f13d69d86fc8f710de69fcc9ff99556df140`,
source `fee50280d9e750ac09afcf81ef20ce066901ff18`, `source_state=dirty`.
Archive hash, executable hash and source identity are separate facts. These
read-only checks do not close its outstanding release/customer gates.

### Accepted work and open proof at the earlier checkpoint (2026-10-08)

- **Native MCP and agent loop:** `f1afc875` refreshes the observe session and
  bounds; `fac4f7e8` improves shared schema checks and execution status;
  `3520604c` pages impact and marks MAY_CALL/UNKNOWN; `3ee01f34` adds a proved
  native coding loop. Tests and receipts exist at the pinned integration point.
- **Demo and free Computer:** `2a8bbce8` fixes executable-vs-graph checks and
  compiles the fixture; `0c7d0080` corrects the Computer first minute and
  installer checks. Current GitHub release bytes remain older than the local
  source/docs. A 60-second recording and five-viewer result remain unproven.
- **Evidence, HUD and integration:** `4020068f` repairs benchmark pair
  orientation/provenance; `9f637707` reviews reusable spatial-shell work;
  `d925f9e2` integrates persistent focus/object views. `01d36d4d` records the
  AUD-13 gate receipt. `8e633257` adds the local real-host AUD-05 receipt.
  These form an ordered candidate, not blanket proof that every tool, HUD
  workflow, claim or release artifact is accepted.
- **Coordination/research:** `ddaaa029` adds this plan; `b98262e4` and
  `6ab9574d` record loop receipts. Review provenance and size for every generated
  result; keep current untracked binary scratch out of commits.

- AUD-01 through the default staging path have code/receipts and are in the local
  commit range. AUD-02 remains partial for the custom target override and exact
  rebuilt archive, despite the broad checkbox.
- AUD-03/03a now qualify the local demo/install source, including failing and
  unavailable-check controls. The public v1.2.0 attached archive predates these
  fixes. Public product README/corpus content needs no blind resync; rebuild and
  compare delivered binaries, installer, checksum and README before any new cut.
- AUD-06/07/08/09 contain benchmark-oracle, shared validation/routing, impact
  bounds and native loop work. AUD-11 integrates spatial HUD focus/object views.
  AUD-13's full gate receipt is pinned at `d925f9e2`, not current `HEAD`; it
  excludes the dirty symbolic-MCP files and operator-dependent production key.
- AUD-04 remains blocked on production verifier material and exact production
  artifact qualification. AUD-05 proves a real OpenCode host against a
  test-signed debug binary; paid delivery, production build and current public
  bytes are not proven. AUD-12 viewer/buyer outcomes and AUD-14 publication stay
  open. No verified paid customers.
- A completed Computer review found the published v1.2.0 tag/tree and its attached
  archive disagree: archive checksum is internally valid, but archived CLI/HUD
  hashes differ from binaries at the current tag; the archive README still says
  `physis-next`, and the tag is unsigned. Treat the attached archive as stale.
  The checksums-only copy and corrected README in local source do not update the
  public bytes. Keep the earlier install/signature claims only when the exact
  artifact supports them.
- The sales host reports configuration flags set, and its old 1.1.1 URL responds;
  1.2.0 returns 404. Configuration is not proof of payment, license delivery,
  buyer activation or repeat use. Commercial verdict remains NOT READY.

### Latest OpenCode follow-up (2026-10-08)

The two latest sessions advanced during this review. These facts supersede the
earlier checkpoint's ref/production-key statements; keep the older proofs tied
to their actual hashes.

- **Compute:** `ses_ee7a887a4ffeNFDHJgJIdeMPbf` finished and pushed
  `feat/compute-primitives` at `23f3fd4a`. The actual saved test output records
  14/14 compute tests; its conclusion reports cognition 81/81, runtime 51/51,
  MCP 90/90 and clippy. It adds arithmetic, unit conversion and date arithmetic.
  Those are explicit deterministic functions, not behavioral model learning or
  cellular automata. The current checkout is on that feature branch.
- **Release:** `ses_ee7f67f4effe93s20bY1bCzmxX` recovered the production verifier
  configuration and qualified clean source at `3b9dd51d`. Its receipt reports
  supplied-byte acceptance 15/15 and value checks 10/10 on built and downloaded
  bytes. This review independently confirmed GitHub's tag at that commit, three
  assets, and publication at 2026-10-08 01:50:45 UTC. Live remote main is now
  `51ed2c96dd20408e3bd3670a3254d484fb95ce86` (release handoff), not `79284a58`.
- **Visibility matters:** GitHub API confirms `ilPez00/physis-next` is private.
  Its published release is operator-accessible, not an anonymous public download.
  The public `physis-computer` v1.2.0 archive still has digest `f167239a…`; the
  newly qualified root archive has digest `fd442aab…`, and Context MCP archive
  `960d344a…`. The public Computer/sales delivery handoffs remain separate.
  Do not close those gates from an authenticated private-repository download.
- **Commercial limit:** real payment, production issuance/customer delivery and
  buyer repeat use remain unverified. No verified paid customers. Technical
  artifact qualification has advanced; customer readiness is still conditional.
- **Symbolic scope:** typed primitive/program/rule objects already exist. The
  composition plan still identifies literal-only step arguments, missing goal
  planning and a textbox path that displays a compiled program while executing
  the older answer/cascade route. An arithmetic primitive does not close those
  gaps. Black-box behavioral models would be a separate, bounded hypothesis
  over observations, with explicit prediction/counterexample/replay evidence.

No code, refs, tests or publication were changed by this follow-up review.
Gate counts above are attributed to the stored OpenCode receipts; they were not
rerun here. Current source and artifacts need their own pinned acceptance.

## 2. Decisions reserved for the stronger reviewer

Give the reviewer this plan, pinned source excerpts, failing/passing receipts and
the complete proposed diff. Do not ask it to certify a README alone. Record the
actual reviewer/model identity when available; do not infer it from a tool name.

The reviewer must resolve these before the corresponding implementation packet:

| Decision | Recommended default | Gate |
|---|---|---|
| Native observation root | Keep the configured session root immutable. Accept omitted path, `.` and canonical-equivalent root paths. Reject root escapes and unsupported descendant re-rooting explicitly. | AUD-01; canonical IDs/store must not silently change namespace. |
| Observation bounds | Honor a positive, checked `max_files`; preserve scanner truncation/liveness semantics. Use the existing default. | AUD-01; no invented arbitrary ceiling or zero convention. |
| Commercial final-text budgets | Preserve MC02's decided cap. Current source tests allow an explained over-envelope response; qualify actual wire and resolve that mismatch rather than silently closing MC02. | AUD-04; raw serialized output and tiny budgets required. |
| MCP execution errors | Distinguish dispatch success from child success; expose nonzero/timeout as unsuccessful execution without losing its receipt. | AUD-07; compatibility impact reviewed. |
| Public release identity | Preserve already-published v1.2.0 assets/tags. Prefer a new 1.2.1 release for changed executable contents. | AUD-13/14; check all owned versions before cutting. |
| Benchmark claims | Say “18/18 oracle caller pairs on this frozen Rust fixture”; broader claims require holdout results. | AUD-06/12; no claim of universal caller completeness. |
| HUD integration | Review existing branch, then integrate bounded slices with reachability tests. | AUD-10/11; no bulk merge based on snapshots alone. |
| Paid product | Keep 12 tools unless a measured customer failure needs a different surface. | AUD-05/12; native feature count is not the commercial roadmap. |

If independent review cannot be obtained, record `UNVERIFIABLE` for that review
gate. Deterministic tests remain useful, but a worker must not impersonate a
second reviewer. Review can reject a patch, ask for one narrower packet, or accept
it; it cannot silently waive an acceptance assertion or enlarge file ownership.

## 3. Schedule, ownership and dependencies

These are timeboxes, not a promise that every card fits the first window. Stop
at a reproducible proof or explicit handoff. Do not spend the window rewriting
positioning while a stale executable is still the customer-facing artifact.

| Window | Work | Exit condition |
|---|---|---|
| NOW: first 15 minutes | AUD-00; reconcile concurrent AUD-01/02 ownership | Current identities and actual remaining work, no duplicate patch assignment. |
| NOW: remaining two-hour budget | Seal AUD-03a demo truthfulness repair and AUD-04/05 commercial qualification; prepare AUD-12's buyer packet | One runnable demo defect reproduction, one concrete paid-delivery gate, one supported offer. Existing inspection evidence can satisfy preparation; do not repeat the whole audit. |
| NEXT: next day | AUD-03a → first 60-second cut; AUD-04/05/13 and commercial portion of AUD-14 | Inspectable demonstration and qualified customer path; original code/identity promises remain enforced. Maximum four engineering hours on first demo cut before viewer feedback. |
| THEN: next seven days | Five qualified Computer viewers, ten qualified buyer conversations/demos, first supported paid outcome under AUD-12; AUD-02/03 public qualification if publishing | Record local trials, actual payments, useful tasks, seven-day repeat use and support cost. Numbers are experiment targets, not promised conversions. |
| NEXT engineering capacity | AUD-01/07/08/09; HUD AUD-10/11 after measured user interruption | Preserve pending work; assign platform changes by an observed blocker, not because the backlog exists. |
| LATER | AUD-06 broader holdouts, AUD-15 and conditional follow-ups | Fixture-harness correctness precedes new benchmark claims; larger research waits for a measured need. |

Dependency graph:

```text
AUD-00 → AUD-04 → AUD-05 → AUD-13 → AUD-14 [commercial channel]
AUD-00 → AUD-03a → bounded viewer experiment [AUD-12]
AUD-00 → AUD-02 + AUD-03 → AUD-13 → AUD-14 [public Computer]
AUD-12 discovery starts now; paid use requires the commercial gates above

AUD-01 + required AUD-07/08 fixes → AUD-09 [native loop]
AUD-10 → reviewed AUD-11 slices            [HUD]
AUD-06 → benchmark marketing eligibility  [not a buyer-discovery dependency]
repeated measured gap + shipping/value gates → AUD-15
```

AUD-12's paid-use proof depends on AUD-04/05 and qualified delivery. Discovery,
offer preparation and bounded Computer viewer tests can start sooner. AUD-06
gates use of its benchmark claims, not all customer work. AUD-09
depends on AUD-01 and any AUD-07/08 repair required by its reproduction. HUD work
does not block commercial release. Research does not block either product.

| Candidate | Required cards/evidence | Separate or conditional evidence |
|---|---|---|
| Computer distribution | AUD-00/02/03/13 and its AUD-14 artifact/install gates | HUD branch integration is optional; any newly advertised native behavior needs its native acceptance. |
| Native MCP | AUD-00/01/07/08/09/13 and real-wire qualification | AUD-06 is required for benchmark claims; customer payment is irrelevant to native correctness. |
| Context MCP technical candidate | AUD-00/04/05/13 and its AUD-14 exact-byte gates; unresolved existing MC contracts remain blockers | Native/HUD feature work is not a dependency. |
| Context MCP paid-delivery readiness | Technical candidate plus operator download/payment/issuance/delivery evidence | AUD-12 is required for superiority/cost/retention claims; unmeasured claims stay out of launch copy. |
| New HUD slice | AUD-00/10/11/13 with event/persistence/user-session proof | Commercial conversion and speculative research do not certify or block its behavior. |

AUD-13 runs gates applicable to the accepted source scope; AUD-14 tracks each
channel independently. Building both archives for the existing publisher does
not certify both products or require speculative features in either.

For the next two 40-hour weeks: at most 12 hours engineering, 20 hours qualified
customer/demo work, eight hours support/measurement; unused support time goes to
customer work. This temporarily supersedes the older 20/12/8 split. Reassess at
day 14 from actual blockers and receipts. No new upfront spend. Native/HUD work
gets at most two of those engineering hours unless it blocks the chosen task.

At most two implementation workers at once; one Cargo build/test lane on this
host. These are ceilings, not a requirement to run agents. Reserve files before
assignment. AUD-01/07/08 share MCP tests/catalogue: serialize their mutations.
AUD-02/03/04 share distribution documentation/gates: reconcile ownership first.
Fresh worktrees cannot see another worker's uncommitted fixes or this uncommitted
plan; supply read-only packet paths explicitly.

### Reconcile old cards without losing their contracts

| Existing queue | Current owner | Rule before reassignment |
|---|---|---|
| MC01/02 context delivery and budget receipts | AUD-04 qualification; narrow follow-up only if it fails | Re-run tiny/zero/full-wire and stale/unknown-receipt assertions. Current source already contains corresponding tests; do not assume an unchecked box means missing code. |
| MC03/04 polling and commercial session freshness | AUD-04/05 | Persistent licensed wire proof owns acceptance; native freshness is a separate handler. |
| MC05 private checkout prerequisite; MC09 observation hang | AUD-00/04 | Reproduce clean dependency closure/observation before accusing current code. Environment and old binaries are distinct causes. |
| MC06 license narrative | AUD-04/05 | Signed offline install and optional legacy network activation must be described separately. |
| MC07/L10 product benchmark | AUD-12 | Real task outcome/whole interaction costs remain distinct from AUD-06's structural fixture. |
| MC08/BIZ-03 native/IP distribution | AUD-02/13/14 | Preserve operator/license boundary decisions; executable-only distribution is not permission to expose private source. |
| BIZ-01/02/05 buyer proof | AUD-05/12/14 | Local fixture PASS does not close paid delivery, real host or repeat-use requirements. |
| NAT/FUR notebook/task/edit/HUD work | AUD-09/10/11 | Reuse committed mechanisms and existing branch work; assign only a reproduced missing step. |

Each accepted AUD receipt names which old acceptance assertions it covers.
Coordinator closes an old card only when all of its assertions are covered at
the appropriate source/artifact identity; otherwise leave it open with the
remaining requirement. No parallel “old fix” and “new fix” for the same owner.

## 4. Ready-to-seal work cards

Paths below are owner candidates. Coordinator confirms callers and seals exact
paths before assignment. Prefer at most five source files per packet. Split a
card if that limit would hide a second behavior. A mechanical version inventory
may span more manifests; reserve and review its explicit metadata allowlist as
one coordinator operation rather than handing a worker `**/Cargo.toml`.

### AUD-00 — pin reality and the candidate

- **Goal:** replace stale assumptions with a current evidence bundle.
- **Files/modules:** read-only Git refs/worktrees; `dist/`; commercial manifests;
  public repository/releases; sales download URLs. Evidence goes to a new
  task-specific directory outside product source.
- **Dependencies:** none. Permission to execute a future inspection is not
  permission to modify refs or production services.
- **Acceptance:** record local/tracking/live remote HEADs, merge-base/divergence,
  relevant branch unique commits and dirty paths; archive SHA, extracted binary
  SHA/version, BUILD/source manifests, licensing mode and installed paths.
  Distinguish current manifest, old loose binary and qualified archive.
- **Difficulty / leverage:** XS / extreme. Budget: 15 minutes if network works.

### AUD-01 — native observe refreshes this session

- **Goal:** an observation updates subsequent calls in the same MCP process and
  applies its declared file limit.
- **Files/modules:** `crates/physis-mcp/src/tools/areas/workspace.rs`,
  `crates/physis-mcp/src/tools.rs`, `crates/physis-mcp/src/tests.rs`;
  `crates/physis-runtime/src/lib.rs` and `crates/physis-discover/src/scan.rs`
  are read-only unless a reproduction proves a shared defect.
- **Dependencies:** AUD-00 and root/bounds policy review.
- **Acceptance:** initialize one real server, observe, search a newly created
  symbol, edit/re-observe, delete/re-observe; no restart or stale live symbol.
  Cap below fixture size reports truncation and applied cap. A truncated scan
  does not tombstone omitted files. Outside-root/symlink escape is refused;
  canonical-equivalent root succeeds. Regression fails on baseline.
- **Proof:** focused native tests, then the current real-binary MCP e2e gate.
  Reuse `*rt = next`; no dispatcher API refactor.
- **Difficulty / leverage:** S / extreme. Budget: 45–90 minutes after sealing.

### AUD-02 — one qualified public build feeds both outputs

- **Goal:** public archive and staged repository contain the same qualified CLI
  and HUD binaries, with the intended Computer README.
- **Files/modules:** `scripts/open-release.sh`,
  `scripts/stage-physis-computer.sh`, `scripts/test-publish-release.py`;
  add a small assembler regression only if existing publisher tests cannot
  exercise the build-output seam. Seal its exact path first.
- **Dependencies:** AUD-00.
- **Acceptance:** fake distinguishable old/new binary inputs expose wrong-path
  copying, including a nondefault `CARGO_TARGET_DIR` with stale binaries still
  present in the default directory. Missing required binaries fail release
  staging; extracted archive and
  staged CLI/HUD hashes equal the qualified build hashes. Archive README starts
  with the Computer product identity. Public layout/install paths still work.
  Required corpus and no-source/IP gates pass on the actual staged tree.
- **Design constraint:** use one explicit qualified binary directory or the
  extracted qualified archive. Preserve useful docs-only staging only through
  an explicit mode that cannot be mistaken for a runnable release.
- **Difficulty / leverage:** S / extreme. Budget: 45–90 minutes excluding build.

### AUD-03 — truthful free first minute

- **Goal:** newcomer reaches the strongest existing demonstration quickly and
  sees claims supported by delivered assets.
- **Files/modules:** presentation slice: `physis-computer/README.md`,
  `physis-computer/install.sh`, `physis-computer/docs/TOUR.md`,
  `physis-computer/docs/CHANGELOG.md`.
- **Dependencies:** AUD-00 and AUD-03a before public demo claims; AUD-02 before
  publishing new distribution bytes. A private storyboard/viewer test does not
  require a full new public release.
- **Acceptance:** fresh extracted installation outside the checkout can run
  `physis demo`, then `q` and a corpus-backed command. Demo exposes objects,
  caller/test evidence, bounded context and observed mutation/check results.
  Dry-run is labeled a plan, not verification. Checksum behavior is explicit;
  corrupt or absent required manifests cannot silently look verified.
  Remove undelivered signature/cognition-download/broken-badge claims or supply
  separately qualified artifacts. Correct stale example counts from that build.
- **Demonstration acceptance:** use the 60-second storyboard in FAST_PATH; the
  recorded before/after world and CLI/HUD lenses share actual IDs and fixture
  state. Label the bundled scripted edit. Do not present it as autonomous repair.
  A graph screenshot, seven success labels or zero model calls alone do not prove
  user surprise or value; run the bounded five-viewer experiment.
- **Difficulty / leverage:** S / high. Two engineering hours for the first
  presentation attempt after AUD-03a; if exceeded, return a blocker instead of
  starting a HUD redesign. Viewer feedback decides the next presentation change.

### AUD-03a — demo checks mean what they say

- **Goal:** the bundled example compiles and distinguishes structural evidence
  from executed project checks, including an independently failing case.
- **Files/modules:** `apps/physis-cli/src/demo.rs`,
  `apps/physis-cli/fixtures/timeout-client/src/client.rs`,
  `apps/physis-cli/tests/demo_e2e.rs`. Inspect existing bounded command/check
  helpers before choosing the smallest integration; no new check framework.
- **Dependencies:** AUD-00. No public release, customer or HUD branch is needed
  to reproduce the current fixture compile failure.
- **Acceptance:** fixture compiles and tests run before/after the intended edit;
  assertions check behavior rather than `assert!(true)`. A deliberately wrong
  return value that preserves graph edges fails an actual project check.
  Label retained edges as structural evidence; they do not prove preserved
  behavior, complete coverage or an unchanged public API. Structural, executed
  and unavailable checks have separate statuses. Missing Cargo means project
  tests UNVERIFIABLE and cannot produce an aggregate verification PASS.
  The independent caller oracle includes the real example caller. A failed
  required check produces a non-success gate result. The saved world remains
  inspectable after failure, with the actual outcome in its evidence.
- **Proof:** isolated generated fixture, real CLI demo in text and JSON modes,
  actual project tests and the explicit ignored real-binary e2e. A graph-preserving
  wrong-result control must fail the project test; a missing-checker control
  must remain UNVERIFIABLE. Preserve failed logs and exact executable identity.
- **Difficulty / leverage:** S / extreme. Two engineering hours; if the existing
  check seam needs broader changes, return evidence and reseal the card.

### AUD-04 — qualify the exact commercial candidate

- **Goal:** prove the separately built Context MCP artifact, not a stale loose
  binary or a test-key build substituted for customer bytes.
- **Files/modules:** `commercial/physis-context-mcp/Cargo.toml`,
  `commercial/physis-context-mcp/Cargo.lock`, commercial `README.md` and
  `docs/INSTALL.md`; `scripts/commercial-release.sh` changes only
  if a candidate reproduction identifies a defect. Lock regeneration is
  conditional: current locked metadata/tests already passed.
- **Dependencies:** AUD-00. Uses supported Linux x86_64, isolated build/config
  paths and an operator-supplied license matching the candidate verifier.
- **Acceptance:** locked metadata/test/boundary/contract gates; candidate
  version matches canonical version; exact extraction verifies inner/outer
  checksums, ELF target and source manifests. Run supplied-byte acceptance and
  value scripts, unlicensed/invalid/valid license paths and offline probes.
  No issuer/private key/research payload in the archive. Record every
  `UNVERIFIABLE` row instead of treating script exit 0 as complete readiness.
  Preserve current commercial delivery contracts: final serialized text at
  budgets 0/1/10/200/800/1600/3200, truthful over-envelope behavior, bounded
  polling, receipt/world binding and stale/unknown-receipt errors. Reproduce any
  failure before sealing a `server.rs` follow-up; do not reopen completed MC code
  merely because its old checklist is unchecked.
  In particular, current
  `the_delivered_reply_respects_the_requested_budget_or_says_why_not` permits an
  explained over-budget envelope, whereas MC02 requires empty final text when
  metadata cannot fit. Record that contract conflict and its real wire output;
  passing the current unit test alone cannot close MC02. The release-generated
  `INSTALL.txt` also says assembled-content budget only; reconcile delivery
  instructions with the actually accepted boundary contract.
- **Proof:** `check-commercial-contract.sh <exact-binary>`;
  `commercial-acceptance.sh --binary <exact-binary> --license <matching-license>`;
  `commercial-value-check.py --binary ... --license ... --work <new-directory>`.
  Commands are templates for later execution, not instructions to run now.
- **Difficulty / leverage:** S / extreme. Budget: 1–3 hours with warm caches.

### AUD-05 — one supported customer/agent first run

- **Goal:** stranger can install the qualified bytes, install an entitlement,
  connect a real client and verify useful structural evidence.
- **Files/modules:** commercial `docs/INSTALL.md`, `docs/MCP_SETUP.md`,
  `docs/LICENSE_ACTIVATION.md`, `docs/TROUBLESHOOTING.md`;
  `scripts/commercial-agent-host-check.py` only if its actual probe is defective.
- **Dependencies:** AUD-04; any external provider/model call needs the relevant
  assignment and configured access. No synthetic result substitutes for it.
- **Acceptance:** clean config and one documented MCP client, persistent
  initialize/list/call sequence, search → exact ID → explore/impact/context;
  edit/re-observe/change evidence; useful response matches independent source.
  A missing/invalid license fails clearly; installed valid license works offline.
  Record model/host/tool events and actual usage availability.
- **Production seam:** prepare exact edits under `/home/gio/praxis_webapp` for
  `client/src/features/physis/PhysisLandingPage.tsx`, `PhysisDeliveredPage.tsx`,
  their adjacent tests, and relevant `src/__tests__/physisContextSales.test.ts`
  assertions. Recheck these verified paths before the separate assignment.
  Download URL/version/hash must match AUD-14's published bytes. Actual payment,
  production issuance/email and refund proof are operator acceptance, not
  authorized by a local fixture test or by this planning update.
- **Difficulty / leverage:** S / extreme. Budget: 1–2 hours excluding buyer wait.

### AUD-06 — benchmark oracle orientation and build provenance

- **Goal:** caller numbers cannot be produced by a mislabeled build or an
  incorrectly oriented external-call exemption.
- **Files/modules:** `scripts/bench-world-fidelity.py`, a sealed focused harness
  test path if needed, `research/benchmark/oracle.json`,
  `research/benchmark/manifest.json`; corpus is read-only in the harness repair.
- **Dependencies:** AUD-00; independent human/reviewer oracle validation.
- **Acceptance:** external same-name invented pair is rejected; legitimate
  internal same-name pair listed as `(caller, target)` is not falsely rejected.
  Use global expected pairs, not incoming edges of the caller. Pin binary SHA,
  its build-source HEAD/manifest, harness HEAD, oracle hash and corpus hashes.
  Every withheld caller must have outgoing edges queried; missing coverage must
  be explicit rather than silently skipped because it is not a scored target.
  Reject unsupported provenance in marketing comparisons; preserve legacy
  receipts as historical. Do not run `--freeze` to make a changed oracle pass.
  Current fixture rerun has exact denominators and no blended MAY_CALL/CALLS.
- **Follow-up:** a separately sealed holdout corpus/oracle packet covers aliases,
  traits, ambiguity, generics, external names and supported language limits.
  Engine author cannot silently amend expected edges after seeing output.
- **Difficulty / leverage:** S / high. Independently sealed holdouts are M;
  they are not part of the harness-repair worker's allowlist.

### AUD-07 — shared validation and execution status

- **Goal:** documented schema constraints and command outcomes are truthful at
  the actual wire boundary.
- **Files/modules:** `crates/physis-mcp/src/tools.rs`,
  `crates/physis-mcp/src/tools/areas/ops.rs`, `crates/physis-mcp/src/tests.rs`;
  owning executor source is read-only until a shared cause is proven.
- **Dependencies:** AUD-00; serialized after AUD-01.
- **Acceptance:** enforce declared enum/minimum/maximum for the supported schema
  subset with useful argument errors; invalid ports cannot wrap into `u16`.
  Preserve valid zero values where a tool contract allows zero. Nonzero child
  exit and timeout cannot be described as completed successfully; bounded output,
  exit/timeout status and available receipt remain inspectable. Plan-only tools
  stay plan-only. Test both dispatch and a real wire error response.
- **Difficulty / leverage:** S / high. Split validation and execution if owners
  or compatibility review exceed a five-file packet.

### AUD-08 — impact coverage, pagination and honest routing

- **Goal:** “no callers” and “20 callers returned” cannot imply complete safety.
- **Files/modules:** `crates/physis-mcp/src/tools/areas/diff.rs`,
  `crates/physis-mcp/src/tools/areas/workspace.rs`,
  `crates/physis-mcp/src/tools.rs`, `crates/physis-mcp/src/tests.rs`.
- **Dependencies:** AUD-00/01; serialize shared catalogue/test ownership.
- **Acceptance:** fixture with >20 callers reports total/returned/truncation and
  a usable continuation; pagination has no gaps/duplicates. Unknown target and
  observed zero certain callers differ. MAY_CALL candidates/known unresolved
  limits are separate from certain callers. Use existing incident-relations
  paging if sufficient; do not add a second graph query engine.
  Guide describes current resolver residuals. `test.run` explicitly means
  snapshot/kernel invariants; `object.patch` explicitly means recorded mutations,
  not a source edit. Deprecated callable tools are retained unless separately
  migrated with compatibility proof.
- **Difficulty / leverage:** M / high.

### AUD-09 — prove the full native agent loop

- **Goal:** one agent task goes from current evidence to checked changed state
  using existing primitives and explicit host-test execution.
- **Files/modules:** `scripts/demo-native-loop.sh`, `docs/NATIVE_LOOP.md`,
  `crates/physis-mcp/tests/e2e.rs`; read Runtime canonical writes,
  `crates/physis-task/src/solve.rs` and registry adapters before proposing edits.
- **Dependencies:** AUD-01 and reproduced blockers from AUD-07/08.
- **Acceptance:** one persistent session resolves ID, compiles bounded source,
  inspects impact, acts in an isolated/canonical path, runs declared project
  checks, re-observes, inspects delta and retains a truthful outcome note.
  Deliberately failing check blocks completion; stale root/binary and unsupported
  mutation fail clearly. Kernel invariants never substitute for project tests.
  Record where host shell is required. Only propose a new MCP primitive if a
  reproducible indispensable step cannot compose solve/registry/runtime paths.
- **Difficulty / leverage:** M / high. No new runner by default.

### AUD-10 — triage existing HUD work before integration

- **Goal:** salvage useful spatial-shell work without merging a second computer.
- **Files/modules:** read `feat/hud-spatial-shell` against assigned main;
  `apps/physis-hud/src/{focus,object,layout,event,app,views,lib}.rs` and branch HUD
  plans/status; also check `feat/doc-ranking`, `feat/task-outcome-age` and dirty
  detached worktrees for relevant dependencies. No merging on this card.
- **Dependencies:** AUD-00.
- **Acceptance:** commit/slice map with ownership, duplicate behavior, current
  conflicts, reachable actions and baseline tests. Explicitly classify branch
  navigation, left/center/right rendering and object renderer as keep/adapt/drop
  only with evidence; preserve useful unmerged work. Dirty detached tests are
  unresolved evidence, not an automatically approved patch.
- **Difficulty / leverage:** S / high. Budget: 1–2 hours.

### AUD-11 — one HUD vertical slice, using the same objects

- **Goal:** competent user can search → focus → navigate → inspect → pin note →
  edit → check → see delta, with stable object identity and visible failure.
- **Files/modules:** seal separate slices: navigation (`focus.rs`, `event.rs`),
  rendering (existing branch `object.rs`, `layout.rs`, `views.rs`), orchestration
  (`app.rs`, `textbox.rs`, focused HUD tests). Reuse `physis-editor`,
  `physis-notebook`, Runtime and solve; do not create parallel stores or dispatch.
- **Dependencies:** AUD-10 and accepted prerequisite slices; runtime defects get
  their own reproducing card instead of being hidden inside UI work.
- **Acceptance:** real terminal events at 120×36 and 80×24; focus survives lens
  changes, back/forward works, source/relations refer to the same canonical ID,
  notebook reopens its references, deleted objects stay legible. Canonical save
  produces changed bytes/world revision; concurrent edit is refused or visibly
  reconciled, never overwritten silently. Failed/unrun checks stay failed/unrun.
  Search/command help makes each step discoverable. Screenshots alone cannot
  prove reachability or persistence.
- **User proof:** 20-minute scripted session first, then a two-hour real session
  logging forced exits, lost focus, stale source and blocked tasks. Repair the
  most frequent measured interruption; do not remove modules to hit a tab count.
- **Difficulty / leverage:** M / high. Budget: 1–3 days, stop per accepted slice.

### AUD-12 — paid value and claims from paired tasks

- **Goal:** determine whether €49 founder / €79 standard buys repeated value.
- **Files/modules:** buyer/offer packet in `docs/commercial/FAST_PATH.md`;
  product-measurement slice uses `scripts/commercial-agent-host-check.py`,
  commercial `docs/BENCHMARKS.md`, `docs/PRIVACY.md`, `README.md` and a
  task-specific evidence directory. Seal those as separate five-file assignments;
  website copy is a separate assigned patch.
- **Dependencies:** none for discovery and offer drafts. Paid-use proof requires
  AUD-04/05 and its delivery gate; external benchmark marketing requires AUD-06.
  A supported buyer/evaluator is required for customer outcome evidence.
- **Acceptance:** at least three distinct real Rust coding tasks, matched
  the buyer's ordinary tools versus commercial conditions; include ripgrep/read
  as a cheap baseline, and existing free native MCP/Serena where those are the
  actual buying alternative. Same initial files, intent,
  model/config, check oracle and time limit. Score correctness first, then calls,
  time and actual usage/cost where available. Include failed/abstained tasks and
  cases where ordinary tools win. No customer repository upload without consent.
  Buyer independently verifies one useful caller/impact/context result and
  returns for a second task. Missing usage means cost savings are unverified.
- **Commercial acceptance:** ten qualified demos have explicit won/lost/pending
  outcomes, reasons, setup/support minutes and actual paid receipt IDs where
  available. Gift/test licenses are not sales. Ask whether the smaller supported
  paid package is worth paying for when the full native MCP is free. No hidden
  feature removal or misleading exclusivity to manufacture that answer.
- **Decision:** sell supported local structural context if it wins that pain;
  narrow claims if it does not. Keep price until actual conversion/retention
  evidence justifies a change. Do not add more tools as a substitute for proof.
- **Difficulty / leverage:** M / extreme; customer availability is a dependency.

### AUD-13 — integrate, freeze versions, run applicable gates

- **Goal:** one coherent candidate source state with reproducible product gates.
- **Files/modules:** accepted patches; `Cargo.toml`, owned standalone manifests
  discovered by `scripts/check-versions.py`, relevant lockfiles; existing gate
  definitions. No research relocation or unrelated refactor.
- **Dependencies:** release-scoped AUD cards accepted; HUD can ship separately.
- **Acceptance:** reviewed integration diff; version inventory agrees; locked
  dependency resolution; fresh fmt/clippy/workspace tests, commercial tests and
  boundary, native real-wire proof, applicable HUD/invariant/demo gates. Separate
  socket/permission/dependency/space failures from attributable code failures.
  Required UNVERIFIABLE cards keep release blocked. No globally shared store
  override across independent tests. Commit/merge operations belong to a later
  explicitly authorized execution assignment.
- **Version rule:** prefer new 1.2.1 for changed public bytes, with one reviewed
  metadata inventory; never retag v1.2.0 or replace its assets silently.
- **Difficulty / leverage:** M / extreme; gate time is separate from patch time.

### AUD-14 — immutable release and production handoff

- **Goal:** prepared release bytes, public tree and sales delivery agree.
- **Files/modules:** existing commercial/open/stage scripts,
  `scripts/publish-release.py`, candidate archives/manifests and release notes;
  external sales/public-repo changes are separately bounded assignments.
- **Dependencies:** AUD-13 and the selected channel's evidence: AUD-04/05 for
  commercial delivery, AUD-02/03/03a for public Computer. The existing combined
  root publisher requires both artifact sets; a commercial qualification report
  can finish independently. Operator delivery gates apply to their channel.
- **Acceptance before publication:** clean source candidate; final binaries and
  source manifests match; local install/wire/demo/offline checks pass; staged
  public tree passes allowlist/IP gate; publisher dry-run validates immutable
  target/hash constraints. Record whether artifacts have checksums only or a
  real detached signature. License signatures do not sign archives.
- **Acceptance after separately authorized publication:** download exact URLs,
  compare advertised and downloaded hashes, fresh install/client proof, and
  sales page/delivery URL/version agreement. Production paid/gift issuance paths
  have their own operator receipts. No rollback by deleting/replacing a release;
  stop promotion, preserve bad artifact evidence and cut a new version if needed.
- **Difficulty / leverage:** S / extreme; publication is never a worker default.

### AUD-15 — graduate research only for a measured gap

- **Goal:** maintain the long-term moat without turning release work into a
  research programme.
- **Files/modules:** only the module implicated by accepted evidence: solve,
  kernel receipts, retrieval/semantic ranking, cognition, host world or memory.
  Seal one hypothesis, exact allowlist and independent outcome oracle first.
- **Dependencies:** shipping/value gates and a repeated unresolved limitation.
- **Acceptance:** deterministic baseline versus proposed mechanism; failure and
  abstention included; improved task outcome with bounded cost; no commercial
  closure expansion or second world/store. A receipt verifier is justified only
  when current receipts cannot answer a real verification question.
- **Postpone:** broad autonomy, generic workflow DSL, more MCP tools, advanced
  semantic reranking, enterprise administration, dynamic dispatch completeness,
  new HUD design systems and publication-scale studies without a measured need.
- **Difficulty / leverage:** L / low until measured; exact scope remains unsealed.
  Do not pretend this is a ready implementation card. Research data/capabilities
  remain preserved.

## 5. Sealed packet and receipt contracts

The coordinator supplies all values. An unfilled example is not an assignment.

```text
Packet:
  task_id, one_behavior, dependencies_with_receipts
  source_head, source_manifest, clean_absolute_worktree
  read_only_plan_paths, exact_write_allowlist, protected_paths
  owner_function, verified_callers, failing_assertion_or_readonly_question
  binary_paths_and_sha256, build_commands, target_directory
  isolated_store, fixture, config, evidence_directory
  license_mode_and_matching_license_path_without_secrets
  focused_proof, nearest_gate, independent_acceptance_oracle
  time_limit, max_patch_attempts=2, output_bounds, coordinator_contact
  actual_worker_model_and_harness_if_known
  handoff_path, forbidden_actions, review_requirement

Receipt:
  task_id, source_before, source_after, changed_file_hashes
  binary_sha256, artifact_sha256, harness_and_model_identity
  commands, exits, timeouts, decisive_assertions, bounded_log_paths
  verdict=PASS|FAIL|UNVERIFIABLE
  attribution=CLEAN|UNATTRIBUTED
  missing_evidence, source_scope, fixture_scope, known_limits
  world_root, world_revision_before_after, observation_receipt_when_available
  reviewer_identity_and_decision, handoff_status
```

Existing receipt formats may be extended only where a card needs missing
identity fields. Do not build a universal receipt database or a second event
store. `HANDOFF.md` plus the existing receipt file is sufficient.

`PASS` requires the named assertions. `FAIL` means an attributable assertion
failed. `UNVERIFIABLE` means required evidence could not be produced. A dirty
concurrent source failure is `UNATTRIBUTED` until reproduced on its pinned tree.
`NEEDS_COORDINATOR`, `SCOPE_CHANGE` and `STALE_BASELINE` are handoff states; none
is a successful task. `ALREADY_SATISFIED` carries fresh acceptance evidence and
requires coordinator review before closure; it does not authorize a redundant
patch. Empty output, a timeout, a skipped case and a tied result retain their
own meaning.

### Proof commands and exact assertions

Run these only in a later assigned execution, from the candidate root. Coordinator
fills absolute paths and deadlines. Export `CARGO_TARGET_DIR` only to the assigned
target; do not apply one global `PHYSIS_STORE_DIR` to the workspace test gate.
Rebuild a changed binary before exercising it and record its SHA. A test-name
filter that runs zero tests is not proof.

| Scope | Existing command | Required evidence |
|---|---|---|
| Native handler | `cargo test --locked -p physis-mcp --lib` | Named freshness/bounds/error assertions, not just test count. |
| Native process | `cargo build --locked -p physis-cli`; `PHYSIS_BIN="$TASK_TARGET/debug/physis" cargo test --locked -p physis-mcp --test e2e -- --ignored --test-threads=1` | Same qualified binary, persistent requests, correlated wire responses. Set `CARGO_TARGET_DIR="$TASK_TARGET"` for both Cargo commands. |
| Catalogue/wire size | `PHYSIS_BIN="$TASK_TARGET/debug/physis" bash scripts/mcp-surface-cost.sh` | Current advertised tools and serialized surface sizes; not semantic completeness. |
| Commercial source | `cargo test --locked --manifest-path commercial/physis-context-mcp/Cargo.toml` | Freshness, delivery, receipt and license assertions; keep source-test key separate from customer artifact. |
| Commercial dependency boundary | `bash scripts/check-commercial-boundary.sh` | Allowed dependency closure and no forbidden engine/research imports. |
| Commercial exact bytes | `bash scripts/check-commercial-contract.sh "$CANDIDATE_CONTEXT_BIN"` | Catalogue contract from extracted executable; no default loose-binary fallback. |
| Commercial artifact behavior | Supplied-byte acceptance/value commands from AUD-04; existing agent-host check from AUD-05 | Binary SHA unchanged before/after; meaningful licensed responses, failures and remaining UNVERIFIABLE rows. |
| Caller fixture | `python3 scripts/bench-world-fidelity.py --cli "$CANDIDATE_PHYSIS_BIN" --tag <unique-run-id>` | Frozen hashes, actual build provenance and raw expected/certain/candidate pairs. Runs only in the assigned worktree because it writes receipts. |
| Publisher regression | `python3 scripts/test-publish-release.py`; `python3 scripts/test-check-versions.py` | Dirty/wrong-head/version/target/hash/tag cases rejected. |
| Version identity | `python3 scripts/check-versions.py --json`; exact extracted executables' `--version` | Owned packages, BUILD and every delivered binary agree. |
| HUD | `cargo test --locked -p physis-hud --all-targets` | Actual events/rendered buffer and persistence, plus existing width/snapshot/frame checks. |
| Invariants | `cargo test --locked -p physis-tests --test invariants` | Mutation/world/task invariants; not a substitute for project checks in an agent task. |
| Integrated source | `cargo fmt --all -- --check`; `cargo clippy --locked --workspace --all-targets -- -D warnings`; `cargo test --locked --workspace --all-targets` | Final pinned source, final exit codes and attributable failures. Includes license tests; permission reruns identified. |
| Outside-checkout demo | Extracted/installed `physis demo`, `physis q`, chosen corpus command; existing `demo_e2e`/native-loop gates | Useful output and checked world mutation from delivered bytes, not a developer checkout corpus fallback. |

Proposed regression names for AUD-01, not existing tests claimed as implemented:
`observe_refreshes_persistent_session`,
`observe_applies_max_files_without_tombstoning_omitted_files`, and
`observe_refuses_session_root_escape`. Put the meaningful regression in the
existing native test module, with an additional real-process proof if in-process
coverage cannot catch the failure. Do not create a parallel generic MCP harness.

For AUD-06, proposed harness cases are
`invented_external_same_leaf_is_rejected`,
`oracle_internal_same_leaf_is_not_false_flagged`, and
`receipt_pins_build_source_separately_from_harness`. Their sealed test path is
chosen after checking existing Python test conventions. Both positive and
negative controls must run; frozen fixture scores alone cannot test the harness.

## 6. Coordinator loop: ready card → reviewed candidate

```text
function coordinate(queue, current_authorization):
    baseline = capture_refs_status_worktrees_and_artifact_identities()
    preserve_unrelated_changes()
    for card in ready_order(queue, first=buyer_blocker_or_bounded_demo_test):
        if not all_required_dependencies_have_current_receipts(card):
            leave_pending(card)
            continue
        if not authorized(card, current_authorization):
            prepare_reviewable_handoff(card)
            continue
        revalidate_owned_sources_and_dependency_fingerprints(card, baseline)
        if owned_source_or_dependency_changed():
            mark_STALE_BASELINE_and_reseal_after_reproduction()
            continue
        reserve_exact_files_and_one_build_lane()
        packet = seal_one_behavior_with_independent_assertion(card)
        worker_result = run_one_assigned_worker(packet)
        if worker_result.verdict != PASS:
            retain_failure_or_unverifiable_evidence(worker_result)
            unreserve_files()
            continue                    # no automatic third attempt/new worker
        review = independent_review(packet, complete_diff, receipts)
        if review.accepted and fingerprints_still_match():
            if worker_result.handoff_status == ALREADY_SATISFIED:
                close_only_the_assertions_proven_at_this_baseline()
            else:
                place_in_candidate_queue(worker_result)
        else:
            create_one_narrow_followup_or_stop(review)
        unreserve_files()
    integrate_only_accepted_candidates_when_authorized()
    run_applicable_final_gates_at_one_pinned_source_state()
    return_release_handoff_without_publishing_by_default()
```

This is a procedure for the coordinator using existing tools. It does not
authorize automatic agent creation, model selection, commits or publication.

## 7. Smaller-worker loop: one defect, two attempts, then stop

```text
function worker(packet):
    read_local_instructions_packet_and_relevant_skills()
    verify_exact_baseline_paths_binary_license_and_isolation(packet)
    if unexplained_change_inside_allowlist(): return STALE_BASELINE
    dogfood_observe_context_diff_in_isolated_world()
    baseline_result = reproduce_named_assertion()
    if baseline_result.satisfies_card:
        return receipt(verdict=PASS, handoff_status=ALREADY_SATISFIED,
                       fresh_assertion_evidence, no_patch)
    if baseline_result.environment_blocked:
        return UNVERIFIABLE_with_exact_environment_reason()
    trace_owner_and_all_relevant_callers()
    if cause_unknown_after_two_hypotheses_and_two_cheap_probes():
        return NEEDS_COORDINATOR_with_evidence()
    for attempt in [1, 2]:
        proposed_files_and_dependencies = identify_minimum_patch_scope()
        if outside_allowlist_or_requires_unassigned_dependency(proposed_files_and_dependencies):
            return SCOPE_CHANGE_before_mutation()
        make_smallest_patch_using_existing_mechanism()
        if writes_outside_allowlist_or_requires_new_dependency():
            stop_without_reverting_other_changes()
            return SCOPE_CHANGE
        proof = run_focused_assertion_and_nearest_gate()
        if proof.timeout_or_environment_failure:
            return UNVERIFIABLE_with_logs()
        if proof.passes:
            reobserve_and_inspect_actual_changed_objects()
            inspect_git_diff_for_uncommitted_patch()
            verify_limits_and_known_public_contracts()
            remember_truthful_outcome_in_verified_task_world_and_read_back()
            write_handoff_with_actual_provenance()
            return PASS_for_this_card_only
        record_first_decisive_failure(proof)
    return FAIL_with_two_attempt_receipts_and_no_more_work()
```

No test weakening, oracle adjustment, fallback PASS, speculative refactor,
automatic stash/reset/clean, nested delegation or new feature after acceptance.
Documentation-only cards use source/asset assertions; do not manufacture a
failing unit test for prose. Broad gates run after integration unless new evidence
justifies repeating them sooner.

## 8. Existing research-runner loop: derive, classify, repair, learn

Use `scripts/research-loop.sh`; it already derives cards from surfaces. Do not
maintain a second list of all tools/commands, and do not write another runner.

```text
function inspect_machine(candidate_root, required_gate_policy):
    assert_root_binary_and_store_are_candidate_owned()
    receipt = existing_research_loop(static_and_surface_then_applicable_gates)
    rows = parse_actual_receipt_rows(receipt)
    # Current exit code counts FAIL; UNVERIFIABLE can coexist with exit 0.
    for required_card in required_gate_policy:
        if absent(rows, required_card): mark UNVERIFIABLE("missing coverage")
        else if rows[required_card].verdict != PASS: block_that_gate()
    for row in rows:
        if row.verdict == FAIL and row.attribution == CLEAN:
            reproduce_failure_before_assigning_code()
            packet = existing_research_loop_repair_for_that_card()
            coordinator_seals_allowlist_assertions_and_retry_cap(packet)
        else if row.attribution == UNATTRIBUTED:
            request_clean_reproduction_without_destroying_dirty_tree()
        else if row.verdict == UNVERIFIABLE:
            assign_environment_or_manual_proof_only_if_required()
    if same_attributable_signature_recurs:
        existing_learn_path_records_hypothesis_and_traceable_evidence()
        preserve_contradictions_and_require_review_before_behavior_change()
    return_per_product_gate_verdict_not_one_misleading_green_count()
```

Runner repair packets currently land under its checkout's
`research/loops/repair/`. Run future repair/learn work in the assigned candidate
worktree with its own results/store, not the primary inspection tree. Generation
of a handoff command is not execution authorization. Derived surface coverage is
not semantic coverage of every tool.

## 9. Native observation loop: one root, refreshed Runtime

This describes AUD-01's minimal owner repair. Helpers below are conceptual;
prefer existing canonicalization/validation, not a new path framework.

```text
function native_observe(rt_mut, configured_root, args):
    root = canonicalize_existing_directory(configured_root)
    requested = canonicalize(args.path relative_to root, default=root)
    if requested != root:
        return explicit_error("session root immutable; unsupported scan root")
    requested_cap = args.max_files if_present else DiscoverConfig.default.max_files
    cap = checked_positive_usize(requested_cap)  # explicit zero is not a default
    config = DiscoverConfig.default with max_files=cap
    (next_runtime, discovered) = Runtime.observe(root, config)
    # Canonical observe owns append/re-observe/liveness behavior.
    # A truncated scan cannot establish that omitted objects were deleted.
    *rt_mut = next_runtime
    return receipt(root, applied_cap=cap, truncated=discovered.truncated,
                   observed_counts, current_world_revision)

acceptance_session:
    initialize_once(); notify_initialized(); tools_list()
    observe(); search_exact_symbol()
    mutate_fixture(); observe(); search_and_relations_show_new_revision()
    delete_fixture_symbol(); observe(); no_live_deleted_symbol()
    capped_observe(); omitted_preexisting_files_remain_live()
    escape_request(); assert_refused_and_root_world_unchanged()
```

Before sealing, verify whether subtree observation can preserve root-relative
IDs/store today. If it cannot, do not silently add that capability under this fix.

## 10. Agent coding loop: current evidence → checked outcome

Tool names are native MCP names. Commercial uses its separate 12-tool read/context
surface; it does not inherit native write/delegate/task execution by implication.

```text
function coding_task(intent, declared_checks, isolated_root, native_client):
    require_at_least_one_independent_desired_state_check(declared_checks)
    assert_client_root_and_binary_match(isolated_root)
    native_client.observe()
    before_revision = current_revision()
    before_cursor = current_next_observation_cursor()
    hits = native_client.search(intent_target)
    target = resolve_exact_id_or_abstain(hits)
    packet = native_client.explore(query=intent, focus=target,
                                  budget=assigned_budget, include_source=true)
    independently_verify_relevant_source_and_revision(packet)
    impact = native_client.delta_affected(target)
    page_needed_relations_and_keep_CALLS_MAY_CALL_limits_distinct()
    if evidence_insufficient: return UNVERIFIABLE_or_explicit_abstention()
    action = choose_existing_canonical_write_or_registry_delegate_or_solve()
    execute_in_assigned_isolated_root_with_timeout_and_provenance(action)
    edited_root = verified_actual_edited_root(action.receipt)
    checks = run_declared_project_checks_via_existing_solve_or_host_executor(edited_root)
    require_result_for_every_declared_check_or_mark_UNVERIFIABLE()
    review_client = native_client if edited_root == isolated_root else open_verified_client(edited_root)
    review_client.observe()
    invariants = review_client.test_run()   # kernel/snapshot invariants only
    if edited_root == isolated_root:
        inspect_changes_since_seq(before_cursor)
        compare_world_revision(before_revision, current_revision())
    else:
        inspect_candidate_against_recorded_worktree_base()
        never_reuse_another_worlds_cursor_or_claim_original_root_changed()
    inspect_actual_source_diff_and_changed_object_relationships(edited_root)
    reached = action.completed and desired_state_assertion(edited_root)
              and checks.all_declared_ran_and_passed and invariants.pass
    remember(outcome=success if reached else failure_or_unverified,
             actual_actor, bounded_evidence_paths_and_revision)
    return outcome_with_candidate_root_scope_partial_writes_and_failed_unrun_checks()
```

Desired-state assertions and checks are chosen before the worker edits. An action
receipt proves an action occurred; it does not by itself prove task correctness.
Git-ref structural comparison requires commits that actually contain the change;
use working-tree/source evidence for uncommitted patches.
Solve may create its own candidate worktree. Checks then belong to that worktree;
opening a different root requires a correctly identified new client/world, not
retargeting an existing session or reusing its observation cursor. Return the
candidate proof for review, ingest supported task evidence into the original
world through existing mechanisms, and merge only under the assigned authority.

## 11. Benchmark loop: oracle first, builds identified, limits retained

```text
function measure_callers(qualified_binary, build_provenance, frozen_fixture):
    assert_sha256(qualified_binary, build_provenance.binary_sha256)
    assert_source_manifest_and_actual_build_head_are_known()
    verify_frozen_corpus_and_oracle_hashes_without_rewriting_them()
    observe_fixture_in_fresh_isolated_store(qualified_binary)
    expected = {(caller, target) for target in oracle.targets
                                for caller in oracle[target].callers}
    certain = all_queried_CALLS_pairs_in_oracle_target_scope()
    candidates = all_queried_MAY_CALL_pairs_in_that_scope()
    invented = certain - expected
    withheld_violations = []
    for caller, external_names in oracle.withheld:
        require_that_callers_outgoing_edges_were_actually_queried()
        for target in certain_outgoing_targets(caller):
            if leaf(target) in leaves(external_names) and (caller, target) not in expected:
                withheld_violations.append((caller, target))
    TP = size(certain intersect expected)
    precision = TP / size(certain) if certain else UNDEFINED
    recall = TP / size(expected) if expected else UNDEFINED
    record_binary_build_source_and_harness_identities_separately()
    record_raw_pairs_denominators_invented_candidates_withheld_and_latency()
    abstention_gate = FAIL if invented or withheld_violations else PASS
    # Keep the measured scores even on FAIL; do not turn them into a release claim.
    return_result_scoped_to_frozen_fixture_not_general_accuracy()

function compare(before, after):
    require_identical_oracle_corpus_and_compatible_metric_semantics()
    require_verifiable_distinct_build_identity_for_an_improvement_claim()
    preserve_legacy_receipts_but_do_not_promote_unverified_source_provenance()
    show_all_metrics_and_negative_cases_including_abstentions()
```

The pair oracle cannot distinguish two same-name call sites inside one caller if
one is external and one internal. Do not claim call-site proof from pair-level
tests. A new call-site oracle would be a separately justified measurement task.
Reject fabricated edges; imperfect recall is allowed and must be reported.

## 12. Artifact qualification and immutable release loop

```text
function qualify_release(candidate_worktree, expected_version, channels, license_inputs):
    require_clean_pinned_candidate_and_unchanged_source_manifests()
    require_owned_versions_agree(expected_version)
    run_applicable_source_and_dependency_boundary_gates()
    if commercial in channels or computer in channels:
        build_commercial_with_approved_verifier_configuration()
        # Existing open assembler needs this archive for combined checksums.
    if computer in channels:
        build_open_CLI_HUD_using_existing_assembler()
    # Existing assemblers write dist inside their root: candidate root only.
    extract_archives_into_new_qualification_directory()
    verify_outer_and_inner_checksums_target_modes_required_contents()
    verify_versions_source_manifests_and_exact_binary_hashes()
    if commercial in channels:
        run_contract_with_explicit_extracted_commercial_binary()
        run_exact_byte_license_wire_value_and_offline_probes()
    if computer in channels:
        stage_public_tree_from_same_qualified_open_binaries()
        assert_staged_and_archived_binary_hashes_match()
        run_public_allowlist_and_research_source_key_leak_checks()
        run_install_and_demo_outside_engine_checkout()
    preserve_UNVERIFIABLE_and_operator_only_delivery_rows()
    if preparing_existing_combined_root_release:
        require_both_channels_qualified()
        run_existing_publisher_dry_run_at_pinned_commit_without_go()
    return_reviewable_manifest_artifacts_notes_and_remaining_gates()

function publish_only_after_separate_authorization(qualified_release):
    revalidate_refs_source_artifacts_and_authorization()
    refuse_existing_tag_or_asset_hash_mismatch_without_overwriting()
    use_existing_publisher_and_public_repo_handoff()
    download_public_URLs_and_compare_to_qualified_hashes()
    prove_clean_install_and_sales_delivery_version_agreement()
    if any_mismatch: stop_promotion_and_prepare_new_version_correction()
```

Use actual existing script grammar, not these conceptual function names. Never
run a release script in the primary checkout merely to inspect it. Check disk
space first; `/tmp` was nearly full during inspection, so use an assigned target
on the workspace filesystem rather than assuming a temporary Cargo build fits.
No production secret is printed or embedded; a verifier public key is public.
Test-key source regression and production-verifier artifact acceptance are
distinct evidence. Offline proof must block network for the exercised flow;
“no request noticed” is weaker than a network-denied test.

### Git and release order

1. Revalidate main against live GitHub and preserve every dirty/untracked path.
   At the audit baseline main already equals GitHub: there is nothing to push
   wholesale. No force push, orphan-root operation or research-history rewrite.
2. Assign implementation in isolated worktrees from a pinned baseline. Review
   relevant branch work first; HUD/doc-ranking/task-age branches are candidates,
   not automatic merge prerequisites for the commercial product.
3. Accept bounded patches and merge/commit only under the subsequent execution
   authorization. Re-run applicable integration gates after shared-owner changes.
   Keep dirty detached experiments outside the release candidate until proven.
4. Freeze a new reviewed version inventory before the final builds. Proposed
   1.2.1 is a decision, not a version bump made by this plan. Preserve published
   v1.2.0 tags/assets and the existing recoverable history archive.
5. In a clean candidate worktree, use the existing commercial → open assembly
   order: open-release currently expects the matching commercial archive while
   generating the combined checksum manifest. Keep product qualification separate
   even where current scripts package both in one source release.
6. Restage Computer only from qualified bytes after AUD-02. Its seven allowlisted
   docs and corpus already matched public at inspection; sync only reviewed
   changed documents/assets/provenance. Do not copy the private repository.
7. Prepare root publisher dry-run, separate public-repository/asset handoff and
   sales delivery patch. Verify archive names/URLs per channel: the root assembler
   names `physis-<version>-linux-x86_64`, commercial uses `physis-context-...`,
   and public Computer asset naming must be checked rather than assumed.
8. Publish only on a separately authorized instruction after concrete review.
   Download and qualify the served bytes; then promote matching sales download
   links/checksums and complete the operator paid/gift delivery receipts.

Push accepted source commits only, preserving intentional research datasets.
Do not push temporary binaries, task stores, generated scratch results, license
files/private keys, dirty experimental patches or an unreviewed feature branch.
Evidence receipts may enter tracked history deliberately with provenance and
size review; they do not belong there merely because a gate generated them.

## 13. HUD loop: persistent focus, existing mechanisms

```text
function hud_event(event, app):
    intent = existing_textbox_or_keyboard_route(event)
    if intent.selects_object:
        app.focus.navigate(canonical_id)     # existing branch trail/back/forward
    if intent.selects_lens:
        app.lens = chosen_lens               # focus is unchanged
    render_left_navigation_center_object_right_action_evidence(app.focus)
    if intent.pins_note:
        existing_notebook_pin(id=app.focus.id, observed_revision)
    if intent.edits:
        existing_editor_open_same_file_object()
        preview_and_confirm_through_existing_supported_flow()
        canonical_save_with_conflict_detection_or_explicit_refusal()
        reload_current_world_and_keep_focus_or_mark_object_missing()
    if intent.runs_task_or_checks:
        reuse_existing_task_solve_or_bounded_executor()
        expose_status_and_failed_unrun_checks()
        reobserve_and_show_changed_objects_and_receipt()
    if intent.unsupported_or_ambiguous:
        show_exact_available_action_or_explicit_abstention()
    never_report_success_from_a_rendered_label_or_model_prose()
```

Map interruption to owner, not to another tab:

| Interruption | Existing seam | Smallest admissible repair |
|---|---|---|
| Lost object after tab/lens switch | `focus.rs`, branch navigation trail | Route selection through persistent focus. |
| Source/graph/note refer to different things | canonical IDs, object renderer, notebook references | Pass same ID/revision; flag missing/stale references. |
| Textbox promises an action it cannot execute | `textbox.rs`, registry action mapping | Wire existing action or explain unsupported route. |
| Save returns old graph | Runtime canonical write/re-observe, HUD reload | Refresh same world and show save receipt. |
| Test label says done without tests | solve CheckSpec/results, HUD task/outcome view | Show explicit pass/fail/unrun and gate completion. |
| Shell/process forces exit | existing HUD shell/exec/task modules | Fix measured cwd/status/input seam before adding a terminal emulator. |
| Dense tabs bury next action | existing object/actions renderer and help | Make current focus/actions visible; preserve module reachability. |

## 14. Measured small-model/value loop

Use existing commercial host check for the first matched read-only task. Extend
only after its assertions and provenance hold. Four-arm experiments are later
research unless the first paid-value question requires them.

```text
function matched_trial(task, arms, frozen_initial_state, independent_checks):
    # Minimum arms: ordinary read/grep versus commercial MCP.
    # Optional later arms: native MCP and native MCP + solve/counterexamples.
    seal_task_oracle_limits_supported_languages_and_model_configuration()
    randomize_or_counterbalance_arm_order_without_changing_task_content()
    for arm in arms:
        fresh_fixture_and_config_from_identical_initial_bytes()
        result = existing_host_with_only_that_arm_tools_and_fixed_deadline()
        verdict = independent_source_and_project_check_assertions(result)
        record_tool_events_time_calls_output_bytes_actual_usage_if_available()
        record_FAIL_UNVERIFIABLE_abstention_and_host_environment_failures()
    compare_correctness_first_then_cost_time_and_calls()
    if ordinary_tools_win_or_results_are_tied: publish_that_result()
    if no_reported_usage: prohibit_claimed_bill_savings()
    ask_operator_to_record_repeat_use_and_buying_decision_separately()
```

Context heuristic units, selected packet size, assembled text size, serialized
wire bytes and provider tokens are different measures. Keep them separate.
No automatic oracle tuning, generated customer testimonials or contact messages.

## 15. Final acceptance and stop rules

Release readiness has three verdicts: `READY`, `READY AFTER SMALL FIXES`,
`NOT READY`. Assign them per product, never for the entire repository from a
single green test count.

- **Computer:** exact delivered CLI/HUD/corpus; honest integrity/install/docs;
  outside-checkout first-minute demo; no private source/research payload.
- **Native MCP:** fresh same-session world; truthful bounds/errors/coverage;
  canonical writes/delegates; one independently checked coding loop.
- **Context MCP:** separate allowed closure and 12-tool contract; exact-byte
  license/offline/client proofs; usable first run; delivered URL/hash/version
  agreement. Missing required operator/customer evidence stays explicit.
- **HUD:** one stable focus and graph/object/action path; note/edit/check/delta
  reachability; measured repeated-session interruptions, not a tab-count target.

Strong reviewer receives a compact bundle: this plan, one-page candidate summary,
source/build hashes, complete diff, failing baseline assertions, passing patched
assertions, exact artifacts, and the remaining UNVERIFIABLE/operator gates.
Reject unsupported claims, duplicated worlds/pipelines, hidden weak tests,
unnecessary rewrites and accidental product-boundary expansion.

Stop a card when acceptance is met. Stop a worker after two failed patch attempts.
Stop release promotion on mismatched identity or required missing proof. Stop
feature expansion when the customer task or HUD interruption does not require it.
Preserve useful research and all contrary evidence.

## 16. Five moves, ranked

1. Fix the demonstration's verification claim and uncompilable fixture under
   AUD-03a; inspect the real change, then test the first 60-second cut with five
   target developers. Reuse the existing world, canonical write and HUD lenses.
2. Qualify one exact commercial archive and complete a real client/delivery path;
   preserve current grants and record the first actual customer outcome.
3. Run ten qualified buyer conversations/demos, offering the existing €49/€79
   product or one separately scoped €750 investigation pilot when appropriate.
4. Measure seven-day repeat use, outcome quality and total founder effort; make
   the repeated paid task easier before adding a recurring service or feature.
5. Promote the proven free demonstration, then fund one measured HUD/native
   improvement. Broader research and platform expansion follow evidence of use.
