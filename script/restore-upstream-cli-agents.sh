#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCH="$ROOT/.github/patches/upstream-third-party-cli-agents.patch"
CURSOR_PATCH="$ROOT/.github/patches/cursor-cli-command.patch"
UPSTREAM_REPOSITORY="${WARP_UPSTREAM_REPOSITORY:-https://github.com/warpdotdev/warp.git}"
UPSTREAM_REF="refs/remotes/warp-lite-upstream/master"
CHECK_ONLY=false
[[ "${1:-}" == "--check" ]] && CHECK_ONLY=true

SAFE_PATHS=(
    app/assets/bundled/svg
    app/src/server/telemetry/events.rs
    app/src/terminal/cli_agent.rs
    app/src/terminal/cli_agent_sessions/listener/mod.rs
    app/src/terminal/cli_agent_sessions/plugin_manager
    app/src/terminal/cli_agent_tests.rs
    app/src/terminal/view/use_agent_footer/mod.rs
    crates/input_classifier/src/util.rs
    crates/input_classifier/src/util_tests.rs
    crates/warp_core/src/ui/icons.rs
)
VETTED_VARIANTS=(OhMyPi Goose Hermes Vibe Grok Antigravity)

cd "$ROOT"
git fetch --no-tags "$UPSTREAM_REPOSITORY" "master:$UPSTREAM_REF"

upstream_source="$(git show "$UPSTREAM_REF:app/src/terminal/cli_agent.rs")"
extract_variants() {
    sed -n '/pub enum CLIAgent {/,/^}/p' \
        | sed -E -n 's/^[[:space:]]*([A-Za-z][A-Za-z0-9_]*),.*/\1/p'
}
upstream_variants="$(printf '%s\n' "$upstream_source" | extract_variants)"

for agent in "${VETTED_VARIANTS[@]}"; do
    if ! grep -qx "$agent" <<<"$upstream_variants"; then
        echo "Upstream no longer supports $agent; refusing to restore a stale integration." >&2
        exit 1
    fi
done

if git apply --unidiff-zero --reverse --check "$PATCH" 2>/dev/null \
    || { grep -Fq 'CLIAgent::OhMyPi => &["omp"]' app/src/terminal/cli_agent.rs \
        && grep -Fq 'CLIAgent::Grok => &["grok"]' app/src/terminal/cli_agent.rs; }; then
    echo "$(basename "$PATCH") is already applied."
elif git apply --unidiff-zero --check "$PATCH" 2>/dev/null; then
    if $CHECK_ONLY; then
        echo "$(basename "$PATCH") can be applied."
    else
        git apply --unidiff-zero "$PATCH"
        echo "$(basename "$PATCH") restored."
    fi
else
    echo "$(basename "$PATCH") no longer matches this Warp Lite revision." >&2
    exit 1
fi

if git apply --unidiff-zero --reverse --check "$CURSOR_PATCH" 2>/dev/null \
    || { grep -Fq 'CLIAgent::CursorCli => &["agent", "cursor-agent"]' app/src/terminal/cli_agent.rs \
        && grep -Fq '"cursor-agent"' crates/input_classifier/src/util.rs; }; then
    echo "$(basename "$CURSOR_PATCH") is already applied."
elif git apply --unidiff-zero --check "$CURSOR_PATCH" 2>/dev/null; then
    if $CHECK_ONLY; then
        echo "$(basename "$CURSOR_PATCH") can be applied."
    else
        git apply --unidiff-zero "$CURSOR_PATCH"
        echo "$(basename "$CURSOR_PATCH") restored."
    fi
else
    echo "$(basename "$CURSOR_PATCH") no longer matches this Warp Lite revision." >&2
    exit 1
fi

local_variants="$(extract_variants < app/src/terminal/cli_agent.rs)"
for agent in "${VETTED_VARIANTS[@]}" DeepSeekHarness Qoder; do
    local_variants="$local_variants
$agent"
done

pending=""
while IFS= read -r agent; do
    case "$agent" in
        WarpTui|Unknown) continue ;;
    esac
    grep -qx "$agent" <<<"$local_variants" && continue

    commit="$(git log --reverse --format=%H -S "    $agent," "$UPSTREAM_REF" -- \
        app/src/terminal/cli_agent.rs | head -n 1)"
    if [[ -z "$commit" ]]; then
        echo "Could not locate the upstream introduction commit for $agent." >&2
        exit 1
    fi
    timestamp="$(git show -s --format=%ct "$commit")"
    pending+="$timestamp $commit $agent"$'\n'
done <<<"$upstream_variants"

[[ -z "$pending" ]] && exit 0

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/warp-lite-cli-agents.XXXXXX")"
trap 'rm -rf "$temp_dir"' EXIT

while read -r _ commit agent; do
    [[ -z "${agent:-}" ]] && continue
    grep -qx "$agent" < <(extract_variants < app/src/terminal/cli_agent.rs) && continue

    candidate="$temp_dir/$agent.patch"
    git diff "$commit^" "$commit" -- "${SAFE_PATHS[@]}" > "$candidate"
    if [[ ! -s "$candidate" ]]; then
        echo "No safe-path integration diff found for $agent in $commit." >&2
        exit 1
    fi

    if grep -E '^\+[^+].*(WarpTui|send_telemetry|TelemetryEvent::|reqwest|warp_server_client|graphql|firebase|remote[_-]control|cloud[_-]agent|billing|login)' "$candidate" >/dev/null; then
        echo "Rejected unsafe upstream integration candidate for $agent ($commit)." >&2
        exit 1
    fi

    if ! git apply --3way --check "$candidate" >/dev/null 2>&1; then
        echo "Upstream integration for $agent requires manual conflict review ($commit)." >&2
        exit 1
    fi

    if $CHECK_ONLY; then
        echo "$agent can be restored from upstream commit $commit."
    else
        git apply --3way "$candidate"
        echo "$agent restored from upstream commit $commit."
    fi
done < <(printf '%s' "$pending" | sort -n)
