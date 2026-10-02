# SSH Remote and Collaboration Contracts

Date: 2026-10-03. Status: revised semantic contracts; new remote types/operations below are proposals until implemented. Existing Rust serde/generated MCP schemas remain the executable source for local tools. This document supersedes the enrolled-device SSH contract, retained in [legacy-machine-collaboration/API.md](legacy-machine-collaboration/API.md).

## 1. Trust and authority rules

- Local GUI controller resolves profile → verified SSH environment → explicit remote project → service/attachment. Agent payloads do not select any of those authorities.
- Remote MCP resolves remote capability → current project/actor/run. Agent tools never receive unrestricted GUI shell/connection/profile privileges.
- Project task/message/history authority is the remote companion's existing Store; local GUI projections and pending intents are not writable task copies. Local projects retain their existing local Store authority.
- SSH credentials and keys remain system-managed. Do not forward local terminal capability/endpoint/launch variables or Vibe MCP configuration. No device invitation, persistent cross-app bearer enrollment or space grant exchange is part of the new channel.
- A trusted SSH account is allowed to execute its own commands through GUI terminals. That does not authorize a task/message payload to issue arbitrary terminal input, file access outside its admitted project or process-control calls.
- Every mutation/launch/file-finalization intent keeps original request ID/content and original authority/run/version. A lost reply is unknown, not safe to replay. Authorization and connection/run fencing precede receipt lookup or execution.
- Existing task revision/version/attempt/epoch semantics and twelve original MCP names stay unchanged. Domain tools are schema-derived, not duplicated GUI-specific transitions. Text remains bounded at 8192 UTF-8 bytes; optional timeout durations remain 1–604800 seconds. UTC defines durable deadlines and monotonic receipt time defines live observations.
- Existing optional expected_version/attempt_id compatibility remains resolved only through authenticated current run/revision. New controller mutations require explicit versions where relevant; changed original payload is request_conflict. Schema major negotiation is separate from SSH/helper negotiation.
- Control frame limit stays bounded (initial target 1 MiB); file/terminal bytes use bounded streaming chunks and do not enlarge that cap. Page default 50/max 200; worker queues/backpressure limits become executable constants at R2/R3.
- No raw peer error, terminal transcript, secret, environment dump or file content is retained in diagnostics/events. User-requested remote file/terminal display is distinct from diagnostics export.

## 2. Identity shapes

All IDs below are opaque identifiers. A display string is never an authorization key. New resource names are semantic design names, not currently exported Rust symbols.

| Resource | Required non-secret fields | Boundary |
| --- | --- | --- |
| RemoteConnectionProfile | profile_id, display_name, SSH alias or host/user/port, optional config/identity-file reference, optional default root | Local bookmark only; no password/key contents/coordinator credential/device grants |
| RemoteEnvironment | environment_id, verified server identity/trust reference, authenticated account, remote OS/architecture, helper version/capabilities | Alias/root text alone cannot derive identity; trust change requires review |
| RemoteProject | project_id, environment_id, remote canonical root identity, display root, remote path semantics, collaboration scope | Same host's other projects remain isolated; selected root immutable for existing bindings |
| ServiceAttachment | durable service_id, service_boot_id, connection_epoch, attachment_id, project_id, negotiated features | Boot/connection replacement fences old callbacks and input; not an Agent epoch |
| RemoteSession | session_id, project_id, terminal kind, display name, attachment state, current run_id, owned retention capability | Session label/PID is not process authority |
| RemoteRun | run_id, service_boot_id, owned process token, actor_id if Agent, current mutation epoch, lifecycle source/state | Replacement never inherits old readiness/tasks/input |
| RemoteFileRef | environment_id, project_id/root handle, remote relative name, type, optional fingerprint, displayed absolute spelling | Never passed to local canonicalization as a local path |
| TransferIntent | transfer_id, original profile/environment/root, direction, explicit source/destination, source fingerprint, overwrite policy, owned temporary path | Does not authorize arbitrary extra files or adopt another partial transfer |
| GuiPendingIntent | authority/project, original operator/actor and epoch, request_id, operation payload/fingerprint, pending/confirmed result | Bounded immutable original content; unresolved rows never silently evicted |

New IDs do not reinterpret old device_id/coordinator_id columns. Legacy metadata remains read-only until a deliberate schema migration. Remote evidence can retain legacy device fields for old rows, but new file resolution uses the authenticated producing environment/project.

