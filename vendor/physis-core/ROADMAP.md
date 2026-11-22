# physis-core roadmap

Living plan for the `physis-core` crate. The engine is the one described in
`src/lib.rs`: competing interpretations over an adaptive semiotic grid, scored
for coherence, revised without deleting history.

## Where the source lives

The live source is **vendored at `hermes-physis/vendor/physis-core`** and is the
copy that builds hermes-physis's bundled `physis-core` binary. The public
`github.com/ilPez00/physis-core` repository is **retired** (`RETIRED.md`, frozen
at 0.1.26) — it is history, not the development target. `Cargo.toml`'s
`repository` field still points there; see the decision log below.

## Current release — 0.4.0 (2026-11-22)

Prepared and gate-green:
- `cargo build` / `cargo test` green (5 unit + 4 doctest).
- `cargo publish --dry-run` packages + compiles + verifies (120 files).
- `physis-check capabilities` 3/3 (`.physis-capabilities`).
- `scripts/check-release.sh` enforces version/README/CHANGELOG/tag/clean/capability.

Since 0.3.0: observer-eval ws14 rollup hardening, `GateReasonCode` gate reasons
with drift sentinel, `selftest` + `--baseline` receipt-drift watch,
`calibrate-floor`, model-aware default observer floor; reverted the sync-bot
injection from 0.3.0. Plus first-run `demo` corpus and an honest
missing-corpus error.

## Next

- [ ] **Publish 0.4.0 to crates.io** — blocked: no `CARGO_REGISTRY_TOKEN` on this
      host. Needs the operator's token. Run `scripts/check-release.sh` first;
      crates.io versions are immutable.
- [ ] **Decide the `repository` field.** It points at the retired public repo.
      Options: leave it (records origin), repoint to `hermes-physis` (where it is
      actually maintained), or stand up a dedicated crate repo. Not changed
      unilaterally — this is a product decision.
- [ ] **Installed-binary corpus.** `examples/demo-corpus` makes `demo` work
      in-tree; a `cargo install`ed binary still needs `--dir`. Consider an
      embedded default corpus so `physis-core demo` works anywhere.
- [x] **Runnable, compile-checked example.** `examples/observation_log.rs`
      appends typed observations and reads them back (`cargo run --example
      observation_log`), demonstrating the observation-log capability.
- [ ] **`selftest` needs a shipped fixture.** `selftest`/`calibrate-floor`
      require corpus+probes fixtures that live in the private engine repo, so
      they are NOT MEASURED from this crate alone. Ship a tiny probe set.

## Known limits (stated, not hidden)

- ONNX embedder is optional; without a model the crate falls back to
  deterministic random projection and every receipt says so (`semantic: false`).
- Caller/edge resolution is conservative — a missing edge is preferred to a
  wrong one.
- `physis-core.git` is retired; do not treat it as current source.

## How to release

```bash
cd vendor/physis-core
bash scripts/check-release.sh        # gate: version/tag/clean/capabilities
cargo publish --dry-run              # package + verify
cargo publish                        # irreversible; needs CARGO_REGISTRY_TOKEN
git push origin main                 # + the tag
```

Then `fleet-sync` (bundle to air's USB when mounted; a skip is logged, never
faked).
