# Local shared-space routing implementation contract

Scope note (2026-10-03): preserve these implemented local space semantics and historical evidence. New SSH Remote project scope is private by selected remote root; it does not require device grants or cross-host grouping. Use PLAN.md, TECH.md and CUTOVER D13 for the current delivery.

Status: backend implementation under verification; production UI admission is gated.
SpaceJoin/WorkspaceMap alone remain metadata and do not merge existing task scopes.
Implements M4.1/M4.2, preserving M1/M3/M5 authority and privacy guarantees.

## Binding and exposure

Start shared participation in a newly prepared terminal pane. Its new terminal
UUID/capability prevents an old private bridge from registering into a changed
scope: AgentRegister intentionally supports first discovery without a run ID.
Do not change the scope of an existing prepared private pane or agent identity.
The trusted app selects an explicit existing workspace mapping and space before
preparing that pane. Native MCP arguments cannot supply or override that choice.

Reuse the local broker and storage engine. A selected shared space uses a reserved
non-path domain key such as `space:<uuid>` for tasks/messages/events/agent-name
uniqueness. Keep physical canonical roots as separate internal workspace data.
Private domain keys remain their existing canonical roots. No Git remote matching,
task-row reassignment, private identity mutation or cross-space history copying.

Broker preparation captures mapping ID, space ID and canonical physical root.
Activation/discovery checks that snapshot against the current mapping. Even before
first discovery, a scoped bridge reporting another directory must fail rather
than silently changing its scope. Reclaim a shared offline identity only in its
original workspace; another checkout cannot claim its name and pending work.

Joining shared membership creates a distinct scoped identity. Existing private
identities/tasks remain unchanged, including running and interrupted attempts.
Leaving revokes scoped authorization and wake eligibility immediately; it does
not prove execution stopped or relocate outstanding work. Mapping changes cannot
silently move an existing bound identity to another space.

Before a user joins, the UI must show mapped checkouts, repository IDs and
participants, explain which new sessions become visible, and distinguish metadata
members from identities with functioning shared routing. Opening a new shared
pane is explicit; normal terminal creation stays private. Live UI integration
continues to require the static screenshot checkpoint.

## Storage and physical operations

SQLite v5 stores immutable agent-to-workspace and space bindings plus revocation.
The existing workspace row already retains its immutable canonical root; no second
copy of that path is needed. Peer identities expose their domain, not that root. Preserve
legacy private rows and a separate pre-v4 upgrade backup; advance the downgrade
sentinel. Construct old-schema fixtures accurately rather than retaining new
columns while only changing their version marker.

Audit every project-key use in transport/storage: grouping and authorization use
the domain key; path normalization, reservation conflicts and local evidence
verification use the producing physical workspace. Broker native-directory checks
use the immutable physical binding, not the shared domain key. Existing private
behavior remains the reference for default operation.

Reservation ownership remains task/run/attempt fenced. Physical exclusivity spans
all scopes in one actual checkout, while list projections remain scoped. An
exclusive conflict with private work must not disclose its task ID or private
identity. Preserve physical-workspace ordering for reservation cursors even when
its records originate in different domains. Cross-checkout warnings require the
existing explicit repository UUID and current membership; warning metadata cannot
disclose task references belonging to another private domain.

Evidence retains its immutable attempt owner, whose workspace binding resolves
the producing checkout even after membership departure. Verifying a shared
task must resolve that snapshot, not treat `space:<uuid>` as a filesystem root or
substitute the operator's current checkout. Missing/remote producing content stays
unavailable, and descriptors never trigger implicit file reads or command runs.

## Required runnable checks

- Two real IPC clients in explicitly mapped distinct checkouts discover each
  other, delegate/start/submit/rework/accept through the existing state machine.
- Unjoined worktrees and ordinary newly prepared panes remain isolated even with
  matching repository metadata. Membership metadata alone grants no task access.
- An active private task/attempt, messages, evidence and retry records remain in
  their original scope when another new pane joins a shared space.
- An old private terminal capability/run and AgentRegister cannot enter the new
  pane's shared binding; tool arguments and directory drift cannot choose scope.
- Equal names in different spaces are independent. Reclaim from a different
  physical checkout is denied, including while its prior process is offline.
- Departure/remapping fences calls and wake immediately; running effects remain
  unknown and retry/reassign still requires stop proof or explicit override.
- Same-checkout exclusive conflicts work across scopes without exposing private
  identities/tasks. Scoped reservation pagination remains ordered and bounded.
- Evidence verification uses its original checkout and retains path/credential
  protections; a different current checkout cannot verify a coincidental file.
- Migration/backups/downgrade guards and all existing private-scope suites pass on
  macOS and Windows. Device-qualified remote workspaces remain a separate gate.

Do not enable shared-space selection in production until these backend checks and
the actual join/leave preview path are implemented and verified.

Local IPC remains major 2: the existing operation/state contract is unchanged,
and old private terminal capabilities retain their private domain. Shared admission
is an internal trusted broker method, never a new native tool argument. No remote
capability is advertised. `tests/local_spaces.rs` exercises real authenticated IPC;
GitHub verification and the shared-pane UI remain pending.

## Native admission implementation checkpoint

The native panel previews explicit spaces, mapped canonical checkouts, repository
identities and participants before opening a new local tab. Pass the reviewed
workspace ID, space ID and root as a trusted NewTerminalOptions field, never as
a shell environment selector. Validate that immutable snapshot again during
broker preparation; a remap between review and terminal creation fails closed.
Normal tabs, saved layouts and restored sessions remain private until explicitly
joined again. Leaving revokes a selected shared identity and its wake authority
without moving private work or claiming external execution stopped.
Native production/read/action checks remain required for this checkpoint.
