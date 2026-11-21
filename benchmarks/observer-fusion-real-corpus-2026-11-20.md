# Observer fusion on a real corpus — measured 2026-11-20

> **Current verdict (read this first; the sections below are the audit trail
> in the order the measurements overturned each other):** the semantic
> default floor is **0.66** — the margin-aware zero-contamination point,
> validated at n=22 held-out / 46 noise (14/22 answerable, zero noise text;
> held across two sample broadenings). The relevance ROC on real text is
> SHALLOW: no floor gives both high recall and zero contamination. Under RP
> the gate cannot separate relevant from noise at all, so fusion ships
> opt-in (floor 0.99). The relative-signal gate (claim vs engine's own best
> chunk score) was measured and does not beat the absolute floor — rejected
> in `8e602`. Auto reserve strictly dominates fixed reserves. The harness
> and recorded results now live in `observer-fusion/` in-repo.

Engine: `physis-core` 0.2.0 (post `0cc7e7c` + reserve), production embedder
**bge-small** (semantic ONNX, `PHYSIS_MODEL_DIR`), embedder-aware default
floor (0.6 semantic — superseded, see verdict above and Result sections).

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

## n=7 correction: the semantic 0.6 default also admitted noise (same corpus, same day)

The floor sweep above rested on 3 held-out queries. Broadening to 7
(architectural-convergence, impossibllm-reasoning-ladder, shock-economic-demo,
release-closeout added) changed the verdict:

- relevant band: 0.6185 / 0.6536 / 0.6928 / 0.6945 / 0.6976 / 0.7314 / 0.768
- noise band: up to **0.6385** (noise-playground on the impossibllm query)

The bands OVERLAP at the shipped 0.6 default: on the impossibllm query the
noise claim scored 0.6385 ≥ 0.6, was admitted, and its text landed **verbatim
in the packet** (observer_selected 2). The n=3 sample made the gap look clean
(noise ceiling 0.5534 vs relevant floor 0.6928); samples 4–7 found a relevant
claim below the old noise ceiling. Fixture-calibrated margins do not survive
contact with a broader real sample — third time this session's measurements
have proven that pattern.

Response: the semantic default moves 0.6 → **0.65**, inside the measured
zero-noise gap (0.6385, 0.6536]. Re-measured at 0.65 and via the deployed
binary's default path: **6/7 held-out queries answerable, zero noise
admitted, zero noise text**. The one deferral (shock-economic-demo, 0.6185)
is the safe direction — a visible receipt, not silent contamination.

Honest caveat, recorded: the gap is 0.015 wide and rests on 7 relevant + 14
noise samples. The floor is a measured point on an imperfect ROC curve, not
a discovered natural boundary; a broader sample could move it again. The
gate's receipts make every decision visible, so retuning stays cheap.

Verdict update: default-off still stands; the semantic default is 0.65, and
the shipping configuration (semantic + 0.65 + auto reserve) is the only
measured configuration with zero noise admissions across all sweeps.

## n=13: the ROC is shallow — the default moves to the zero-contamination point

Six further held-out queries (physis-cc, gui-navigation, impossibllm-rescope,
invariant-coverage, defect-remediation, economic-relevance) broaden the sample
to 13 relevant / 28 noise. The measured ROC over all samples:

| floor | relevant admitted | false-admits (noise text in packet) |
|---|---|---|
| 0.60 | 12/13 | 3 |
| 0.65 | 10/13 | 1 |
| **0.66** | **9/13** | **0** |
| 0.70 | 5/13 | 0 |