## 3. Connection and capability state

Connection lifecycle: disconnected → connecting → authenticating/host_verification → connected; a failure becomes failed with a typed reason. Loss after connection becomes reconnecting/offline, not authenticated-ready. Profile removal marks removed and revokes local callback eligibility before cleanup.

Independent capabilities: terminal, sftp, scoped_file_metadata, conflict_checked_write, atomic_replace, transfer_resume, helper, collaboration, native_lifecycle, guarded_wake, session_retention, session_reattach. A server may provide only SFTP/terminal. Required features are negotiated before advertising operations. Unsupported features return feature_unavailable and disable related GUI actions.

States and feature replies contain source/observation time, not secrets. A heartbeat refreshes connectivity only. It cannot acknowledge a task, grant readiness, validate a file version or prove process exit.

## 4. Managed control channel

Prefer extending the existing remote_server protobuf envelope and client. Its current framing is four-byte little-endian length plus protobuf bytes. The obsolete agent-device relay uses four-byte big-endian length plus JSON: these are incompatible protocols. R0/R2 must pin one versioned managed-control wire contract; do not mix encodings, infer them from untrusted input or expose the old gateway under a new name.

Semantic envelope fields: negotiated protocol/version/features, connection_epoch, service_boot_id, project_id, attachment/session/run identities where applicable, request_id/correlation_id, original mutation epoch for domain writes and typed payload. Authentication identity comes from the SSH connection/owned private service attach, not claimed payload fields. Fresh correlation IDs may transport the same original request; they cannot replace its request ID/content.

| Proposed control operation | Required precondition | Result / failure handling |
| --- | --- | --- |
| Initialize | Verified SSH account; compatible companion | Server/account/boot identity, protocol/features/limits; mismatch before any write |
| ProjectOpen | Explicit reviewed root; server-side canonical/containment checks | Stable project/root handle; matching alias text alone is insufficient |
| ProjectSnapshot / Events / CursorAck | Current authorized attachment/project | Bounded snapshot/high-water or ordered batches; cursor_expired requires fresh snapshot |
| SessionOpen | Explicit shell/Agent launch intent, root, safe executable/argument boundary | Owned session/run and launch receipt; lost reply reconciles before another launch |
| SessionAttach / SessionList | Same project and service ownership; valid boot/run | Current exact attachment/state; stale/replaced process refused |
| TerminalInput / Resize | Exact attachment and current run; independent input generation | Input accepted/rejected; old queued bytes cannot target replacement |
| SessionStop | Exact owned process/run | stop_requested first; stopped only after exit observation |
| SessionDetach / ProjectDisconnect | Exact local attachment | Input eligibility fenced; retained remote process state reported truthfully |
| AgentDiscover / AgentLaunch | Current remote project; remote executable/version and eligible adapter | Remote CLI/MCP/native status; no local authentication copy |
| CollaborationOperation | Bound operator or Agent principal, project, original request/epoch/version | Existing Store result/receipt; no second domain state machine |
| QueryOriginalReceipt | Current authority plus original identity/content | committed, not_committed, expired, inaccessible or unknown; no implicit execution |
| ManagedWake | Current Agent/run/message metadata plus both input safety gates | claimed/submitted/cancelled observation; not AgentAck/TaskStart |
| FileList / Stat / Read | Selected root; current environment; remote containment | Bounded entries/bytes/fingerprint; link/escape/permission errors typed |
| FileWritePrepare / Commit | Original source/fingerprint, destination policy and scoped temporary identity | Receipt/new fingerprint or conflict/commit_unknown; no blind overwrite |
| FileMkdir / Rename / Delete | Exact reviewed paths/version/intent within root | Confirmed mutation or uncertain receipt; no arbitrary root redirection |

Control-only command execution is tied to an explicit GUI user terminal/command intent. A machine protocol RunCommand request must not be exposed as an unrestricted Agent communication tool. Ordinary vendor Agent commands execute through the vendor's remote runtime/permissions.

Reconnect obtains a new connection/attachment fence before reporting not_committed for an old request. Surviving remote Agent runs retain their original authoritative run epoch; GUI reconnection does not manufacture another actor or automatically mark old execution stopped. A restarted/replaced service uses its boot fence and retained durable receipts with explicit interrupted-attempt recovery.

