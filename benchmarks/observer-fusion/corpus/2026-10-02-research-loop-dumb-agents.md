# Research loop for dumb agents (2026-10-02)

Directive: **remake the computer**. Meaning here: every component of this
machine — 20 crates + 2 apps, 31 CLI verbs, 61 MCP tools, 20 HUD tabs, the
adapters and the live integrations — must be a unit a *weak* agent can test,
and the loop that tests it must be deterministic enough that the weak agent
never has to exercise judgement.

Status: **all four stages landed and have run.** Stages 1–3 in the first pass,
stage 4 (recall) in the second. **Nine defects were found in the loop itself**,
and six of them are one failure wearing different clothes: an instrument that
cannot tell its own breakage from the tree's.

What the loop found in the *machine*, in order of cost:

| # | Finding | Outcome |
|---|---|---|
| 1 | A dead `doc_freq` placeholder line, ~68M wasted allocations per search build | fixed — 245 s → 4 s |
| 2 | Doc ids and terms cloned per token occurrence, ~89M allocations per build | fixed — build 67.5 s → 51.4 s (§0o corrects the earlier overclaim) |
| 3 | `.physis-bundle`, 506 MB of other repos' docs and agent chat exports, walked into the source graph | fixed in `skip_dir`; the log residue needs compaction |
| 4 | `cargo fmt --check` failures named no file, so repair packets had no allow-list | fixed in `physis.diagnose` |
| 5 | `source_graph` built the whole-tree snapshot six times concurrently | fixed — exit 137 → 103/103 in 1.77 s |
| 6 | `demo-interview.sh` runs `target/release/physics` **first**, which was 3 days stale | detected by a new card, not by convention |

The single largest remaining cost is **not** in this plan's scope but is
measured here: `compile_context` builds the BM25 index unconditionally, so
`physis.explore` spends ~51 s tokenising 411 MB of file bodies on every call,
while `physis search` avoids it only because `SearchService` is lazy (§0o).

## 0c. Stage 4 — what "perfect it" actually needed

The missing half was not integration, it was **recognition**: the second
encounter with a defect has to be cheaper than the first. Implementation, in
one line of architecture:

> The ledger *is* the receipts. A FAIL row carries a normalised signature; stage
> 4 groups by it.

No signature index, no side table. A side table is state that can drift from
the evidence, and a signature is three lines of `sed` — paths collapse to
`FILE`, line numbers to `L`, digit runs to `N`. The same defect at
`app.rs:2046:31` and `app.rs:9999:2` reduces to one signature, which is the
entire point.

`--learn` then groups by signature and, for any signature seen more than once,
records a claim through the reasoning stack that already existed
(`physis.hypothesis.propose` + `evidence`). Verified against a scratch store:

```
receipts scanned: 2 FAIL rows, 1 distinct signatures
NEW     error: bounds on T: Drop are most likely incorrect at FILE:L:L (id=hyp-001)
  + supports  20260101T000000Z gate/ci (a.jsonl)
  + supports  20260103T000000Z gate/ci (b.jsonl)
  - contradicts gate/ci last PASS
1 chronic signature(s) processed, 1 new hypothesis proposal(s)
```

The kernel then derived `status: contradicted`, `supporting: 2`,
`contradicting: 1`, coherence 0.5, fitness 0.442 — from evidence polarity, not
from a label the loop wrote. **Both polarity branches matter**: if a signature
recurred but the card's latest outcome is a PASS, the failure is intermittent,
not an unfixed defect, and the loop says so instead of inflating a count.

### Two things deliberately NOT built

- **No `Runtime::compile_trace` integration.** That compiles *decision traces* —
  the cognition cascade's own choices. A gate failure is not a decision, and
  routing one into the other is a category error wearing an integration's
  clothes. Recognition belongs to the loop; learned decisions belong to the
  cascade. Building it would have looked like finishing the plan and been wrong.
- **No hypothesis cloning.** The first `--learn` proposed a fresh hypothesis on
  every run (L8 below), so two claims with identical statements double-counted
  the same evidence and neither could be refuted on its own. `--learn` now lists
  first and appends to the existing id. A store that accumulates near-duplicates
  is a store nobody can read.

## 0d. Loop defects found by running the loop (L6–L8)

| # | Found | Fix |
|---|---|---|
| L6 | A patch sequence spliced the fixed gate loop *inside* the surface family and left the old loop running. `bash -n` passed — valid syntax, wrong semantics — so gates ran `cargo fmt ''` and `just ci ''`. | Rewrote the file whole. A syntax check is not a behaviour check. |
| L7 | `n_unver $((n_unver + 1))` — missing parens, so the UNVERIFIABLE counter never incremented. Silent because **no census card had ever returned UNVERIFIABLE**; the bug was invisible until the count was exercised with `PHYSIS_BIN=/nonexistent` (1 pass + 1 unverifiable = 2). | Fixed, and the path is now exercised. |
| L8 | `--learn` ran a full census before learning, and proposed a duplicate hypothesis on each run. | Standalone mode, and list-before-propose. |

Two of these (L7, L8) are the same defect class as L3/L6: a path that no run
had ever taken. **The cheapest way to find them is to force the path**, which
is why the loop now takes `PHYSIS_LOOP_RESULTS` (scratch receipts) and works
against `PHYSIS_STORE_DIR` (scratch store). Testing the loop must not require
inventing evidence in the real receipt directory.

## 0e. One fix outside the loop

The loop found a real, committed, non-dirty gate failure: `just ci` exit 101 on
rustc's `drop_bounds` lint at `apps/physis-hud/src/app.rs:2046` —
`fn assert_has_drop<T: Drop>() {}` in a test.

That code wanted compile-time proof that `TerminalGuard` implements `Drop`.
A `T: Drop` bound asserts nothing about a concrete type, which is exactly why
the lint fires. Rather than `#[allow(drop_bounds)]` — which weakens the
workspace's lint posture to accommodate a test — the assertion became a `const`
check on `std::mem::needs_drop::<TerminalGuard>()`, the alternative clippy's own
help text names. Same intent, compile-time, and strictly stronger than what it
replaced: it fails at compile time *with a message* instead of at clippy time
with a lint name.

