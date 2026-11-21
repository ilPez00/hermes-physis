# Observer fusion on a real corpus — measured 2026-11-20

Engine: `physis-core` 0.2.0 (post `0cc7e7c` + reserve), production embedder
**bge-small** (semantic ONNX, `PHYSIS_MODEL_DIR`), embedder-aware default
floor (0.6 semantic).

## Method

- **Corpus (engine-visible)**: 24 real physis-next plan documents
  (`docs/plans/2026-09-24…2026-10-09`), chunked and retrieved by the engine's
  own pipeline.
- **Held-out (observer-only)**: 3 further real plan docs the engine can never
  retrieve — `2026-10-08-symbolic-composition-plan.md`,
  `2026-10-09-gate-ambient-state.md`,
  `2026-10-09-language-organs-execution.md`. Observer claims are **verbatim
  excerpts** from these docs; a query is answered only if the packet contains
  a distinctive phrase from the held-out doc.
- **Noise**: verbatim excerpts from genuinely unrelated real repositories
  (physis-nas README, physis-playground README), attached to every query.
- **Control**: one query answerable from the corpus itself
  (quarry-migration plan), no relevant observer claim.
- Checks are packet-level phrase presence; gate relevances recorded from the
  engine's own receipts (`observer_gated`).

## Result 1 — without reserve, fusion is structurally inert on rich corpora

Budget 600 and 1200, no reserve: `observer_selected = 0` on every query.
The engine's greedy budget-capped retriever fills any budget
(596/600, 1195/1200 tokens), leaving zero slack — and the fill-slack-only
tier contract (observer content never displaces engine evidence) correctly
admits nothing. Not a gate failure: the contract working as designed. The
win case requires slack, and a rich corpus at a sane budget leaves none.

## Result 2 — the gate separates cleanly on real text (bge-small)

Raw-relevance probe (floor 0.0), query↔snippet cosine:

| claim | relevance |
|---|---|
| gate-ambient (relevant, held-out) | **0.6928** |
| symbolic-composition (relevant, held-out) | **0.7314** |
| language-organs (relevant, held-out) | **0.6976** |
| noise-nas | 0.3971–0.5356 |
| noise-playground | 0.4174–0.5534 |

