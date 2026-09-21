#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCH="$ROOT/.github/patches/project-explorer.patch"

cd "$ROOT"

if git apply --unidiff-zero --reverse --check "$PATCH" 2>/dev/null; then
    echo "Warp Lite customizations are already applied."
elif git apply --unidiff-zero --check "$PATCH" 2>/dev/null; then
    if [[ "${1:-}" == "--check" ]]; then
        echo "Warp Lite customizations can be applied."
    else
        git apply --unidiff-zero "$PATCH"
        echo "Warp Lite customizations restored."
    fi
else
    echo "Warp Lite customizations no longer match upstream; refusing to modify the tree." >&2
    exit 1
fi
