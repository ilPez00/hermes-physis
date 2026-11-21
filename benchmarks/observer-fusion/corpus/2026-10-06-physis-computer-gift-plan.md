# Plan — ship `physis-computer` (binary-only public repo)

## State
- Public repo live: `ilPez00/physis-computer` (`main`, `deb6e42` README update in engine + the
  staged tree on remote). **v1.2.0 staged locally 2026-10-07, push pending operator approval**
  (fleet rule: never automate a public push). Staged tree: 169 files, `staged_from=e974cb4`,
  0 `.rs`/`Cargo.toml`, binaries `physis 1.2.0` + `physis-hud 1.2.0`.
- Archives cut and checksum-verified: `dist/physis-1.2.0-linux-x86_64.tar.gz`,
  `dist/physis-context-1.2.0-linux-x86_64.tar.gz` (commercial, `open-release.sh` source seal
  intact). Engine version guard green at 1.2.0 across the root workspace and the four
  separate-workspace lockfiles (`e8ad736`).
- Binary v1.1.1 → v1.2.0 delta: the `physis solve` verification ladder (repeatable `--check`
  with rung levels, stop-at-first-failure, repair evidence, `delta_score`).
- Engine gates: fmt clean, clippy `-D warnings` clean, `research-loop.sh --gate` 61/61.
- IP-leak gate: `scripts/stage-physis-computer.sh` asserts 0 `.rs`/`Cargo.toml`/`Cargo.lock`,
  forbids `crates/ apps/ docs/commercial/ research/ commercial/ .physis/`, no `.git`.

## The rule
Free as in beer. The engine source stays private
(`docs/commercial/MONETIZATION.md`). The gift is *binaries + data + tour*,
nothing that compiles the engine. No future change to this repo relaxes
`scripts/stage-physis-computer.sh`'s allowlist to include source — that single
decision is what separates the marketing stunt from an IP leak.

## Procedure to re-ship (version bump)
1. Engine side: `cargo build --release -p physis-cli -p physis-hud` (and the
   transitive crates the binary links: query, runtime, store, discover, kernel,
   ontology, task, exec, registry, mcp, agent, delta, projection, search,
   reasoning, cognition, config, embeddings, fleet, connectors).
2. `bash scripts/stage-physis-computer.sh /tmp/physis-computer-public`
3. `cd /tmp/physis-computer-public && git add -A && git commit -m "chore: v1.1.0"`
4. `git remote add origin git@github.com:ilPez00/physis-computer.git`
   `git push -u origin main`
5. Verify on remote:
   `gh api repos/ilPez00/physis-computer/git/trees/main -q '.tree[].path' | grep -E '\.rs$|Cargo'`
   → empty.
6. Re-run `physis computer` from a freshly staged tree; confirm non-zero entity
   count (not a splash screen).

## Verification of features (all green on the shipped v1.1.0)
- `physis computer` → 155 files → 168 entities, real bounded packet, escalates at
  p=0.000 (refuses to answer without evidence).
- `physis roots corpus docs` → 472 entities, namespaced `r0:/r1:`, nothing on disk.
- `physis observe .` + `physis compile "…"` → hard-budget packet, bm25+graph.
- `physis embed "…"` → 768-dim, labeled `random-projection (semantic: false)`.
- `physis solve --intent … --execute … --check …` → isolated worktree, derived
  verdict, append-only `constraints.jsonl`. v1.2.0: `--check` is repeatable
  (`level:name:command`, `name:command`, bare command), the ladder stops at the
  first failing rung, and the report carries `delta_score`. Verified end to end
  on a fresh checkout (no prior `physis observe`): `verdict: success`,
  `delta: 1 subject(s)`, constraint log written — the store bootstraps on
  first write, so a stranger's first `physis solve` works without a setup step.
- `physis mcp physis.search --arg q=…` / `physis.describe --arg name=…` → 61 tools,
  canonical-id resolution + on-demand tool docs.
- `physis-hud --snapshot --view {overview,graph,cognition}` → real engine output.
- `physis-hud --setup` → honest gap report (INTELLIGENCE: deterministic, model: none).

## What is NOT in scope for this repo
- The scoring/intelligence layer (`sys1_backend`, model linkage). Ships as
  `deterministic`. The `quarry` feature + an embedded model is engine-private.
- Any solving automation beyond the caller-supplied `--execute`/`--check` harness.
- An A/B measurement of whether the first-run packet helps a stranger finish a
  task faster (carried as the open thread in `docs/HANDOFF_2026-10-06.md`).

## Owner
`ilPez00/physis-computer` — `staged_from` recorded in each tree's `BUILD.txt`.
Re-stage and re-push from any engine checkout at `main` tip.
