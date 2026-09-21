# Restore Project Explorer

## Problem

Warp Lite retains Warp's Project Explorer implementation, but its Tools Panel entry point is disabled. Users cannot discover or open the file manager from the header toolbar.

## Goal

Restore the existing Project Explorer through Warp's native Tools Panel and keep that restoration applied after updates from `terzigolu/warp-lite`.

## Requirements

- Show the Tools Panel button in the default header toolbar.
- Open the existing `LeftPanelView` and its Project Explorer; do not restore the removed custom Context Panel.
- Preserve configurable left/right panel placement.
- Sync only from the latest `v*-lite` release tag published by `terzigolu/warp-lite`.
- Stop without pushing or publishing when merge, patch, verification, build, or packaging fails.
- On success, update `warp-lite/main` and publish `WarpLite.app.zip` and `WarpLite.dmg`.

## Acceptance Criteria

- A default macOS build exposes the Tools Panel button.
- The button opens the existing Project Explorer and follows its configured side.
- The restoration script is idempotent and detects upstream drift.
- The automation supports scheduled and manual runs.
- The downstream release tag exactly matches the upstream Lite release tag.
- No AI, telemetry, account, cloud, or billing surface is restored by this feature.

## Non-goals

- Rebuilding the file tree.
- Restoring the removed Context Panel widgets.
- Automatically merging Warp's official `master` branch or unreleased Lite commits.
- Adding signing or notarization credentials.
