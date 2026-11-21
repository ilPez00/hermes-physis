# Consolidate branches and unify Physis release versions

Prepared 2026-10-05. Planning only; implementation checklist:
[branch-version-todo.md](../../tasks/branch-version-todo.md). Existing
`tasks/plan.md` and `tasks/todo.md` retain their open assignments.
Machine-readable inventory: [branch-version-evidence.json](../../tasks/branch-version-evidence.json).

## Outcome

One canonical `main`, every branch's intended work accounted for, and one release
version across repository-owned Physis packages, CLI/HUD, MCP handshakes and
download artifacts. Rebuild and install the matching executables so the version
seen by an operator matches the checked source. Preserve historic refs until
their content audit and candidate checks pass.

Recommended next release: **1.1.1**. This consolidates and restores existing work;
the stopped notebook implementation plan already proposes `1.2.0`. Keep that
feature plan stopped. Choosing a common release number does not certify library
API stability or the unimplemented notebook surface.

## Verified starting point

- Local `main`: `2141bcf1127c72bd887549461c2182de1e83a0a9`; initially clean.
- Remote `main`: `1d606eec0d8bac813f03a3963bfc0aa383298914`, confirmed with
  `git ls-remote --heads --tags origin`; local `main` is six commits behind.
- `feat/lang-coverage` and `origin/feat/lang-coverage` point at remote `main`.
  Its worktree, `/home/gio/pw-lang`, was clean when inspected.
- Current history starts at recovered root `3217b1d`. Remote `AGENTS.md` records
  the already-completed publication of that line and preservation of former
  `main` at `origin/archive/main-2026-10-04`. No new history rewrite is needed.
- The six-commit fast-forward changes seven paths: CI, `AGENTS.md`, `DEV_PLAN.md`,
  HUD `views.rs`, demo/coverage docs and `tasks/todo.md`. CI installs `just`
  correctly and observes the workspace before tests needing its snapshot.
- Static/surface research loop at local HEAD: **46/46 PASS, 0 FAIL,
  0 UNVERIFIABLE**, receipt `research/loops/results/20261005T191320Z.jsonl`.
  This does not certify workspace tests, branch recovery, or release artifacts.
- Physis MCP root was verified as `/home/gio/dev/physis-next`; observation was
  refreshed without truncation. Bounded Cargo context and `HEAD~1..HEAD` diff
  were inspected. The PATH executable is unsuitable for current-tree probes:
  `/home/gio/.local/bin/physis --version` reports
  `physis 0.1.21 (Physis Core + Pro)`, while checkout debug/release binaries
  report `physis 0.1.0`. `.mcp.json` uses this checkout's release executable.

Ref names can move. Recheck remote SHAs and all worktree states before execution;
update the inventory whenever the source set changes.

## Branch disposition and integration order

Counts below are against frozen remote `main` at `1d606ee`. Ancestry and stable
patch IDs prove historical inclusion; current behavior still needs its gate.

| Ref(s) | Tip | Evidence | Planned disposition |
|---|---|---|---|
| local `main` | `2141bcf` | ancestor, six commits behind | Fast-forward after candidate checks. |
| `origin/main`, local/remote `feat/lang-coverage` | `1d606ee` | identical tips | Candidate base; close language branch after integration. |
| `card/bm25-guard` | `7f6e632` | patch identical to `02711ad` | Record inclusion; do not replay its patch. |
| `card/mc0203` | `9e4a007` | patch identical to `fb915f3` | Record inclusion; preserve later MCP repairs such as `872e8e2`. |
| `mc05`, `mcp-receipt` | `9e4f345` | ancestors, 42 commits behind | Already integrated; eligible for cleanup after gates. |
| local `native-loop-parity` | `ce62090` | ancestor, 38 commits behind | Already integrated. |
| `origin/native-loop-parity` | `122b503` | ancestor, 37 commits behind | Already integrated; includes MCP observe/write work. |
| `origin/fix/graph-answer-honesty-v2` | `7ebbce7` | unrelated to current root; four commits after old base `6bc2174` | First recovery lane: graph query and MCP rendering fixes. |
| `origin/task/INTEGRATION-squashed` | `e3e269b` | unrelated; eight commits after `6bc2174` | Second recovery lane: license proof/errors and snapshot freshness. Audit the large `144370d` integration patch separately. |
| `origin/squashed-2026-10-04` | `d8a15e6` | separate root; 270 differing paths versus candidate base | Audit intended source/doc changes by content; archive after disposition. |
| `origin/archive/main-2026-10-04` | `1ffd125` | historical line; 996 differing paths, including 577 `.physis-bundle` paths | Retain as historical backup; audit the unique go-live documentation tail. |

