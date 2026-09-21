#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCHES=(
    "$ROOT/.github/patches/project-explorer.patch"
    "$ROOT/.github/patches/antigravity-cli.patch"
    "$ROOT/.github/patches/deepseek-harness.patch"
)

cd "$ROOT"

for patch in "${PATCHES[@]}"; do
    if git apply --unidiff-zero --reverse --check "$patch" 2>/dev/null; then
        echo "$(basename "$patch") is already applied."
    elif [[ "$(basename "$patch")" == "antigravity-cli.patch" ]] \
        && git apply --unidiff-zero --reverse --check \
            "$ROOT/.github/patches/deepseek-harness.patch" 2>/dev/null; then
        echo "$(basename "$patch") is already applied as a DeepSeek Harness prerequisite."
    elif git apply --unidiff-zero --check "$patch" 2>/dev/null; then
        if [[ "${1:-}" == "--check" ]]; then
            echo "$(basename "$patch") can be applied."
        else
            git apply --unidiff-zero "$patch"
            echo "$(basename "$patch") restored."
        fi
    else
        echo "$(basename "$patch") no longer matches upstream; refusing to modify the tree." >&2
        exit 1
    fi
done
