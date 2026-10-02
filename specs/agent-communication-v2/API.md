# Agent Collaboration Contract

Status: proposed API, 2026-10-01. This is the contract source for the future implementation, not a list of currently callable tools. See [PRODUCT.md](PRODUCT.md), [TECH.md](TECH.md) and [PLAN.md](PLAN.md). Rust serde types and generated MCP schemas must implement this contract together.

## Identity and common rules

- `space_id`, `workspace_id`, `device_id`, `agent_id`, `task_id`, `thread_id`, `message_id`, `attempt_id`, `event_id` and `request_id` are opaque UUIDs. Display names are not authorization identities.
- The authenticated local bridge derives agent, terminal, run and local workspace. The authenticated remote connection derives device/session/grants. Tool arguments cannot override any of these identities.
- `revision` is the task's work/review generation. `version` increments on every authoritative task mutation. `attempt_id` identifies one execution owner. A retry, reassignment or requested rework increments revision; none silently reuse an old execution grant.
- Every new domain mutation carries a stable `request_id`; mutations of an existing task also carry `expected_version`. Repeating the same actor/epoch/request ID and payload returns its committed result. A changed payload is `request_conflict`.
- The bridge/controller supplies the server-issued mutation epoch internally and persists it with pending requests. Agents do not choose a fresh epoch to recover an uncertain operation. An expired epoch is rejected before a lookup can fall through to execution.
- Existing twelve MCP tool names and required arguments are preserved. New `expected_version` and `attempt_id` fields on existing tools are optional during compatibility rollout; omission is resolved only through the authenticated current run and supplied revision, with all original state/revision checks. New control tools require explicit versions. Internal IPC clients that cannot negotiate the v2 contract are rejected, not silently trusted.
- Local IPC requests declare `protocol_major: 2`. Registration returns the matching
  major, minor and explicit local feature list; the bridge checks them before
  advertising tools. This handshake does not advertise remote connectivity or
  a production shared-space admission UI. Missing/unsupported majors fail closed; install
  the matching application and companion together.
- Text fields remain at most 8192 UTF-8 bytes. Lists and reference metadata have explicit schema bounds; a frame cannot exceed 1 MiB. Unknown fields are rejected at trust boundaries. Pagination defaults to 50 items and caps at 200.
- Domain errors contain a stable code, safe human message, `retryable` flag and optional current resource version. No request payload, credential or environment dump is included. A retryable transport failure is not proof that a mutation was uncommitted.
- Deadline inputs are an optional RFC 3339 UTC `start_deadline`, `execution_timeout_seconds` measured from each attempt's committed start, and `review_timeout_seconds` measured from submission. Timeout durations are 1–604800 seconds. Retry/reassignment starts a fresh execution timeout; an expired start deadline must be explicitly replaced or cleared by the issuer, otherwise retry is rejected. The coordinator records the resulting absolute deadlines.

Trusted local admission prepares a fresh terminal/capability with an explicitly
selected workspace mapping. Its task/message/event namespace is `space:<uuid>`;
its physical canonical checkout remains immutable internal data. Ordinary panes
and metadata membership alone remain private. Directory overrides cannot change
a shared admission, including before discovery. Departure/remapping revokes the
capability and fences replay/wake without proving the old execution stopped.
Only a fresh explicit admission in the original checkout can reclaim its identity.
Physical reservations conflict across private/shared domains, while list results
and identity/task references remain scoped. Evidence verifies in its immutable
attempt owner's producing checkout. Backend verification does not enable the UI.

Invitation controller receipts persist only ID/grants/expiry. A new committed
request delivers its 256-bit invitation once, in memory; replay returns the same
receipt with `invitation: null, secret_available: false`. Lost secret delivery
requires creating a new invitation, rather than storing a bearer in retry history.
Invitations expire within five minutes and enroll one device atomically. Enrollment
returns a distinct credential once; only its SHA-256 verifier is durable. Every
remote operation rechecks current device generation and explicit space/role grants.
Controller device lists omit verifiers. An explicitly constructed enrollment-only
controller is available for IPC verification; it does not enable application remote
participation, persist participant credentials or dispatch remote agent operations.

The enrollment controller uses the existing four-byte big-endian length framing
and bounded hello negotiation, followed by these internal frames. Sensitive frames
have no Debug representation and must never enter logs or retry persistence.

| Frame type | Fields | Authority |
| --- | --- | --- |
| `enroll` | `invitation`, `name` | Single-use invitation after hello |
| `enroll_result` | `device_id`, `credential`, `generation`, `space_ids` | Credential delivery once in memory |
| `authenticate` | `credential` | Current verifier/generation |
| `authenticated` | `device_id`, `generation`, `connection_epoch`, `space_ids` | Server-derived principal |
| `heartbeat` | `connection_epoch`, `space_id` | Current principal, controller ownership and read grant |
| `heartbeat_result` | `connection_epoch` | Receipt on this connection only |
| `goodbye` | `connection_epoch` | Authenticated connection only |
| `error` | `error` | Redacted stable domain error |

