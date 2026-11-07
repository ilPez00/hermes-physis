# Third-party notices

This plugin ships binaries and source snapshots of two separate projects. Both
are Apache-2.0; their LICENSE and NOTICE files are preserved verbatim.

## physis-core 0.1.26

- Upstream: https://github.com/ilPez00/physis-core
- License: Apache-2.0 (`vendor/physis-core/LICENSE`)
- Bundled as: `bin/physis-core` + `vendor/physis-core/`
- Build: `cargo build --release --manifest-path vendor/physis-core/Cargo.toml`

## all-MiniLM-L6-v2 (embedding model)

- Upstream: https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2
- Export: Xenova/all-MiniLM-L6-v2 `model_uint8.onnx` + `tokenizer.json`
- License: Apache-2.0 (`models/minilm/LICENSE`)
- Bundled as: `models/minilm/` so semantic embeddings work offline

## physis-replant 0.3.0

- Upstream: https://github.com/ilPez00/physis-replant
- License: Apache-2.0 (`vendor/physis-replant/LICENSE`, `NOTICE`)
- Bundled as: `bin/replant` + `vendor/physis-replant/`
- Build: `cargo build --release --manifest-path vendor/physis-replant/Cargo.toml`

The plugin's own code (`__init__.py`, `tools.py`, `schemas.py`, `engine.py`,
`examples/replant_fixture.py`, `tests/`, `skills/`) is MIT.