## 5. File and transfer contract

SFTP is the only transfer protocol. An SFTP-only connection can operate without helper/MCP. File metadata/edit calls and SFTP transfers must resolve the same verified remote environment/root; never combine a file listing from one host with transfer credentials for another.

Remote path rules are negotiated: OS spelling, case behavior, supported filename encoding, separators and canonical root. Reject NUL and malformed names at the boundary, reject escapes/invalid drives/junctions remotely and preserve display spelling. Do not rewrite remote paths with local OS semantics. Raw filenames must not become shell programs or unescaped SFTP batch syntax.

FileFingerprint identifies the version/content originally opened, with remote hash/version where possible. Save carries expected_fingerprint and explicit overwrite policy. The result distinguishes unchanged/conflict/missing/permission_denied/confirmed/commit_unknown and returns confirmed new fingerprint. A size/mtime-only check cannot claim full lost-update protection.

Transfer states: queued → preparing → transferring → verifying → committing → completed. Error paths: failed, cancelled, cleanup_pending or commit_unknown. paused is advertised only with safe resume capability. Byte progress never implies remote destination commit. Cancellation after final commit reports completed/confirmed rather than inventing a rollback; cancellation before commit cleans only owned temporary paths.

Transfer identity pins direction, endpoints, source fingerprint, overwrite policy and original partial file. Resume checks unchanged identity/content; conflict requires explicit restart/review. App restart restores safe metadata only, not assumed live worker handles. Filesystem mutation receipts and actual source/destination checks reconcile missing responses; a task ledger receipt is not a filesystem receipt.

GUI file actions consult same-checkout advisory reservations. They expose conflict/warning and deliberate override where appropriate but cannot assert protection against arbitrary outside writers. Explicit evidence Open uses producing RemoteFileRef; opening does not execute/verify the file.

## 6. Session, input and Agent state contract

Project authority owns a process handle/token, run ID and boot identity. Numeric PID/display name alone cannot authorize stop, reattach or input. Stale service/connection/project/session/run/draft generations fail closed.

For managed session acceptance, detach/close-view/SSH loss retains the owned remote process until explicit Stop or observed exit. Reattachment requires the same service/run and bounded output replay; buffer truncation is visible. Remove-project with active runs explicitly offers Stop or Leave running. A helper idle exit is allowed only after no owned sessions/requests remain. Ordinary raw SSH tabs cannot advertise this managed retention contract.

Remote Agent status includes cli/version, actor/run/project, lifecycle state/source, observation age, ready flag/reason and managed attachment. idle, ready, working, approval_required, waiting_input, draft_protected, paused, offline, unknown and exited remain separate. Initial readiness behavior follows the verified native adapter, not arbitrary terminal text.

A wake references actor/run/message/task revision only. The local view checks current project/attachment, local draft/input and manual pause; the remote owner checks native activity/readiness, remote draft/approval/run and current pending work. Claim must commit its observation before bytes; delayed Enter repeats guards. Submission failure/cancellation never synthesizes acknowledgement or task outcome. Task notifications reject AgentAck with invalid_state; only corresponding task transitions consume them.

## 7. Reused task, message, history and evidence semantics

The following domain shapes/tool transitions are retained from the prior implementation. Device references in legacy evidence mean historical provenance, not new enrollment authority. New remote operations run under the remote project scope and reused engine; tool names/required arguments remain compatible.

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

An agent can inspect task content when it is the issuer, assignee or reviewer, or while it is eligible to claim an unassigned task. Shared queue visibility ends for nonparticipants after a successful claim. A local human operator can inspect its authorized spaces. Thread content is visible to its participants, with task links independently checked; a GUI SSH attachment does not grant another project's Agent identity.

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

No ordinary agent can override uncertain execution or administer connections/projects. Reassignment of running work requires the cancel/stop flow first. A task's accepted result is immutable; further work creates a new task rather than reopening historical acceptance. The execution deadline does not discard a submitted result; the review deadline only marks it overdue.

All competing transitions use a database transaction and version/state checks. The winning transition determines the next valid operations; the loser receives `version_conflict`, `invalid_state` or `stale_attempt` with safe current state. Resource and event writes commit together.

## Existing MCP operations to extend

