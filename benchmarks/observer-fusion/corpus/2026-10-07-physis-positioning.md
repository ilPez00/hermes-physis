# Physis positioning — 2026-10-07 (execution plan)

Goal: two public READMEs rewritten so every claim is a runnable receipt.

- `README.md` (physis-next) — **unbeatable**: the full arsenal, organized so the
  combination (world model + receipts + abstention + self-hosting + killed
  hypotheses + solve ladder + measured call resolution) cannot be matched by a
  feature copy.
- `physis-computer/README.md` (the giveaway) — **shocking**: leads with
  true-but-startling facts, each backed by a command the reader can run in
  30 seconds.

Done when: every numeric claim maps to a command or committed receipt; every
fenced command runs verbatim from a clean checkout; no claim violates the
repo's measurement contract (`docs/commercial/VALUE_PROPOSITION.md`).

## Session findings (the last 6 + opencode)

Command-code sessions (`~/.commandcode/projects/home-gio-dev-physis-next/`),
newest first:

1. `a8d1685d` — "resume the last command-code session concerning
   physis-computer" → "push". Gift repo push flow.
2. `30678796` — "make physis computer as cool as possible, apply impeccable
   skill to gui and tui (copy tui framework from aion if needed)".
3. `dd40ebed` (4.4 MB, the big one) — "check github.com/ilPez00/physis-computer
   and what else we can give away for free, or how to give it a gui … make it
   marvelous." Key positioning directives, verbatim:
   - "physis-computer is free software as in free beer, not free speech. the
     binary is gratis, and only that and its dependencies and primitives are
     needed on the repo. the valuable stuff is in physis next.
     **build physis-computer to sell physis-next.**"
   - "Make the first 60 seconds undeniable. Record the TUI. Use asciinema or
     a GIF at the top of the README … Lead with something that works. Open
     with `physis q` …"
   - The demo should prove: world → intent → context → action → filesystem
     change → re-observe → changed world → verify ("much closer to your
     actual 'computer underneath the agent' claim").
4. `33dc5f6a` — "start mcp server dev loops according to plan (marvelous) by
   dispatching worker agents".
5. `366dd904` — LEDGER-AUDIT bounded documentation worker (FEATURE_LEDGER).
6. `b3405833` — MVP-CLIENT bounded implementation worker (licence + MCP e2e).

Opencode sessions (recovered from `opencode.db` WAL + committed docs — the DB
is 15.2 GB SQLite, prompts not cleanly extractable; outputs are the commits):

- Call-resolution session (`904592c`, `1c5964dc`): fixed cross-file callers.
  Frozen corpus: caller recall **0.278 → 1.000**, precision 1.000, F1
  0.435 → 1.000, invented edges **0**, certain pairs 5 → 18. Wrote
  `docs/CALL_RESOLUTION.md`, `docs/RETROSPECTIVE_2026-10-07.md`,
  `docs/CONTEXT_NEXT_AGENT.md`. Mission restated: *"an AI can actually SEE
  and REMEMBER my software project. Not 'a RAG wrapper with an MCP
  interface.'"* Also: edge dedup (19 → 13 edges, set membership +
  provenance accumulation) — counts are now trustworthy.
- Task-outcome pilot (`6bce46a`, `a86b98b`, `5b26417`, `9938ba99`):
  pre-registered, fail-closed harness (10 tests), acceptance verified by the
  harness — never the agent's claim. FUR-05 arm A FINISHED (+46/−4, 263 s,
  four `task_outcome_*` tests); arm B receipt closed the L04 card.
- Solve ladder session (`6b50cc1f`, `1cea80b`, `ff5107cd`): A5 ladder +
  A6 repair evidence + A7 delta score. Caught a real unformatted defect in
  HEAD that prose review missed (`5e93e9a`); its own loop wrote a regression
  test that merged to main (`73f76fb`).
