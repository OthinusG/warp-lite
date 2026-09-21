# Restore Project Explorer: Technical Plan

## Existing Architecture

- `HeaderToolbarItemKind::ToolsPanel` controls toolbar availability and placement.
- `Workspace::render_header_toolbar_button` renders the toggle.
- `Workspace::render_config_panel` renders `LeftPanelView` on the configured side.
- `LeftPanelView` already owns `FileTreeView`; `CodeSettings::show_project_explorer` defaults to `true`.

## Implementation

1. Restore the upstream-native Tools Panel connections in `header_toolbar_item.rs` and `view.rs`, including persisted-toolbar migration.
2. Store that focused diff as `.github/patches/project-explorer.patch`.
3. Add an idempotent shell script that:
   - exits successfully when the patch is already applied;
   - applies it when the disabled upstream state is present;
   - fails on unexpected drift.
4. Add one GitHub Actions workflow that fetches and merges the latest upstream `v*-lite` tag, runs the restoration script, pushes the restored source, builds the app, then publishes artifacts under the same tag name.

## Constraints And Risks

- Upstream merge conflicts or source drift must fail closed.
- Unreleased upstream branch commits are ignored so downstream release versions stay aligned.
- The workflow uses the repository-scoped `GITHUB_TOKEN`; no new secret is required.
- Artifacts are ad-hoc signed, matching the existing local packaging script; Apple notarization is out of scope.
- The macOS runner explicitly selects full Xcode, verifies its Metal compiler, and installs `protoc` before compiling.
- A scheduled release can consume substantial macOS runner time, so unchanged upstream revisions exit before building.

## Verification

- `bash -n script/restore-project-explorer.sh`
- restoration script check mode against the patched tree
- focused Rust formatting
- focused workspace/UI tests where build resources permit
- `cargo check -p warp --bin warp-oss`
- GitHub Actions syntax and action pin review
