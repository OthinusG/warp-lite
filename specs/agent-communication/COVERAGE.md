# Native MCP Coverage

Checked on 2026-09-30 against the restored `CLIAgent` enum. Native MCP client support is the eligibility boundary. MCP server mode alone does not qualify. No shell fallback, third-party MCP adapter, or dormant-TUI input injection is included.

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
| Custom/Unknown | User-defined detection patterns | Detection alone does not establish MCP capability | Excluded from automatic native binding; not classified as lacking MCP |

## Excluded or unverified

- Installed versions lacking native MCP client support, including older Pi releases that require an external MCP extension. Upgrade rather than ship a fallback adapter.
- Custom managed agents with unverified MCP capability. No claim is made that all custom agents lack MCP.
- HTTP-only clients cannot use this release's local stdio bridge until native stdio capability is established. No network listener is added for them.
- Cloud Amp orbs are outside the local Warp terminal boundary.
- Trae's current executable aliases require configuration verification before an adapter is marked ready. The newly documented `traecli` command is not in the current Warp detector; do not count it as a managed command without an explicit detector change.

No currently listed named type has been conclusively established as having no native MCP support in its current release. Do not invent an exclusion list from older product knowledge.

## MCP configuration

Configure the packaged executable as a local stdio server with `mcp` as its sole argument. On macOS the installed executable is `/Applications/WarpLite.app/Contents/MacOS/warp-agent`; on Windows it is `warp-agent.exe` beside Warp Lite. Do not configure a shell command or a cloud endpoint.

The bridge must inherit `WARP_AGENT_ENDPOINT`, `WARP_AGENT_CAPABILITY`, and `WARP_TERMINAL_SESSION_UUID` from its live Warp terminal. Clients that scrub their subprocess environment need explicit passthrough of these three names. Store variable references or name-based passthrough rules, never capability values. Codex uses `env_vars` in its MCP server definition; consult the linked vendor contract for other clients. Configuration is local to the user's machine and must not commit the live capability.

Verified installed-CLI setup command shapes (not executed on personal settings):

```sh
agy mcp add --type stdio warp-agent /Applications/WarpLite.app/Contents/MacOS/warp-agent mcp
codex mcp add warp-agent -- /Applications/WarpLite.app/Contents/MacOS/warp-agent mcp
claude mcp add --scope local warp-agent -- /Applications/WarpLite.app/Contents/MacOS/warp-agent mcp
qodercn mcp add --scope local warp-agent -- /Applications/WarpLite.app/Contents/MacOS/warp-agent mcp
```

After configuring environment passthrough, launch the CLI in a fresh patched Warp pane, register a distinct name, and ask it to follow the server's cooperation instructions. A server discovered outside a managed Warp terminal has no live binding and cannot participate.

## Acceptance and status

GitHub tests simulate the restored detector's managed program identities, exercise real local sockets, negotiate the pinned MCP SDK over child-process stdio, and verify the shared task state machine. These tests do not launch vendor models.

For each eligible vendor/version, separately verify: bridge discovery with all 11 tools; inherited terminal binding; unique-name registration; cross-agent messages; wait returning a new task without a human prompt; busy task queueing; submission with evidence; reviewer rejection and revised rework; final acceptance; and rejection after CLI exit/replacement. Live model acceptance is pending until the patched build and each authenticated vendor runtime are available.