Gate after: `just ci` exit 0, 29 test binaries, 0 failures. The fix rode out in
the other agent's `6d1bc14`, so it is theirs to keep or revert; it is one
statement in one test.

This was the only file outside `scripts/`, `docs/` and `research/loops/` that
this work touched.

## 0f. F5 closed — `physis.diagnose` understands rustfmt

`cargo fmt --check` never prints an `error:` line. It prints
`Diff in <path>:<line>:` followed by an indented, ANSI-coloured `-old`/`+new`
snippet. The parser classified every one of those rows `unparsed` and named no
file, which meant a formatting failure was the one gate failure the loop could
not localise — and therefore the one failure it could not hand a weak agent as a
scoped card.

`diagnose.rs` now parses that shape, with three tests. Two of the three bugs were
in my own first attempt, and both are the kind that pass a code review:

- **Field order.** rustfmt writes `path:line:`, not `path:col:line:`. My first
  version assigned the fields in rustc's order, so `col` swallowed the line
  number and `file` came out empty — the header parsed to `None` and every
  finding stayed `unparsed`, exactly as before. Splitting from the right and
  treating the second field as a column *only if it is numeric* handles both
  shapes.
- **A dead check.** `is_fmt_diff_body` did `trim_start()` and then asked whether
  the line started with a space. After a `trim_start` nothing can start with a
  space, so the context-line branch was unreachable and only the `-`/`+` lines
  were caught. rustfmt *indents* its context lines, which is the actual signal.
- **`-` is ambiguous.** The removed-line marker is also how rustc writes a
  location (`--> path:line:5`). A body check ordered before the location check
  would swallow every rustc location in a combined run. The header and location
  checks therefore come first; the body skip runs only after both miss.

Verified at the tool surface, not just in unit tests:

```
severity_counts: {'error': 1}
  error | /home/.../apps/physis-cli/src/render.rs 456 | rustfmt would reformat this file
```

One located row, zero `unparsed` noise. `cargo clippy -p physis-mcp --all-targets
-- -D warnings` clean; `just mcp-e2e` exit 0.

With F5 fixed, the loop's private rustfmt parser was **deleted**. It existed only
as a workaround, and a workaround that keeps working while the real parser is
broken is precisely how a broken instrument stays invisible. When
`physis.diagnose` cannot name a file, the card says UNDETERMINED and tells the
agent to run `physis diagnose` itself — the honest report.

## 0g. FIXED: the invariants suite no longer gets OOM-killed in parallel

`cargo test -p physis-tests --test invariants` used to die with `signal: 9,
SIGKILL` (exit 137) after printing only passing tests — no assertion ever fails.
Measured on one binary:

| threads | before | after |
|---|---|---|
| 1 | 103/103 ok, 143.77 s | 103/103 ok |
| 2 | 103/103 ok, 92.83 s | — |
| 4 | **exit 137** | 103/103 ok, 2.79 s |
| default | **exit 137** | **103/103 ok, 1.77 s** |

Pinned to one module first: `source_graph` alone (6 tests) went from 172.46 s at
1 thread and **exit 137** at 4 threads, to 6/6 ok in 1.37 s at 4 threads.

**Cause.** All six `source_graph` tests called `discover_tree(&physis_root(), ..)`
and `build_snapshot(..)` themselves — six full-tree walks and six full-tree
snapshots of the same tree, which cargo then runs concurrently. Each of those
snapshots is the whole repo's entity and relation set, so six concurrent copies
is what crossed the ceiling. `DiscoverConfig::default()` already skips `target`
and `.physis-next`, so the walks were not wasteful in *what* they read — only in
being repeated.

**Fix** (`tests/invariants/source_graph.rs`): one `OnceLock<(PathBuf, Snapshot,
usize)>` shared by the six tests. The tests assert six different properties of
the *same* graph, so per-test discovery bought no independence and cost six
times the peak. One scan, six assertions; the graph under test is identical.

Deliberately **not** done: pinning `--test-threads` in the just recipe. That
would make `just ci` green by slowing every future gate and hiding real
parallelism bugs — treating the symptom in the shared gate instead of the cause
in the suite.

After the fix, `just ci` is **exit 0, 29 test binaries, 0 failures** on this
tree. The committed gate had not completed at all since the `diagnose.rs`
change until now.

### Correction to the earlier diagnosis of this same finding

The first version of this section blamed host pressure — `opencode` at ~7 Gi
RSS with swap at 16.2/17 Gi. **That was wrong.** The kill reproduced with 23–25
Gi available. What is proven is only that it is a function of thread count, that
it is `source_graph`, and that it was a kill rather than a test failure. The
killer's identity was never established: there is no applicable rlimit and
`dmesg` is unreadable here. Now that the cause is fixed, the question is moot —
but the wrong attribution was recorded first, which is the part worth keeping.

## 0h. Two measurements I nearly got wrong

Both were caught by measuring instead of explaining, and both are recorded
because the reflex to explain is the expensive one.

**Store growth is not the demo bottleneck.** `demo-h0.sh` went from 197 s to
exceeding 900 s, and `.physis-next/observations.jsonl` has grown to 84k lines /
20 MB, so the obvious story is "every `serve --stdio` replays a bigger log".
Measured: a `physis serve --stdio` roundtrip is **1 s** at load 12. The store
replay is not the cost. Hypothesis dropped.

**A `.physis-next/embed-cache` with 3238 files does not bloat discovery.**
`DiscoverConfig` does not exclude it by name, which looked like it would eat the
4000-file budget and silently truncate the real source graph. `skip_dir` does
list `.physis-next`. Hypothesis dropped.

## 0i. Root cause found: store replay is superlinear in the recent window

`demo-h0.sh` went 125 s → 197 s → 334 s → >900 s across this session. That is
monotonic, and machine load is not, so the store was the suspect all along. My
first dismissal of it was wrong twice over: I "ruled out" `serve --stdio` at 1 s
when I had piped that measurement's **stderr to `/dev/null`**, so a fast failure
read as a success.