The names below are existing public tools; optional additions must retain their current field meanings.

| Tool | Extension |
| --- | --- |
| `warp_agent_register` | Return producing environment/project/workspace/scope and negotiated features; retain legacy device provenance fields for compatibility, with identity derived from the managed remote run |
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

Reject absolute paths, traversal and schemes masquerading as local files. Do not read a referenced file until an authorized local viewer/verifier requests it. Remote task messages carry descriptors/summaries only; an explicit authorized Open action may fetch referenced bytes through the producing SSH project's file API. That fetch is separate from task-message transfer and is never automatic.


## 8. GUI intent and projection rules

The existing Tab/Pane sidebar is the sole session-management consumer of RemoteSession/RemoteRun status and actions. Backend SessionList/AgentList remains available for discovery/admission and projections; it does not authorize a separate GUI manager. Existing task/message/history APIs drive an on-demand collaboration panel, closed by default on fresh profiles without overriding existing saved selection.

Sidebar status/task badges and task details derive from the same scoped authoritative projections; do not maintain another GUI task state machine. Session navigation carries environment/project/session/run/attachment and resolves the exact existing pane; missing/replaced ownership returns unavailable. Task links never fall back to Agent display name. Opening a task from a sidebar row or focusing a terminal from task detail requires an explicit user action; events cannot open/focus the view.

SessionStop from the sidebar and TaskCancel from task detail have distinct receipts/outcomes: observed process exit can be evidence for task interruption/cancellation only through the existing owned-run checks. Requesting one never synthetically confirms the other. Retiring duplicate Agent roster/rename/reconnect/stop controls preserves actor-list queries, participant labels, task assignee pickers and historical attribution.


Reuse existing local operator contract: pin project/authority, original task revision/version, original request UUID and all form fields. A changed form is a new explicit intent. Typed uncertain-execution override remains scoped to the exact old attempt/replacement intent and never proves OS stop.

Projection replies carry authority/environment/project and a local query generation. File buffers additionally pin producing file identity; session controls pin run/attachment. Project switch, profile removal, feature disablement and connection replacement invalidate late responses before applying them. Refresh never steals terminal/editor focus.

Task snapshot/events retain bounded high-water/cursor semantics. Agent volatile observations expire independently of task history. Export is an explicit user action; cached remote data is clearly labeled stale/offline and cannot authorize a new claim/write.

## 9. Error vocabulary and receipt certainty

Keep existing domain codes unauthorized, scope_denied, invalid_input, invalid_state, version_conflict, request_conflict, stale_attempt, capacity_exceeded, epoch_expired and feature_unavailable. Add explicit SSH/helper/filesystem reasons as required by executable schemas: authentication_required, host_unverified, host_key_changed, ssh_unavailable, sftp_unavailable, helper_unavailable, incompatible_version, connection_lost, stale_attachment, path_escape, file_conflict, permission_denied, transfer_interrupted and commit_unknown.

Messages are static/sanitized and never reflect an untrusted peer payload. retryable describes possible user remediation; it does not establish uncommitted side effects. GUI offers reconnect/reconcile for unknown writes, not a generic execute-again button.

## 10. Superseded endpoints and compatibility

Retire InvitationCreate, DeviceGrantUpdate, DeviceList, DeviceRevoke and RemoteWorkspaceMap from active product/controller/gateway surfaces after CUTOVER compatibility fencing. Remove enrolled-device Authenticate/Enroll/ActorAnnounce roles, expected-coordinator enrollment metadata and gateway forwarding to another running native app. Keep local Space/Workspace operations and existing history/task controls needed by local behavior/data.

Old v6 device/workspace/run/intents are not converted through a rename. Their receipts/history remain read-only/exportable; raw enrollment credentials are not exported. Migration may remove owned legacy keys explicitly and reports cleanup failures. A newer binary never downgrades schema or leaves a previously supported old writer able to mutate new history.

## 11. Executable checks required

Derived types/feature schemas must validate bounds, unknown fields and identity pinning. Check SSH argv versus remote shell encoding separately, malicious filenames, unsupported capabilities, receipt matching, changed original content, authority/run replacement and two concurrent session/claim attempts. Use real controlled SSH/SFTP and remote helper processes for execution/file/session checks; codec fixtures alone are insufficient. See V01–V24 in ACCEPTANCE.md.
