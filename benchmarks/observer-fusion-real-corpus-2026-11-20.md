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
