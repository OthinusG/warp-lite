# Antigravity CLI Support

## Goal

Recognize `agy` as the Antigravity CLI agent and expose the existing CLI Agent footer and management experience without enabling Warp AI, Cloud, collaboration, login, remote control, or telemetry uploads.

## Acceptance Criteria

- `agy` is detected as `CLIAgent::Antigravity` and classified as a shell command.
- The UI displays `Antigravity` with `AntigravityLogo` and the bundled SVG.
- Rich input uses the ordinary inline fallback; listener and plugin manager remain unsupported.
- The downstream restoration script reapplies and verifies the integration after upstream syncs.
- Focused tests and the default Warp Lite build check pass.
