#!/usr/bin/env bash
# Replay the native communication feature after existing CLI restorations.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCH="$ROOT/.github/patches/agent-communication.patch"
case "${1:-}" in ""|--check) ;; *) echo "Usage: $0 [--check]" >&2; exit 2 ;; esac
cd "$ROOT"
if git apply --reverse --check "$PATCH" 2>/dev/null; then
    echo "Agent communication is already restored."
elif git apply --check "$PATCH" 2>/dev/null; then
    if [[ "${1:-}" == --check ]]; then
        echo "Agent communication patch can be applied."
    else
        git apply "$PATCH"
        git apply --reverse --check "$PATCH"
        echo "Agent communication restored."
    fi
else
    echo "Agent communication patch conflicts or is partially applied; refusing to continue." >&2
    exit 1
fi
