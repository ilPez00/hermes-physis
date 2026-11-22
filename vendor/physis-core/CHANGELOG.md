# Changelog

## [0.4.0] — unreleased

### Added
- `physis-core selftest` — the engine audits its own fusion gate; `--baseline`
  records a receipt-drift watch for later runs (`a54e0b9`, `d166958`).
- `calibrate-floor`: per-corpus, advisory floor placement (`da1a43f`).
- `engine_relative` diagnostic on observer gate receipts (`8da3d33`).
- Probe `expect answer|defer` — selftest verifies the measurement record; full
  22-probe set shipped (`7cb52ed`).

### Changed
- Gate reasons prefer `GateReasonCode`; added stable `GateReasonCode` to gate
  results and a mapping-drift sentinel test (`10805a1`, `adb1e11`,
  `5dceeb0`).
- Deterministic reason-count rollup merge; single-pass gate outcome
  accumulation (`3cde7a1`, `0ce0a08`).
- `ws14` observer-eval: deterministic summary rollup merge, fixture ablation
  and report helpers, coverage hardening (`d3a659d`, `25c477a`,
  `7bdbe3d`).
- `fix(core)`: model-aware default observer floor — the kind-level semantic
  default admitted noise on bge-base and minilm (`559af42`).
- `revert(core)`: sync-bot injection reverted from the 0.3.0 release commit
  (`e6c014e`).

### Fixed
- Gate outcome accumulation no longer double-counts across rollup boundaries.

## [0.3.0] — 2026-XX-XX

First crates.io candidate. Deterministic structural mapping, token-fixed RAG,
semantic embeddings, optional ONNX MiniLM embedder, `cli`/`studio`/`embed-onnx`
feature flags, `physis` dispatch binary and `physis-core` library binary.