The real finding, on `physis search "service:"`, same binary, same box:

| store | time |
|---|---|
| empty (isolated) | **0 s** |
| first 10,592 observations | 3 s |
| first 21,184 | 4 s |
| first 42,368 | 4 s |
| first 63,552 | 5 s |
| first 74,145 | 5 s |
| **last 10,592 only** | **212 s** |
| last 21,184 only | 230 s |
| full 84,737 (the live store) | **245 s** |

Not the store's size: 21k observations cost 4 s from the head of the log and
230 s from the tail. Not an interaction either — the last 10,592 alone is 212 s.
The cost is **intrinsic to roughly the last 10.5k records**, and it is
superlinear: 7× the records, 40× the time.

Ruled out by construction, each by removing the records from the expensive window
and re-measuring:

| removed | kept | time |
|---|---|---|
| `host_scan` (4,634) | 16,550 | 215 s |
| `gone` tombstones (721) | 20,463 | 232 s |
| all `scan_git` (2,040) | 8,552 | 214 s |
| all `rel\|CHANGED_IN` edges | 8,588 | 220 s |

Also ruled out: embeddings (`PHYSIS_EMBEDDER=none` → 240 s) and duplicate
records (the expensive window has *fewer* duplicates, 9%, than the cheap one,
42%). So not the embedder, not the host census, not tombstones, not git edges,
not dedup.

What remains inside those 10.5k records is `scan_fs` (3,402), `scan_rust`
(2,299) and `scan_resolve` (42). **Open.** Narrowing further needs a profile of
one load, not more elimination — roughly 20 ms per record is not explained by
JSON parsing, and the shape of the curve says quadratic behaviour somewhere in
load or query.

### Why this is what broke the demo, and the gate

`physis search` does *not* re-observe the tree: with an empty store the same
command is 0 s. So all 245 s is store load plus query. `demo-h0.sh` makes six
`physis` invocations, so its floor is now ~15 minutes, and `--gate` inherits
that. The store grew during this session — every `demo-h0` run appends a full
host census via `physis host .` — which is exactly the monotonic curve measured
above, and why `just ci` and the loop's gate runs got slower as the session went
on.

The store is derived, gitignored, regenerable state. **It has not been pruned
here** — deleting data is not automated, and which observations to keep is a
judgement call, not a mechanical one. The decision belongs to the operator:
`.physis-next/observations.jsonl` is 20 MB / 84,737 lines, and every command
costs 245 s until it is compacted or rebuilt.

## 0j. `.physis-bundle` was in the source graph — fixed, but it was not the slowness

The last 10.5k records were 96% `.physis-bundle/` files: **1,859 files, 506 MB**
(`.physis-bundle/histories` 1,281 + `docs` 576), containing *other* repos' doc
copies (`physis-pro`, `physis-core`, `physis-global`, `physis-skill`) and agent
chat exports (`claude`, `cline`, `hermes`, `chatgpt`, `agy`) — largest single
item a 90 MB SQLite conversation dump. Nothing in the codebase creates it; it
appears only inside observation logs, so it is external tool state.

Two problems, and only one of them was the performance problem:

1. **It was in the world graph.** `skip_dir` is a name denylist and did not list
   it, so the walker descended into 506 MB of another repo's documentation and
   500 MB of chat logs and added them to physis-next's source graph. Fixed in
   `crates/physis-discover/src/scan.rs` by adding `.physis-bundle`, with a
   tripwire test (`skip_dir_covers_tool_state_but_not_source`) asserting both that
   tool-state names are skipped and that `crates`/`apps`/`tests`/`docs`/
   `corpus`/`scripts`/`research` are **not** — so the list cannot quietly grow
   into a blanket ignore and starve the graph.

2. **It was not the slowness.** A freshly observed store with the bundle excluded
   holds 1,144 file observations instead of ~3,800 and **zero** bundle files, and
   `physis search` on it still costs **233 s**. The fix is right on its own
   merits; it is not the fix for the cliff.

### The cliff, narrowed to the search path

| probe | cost | what it rules out |
|---|---|---|
| `architecture` (loads store, builds snapshot) | **0 s** | load and snapshot build are free |
| `search "zzzznotfound"` (matches nothing) | 182 s | cost is independent of the query term |
| store with all `file:` observations removed | 184 s | file observations are not the cost |
| store with `file:` paths rewritten to nonexistent roots | 167 s | disk reads of those files are not the cost |
| first 74,145 records of the live log | **4 s** | not size — that store is *larger* on every axis |
| `PHYSIS_EMBEDDER=none` | 240 s | not embeddings |

The fast store has 37,587 relation records to the slow one's 8,328, and 2.66 MB
of relation bodies to 0.77 MB. Size, file bodies, path resolution, relation
count, host census, tombstones, `CHANGED_IN`/git edges, embeddings and duplicate
records are all eliminated. Load is free; the whole cost is inside the BM25
search path, and a *prefix* of the same log on the same binary searches in 4 s
while its tail takes 212 s.

**Open.** What in the search path reacts to the recent window. Elimination has
run out; this needs one profiled search, not another round of removing
categories. Listed as §7 step 1 for the third time, with a far smaller search
space than the first time.

## 0k. The cliff was one dead line (and it was never the vectors)

Instrumented `Bm25Projection::build` (`crates/physis-projection/src/lib.rs:58`)
rather than reasoning about it further. The probe, on the live store:

```
total=156.9s  files=2988 symbols=3317  read=231ms  bytes=411,546,793  terms=1,798,570
```

Three numbers answered the question at once:

- **`read=231ms`.** Reading every body from disk is a quarter of a second. Disk
  was never the problem, which also explains why `PHYSIS_EMBEDDER=none` changed
  nothing.
