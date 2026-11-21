# Marvelous machinery: gated development plan

> **Status:** planning only, 2026-10-05. Latest operator instruction supersedes
> implementation dispatch. No feature work, release, merge or publication from
> this document. Future execution follows the bounded loops below.

**Goal:** support reviewable AI changes across codebases through portable
investigations, honest evidence expiry and checks bound to exact inspected inputs.
Whole-task token savings require the separate [claim gate](../commercial/VALUE_PROPOSITION.md);
smaller packets alone do not satisfy it.
**Architecture:** canonical observations → derived evidence validity → bounded
packet → native task/check receipt → notebook/HUD/MCP read projections.
**Stack:** existing Rust crates, JSON receipts and stdlib fixture gates. No new
service, model subscription, second graph or background inference.

Strategy: [commercial machinery assessment](../commercial/MARVELOUS_MACHINERY_PLAN.md).
Rules: [LOOPS.md](../../tasks/orchestration/2026-10-05-marvelous-plan/LOOPS.md).
Acceptance: [CHECKS.md](../../tasks/orchestration/2026-10-05-marvelous-plan/CHECKS.md).
Machine-readable ledger: [CHECKS.json](../../tasks/orchestration/2026-10-05-marvelous-plan/CHECKS.json).

## Ordered cards

| Card | Cap | Required predecessors | Future deliverable | Owned seam |
|---|---:|---|---|---|
| M0 | 8 h | none | Supported installed MCP task + exact artifact/license/customer-path evidence, freeze corpus | commercial docs/gates; no payment/provider mutation by worker |
| M1 | 24 h | M0 proof | Strict notebook resume, printable editor config, portable investigation metadata | `physis-notebook`, product `notebook.rs`/`workflows.rs`; thin main/server callers |
| M2 | 24 h | M1 + scoped dependencies | Evidence validity projection and stale reasons | `physis-search/cache.rs` validator reuse; native runtime/HUD consumers, additive notebook fields |
| M3 | 20 h | M1 + declared runnable check | Independently captured check receipt; native task and read-only report consumers | `physis-task`, `physis-exec`, native facade; commercial receipt projection only after boundary review |
| M4 | 16 h | frozen corpus + M1 | Task intent, required evidence slots and selective expansion | `physis-search/context.rs`/`explore.rs`, existing MCP descriptions/presets |

Total: 92 h + 8 h reserve. No dependent implementation begins on an unverified
candidate. Independent fixture authoring may run alongside its producer once the
interface is written. M2/M3 integration share consumers: serialize their writes.

## M0: freeze a task and its baseline

1. Inspect current root and commercial dependency closure; identify the exact
   delivered binary and its verification key identifier. Never load private keys.
2. Confirm installed licensed wire behavior on a synthetic supported repository.
   Keep paid-fulfillment/customer proof distinct from an internally signed evaluation.
   Sales owner checks the inspected Praxis signer-variable mismatch documented in
   [self-review](../commercial/REVIEW_FIXES.md). A healthy signing-key flag cannot
   substitute for a routed fulfillment test using the documented variable.
3. Freeze 20 tasks: 5 direct source, 5 caller/impact, 5 resumed investigation,
   5 adversarial/missing-coverage. Reserve 8 hidden variants after design freeze;
   expose them only to the evaluator. Start with a four-task smoke for iteration.
4. Run present/native workflows on the same questions; preserve full observable
   interaction and final answers. Record missing provider usage as unavailable.
5. Store source hashes, oracle facts, binary SHA, checks and limitations. M0 can
   finish local engineering proof while customer/paid-order proof remains explicitly
   outstanding; sales claims cannot treat those outstanding checks as passed.

## M1: investigation passport

1. Review existing isolated drafts before coding. Rebase onto the current native
   model; fix confirmed gaps with the smallest patch rather than rebuild modules.
2. Strict notebook load distinguishes invalid argument, missing file, malformed
   notebook and environment faults; preserve existing native files and readers.
