> Ownership correction (2026-10-05): Codex proxy/alias acceptance below is
> historical and superseded. Current source removes command impersonation and
> Codex user-config writes. The replacement uses explicit native per-invocation
> MCP options; local managed-launch UI and native model-level acceptance remain
> pending. See [current plan](../agent-communication-v2/TECH.md#replacement-plan-native-explicit-session-only-codex-mcp).

# Native MCP Coverage

**English technical reference** | [中文使用指南](../../docs/AGENTS.zh-CN.md)

Scope update (2026-10-03): this document records the original local collaboration
behavior/coverage. The active remote product is [SSH Remote project management](../agent-communication-v2/PLAN.md):
SSH/SFTP only, remote Linux/macOS/Windows and local macOS/Windows GUI. Local
handshake/eligibility evidence does not establish remote Agent execution or wake.
The former enrolled-device remote architecture is superseded; see
[the source cutover inventory](../agent-communication-v2/CUTOVER.md).

Checked on 2026-09-30 against the restored `CLIAgent` enum. Native MCP client support is the eligibility boundary. MCP server mode alone does not qualify. No shell fallback or third-party MCP adapter is included. Dormant native prompts are woken through Warpai's existing per-agent submission strategies once the common readiness signal or installed completion hook establishes idle state.

`Eligible` means that native local MCP support is established by vendor documentation or installed CLI help. It does not mean authenticated live-agent communication has passed. Every row still needs the end-to-end acceptance below in a patched Warpai build.

| Managed type | Recognized commands | Native MCP evidence | Eligibility |
| --- | --- | --- | --- |
| Claude Code | claude | Installed `claude mcp add --help` | Eligible |
| Gemini | gemini | [Official MCP guide](https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md) | Eligible |
| Codex | codex | Installed native `-c` / `--no-daemon` help | Eligible with native --no-daemon; automatic session MCP |
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
| Qoder | qoder, qodercli, qoder-cli | [Official CLI MCP reference](https://docs.qoder.com/cli/mcp-reference) | Eligible; separate from QoderCN |
| QoderCN | qodercn, qoderclicn | Installed QoderCN CLI help confirms native stdio | Eligible; independent discovery, configuration and selection |
| Trae | trae, traecn, trae-cli, traecn-cli, traecli | [Official CLI MCP documentation](https://docs.trae.cn/cli_model-context-protocol) | Eligible on the documented native MCP version; launch-name compatibility to verify |
| Custom/Unknown | User-defined detection patterns | Detection alone does not establish MCP capability | Eligible individually after confirming native MCP; common custom binding provided |

## Installed-client handshake verification (2026-10-01)

Initial probes launched real installed vendor clients against the installed Warpai bridge and a synthetic authenticated local broker. Follow-up probes used the macOS GitHub-built bridge from source commit 632bd1c after both OS protocol suites passed; all six installed frontends passed again. They verify native MCP initialization and tool discovery, without model turns or cross-pane wake acceptance. Temporary homes/workspaces preserved personal server configurations and credentials.

| Installed frontend | Native handshake | Conditions |
| --- | --- | --- |
| Codex 0.159.2 | Passed; native remote TUI, HTTP 101, broker registration | Add IPv4 loopback proxy exclusions on the TUI child; real shared daemon remains running |
| Claude Code 2.1.267 | Passed; initialize, 12 tools, registration | Native inline MCP config; trust only the empty temporary workspace |
| QoderCN 1.1.65 | Passed; initialize, 12 tools, registration | Separate QoderCN executable and native inline config |
| Cursor 2026.09.18-9a7762b | Passed; initialize, 12 tools, registration | Runtime environment references and native approval in temporary HOME/workspace |
| Antigravity 1.2.14 | Passed; initialize, 12 tools, registration | Temporary native configuration; probe prelude returns MethodNotFound before SDK initialization |
| DeepSeek Harness dsh-tui | Passed; initialize, 12 tools, registration | Both clean temporary and repaired ordinary profiles passed; retain the global journal hook and remove its identical profile-level insertion |
| Qoder | Not installed | Existing entry wrapper reports missing Qoder CLI; do not count it as QoderCN |

The initial Antigravity probe used a temporary protocol prelude around the installed binary to validate the fix before GitHub compilation; the follow-up passed directly through the compiled source fix with no external compatibility filter. Codex also passed with inherited network proxies and both loopback exclusion variables absent before launch. Source regressions verify the same fallback, buffered input retention and frame bounds. The installed app is not updated by source edits. Existing Cursor settings require unchecking/re-enabling its managed entry after upgrading to regenerate the environment references. The DeepSeek personal-profile hook conflict was repaired locally at the user's request by removing only its redundant profile-level insertion; the original was backed up and the ordinary profile passed the native handshake with a temporary MCP overlay.

## Excluded or unverified

- Installed versions lacking native MCP client support, including older Pi releases that require an external MCP extension. Upgrade rather than ship a fallback adapter.
- Custom managed agents whose native MCP capability has not yet been confirmed are pending verification. Confirmed native MCP clients use the common `custom` binding and are included; they are not excluded based on the `Unknown` enum value.
- HTTP-only clients cannot use this release's local stdio bridge until native stdio capability is established. No network listener is added for them.
- Cloud Amp orbs are outside the local Warpai terminal boundary.
- Trae aliases include the documented `traecli` executable in this revision. Its native YAML list adapter uses the vendor-documented per-platform configuration path.

No currently listed named type has been conclusively established as having no native MCP support in its current release. Do not invent an exclusion list from older product knowledge.

## Settings-managed MCP configuration

Use **Settings > Features > Agent communication**. The switch is off by default. Enabling it discovers installed managed commands and their aliases. Check an available agent to configure the bundled stdio bridge in the background. No manual registration prompt, `mcp add`, or environment editing is required. Uncheck to revoke access and remove the managed entry; turning the switch off revokes every active participant before cleanup. Failed cleanup is visible, stays unauthorized, and can be retried with **Refresh agents / retry cleanup**.

Warpai owns only `warp-lite-communication`, not a user's existing `warp-agent` server or unrelated MCP configuration. JSON/YAML updates preserve unrelated values; TOML uses a marked block and preserves existing text. Malformed configurations, ownership collisions, user edits to the managed entry, and symlinks stop the operation. Preferences and cleanup metadata are machine-local. The packaged bridge is `/Applications/Warpai.app/Contents/MacOS/warpai-agent` on macOS and `warpai-agent.exe` beside Warpai on Windows.

The bridge inherits `WARP_AGENT_ENDPOINT`, `WARP_AGENT_CAPABILITY`, and `WARP_TERMINAL_SESSION_UUID` from its Warpai terminal. Eligible ordinary Codex commands inside Warpai receive per-invocation native MCP overrides and --no-daemon when communication and Codex are selected. Each launch resolves and probes the current installation; no vendor command, PATH, CODEX_HOME or Codex configuration is modified. Gemini and Hermes receive runtime variable references. Values are never persisted in configuration or metadata. Other eligible native stdio clients inherit the terminal environment through their native subprocess contract. A process launched outside a managed Warpai terminal cannot participate.

Documented adapters cover every named managed type and alias. Codex uses session-only overrides and never reads/writes its user configuration through the adapter; Grok uses TOML tables; Trae uses its native YAML server list; DeepSeek Harness uses a dedicated insertion in the home Cordis patch; other named clients use their native JSON/YAML sections or confirmed native CLI setup contracts. Custom literal toolbar commands retain native contract probing. Aliases of the same client share one configuration and one settings row. Qoder and QoderCN are separate clients with independent rows and authorization; an unavailable Qoder launcher must not suppress QoderCN discovery. No shell-tool communication fallback is added.

Vibe uses its native `VIBE_MCP_SERVERS` environment configuration layer. Warp prepares the list and the three dynamic bridge environment variables in memory for a newly opened pane, preserves existing user/project MCP entries at that pane's startup directory, and leaves the Vibe configuration file untouched. Open a new terminal pane after enabling Vibe. Changing Vibe profiles or project configuration after opening the pane can override or stale this environment snapshot; use a new pane in the target directory. Capabilities are never written to disk. The upstream implementation is the [environment layer](https://github.com/mistralai/mistral-vibe/blob/main/vibe/core/config/layers/environment.py) and [configuration precedence](https://github.com/mistralai/mistral-vibe/blob/main/vibe/core/config/default_orchestrator.py). Native discovery is the final participation check; unsupported installed versions or organization policy may still prevent loading.

Restart running clients that do not hot-reload MCP configuration. A fresh managed CLI run loads the bridge and receives a unique project-local identity automatically. All live enabled agents in the same canonical project can discover, message, and delegate to one another; there is no peer picker. Repository subdirectories share scope; separate repositories and worktrees remain isolated.

The cooperation instructions require `warp_agent_ready` as the last action before ending a turn. Warpai polls every 250 ms, waits for output to settle, and automatically submits queued inbox work when eligible. Busy/draft/permission protections remain unchanged. Removal of MCP/environment configuration takes effect in a running vendor client after reload/restart; broker revocation takes effect immediately.

Adapter references: [Pi](https://pi.dev/docs/latest/mcp), [OMP](https://github.com/can1357/oh-my-pi/blob/main/docs/mcp-config.md), [Hermes](https://hermes-agent.nousresearch.com/docs/user-guide/features/mcp), [Goose](https://github.com/aaif-goose/goose/blob/main/documentation/docs/getting-started/using-extensions.md), [Vibe](https://docs.mistral.ai/vibe/code/cli/mcp-servers), [Auggie](https://docs.augmentcode.com/cli/integrations).

## Shared background service status (2026-10-01)

The common Warp broker and per-launch relay have simulated isolation coverage for every managed type plus custom, with two clients per type in one process. That verifies the shared communication backend, not every vendor's external daemon. Launch recognition remains independent of readiness and supports parameterized commands; installed help supplies option arity.

| Native transport | Implementation | Verification |
| --- | --- | --- |
| Codex interactive, resume and fork | Original installed CLI with --no-daemon and per-invocation -c MCP overrides | Native option, directory, update-resolution and shared broker isolation regressions; current source acceptance recorded below when complete |
| Claude and Qoder/QoderCN native interactive clients | Confirmed inline MCP configuration to a per-launch relay | Common relay/setup regressions; two real QoderCN clients independently registered with the 843681f bridge; Claude stopped at native directory trust selection; patched Warpai acceptance pending |
| Other managed native stdio clients | Existing documented adapters and terminal binding through the common Warp broker | Common protocol/setup coverage; authenticated native acceptance pending |
| Arbitrary vendor external daemon or explicit remote attachment | No general per-terminal binding guarantee | Requires a verified vendor session context API; not implemented by environment passthrough |

Codex explicit remote endpoints, unknown launch syntax and server-feature compatibility overrides retain native behavior; they are not claimed as transparently bound shared transports. No shared daemon is restarted to acquire a pane's environment.

OpenCode's documented [server API](https://opencode.ai/docs/server/) exposes instance configuration and dynamic MCP addition separately from session creation. Its [native MCP implementation](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/mcp/index.ts) stores MCP clients by instance. This indicates that replacing one shared MCP entry cannot isolate multiple attached terminals in the same instance; no global replacement is performed. Copilot's [ACP server](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server) permits per-session MCP configuration, but ACP server mode is not an interactive terminal frontend and has no implemented Warpai attachment adapter.

Native probes above used an isolated synthetic broker and no model turn. They prove frontend MCP loading and binding, not native model processing or Warpai automatic PTY handoff. Client exit completed through native EOF; existing shared daemons were not restarted or stopped.

QoderCN's zero-exit missing-entry response is handled only when it exactly matches the confirmed user-scope not-found sentence. Existing or edited user entries remain protected.

## Acceptance and status

GitHub tests simulate the restored detector's managed program identities plus a managed custom client, exercise real local sockets, negotiate the pinned MCP SDK over child-process stdio, and verify the shared task state machine. These tests do not launch vendor models.

For each eligible vendor/version, separately verify: bridge discovery with all 12 tools; inherited terminal binding; unique-name registration; cross-agent messages; a completed turn returning to the input prompt, then new work automatically being submitted without an open wait call or human prompt; busy task queueing; submission with evidence; reviewer rejection and revised rework; final acceptance; and rejection after CLI exit/replacement. Live model acceptance is pending until the patched build and each authenticated vendor runtime are available.

Qodercn is a confirmed native stdio MCP client: installed `qodercn mcp add --help` exposes the `stdio` transport. Warpai recognizes it separately as `CLIAgent::QoderCN`, including the underlying `qoderclicn` executable. It is not part of the unverified custom-agent exclusions. No personal MCP configuration was read during this check.

## Receiver feedback repair (2026-10-01)

All types use the same broker readiness lease and guarded PTY path. Listener existence and stale InProgress presentation no longer block explicit final-action MCP readiness. Opaque OSC notifications cannot establish readiness; only structured lifecycle events or the explicit tool can. Queued work shows a native receiver notification, even before readiness. Send/assign report `delivery: queued`; listing exposes `ready` and `pending_count`. Queued delivery never means acknowledgement or completed work. The application regression installs each available native listener and checks draft/permission protection for every enum type.

Vendor-named launch aliases, runtime command catalogs and PATH injection have been removed. Codex adaptation occurs only at the Warpai execution event. Selecting it does not write its user configuration; disabling it revokes broker authority.