- **`bytes=411 MB`, `terms=1.8M`.** Every `physis` invocation re-tokenised 411 MB
  of file bodies — including the 506 MB `.physis-bundle` corpus and the
  `docs/quarry` copies — into a fresh in-memory index. The cost was therefore
  *per-invocation setup*, which is why it was query-independent.
- **The hot loop did the work twice.** For every token occurrence it ran
  `idx.doc_freq.entry(t.clone()).or_insert(0) = 0` — a placeholder, immediately
  discarded by the `idx.doc_freq.clear()` three lines later. One extra `String`
  clone and one extra `BTreeMap` insert per token, ~68M times.

Deleting that one line:

| | before | after |
|---|---|---|
| fresh store (13.6k observations) | 157 s | **109 s** |
| live store (84.7k observations) | 245–271 s | **4 s** |

Stable across three consecutive runs, and `explore` is 4 s too. `just ci` is
exit 0 / 29 test binaries, `physis-projection` 49 tests and `physis-interview` 26
pass, and the three `bm25_projection` invariants still assert ranked results —
the line was dead, so removing it cannot change behaviour.

The ratio is the lesson: the waste scaled with corpus size, so it was invisible
on small inputs and catastrophic on a real store. The fresh store held 13.6k
observations; the live one held 84.7k, and 6% more data cost 60× more time.

### On the two suggestions in the question

**Saving vectors: already done, and it is not the cost.** Embeddings are cached
in `.physis-next/embed-cache` (3,238 entries), and `PHYSIS_EMBEDDER=none` still
cost 240 s. `read=231ms` says the same thing from the other direction.

**Save and reload the index: right instinct, wrong layer, now unnecessary.**
Persisting the index would have worked — it is deterministic given the snapshot —
but the cheaper fix was to stop doing the work twice. Worth revisiting if the
index ever becomes the cost again.

**grep first: already the documented doctrine, and it was being ignored.**
`physis.guide` tells agents to use their own `read_file`/`grep`/`glob` for raw
reading and text matching — "physis deliberately does not re-offer them" — because
text matching is not a graph question. `physis search` remained the expensive
path, and the demo scripts reach for it. The honest summary is that the loop's
slowest card was slow for a reason no amount of grep-first routing would have
avoided: the tool was slow even when used correctly.

## 0l. The fix held, but the store kept the evidence (and a demo was green for the wrong reason)

Two things surfaced immediately after the cliff fix, both worth more than the fix.

**The 4 s was conditional, not stable.** Measured on the same command, same
binary:

| store | `search` |
|---|---|
| minimal fresh store | 4 s |
| **live store as it stands now** | **115 s** |
| live store minus 9,744 `.physis-bundle` observations | **5 s** |

`.physis-next/observations.jsonl` is **append-only**, so adding `.physis-bundle`
to `skip_dir` stopped new pollution and could not remove the 7,898 `scan_fs`
observations already logged (seq 351–5108 of 91,696 — old, not new writes). The
`removed:file` / `restored:file` tombstone pairs show the lifecycle churning:
present, absent, present again. Those entities stay live and keep getting indexed
on every invocation.

So the durable fix is **store compaction**, not the skip list. The skip list
stops the bleeding; only compaction removes the clot. Compaction is a store
operation over derived, gitignored state — it is not something to do from a
session that also has agents writing.

**A gate was green for the wrong reason.** `scripts/demo-interview.sh` assigns
`target/release/physis` *first* and only falls back to debug when release is
missing. On this checkout `target/release/physis` was 3 days old and did **not**
contain the skip fix added that day — `strings … | grep -c physis-bundle` gives
0 for release and 1 for debug. So `gate/demo-interview.sh` reported `PASS` while
executing month-old code. A gate running the wrong artifact is worse than no
gate: it is green *and* wrong, which is the one combination a reader cannot
detect from the receipt.

Fixed by adding a card rather than a convention, because conventions are what
this loop exists to replace:

```
FAIL  static/binary-freshness/release  stale (2 d old): older than crates/physis-interview/src/cache.rs — a gate using this binary tests old code
FAIL  static/binary-freshness/debug    stale (0 d old): older than apps/physis-hud/src/lib.rs — a gate using this binary tests old code
```

The marker is a **fact**, not a judgement: a binary older than the newest source
file it could contain. It has already earned its keep — `debug` is flagged
because another agent edited a source file after the last build, which is true
and worth knowing. Its signature also recurred, so the receipt carried
`[KNOWN: 1 prior occurrence]` on the second hit: stage 4's recall working
unprompted on real data.

### On the GPU question

This machine has none (`nvidia-smi` absent, no VGA/3D device in `lspci`), and it
would not have helped. The measured hot path is `BTreeMap<String,
BTreeMap<String, usize>>` insertion — ~68M string-keyed inserts per invocation —
plus tokenisation. Both are pointer-chasing, allocation-bound and inherently
serial; a GPU accelerates dense matrix multiplication and cannot touch a string
map. The two measurements say so directly: disk `read=231ms`, and
`PHYSIS_EMBEDDER=none` left the cost unchanged at 240 s, so no embedding is on
this path at all. The ML backends (candle) are behind default-off cargo
features, so the default build contains no model code.

A GPU would matter for dense retrieval and the Tier-3 decision LLM. Neither is
on the search hot path. The levers that would work are all CPU-side: persist the
index instead of rebuilding it, intern tokens or flatten the postings map, and
compact the store.

## 0m. Measured recovery after rebuilding the release binary

`demo-interview.sh` prefers `target/release/physis`, and that binary was 3 days
old (§0l) — so every gate run was partly executing code that predated the skip
and the index fix, and re-scanning the very directory the skip excludes. That is
how 7,898 `.physis-bundle` observations kept entering the log and why bundle
`restored:` markers so precisely balanced the `removed:` ones.

Rebuilding it and re-observing:

| | before | after |
|---|---|---|
| `physis observe .` | 766 s | **14 s** |
| `physis search "service:"` | 115 s | **22 s** (steady, 22–24 s over repeated runs) |
| `just ci` | 31 test binaries | 31 test binaries, exit 0 |

