#!/usr/bin/env bash
# Prepare/publish this tree as a new GitHub repo named hermes-physis.
# Review the tree first; this creates the remote and pushes.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OWNER="${GITHUB_OWNER:-ilPez00}"
REPO="${REPO_NAME:-hermes-physis}"

git status --short
echo "Checksums:"; cat bin/SHA256SUMS
read -r -p "Create ${OWNER}/${REPO} and push main? [y/N] " ok
[[ "$ok" == "y" || "$ok" == "Y" ]] || exit 1

gh repo create "${OWNER}/${REPO}" --public --source . --remote origin --push
VERSION="$(awk '/^version:/{print $2; exit}' plugin.yaml)"
git tag -a "v${VERSION}" -m "hermes-physis v${VERSION}"
git push origin "v${VERSION}"
echo "Published ${OWNER}/${REPO} v${VERSION}"