- Earlier commercial sessions (merged via `89a7f459`) produced the docs the
  user named: `docs/HUD_FILTERS.md` (hud), `docs/commercial/VALUE_PROPOSITION.md`
  (value), `docs/commercial/MARKET.md` (market),
  `docs/commercial/LANGUAGE_COVERAGE_2026-10-05.md` (languages), plus
  `DEMO_2026-10-05.md`, `GOLIVE.md`, `LISTING.md`, `FIRST_SALE.md`,
  `SELLER_CHEATSHEET.md`, `DISTRIBUTION.md`.
- Gift staging sessions: `physis demo` seven-stage loop (`4d14fd3b`), styled
  cognition traces (`7048b479`), Go grammar + doc-comment primitives
  (`df1419b3`), corpus 145 → 157 (`0df077ca`), v1.1.1 gift staging
  (`523b475e`), tool-count reconciliation (`1a10c716`), v1.2.0 staged
  (`d64d9a07`, push pending operator OK).

## The positioning decision

**Most valuable + shocking true sentence in the repo:** the decisions are made
by **zero parameters** at **sub-0.1 ms** — the LLM is a replaceable peripheral
at the end of an escalation cascade, and when nothing clears the threshold the
machine **abstains** (p=0.000) instead of guessing. In a market where every
competitor puts the model at the center and inflates a number, this project
demotes the model, publishes its own null results, and **withdrew its own best
marketing number** (11.7×, retracted because the harness measured an
unlicensed binary). Honesty-with-receipts is both the moat (architectural:
append-only provenance; cultural: a portfolio of killed hypotheses) and the
shock.

- physis-next spine: *"The evidence layer for coding agents — a machine that
  shows its work."* Unbeatable = combination moat, not any single feature.
- physis-computer spine: *"No account. No network. No model. It still decides
  — and abstains rather than guess."* Shock = receipts in the hero, not
  adjectives. Funnel per command-code: free beer binary sells physis-next.

Alternatives considered and rejected: commercial-first ("€49, offline, 12
tools" — commoditizing per VALUATION_PLAN §3); demo-first (strong but not
shocking; subsumed).

## Pinned receipts (verify again at edit time — numbers drift)

| Fact | Value | Source |
|---|---|---|
| Version | 1.2.0 (`target/release/physis`, `physis-hud`) | binary `--version` |
| Demo (fresh, 2026-10-07) | 84 entities / **117 relations**; callers 4; packet 313/800 tok; +15 observations; all PASS; memory seqs 203/218 | `physis demo` — README's old transcript (96 relations, 3 callers, +11) is **stale, do not copy** |
| Tier-0 bench | intent 20/20 (ECE 0.0); tool routing 14/15 — the miss **escalated**; command selection 107/108; p50 0.009–0.056 ms; ECE 0.057 on command selection; neural arms **skipped** (weights absent — skipped ≠ tied) | `experiments/cognition/results/system_one_bench.json` |
| Call resolution | recall 0.278→1.000, precision 1.000, F1 0.435→1.000, invented 0; licensed N=32 whole-repo figure **still 0.132, not re-run** — state scope | `docs/RETROSPECTIVE_2026-10-07.md` |
| MCP tools | 62 `def()` entries / 62 dispatch arms / 0 unroutable (engine); giveaway binary: 60 advertised on `tools/list`, 58 distinct, 2 deprecated aliases | `docs/CONTEXT_NEXT_AGENT.md`, gift CHANGELOG |
| Solve ladder | levels syntax→static→types→lint→tests→targeted→property→invariant→model→formal; stops at first failing rung; unreached rung is never evidence; timeout = unverifiable; delta_score = added+removed; e2e: fmt[static] 576 ms + check[types] 7630 ms, delta score 5 (+5 −0), seq 28 | `docs/HANDOFF_2026-10-06.md` |
| research-loop | 48/50 now; 2 FAILs are `binary-freshness` (stale binaries vs uncommitted `tools.rs` — UNATTRIBUTED on a dirty tree); 61/61 recorded on `1cea80b` | `research/loops/results/` |
| Offline proof | `strace -f -e trace=network physis q … \| grep -c connect` → **0** | gift README install section |
| Withdrawn claim | 11.7× retracted (harness measured an unlicensed binary); receipts `2d710ff`, `43a28d9` | `docs/commercial/MARKET.md`, `physis-computer/docs/TOUR.md` |
| A/B honesty | N=2 pre-registered: no finish-rate difference; measured packet value = **precision** (18%/14% smaller diffs at equal correctness) | `research/ab-task-outcome-2026-10-06.md` |