Initial hello/authentication has a five-second deadline; authenticated input has a
30-second receipt deadline. A controller accepts at most 32 concurrent sessions.
Shutdown/restart fences every frame by controller ownership nonce and invalidates
device generations. Coordinator UUID survives broker/database restart; connection
epoch does not. The fixed `warp-agent remote-stdio` adapter forwards only bytes to
this per-user endpoint and emits a framed `coordinator_unavailable` failure when
there is no running controller. This subset does not advertise production remote
task availability.

The participant SSH session owns its system-SSH child and all three pipes. It
validates the expected durable coordinator UUID before delivering an invitation
or credential, retains the server connection epoch internally, and requires
successful authentication plus an explicit grant before heartbeat. Stderr is
discarded through a bounded copy buffer rather than persisted or exposed. Peer
errors retain only recognized codes and static messages; returned credentials,
identities and grant lists are bounded and validated before delivery/use. Closing
the session terminates only its owned child. Enrollment returns credentials in
memory so the application can persist them through platform secure storage; this
client alone does not enable production participation or reconnect/replay.

Device-qualified routing storage uses additive SQLite v6 tables separate from
host workspace roots. A remote workspace is keyed by enrolled device plus an
app-verified checkout UUID; its display path is never opened on the coordinator.
An actor is qualified by its original device/workspace/native UUID, and cannot
reclaim a host or another device's name. Each native run receives a durable
seven-day mutation epoch; reconnect keeps that epoch, while replacement closes
old runs and preserves interrupted attempts. Leaving/remapping fences old runs
permanently. Run tombstones are bounded at 10,000 and are not silently pruned into
new execution authority. This storage subset does not expose remote operation
frames or enable production task routing. Participant local reservation checking,
remote evidence provenance, repository warnings and wake remain integration gates.

## Resource shapes

Local workspace mappings may carry an optional opaque UUID `repository_id`,
chosen explicitly by the operator when grouping checkouts of one logical
repository. Legacy mappings omit it and remain ungrouped. Neither Git remotes nor
matching paths grant membership. Cross-checkout reservation warnings disclose
only mapped workspace IDs and reservation metadata to current members of the
same space and repository. Reserve responses include bounded `overlap_warnings`
and `warnings_truncated`; these warnings never reject a physical grant. Both
owners must still be members, and expired/shared-only leases are excluded.
Each reservation snapshots this sharing scope at creation. Joining/remapping a
workspace does not expose preexisting private reservations; leaving stops warning
disclosure, while existing physical leases retain their normal expiry behavior.

| Resource | Required semantics |
| --- | --- |
| Agent | ID, display name, native CLI/version when known, device, space, workspace, presence and observed readiness source; unknown values remain unknown |
| Task | IDs for issuer/assignee/reviewer, eligible pool if unassigned, description, acceptance, state, revision, version, prerequisites, optional deadlines, current attempt, archive marker |
| Attempt | Task/revision, owner agent/device/run/session epoch, started/finished timestamps when observed, certainty (`active`, `interrupted`, `unknown`, `finished`), outcome and prior evidence |
| Message | ID, space, sender/recipient, thread, optional reply/task/revision, subject/body, coordinator sequence, acknowledgement; queued time is distinct from acknowledgement |
| Event | Event ID, per-space sequence, type, actor, resource/attempt references, coordinator observed time, bounded payload; imported events may lack historical timestamps |
| Reservation | ID, workspace/checkout, relative file/subtree, exclusive/shared mode, agent/attempt, expiry and abandoned-owner warning |
| Evidence | ID, task/attempt, kind, producing device/workspace, bounded metadata, optional commit/hash, provenance (`reported` or `locally_verified`) |

An agent can inspect task content when it is the issuer, assignee or reviewer, or while it is eligible to claim an unassigned task. Shared queue visibility ends for nonparticipants after a successful claim. A local human operator can inspect its authorized spaces. Thread content is visible to its participants, with task links independently checked; device enrollment alone does not grant every space.

## Task transitions

`blocked` means unresolved prerequisites. Other waiting causes remain explicit conditions on queued/running/submitted work, such as offline assignee, busy terminal, protected draft, interrupted attempt or overdue review.

