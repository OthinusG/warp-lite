# Agent usage support

Source: [Orca](https://github.com/stablyai/orca), immutable revision
`3e0b82835856dde57f43a47661cf736e3e3dd90d`. Request contracts and quota mappings
are adapted from `src/main/rate-limits`. Copyright (c) 2026 Lovecast Inc.;
[MIT notice](../../docs/ORCA-LICENSE) is included in both desktop bundles.

Accounts are opt-in. CLI accounts follow the current vendor CLI login, rather
than pinning a previous user's identity. API/token/cookie accounts use OS secure
storage; the account file contains labels, provider IDs, visibility and context
only. No Warp cloud login, model calls or provider credentials enter Agent IPC.
The same configured accounts appear in every local and SSH project. Current CLI
login means the desktop computer's login; no remote host credentials are read.
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

## Native compatibility audit

| Orca dependency / contract | Warpai implementation | Verification |
| --- | --- | --- |
| Electron `net.fetch` | Existing reqwest with native TLS roots and system proxy support; fixed HTTPS endpoints and bounded responses | Request-object tests for all 12 HTTP providers, method/auth/required headers and Gemini JSON body |
| Electron session / browser cookies | Explicit user-provided Cookie header for Cursor and Zen, stored through OS secure storage; no shared browser cookie jar | Sensitive-header and workspace requirements tested; MiniMax uses Orca's separate Bearer API-key contract |
| Node CLI process / path discovery | Existing native executable discovery plus bounded Tokio child process; exact unmodified agy metadata arguments and version probe; hidden Windows child console | Version, successful command envelope and noisy/pretty JSON stdout regression tests; stop subsequent polling if agy runs a model turn |
| Orca credential stores | Warpai Keychain on macOS and DPAPI-backed secure storage on Windows; bounded read-only vendor CLI auth discovery | Metadata rejects credential fields; no credentials enter Agent IPC or raw errors |
| Electron IPC / renderer components | Native singleton model, typed view actions, dropdowns/editors/icons and independently scrolling footer | Both desktop native captures and focus/cache assertions required |
| Electron activation / renderer lifecycle | Application-owned account metadata, configuration generation and usage cache | Panel/project/focus changes have no account reset path; restart restores metadata and queries fresh usage |

The port does not execute Orca JavaScript or depend on Electron, Node fetch,
preload globals, React components or files in the temporary research clone.
CLI accounts mean native desktop CLI logins; WSL/remote profile discovery and
browser OAuth/cookie extraction are not claimed. Synthetic transport tests
construct real reqwest requests without sending credentials or contacting
providers. Compilation and native runtime acceptance are still required;
authenticated third-party service access cannot be certified from fixtures.

Header audit also preserves Codex client/beta/originator headers, Zen Origin and
Grok user ID (discovered from CLI auth or provided as account context). Hidden
account caches and blocked unsafe agy readings survive other account refreshes;
workspace or focus changes never reset either state. Updating agy and restarting
the application restores eligibility after an unsafe-command detection.