**Trap:** `research/product/runs/` holds a 2.1 GB untracked copy of this
repo — `physis observe .` walks it, so **self-observation counts on this
checkout are not quotable**. Quote the self-contained `physis demo` fixture
instead (`docs/CONTEXT_NEXT_AGENT.md` §traps).

## Implementation steps

### Step 1 (done): receipts pinned above.

### Step 2: rewrite `README.md` (physis-next — unbeatable)
Structure: hero (evidence-layer positioning + three provable shocks as
commands) · prove-it block (abstention, bench, strace-equivalent) · 30-second
try (**fresh** demo transcript) · System One ("the model is a peripheral",
bench receipt, skipped-not-tied stated) · **solve ladder (NEW — currently
absent from the root README)** · full loop · why Physis = moat list (receipts
per token, unknown≠empty, append-only history, one runtime three surfaces,
measured call resolution, self-observation, invariants enforced by tests) ·
research culture promoted (killed router ≈ chance, refuted power law,
withdrawn 11.7×, pre-registered pilot, the fabricated-baseline incident kept
in) · status with current gates · commercial pointer (one paragraph) · crate
table + license/provenance (keep).

### Step 3: rewrite `physis-computer/README.md` (shocking)
Structure: hero ("No account. No network. No model. It still decides — and
abstains rather than guess." + strace receipt inline) · `physis q` verbatim
output (re-run to verify) · shock: zero parameters on the record (bench
table) · shock: abstention p=0.000 · keep the `physis demo` 7-stage
transcript (**fresh**) · **NEW: `physis solve` + verification ladder + delta
score + the caught-defect story** (changelog-only today) · "we withdrew our
own number" as a feature · TUI · corpus (157 primitives, Go grammar) ·
install (checksums, dry-run, reversible backups) · free-vs-paid table (free
links **more** engine; fix the tool-count line to the 60/58/2 frame) ·
honest limits (Linux x86_64 only; lexical embedder `semantic: false`;
conservative caller resolution — method dispatch, instance calls,
imported-package calls, macros withheld) · license (Apache-2.0 corpus;
binaries redistributable as-is, not source-available).

### Step 4: consistency + claim audit (after 2 and 3)
Align tool counts, version links (1.2.0), demo counts across
`physis-computer/docs/TOUR.md` and `docs/commercial/LISTING.md`; grep every
numeric claim in both READMEs and map to the receipts table; delete or
downgrade anything unmapped.

### Step 5: final verification
Run every fenced command in both READMEs from a fresh temp dir; diff outputs
against the receipts; `bash scripts/research-loop.sh`; docs-only diff.

## Rules for executors

- Every numeric claim maps to a command you ran or a committed doc; quote
  version 1.2.0.
- **No token-savings claims** (measurement contract forbids them); `tok` is
  heuristic units — say so wherever `tok` appears.
- UNKNOWN > WRONG: state the honest limits; never invent an edge or a number.
- Touch **only** the assigned README. Another agent is concurrently editing
  this tree (`research/product/runs/`, `tasks/todo.md` are dirty) — do not
  stage, commit, or push anything. Never `git add -A`.
- Do not run `cargo build`/`cargo test` (expensive, /tmp tmpfs risk); only
  `physis`/`physis-hud` CLI commands and `scripts/research-loop.sh`.
- Keep each README ≤ ~380 lines. Markdown only.

## Risks

- Number drift between builds → re-verify at edit time, quote with version.
- Honesty discipline vs "shocking" → shock comes from receipts, never from
  adjectives; the claim audit (Step 4) is the gate.
- Public push of the gift repo is operator-gated (fleet rule) — out of scope.
- Bloat ("everything" → unreadable) → shock-first hero, one command per
  claim, details behind existing docs.

## Specialist routing

- Steps 2 and 3: two `general` subagents in parallel (disjoint files).
- Steps 4–5: follow-up pass after both land (consistency needs both files).

## Execution log (2026-10-07)

- Step 2 DONE — `README.md` rewritten (355 lines) by subagent
  `ses_eea13759affeWRQsKuRw783kAp`: hero + three shocks as commands,
  fresh demo transcript (117 relations / 4 callers / +15), System One
  bench receipt, solve ladder section (new), moat list, research
  culture, current gates (48/50, FAILs attributed), commercial
  pointer. strace receipt pinned from the gift README (strace absent
  on this host — stated, not papered over).
- Step 3 DONE — `physis-computer/README.md` rewritten (380 lines) by
  subagent `ses_eea133a36ffejLHsF9CLmRJlZf`: hero + strace receipt
  (verified 0 connects on three builds), fresh `physis q` output
  (corpus cd6d0070), bench table, abstention trace from
  `physis computer .` in a stranger repo (all p=0.000, ESCALATED),
  fresh demo transcript, solve-ladder section (new), withdrawn-11.7×
  section, TUI, corpus, install, free-vs-paid (62/60/58/2 frame).
- Step 4 DONE (operator pass) — consistency + claim audit found and
  fixed:
  1. `README.md` — `physis-hud .` → `physis-hud` (the binary
     rejects a positional: `error: unexpected argument '.'`).
  2. `physis-computer/README.md` — price regression fixed: the
     subagent had reintroduced the withdrawn "€79 / €50 first 50"
     duality; restored to the unified "€49 founding (first 25
     licences), €79 after".
  3. `docs/commercial/MARKET.md:24-26` — stale live €50 price
     (predated the 2026-10-06 unification, missed by its 21-edit
     sweep) → €49.
  4. `apps/physis-cli/src/main.rs:881` — the `physis computer`
     NEXT-hint printed `physis-hud .` (a command the binary
     rejects) → `physis-hud`. No test pins the string; ships with
     the next rebuild.
  5. Fence balance verified (30/30 balanced); no test pins the
     changed strings; `cargo test -p physis-cli --test demo_e2e`
     compiles and its demo test is `ignored` by design (spawns the
     built binary via `just demo-e2e`).
