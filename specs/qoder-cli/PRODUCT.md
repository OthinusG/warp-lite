# Qoder CLI Agent Detection

## Goal

Recognize Qoder CLI commands as CLI Agent sessions so Warp Lite shows its existing toolbar, pane management, and rich input.

## Behavior

- Recognize `qoder`, `qodercli`, `qoder-cli`, and commands with additional arguments.
- Preserve environment-assignment and alias resolution before detection.
- Display `Qoder` and submit rich input inline.

## Boundaries

- No structured status listener, notification plugin, Warp AI, Cloud, login, collaboration, or telemetry upload.

## Acceptance Criteria

- Positive detection cases and alias/env-var cases have regression coverage.
- `qoder` is classified as a shell command keyword.
- The customization is reapplied by the existing upstream-sync restoration script.