The bundle's `file:`/`dir:` observations fell 7,898 → 3,260 and the store settled
at 91,919. The remaining ~22 s is the residue: those observations are tombstoned
(`removed:`) and so no longer produce entities, but 3,260 of them plus 1,846
`CHANGED_IN` edges are still **in the log**, and the log is scanned on every
build. Only compaction removes that, and compaction is a store mutation over
shared state, so it is not automated here.

### What is still unexplained, stated plainly

The cliff has been reduced but not fully explained, and the honest shape is
**two regimes rather than one cause**:

- a *content* regime — a 21,184-observation window from the log head searches in
  4 s while the same count from the tail takes 230 s;
- a *size* regime — 81,952 observations search in 5 s, 91,696 in 115 s.

Neither store size nor content alone predicts the cost, and dropping any single
class of bundle observation (lifecycle markers: 124 s, `CHANGED_IN`: 109 s, or
rewriting bundle paths out of reach: 120 s) does not recover the 5 s that
dropping all 9,744 does. Nine categories are eliminated and a tenth is not. This
needs one sampled profile of the search path; it is not going to be found by
another round of removal, and pretending otherwise would just produce a tenth
plausible wrong answer.

## 0n. The profile that was needed: 44M tokens, 45.8s of inserts, 21.7s of tokenize

Instrumenting the three phases inside `Bm25Projection::build` (probe removed
afterwards) on the live store:

```
tokenize     = 21.7s
docs_clone   =  0.09s
inserts      = 45.8s
total_tokens = 44,322,247      entities = 8,533
```

So the remaining cost is not a mystery any more, it is arithmetic: **44.3M token
occurrences**, each tokenised into a `String` and then cloned again into the
postings map. Inserts are 54% of the build, tokenisation 26%, and the 0.09s of
body cloning is why disabling it changed nothing.

That names the two real levers, with their measured sizes:

1. **Stop rebuilding the index per invocation** — attacks all ~67s at once. The
   index is deterministic given the snapshot, so it can be persisted or memoised.
   This is the original suggestion, and the profile is why it is the right one:
   the cost is *setup*, not query, which is exactly what a cache removes.
2. **Stop paying twice per token** — attacks the 21.7s. `tokenize` returns
   `Vec<String>` and the loop cloned every element, because a `doc_freq`
   placeholder used to need the same value a second time. That placeholder is
   gone (commit `d119faa`), so the loop now consumes the vector and the key is
   moved rather than cloned: 44M fewer `String` allocations, same semantics.

Lever 2 is committed. Its effect is **not separately measurable on this box**:
six runs across two variants gave 84–128s for identical code while the load
average moved between 4.8 and 15.4. It is kept because it strictly removes work,
not because a number was demonstrated. Recording that beats quoting a speedup
from a confounded measurement.

## 0o. CORRECTION: my headline "80s -> 2-3s" measured a short-circuit

Commit `51ab0de` reported `physis search` going 80 s → 2–3 s. That number was
**not measuring the fix.** It was measuring a different path.

`SearchService` now holds `index: OnceCell<Bm25Index>` and builds lazily, with a
doc comment explaining why: "most queries never reach BM25 at all: `Exact` and
`Substring` return before it is consulted." So `physis search "service:"` returns
at the exact/substring legs and **never builds an index**. My probe confirmed it —
the instrumented `SearchService::new` printed nothing on that path.

What my change actually did, measured on the leg that *does* build
(`physis explore` → `compile_context`, which builds unconditionally):

```
read = 1.35s   (411 MB, 3,070 files)
tokenize + insert = 51.4s
entities = 8,533   distinct terms = 1,798,740
```

| | before | after |
|---|---|---|
| build: tokenize + insert | 67.5 s | **51.4 s** |

and even that delta is indicative only — the store grew between the two
measurements, so the two are not strictly comparable. What is *not* in doubt:
interning doc ids and streaming tokens removed 89M allocations per build and
made it ~24% faster; it did not turn a 67 s build into a 2 s one.

So the honest summary of where search performance stands:

- `physis search <term>` is fast (1–2 s) because of the lazy `OnceCell`, which is
  the other agent's work, not mine.
- `physis.explore` is **68 s**, essentially all of it `Bm25Projection::build`
  inside `compile_context`, which has no lazy path.
- The remaining 51 s is inherent to re-tokenising 411 MB of file bodies on every
  invocation. The two levers that would actually remove it are the ones already
  identified: build once and reuse (needs a sound cache key — see §0m on why
  keying on `snap.revision` is unsafe), or do not index 411 MB of body text at
  all.

Applying the `OnceCell` pattern to `compile_context` would make `explore` skip the
lexical leg the same way `search` does, but that changes retrieval results rather
than just making them faster, so it is a product decision and not a drive-by.

## 0p. Stage 3 proven end to end, and the packet was found self-contradictory

§6 listed "stage 3 has never repaired a real defect end to end" as the last
unproven claim. It is now tested, with a real failure rather than a simulated
one.

**The test.** Introduce a genuine `cargo fmt` violation in a file this work owns
(`pub n_docs:usize,`), run the single gate card, then run `--repair` and follow
the emitted packet step by step.

To make that affordable, `--card=NAME` was added: it runs one gate card and
skips the rest of the family. The gate family is minutes long end to end, which
is exactly why iterating on one gate — or reproducing one FAIL — was impractical
before. Naming a gate card implies `--gate`, because `--card=gate/fmt` silently
running no gate would be the worst way to be wrong.

**What the test found, in the loop:**

1. `emit_repair_card` read `${#ARGV[@]}` on a variable `argv_from_row` only
   assigns when the card is in the gate table. The first card in that receipt
   was a `static/` card, so `--repair` died with `ARGV: unbound variable` and
   emitted nothing. Fixed with a per-card `local ARGV=()` — which also closes a
   worse version of the same bug: without the reset, a card absent from the
   table inherits the **previous** card's argv and silently re-runs the wrong
   command. Tenth defect in the loop, and the first one that only a real
   end-to-end run could reach.

