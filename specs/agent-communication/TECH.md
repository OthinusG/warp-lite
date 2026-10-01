# Native CLI Agent Communication

## Context

See PRODUCT.md for behavior. `CLIAgentSessionsModel` emits terminal start/end events; Qoder and Antigravity already have command recognition but no rich lifecycle listener. `WARP_TERMINAL_SESSION_UUID` is already assigned at pane creation. The existing PTY submit helper handles vendor-specific paste/Enter strategies but does not prove prompt consumption or guard all delayed writes against a replacement process.

## Proposed changes

- Add `warp-agent-bus`, a small workspace crate using existing serde, UUID, SQLite/Diesel, Tokio, and the pinned rmcp dependencies. It contains the state machine, durable store, local socket transport, stdio MCP bridge, and companion `warp-agent` executable.
- A Warp singleton starts a broker after CLI session tracking is registered. Pane creation receives a random per-terminal capability and the broker endpoint. Session start binds the capability to the actual CLI type, project directory, and a fresh run ID. Session end invalidates that run and wakes its waiting requests.
- A bridge cannot create a terminal binding itself. First registration returns the live run ID; subsequent operations require the same run. Tokens stay in memory/environment and never enter the database or MCP config.
- Resolve project identity from the canonical terminal working directory's nearest `.git` ancestor; without a repository, use the canonical working directory. Worktrees remain separate project scopes in this release.
- Poll queued work while busy and submit an inbox notification when the agent returns to its input prompt. Use `warp_agent_ready` for native MCP clients without lifecycle hooks, installed completion/busy events where available, and the existing per-agent PTY submission strategies. `warp_agent_wait` remains an optional active-turn delivery path.
- Store a versioned snapshot in a dedicated SQLite database, transactionally committing task mutations and their messages together. Bound records and input sizes. `ponytail`: snapshot writes are O(n); normalize tables when the explicit first-release capacity becomes limiting.
- Use bounded JSON frames over Unix sockets/Windows named pipes rather than exposing a TCP listener. Limit connections, frame size, and IO deadlines. Keep server errors free of payloads and capabilities.
- MCP schemas and operation parsing share one contract. Requests use idempotency IDs for mutations and revisions for task transitions. Results contain stable task/message IDs and explicit states.
- Package the companion beside Warpai. Setup uses native vendor commands or narrowly edits the documented MCP section of the vendor configuration, preserving unrelated values. Credentials, credential files, and capability values are never copied into Warp metadata or diagnostics.

## Native shared-service binding revision (2026-10-01)

- Install private executable aliases only for settings-enabled native clients. Dispatch directly to resolved native executable paths with untouched argument boundaries and exit codes. Shell bootstrap retains this private PATH after user startup files; discovery excludes it. Old persisted entries can lack option contracts while restored panes precede asynchronous discovery; Codex refreshes an empty contract from the installed native help at launch. The aliases are launch integration, not a model-facing shell communication interface. All messages still use the existing twelve native MCP tools and authenticated local broker.
- Publish the current native launcher catalog atomically in the broker's private runtime directory. It contains executable paths and public option contracts only. Old panes read this shared catalog at invocation; retired aliases retain their native executable and bypass communication overrides. Missing or invalid catalogs can use a pane snapshot only for native execution. Initial enablement requires a new pane to inherit the private launch PATH; changing a parent app cannot update an existing shell environment.
- Each supported per-session adapter owns a private MCP relay for its launch. The daemon's server configuration contains only the companion and ephemeral local IPC address; terminal capabilities remain in the launch process. Relay closure and broker run revocation both invalidate old clients. Use macOS private Unix sockets and Windows named pipes, retaining the broker's existing same-user trust boundary.
- Codex uses its native `app-server daemon start` (idempotent) and `app-server proxy` control channel. A loopback WebSocket proxy implements Codex's native remote TUI transport, authenticates with a fresh environment-only token, and replaces only the dedicated MCP server in native thread/start, thread/resume and thread/fork overrides. Other MCP entries, permissions, model settings and user arguments are preserved. No daemon restart, persistent capability, or global daemon environment rebinding is permitted. Explicit embedded/configuration/remote modes follow the native launch policy; server-feature overrides remain native until compatibility is established.
- Claude and Qoder use their confirmed native inline MCP configuration option for the same dedicated relay. Other eligible clients retain their documented native adapters and share the same Warp broker. A vendor background service requires a verified session-scoped configuration/context interface: common transport simulations do not prove that an arbitrary pre-existing vendor daemon consumes the launching client's configuration. Unsupported external-service transports must not be reported as implemented or authenticated live coverage.
- Only responses to this TUI's own thread creation/resume/fork requests bind native lifecycle events. Other threads' idle events are ignored; active, approval and user-input states invalidate readiness. Initial cached native readiness is replayed once after actual MCP discovery. Rediscovery cannot replay an old idle event, and a model's readiness call cannot override authoritative native busy state. Existing manual-draft and current-run PTY checks remain mandatory.
- Thread creation and owned turn-start requests establish busy before forwarding, including the discovery-before-thread-response race. Task-bearing initial launches suppress idle until an owned turn begins or the native thread is observed active; thread creation alone must not interrupt the initial task. The same installed option parser distinguishes a supplied prompt from session selection: empty resume/fork launches may become ready after the owned thread binds, while resume/fork with an initial prompt still waits for that turn. An authenticated internal frame can suppress the heuristic initial lease and supply the native absolute workspace before initial registration. Each forwarded MCP child supplies its actual working directory in a bounded private IPC handshake; embedded Codex receives the same dedicated relay through native command-line configuration overrides. These are bridge transport context, never MCP tool fields. Directory switches after registration fail closed rather than moving an identity with pending work into another project; start a fresh managed CLI for a new project.
- Validation: GitHub tests the common relay with two simultaneous panes for every detector-derived managed type plus custom, without deriving their identities from the shared client's process environment. Test message isolation, draft protection, busy state, premature readiness, rediscovery and stale-run rejection. Test parameter values that resemble subcommands, native mode preservation, per-thread overrides and foreign lifecycle events. Native vendor/live-model validation requires installed authenticated clients and a patched application; keep it separate from these tests.

