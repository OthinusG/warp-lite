#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCH="$ROOT/.github/patches/project-explorer.patch"

cd "$ROOT"

if git apply --unidiff-zero --reverse --check "$PATCH" 2>/dev/null; then
    echo "Project Explorer patch is already applied."
elif git apply --unidiff-zero --check "$PATCH" 2>/dev/null; then
    if [[ "${1:-}" == "--check" ]]; then
        echo "Project Explorer patch can be applied."
    else
        git apply --unidiff-zero "$PATCH"
        echo "Project Explorer restored."
    fi
else
    echo "Project Explorer patch no longer matches upstream; refusing to modify the tree." >&2
    exit 1
fi