3. Exact pinned IDs, goal-first deduplication, current source syntax and byte
   evidence; ambiguous/deleted/unparsed targets cannot be fresh.
4. Existing design caps: 64 cells; input256KiB; 8 inspected targets; source 200000B
   per file; ID512B; assembled budget0..8000; payload24KiB/wire text32KiB.
5. Static prompts/resources and printable setup config call the same live reader.
   Confirm argument defaults from design, not a worker's guessed test schema.
6. Add versioned optional passport metadata (repository identity, base revision,
   pin/source digest, schema version) only after old-file compatibility tests.
   Preserve old notebook content; reject wrong-root import; export selected data.
7. Root identity and canonical namespaces remain owned by the canonical model;
   passport metadata carries references to those identities. Define stable repository
   identity once, reuse it for evidence, contracts and later adapters; no identity DB.
8. Independent exact-artifact wire gate, then a real free-model resume and matched
   native task. GUI setup stays unverified until an actual GUI host is exercised.

Existing detailed contract: [notebook design](../commercial/MCP_NOTEBOOK_VALUE_DESIGN.md).
It describes the candidate scope; neither it nor a v1.2 manifest proves shipment.

## M2: evidence validity, reusing dependency validation

Proposed shape, not an existing public API:

```text
evidence_id, scope, source_revision, dependency_fingerprints[],
validation_method, status={current,stale,missing,unknown}, reasons[], coverage
```

1. Enumerate actual dependencies for one pinned source/caller claim. Byte source
   checks and graph/caller checks have different dependencies; caller claims must
   retain graph/scan coverage, not only the callee file hash.
2. Reuse existing `Fingerprint` comparison; project invalidation reasons into
   notebook and graph views. Incomplete dependency enumeration means unknown.
   First require unique scoped dependency keys: current `deps_match` collapses
   duplicate `kind` keys. Reject duplicates; plain repeated `file` kinds cannot
   identify multiple files. Keep scoped validity separate from the cache's
   conservative whole-world fingerprint gate. [Review](../commercial/REVIEW_FIXES.md).
3. Use content identity + explicit scan coverage, not mtime/size agreement as proof.
   Restore identical bytes may restore source validity; test/runtime evidence also
   needs matching toolchain, command and declared environment inputs.
4. Preserve old observations and receipts. A new validity projection supersedes
   their current applicability; it never rewrites history into failure/success.
5. Prove create/edit/delete/rename, same-size edits, corrupted log and offline root.
   Check cold/start/restart consistency and bounded invalidation fan-out.

## M3: independently captured check receipts

Proposed shape:

```text
schema_version, task_id, root_identity, argv[], selected_input_manifest_sha,
binary/toolchain_identity, environment_key_names_and_digests,
start_revision, end_revision, exit_code, duration_ms, output_digest,
origin=captured, result={PASS,FAIL,UNVERIFIABLE}, limits[]
```

A declared check with no run has task state UNRUN and **no captured receipt**.
Reported status remains a separately labelled task record; execution receipts
exist only for actual attempted runs. This schema does not erase UNRUN.

1. Distinguish declared check, agent/operator-reported status and captured run.
   Capture only a previously permitted command through existing native execution.
2. Bind to selected input manifest including relevant build/toolchain/config inputs.
   Hashing environment inputs needs an explicit safe allowlist; never store secrets.
3. Source changes mid-run invalidate a current PASS; timeout, missing executor or
   unavailable output becomes UNVERIFIABLE. Bad exit remains FAIL for its inputs.
   Start/end hashes alone miss edit-then-restore. Use an immutable selected-input
   snapshot or a complete mutation journal; otherwise current applicability is
   unknown. Git HEAD alone cannot identify dirty/untracked execution inputs.
4. Consume one receipt in native task ledger, PR report and notebook; no duplicate
   outcome store. Read-only commercial projection needs a documented allowlist audit.