Relevant ≥ 0.69, noise ≤ 0.55 on real text — the 0.6 semantic floor holds
with margin ≥ 0.14. (Thinner than the synthetic fixture's gap, but clean.)

## Result 3 — with `--observer-reserve 250`, fusion wins on held-out knowledge AND the cost is measured

| query | baseline phrase | fused phrase | observer sel | noise in packet | tokens base→fused |
|---|---|---|---|---|---|
| gate-ambient (held-out) | absent | **present** | 1 | no | 1195 → 1006 |
| symbolic-composition (held-out) | absent | **present** | 1 | no | 1190 → 1022 |
| language-organs (held-out) | absent | **present** | 1 | no | 1196 → 975 |
| quarry-migration (control) | present | **absent** | 0 | no | 1171 → 948 |

- **Win**: all three held-out queries go from unanswerable to answerable —
  the observer supplied knowledge the index demonstrably lacked, gated in at
  0.69–0.73, with every noise claim deferred at the floor.
- **Cost, same run**: the control query **lost** its answer — the 250-token
  reserve cut the engine's retrieval budget enough that the quarry-migration
  content fell out of the packet. The reserve is a real trade of corpus
  coverage for observer knowledge, measurable per query, not a free lunch.

## Verdict

1. The relevance gate is validated on real text with the production embedder:
   relevant/noise separation is clean at the default semantic floor.
2. Fill-slack-only fusion is inert on rich corpora without reserved headroom —
   the reserve knob is what makes fusion usable there, and it is an explicit
   caller policy (engine tokens traded for observer tokens).
3. Default-off stands as the shipping default: the reserve changes what the
   packet contains in both directions, so it must be a per-call decision by
   a caller that knows whether the observer or the corpus is more likely to
   hold the answer. There is no measured regime where an unconditional
   default beats leaving the choice to the caller.

Fixture numbers from this run are recorded above; they are measurements of
this corpus and these queries, not production claims about any other corpus.

## Follow-up measurement: auto reserve (same corpus, same day)

Fixed reserves force the caller to guess the block size. `--observer-reserve
auto` sizes the reserve from the gated-in claims themselves (pre-gate: the gate
is pure and needs only embeddings, both available before retrieval; reserve =
sum of accepted block tokens + the join-overhead bound).

A measurement defect surfaced and was fixed first, then closed at the source:
the retriever's budget accounting counted bare chunk tokens, but the assembled
packet adds the "\n\n---\n\n" separator — 3 tokens per join (each '-' is a
non-alphanumeric run). Measured on this corpus: the pre-gate priced a
104-token block, the engine's chunk sum fit its 1096 budget, yet the assembled
count reached 1098 and the append stage deferred the block as "over budget:
104 tokens, 1098 used". First patch: auto reserved block + `top_k - 1`. Then
the seam was closed properly: `TokenFixedRetriever::with_separator` checks the
budget against the assembled string itself (opt-in — callers that never
assemble keep legacy accounting), `retrieve_from_texts` opts in, and auto is
back to the exact block sum. Red-first: `assembled_packet_respects_the_budget_
not_just_the_chunk_sum` (two 40-token chunks, budget 80, assembled 83 — red
before the fix). Baseline packets can no longer exceed their nominal budget
either.

Results with auto reserve, budget 1200, bge-small, floor 0.6 (default):

| query | baseline phrase | fused phrase | observer_selected | noise admitted | tokens |
|---|---|---|---|---|---|
| gate-ambient-state (held-out) | absent | **present** | 1 | 0 | 1195 → 1195 |
| symbolic-composition (held-out) | absent | **present** | 1 | 0 | 1190 → 1157 |
| language-organs (held-out) | absent | **present** | 1 | 0 | 1196 → 1191 |
| quarry-migration (control) | present | **present** | 0 | 0 | 1171 → 1171 |

All three held-out queries answerable, zero noise admitted — and the control
query keeps its answer, which fixed-250 lost. On this corpus auto strictly
dominates the fixed reserve: the trade is exactly as large as the observer
content needs. Red-first: `auto_reserve_sizes_the_trade_exactly_and_avoids_
starving_the_engine` pins that auto keeps engine content an over-guessing
Fixed(40) starves; can-fail verified (auto zeroed → test fails). Verdict
update: the reserve knob is now usable without guessing, but default-off
still stands — auto still changes the packet in both directions by design.

## Reserve-size sweep: the coverage-vs-knowledge frontier (same corpus, same day)

Four settings over the same four queries (budget 1200, bge-small, floor 0.6,
zero noise admitted at every setting — the gate holds across the sweep):

| reserve | held-out answerable | control keeps its answer | engine tokens (fused) |
|---|---|---|---|
| **auto** | **3/3** | **yes** | 1195 / 1157 / 1191 / 1171 |
| 100 | 2/3 (symbolic fails) | **no** | 1162 / 1098 / 1115 / 1069 |
| 250 | 3/3 | no | 1006 / 1022 / 975 / 948 |
| 500 | 3/3 | no | 654 / 793 / 717 / 657 |

Readings, all measured:

1. Fixed reserves are dominated in both directions. Too small (100) and the
   symbolic block (~104 tokens) does not fit — the held-out query stays
   unanswerable while the control query STILL loses its answer. Too large
   (250, 500) and the control query loses its answer for headroom the
   observer content does not need — 500 burns ~500 engine tokens to admit a
   ~30-token block on the language-organs query.
2. Auto is the only setting that answers all three held-out queries AND keeps
   the corpus-covered control query. It sizes the trade per query: ~0 engine
   cost where the block is small (language-organs: 1196→1191), ~40 tokens
   where it is large (symbolic: 1190→1157).
3. The frontier is not a smooth trade-off with a sweet spot to tune — it is a
   dominated region (every fixed size loses somewhere auto does not) plus a
   single adaptive point. That is the strongest form of the auto argument:
   there is no fixed number a caller could pick that does as well.
4. Zero noise admission at every reserve size, including the raw-relevance
   probes: the 0.6 semantic floor's separation (relevant 0.69-0.73, noise
   0.40-0.55) is stable across the whole sweep.

