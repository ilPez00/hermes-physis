#!/usr/bin/env bash
# Release gate for the physis-core crate. Prints WHY it failed; never a silent
# skip. Run from the crate root:  bash scripts/check-release.sh
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

version=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
[ -n "$version" ] || fail "no version in Cargo.toml"

# 1. README and CHANGELOG must name the version being released.
grep -qF "version = \"$version\"" Cargo.toml || fail "Cargo.toml does not declare $version"
grep -qF "physis-core = \"$version\"" README.md \
  || fail "README.md does not offer dependency physis-core = \"$version\""
grep -qF "[$version]" CHANGELOG.md \
  || fail "CHANGELOG.md has no [$version] section"

# 2. HEAD must carry the crate release tag (crates.io versions are immutable).
tag="physis-core-v$version"
if ! git tag --points-at HEAD | grep -qx "$tag"; then
  fail "HEAD does not carry $tag (at HEAD: $(git tag --points-at HEAD | tr '\n' ' '))"
fi

# 3. A clean tree is reproducible; a dirty one is not what will publish.
[ -z "$(git status --porcelain --untracked-files=no)" ] \
  || fail "working tree has uncommitted changes (cargo publish would refuse)"

# 4. Capability claims must have both halves running (NOT MEASURED is a failure
#    here — an unchecked capability is not a passing one).
if command -v physis-check >/dev/null 2>&1; then
  out=$(physis-check capabilities . 2>&1) || fail "capability check exited nonzero"
  printf '%s\n' "$out" | grep -q "capability(ies) have both halves running" \
    || fail "capability check did not run to completion:\n$out"
  printf '%s\n' "$out" | tail -1
else
  printf 'NOT MEASURED: physis-check not on PATH — capability claims unchecked\n'
fi

printf 'OK: physis-core %s is release-ready (tag %s at HEAD)\n' "$version" "$tag"
