# Warp Lite MCP Runtime Removal

## Goal

The default Warp Lite app must not scan MCP configuration files, start Warp-managed MCP servers, or expose Warp's MCP management page. CLI agents launched in a terminal retain their own MCP behavior and configuration.

## Scope and constraints

- Gate Warp MCP startup and UI construction behind the existing `warp_platform` feature. Keep the platform build compiling.
- Keep only an inert manager singleton in the Lite build where existing compiled Warp AI code still expects that type. Do not migrate, load credentials, connect, or start servers in Lite.
- Preserve CLI agent detection, footer, notification plugins, and DeepSeek Harness integration.
- Replay the change after upstream sync through an auditable patch/script, not a client-only edit.
- Do not restore Warp AI, cloud, account, telemetry transport, or collaboration code.

## Acceptance and verification

1. In Lite startup, no file MCP watcher, file-based server manager, or MCP gallery manager is registered; no live MCP manager constructor is called.
2. Lite settings do not construct or navigate to the Warp MCP page.
3. Default and `warp_platform` Rust checks pass; CLI agent tests still pass.
4. The restoration script is idempotent, passes shell syntax checks, and applies the patch after an upstream merge.
5. CI builds and publishes the requested macOS release.

Manual rebuilds use a run-specific release tag. The previous same-tag asset replacement failed with GitHub API 404; preserving old releases avoids losing their artifacts or download history.

## Risk

Other Warp AI modules retain compile-time references to the MCP manager. Removing the type outright would require a much broader AI subsystem deletion; the Lite-only inert singleton avoids missing-singleton panics without activating MCP behavior.