| From | Action and caller | Preconditions | To / result |
| --- | --- | --- | --- |
| absent | Assign/create: participating issuer or operator | Authorized space, reviewer valid, no cycle, eligible target/pool | `queued` if prerequisites accepted, otherwise `blocked`; revision 1 |
| blocked | Dependency completion: coordinator | Every prerequisite accepted | `queued` |
| queued/blocked | Set dependencies: issuer/operator | Expected version, no executing attempt, acyclic same-space edges | Recompute `queued`/`blocked` |
| queued, unassigned | Claim: eligible agent | Current presence, expected version, no other running task, prerequisites accepted | Assigned queued task; one concurrent winner |
| queued, assigned | Start: assignee | Current run/revision, expected version if supplied, prerequisites accepted, no other active attempt | `running`; coordinator issues attempt ID |
| running | Progress: active assignee | Matching attempt/run/revision | `running`, append progress/wait condition |
| running | Submit: active assignee | Matching attempt/run/revision and valid evidence | `submitted`; execution attempt finished |
| submitted | Review accept: reviewer/operator | Expected revision/version, reviewer is not assignee | `accepted` |
| submitted | Review request changes: reviewer/operator | Expected revision/version, feedback nonempty | Increment revision, clear current attempt, `queued`/`blocked`; retain earlier evidence |
| queued/blocked/submitted | Cancel: issuer/operator | Expected version | `cancelled`; stop pending assignment/review delivery |
| running | Cancel: issuer/operator or execution deadline | Expected version for external command | `cancel_requested`; emit stop request, no new work grant |
| cancel_requested | Confirm cancelled: active assignee or local controller | Explicit stopped outcome or observed owning process exit | `cancelled`; finish attempt |
| cancel_requested | Late submit | Any submission, including success | Reject current-state mutation; retain optional late evidence through progress/audit path |
| running | Fail: active assignee | Matching attempt/run/revision, reason | `failed`; finish attempt |
| queued/blocked | Start deadline passes: coordinator | Never started, deadline reached | `expired` |
| failed/expired/cancelled | Retry: issuer/operator | Expected version, previous execution known stopped or explicit operator override | Increment revision; new queued/blocked work |
| queued/blocked or terminal failure state | Reassign: issuer/operator | Expected version, new authorized target/reviewer, no live prior attempt | Increment revision; new queued/blocked work |
| running/cancel_requested with unknown execution | Recover/override: issuer/operator as constrained below | No silent ownership transfer | Explicit resume of valid owning run, or operator override fences old attempt and records risk before retry/reassignment |

No ordinary agent can override uncertain execution or revoke devices. Reassignment of running work requires the cancel/stop flow first. A task's accepted result is immutable; further work creates a new task rather than reopening historical acceptance. The execution deadline does not discard a submitted result; the review deadline only marks it overdue.

All competing transitions use a database transaction and version/state checks. The winning transition determines the next valid operations; the loser receives `version_conflict`, `invalid_state` or `stale_attempt` with safe current state. Resource and event writes commit together.

## Existing MCP operations to extend

The names below are existing public tools; optional additions must retain their current field meanings.

| Tool | Extension |
| --- | --- |
| `warp_agent_register` | Return device/workspace/space identity and negotiated collaboration features; identity still derives from terminal binding |
| `warp_agent_list` | Return qualified display names, connection/readiness certainty, current task and waiting reason; paginate if needed |
| `warp_agent_send` | Optional `subject`, `thread_id`, `reply_to`, `task_id`; validate recipient/thread/task visibility and preserve request ID semantics |
| `warp_agent_inbox` | Optional cursor/limit; distinguish unread messages and control/task notifications |
| `warp_agent_ack` | Remains acknowledgement of ordinary messages; cannot consume required task transitions |
| `warp_agent_wait` | Remains optional active-turn waiting; remote delivery cannot bypass local automatic wake guards |
| `warp_agent_ready` | Readiness of the authenticated current run only; never authoritative cancellation or task completion |
| `warp_task_assign` | Optional dependencies, start/execution/review deadlines; existing explicit `to` path stays valid. A new create-pool operation handles unassigned tasks |
| `warp_task_get` | Return versions, attempts, prerequisites, wait reasons, structured evidence and delivery facts; large history is paginated separately |
| `warp_task_start` | Optional expected version; return new attempt ID and require a committed result before executing assigned work |
| `warp_task_submit` | Optional expected version/attempt ID/evidence IDs; retain text `result` and `evidence` for compatibility |
| `warp_task_review` | Optional expected version; preserve designated reviewer and no-self-review rules; return resulting revision/version |

`warp_task_submit` from an older tool call without an attempt ID is accepted only if its authenticated run and revision identify exactly one current active attempt. Recovery never relaxes that requirement.

## New MCP operations

All mutating rows require `request_id`; existing-resource changes require `expected_version` where the resource is versioned. Returned resources carry their new version and event sequence.

