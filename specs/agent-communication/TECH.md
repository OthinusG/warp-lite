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
- Package the companion beside Warp Lite. Setup calls vendor-supported `mcp add` commands and never parses existing personal settings.

## API contract

All tool arguments reject unknown fields. Sender/project/run are authenticated transport context, never caller-supplied tool fields.

| Tool | Arguments | Result |
| --- | --- | --- |
| warp_agent_register | name | agent identity and live run |
| warp_agent_list | none | selected same-project peers and online/waiting state; legacy explicit registrations retain project discovery until selection |
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
3. Integrate native terminal bindings and companion packaging through the patch. Test clean application, repeat application, reverse checks, and failure on partially restored files.
4. GitHub Linux/Windows jobs run focused crate tests. macOS checks default and `warp_platform`, builds the application and companion, and uploads reviewable artifacts. Do not publish a release from the validation workflow.
5. Live model tests for every managed CLI require their authenticated local environments and a new patched Warp build. GitHub runners have no vendor credentials; record these tests as pending rather than reporting protocol simulations as live acceptance.

## Risks

- Same-user local processes are one trust domain; session capabilities prevent accidental cross-session routing, not compromise of the OS account.
- Cancellation or transport loss after a mutation can precede its response. Retry the same request ID; never assume failure means no side effect.
- A disconnected CLI invalidates outstanding waits. Persisted running tasks require explicit recovery after the next live registration.
- Patch context drift intentionally stops synchronization. Preserve and review provenance instead of silently rewriting unmatched upstream code.

## Coverage constraint

Use `CLIAgent::command_prefix()` for known managed types and `custom` for user-managed unknown sessions connecting through their confirmed native MCP client. Do not add a separate program allowlist to the broker. The trusted Warp start event determines the program; callers cannot spoof it. Command aliases remain the existing detector's responsibility.

The MCP interface invokes the common state machine and exposes both dormant readiness and optional cooperative wait. Four installed CLIs are initial runtime probes, not the release coverage boundary. Vendors without verified MCP setup commands use explicit/manual MCP configuration; never invent configuration flags or install missing agents silently.

The user explicitly excludes adapters for agents without native MCP client support. No shell interface is shipped. Custom managed sessions are verified individually and included when native MCP capability is established; the enum value `Unknown` is not an exclusion rule.

The first-release snapshot has explicit 1000-record capacities for identities, messages, tasks, and idempotency records; a capacity error leaves existing data intact. Acknowledged messages can be discarded when their capacity is reached. Other records currently require an archival design before long-running high-volume use.

Project-local names are unique among live terminals. An explicit registration can reclaim an offline name from a new pane, preserving its identity and pending work; it cannot take a name owned by another live terminal.

## Dormant prompt wake delivery

The user requires automatic delivery after a model turn ends, not only while an MCP wait call is active. Add `warp_agent_ready` to the shared contract, instruct every participating MCP client to call it as its final action before returning to its prompt, and reuse native lifecycle completion events where present. Readiness is transient and run-scoped; every other MCP operation, user PTY input, blocked/busy event, replacement, or exit invalidates it. User edits inhibit submission until a new user submit input boundary; cancellation keeps automatic work paused; MCP readiness cannot override an outstanding user draft.

The Warp singleton checks pending work every 250 ms. After readiness has settled, it claims one pending notification, submits only a fixed inbox instruction plus validated UUID, and invalidates readiness until a new completion. The original message is not acknowledged by PTY delivery. Existing per-agent paste/Enter strategies are reused, with current-run and manual-input checks repeated before delayed Enter. A failed/cancelled submission leaves the notification pending and does not spin-retry into a terminal.

Acceptance: simulate dormant readiness for every managed identity plus custom; deliver without AgentWait; keep busy/draft/blocked/replaced runs queued; prevent duplicate submission and stale delayed Enter; prove the message still requires MCP acknowledgement. GitHub must check both application configurations and the focused protocol tests. Real vendor input behavior remains a separate acceptance level.

## Native communication picker

Reuse the existing tab context menu for **Select communication peers** and the peer list. Pane menus carry the clicked terminal's identity; a selected item carries both current run IDs and is revalidated by the broker. Open the peer list after the original menu closes so pointer and keyboard selection cannot immediately dismiss it. All new UI text is English.

Native MCP tool discovery requests an automatic identity before returning the tool list. The broker creates a unique program-prefixed name; no separate model registration prompt is required. Automatically registered runs start with selected-only routing. Legacy explicit protocol registration remains compatible, but selecting a connection opts both runs into selected-only routing. Peers are reciprocal run identities, not persisted permissions. Exit/replacement clears them. Message sends and task assignments, including a separately designated reviewer, are checked at the broker boundary; selected-only discovery hides unrelated identities.

Connection establishment writes both inbox notices in one SQLite snapshot transaction before changing live routing. Repeated selection does not duplicate notices. The notices describe the communication tools and readiness rule without granting additional permissions. Existing delivery protects busy turns, drafts, and authorization prompts. A picker with no discovered native bridge shows an explicit unavailable state; this iteration does not implement automatic vendor configuration or claim that running clients can hot-load MCP servers.

## Settings-managed setup revision (2026-09-30)

Replace manual vendor setup with a local settings model and serialized background configuration jobs. Discover installed managed command aliases without reading credentials. Use documented vendor config formats or native setup commands; retain ownership metadata only for the dedicated `warp-lite-communication` entry. Apply atomic file updates, preserve unrelated values, refuse collisions, and retain cleanup failures for retry. Codex explicitly passes the three dynamic environment names through `env_vars`; clients with inherited subprocess environments need no persisted capability. UI observers render per-agent status and restart guidance. Broker policy is default-deny in the app, immediately revokes unchecked programs and all programs on global disable, and requires fresh native discovery after re-enable.

Second implementation phase removes the tab picker, reciprocal run sets, and selected-only routing. Existing authenticated project scope remains the single routing boundary; only live, enabled, native-discovered recipients are eligible. Preserve task review authorization and lifecycle/wake protections.