## Codex loopback proxy repair (2026-10-01)

- Reproduced the reported WebSocket `Handshake not finished` with the installed Codex 0.159.2, inherited proxy variables, an isolated temporary Codex home and the installed Warpai bridge. The local listener received no connection; adding loopback exclusions reached HTTP 101 and the native initialize stage. No model request or personal configuration change was involved.
- Add `127.0.0.1` to both `NO_PROXY` and `no_proxy` only on the shared-mode TUI child. Preserve each existing exclusion list and fall back to the other spelling when absent. Keep provider proxy variables, daemon environment, explicit remote and embedded modes unchanged.
- Acceptance: the generated child environment retains existing exclusions and bypasses the launch's IPv4 loopback listener, including absent, empty, mixed-case and wildcard configurations. Run the focused Rust regression and application checks on GitHub only; verify the installed native transport separately without treating a mock daemon as model acceptance.

## Independent QoderCN discovery (2026-10-01)

- QoderCN and Qoder are separate installed clients with separate configuration roots. Do not deduplicate QoderCN as a Qoder alias or let a nonfunctional Qoder entry script hide it.
- Add a distinct QoderCN managed identity for `qodercn` and its native `qoderclicn` executable. Reuse Qoder's existing icon, prompt transport and native MCP configuration contract; retain separate settings selections, launchers and broker authorization.
- Refresh the program identity of previously selected QoderCN entries from successful native discovery. Verify separate command detection, discovery despite an unavailable Qoder executable, separate selection authorization and native MCP handshake on the installed QoderCN client. No personal credentials or MCP configuration are copied into fixtures.

## Cursor native environment repair (2026-10-01)

- The installed Cursor 2026.09.18 client filters inherited stdio subprocess variables. A native list-tools probe failed with the old command/args-only entry and passed initialize plus all twelve tools when explicit `${env:WARP_AGENT_*}` and `${env:WARP_TERMINAL_SESSION_UUID}` references were supplied.
- Add only runtime variable references to the dedicated native JSON entry. Never persist their values or disable native MCP approvals. Validate with an approved test server in a temporary HOME/workspace, preserving personal server configuration and approvals.

## API contract

### Modern discovery fallback (2026-10-01)

Antigravity 1.2.14 sends a `server/discover` request before legacy initialization. The pinned rmcp codec classifies the unknown method as a custom notification and its handshake closes the stream. Reject one initial discovery request with JSON-RPC MethodNotFound while preserving its request ID, then pass the original standard handshake and all subsequent traffic to rmcp. Do not advertise modern stateless support. Share this bounded prelude between stdio and launch relays. Native Antigravity's temporary configuration confirmed fallback, initialize, twelve tools and authenticated broker registration without a model turn. Add a regression for discovery fallback, normal initialization and oversized input. Official fallback rules: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/draft/basic/transports/stdio.mdx.

All tool arguments reject unknown fields. Sender/project/run are authenticated transport context, never caller-supplied tool fields.