Bands: relevant 0.5543–0.7774, noise 0.3954–0.6522. The overlap is now wide
and structural, not a thin-gap artifact: **no floor gives both high recall and
zero contamination.** The 0.65 false-admit is instructive — noise-nas at
0.6522 on the invariants query ("inventory, change tracking, evidence
tooling" against "invariants report coverage") is a domain-adjacent HARD
NEGATIVE, not embedder noise. Production observer claims come from the same
ecosystem, so hard negatives are the realistic case, not an artifact of the
noise set mentioning Physis.

Response: the semantic default moves 0.65 → **0.66**, the measured
zero-contamination point. Re-verified via the deployed binary's default path:
9/13 held-out answerable, zero noise admitted, zero noise text. The four
deferrals (0.5543, 0.6052, 0.6185, 0.6536) are all visible receipts.

Final verdict for the relevance gate, replacing the earlier "0.6/0.65 is
measured-placed" claims: **the gate is a recall-vs-contamination trade with a
shallow ROC on real text.** The shipping default picks zero contamination and
pays in recall; callers wanting more recall pass `--observer-floor` and own
the measured risk (each step down the table buys ~1-3 relevant queries at the
cost of real noise text in packets). Candidate future unit (not built): a
relative signal — e.g. requiring a claim to beat the engine's own best chunk
score — might separate answer-versus-adjacent better than an absolute
cosine floor; the current measurement is the baseline such a change must
beat.

## Negative result: the relative-signal gate does not beat the absolute floor

The candidate future unit — gate a claim not on an absolute cosine floor but
relative to the engine's own best chunk score for the same query — is now
measured, and it does not win. Method: `engine_best_score` added to
ContextReport (the engine retrieval's top chunk score, now visible in every
JSON report), then ratio = claim_relevance / engine_best computed for all 13
held-out queries and 28 noise samples.

| rule | relevant admitted | false-admits |
|---|---|---|
| absolute floor 0.60 | 12/13 | 3 |
| absolute floor 0.66 | 9/13 | 0 |
| **ratio ≥ 1.05** | **9/13** | **0** |
| ratio ≥ 1.0 | 10/13 | 2 |
| floor 0.66 AND ratio ≥ 1.0 | 9/13 | 0 |

The relative rule's best operating point (ratio ≥ 1.05: 9/13, 0 noise) is
exactly the absolute floor's best (0.66: 9/13, 0 noise), and the conjunction
adds nothing. The reason is visible in the data: engine_best is nearly
constant across queries (0.587–0.738 on this corpus), so the ratio is close
to a monotone transform of the absolute relevance — it carries no separation
information the absolute floor lacks.

Per the project rule (the machine proposes; the measurement decides), the
relative gate does not ship. The absolute floor at 0.66 stands as the
baseline, now with a measured refutation of the obvious alternative rather
than an open question. If a future embedder makes engine_best vary meaningfully
across queries (e.g. on corpora with heterogeneous chunk quality), this
measurement should be repeated before dismissing the idea again.

## n=22: the 0.66 placement holds on a doubled sample

Nine further held-out queries (viewspec-lens, dream-grading, sovereign-sequence,
praxis-census, full-review-verified, gui-pulse-bar, blast-radius-timing,
semantic-lens-exclusions, workbench-receipts) broaden the sample to 22 relevant
/ 46 noise. Result at the shipped default 0.66: **14/22 answerable, zero noise
admitted, zero noise text** — the zero-contamination property held as the noise
sample nearly doubled (ceiling still 0.6522; no new tail appeared).

Full ROC at n=22:

| floor | relevant admitted | false-admits |
|---|---|---|
| 0.60 | 20/22 | 4 |
| 0.65 | 15/22 | 1 |
| **0.66** | **14/22** | **0** |
| 0.70 | 10/22 | 0 |

Frontier note, recorded for honesty: the measured-optimal zero-noise floor is
any point in (0.6522, 0.6536] — it would admit 15/22 (release-closeout at
0.6536) with zero noise. That sits 0.0014 above the observed noise ceiling of
46 samples. After three consecutive overturns from tails the sample did not
contain (0.9 RP, 0.9 fixed deferral, 0.6 semantic), a 0.0014 margin over an
observed maximum is overfit territory; 0.66 keeps 0.0078 of margin and gives
up one query for it. The recall trend as the set broadened is also recorded:
10/13 (77%) at n=13 became 14/22 (64%) at n=22 — the later queries are harder,
and every deferral is a visible receipt, which is the safe direction.

Verdict: 0.66 stands, now validated on n=22/n=46 rather than assumed.

## n=30: the ceiling holds across a third broadening

Eight further held-out queries (durable-tms, rau-semantics, temporal-defect,
paired-bench-metric, playground-phase1, hackathon-dating, spaceship-matrix,
steering-ir-rule) broaden the sample to 30 relevant / 62 noise. At the shipped
default 0.66: **20/30 answerable (67%, up from 64%), zero noise admitted** —
the noise ceiling is STILL 0.6522, unmoved across three broadenings (28 → 46 →
62 noise samples). The ROC shape is unchanged: 0.65 → 22/30 + 1 noise; 0.66 →
20/30 + 0. Two new measured deferrals (rau-semantics 0.6411, steering-ir-rule
0.6568).

Verdict: the 0.66 placement now rests on n=30 relevant / 62 noise with zero
contamination at every sample size measured. Within this embedder and corpus
family, the placement is as firm as sampling can make it; further broadening
would refine the recall estimate, not the contamination verdict.

Tooling cross-check: the shipped `selftest-probes.json` now carries all 30
probes with measured expectations (20 answer / 10 defer); `physis-core
selftest` passes 30/30 against the deployed binary, and `calibrate-floor` on
the full set re-derives the entire record (bands 0.5543–0.8630 / ceiling
0.6522, recommendation 0.6602, recall 20/30) through the shipped tooling.

## Cross-family check: the floor is corpus-relative, zero contamination transfers

A second corpus family — 30 heterogeneous praxis_webapp docs (API references,
runbooks, business plans, data-access guides; 1.6 MB) with 5 held-out probes
from docs verified absent — tests whether 0.66 generalizes beyond the
physis-plan family. Shipped tooling results on the new family:

- `selftest`: **5/5 PASS, exit 0** (all answered, zero noise text).
- `calibrate-floor`: noise ceiling **0.5590** — well below the plan family's
  0.6522. The per-corpus recommendation is 0.567; the shipped 0.66 keeps ~0.10
  of margin here. Relevant band 0.6694–0.8155.

**Verdict: 0.66 is not a universal constant — it is the plan-doc family's
zero-noise point.** Zero contamination transfers across families (0.66 sits
above both measured ceilings), but the *optimal* floor is corpus-relative:
on the heterogeneous family the same floor is over-cautious by ~0.09. This
is exactly the case for shipping `calibrate-floor` rather than claiming one
universal number: a caller with a different corpus measures their own ceiling
and chooses with the receipt in hand.

Method note, recorded honestly: the first probe draft had 8 probes, three of
which (dr-runbook, hosting-retirement, metrics-endpoint) turned out to draw
from docs INSIDE the corpus — a selection error. The baseline-presence check
flagged exactly those (base=True on two; the third's phrase merely failed to
surface), which is the selftest's `baseline_absent` contract working on real
data. The shipped `selftest-probes-praxis.json` contains only the 5 clean
probes; the 8-probe raw run is preserved in `results-praxis-family.json`.
