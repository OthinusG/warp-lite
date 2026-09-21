# DeepSeek Harness CLI Agent Detection

## Goal

Recognize DeepSeek Harness TUI commands as CLI Agent sessions so Warp Lite shows its existing toolbar, pane management, and rich input.

## Behavior

- Recognize `dsh-tui`, `dsh --profile tui`, `dsh --profile=tui`, and TUI profile commands with additional arguments.
- Preserve environment-assignment and alias resolution before detection.
- Treat non-TUI modes, help, and plugin management as ordinary shell commands.
- Display `DeepSeek Harness` and submit rich input inline.

## Boundaries

- No structured status listener, notification plugin, ACP integration, Warp AI, Cloud, login, collaboration, or telemetry upload.

## Acceptance Criteria

- Positive and negative detection cases have regression coverage.
- `dsh` is classified as a shell command keyword.
- The customization is reapplied by the existing upstream-sync restoration script.
