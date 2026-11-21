# Observer fusion — real-corpus measurement harness

This directory is the evidence behind the observer-fusion constants shipped in
`physis-core` (semantic floor 0.66, auto reserve, assembled-packet accounting,
per-path pricing). Everything here is reproducible from a fresh clone.

## What is here

- `measure.py` — the sweep harness. Per query it runs: baseline packet (no
  claims), fused packet (relevant claim + 2 noise claims) at the chosen
  floor/reserve, and a raw-relevance probe at floor 0.0. Verdicts are
  packet-level distinctive-phrase presence; relevances come from the engine's
  own `observer_gated` receipts.
- `corpus/` — the 24 engine-visible real physis-next plan docs
  (`2026-09-24` … `2026-10-09`).
- `results-*.json` — recorded measurements. `results-n22.json` is the n=22
  held-out / n=46 noise sweep backing the 0.66 default; `results-repro.json`
  is the re-run that reproduced it 23/23 exactly on 2026-11-20. The
  `results-auto/100/250/500` files back the auto-reserve dominance result;
  `results-floor-*` and `results-rp-auto` back the floor placement and the
  RP-ships-disabled verdict.
- The full narrative lives in `../observer-fusion-real-corpus-2026-11-20.md`.

## How to run

```sh
# semantic embedder (production bge-small), default floor, auto reserve:
PHYSIS_BENCH_MODEL_DIR=/path/to/bge-small python3.13 measure.py auto

# explicit floor:
python3.13 measure.py auto 0.66

# random-projection embedder (no PHYSIS_MODEL_DIR — fusion is opt-in there):
python3.13 measure.py auto --rp
```

Overrides: `PHYSIS_BENCH_ENGINE` (default `~/.local/bin/physis-core`),
`PHYSIS_BENCH_CORPUS` (default the bundled `corpus/`),
`PHYSIS_BENCH_MODEL_DIR` (default the operator's bge-small path). Output goes
to stdout as JSON; each run is a full-fidelity receipt (relevances, decisions,
token counts, phrase checks).

## What each result field means

- `baseline_phrase_present` — control that the engine alone cannot answer a
  held-out query (must be `false` for held-out, `true` for the control query).
- `fused_phrase_present` — the fused packet contains the held-out doc's
  distinctive phrase: the observer fusion answered the query.
- `observer_selected` — how many observer snippets entered the packet.
- `noise_admitted` / `noise_text_in_packet` — a noise claim passed the gate /
  its text is verbatim in the packet. Must be 0 at the shipped default.
- `relevances` — raw cosine per claim from the floor-0.0 probe (the ROC data).
