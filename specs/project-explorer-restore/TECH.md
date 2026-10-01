# Restore Project Explorer: Technical Plan

## Existing Architecture

- `HeaderToolbarItemKind::ToolsPanel` controls toolbar availability and placement.
- `Workspace::render_header_toolbar_button` renders the toggle.
- `Workspace::render_config_panel` renders `LeftPanelView` on the configured side.
- `LeftPanelView` already owns `FileTreeView`; `CodeSettings::show_project_explorer` defaults to `true`.

## Implementation

1. Restore the upstream-native Tools Panel connections in `header_toolbar_item.rs` and `view.rs`, including persisted-toolbar migration.
2. Maintain Project Explorer, toolbar migration and packaging directly in source.
3. Keep independent GitHub validation and macOS/Windows release workflows; tagged releases build repository source without upstream fetch/merge or patch replay.

## Constraints And Risks

- Retain privacy/product boundaries and original licensing/provenance.
- Artifacts are ad-hoc signed; Apple notarization remains out of scope.
- macOS requires full Xcode with Metal and protoc.
- Only macOS and Windows are target platforms.

## Verification

- clean checkout source and packaging checks
- focused Rust formatting
- focused workspace/UI tests where build resources permit
- `cargo check -p warp --bin warp-oss`
- GitHub Actions syntax and action pin review
