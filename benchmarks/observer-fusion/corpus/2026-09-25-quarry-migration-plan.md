# Quarry migration plan — physis-pro → physis-next

Date: 2026-09-25. Status: **proposed programme; sequencing authority for the
physis-pro port**. Companion to `MIGRATION.md` (ledger rules) and
`MIGRATION_INVENTORY.md` (P0 first pass). Provenance snapshot of the quarry's
docs: `docs/quarry/physis-pro/` (read `PROVENANCE.md` there first).

## Decision recorded

**All future physis-pro work happens in physis-next.** The quarry
(`ilPez00/physis-pro` @ `6462fd5`, branch `agent/P-037-substrate`) is frozen
as read-only: no new features, no doc updates that matter, no fixes that do
not first land here. Plans, packets and research reports are snapshotted
under `docs/quarry/physis-pro/` and become work here only through receipts.

This does **not** mean "copy 132k lines". It means every physis-pro surface
gets an explicit disposition below: PORT (transplant smallest correct unit),
ADAPT (re-shape to kernel primitives), RESEARCH (index the idea, no code
yet), APPLICATION (never kernel), or DROP (superseded / refuted — do not
relitigate; see `docs/quarry/physis-pro/docs/WHAT_PHYSIS_IS.md` §5 settled
negatives).

## Current state (measured 2026-09-25)

- quarry: 257 `.rs` files, ~132k lines, 15 `src/` subtrees
- physis-next: 43 files, ~7.9k lines, 15 crates + 2 apps, 48 tests green
- already salvaged (see `MIGRATION_INVENTORY.md`): kernel relation/temporal/
  provenance, store revision/log, discover scanner/git/cargo, query budget
  pack, BM25 projection, semantic-delta research index, HUD shell + entity
  lens (this repo's `apps/physis-hud/src/lens.rs` ports the quarry's
  `hud::Lens` contract)

## Phases (one packet at a time; gate before next)

### Q1 — World surface (the HUD programme core)
| unit | quarry source | target | disposition |
|---|---|---|---|
| Focused-object lens | `src/hud/lens_object.rs` (169 L) | `apps/physis-hud/src/lens.rs` | PORT — extend existing EntityLens with recorded-relation glyphs (solid/double/dashed) once provenance classes exist |
| Semantic lens | `src/hud/lens_semantic.rs` (352 L) | `apps/physis-hud/src/lens.rs` | ADAPT — strong/weak/bridge/sibling categories over `physis-projection` embeddings when embedder lands; lexical-only first, degradation labelled |
| ProjectionRow/group grammar | `src/hud/row.rs` (283 L) | `apps/physis-hud/src/lens.rs` | PORT — row identity (`origin:key`), groups name rows, never duplicate |
| Focus invariant | `src/hud/world.rs` FocusState (208+ L) | `apps/physis-hud` state | PORT — projection switch never moves the focus; test first |
| nav hybrid scorer | `src/nav/mod.rs` (1688 L) | `crates/physis-projection` | ADAPT — semantic+lexical+recency+context blend behind the existing `Projection` trait; per-component `why:` is the contract, not a nicety |
| NavWorkspace façade | `src/nav/workspace.rs` | `crates/physis-runtime` | ADAPT — hot-swap index+store behind one handle |
Gate: `--snapshot --view=entity:<id>` renders recorded + inferred edges with
distinct glyphs; focus invariant test green.

### Q2 — Embeddings + semantic retrieval
| unit | quarry source | target | disposition |
|---|---|---|---|
| Embedder trait + registry | `src/embed/` (clip/jina/hash backends) | `crates/physis-projection` | PORT trait, ADAPT backends; lexical hash fallback is the default, real model optional feature |
| Embed cache | `src/nav` `pack::EmbedCache` | `crates/physis-projection` | PORT |
| `/api/embed` contract | quarry `web/embed` shape | `crates/physis-mcp` tool | ADAPT |
Gate: same query answers identically with and without model, header says
`semantic: off` when degraded. F7 lesson applies: neural must beat lexical on
the routing decision or it stays off by default (`scripts/f7-kill-decision.py`
methodology, quarry `docs/recon/f7-corpora.md`).