Four historical tips have no merge base with the canonical line. An unrestricted
`--allow-unrelated-histories` merge would mix recovered snapshots and old copies
of later-edited files. Recover verified missing changes as small commits on the
canonical base, with original commit attribution. The existing `.physis-bundle`
material is archival context, not a source tree to reintroduce.

Every ref gets a ledger entry: ancestral, patch-equivalent, recovered with
replacement commit/test evidence, or superseded with a specific reason. A branch
with unresolved desired work remains open. “All branches consolidated” means
all desired work has a verified disposition; archived roots remain recoverable.

## Recovery work that fast-forward will not supply

These are source-level findings. Their intended behavior must be reproduced and
tested on the candidate before claiming recovery complete.

1. **Graph query correctness**, `b2a32d7` / `d38e41d`: current `depends_on`
   lacks the file-to-owning-package lift; `related_tests` still matches file
   stems; exported `Resolution` and observed-versus-unresolved target handling
   are absent. Recover the discriminating fixtures for `lib.rs` across two
   crates, `scan.rs` versus `rescan.rs`, package-level queries, and unknown IDs.
   Keep current exact canonical-ID matching and mixed-language relations.
2. **MCP and CLI absence rendering**, same lane: current `rows_tool` returns
   `display.join("\n")`, so empty display produces empty text. Context sections
   also join empty arrays directly. Restore explicit unknown/empty text while
   retaining structured rows, counts, paging and current budget contracts.
   Reconcile `8a39b6d` docs and `7ebbce7` ignore additions individually.
3. **License integration**, `48c7fac` / `7190e8f`: current tree lacks
   `activation_wire.rs`, `praxis_envelope.rs`, and its fixture. Audit HTTP error
   handling before restoring code: current commercial licensing glue has since
   changed to verified offline installs. Recover applicable Rust verifier and
   signed-envelope regressions; do not revive superseded activation behavior.
4. **Observed-world identity**, `9554f21` / `e3e269b`: current engine lacks
   `absence_note`, `Staleness`, recorded `base_commit`, and `world-base` handling.
   Recover supported absence/freshness behavior against current shared ingestion.
   Prove observation at commit A, HEAD movement to B without observation, stale
   answer, then observation and current answer. An unchanged observe must not
   grow history. A comparison against HEAD at load time is not freshness proof.
5. **Remaining snapshot differences**: compare `144370d`, `d8a15e6`, and archived
   `1ffd125` against the candidate, including HUD/notebook, config, scripts,
   proofs and go-live docs. Preserve later canonical fixes and language coverage.
   Old receipts document old source; recovered receipts cannot certify new HEAD.

Do not import physis-pro source or retired machine concepts. Praxis still has a
live dependency on `physis-pro-web:19876`; migration off it is separate work.

## Version contract

| Surface | Observed value | Target and mechanism |
|---|---|---|
| Root workspace packages | 25 at `0.1.0`; `physis-license` at `1.0.0` | `[workspace.package] version = "1.1.1"`; all 26 members inherit `version.workspace = true`. |
| Standalone commercial package | `1.1.0` | Explicit `1.1.1`, checked against root version. Retain its separate workspace and commercial boundary. |
| Three standalone Physis adapters | `0.1.0` | Explicit `1.1.1`, checked against root. Retain their isolated workspaces and `publish = false`. |
| CLI / HUD / license-admin / open MCP | package-derived values | Built executables and `initialize.serverInfo.version` agree with root version. |
| Commercial binary / commercial MCP | package-derived `1.1.0` | Both agree with release version and archive metadata. |
| Release tag and archives | publisher uses `v<version>` | One immutable `v1.1.1` tag at tested candidate SHA; all intended artifacts identify that SHA and version. |

Root `Cargo.toml` becomes the authoritative release version. Separate workspaces
cannot inherit it through their own `[workspace]`; use explicit mirrors plus one
small drift check/synchronization command, rather than changing workspace
membership. Discover root members from the manifest and standalone owned
packages from their manifests. Regenerate the root, commercial and three adapter
lockfiles with Cargo; do not rewrite dependency versions by text replacement.

The guard validates **all 30 repository-owned Physis packages**. It excludes
registry dependencies such as the linked `physis-core 0.1.25`, experimental
`demo-api`, and test fixture packages. MCP `protocolVersion = "2025-06-18"`,
license entitlement `MAJOR_VERSION = 1`, observation/schema versions and
historical benchmark references have distinct contracts and retain their values.
Audit any version-qualified local dependency during manifest conversion.

## Release and executable corrections

- Update packaging/version reads to the canonical version contract. Current
  commercial packaging and publisher parse a literal product manifest version;
  explicit mirrors can remain, but both scripts must run the drift guard.
- Publisher currently prints “no overwrite” then calls `gh release upload
  --clobber`. Make existing tags/assets immutable: verify exact expected source
  SHA; accept identical asset checksums or fail. A changed asset needs a new
  version. Explicitly target the tested commit when creating a release.
