# Monetization fast lane — TokenSave proof and shock factor

Date: 2026-10-08. Status: **planning amendment only.** No implementation,
publication, payment, customer contact or price change is authorized by this
document. It ranks the revenue path and writes the loops; each step still needs
its own assignment, and the operator-gated steps stay marked `(OP)`.

Companion documents:

- Value and engine cards: [learning, responsive Computer plan](2026-10-08-physis-computer-value-plan.md) (V00–V21).
- Claim gate and measurement contract: [VALUE_PROPOSITION](../commercial/VALUE_PROPOSITION.md).
- Free funnel plan: [FAST_PATH](../commercial/FAST_PATH.md), [positioning plan](2026-10-07-physis-positioning.md).
- Worker protocol: [agent-loops](../../tasks/agent-loops.md); this lane adds [monetization loops](../../tasks/monetization-loops.md).
- Sealed reality: [V00 evidence](../../tasks/handoffs/V00/EVIDENCE.md).

## The one sentence

**The shock sells the trial; the TokenSave proof sells the licence.**

The shock is already built and mostly verified: `physis demo` changes a real
file, re-observes the world, runs the project tests, and the machine abstains
at `p=0.000` when nothing clears the threshold. The TokenSave proof does **not**
exist yet: the only complete matched-host measurement on record has Physis
consuming **more** provider tokens (13,000 vs 6,278) while returning fewer tool
bytes. So the fastest honest path to first revenue is:

```text
public bytes a stranger can install  →  60-second shock  →  trial
        ↑                                                        ↓
   MT-01 public delivery                              MT-03/MT-04 matched TokenSave numbers
        ↑                                                        ↓
   operator push (OP)                                 MT-08 promote or narrow the claim
                                                                 ↓
                                                   MT-10 ten qualified demos → paid licence
```

Everything in this lane either closes a distribution gate, measures the paid
value on qualified bytes, or packages the shock. Nothing new is invented.

## 1. Verified starting facts (re-pin before assigning)

