#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCHES=(
    "$ROOT/.github/patches/project-explorer.patch"
    "$ROOT/.github/patches/antigravity-cli.patch"
    "$ROOT/.github/patches/deepseek-harness.patch"
)

cd "$ROOT"

patch_is_present() {
    case "$(basename "$1")" in
        antigravity-cli.patch)
            [[ -f app/assets/bundled/svg/antigravity_cli.svg ]] \
                && rg -Fq 'CLIAgent::Antigravity => "agy"' app/src/terminal/cli_agent.rs \
                && rg -Fq 'Icon::AntigravityLogo => "bundled/svg/antigravity_cli.svg"' crates/warp_core/src/ui/icons.rs \
                && rg -Fq '"agy"' crates/input_classifier/src/util.rs
            ;;
        deepseek-harness.patch)
            [[ -f app/assets/bundled/png/deepseek_harness.png ]] \
                && rg -Fq 'Self::is_deepseek_harness_tui' app/src/terminal/cli_agent.rs \
                && rg -Fq 'Icon::DeepSeekHarnessLogo => "bundled/png/deepseek_harness.png"' crates/warp_core/src/ui/icons.rs \
                && rg -Fq 'CLIAgent::DeepSeekHarness' app/src/ui_components/icon_with_status.rs
            ;;
        *) return 1 ;;
    esac
}

for patch in "${PATCHES[@]}"; do
    if git apply --unidiff-zero --reverse --check "$patch" 2>/dev/null; then
        echo "$(basename "$patch") is already applied."
    elif patch_is_present "$patch"; then
        echo "$(basename "$patch") is already applied."
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

"$ROOT/script/restore-upstream-cli-agents.sh" "${1:-}"