| Proposed tool | Main arguments | Caller and result |
| --- | --- | --- |
| `warp_task_list` | state/assignee filters, cursor, limit, include_archived | Authorized visible tasks only |
| `warp_task_history` | task ID, cursor, limit | Task participant; ordered durable events including prior submitted results/reviews |
| `warp_task_create_pool` | description, acceptance, eligible agent IDs, reviewer, dependencies, optional deadlines | Issuer; create an explicitly scoped unassigned task |
| `warp_task_claim` | task ID, expected version | Eligible agent; atomically obtain assignment |
| `warp_task_progress` | task ID, attempt ID, revision, expected version, note, optional waiting reason/evidence IDs | Active assignee; append attributed progress without changing ownership |
| `warp_task_cancel` | task ID, expected version, reason | Issuer; immediate cancellation or a stop request |
| `warp_task_finish_cancel` | task ID, attempt ID, revision, expected version, reason | Active assignee explicitly confirms it stopped; controller exit evidence uses a private controller operation |
| `warp_task_fail` | task ID, attempt ID, revision, expected version, reason, evidence IDs | Active assignee reports failure |
| `warp_task_retry` | task ID, expected version, reason, optional start-deadline replacement/explicit clear | Issuer; known-stopped failed/expired/cancelled work only |
| `warp_task_reassign` | task ID, expected version, assignee, optional reviewer/start-deadline replacement, reason | Issuer; eligible non-running work only |
| `warp_task_set_dependencies` | task ID, expected version, prerequisite IDs | Issuer; acyclic, unstarted task only |
| `warp_thread_get` | thread ID, cursor, limit | Thread participant; ordered history |
| `warp_message_search` | query, optional task/thread filter, cursor, limit | Search only caller-visible records |
| `warp_evidence_add` | task ID, attempt ID, descriptor | Active assignee; create a bounded evidence reference |
| `warp_file_reserve` | workspace-relative paths, mode, task/attempt, TTL | Current workspace owner; all-or-nothing conflict check and grant |
| `warp_file_renew` | reservation IDs, TTL | Current owner of the same valid attempt |
| `warp_file_release` | reservation IDs | Owner; idempotent release |
| `warp_file_reservations` | optional relative path filter, cursor, limit | Same authorized workspace/space; disclose only coordination metadata |

Initial bounds: 100 dependency edges per task, 100 eligible agents per pool, 100 paths per reservation request and 32 evidence descriptors per submission. Reservation TTL defaults to 600 seconds with a maximum 3600 seconds. Bound names/subjects at 256 UTF-8 bytes and reference metadata at 8192 bytes each; total frames remain bounded. Test these as boundary constraints, not as inferred agent capabilities.

The panel can expose the corresponding operations to the human operator. Separate private controller APIs cover space create/join/leave, enrollment/grants, device revocation, event subscription, history archive/export/purge, observed process exit and uncertain-execution override. These operations are not registered as agent MCP tools.

Task detail includes recent attempts/review feedback and `history_truncated`;
older work remains accessible through `warp_task_history`. New submissions retain
result/evidence in their atomic attempt-attributed event before a rework clears the
current result. Old events that never recorded those values remain historical
metadata; missing prior text is not fabricated. Read pages may return fewer rows
than their requested limit to stay within the negotiated frame bound.

## History retention safety

Archival requires every attempt to have a recorded finish and known certainty,
including superseded revisions. An overlap override is not proof of completion.
Age-based archival uses the same requirement. Purge previews and deletion share
one eligibility predicate, retaining uncertain attempts, prerequisite references,
unread task messages and parent messages referenced outside the purged task.
Purge previews return the current project sequence. `HistoryPurge.expected_sequence`
optionally fences deletion against that reviewed sequence inside the mutation
transaction; native confirmation always supplies it. A changed sequence requires
a refreshed preview and new intent. Legacy callers may omit this field.
Acknowledged standalone messages are eligible only when no reply/thread still
references them; each purge deletes exactly the eligible set observed before deletion.

## Evidence descriptor

`kind` is one of `file`, `commit`, `diff`, `test`. All descriptors identify the producing workspace and task attempt through authenticated context.

- `file`: relative path, optional content hash and optional commit. Local verification
  requires a 64-digit SHA-256 hash (optionally prefixed `sha256:`), hashes the
  explicitly requested repository file and rejects a mismatch. Verification reads
  at most 64 MiB and excludes credential paths. Metadata creation never reads files.
- `commit`: repository ID and Git object ID; an optional branch is descriptive, not proof of content identity.
- `diff`: relative path to a patch or base/head commit IDs. No automatic patch application or fetching.
- `test`: command label, reported outcome (`passed`, `failed`, `not_run`), optional exit code and bounded summary/reference. A reported exit code is not independently verified merely because it is structured.

