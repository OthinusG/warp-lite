# Project Memory

## Product Boundary

- warp-lite is an AGPL, local-first fork of Warp Terminal for macOS.
- The default product excludes AI agents, telemetry, cloud account/login, billing, and related platform surfaces.
- Project Explorer is a desired terminal-adjacent feature and must be restored without reintroducing excluded product dependencies.

## Build And Release

- Primary check: `cargo check -p warp --bin warp-oss`.
- App packaging: `script/build-warp-lite-app.sh`.
- Release artifacts include `WarpLite.app.zip` and `WarpLite.dmg`.

## Maintenance

- Upstream changes are historically selected and applied with provenance rather than merged wholesale.
- Automating upstream sync must retain privacy/product-boundary checks before merging or publishing.
- `OthinusG/warp-lite` is a fork of `terzigolu/warp-lite`; both default to `warp-lite/main` and do not use `master`.
- Project Explorer source remains in the tree. Warp Lite disabled only its `ToolsPanel` entry point; its restoration and the native Mono packaging icon are stored in `.github/patches/project-explorer.patch` and replayed by `script/restore-project-explorer.sh` after every upstream merge.
- Workspace initialization restores the right-side Tools Panel button for older persisted toolbar configurations that omit it.
- `.github/workflows/sync-upstream-warp-lite.yml` syncs only the latest upstream `v*-lite` release tag, verifies before push, and publishes ad-hoc-signed macOS ZIP/DMG artifacts under the same tag name.
- macOS packaging uses Warp's native 1024×1024 `app/DockTilePlugin/Resources/mono.png`; manual workflow dispatches create a run-specific release tag and preserve older assets because GitHub's asset endpoint returned 404 when `gh release upload --clobber` tried to replace them.
- Antigravity CLI (`agy`) support is preserved across upstream syncs by `.github/patches/antigravity-cli.patch`; it uses the standard CLI Agent fallback and does not add telemetry transport or Warp AI/Cloud dependencies.
- DeepSeek Harness TUI support is preserved across upstream syncs by `.github/patches/deepseek-harness.patch`; only `dsh-tui` and `dsh` TUI profiles enter CLI Agent management, with no listener, plugin, or telemetry upload added.
- DeepSeek Harness uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles; the standard monochrome icon renderer turned the original RGB PNG's opaque white background into a blank block.
- Qoder CLI support is preserved across upstream syncs by `.github/patches/qoder-cli.patch`; `qoder`, `qodercli`, and `qoder-cli` commands enter CLI Agent management with no listener, plugin, or telemetry upload added.
- Qoder CLI uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles, matching DeepSeek Harness.
- `script/restore-upstream-cli-agents.sh` reads `warpdotdev/warp` master during sync, restores the vetted third-party CLI Agent patch, and automatically imports newly introduced non-Warp CLI Agent commits only through audited CLI-integration paths. `WarpTui`, product/network additions, and telemetry send calls are rejected; DeepSeek Harness and Qoder CLI remain independent downstream patches.
- Cursor's CLI starts with `cursor-agent`, while upstream Warp currently recognizes only `agent`; `.github/patches/cursor-cli-command.patch` adds the official command without removing the legacy alias and is replayed by the CLI Agent sync script.
- The default Lite build does not register Warp MCP file watchers, file-based server management, or MCP gallery; it retains an inert `TemplatableMCPServerManager` singleton only for compiled Warp AI callers. The full MCP runtime remains available in `warp_platform` builds, while third-party CLI agents manage their own MCP configurations.
