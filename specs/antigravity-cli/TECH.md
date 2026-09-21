# Antigravity CLI Support — Technical Specification

## Approach

Mirror the current `warpdotdev/warp` master implementation inside the existing `CLIAgent` enum and exhaustive matches. Add only the telemetry enum variant required by the local type conversion; do not add or restore telemetry transport.

Persist the downstream delta as `.github/patches/antigravity-cli.patch` and apply it through the existing restoration script and sync workflow.

## Constraints

- No dedicated listener or plugin manager exists upstream; both remain `None`.
- No new dependencies, network clients, feature flags, services, or product surfaces.
- Preserve all existing Warp Lite privacy boundaries.

## Verification

- Test `agy` CLI-agent detection and shell-command classification.
- Check patch reversibility and script syntax.
- Run `cargo check -p warp --bin warp-oss` and targeted tests.