| Fact | Evidence | Consequence for this lane |
|---|---|---|
| Public `physis-computer` repo is published; its v1.2.0 archive is stale | V00 §3: staged `bin/physis` `3c3bd2f9…`, `staged_from=1ed594b9`; GitHub release bytes older than local source | MT-01 must re-stage from qualified build, not copy the old archive |
| `GET /downloads/physis-1.2.0-linux-x86_64.tar.gz` → 404 on praxisweb.xyz | V00 §3 anonymous probe, 2026-10-08 | The free funnel's bytes are not anonymously downloadable today |
| Commercial 1.2.0 archive served byte-identical to the qualified build | V00 §3: body SHA256 `960d344a…` = AUD-14 archive; SHA256SUMS flipped in the same deploy | Paid delivery (Creem path) is ahead of free delivery |
| Only complete matched host task lost on tokens | VALUE_PROPOSITION: 2,907 vs 8,619 tool bytes but 13,000 vs 6,278 provider tokens, 21.302s vs 18.448s | No TokenSave claim exists; measurement must lead, not follow |
| AUD-12 local measurement exists for three matched tasks | `tasks/handoffs/AUD-12/MEASUREMENT.md` | Reuse its task shapes; it lacks complete provider accounting |
| M4 promotion gate is frozen | VALUE_PROPOSITION: 0 critical false claims, ≥16/20 main + 8/8 hidden PASS, no category regression, ≥12 paired PASS, ≥20% median reduction in the predeclared primary metric | A claim may only be promoted through this gate |
| Compact display arm exists, opt-in | `crates/physis-mcp/src/compact.rs`, `PHYSIS_MCP_COMPACT=1` | Encoding saving is measured; host comprehension is not (see FAILURE_MODES #11) |
| Core surface is three tools and has no discovery tool | `tools.rs` `MINIMAL_SURFACE`; `protocol.rs` `tools/describe` | A trial session on `core` cannot page the catalogue in cheaply; fix only with a measured complaint |
| Shock assets exist and are fresh | README: demo 84 entities/117 relations, 4 callers, +15 observations, all PASS; abstention state hash `5d77c15f336a77a6`; `strace … \| grep -c connect` → 0 | MT-07 packages these; it does not build new ones |
| Qualifying commercial bytes passed supplied-mode checks | AUD-14: 15 acceptance + 10 value checks on built and downloaded bytes | MT-03 must run on these exact bytes (or a re-cut with the same rigour) |

Open, unresolved-by-evidence items: does compact output reduce real model
tokens or degrade answers (untested); does session resume reduce orientation
cost on real tasks (untested); does the paid product's smaller reply reduce
whole-session usage (measured once, in the wrong direction).

## 2. Max potential value ranking

Ranked by revenue proximity × differentiation × readiness. "Now" items sit in
this lane; "next" items are already carded in the V-plan; "later" items are
research-gated and must not consume the paid-proof budget.

| Rank | Item | Why it is max value | Cost | Gate |
|---:|---|---|---|---|
| 1 | Public Computer bytes a stranger can install (MT-01) | The shock asset is unusable if the download is 404/stale; this is the only missing byte between the funnel and a viewer | 1–3h + (OP) push | anonymous download → hash → install → demo |
| 2 | Frozen TokenSave protocol (MT-02) | Every token claim, price, and promo needs this oracle; without it the lane guesses | 2–3h | protocol + oracle sealed and hashed |
| 3 | TokenSave diagnostic on qualified bytes (MT-03) | The first honest whole-session number decides the claim, the copy, and the next lever | 3–5h | per-task provider-usage ledger, losses preserved |
| 4 | Session-resume value proof (MT-04) | Repeated orientation is the recurring cost a licence can charge for; resume is the product seam already planned (M1/M3) | 3–5h, shares M1 seam | cold vs resumed on pinned investigations |
| 5 | Shock recording + hero (MT-07) | Converts the trial: refusal + selection + changed file in 60 seconds, no account, no network | ≤4h (V04 cap) | every claim maps to a run command |
| 6 | Claim promotion or narrowing (MT-08) | Turning a number into copy (or refusing) is the revenue decision itself | 1–2h + reviewer | M4 gate or explicit narrowed copy |
| 7 | Five-viewer shock test (MT-09) | Small-sample read on whether the story converts; V05 thresholds | 2h/round, max 2 | 4/5 understand, 3/5 install, 2/5 own task |
| 8 | First-sale loop (MT-10) | Ten qualified demos → paid → day-7 repeat; the actual money step | 12h/week customer slot | paid order + delivered licence + useful task |
| 9 | Compact host-comprehension probe (MT-06) | Cheap can-fail test of the encoding already shipped; a wrong answer is worse than a bigger reply | 2h | tokens down AND answers not worse |
| 10 | Selective expansion slice (MT-05) | Only earns work if MT-03 names insufficiency as a cost driver | 4h, conditional | remeasured after |
| 11 | Execute-exactly-the-program-shown demo (MT-11) | The one capability no competitor can show: the displayed program is the executed program (V09/V10) | shared with V09/V10 | real fixture, displayed fingerprint = executed |
| 12 | Confirmed-learning visible loop (MT-12) | Teach → reuse → counterexample withdraws; the "it learns" shock, only after V14a/b/c honesty | shared with V14 | distinct cases, held-out, withdrawal proven |

Deliberately **not** in this lane: any new feature, pricing change, ad spend,
new provider dependency, or rewrite of the store/context compiler. The lane
uses the qualified 1.2.0 engine and the existing harnesses
(`scripts/commercial-agent-host-check.py`, `scripts/mcp-surface-cost.sh`,
`scripts/research-loop.sh`).

## 3. Work cards

All cards use the bounded worker protocol. Measurement cards carry sealed
oracles; shock cards carry receipts that a stranger can reproduce.

### MT-01 — public Computer bytes closure

**Goal:** an anonymous user downloads, verifies, installs and runs the free
Computer from the public host. **Dependencies:** V01/V02; qualified build.
**Difficulty:** S, 1–3h + operator push. **Leverage:** extreme.
**Files:** `scripts/stage-physis-computer.sh`, `scripts/open-release.sh`,
`physis-computer/README.md`, `dist/physis-computer-repo/`,
`tasks/handoffs/MT-01/RECEIPT.md` (new); praxis download staging only through
the separately authorized V02 seam.

- [ ] Re-stage from the qualified `target/open-release/...` build (AUD-02
      behavior), assert `0 .rs`, README matches committed source, archive
      inner checksums pass.
- [ ] Host the free archive beside the commercial one
      (`/downloads/physis-1.2.0-linux-x86_64.tar.gz` → 200, SHA256SUMS carries
      both lines). Verify from a fresh shell with no checkout: download →
      `sha256sum -c` → `install.sh` → `physis demo` → `physis q`.
- [ ] `(OP)` push/release operations. Record URL/version/SHA256 triple.

**Gate:** anonymous probe → hash equality with the qualified build → demo
prints fresh counts on the downloaded binary.

### MT-02 — freeze the TokenSave evaluation protocol

**Goal:** one oracle every later number is measured against. **Dependencies:**
V00. **Difficulty:** S, 2–3h. **Leverage:** extreme.
**Files:** `research/product/tokensave/tasks.json`, `oracle.json`,
`manifest.json`, `PROTOCOL.md`; reuse `scripts/commercial-agent-host-check.py`
and its receipt shape; `tasks/handoffs/MT-02/RECEIPT.md` (new).

- [ ] Freeze 5 visible + 5 hidden matched tasks across the four supported
      shapes: direct source, caller/impact, resumed investigation, freshness
      after edit. Every task pins: repository revision, oracle answer,
      deterministic outcome check, budget, deadline, model identity and host.
- [ ] Primary metric: complete provider tokens (input + output + separately
      reported reasoning + cache reads/writes), per VALUE_PROPOSITION. Bytes
      are recorded but can never promote.
- [ ] Hidden half is evaluator-held; no worker edits it. Manifest hashes the
      corpus and oracle; harness refuses drift.
- [ ] Record harness SHA256, binary SHA256, and the exact opencode config.

**Gate:** a dry run of one visible task on both arms produces a receipt with
every usage field present or explicitly `unavailable`.

### MT-03 — TokenSave diagnostic on qualified commercial bytes

**Goal:** the first honest whole-session number on the shipping product.
**Dependencies:** MT-02; AUD-14 qualified archive. **Difficulty:** M, 3–5h.
**Leverage:** extreme. **Files:** `research/product/tokensave/receipts/…`,
`tasks/handoffs/MT-03/RECEIPT.md` (new). No product source changes in this
card.

- [ ] Run the frozen protocol against the qualified `physis-context` binary
      (MCP arm) and native `read`/`grep` (control arm), same model, same host.
- [ ] Report per task: correctness (deterministic check), provider tokens by
      field, tool bytes, calls, elapsed, retries, failures. Missing usage →
      `UNVERIFIABLE` for that row, never a pass.
- [ ] Name the single largest cost cause with evidence (orientation re-reads,
      catalogue overhead, receipt ledger, expansion, retries).
- [ ] If the result is a loss, publish it as a loss and route to MT-04/MT-06/
      MT-08 accordingly; do not repeat the task hoping for a better draw.

**Gate:** every row has a denominator and a correctness verdict; the summary
distinguishes `tokens`, `bytes` and `elapsed`; no aggregate without the
per-task table.

### MT-04 — investigation-session continuity (resume) value proof

**Goal:** prove the recurring orientation cost falls when one investigation is
resumed instead of re-derived. **Dependencies:** MT-02; product M1/M3 seam.
**Difficulty:** M, 3–5h. **Leverage:** high. **Files:** product notebook/resume
path through M1; `research/product/tokensave/resume/…`;
`tasks/handoffs/MT-04/RECEIPT.md` (new).

- [ ] Same pinned investigation run twice: cold session vs resumed session
      (notebook refs + passport metadata + captured checks as built by M1/M3).
- [ ] Acceptance: resolved target, source freshness and check provenance agree;
      the resumed run reaches a correct outcome without re-reading the same
      evidence; stale/deleted targets refuse with the reason.
- [ ] Measure provider tokens for the resumed half against the cold half.
      "Same answer, fewer orientation turns" is the claim to test — not fewer
      bytes.
- [ ] Negative control: change the source between runs; resume must detect
      staleness and fall back, not silently answer from the old passport.

**Gate:** correct outcomes in both arms; resumed arm shows the measured delta
in the frozen ledger, or the card reports that resume does not pay yet.

### MT-05 — selective expansion (conditional)

**Goal:** expand only the evidence leg that MT-03 named insufficient.
**Dependencies:** MT-03 naming the cause. **Difficulty:** M, 4h.
**Leverage:** medium until proven. **Files:** `crates/physis-search/src/
context.rs`, `explore.rs`; the frozen protocol.

- [ ] Baseline-reproduce one task where the packet misses a required
      evidence slot and the host compensates with extra reads.
- [ ] Expand that leg only, within the existing budget contract; no new tool,
      no schema change.
- [ ] Acceptance: the task's correctness holds, provider tokens for the pair
      improve, and no other visible task regresses. Promotion through M4 only.

### MT-06 — compact-output host comprehension probe

**Goal:** does `PHYSIS_MCP_COMPACT=1` reduce model tokens without degrading
answers? **Dependencies:** V00. **Difficulty:** S, ≤2h. **Leverage:** medium.
**Files:** `research/product/tokensave/compact/…`;
`tasks/handoffs/MT-06/RECEIPT.md` (new). No source changes.

- [ ] Same 5 visible tasks, `compact=0` vs `compact=1`; correctness judged by
      the frozen deterministic checks, not by prose.
- [ ] Record provider tokens and wrong/omitted answers separately.
- [ ] A token drop with a correctness drop is a FAIL, and the encoding stays
      off by default. A wash is reported as a wash.

### MT-07 — shock recording pack

**Goal:** 60 seconds a stranger can reproduce. **Dependencies:** MT-01 for the
public arm; V04. **Difficulty:** S, ≤4h first cut. **Leverage:** high.
**Files:** `physis-computer/README.md` hero, `docs/DEMO.md`,
`physis-computer/docs/TOUR.md`, recording under `dist/` or docs assets,
`tasks/handoffs/MT-07/RECEIPT.md` (new).

- [ ] Record in this order: `physis q` cited answer in an empty dir → `physis
      computer .` abstention (`p=0.000`, ESCALATED, reason line) → `physis
      demo` through stage 6 with executed tests → `:cog <line>` trace resolving.
- [ ] Every on-screen number must come from the installed binary; re-record
      rather than patch a transcript. State honestly that `strace` is pinned
      from a committed receipt where the host lacks strace.
- [ ] README hero links the recording; the three-command quick start is above
      the fold; the withdrawn 11.7× stays withdrawn and visible as the trust
      hook.

**Gate:** fresh extract of the public archive reproduces every shown command
verbatim; no claim in the hero lacks a run command.

### MT-08 — claim promotion or narrowing

**Goal:** decide the copy from the numbers. **Dependencies:** MT-03 (and
MT-04/MT-06 if used). **Difficulty:** S, 1–2h + reviewer. **Leverage:** high.
**Files:** `docs/commercial/VALUE_PROPOSITION.md`,
`docs/commercial/LISTING.md`, `docs/commercial/FAST_PATH.md`,
`physis-computer/README.md` (claim sections only),
`tasks/handoffs/MT-08/RECEIPT.md` (new).

- [ ] Run the M4 gate over the MT-02 hidden half. Pass → draft the measured
      claim with scope, denominator and host. Fail → write the narrowed,
      still-true claim (precision, exact ids, evidence continuity, offline)
      and keep the token claim labelled unproven.
- [ ] No "up to", no extrapolated multipliers, no token claim from bytes.
- [ ] Reviewer checks every numeric sentence against a receipt; unmapped
      numbers are deleted.

### MT-09 — five-viewer shock test

**Goal:** distinguish surprise from confusion. **Dependencies:** MT-01/MT-07.
**Difficulty:** S, 2h/round, max 2 rounds. **Leverage:** high.
**Files:** `tasks/handoffs/MT-09/VIEWERS.md`, `RECEIPT.md` (new). `(OP)` for
contacting viewers.

- [ ] Thresholds (V05): 4/5 describe what changed, 3/5 install unaided, 2/5
      attempt their own repository. Denominator reported; gifts are not sales.
- [ ] Record every install failure verbatim; fix at most one obstacle between
      rounds.

### MT-10 — first-sale loop

**Goal:** verified payment → delivered licence → useful task → day-7 repeat.
**Dependencies:** MT-03/MT-08; V02; Creem flow. **Difficulty:** M, customer
slot. **Leverage:** extreme. **Files:** `tasks/handoffs/MT-10/FUNNEL.md`,
`RECEIPT.md` (new); commercial docs after the fact. `(OP)` for pricing,
payment, messages.

- [ ] Verify the existing paid path once on an authorized sale: price/product,
      idempotent webhook, production licence, email/download, anonymous
      correct artifact, install, first useful task.
- [ ] Ten qualified demos using the free shock + the honest claim. Record
      wins, declines, objections, founder/support minutes.
- [ ] Day 7: one repeated task and the reason to retain or abandon. A download
      or a star is not willingness to pay.
- [ ] Decision rule: no purchase after ten qualified demos → reassess
      problem/offer before expanding; purchases without repeat use → one
      activation/retention repair; three paid repeat users → test repeatable
      acquisition.

### MT-11 — engine shock A: execute exactly the program shown

**Goal:** the public "watch it run" moment with no model. **Dependencies:**
V09/V10. **Difficulty:** shared with V09/V10. **Leverage:** high.
**Files:** per V09/V10; fixture + snapshot under `apps/physis-cli`.

- [ ] One fixture journey: resolve exact symbol → read observed callers →
      compile source evidence → pin; the displayed program fingerprint equals
      the executed program fingerprint.
- [ ] Ship as `--snapshot` output plus a short script so CI can assert it.

### MT-12 — engine shock B: confirmed-learning visible loop

**Goal:** teach once → reuse without a model → change the condition → watch
eligibility withdraw. **Dependencies:** V14a/V14b/V14c. **Difficulty:** shared
with V14. **Leverage:** high after honesty work.

- [ ] Reuse the V14 confirmation writer; the demo must show distinct cases,
      a held-out case, a counterexample and the withdrawal — never a seeded
      counter or a promotion badge.
- [ ] Any demo using a synthetic pre-promoted rule is labelled synthetic
      (the existing L00 rule).

## 4. Budget and order

The 2026-10-08 value plan's week split stands (20h engineering, 12h customer,
8h support/evidence). Within it this lane claims:

| Slot | Hours | Cards |
|---|---:|---|
| Free funnel / shock | 4 | MT-01 (1–3h), MT-07 (≤4h, may split across two days) |
| TokenSave proof | 8 | MT-02, MT-03, then MT-04 or MT-06 by MT-03's finding |
| Claim and copy | 2 | MT-08 |
| Customer slot | 12 | MT-09 + MT-10 |
| Support/evidence | 4 | receipts, hashes, re-pinning after each operator step |

MT-05, MT-11, MT-12 are conditional on MT-03's named cause, V09/V10 and V14
readiness respectively; they never delay MT-01/MT-07/MT-10.

**Order is strict for these pairs:** MT-02 before MT-03 (no number without an
oracle); MT-03 before MT-05 and MT-08 (no lever or claim without a diagnosis);
MT-01 before MT-10 (no demos on bytes a buyer cannot download); MT-08 before
any copy change.

## 5. Gates and honest limits

- **TokenSave promotion** requires the M4 rubric in full, on the hidden half,
  with complete provider usage. Bytes, calls and latency are secondary and
  cannot substitute.
- **Shock claims** require a fresh extract of the public archive and a run
  command per claim. `tok` remains a heuristic content unit — never a model
  token, never a bill.
- **UNVERIFIABLE stays visible** in denominators: missing provider usage,
  missing strace, missing platform, timeout. Skipped ≠ tied.
