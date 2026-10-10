# Agent usage support

Source: [Orca](https://github.com/stablyai/orca), immutable revision
`3e0b82835856dde57f43a47661cf736e3e3dd90d`. Request contracts and quota mappings
are adapted from `src/main/rate-limits`. Copyright (c) 2026 Lovecast Inc.;
[MIT notice](../../docs/ORCA-LICENSE) is included in both desktop bundles.

Accounts are opt-in. CLI accounts follow the current vendor CLI login, rather
than pinning a previous user's identity. API/token/cookie accounts use OS secure
storage; the account file contains labels, provider IDs, visibility and context
only. No Warp cloud login, model calls or provider credentials enter Agent IPC.
Expired access tokens require vendor sign-in again; Warpai does not extract
OAuth client secrets or modify vendor credentials. Missing values never mean 0%.

| Managed agent | Usage support | Connection / source |
| --- | --- | --- |
| Claude | Supported | Current CLI OAuth login or access token; `/api/oauth/usage` |
| Codex | Supported | Current CLI OAuth login or access token; WHAM usage |
| Gemini | Supported | Current CLI login or access token; Code Assist project/quota metadata |
| Amp | Unsupported | No quota adapter in inspected Orca source |
| Droid | Unsupported | No quota adapter in inspected Orca source |
| OpenCode | Supported providers | Go API key; Zen dashboard cookie plus workspace ID; Kimi, MiniMax and GLM provider accounts |
| Copilot | Unsupported | No quota adapter in inspected Orca source |
| Pi | Unsupported | No quota adapter in inspected Orca source |
| OhMyPi | Unsupported | No quota adapter in inspected Orca source |
| Auggie | Unsupported | No quota adapter in inspected Orca source |
| Cursor CLI | Supported | Dashboard cookie; monthly allowance and pool usage |
| Goose | Unsupported | No quota adapter in inspected Orca source |
| Hermes | Unsupported | No quota adapter in inspected Orca source |
| Vibe | Unsupported | No quota adapter in inspected Orca source; native terminal icon fallback |
| Antigravity | Supported | Current `agy` login, version >= 1.1.11; free `/usage` metadata command |
| Grok | Supported | Current CLI login or access token; credit/monthly billing metadata |
| DeepSeek Harness | Unsupported | No quota adapter in inspected Orca source |
| Qoder | Unsupported | Interactive `/usage` is documented; no verified safe background metadata interface |
| Qoder CN | Unsupported | Installed CLI help exposes login/status/config/MCP, no standalone usage query; print prompts could spend quota |
| Trae | Unsupported | No quota adapter in inspected Orca source |
| Unknown / custom | Unsupported | No verified provider contract |

Additional API account providers: Kimi Code, MiniMax global/CN, Z.ai and Zhipu
Coding Plans. These do not add new managed Agent types. Zen reports USD balance
only, without an invented percentage. Do not confuse standard model API keys
with CLI OAuth access tokens or coding-plan keys.

Qoder reference: [official CLI usage](https://docs.qoder.com/cli/usage).
Antigravity provenance: Orca `antigravity-usage-command.ts` protects versions
before 1.1.11 and explicitly forbids disabling slash commands. Every query
probes the plain executable first, has a timeout and bounds captured output.
No personal provider credentials or real usage queries are used in development
or fixtures. Synthetic parser/account tests and native light/dark captures gate
desktop acceptance; authenticated provider service availability remains account
and vendor dependent.
