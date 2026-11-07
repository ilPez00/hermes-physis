#!/usr/bin/env bash
# Rebuild the bundled engines from the source snapshots in vendor/.
# Requires Rust 1.89+ and cargo on PATH. Binaries are Linux x86_64.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> building physis-core"
cargo build --release --manifest-path vendor/physis-core/Cargo.toml

echo "==> building replant"
cargo build --release --manifest-path vendor/physis-replant/Cargo.toml

install -m 0755 vendor/physis-core/target/release/physis-core bin/physis-core
install -m 0755 vendor/physis-replant/target/release/replant bin/replant

echo "==> checksums"
(cd bin && sha256sum physis-core replant > SHA256SUMS)
cat bin/SHA256SUMS