- Step 5 PARTIAL — research-loop after edits: **49/50, 1 FAIL**
  (`static/binary-freshness/release` — the release binary predates
  the uncommitted `tools.rs`/`main.rs` edits; the debug card
  recovered when the test compile refreshed the debug binary).
- Step 6 DONE (workers, tmux-detached `commandcode -p`) —
  1. **docs sweep** → commit `94dfb8d9` (5 files, 7 edits:
     DEMO.md/STATUS.md `physis-hud .` → `physis-hud`;
     LISTING/FAST_PATH/SELLER_CHEATSHEET 1.1.1 → 1.2.0).
  2. **release re-cut** — `open-release.sh` PASS, fresh
     archives (checksums PASS, demo counts 84/117, callers 4,
     +15 obs, seq 203/218 all match the README receipts),
     but **DRY-RUN FAIL**: the embedded install.sh template
     had drifted from the gift installer and shipped without
     `--dry-run` (`mkdir: unrecognized option '--dry-run'`).
  3. **verify (read-only, 24 commands)** — 11 PASS, 4
     MISMATCH (all honesty defects), 9 pinned/environmental.
     Byte-exact PASS: `physis decide` (state hash
     `5d77c15f336a77a6`), bench tabulation, demo, compile
     (312/800, 14/14), `physis computer .` (75 entities/12
     relations, all candidates p=0.000, ESCALATED, state hash
     `8fb2a157ecd3e843`), MCP wire (60 advertised, 0
     deprecated, protocol 2025-06-18), HUD snapshots,
     `physis solve --help`, gates receipts (48 PASS / 2 FAIL
     binary-freshness).
