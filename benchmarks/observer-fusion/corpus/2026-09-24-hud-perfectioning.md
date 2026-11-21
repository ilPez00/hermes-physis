# HUD perfectioning plan — physis-next

Target: `apps/physis-hud/src/main.rs`. One focused commit. No privileged HUD path.

## Bugs to fix

1. `Box::leak` per frame — line ~364-366.
   Context tab, empty packet case, leaks a `ContextPacket` every draw frame
   (~5 fps). Replace with a literal empty render, no allocation.

2. Wasted startup work — line 77.
   `let _answers = rt.answer(&query, 15);` discarded. Either drop, or seed
   the Search tab results with the answer set (cheap; reuses warm query).
   Choice: seed search_results at load so Overview→Search is first-class.

3. Dead Invariants tab — line 94, 429-437.
   `invariants_report` stays empty; renders "(none)"-like empty list forever.
   Collect invariants via a pure helper over `tests/invariants/` (path + names)
   at load. Unit test the helper in `tests/snapshot.rs`.

4. "Live" tab not live — line 449-457.
   Reads `rt.snap.observations` (same as History). Re-observe on `r` key and
   show age of snapshot in the tab title.

5. Per-frame allocations (tolerable, bundle with fixes only if cheap):
   - `diff_report.stat.clone()` line 386 — cache the formatted line, not the struct.
   - snippet `chars().take(60)` line 345 — pre-snippet the search results once on Enter.

## Settings tab — NO CHANGE
`e/s/r/d` over `RuntimeConfig::save_dirty` + pure `stage_cfg_edit`, already
tested (`stage_edit_sets_and_marks_unsaved`, `stage_edit_rejects_garbage`).
Leave.

## Test additions
- `tests/snapshot.rs`: pure helper test for invariants collection.
- Keep `--max-files`-bound snapshot in CI; Overview + Settings snapshots
  must not regress.

## Gate
```
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo run -p physis-hud -- --snapshot            # overview
cargo run -p physis-hud -- --snapshot --view settings
```