- Package the intended CLI/HUD and commercial artifacts with version, source
  SHA and checksums. Preserve the existing commercial boundary, public verifier,
  source-stability checks and secret/path scan. Validate archives after extraction.
- Build checkout executables explicitly. Back up and replace the stale installed
  `physis` only during authorized installation; check `command -v` and all
  versions after installation. Inspect actual MCP process executable/root and
  restart hosts after switching binaries; a running process keeps its old build.
  Preserve unrelated MCP registrations when updating config.
- Update current install/listing/changelog examples for the new release. Retain
  old evidence and version-specific reports as history. Do not claim stopped
  notebook features, new platforms, or paid fulfillment from consolidation.

## Execution sequence

1. **Freeze and preserve.** Recheck refs/worktrees; save exact SHAs and a local
   `git bundle create /tmp/physis-branches-20261005.bundle --all`; verify it.
   Assign one integration owner and derive a branch-content ledger.
2. **Create isolated candidate.** Branch `integration/branch-version-unification`
   from the rechecked canonical remote tip in a worktree under `/tmp`. Read that
   tip's `AGENTS.md`. Build its CLI/HUD and observe that worktree before baseline
   tests. Record a clean-source baseline before recovery changes.
3. **Recover graph behavior**, then **MCP/CLI rendering**, then **license proofs**,
   then **world freshness**. Each commit gets focused regressions and a Physis
   context/diff/invariant receipt. Reapply selected hunks, not whole old files.
4. **Close the remaining content audit.** Map old snapshot differences to
   retained/recovered/superseded behavior. No unexplained missing work at this
   checkpoint. Record patch-equivalent card inclusion without duplicating patches.
5. **Unify versions**, using the small manifest batches in the checklist. These
   intermediate commits stay on the candidate; they are not release points.
   Land the guard, lockfiles and script integration before final validation.
6. **Validate exact source and artifacts.** Run the full matrix below, then
   review behavior, history disposition, commercial boundary and version drift.
7. **Integrate and publish when authorized.** If `main` advanced, rebase the
   candidate and rerun affected gates on the resulting SHA. Fast-forward local
   `main` to the checked tip, push through the normal protected-branch/PR flow,
   verify remote CI for that SHA, then create the immutable versioned release.
8. **Install and close branches.** Verify executable/MCP agreement after host
   restart. Retain backup refs; remove only accounted-for branches and unused
   clean worktrees after explicit cleanup authorization. Record `physis.remember`
   with actor `codex`, source SHA, results and unresolved states.

Only this planning document and its evidence/checklist are delivered now.
Future publishing, branch deletion and installed-binary replacement remain
explicit execution stages; no confirmation is needed merely to review the plan.

## Candidate validation matrix

Run from the isolated candidate with its own observed store. Do not force one
shared store across unrelated tests. Record command, exit code, source SHA,
features, environment and timeout. **UNVERIFIABLE is not PASS.**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p physis-cli -p physis-hud
./target/debug/physis observe .
cargo test
cargo test --workspace --all-targets
cargo test -p physis-tests --test invariants
just mcp-e2e
cargo test -p physis-hud --test snapshot
just snapshot-smoke
bash scripts/commercial-acceptance.sh
bash scripts/check-commercial-boundary.sh
just quarry-check
bash scripts/research-loop.sh --gate
```

Also run standalone product/adapters' fmt, strict clippy and tests with each
manifest; regenerate and validate every owned lockfile. The SQLite manifest
documents a nightly test invocation: verify its actual toolchain requirement
and record the selected toolchain rather than skipping the adapter.

Run `just commercial-release` with the operator's existing public verification
key; validate the exact extracted archive with the contract/value/PR checks
already used by the release pipeline. Test-signed evaluation evidence does not
prove paid fulfillment. Probe all built `--version` paths and both actual MCP
`initialize` handshakes. Run the new version guard with an intentional mismatched
standalone manifest in a temporary fixture to prove drift is detected.

Before integration: observe the candidate, inspect `physis diff` and invariant
results, review the branch ledger and record the outcome with `physis.remember`.
Full implementation gates are **NOT RUN** during this planning task.

## Completion and rollback

Complete only when every planned ref has a reviewed disposition, recovered
behaviors have discriminating current-source proofs, all owned package/artifact
versions agree, clean-source gates and extracted-artifact checks pass, and the
published source SHA matches the tested candidate. Historical archives may stay
as archives; no desired work remains stranded on a feature branch.

Candidate failures stay isolated. Keep local and remote `main` at the last
verified tip until integration. After integration, revert faulty recovery/version
commits through normal history and restore the backed-up installed executables
if needed. Preserve any published tag/assets; issue a later patch version for
artifact corrections. Backup bundle and archive refs remain the recovery source.