2. **The packet contradicted itself.** It forbade editing any file outside a
   one-file allow-list, then prescribed the repair a weak agent would reach for
   — `cargo fmt --all` — which rewrites the whole workspace. Following the
   packet reformatted **three files belonging to another agent's in-flight
   work**: `physis-discover` (their new `is_skipped_path` export) and
   `physis-runtime` (their `DBG` eprintln). Semantically harmless, but it is
   precisely the scope violation the allow-list exists to prevent, invited by
   the packet itself.

   `cargo fmt` gates now prescribe `rustfmt --edition 2021 <file>` for each
   allow-listed file and say explicitly not to run the workspace-wide fixer. On
   the re-test the allow-list named two files (the agent's unformatted `DBG`
   line genuinely fails the gate too), the scoped repair made it green, and
   **zero files outside the allow-list were touched** — verified by diffing the
   dirty-path set before and after.

## 0q. The unattended turn: the loop drives an agent to green with nobody watching

Every link had been proven individually. This runs them chained, with no human
in the loop, against a real and reversible defect.

**The run.**

1. Precondition asserted first: `cargo fmt --all --check` green, so that one
   injected defect yields a **one-file** allow-list. Without this the packet
   would name whatever the other agent currently has unformatted, and an
   unattended agent would be authorised to edit their files.
2. Injected a real violation (`pub n_docs:usize,` — whitespace only, reversible)
   in `crates/physis-projection/src/lib.rs`, a file this work owns.
3. `scripts/research-loop.sh --card=gate/fmt` — 1 s, and it FAILed with the file
   and line named, annotated UNATTRIBUTED (11 dirty paths) and `[KNOWN: 4 prior
   occurrences]`.
4. `--repair` emitted `research/loops/repair/gate-fmt.md`. Gate before handing it
   on: the allow-list contained **exactly one** file, and it was mine.
5. `physis.delegate` → `opencode run --agent build "read the packet and do
   exactly what it says"` — registry-resolved, bounded args, no human input.
6. The agent read the packet, ran the gate itself, formatted the file, and
   reported `Gate fixed`.
7. `cargo fmt --all --check` → **exit 0**.
8. Containment: diffed the dirty-path set before and after. The only change was
   `RESOLVED: crates/physis-projection/src/lib.rs` — the injected defect, gone.
   **Zero files outside the allow-list were touched.**

**A prediction of mine that was wrong.** `physis_agent::delegate` takes
`timeout_secs`, and both the CLI and the MCP handler hardcode **20**. I expected
that to be far too short for an agent to read a packet and edit a file, and
planned to report the timeout as the blocking defect. It completed inside it. The
bound is not the blocker I assumed; I should have measured before predicting.

This closes the last claim in §6. The loop now: catches a real failure, derives a
bounded packet with a derived allow-list, refuses to name a harness the registry
does not know, hands the packet to a real agent, and confirms the gate — with the
scope containment verified rather than assumed.

## 0. What running it actually found

The loop was written before it could be run, so its first execution was the
acceptance test — and it failed, correctly, on its own extractors.

| # | Found | Verdict after fix |
|---|---|---|
| L1 | `static` card reported "0 dispatched, 61 unroutable" — the arm extractor was pinned to the old single-dispatcher layout. The other agent had split dispatch into `tools/areas/*.rs` mid-flight. | Fixed: authority is now the union of every area's `fn handles`. 61 defs / 61 dispatched / 0 gaps. |
| L2 | `surface` produced 31 FAILs: `unrecognized subcommand 'Observe'`. The Rust variant name is not the subcommand name. | Fixed: verbs are read from the binary's own `--help`. 31/31 pass. |
| L3 | An empty extractor reported 61 real defects. | Fixed: empty extractor ⇒ `UNVERIFIABLE` ("parser is stale, not a finding"). A broken parser must never accuse the tree. |
| L4 | `--repair` ran a fresh census first, then repaired its own empty receipt. | Fixed: repair mode is standalone and selects the newest receipt that *contains* a FAIL. |
| L5 | The first repair card was unusable: 32 diagnostics of which 27 were "Checking <crate>" noise, with the one real error at row 23. | Fixed: keep `severity=error` rows that name a file; state the dropped count; cap the test/blast-radius lists. |
| L6 | A patch sequence spliced the fixed gate loop *inside* the surface family and left the old loop running. `bash -n` still passed — the file was syntactically valid and semantically wrong — so the gates ran `cargo fmt --all --check ''` and `just ci ''` and reported "unexpected argument" / "does not contain recipe ``". | Fixed by rewriting the file whole rather than patching further. **A syntax check is not a behaviour check**: the only thing that caught this was reading the gate errors and noticing the empty argument. |

The lesson worth more than the six fixes: **a loop whose first act is to
report on itself will find its own bugs, and a loop that cannot tell a broken
instrument from a broken tree will confidently accuse a colleague.** L3, L6 and
the dirty-tree annotation below are all that same failure wearing different
clothes.

## 0a. Verified state, end of first session

`bash scripts/research-loop.sh --gate`:

```
static  PASS  61 defs, 61 dispatched, 0 unroutable, 0 undeclared
surface PASS  31/31 subcommands
gate    PASS  invariants, snapshot-smoke, mcp-e2e, and all 4 demo scripts
gate    FAIL  fmt, ci  — both in apps/physis-hud/src/, both UNATTRIBUTED (25 dirty paths)
== 41 checked: 39 pass, 2 FAIL, 0 unverifiable ==
```

`just ci` was separately confirmed green on this tree (exit 0, 29 test
binaries, 0 failures); the two gate FAILs above appeared later, inside the
concurrent agent's edits to `apps/physis-hud/`. Measured demo costs:
`demo-h0.sh` 197–334 s (`physis host .` alone is 147 s), `demo-impossible-computer.sh`
16 s, `demo-interview.sh` 1–7 s, `demo-steal.sh` <1 s.

`--repair` on that receipt: 2 FAIL cards, 1 packet written, 1 skipped because
it re-ran clean. The emitted packet is 58 lines and names
`apps/physis-hud/src/app.rs:2046:31`.

