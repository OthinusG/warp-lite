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
- Project Explorer source remains in the tree. Warp Lite disabled only its `ToolsPanel` entry point; the focused restoration is stored in `.github/patches/project-explorer.patch` and replayed by `script/restore-project-explorer.sh`.
- Workspace initialization restores the right-side Tools Panel button for older persisted toolbar configurations that omit it.
- `.github/workflows/sync-upstream-warp-lite.yml` syncs only the latest upstream `v*-lite` release tag, verifies before push, and publishes ad-hoc-signed macOS ZIP/DMG artifacts under the same tag name.
- macOS packaging uses Warp's native 1024×1024 `app/DockTilePlugin/Resources/mono.png`; manual workflow dispatches intentionally rebuild and replace the current release assets even when the upstream tag is unchanged.
