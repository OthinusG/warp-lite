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

## API contract

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
