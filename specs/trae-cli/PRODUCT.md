# Trae CLI Agent Detection

## Goal

Recognize ByteDance Trae CLI and domestic Trae CN commands as CLI Agent sessions so Warp Lite shows its existing toolbar, pane management, and rich input.

## Behavior

- Recognize `trae`, `traecn`, `trae-cli`, `traecn-cli`, and commands with additional arguments.
- Preserve environment-assignment and alias resolution before detection.
- Display `Trae` and submit rich input inline.

## Boundaries

- No structured status listener, notification plugin, Warp AI, Cloud, login, collaboration, or telemetry upload.

## Acceptance Criteria

- Positive detection cases and alias/env-var cases have regression coverage.
- `trae` and `traecn` are classified as shell command keywords.
- The customization is reapplied by the existing upstream-sync restoration script.
