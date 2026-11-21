# Primitive library: bounded seed experiment plan

> Planning only. Documentation and brainstorming authorized; no runtime edits,
> imports, feature integration or worker implementation dispatch in this pass.

**Goal:** identify a small reusable library improving one repeated supported task.
**Architecture:** existing `CogPrimitive` definitions/version history with proposed
independent scoped validation receipts; selected read projections. No second graph.
**Stack:** existing Rust operations and deterministic fixtures; €0 upfront spend.
**Design:** [primitive proposal](../commercial/PRIMITIVE_LIBRARY.md).
**Value gate:** [whole-task contract](../commercial/VALUE_PROPOSITION.md).

Experiment plan, not a code-ready API/import specification. Its output decides
whether an implementation spec is worthwhile. [M0–M4](2026-10-05-marvelous-machinery.md)
remains priority.

## P0, optional cap 8h

Trigger: M4 acceptance or an accepted buyer task explicitly needs reuse. Deduct 8h
from remaining 160h engineering only if activated; 152h remain. No additional model
benchmark batch or pack importer within this card.

### P0.1: source inventory and contract, 2h

Read `crates/physis-cognition/src/cogprim.rs`,
`crates/physis-runtime/src/primitives.rs`, `apps/physis-cli/src/cmd_primitive.rs`,
then only the selected operation's producer.

- [ ] Start with resolve/source/callers; eight is a ceiling. Bind exact builtin/API.
- [ ] Record typed inputs/output, scope, consumer, preconditions and digest.
- [ ] Mark unwired/dependent mechanisms CANDIDATE; no repairs in this card.
- [ ] Parent freezes oracle IDs/status/citations before worker proposals.

Deliverable: source-backed eligibility table. Reject cards without useful consumers.

### P0.2: replay existing operations, 3h

- [ ] Isolated supported fixture; pin revision, definition and observation state.
- [ ] Run existing CLI/API; inspect exact output instead of example notes alone.
- [ ] Check identity, omissions, freshness and unchanged read-only source hashes.
- [ ] Exclude zero-example/empty-note/wrong-result cards from validation. Expose
  malformed-log degradation and unknown target/predicate; do not repair runtime here.
- [ ] Link results to task; retain failed/empty runs in denominator.

Deliverable: deterministic replay receipt. Missing producer or evidence means
UNVERIFIABLE/CANDIDATE, never VALIDATED_FOR_SCOPE.

### P0.3: negative controls and task comparison, 2h

- [ ] Change prerequisite/remove edge; output must change or refuse reuse.
- [ ] Change a transitive primitive version or oracle; invalidate procedure eligibility.
- [ ] Exercise wrong root, ambiguity and unrun checks where applicable.
- [ ] Compare native workflow including card/evidence payload, retries and setup.
- [ ] Procedure candidates need two unseen variants after ≥5 accepted comparable
  runs motivate consideration; otherwise defer procedure promotion.

Deliverable: can-fail evidence and trade-offs. Missing provider usage prevents
token claims. Fewer calls alone does not establish bill savings.

### P0.4: disposition, 1h

- [ ] Publish individual accepted/rejected/unverified cards with limitations locally.
- [ ] Manual scoped promotion proposal; no automatic runtime import.
- [ ] Decide whether strict import/composition specification is justified.
- [ ] Preserve losses; stop at cap with current disposition.

Later implementation uses sealed interfaces/owned files and existing free/max
rules, production-consumer review and exact-artifact boundary checks. Existing
substring examples alone cannot certify a pack. No new executable format or CLI
command is promised by this plan.
