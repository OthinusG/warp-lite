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
  functional shared-space routing. Missing/unsupported majors fail closed; install
  the matching application and companion together.
- Text fields remain at most 8192 UTF-8 bytes. Lists and reference metadata have explicit schema bounds; a frame cannot exceed 1 MiB. Unknown fields are rejected at trust boundaries. Pagination defaults to 50 items and caps at 200.
- Domain errors contain a stable code, safe human message, `retryable` flag and optional current resource version. No request payload, credential or environment dump is included. A retryable transport failure is not proof that a mutation was uncommitted.
- Deadline inputs are an optional RFC 3339 UTC `start_deadline`, `execution_timeout_seconds` measured from each attempt's committed start, and `review_timeout_seconds` measured from submission. Timeout durations are 1–604800 seconds. Retry/reassignment starts a fresh execution timeout; an expired start deadline must be explicitly replaced or cleared by the issuer, otherwise retry is rejected. The coordinator records the resulting absolute deadlines.

## Resource shapes

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