Commit/diff object verification accepts only full hexadecimal Git object IDs and
checks existing local objects with lazy fetching disabled and a bounded process
deadline. It does not execute a test command, apply a patch or fetch remote content.

Reject absolute paths, traversal and schemes masquerading as local files. Do not read a referenced file until an authorized local viewer/verifier requests it. The first remote release transfers descriptors and summaries only, not referenced bytes.

## Internal SSH session contract

This protocol is independent of the native MCP wire protocol. Use the existing four-byte big-endian length prefix plus UTF-8 JSON with `0 < length <= 1 MiB`. The parser rejects malformed, oversize and unknown message types and has bounded read/write deadlines and queue sizes.

```json
{
  "protocol_major": 2,
  "protocol_minor": 0,
  "type": "operation",
  "request_id": "<uuid>",
  "space_id": "<uuid>",
  "mutation_epoch": "<server-issued-id>",
  "payload": {"op": "task_get", "task_id": "<uuid>"}
}
```

The example contains identifiers only. Device authentication is a separate initial in-memory handshake over verified SSH; credentials never appear in persisted event envelopes. Reads do not require a mutation epoch; mutations do. Identity fields are populated/validated by the trusted local controller, not forwarded blindly from native tool arguments.

| Message type | Direction and meaning |
| --- | --- |
| `hello` / `hello_result` | Negotiate protocol major/minor/features, coordinator ID, connection epoch, frame limits and presence lease; incompatible major fails before mutation |
| `enroll` / `enroll_result` | Single-use invitation exchange bound to this SSH connection; returns device credential once, then discards invitation |
| `authenticate` / `authenticated` | Present device credential over SSH; bind channel to device, current grant generation and granted spaces |
| `presence` | Participant announces only its own verified native agent/run/workspace bindings; coordinator assigns qualified identities |
| `heartbeat` | Refresh connection presence using coordinator receipt time; cannot refresh a revoked grant |
| `operation` / `operation_result` | Same domain operation semantics as local calls; result includes committed version/sequence or typed error |
| `query_request_result` | Reconcile an uncertain original epoch/request ID; return committed, not_seen, expired or inaccessible |
| `subscribe` / `event_batch` / `cursor_ack` | Per-space ordered events after confirmed cursor, bounded batches and resumption |
| `snapshot` / `snapshot_result` | Authorized projection plus high-water sequence when no cursor exists or compaction expired it |
| `wake` | Existing agent/run/message reference only; recipient app decides whether local PTY delivery is safe |
| `reconcile_attempt` | Compare known attempt owner/state to current local run after reconnect; never implicitly creates a new owner |
| `goodbye` / `revoked` | Close a session and invalidate its presence; running effects may still be unknown |

Initial negotiation frames use `type: hello`, integer `protocol_major` /
`protocol_minor`, a `features` array (at most 32 unique ASCII capability names,
1–64 bytes each), and `max_frame_bytes` (4096–1048576). `hello_result` adds
UUID `coordinator_id` / fresh `connection_epoch` and `presence_lease_ms: 30000`;
the frame maximum is the smaller advertised limit and minor version is the
smaller supported version. Unknown frame types/fields and duplicate capabilities
are rejected. Required semantic features must all be present. Negotiation grants
no identity, enrollment or operation authority; authentication is still required.

The negotiated minor version enables only intersecting capabilities (`task_control`, `dependencies`, `threads`, `reservations`, `evidence_refs`, `event_resume`). Do not infer compatibility from an app version string alone. Required feature absence prevents entering that shared space and produces actionable guidance.

Device/space enrollment changes use a separate operator request path and are persisted before success is returned. Reusing an invitation, revoked credential, prior connection epoch or another device's agent ID is rejected. A gateway must not expose prepare/activate for arbitrary terminal IDs.

## Error vocabulary

| Code | Meaning / caller response |
| --- | --- |
| `unauthorized`, `scope_denied`, `device_revoked` | Stop; do not retry with another identity automatically |
| `version_conflict`, `stale_revision`, `stale_attempt` | Refresh state; never silently substitute a new version/attempt for the same intent |
| `invalid_state`, `dependency_cycle`, `dependency_blocked` | Correct the request or resolve prerequisites |
| `request_conflict` | Same request ID used with different content; reject |
| `request_epoch_expired` | Original mutation can no longer execute; reconcile resource history before proposing new work |
| `outcome_unknown`, `execution_unknown` | Transport/process evidence is incomplete; do not claim failure or success and do not automatically reassign |
| `reservation_conflict` | Return owner/path/expiry metadata; no exclusive grant |
| `coordinator_unavailable`, `connection_required` | Preserve pending intent and show disconnected status; TaskStart cannot proceed |
| `protocol_incompatible`, `feature_unavailable` | Upgrade/restart or avoid unsupported scope; no semantic downgrade |
| `cursor_expired` | Fetch scoped snapshot and high-water sequence before resubscribing |
| `capacity_exceeded`, `storage_full` | Show capacity controls; preserve existing records; no blind retry loop |
| `invalid_input`, `frame_too_large` | Reject safely without echoing the payload |

