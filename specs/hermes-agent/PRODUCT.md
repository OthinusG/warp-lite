# Hermes Agent Detection and Icon Restoration

## Goal

Provide full CLI Agent session support and authentic brand presentation for Nous Research Hermes Agent in Warp Lite.

## Behavior

- Recognize `hermes`, `hermes-agent`, and commands with additional arguments.
- Preserve environment-assignment and alias resolution before detection.
- Display `Hermes` with its official brand logo in CLI agent tabs and status circles.
- Submit rich input with bracketed paste.

## Boundaries

- No structured status listener, notification plugin, Warp AI, Cloud, login, collaboration, or telemetry upload.

## Acceptance Criteria

- Positive detection cases and alias/env-var cases have regression coverage.
- `hermes` is classified as a shell command keyword.
- Hermes logo icon renders cleanly with transparent background over Hermes purple circle.
- The customization is reapplied by the existing upstream-sync restoration script.