- Step 7 DONE (operator) — root-cause fixes for all four
  worker findings:
  1. `scripts/open-release.sh` — installer template replaced
     with `cp physis-computer/install.sh` (single source of
     truth; handles both layouts, `--dry-run`, `-h/--help`).
     Release re-cut; archive `./install.sh --dry-run` now
     PASS. Commit `64b8b994`.
  2. Remaining stale sweep — `physis-computer/install.sh` +
     `docs/INTERVIEW_DEMO.md` (hud positional), `RELEASE.md`,
     `GITHUB_INTEGRATION.md`, `PRIMITIVE_LIBRARY.md`
     (1.1.1→1.2.0), `FIRST_SALE.md` (v1.2.0 URLs),
     `MONETIZATION.md` (superseded-pricing banner; body kept
     as dated history). Commit `64b8b994`.
  3. Four README corrections from verifier findings → commit
     `3e851ff9`, each re-verified against the real binaries:
     corpus **ships with the release** (157 .prim installed
     beside the binary via fixed-prefix fallback — not "inside
     the binary"; the empty-dir demo holds for the packaged
     product, verified verbatim, exit 0); `physis cheat
     --track rust` is **budget-bounded** (44 primitives,
     default budget prints 28 and says so); tool arithmetic
     **62 = 60 advertised + 2 deprecated** (dropped from
     `tools/list`, still callable — not "58 distinct");
     compile receipt prose (per-object fused score and rank;
     strategies are aggregate `stats.strategies`).
  4. Staged tree refreshed (`dist/physis-computer-repo`,
     169 files, **0 .rs**, README matches committed source,
     `install.sh --dry-run` PASS in the staged `bin/` layout,
     demo counts PASS).
- **Operator decisions still open:**
  1. **Push `ilPez00/physis-computer`** — staged and
     verified at `dist/physis-computer-repo`, push pending
     operator OK (fleet rule: never automate a public push).
  2. **`just physis-research-loop` full gate** after the
     release rebuild (the earlier 49/50 was pre-rebuild).
- Operator session (2026-10-07, after the 81-commit
  review) — all four directives executed:
  1. **physis-next pushed.** Reviewed the 81-commit range
     (coherent feat/fix/docs/test/chore history, secrets
     scan clean, 217 files), set `main` to track
     `origin/main` (`## main...origin/main [ahead 81]`),
     pushed `e64f58ae..1ed594b9`.
  2. **physis-computer published from `1ed594b9`.**
     Re-staged (169 files, 0 `.rs`, dry-run + demo counts
     verified), overlaid onto a fresh clone of
     `ilPez00/physis-computer`, committed `5165279`
     ("Release v1.2.0: solve verification ladder,
     receipt-exact README"), pushed. **Tag `v1.2.0`
     force-moved** from the stale 09:17 build (sealed at
     `e8ad7361`, old README, pre-fix binaries) to
     `5165279` — the tag had labeled a superseded build
     of the same version; the corrected staging is the
     real v1.2.0.
  3. **Commercial 1.2.0 re-cut — the stale archive was
     exactly the warned-about failure.** `dist/` held a
     `physis-context-1.2.0` archive sealed at `e8ad7361`
     from a dirty tree (~34 commits behind, binary
     `03fbb6ec…`). Regenerated the commercial lockfile
     (`cargo update`: cc 1.5.1→1.6.0, libc 0.2.189→
     0.2.190, zeroize 1.9.0→1.9.1; commit `fee50280`),
     then `PHYSIS_SIGNING_PUBKEY_HEX=<pubkey> bash
     scripts/commercial-release.sh`: fmt, clippy -D
     warnings, license + product tests, commercial
     boundary, release build, secret/path scan, strict
     12-tool contract vs catalogue.snapshot.json — all
     green. Fresh archive sealed at `fee50280`, new
     binary `b0e66325…`, `physis-context 1.2.0`, inner
     checksums OK. Outer `dist/SHA256SUMS.txt` refreshed
     via a final `open-release.sh` run. The 1.1.1-era
     binary was NOT shipped as 1.2.0.