## Executable contract checks required during implementation

- Generated tool schemas match serde variants, reject extra fields and preserve the existing twelve names/required parameters.
- Table-driven state transitions cover authorized/unauthorized callers, stale versions and both orders of each race.
- Migration fixtures preserve v1 identifiers/revisions and make the known old binary reject the new writable store.
- Concurrent claim/dependency/reservation tests use real transactions; transport replay tests drop responses after commit.
- Golden remote frames cover negotiated features, revoked epochs, partial reads and size limits on both operating systems.
- UI operator requests and agent tools reach the same state-transition functions while retaining distinct authenticated principals.

## Native local panel read contract

`Broker::operator_panel(PanelQuery)` is a trusted in-process application read,
never a native MCP tool or remote frame. Resolve the current pane's explicit
shared admission only when its producing checkout matches the active local root.
Return at most 50 authorized agents, 50 task summaries, one selected task with
independent online/interrupted flags and 50 events after the confirmed sequence.
Agent pagination uses the last unique name; task/event cursors use existing
committed sequences. A changed space clears all prior cursors and selection.
The existing broker condition variable wakes background reads; a one-second
timeout refreshes volatile draft/readiness metadata, without storing terminal
content or polling complete history each render. Offline identity is retained.
Disablement removes the live projection; opening/refreshing never takes terminal
focus. All human mutations remain separate operator calls with stable request
IDs and optimistic versions. Live visual/action acceptance remains required.

## Native local operator intent

Native task controls reuse inline EditorView fields and existing text buttons. Bind each form to its original scope, task revision/version and stable request UUID. Assignment requires explicit recipient/description/acceptance; cancellation and review require reasons. Archive only terminal tasks. Uncertain force cancellation/retry/reassignment require typing ALLOW OVERLAP and preserve recorded uncertainty. Reject context changes before submission, retain failed intent without changing its version or request UUID, and use the same Broker operator/controller functions as protocol tests. No native interrupt is claimed. Local agent focus dispatches the existing workspace terminal focus action.

A force-cancellation override acknowledges cancellation risk for its current revision; it is not permission for a later replacement execution. Retry or reassignment of that revision still requires its own explicit operator override while an attempt remains unknown, including an attempt with an overridden outcome. The authorized replacement increments revision and preserves the earlier unknown attempt.

Native assignment and pool forms expose explicit prerequisites, eligible names, optional start deadlines, execution/review timeouts and reviewer selection through the existing task operations. Details show persisted deadline policy, overdue review, eligible participants, truncated history and complete evidence descriptors; metadata never implies fetched remote content.

The trusted panel reservation projection is scoped to both the active collaboration domain and physical checkout. It pages 50 records using creation sequence, includes expired/abandoned-owner metadata and uses the same lease assembly as native MCP reads. Active private reservations in that checkout may prevent conflicting shared grants, but their owner and record are absent from the shared projection. Pagination never traverses a different checkout implicitly. Renewal/release still require owner authority; UI request controls are a separate remaining gate.

Native task filters reuse the operator task query's state, assignee and include-archived fields. Changing a filter clears task pagination and selected detail; it does not mutate task state. Participant buttons select scoped stable actor IDs, including authorized offline participants. The filtered empty state remains distinct from missing agents or a disconnected coordinator.

Native message search uses the existing literal substring operation, capped at 256 query bytes and 50 results per page. Thread buttons retrieve the immutable root and replies with existing sequence cursors. The trusted local operator may inspect threads within its selected domain; ordinary native agents retain participant-only thread/search visibility. The reserved operator program cannot be registered through native participation. Search text and result content are transient panel state, never capture diagnostics or telemetry. Changing panes, disabling communication or leaving the message view clears the read intent; no search action acknowledges messages or changes task state.

An explicit native local-file evidence action resolves its descriptor through the same producing-checkout lookup used by operator verification, then rechecks canonical containment and that the target is a file. It dispatches the existing code-view action only if the selected task and pane remain current and communication is enabled. Remote references are never treated as coordinator file paths. Opening current content does not verify a reported hash. Unavailable content receives native guidance rather than falling back to a same-named file in the viewing checkout. A shared admission rejected after preflight similarly produces native guidance while withholding shared communication access.