Verdict unchanged: default-off stands — but the caller-facing story is now
complete. `auto` is the measured-recommended setting when fusion is wanted at
all; fixed reserves exist for callers with an explicit headroom policy.

## Embedder-dependence: the RP floor fails on real text (same corpus, same day)

Every measurement above used the production semantic embedder (bge-small).
Re-running the identical sweep under the random-projection embedder (the CLI
default when no model dir is set) breaks the gate's core promise:

| query | relevant | noise | verdict at the old 0.9 RP floor |
|---|---|---|---|
| gate-ambient-state | 0.9372 | **0.9016 / 0.9216** | **noise ADMITTED** (noise text in packet) |
| symbolic-composition | **0.8848 (deferred)** | 0.8683 / 0.8666 | relevant content lost |
| language-organs | 0.9111 | 0.8918 / 0.8774 | worked by luck |
| quarry-migration (control) | — | 0.8727 / 0.8362 | — |

The distributions overlap completely — relevant 0.84–0.94, noise 0.84–0.92 —
so NO floor separates relevant from noise under RP on real text. The 0.9
default (calibrated on synthetic fixtures, where lexically distant noise
scores far below relevant text) admitted real noise on one query while
deferring real relevant content on another. The gate cannot keep its promise
with this embedder on real text.

Secondary finding: under RP the engine itself degrades on the real corpus —
the control query's baseline phrase (corpus-covered content) is absent from
the baseline packet at budget 1200. RP retrieval quality is also
real-corpus-inadequate.

Response, per the project rule that a path failing its gate ships disabled:
`default_observer_floor("random-projection")` is now **0.99** — fusion under
RP is opt-in (the caller must pass `--observer-floor` explicitly and accept
the measured risk). The semantic default stays 0.6 (validated: clean
separation on fixtures AND the real corpus). Re-measured under the new
default: all four queries defer everything, zero noise admitted, zero noise
text in any packet.

Verdict update: the relevance gate is a **semantic-embedder feature**. Under
RP it is inert by default, and that is the correct shipping state.

## Semantic floor sweep: 0.6 is measured-placed, not assumed (same corpus, same day)

Same corpus, bge-small, auto reserve, floor varied. Raw relevances (floor-0.0
probes): relevant 0.6928 / 0.7314 / 0.6976; noise 0.3971–0.5534.

| floor | held-out answerable | noise admitted | noise text in packet |
|---|---|---|---|
| 0.5 | 3/3 | **yes (2 queries)** | **yes** |
| **0.6** | **3/3** | **no** | **no** |
| 0.7 | 1/3 | no | no |

Readings, all measured:

1. At 0.5 the noise band's top (0.5534, 0.5356) clears the floor — real noise
   text lands in packets on two queries. The gate's promise breaks exactly as
   the distributions predict.
2. At 0.7 the relevant band's bottom (0.6928, 0.6976) falls below — two of
   three held-out queries lose their answer.
3. 0.6 sits between the measured noise ceiling (0.5534) and the measured
   relevant floor (0.6928): ~0.05 margin above noise, ~0.09 below relevant.
   The margin is asymmetric — noise is the nearer threat — so a slightly
   higher floor (0.62–0.65) would trade relevant recall for noise margin;
   with only three held-out queries measured, 0.6 is the defensible center
   and the sweep's job is to show 0.6 is inside a real gap, not on a
   knife-edge invented by fixtures.

Verdict: the semantic default 0.6 is measured-placed. The full shipping
story is now: semantic embedder + floor 0.6 + auto reserve = the only
configuration that answers every held-out query, keeps corpus coverage, and
admits zero noise — all three properties measured, none assumed.