5. Check planner lists candidate existing commands and selection reasons. It does
   not promise minimal sufficient tests or execute inferred source instructions.
6. Parent owns commercial receipt projection review at M3 disposition, cap 2h
   inside this 20h card. A blocked/failed audit leaves that projection unavailable;
   it does not turn native receipt proof into a commercial feature.

## M4: sufficient context before cheap context

Cap split: 8h narrow implementation + 8h evaluation (4h maximum controlled-run
wall time, 3h founder adjudication, 1h reserve). M0 owns corpus/oracle preparation.
Two arms × 28 tasks × 180s timeout = 2.8h maximum sequential model-turn time;
startup or provider delays must fit the 4h run cap. Partial batches are inconclusive,
not a reason to exceed the 16h card cap.

Rubric: [RUBRIC.md](../../tasks/orchestration/2026-10-05-marvelous-plan/RUBRIC.md).

M4 may begin after M0/M1: current source freshness comes from the existing 1.1.0
byte/source checks. Dependency-wide validity and captured-check slots remain
explicit unknown/missing until M2/M3 proof. Correctly reported gaps can pass an
oracle-gap case; they cannot support freshness, executed-test or safety claims.
Any M4 scenario requiring those richer results additionally depends on M2/M3.

1. Define task intent only for inspected jobs: explain source, identify callers,
   reopen investigation, review provided changed IDs. Do not infer a Git diff from
   an observation sequence number.
2. Required evidence slots expose source, canonical target, caller scope, revision,
   freshness and missing checks. Unknown remains a gap; never fill from lexical
   neighbors just to produce a complete-looking packet.
3. Expand only the insufficient leg. Reuse focused compiler receipts and keep
   detailed ledger on demand; measure delivered envelope and every tool call.
4. Use rubric policy v2: zero critical false claims; 16/20 main and 8/8 hidden PASS;
   no category regression; at least 12 paired-PASS main cases. Predeclare elapsed
   time or complete provider tokens. Require at least 20% median reduction and
   non-worse totals on all 20 pairs and paired-PASS subset. Bytes alone cannot promote.
5. TokenSave separately passes that gate on provider tokens; count schemas,
   history, cache and fallback without double-counting. One 180s per-arm deadline
   contains retries; 56 executions total. External-repo/accepted-change proof gets
   separate budget. Incomplete/noisy evidence stays inconclusive.

## Optional P0: seed-library experiment

[Primitive plan](2026-10-05-primitive-library.md): cap 8h from remaining 160h only
if activated, leaving 152h. Trigger: accepted M4 or an accepted buyer task needs
reuse. Source-backed contracts→existing-operation replay→independent negative
controls→manual disposition. Eight proposed cards; no runtime import or new API
in this documentation pass. Broad 40h procedure-shelf implementation stays gated.

## Later experiments: one demand trigger each

| Candidate | Start trigger | First can-fail proof |
|---|---|---|
| Contract map | 3 concrete buyer cases of unsupported behavioral promises | deleted consumer/check invalidates a pinned promise |
| Change rehearsal | 3 accepted PR reports needing before/after comparison | served checkout unchanged; deleted edge and parse gap distinguished |
| Language-server bridge | 3 qualified tasks blocked by syntax-only method/type resolution | shadowed reference, document-version mismatch, server absence |
| Runtime bridge | repeated paid investigation needs service/check→code link | wrong checkout/PID reuse refused |
| Procedure shelf | ≥5 comparable accepted task runs with reusable preconditions | hidden variant fails when precondition changed; no automatic promotion |
| Multi-root | 2 teams request the same cross-repo handoff | same path/name in two roots remains distinct |
| Doc/issue adapter | 3 qualified tasks blocked by a specific missing artifact kind | conflicting/stale input retains source identity and no instruction is executed |

Counts are chosen decision thresholds, not existing customer measurements.
Stop at each attempt cap; record the negative result before adding another layer.