### Q3 — Guided execution (constraint surface)
| unit | quarry source | target | disposition |
|---|---|---|---|
| Constraint IR (Choice/ObjectSet/FreeText/Sequence) | `src/guided/constraint.rs` | `crates/physis-reasoning` | PORT — pure IR, no deps |
| World→constraint compiler | `src/guided/compile.rs` | `crates/physis-reasoning` | ADAPT over physis-next world snapshot |
| Affordances from capability table | `src/guided/affordance.rs` + `src/capability.rs` | `crates/physis-registry` | ADAPT |
| JSON Schema + grammar renderers | `src/guided/` | `crates/physis-reasoning` | PORT |
Gate: deterministic mock model compiles a valid action for a seeded world;
invalid action is an absent sentence, not a rejected one.

### Q4 — Agents as clients (no privileged path)
| unit | quarry source | target | disposition |
|---|---|---|---|
| Harness adapters (hermes/opencode/jev) | `src/agents/adapter.rs`, `routing.rs` | `crates/physis-agent` | ADAPT — registry already has module rows; add `HARNESSES` + `adapter_bin` shape (P16 pattern, provenance `1ffa22e`) |
| Agent delegation CLI | quarry `physis agent delegate` | `apps/physis-cli` | PORT command shape |
| Bounded context handoff | `src/agents/decision.rs` | `crates/physis-agent` | ADAPT — context always via `physis-query` budget pack, never raw files |
Gate: `physis agent delegate hermes "task"` runs against the runtime APIs
only; no model in the command path.

### Q5 — Applications (explicitly never kernel)
industrial (oee, maintenance, inventory, monitor, moonraker), backoffice
(reconcile, management_pack), life_os, mails/vault importers, web_server,
studio — **APPLICATION**, quarantined under `applications/` (new dir) only
when a use case demands them. Default disposition: RESEARCH_INDEXED — the
plan docs are in the quarry snapshot; no code moves without a caller here.
`spanner.rs`/`postgres.rs`: DROP (external sink deps; revisit on demand).

### Q6 — Research corpus
`research/` experiment reports (E1–E74, papers, method notes): snapshot is
in the quarry copy. Disposition RESEARCH_INDEXED — `docs/RESEARCH_PAPERS.md`
gains a quarry index section; no result is quoted in physis-next without
re-measurement. Settled negatives (WHAT_PHYSIS_IS §5) carry over as
**permanent bans**: do not relitigate embedding-geometry ontology structure,
certification, or cross-model disagreement as signal.

## Doc updates this change makes

- `README.md` — canonical-target statement + quarry doc pointer
- `PLAN.md` — Q1–Q6 packet rows added
- `MIGRATION_INVENTORY.md` — plans/packets/research section
- `docs/RESEARCH_PAPERS.md` — quarry index note (follow-up)

## Status (updated 2026-09-25)

| Phase | Status | latest |
|---|---|---|
| Q0 | done | `1e3aeb8` docs snapshot + provenance + plan |
| Q1 | done | `b7dee1f` entity-lens identity round-trip invariant, 4 lens tests |
| Q2 | done | `2958644` Embed trait + HashEmbed + SEMANTIC_PROBES in physis-projection, 3 tests |
| Q3 | pending | Constraint IR → physis-reasoning |
| Q4 | pending | harness adapters → physis-agent |
| Q5 | pending | applications quarantine |
| Q6 | pending | research corpus index |

## Rules (inherited, restated)

1. Quarry read-only; provenance on everything (`PROVENANCE.md` here).
2. Smallest correct unit; ~100 good lines beat 10k entangled ones.
3. Every port lands with its test (declared ≠ called — grep call sites
   before claiming a capability).
4. Degradation labelled, never silent; offline-first always.
5. No LLM in the command path. Models are accelerators, never foundations.
6. Gate before next phase: `cargo test` + `cargo clippy --all-targets --
   -D warnings` + HUD snapshot regression.