## 0b. The concurrent-edit finding

The first `--gate` run reported 3 FAILs with one shared decisive line, a Rust
parse error, and 25 dirty paths. Minutes later: `just ci` exit 0 with 29 test
binaries green, and the parse error gone — the other agent had been mid-edit.

So gate cards now annotate failures with attribution:

- tree clean → `[tree clean: this failure is the tree's]`
- tree dirty → `[UNATTRIBUTED: N paths dirty — re-run on a clean tree to attribute]`

A PASS is still evidence on a dirty tree; a FAIL is not. Without this the loop
files confident-looking defects against working code, and a stage-3 agent
"fixes" a colleague's in-flight work. `--repair` also re-runs each failing card
and SKIPs it if it now passes — a transient is not a repair job. Observed:
3 FAIL cards in a receipt, 1 packet written, 2 skipped.

## 1. The problem this solves

Three facts, all verified by reading the tree on 2026-10-02:

1. **`physis.test.run` does not run tests.**
   `crates/physis-mcp/src/tools.rs:2356-2385` evaluates
   `physis_kernel::evaluate_invariants` over the in-memory snapshot and
   returns `{passed, observed, entities, edges, invariants}`. Its own comment
   says "real subprocess test execution is P1". The one tool an agent reaches
   for to "run the gate" cannot name a failing test. A loop built on it would
   report green on tests it never ran — the silent-gate failure mode.

2. **The docs already carry per-unit falsifiable rows and nothing runs them.**
   `docs/HALF_WIRED.md` is 15 rows of WHAT WORKS / WHAT DOES NOT / ENTRYPOINT /
   MISSING WIRING / NEXT MINIMUM SLICE. `docs/STATUS.md` has a Proof column per
   capability. `docs/FEATURE_LEDGER.json` has 58 rows. These are the worklist.
   No executable reads them.

3. **Coverage is asserted where it is cheapest and nowhere else.**
   `tests/invariants/docs_freshness.rs` pins the MCP tool count, the crate
   count and the HUD tab count across the docs — good, and reused as-is. It
   counts definitions; it does **not** check that every definition has a
   dispatch arm. That is the one real gap found by reading, and it is a
   classic silent break: add a `def(...)`, forget the `=>` arm, and the tool
   appears in `tools/list` then errors at call time.

## 2. Design rule

> The loop may only assert things **derived from committed truth**: the
> justfile, CI, the demo scripts, the CLI enum, the MCP catalogue, and the
> repo's own stated rules. A card whose expectation cannot be derived is not a
> card — it is a TODO. Inventing expectations produces a loop that cries wolf
> on turn one, and a crying-wolf loop is worse than no loop.

This rule killed one check during design. `docs/FEATURE_LEDGER.json` row
`tests` looks like it should be path-checkable, but 20+ rows are free prose or
quarry-era paths (`tests/test_delta_engine.rs`, `tests/epistemic_p*.rs`) that
do not exist in this repo. Path-existence over that field is a false-failure
generator, so it is **not** a card. The finding it produced — that the ledger's
`tests` field is stale prose from the physis-pro era — is recorded in §5 as an
observation for triage, not as a red gate.

## 3. Loop shape

Four stages. A dumb agent only ever runs stage 3, and only one turn of it.

```
  1. CARD GEN      deterministic, no model
     derive cards from: justfile · ci.yml · scripts/demo-*.sh ·
                        CLI enum Cmd · MCP def() catalogue ·
                        the repo's own declared rules
        │
  2. RUN + VERDICT  deterministic
     PASS · FAIL · UNVERIFIABLE   (never a silent pass)
     receipt per card → research/loops/results/<ts>.jsonl
        │
  3. BOUNDED TURN  the only stage a weak agent touches
     one card at a time: failing argv + clipped failure output +
     physis.diagnose rows + physis.tests_for + physis.delta.affected
     edit scope hard-bounded to the reported file · hard attempt budget
        │
  4. LEARN          verdicts → hypothesis.propose/evidence/accept|reject
     repeat failure patterns → compile_trace CompiledRules, so the
     same diagnosis costs 0 next time (reuses Phase F replay)
```

`UNVERIFIABLE` is a first-class verdict, not a failure and not a pass. It
exists because `docs/IMPOSSIBLE_COMPUTER.md` §2 already enforces "skipped ≠
tied" and `justfile:33` already prints "quarry link not checked locally"
instead of a green tick. A loop that cannot express "not checked" will
manufacture confidence, and confidence is the thing being optimised away.

## 4. Card families (landed)

| Family | Cost | Cards | Expectation source |
|---|---|---|---|
| `static` | ~1 s | 1 card now: MCP `def()` → dispatch-arm coverage | the catalogue itself |
| `surface` | seconds after a build | 31 cards, one per CLI verb (`physis <verb> --help`) | clap's own enum; marker is exit 0 + no panic |
| `gate` | minutes | `cargo fmt --check`, `just ci`, invariants, `just snapshot-smoke`, `just mcp-e2e`, 5 demo scripts | justfile + ci.yml + the demos' own assertions |

`static` and `surface` are the default because they are cheap enough to run on
every loop turn. `--gate` adds the expensive family.

Deliberately **not** cards, and why:

- *Doc-count drift* — already covered by `tests/invariants/docs_freshness.rs`.
  Reusing it beats reimplementing it.
- *CLI verb → match arm* — would be the natural second `static` card, but
  `apps/physis-cli/src/` is being refactored right now (dispatch split out of
  `main.rs` into `cmd_llm.rs` / `cmd_primitive.rs` / `render.rs`), and a check
  that races another agent's edits reports noise. Follow-up, once that
  refactor lands.
- *Ledger `tests` path existence* — false-failure generator, see §2.

## 5. Findings banked

