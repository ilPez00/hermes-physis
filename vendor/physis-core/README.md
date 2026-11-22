# physis-core

**An engine that maintains competing interpretations of reality, evaluates their
coherence against observations, processes, evidence and outcomes, and
preferentially retains the interpretations that continue to work.**

Built on an adaptive semiotic grid: deterministic structural mapping (repeats,
differences, contradictions), token-fixed RAG, semantic embeddings, and
interchangeable model / n-gram backends. Offline, no API key.

[![crates.io](https://img.shields.io/crates/v/physis-core.svg)](https://crates.io/crates/physis-core)
[![docs.rs](https://img.shields.io/docsrs/physis-core)](https://docs.rs/physis-core)

## What it is

The crate is a **coordinate system plus operations over it**, not a single
function:

- **The grid** — an ontology of domain × mode cells; text classifies into cells,
  and the grid's shape adapts as new domains are discovered (`grid`, `ontology`,
  `facet`, `discover`). It is validated against a drift invariant, so an
  unknown relation verb is refused at the boundary rather than relabelled.
- **The world** — a persisted, append-only observation graph (`store`,
  `observe`, `observer`, `watch`, `ingest`, `vault`, `history`). Re-running over
  an unchanged tree produces nothing.
- **Coherence & truth maintenance** — competing hypotheses are scored across
  five dimensions (semantic fit, ontological grounding, logical consistency,
  empirical support, process coherence); contradictions are preserved as typed
  `Contradiction` records and beliefs revise without deleting history
  (`hypothesis`, `contradiction`, `temporal`, `revision`, `history`).
- **Context compilation** — fixed-budget structural context for a query against
  a corpus, with the measured compression over conventional retrieval
  (`compile`, `context`, `chain`, `coverage`).
- **Classification & linkage** — `classify`, `linkage`, `coverage`, `embed`
  (ONNX MiniLM optionally; deterministic random-projection otherwise),
  `system_one` (calibrated decision heads), `calibrate-floor`,
  `observer_eval`.

## Quick start (library)

```toml
[dependencies]
physis-core = "0.4.0"
```

Dependency-light embedders can drop the default CLI/studio/ONNX surface:

```toml
[dependencies]
physis-core = { version = "0.4.0", default-features = false }
```

## Quick start (binary)

```bash
cargo install physis-core

physis-core demo --dir <corpus>  # corpus → map → n-gram table → model answer, offline
physis-core grid                 # the adaptive semiotic grid, its axes and cells
physis-core scan .               # register this tree as a persisted coherence graph
physis-core context --corpus . --query "how does revision work" --budget 800
physis-core ground               # what is believed, on what evidence, what is disputed
```

`physis-core selftest --corpus <dir> --probes <file.json>` runs known-answer
probes and exits nonzero on any missed answer or admitted noise — usable as a CI
gate. `calibrate-floor` reports where the zero-noise floor sits on *your* corpus
as a measurement receipt, never a directive.

## Feature flags

| flag | default | enables |
|---|---|---|
| `cli` | yes | the `physis-core` binary |
| `studio` | yes | embedded axum GUI (`physis-core studio`) |
| `embed-onnx` | yes | ONNX MiniLM embedder via `ort`; falls back to random projection with a warning |
| `http` | no | `ureq` HTTP client |
| `head-regex` | no | `regex` rules for System-One decision heads |

On a host without a model file, `embed-onnx` builds fine and falls back to a
deterministic random-projection embedder at runtime — the `embedder` line in
every receipt says which one ran, so a lexical result is never printed as a
semantic one.

## Binaries

- `physis-core` — the CLI above.
- `physis` — a dependency-light front door that dispatches to the installed
  Physis executables. Builds even with `cli` off.

## License

Apache-2.0.

## Repository

https://github.com/ilPez00/physis-core
