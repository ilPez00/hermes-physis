# Implementation plan — interview residual (0.96→1.00) + corpus growth

Date: 2026-09-30. Basis: HALF_WIRED row 14 diagnosis + user brief
("residual + corpus growth"). Scope: `crates/physis-interview` only for
ranking; `corpus/` for growth. No budget widening, no global ranker surgery.

## Ground truth (verified by inspection)

- `crates/physis-interview/src/retrieve.rs:412` `retrieve()`:
  direct hits → related expansion over **top-3 direct hits only** (line 418)
  at `RELATED_DECAY = 0.35` (line 28) → recipe expansion at `COMPOSE_DECAY`.
  `retrieve()` never truncates.
- `crates/physis-interview/src/compose.rs`: pack selection is top-K by score
  with hard budget truncation. The diagnosed misses (`backpressure→hpa`,
  `bounded-concurrency→retry-backoff`) score 0.009–0.011 after 0.35× decay —
  edges exist, expansion fires, they only lose the score race.
- `INTERVIEW-TQ` bench: 0.96 coverage @ ~380 tok/question, deep reference
  1.00 @ ~11.7k (31×), deterministic, ms-scale.
- Corpus: 144 objects (`corpus/{rust,k8s,recipes,design,quiz}`), loader
  validates in `Corpus::validate`, invariants in
  `tests/invariants/interview_corpus.rs` (graph reachability + 70 expects).
- Interview cache keys on corpus fingerprint → growth auto-invalidates memos;
  no stale-hit risk by construction.
- gRPC service is the only spec-listed concept without a primitive.

## Risks

- **Ranking change perturbs other benches** — change stays inside
  `physis-interview`; full workspace gate re-run; constants named + commented.
- **Budget creep** — if tok/question > ~400, evict the weakest direct hit
  before weakening any expanded one; never widen the budget.
- **Corpus growth breaks expects** — expects vocabulary grows only where the
  new primitive supplies the answer; invariant count updated in same commit.

## Slices (each lands with tests, clippy -D warnings clean)

### S1 — Fix the expansion-score race (outcome: rejected as not-safe)

1. `retrieve.rs`: tried three localized ranker changes, all measured against
   INTERVIEW-TQ on the committed corpus:

   | change | cov_a | notes |
   |---|---|---|
   | `RELATED_DECAY` 0.35→0.5 uniform | 0.80 | boosts sibling noise; displaces direct lexical hits |
   | `RELATED_DECAY` 0.35 + top-5 expansion | 0.96 | uniform scale preserves rank order, no movement |
   | floor direct-hit neighbours at weakest direct | 0.96 | same |
   | quiz-anchor: exact question match → boost quiz's `related` ×3.0 | 0.80→0.91 | fixed backpressure to 1.00 but displaced real lexical hits in 4 other questions |
   | k 5→6 | 0.93 | budget breach risk, displacement, no net gain |
   | lexical boosting of `k8s.hpa` text | 0.96 | query never names autoscaling/hpa, so BM25 never ranks it in |

2. Verdict: **no ranking change lands 1.00 without displacement.** The residual
   (2 tokens × 2 quiz items: `scale on backlog`, `retry jitter`) is structural —
   the default-depth payload (invariant + answer_30s) of no primitive in the
   ranked pack contains the token `scaling`/`jitter`, and the only primitive
   that does (`k8s.hpa` Deep answer_2m/tags) is reachable only via Deep render.
   Recorded, not forced.

### S2 — Compose-side guarantee
Cancelled: no safe selection reorder reached 1.00 without displacing lexical
hits (see S1 table).

### S3 — Corpus growth
1. Added `corpus/rust/grpc-service.prim` (the only spec-listed concept absent),
   load-validated, graphed (`related: rust.axum, rust.tower, rust.serde,
   rust.tracing`, `question: when do you pick gRPC over HTTP/JSON?`).
2. Hardened `corpus/k8s/hpa.prim`: `answer_30s` "sets replicas" → "scales replicas … queue backlog"; added `when_to_use` line `queue growth or backlog detected by an external metrics adapter`. No-regression wording that lets the Deep arm carry the `scale` token at default depth.
3. Result: cov_a unchanged at 0.96, cov_b holds 1.00; `+hpa scales` wording is
   the only no-regression lever available.

### S4 — Docs + counts
1. HALF_WIRED row 14: 144→145 objects; verdict table recorded.
2. STATUS.md: counts refreshed (110→111 primitives, 38→39 rust).

### S4 — Docs + fingerprint refresh

1. `docs/HALF_WIRED.md` row 14: residual closed or verdict recorded
   (whichever the bench says); "next minimum slice" updated.
2. `corpus/README.md` + any fingerprint/token-count docs touched by growth
   (pattern of commit `3c31e8b`).

### S5 — Full gate + dogfood

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `cargo test --workspace --all-targets`
3. `physis observe .`; MCP `physis.remember` (actor: opencode).
4. Manual: `physis q "how do you bound retry storms?"` receipt check;
   `physis quiz` on the two previously failing items.

## Stop condition

Bench 1.00 at ≤ ~400 tok → done. If 1.00 requires budget > ~400 or
cross-crate ranker changes, record the verdict in row 14 instead —
`:explain` deep remains the designed path for misses.