- **The withdrawn 11.7× stays withdrawn** and is used as the trust hook, not
  retold as a win.
- **One writer per file group** across all lanes; the shared checkout has 15+
  worktrees and 4+ active branches. Re-pin HEAD and status before every
  assignment; preserve other lanes' dirty files.
- **No fake evidence:** no seeded learning counters, no hand-written PASS, no
  recorded demo whose transcript cannot be regenerated.

## 6. Success criteria for the lane

1. Anonymous download of `physis-1.2.0-linux-x86_64.tar.gz` returns 200 with
   the qualified hash, and a fresh extract installs and demos. (MT-01)
2. A frozen, hashed matched-task protocol exists and a full provider-usage
   ledger exists for the qualified commercial bytes. (MT-02/MT-03)
3. One of three outcomes is recorded per number: promoted through M4,
   narrowed honestly, or published as a loss. (MT-08)
4. A 60-second recording exists whose every claim is a command. (MT-07)
5. Five viewers tested; first paid order attempted with the recorded funnel;
   day-7 retention checked. (MT-09/MT-10 — operator gates apply)

---

## 7. Addendum 2026-10-20 — crystallized economics and launch-week money

Planning only; gates above stand unchanged.

**What is sellable now, in evidence order.** (1) Evidence continuity: resume an
investigation with fresh/stale/deleted markers and receipts. (2) Operations
audit: measured agent token/call spend per task, once the ledger exists.
(3) Compounding: marginal cost falling on repeated task classes — **not
sellable** until S20 and a paired verdict. (4) Substrate: one world shared by
humans, agents and CI — horizon, not an offer. Lead with 1, never with 3.

**Launch-week money, ranked by speed.** (a) Fulfillment proof: one real
purchase end to end, pay → key → download → install → MCP run. This blocks
launch; a broken checkout makes every traffic hour worthless. (b) Founding
batch, 25 × €49 = €1,225 ceiling, scarcity real, no discount to buy stars.
(c) Two paid evidence audits for design partners at €150–500 (hypothesis, not
published price). (d) Team bundle €249. (e) Agent-ops audit at €500+
(hypothesis) only after the ledger ships. Realistic 30-day range €500–1,500,
high variance; the milestone is the first real purchase, not the ceiling.

**Standing restraint.** The only complete matched-host measurement has Physis
consuming more provider tokens (13,000 vs 6,278) while returning fewer tool
bytes. No token-savings headline until MT-02/MT-03 pass in full; bytes, calls
and latency remain secondary. Crystallization claims require `replay_rate > 0`
on real sessions and a recorded paired scan, pass or negative.

Details and the priority ranking:
[2026-10-20 max-shock release](2026-10-20-physis-computer-max-shock-release.md)
§19–§21; horizon model:
[2026-10-20 biggest physis](2026-10-20-biggest-physis.md).
