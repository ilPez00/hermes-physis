# Third-party notices

This plugin ships binaries and source snapshots of two separate projects. Both
are Apache-2.0; their LICENSE and NOTICE files are preserved verbatim.

## physis-core 0.2.0

- Upstream: https://github.com/ilPez00/physis-core
- License: Apache-2.0 (`vendor/physis-core/LICENSE`)
- Bundled as: `bin/physis-core` + `vendor/physis-core/`
- Build: `cargo build --release --manifest-path vendor/physis-core/Cargo.toml`

## bge-small-en-v1.5 (embedding model)

- Upstream: https://huggingface.co/BAAI/bge-small-en-v1.5
- Export: Xenova/bge-small-en-v1.5 `model_uint8.onnx` + `tokenizer.json`
- License: MIT (`models/bge-small/LICENSE`)
- Bundled as: `models/bge-small/` so semantic embeddings work offline

## physis-replant 0.3.0

- Upstream: https://github.com/ilPez00/physis-replant
- License: Apache-2.0 (`vendor/physis-replant/LICENSE`, `NOTICE`)
- Bundled as: `bin/replant` + `vendor/physis-replant/`
- Build: `cargo build --release --manifest-path vendor/physis-replant/Cargo.toml`

The plugin's own code (`__init__.py`, `tools.py`, `schemas.py`, `engine.py`,
`examples/replant_fixture.py`, `tests/`, `skills/`) is MIT.