## Space preview pagination and admission

`space_list` accepts optional `cursor` (last unique name) and `limit` (existing
1–200 bound, default 50). Omitted fields preserve old requests. The implicit
private row appears only on the first page; responses include mapped workspaces
and repository metadata plus the next cursor. A private-only one-row page uses
the empty cursor to advance to shared rows. Workspace admission carries the exact
reviewed ID, space and root in a trusted native field; validation and preparation
reject a remapped snapshot. No shell/MCP parameter can select admission.

Native history is an explicit scoped read view. It shows capacity, protected purge
counts and 50-record ordered export pages. Copying a page exports that exact
scoped JSON page through the existing clipboard only on explicit action. Archive
age requires explicit days; purge requires typing DELETE HISTORY and pins the
original preview sequence/counts. No automatic history eviction occurs.

Native reservation maintenance uses trusted controller `ReservationUpdate` with
reservation ID, original owner/checkout/expiry, reason, request UUID and optional
TTL (1–3600 seconds). A TTL renews only an unexpired reservation with a currently
authorized owner and active, unfinished linked attempt. No TTL explicitly releases
the coordination record, including abandoned owners, without completing an attempt
or stopping writes. The original tuple must still match; foreign domains are
rejected. Native release requires typing RELEASE RESERVATION. Agent MCP ownership
and native-run renewal checks remain unchanged.

## Authenticated gateway dispatch

After authentication, `actor_announce` binds the app's verified native session/run
UUIDs to an existing device-qualified workspace tuple. The coordinator supplies
actor and mutation epoch; frame fields cannot select a different enrolled device.
`operation` carries a correlation UUID, connection epoch, assigned actor/run and
an existing agent `Operation`. Every call rechecks current device generation,
space grants and run before using the single Store engine and request ledger.
Read operations require read access; acknowledgement and transitions require write.
Agent registration, readiness and wait are excluded until guarded remote presence
and wake integration is implemented. Controller operations are never accepted.
`events` resumes a 50-record default, 200-record maximum space event page under
current read grants. Replies keep the connection/correlation epoch separate from
the original mutation epoch. Unknown transport outcomes require original-intent
reconciliation rather than fresh request IDs. Production application opt-in and
participant persistence/wake remain separate unfinished gates.

Trusted controller `RemoteWorkspaceMap` reviews device UUID and current generation,
space UUID, device-local checkout UUID, display label and optional repository UUID.
It returns the device-qualified mapping without opening the participant path.
Generation/grants are checked inside the same request-ledger transaction; replay
returns the original mapping receipt and remapping cannot revive old native runs.
This is a host operator API, never an agent operation or remote frame.

The owned SSH client validates actor admission against the authenticated device,
reviewed space and verified native session, and validates every operation/event
response's connection/correlation/original mutation epoch. It never creates a new
mutation request ID. Callers must persist pending intent before sending mutations;
a transport failure does not authorize executing a replacement. Peer error text
is discarded while known domain codes and numeric version metadata are retained.


### Original remote receipt reconciliation

Authenticated `reconcile` frames carry the original actor, mutation epoch and full
mutation operation/request UUID. They query the existing request ledger under the
controller mutex without executing a mutation. Current device generation, space
grant and actor membership are checked before any receipt lookup. Matching retained
receipts return `committed` and the original result even after run replacement;
changed request content is `request_conflict`. An absent receipt returns
`not_committed` only for the still-current authorized original epoch. A closed,
expired or missing original run fails explicitly and cannot authorize a new epoch.
`reconciled` replies preserve connection/correlation/original mutation identities.
Participant persistence and wake routing remain unfinished.


### Participant durable intents

The participant's private SQLite Store exposes staging, bounded recovery pages,
confirmed-result retention and explicit removal of resolved remote intents.
`remote_pending_intents` is additive metadata in schema v6, independent of the
coordinator task tables. A row pins coordinator/device/space, original actor,
mutation epoch, request UUID and serialized operation; credentials, invitations and
terminal capabilities are excluded. A duplicate unchanged stage returns its
original row, while changed identity/content is `request_conflict`. Unknown outcomes
cannot be removed, including after restart or expiry. The limits are 1,000 rows,
16 MiB total payload/result bytes, half a protocol frame per mutation and one frame
per result. Recovery uses ordered actor/request cursors with 50 rows per page.
Native routing must stage before sending and record only a matched operation result
or committed reconciliation receipt. No automatic replay or eviction is enabled.

`Connection::execute_intent/reconcile_intent` consume the staged row directly,
checking its original coordinator, authenticated device, granted space, actor,
epoch and operation/request UUID before framing. They retain all original mutation
identities and create only a new transport correlation ID. They do not hold a
participant Store/broker lock across network I/O or automatically replay failures.


