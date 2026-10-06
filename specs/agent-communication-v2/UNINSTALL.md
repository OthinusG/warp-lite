# Uninstall Warpai Agent MCP

## Acceptance

- Remove only Warpai's reserved `warp-lite-communication` MCP entry from installed and previously selected CLI agents.
- Recognize entries created by older Warp/Warp Lite builds whose stdio command is `warp-agent` as well as current `warpai-agent` entries.
- Preserve every unrelated server, user-modified entry, malformed file, and symlink; report cleanup failures by agent.
- Provide a one-click Settings action that disables communication first, attempts every discovered/selected agent, and retains failed cleanup intent for retry.
- Never persist live terminal capabilities or credentials.

## Verification

- Focused setup tests cover legacy executable ownership and removal across native config formats.
- Settings action review confirms all discovered and selected agents are included and errors remain visible.
- Verify the current machine's installed agent configs contain no Warpai-owned MCP entry after cleanup.