| Tool | Arguments | Result |
| --- | --- | --- |
| warp_agent_register | name | agent identity and live run |
| warp_agent_list | none | live same-project participants and online/waiting state |
| warp_agent_send | to, body, request_id | message |
| warp_agent_inbox | none | pending messages |
| warp_agent_ack | message_id | acknowledgement |
| warp_agent_wait | none | next pending message or idle timeout |
| warp_agent_ready | none | run-scoped idle announcement; finish the turn |
| warp_task_assign | to, description, acceptance, request_id, optional reviewer | task |
| warp_task_get | task_id | visible task |
| warp_task_start | task_id, revision, request_id | running task |
| warp_task_submit | task_id, revision, result, evidence, request_id | submitted task |
| warp_task_review | task_id, revision, accepted, feedback, request_id | accepted task or queued next revision |

## Tasks and validation

1. Implement and test identity/project isolation, assignment, submission, rework, reviewer authorization, stale revisions, idempotent retries, durable reopen, and old-run rejection.
2. Exercise real local sockets and the rmcp stdio transport; run simulated clients for every managed type. Prove waiting resumes on assignment and review results, without calling it a live model test.
3. Integrate native terminal bindings and companion packaging directly in source. Verify clean checkouts compile and package without a restoration step.
4. GitHub Windows and macOS jobs run focused crate tests. macOS checks default and `warp_platform`, builds the application and companion, and uploads reviewable artifacts. macOS and Windows are the only target platforms; Linux implementations and tests are outside scope. Do not publish a release from the validation workflow.
5. Live model tests for every managed CLI require their authenticated local environments and a new patched Warp build. GitHub runners have no vendor credentials; record these tests as pending rather than reporting protocol simulations as live acceptance.

## Risks

## Fresh receiver and default orchestration repair (2026-10-01)

- Parameterized startup repair: derive option arity from the installed client's native help, including Codex's hidden `--yolo` alias and Antigravity's confirmed Go-style declarations. Configuration-only launches can receive the one-time discovery readiness lease. Resolve alias/function shadowing against the parsed executable name. Positional work, unknown syntax, batch/protocol modes, resume/attach/background modes, malformed commands and shadowed launches await authoritative lifecycle readiness. This changes readiness eligibility, not command recognition. Validate the existing peer prompt regression and shared option-contract regression on GitHub. Live acceptance requires a queued peer request to wake a receiver in the patched app.
- Reproduce the reported case: a newly launched Codex with no prior prompt registers MCP but never calls final-action readiness, so peer work remains queued.
- Seed a single initial readiness lease only for a verified empty interactive CLI launch, after native discovery. Launches with initial work, user input, blocked/busy events, replaced runs and absent native discovery must not receive this lease. Subsequent turns keep the existing readiness contract and guarded PTY submission.
- Resolve agents using both shell startup PATH and updated session PATH. Add the confirmed Qoder/QoderCN installation entry directories as narrow fallbacks; preserve vendor alias deduplication. Carry resolved search paths into native CLI probes and setup subprocesses so interpreter-based launchers work from GUI launches.
- Make orchestration the default MCP cooperation rule: an issuer tracks every outstanding task via TaskGet and AgentWait until reviewed; a receiver executes work in its existing terminal, reports progress, submits evidence and sends replies to ordinary instructions. Queue admission never means completion.
- Acceptance: freshly registered receivers wake without a prior model turn or AgentReady; user drafts and launches with task arguments remain protected; repeated discovery cannot re-arm a busy run. Verify QoderCN discovery and its actual user-scope stdio add/get/remove argument contract using temporary fake executables. Extend existing cross-agent protocol and app regressions. Run compilation/tests/packaging on GitHub only.

- Same-user local processes are one trust domain; session capabilities prevent accidental cross-session routing, not compromise of the OS account.
- Cancellation or transport loss after a mutation can precede its response. Retry the same request ID; never assume failure means no side effect.
- A disconnected CLI invalidates outstanding waits. Persisted running tasks require explicit recovery after the next live registration.
- Retain historical provenance while maintaining source directly; do not restore automatic upstream synchronization.

## Coverage constraint

Use `CLIAgent::command_prefix()` for known managed types and `custom` for user-managed unknown sessions connecting through their confirmed native MCP client. Do not add a separate program allowlist to the broker. The trusted Warp start event determines the program; callers cannot spoof it. Command aliases remain the existing detector's responsibility.

The MCP interface invokes the common state machine and exposes both dormant readiness and optional cooperative wait. Four installed CLIs are initial runtime probes, not the release coverage boundary. Installed versions without a verified native setup adapter are displayed as unavailable; never invent configuration flags, use shell-tool fallbacks, or install missing agents.

The user explicitly excludes adapters for agents without native MCP client support. No shell interface is shipped. Custom managed sessions are verified individually and included when native MCP capability is established; the enum value `Unknown` is not an exclusion rule.

The first-release snapshot has explicit 1000-record capacities for identities, messages, tasks, and idempotency records; a capacity error leaves existing data intact. Acknowledged messages can be discarded when their capacity is reached. Other records currently require an archival design before long-running high-volume use.