### Remote native presence lease

`actor_heartbeat` identifies only an already admitted actor and its original
mutation epoch. The coordinator rechecks current device generation/grants and run
membership, then retains a 30-second monotonic receipt-time lease bound to this
connection. No participant clock, draft, approval or readiness assertion is accepted
by this frame. Disconnect, controller shutdown, revocation and run replacement
invalidate presence; durable tasks and uncertain effects remain unchanged. Remote
pool claim requires this current native lease before mutation/replay. Presence alone
never makes an actor ready or enables terminal wake. Host panel online/runtime
projections use the same validated lease, independently of durable task outcome.


### Reviewed device space-grant changes

Trusted local controller `DeviceGrantUpdate` pins device UUID, original expected
generation and one space UUID. `mode` is `read`, `write`, or null to remove that
grant. Validate both durable identities and the 32-space device limit before any
write; atomically change the grant, increment generation and record the operator
audit event through the existing request ledger. Old authenticated connections
lose read, mutation, cursor and presence authority before replay. Grant changes do
not stop execution or erase tasks/attempts. `DeviceList` retains `space_ids` and
adds explicit space/mode grant records for review. Remote agents cannot invoke
this controller operation. Native settings integration remains pending.

Loss of native presence also records `task_execution_unknown` for unfinished active
attempts owned by that exact remote actor/epoch, increments affected task versions
once, and leaves task outcome/state and finish timestamps unchanged. It never
converts reconnect/heartbeat into restored execution authority. Original committed
receipts remain queryable; unresolved effects require explicit recovery before
continuation. Host access sweeps expired/revoked leases and disconnect/shutdown
cleanup uses the same exact-run storage transition.


### Scoped remote snapshot and confirmed cursors

Authenticated `snapshot` carries space UUID, optional record cursor, optional
original `expected_sequence` and bounded limit. It reuses ordered history export
under the broker mutex and returns records/cursor plus a high-water sequence.
Continuation pages require that original sequence; any intervening mutation yields
`cursor_expired` and requires restarting the scoped snapshot. Future/out-of-range
event cursors are also rejected instead of silently skipping later changes.

`cursor_ack` persists only a confirmed sequence for the authenticated device and
granted space. It cannot advance beyond coordinator high-water or move backwards.
Grant/generation checks precede reading/updating cursor rows. These read projections
and cursor metadata are not task ownership or execution authority; no automatic
replay, compaction or remote terminal wake is introduced.

With no explicit `after`, remote event reads resume from that device/space's durable
confirmed cursor (zero before the first confirmation). Replies include
`confirmed_cursor` and current `high_water`. SQLite reopening preserves that cursor;
reads and transport reconnects never advance it without an explicit confirmation.


### Original-owner outcome confirmation after transient loss

An unknown attempt with no outcome/finish timestamp may be explicitly finalized
only by its unchanged authenticated actor, original current mutation run and current
task revision. Submit, failure and cancellation-stop confirmation validate those
identities and the original attempt/version; they retain their existing task-state
preconditions. This confirms an outcome on the same attempt without creating or
restoring execution ownership. New start/claim, progress and lease renewal remain
blocked by unknown execution. Replaced/revoked epochs, overridden outcomes and old
revisions cannot finalize the current task. Reconnect/heartbeat alone does nothing;
only the original owner's explicit result/stop operation changes certainty.


### Native participant observation projection

Panel agent rows additionally expose device (`local` or the qualified enrolled
UUID), physical checkout identity and age/source of the last native observation.
Local ages use the broker's monotonic activity/input/output observations; remote
ages use validated receipt-time presence. These are presentation facts, independent
of readiness and task outcome. Offline observations remain unavailable, not a
fabricated timestamp. Remote actors are labeled by device and never acquire a
local terminal focus target. Fields remain optional in the native read model.


### Native connection replacement fence

The coordinator assigns a monotonically increasing in-memory connection order
after Hello negotiation. ActorAnnounce checks the device-qualified native identity
before registration; an older connection cannot replace a newer admission. Keep
the latest owner after disconnect, bounded to 10,000 native identities, so delayed
announcements cannot reclaim a run. No participant clock determines this order.

Every mutation requires the current native connection and a valid receipt-time
presence lease, before request-ledger replay. Heartbeat requires the same owner
and cannot transfer ownership. A replacement announcement retains the original
mutation epoch and marks previous unfinished execution unknown. Reconciliation
can reveal an authorized retained receipt, but an absent receipt requires current
connection ownership. The replacement fence and receipt lookup share the broker
mutex; old queued frames cannot commit after that lookup returns not_committed.