| # | Finding | Severity | Where |
|---|---|---|---|
| F1 | `physis.test.run` never executes a test; reports `passed` from snapshot invariants alone | high | `tools.rs` (now `tools/areas/ops.rs`) |
| F2 | MCP def→arm coverage complete: 61 defs, 61 dispatched, 0 gaps — in both the old single-dispatcher and the new per-area layout | none | verified by direct grep |
| F3 | `docs/FEATURE_LEDGER.json` `tests` field is stale prose/quarry paths, not a checkable field | medium | `docs/FEATURE_LEDGER.json` |
| F4 | Doc tool count (61) only reconciles if the character class includes `-`; 3 `physis.aion.*` tools are hyphenated | low | `tools.rs` defs |
| F5 | ~~`physis.diagnose` cannot parse rustfmt output~~ **CLOSED 2026-10-02** — see §0f | was medium | `crates/physis-mcp/src/diagnose.rs` |
| F6 | `tools/areas/mod.rs` doc comment is stale: it says each area "returns `None` for names it does not handle", but the implementation is `handles(name)` + `dispatch()` returning `ToolOutcome`. Rust doc-comment drift that `docs_freshness.rs` cannot catch — it only checks hand-written counts. | low | `crates/physis-mcp/src/tools/areas/mod.rs` |
| F7 | `physis.tests_for` returns file-level ids ending in `::test`, not test names. For `crates/physis-mcp/src/tools.rs` it returns **1** test, while the real guard tests live in that file's own `#[cfg(test)] mod tests` — so the guarding-test list under-reports exactly where it matters most. | medium | `physis-query` engine `related_tests` |
| F8 | `physis.delta.affected.used_by` for a *file* returns that file's own `DEFINES` edges (20 of them for `app.rs`), which is not blast radius. Real callers would be `CALLS`/`IMPORTS`. The field is technically correct and practically misleading for a repair card. | medium | `physis.delta.affected` |

F1, F5, F7 and F8 are all in code another agent is actively editing
(`crates/physis-mcp`, `physis-query`). They are recorded, not fixed: a fix
authored against a file being rewritten is a merge conflict with extra steps.
F5 is the one that most directly weakens the loop, because it is the path
between a `cargo fmt` failure and a usable allow-list.

## 6. What is NOT verified

- **A full `just ci` has not run since the index-interning change.** The last
  complete green was exit 0 / 31 test binaries, before `51ab0de`. What *is*
  verified for that change: 124 invariants, `physis-projection` 49,
  `physis-interview` 26, `physis-search` 58, `physis-query` 14,
  `physis-config` 12 across four consecutive runs, clippy `-D warnings` clean on
  every changed crate, research loop 34/34.
- **`physis.explore` is still ~68 s**, of which ~51 s is `Bm25Projection::build`
  inside `compile_context`. The lazy-build pattern that makes `physis search`
  fast has no equivalent there. Skipping the lexical leg would change retrieval
  results rather than just speeding them up, so that call belongs to the operator.
- **Store compaction has not happened.** 9,744 `.physis-bundle` observations
  remain in an append-only log; dropping them from a copy took `search` from
  115 s to 5 s. Compaction mutates shared state, so it is not automated here.
- **Stage 3 is now proven end to end** (§0p): a real `cargo fmt` failure was
  caught, packet emitted with a correct allow-list, followed step by step, and
  the gate went green with no file outside the allow-list touched.
- **The delegate launch is verified in both directions** — unregistered id refused
  before any spawn, registered id resolved through `adapter_bin` with bounded
  args and captured output.
- **The unattended turn is proven** (§0q): real failure, bounded packet with a
  derived one-file allow-list, handed to a real agent through `physis.delegate`,
  gate green afterwards, scope containment verified by diffing dirty paths before
  and after. Nothing was left unproven in this chain.
- **Stage 4 was verified against scratch receipts, not real ones.** Every receipt
  written before the signature feature lacks a `sig`, so a real `--learn` reports
  `distinct signatures: 0` until new failures occur. The chronic path was
  exercised on a synthetic history in an isolated `PHYSIS_STORE_DIR`; the real
  store was never written to by `--learn`.
- **`crates/physis-task` was never read**, so whether `physis task new` isolates
  a worktree per card is unknown. Per-card worktree isolation is designed, not
  built.
- Live integration legs (`PHYSIS_LOOP_LIVE=1`, mirrord/cluster) never ran.
- **Index caching remains unbuilt and the obvious key is unsafe.** Keying a
  process-global memo on `snap.revision` broke four `explore` tests, because
  their fixtures build *different* snapshots under the same hardcoded revision
  (`Some("rev-1")`). It would also be inert: `revision` is `None` unless delegate
  results have been ingested. A correct key must cover the file bodies read from
  disk at build time. See §0m and §0o.
- **F1, F6, F7, F8 are recorded, not fixed.** All sit in crates another agent is
  actively editing; a fix authored against a file being rewritten is a merge
  conflict with extra steps.

## 7. Next experiments, in order

1. **Run a full `just ci` and a full `--gate` on a quiet tree.** The index change
   is verified per-crate but not by the committed gate, and the demo cards have
   no fresh green receipt. Cheapest outstanding item; it only needs a box nobody
   else is building on.
2. **Decide the lexical leg's fate for `explore`.** Either give `compile_context`
   the lazy build `SearchService` now has, or stop indexing 411 MB of file bodies.
   Both are product decisions, not optimisations, and both are worth more than
   any further micro-optimisation of the tokenizer.
3. **Set a store retention policy.** The store is derived, gitignored state that
   reached 22 MB / 91,919 observations in one session, with 9,744 rows that no
   longer correspond to anything the walker can see. Compaction at ingestion is
   the structural fix; choosing what to keep is an operator's judgement.
4. **Drive a repair packet through `physis.delegate`** rather than by hand. The
   packet itself is proven (§0p); the registry-gated launch that a real loop
   would use is not.
5. **Close F1**: `physis.test.run` needs a real subprocess leg
   (`physis.action.execute` is the registry-gated pattern) or a rename that says
   what it does. Until then no loop may treat it as a gate.
6. **Run `--learn` against the real store** once real signed failures accumulate,
   and check whether a chronic hypothesis earns its place.
