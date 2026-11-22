# physis-core

Ontology and classification primitives on an adaptive semiotic grid:
deterministic structural mapping (repeats, differences, contradictions),
token-fixed RAG, semantic embeddings, interchangeable model/n-gram backends.
Offline, no API key.

[![crates.io](https://img.shields.io/crates/v/physis-core.svg)](https://crates.io/crates/physis-core)
[![docs.rs](https://img.shields.io/docsrs/physis-core)](https://docs.rs/physis-core)

## Install

```toml
[dependencies]
physis-core = "0.4.0"
```

## Feature flags

| flag | on by default | what it enables |
|---|---|---|
| `cli` | yes | the `physis-core` binary |
| `studio` | yes | embedded axum GUI (`physis-core --studio`) |
| `embed-onnx` | yes | ONNX MiniLM embedder via `ort`; falls back to random projection |
| `http` | no | `ureq` HTTP client |
| `head-regex` | no | `regex` for System-One pattern rules |

Disable optional deps for a dependency-light core:

```toml
[dependencies]
physis-core = { version = "0.4.0", default-features = false }
```

## License

Apache-2.0.

## Repository

https://github.com/ilPez00/physis-core
