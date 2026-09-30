# Native MCP Coverage

Checked on 2026-09-30 against the restored `CLIAgent` enum. Native MCP client support is the eligibility boundary. MCP server mode alone does not qualify. No shell fallback or third-party MCP adapter is included. Dormant native prompts are woken through Warp's existing per-agent submission strategies once the common readiness signal or installed completion hook establishes idle state.

`Eligible` means that native local MCP support is established by vendor documentation or installed CLI help. It does not mean authenticated live-agent communication has passed. Every row still needs the end-to-end acceptance below in a patched Warp build.

| Managed type | Recognized commands | Native MCP evidence | Eligibility |
| --- | --- | --- | --- |
| Claude Code | claude | Installed `claude mcp add --help` | Eligible |
| Gemini | gemini | [Official MCP guide](https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md) | Eligible |
| Codex | codex | Installed `codex mcp add --help` | Eligible |
| Amp | amp | [Official MCP guide](https://ampcode.com/docs/customize/mcp) | Eligible; local CLI only |
| Droid | droid | [Official connectors documentation](https://docs.factory.ai/harness/connectors) | Eligible |
| OpenCode | opencode | [Official MCP configuration](https://opencode.ai/docs/mcp-servers/) | Eligible |
| Copilot | copilot | [Official CLI MCP setup](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers) | Eligible |
| Pi | pi | [Current official native MCP documentation](https://pi.dev/docs/latest/mcp) | Eligible on a version with native MCP |
| oh-my-pi | omp | [Maintainer's native MCP configuration](https://github.com/can1357/oh-my-pi/blob/main/docs/config-usage.md) | Eligible |
| Auggie | auggie | [Official CLI integrations](https://docs.augmentcode.com/cli/integrations) | Eligible |
| Cursor | agent, cursor-agent | [Official CLI offering](https://cursor.com/en-US/cli), [CLI ACP MCP support](https://prod.cursor.com/docs/cli/acp) | Eligible |
| Goose | goose | [Maintainer's extension documentation](https://github.com/aaif-goose/goose/blob/main/documentation/docs/getting-started/using-extensions.md) | Eligible |
| Hermes | hermes, hermes-agent | [Official MCP integration](https://hermes-agent.nousresearch.com/docs/user-guide/features/mcp) | Eligible |
| Mistral Vibe | vibe, vibe-acp | [Official native stdio configuration implementation](https://github.com/mistralai/mistral-vibe/blob/main/vibe/core/config/mcp_servers.py) | Eligible |
| Antigravity | agy | Installed `agy mcp add --help`: native stdio transport | Eligible |
| Grok Build | grok | [Official MCP servers](https://docs.x.ai/build/features/mcp-servers) | Eligible |
| DeepSeek Harness | dsh-tui; dsh with a TUI profile | [Official MCP client package](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/mcp/mcp-client/README.md) | Eligible when the profile mounts the official MCP client |
| Qoder | qoder, qodercli, qoder-cli, qodercn | [Official CLI MCP reference](https://docs.qoder.com/cli/mcp-reference); installed qodercn help confirms stdio | Eligible |
| Trae | trae, traecn, trae-cli, traecn-cli | [Official CLI MCP documentation](https://docs.trae.cn/cli_model-context-protocol) | Eligible on the documented native MCP version; launch-name compatibility to verify |
| Custom/Unknown | User-defined detection patterns | Detection alone does not establish MCP capability | Eligible individually after confirming native MCP; common custom binding provided |

## Excluded or unverified

- Installed versions lacking native MCP client support, including older Pi releases that require an external MCP extension. Upgrade rather than ship a fallback adapter.
- Custom managed agents whose native MCP capability has not yet been confirmed are pending verification. Confirmed native MCP clients use the common `custom` binding and are included; they are not excluded based on the `Unknown` enum value.
- HTTP-only clients cannot use this release's local stdio bridge until native stdio capability is established. No network listener is added for them.
- Cloud Amp orbs are outside the local Warp terminal boundary.
- Trae's current executable aliases require configuration verification before an adapter is marked ready. The newly documented `traecli` command is not in the current Warp detector; do not count it as a managed command without an explicit detector change.

No currently listed named type has been conclusively established as having no native MCP support in its current release. Do not invent an exclusion list from older product knowledge.

## Settings-managed MCP configuration

Use **Settings > Features > Agent communication**. The switch is off by default. Enabling it discovers installed managed commands and their aliases. Check an available agent to configure the bundled stdio bridge in the background. No manual registration prompt, `mcp add`, or environment editing is required. Uncheck to revoke access and remove the managed entry; turning the switch off revokes every active participant before cleanup. Failed cleanup is visible, stays unauthorized, and can be retried with **Refresh agents / retry cleanup**.

Warp owns only `warp-lite-communication`, not a user's existing `warp-agent` server or unrelated MCP configuration. JSON/YAML updates preserve unrelated values; TOML uses a marked block and preserves existing text. Malformed configurations, ownership collisions, user edits to the managed entry, and symlinks stop the operation. Preferences and cleanup metadata are machine-local. The packaged bridge is `/Applications/WarpLite.app/Contents/MacOS/warp-agent` on macOS and `warp-agent.exe` beside Warp Lite on Windows.

The bridge inherits `WARP_AGENT_ENDPOINT`, `WARP_AGENT_CAPABILITY`, and `WARP_TERMINAL_SESSION_UUID` from its Warp terminal. Codex receives `env_vars` name-based passthrough automatically. Gemini receives runtime variable references. Values are never persisted in configuration or metadata. Other native stdio clients inherit the terminal environment through their native subprocess contract. A process launched outside a managed Warp terminal cannot participate.

Documented adapters cover Codex, Claude, Gemini, OpenCode, Amp, Cursor, Copilot, Droid, Auggie, native-MCP Pi, OMP, Hermes, Goose, and Vibe. Remaining installed managed commands, including Qoder/QoderCN, Antigravity, Trae and custom literal toolbar commands, are probed for native `mcp add`, inspection, and removal contracts. A missing/unsupported contract is visibly unavailable rather than silently claiming configuration success. Non-default vendor profiles and project overrides can supersede user configuration; bridge discovery remains the final participation check.

Restart running clients that do not hot-reload MCP configuration. A fresh managed CLI run loads the bridge and receives a unique project-local identity automatically. All live enabled agents in the same canonical project can discover, message, and delegate to one another; there is no peer picker. Repository subdirectories share scope; separate repositories and worktrees remain isolated.

The cooperation instructions require `warp_agent_ready` as the last action before ending a turn. Warp polls every 250 ms, waits for output to settle, and automatically submits queued inbox work when eligible. Busy/draft/permission protections remain unchanged. Removal of MCP/environment configuration takes effect in a running vendor client after reload/restart; broker revocation takes effect immediately.

Adapter references: [Pi](https://pi.dev/docs/latest/mcp), [OMP](https://github.com/can1357/oh-my-pi/blob/main/docs/mcp-config.md), [Hermes](https://hermes-agent.nousresearch.com/docs/user-guide/features/mcp), [Goose](https://github.com/aaif-goose/goose/blob/main/documentation/docs/getting-started/using-extensions.md), [Vibe](https://docs.mistral.ai/vibe/code/cli/mcp-servers), [Auggie](https://docs.augmentcode.com/cli/integrations).

## Acceptance and status

GitHub tests simulate the restored detector's managed program identities plus a managed custom client, exercise real local sockets, negotiate the pinned MCP SDK over child-process stdio, and verify the shared task state machine. These tests do not launch vendor models.

For each eligible vendor/version, separately verify: bridge discovery with all 12 tools; inherited terminal binding; unique-name registration; cross-agent messages; a completed turn returning to the input prompt, then new work automatically being submitted without an open wait call or human prompt; busy task queueing; submission with evidence; reviewer rejection and revised rework; final acceptance; and rejection after CLI exit/replacement. Live model acceptance is pending until the patched build and each authenticated vendor runtime are available.

Qodercn is a confirmed native stdio MCP client: installed `qodercn mcp add --help` exposes the `stdio` transport, and Warp maps its command to `CLIAgent::Qoder`. It is not part of the unverified custom-agent exclusions. No personal MCP configuration was read during this check.