Project-local names are unique among live terminals. An explicit registration can reclaim an offline name from a new pane, preserving its identity and pending work; it cannot take a name owned by another live terminal.

## Dormant prompt wake delivery

The user requires automatic delivery after a model turn ends, not only while an MCP wait call is active. Add `warp_agent_ready` to the shared contract, instruct every participating MCP client to call it as its final action before returning to its prompt, and reuse native lifecycle completion events where present. Readiness is transient and run-scoped; every other MCP operation, user PTY input, blocked/busy event, replacement, or exit invalidates it. User edits inhibit submission until a new user submit input boundary; cancellation keeps automatic work paused; MCP readiness cannot override an outstanding user draft.

The Warp singleton checks pending work every 250 ms. After readiness has settled, it claims one pending notification, submits only a fixed inbox instruction plus validated UUID, and invalidates readiness until a new completion. The original message is not acknowledged by PTY delivery. Existing per-agent paste/Enter strategies are reused, with current-run and manual-input checks repeated before delayed Enter. A failed/cancelled submission leaves the notification pending and does not spin-retry into a terminal.

Acceptance: simulate dormant readiness for every managed identity plus custom; deliver without AgentWait; keep busy/draft/blocked/replaced runs queued; prevent duplicate submission and stale delayed Enter; prove the message still requires MCP acknowledgement. GitHub must check both application configurations and the focused protocol tests. Real vendor input behavior remains a separate acceptance level.

## Settings-managed setup revision (2026-09-30)

## Receiver and adapter repair plan (2026-10-01)

1. Remove the redundant listener/status veto from the shared PTY wake guard; keep explicit blocked/draft/session protections and the broker's run-scoped readiness lease. Only structured lifecycle handlers may establish readiness from status changes.
2. Show each pending message once using the existing terminal notification event; expose pending count/readiness and truthful queued delivery responses through existing MCP tools. Include the final-action readiness rule in each tool description for clients that omit server instructions.
3. Replace guessed setup for named vendors with documented configuration adapters, retaining probes for genuinely custom versions. Add reversible TOML list, YAML list and Cordis patch support using existing dependencies. Vibe requires runtime environment injection because its Python stdio client filters inherited variables; capabilities stay exclusively in the terminal environment.
4. Extend the existing application wake and protocol/setup regressions. Run GitHub protocol, application, and packaging checks against directly committed source.

## Warpai branding (2026-10-01)

The same version uses Warpai as its application/display name and warpai in product prose. Rename shipped window/menu/settings/notification text, macOS display metadata and executable, Windows executable/installer/portable packages, and the installer sidebar graphic. Documentation/help links target this fork; original licensing and upstream attribution remain accurate. Keep crate/bin identifiers, protocol/tool identifiers, persisted settings paths, bundle ID, and real repository URLs compatible. Maintain UI branding directly in source; update release workflow asset paths together. Verify Rust parsing, shell/workflow syntax, clean checkout builds, installer image dimensions/text, GitHub checks, and packaged plist/executable naming. No local compilation.

Replace manual vendor setup with a local settings model and serialized background configuration jobs. Discover installed managed command aliases without reading credentials. Use documented vendor config formats or native setup commands; retain ownership metadata only for the dedicated `warp-lite-communication` entry. Apply atomic file updates, preserve unrelated values, refuse collisions, and retain cleanup failures for retry. Codex explicitly passes the three dynamic environment names through `env_vars`; clients with inherited subprocess environments need no persisted capability. UI observers render per-agent status and restart guidance. Broker policy is default-deny in the app, immediately revokes unchecked programs and all programs on global disable, and requires fresh native discovery after re-enable.

The second implementation phase removes the tab picker, reciprocal run sets, and selected-only routing. Existing authenticated project scope remains the single routing boundary; only live, enabled, native-discovered recipients are eligible. Preserve task review authorization and lifecycle/wake protections.

Ownership metadata is kept in the local secure state directory, independently of cloud-synced settings. Persist cleanup intent before injecting a server, then authorize only after setup succeeds. Pending cleanup is not participation permission. Files are replaced atomically, malformed configuration and symlinks are preserved, and collisions/user-modified managed entries are reported instead of overwritten. CLI probes and configuration commands have bounded execution time and do not pass through a shell. Shared JSON/YAML formats and Codex TOML blocks use existing dependencies.

## Independent maintenance revision (2026-10-01)

Retire upstream synchronization, replay patches and restoration scripts. All feature/branding/platform changes are direct repository source. Keep independent GitHub validation and tagged macOS/Windows release workflows, including the communication companion. Linux is not a target platform. Historical upstream plans above are superseded by this revision. Receiver state acceptance is specified in RECEIVER-STATES.md.
