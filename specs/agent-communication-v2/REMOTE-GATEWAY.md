# Remote gateway implementation checkpoint

Status: enrollment/storage and SSH negotiation are verified subsets. An enrollment-only
controller and byte relay are verified with real IPC. Authenticated actor announcement,
original-epoch task dispatch and granted event pages now use the same coordinator
Store; extended real-IPC duplicate/revocation checks passed on both target OS in
run 36999119942. Production
application opt-in, participant routing/persistence and guarded remote wake remain
unfinished. This document
fixes those remaining boundaries; it does not accept M6.

## Endpoint and lifetime

Keep the terminal MCP endpoint unchanged. Start a second per-user Unix socket
(macOS) or named pipe (Windows) only through explicit trusted application opt-in.
Reuse the running broker's Tokio runtime, mutex and normalized Store. The host
application remains the database writer; the gateway never opens its database,
starts a daemon or exposes terminal preparation, local operator controls or shell
execution. Use protected current-user Windows ACLs for every pipe instance, with
network clients rejected; use the existing short private macOS runtime directory.

Publish a bounded, non-secret runtime descriptor containing endpoint, process ID,
protocol major, durable coordinator UUID and a fresh ownership nonce. The descriptor
is per-user and never contains terminal capabilities, invitations or credentials.
Remove only the descriptor owned by this nonce. A live descriptor cannot be
replaced by another controller; stale discovery fails visibly and must be recovered
without silently changing coordinator authority. Test isolated descriptors, rather
than reading or replacing the daily application's state.

The fixed `warp-agent remote-stdio` command relays framed protocol bytes between
stdin/stdout and this controller. Stderr carries bounded static diagnostics. It
performs no PTY, command evaluation or model invocation. Absent/stopped applications
return `coordinator_unavailable`. Child streams and connections have bounded
buffers/timeouts and are closed when the app disables remote participation.

## Authentication and dispatch

Reuse the existing typed hello codec and required-feature negotiation; introduce
no second framing implementation. Negotiate before enrollment/authentication.
Enrollment consumes an operator-issued invitation once and returns its credential
in memory once. Authentication binds a connection to a durable device generation.
No free-form frame field may choose another sender/device/native terminal principal.

After each connection's authentication, recheck current generation and space/role
grants for every read, mutation, cursor and heartbeat. Connection epochs are fresh
and do not replace the original mutation epoch of a pending request. Disabling a
controller must fence already accepted requests under the same mutex before they
commit, remove remote wake eligibility and invalidate connection presence. This
never proves a running model stopped or changes a task into a successful outcome.

Credentials on participants must go through the existing platform secure-storage
model. If storage fails, do not write plaintext preferences or enable participation.
Persist only non-secret host aliases, coordinator IDs, device IDs and granted-space
choices. Platform credentials are neither command arguments nor MCP tool inputs.

## Remote actor and workspace boundaries

Map a participant's app-verified native identities to coordinator-assigned actors,
qualified by enrolled device. Reclaim requires that device and original mapped
workspace; native run changes preserve uncertain attempts. A participant cannot
announce host/private actors or map arbitrary local paths on the coordinator.

Remote workspace metadata is device-qualified. It is not a coordinator filesystem
root: evidence is unavailable locally until explicitly fetched/verified, and local
path checks remain the participant application's responsibility. Existing local
shared bindings continue resolving their immutable host workspace rows. Reservation
exclusivity must include private/shared work in one participant checkout, while
coordinator overlap warnings remain scoped to explicit repository memberships.

Route native operations through the same Store state machine under the verified
remote actor/run. Controller-only invitation, grant, mapping and override APIs are
excluded from remote agent dispatch. Wake frames name only an existing
actor/run/message; the receiving app repeats all native draft/approval/readiness
and delayed-Enter checks.

## Reconnect and tests

Bound pending intent storage and retain original request ID, actor, coordinator,
space and mutation epoch. Reconcile unknown outcomes before replay; a new connection
cannot mint a new execution owner. Expired/inaccessible outcomes remain explicit.
Resume per-space events by confirmed cursor; stale/compacted cursors require an
explicit scoped snapshot with a high-water mark. Presence uses receipt-time leases,
not clocks supplied by participants.

Required smallest checks: clean real stdio plus controller IPC; unavailable app;
single-use/expired enrollment; grant and generation revocation during a live
session; malformed/oversize/stalled frames; another principal/space denial; controller
disablement fences pending calls; cross-device identity/workspace isolation;
dropped-response reconciliation; draft-safe local wake; offline evidence; app/device
restart without duplicate ownership. Keep macOS/Windows CI and physical/model
acceptance separate. Do not advertise production remote task availability from an
enrollment-only channel.

## Next implementation: device-qualified actors

This section is a design boundary, not a shipped routing claim. Preserve SQLite
v5 local workspace rows and their unique canonical-root index. Add a v6 migration
with a v5 backup/sentinel and separate remote workspace, actor-binding and run
rows; do not reinterpret an existing host path as participant metadata.

- Remote workspaces carry device, granted space, participant checkout identity,
  optional explicit repository identity and bounded display metadata. Their
  physical reservation key is device plus checkout, independent of collaboration
  space; two computers using the same path never become one physical checkout.
- A remote actor binding records coordinator-assigned agent UUID, authenticated
  device, remote workspace and app-verified native identity. Existing names may be
  reclaimed only through the original device/workspace/native binding. Reject a
  collision with either a host actor or another device before calling the existing
  registration helper. Ordinary operation arguments cannot choose this binding.
- Run rows separate the participant native run from a server-issued mutation epoch
  and its receipt-time expiry/closure. Reconnecting the same verified run resumes
  its original epoch. Replacement closes that run and uses existing interruption
  recovery; it cannot convert uncertain ownership into a stopped outcome.
- Dispatch native operations through Store::execute under the resolved actor and
  run, after current device grant, space membership, controller ownership and run
  checks. Authorization precedes replay lookup. No remote frame dispatches local
  operator, preparation, grant or override APIs.
- Reconciliation checks only the original actor/epoch/request receipt. An absent
  receipt under an expired/closed epoch is not permission to invent a new epoch.
  Retain pending operation content and original identity in bounded participant
  intent storage; never persist invitation, credential or local terminal capability.
- Host filesystem APIs remain limited to host workspace bindings. Remote lease
  paths are bounded normalized relative metadata; participant native code repeats
  canonical/path/symlink validation in its own checkout. Before remote reserve,
  the participant also checks its local broker so private/local shared reservations
  in that checkout still conflict. Unknown reserve outcomes retain their original
  request/lease until reconciliation or expiry, rather than silently releasing and
  minting another request.
- Remote evidence always records the authenticated producing device/workspace.
  Host verification returns unavailable for private participant content; a reported
  hash is not host verification. Existing host evidence keeps its original checkout.
- Presence is a receipt-time lease for each admitted native run. Loss removes claim
  and wake eligibility, while durable tasks and uncertain attempts remain. A wake
  names only an existing actor/run/message and is delivered to its enrolled app;
  that app repeats the existing local native/draft/approval/delayed-Enter guards.

Before production opt-in, real coordinator/participant checks must cover host and
cross-device name/workspace isolation, unchanged private leases/tasks, dropped
responses and original-epoch reconciliation, generation/grant revocation before
replay, stale run replacement, offline evidence and a draft-safe local wake.


## Participant intent storage checkpoint

Reuse the participant's existing private SQLite Store for a separate
`remote_pending_intents` table. Its rows are not authoritative task state. Before
sending, atomically retain coordinator/device/space, original actor/epoch/request
UUID and the serialized mutation. Never include enrollment/credential/capability
frames. Reusing a key with changed context/content is a conflict. Reopening the app
must recover the same rows; expiry does not delete unknown outcomes.

Bound retained rows to 1,000 and total payload/result bytes to 16 MiB, with at most
one protocol frame per payload. Quota rejection must precede network transmission.
Record a confirmed matching result without changing the original intent, and allow
explicit removal only after that result is retained. No automatic eviction, epoch
replacement or ownership synthesis. Focused SQLite checks cover crash/reopen,
changed content, immutable result, pending removal denial and row/byte limits.
Production app routing remains gated on secure credentials, native presence/wake
and the existing draft guards; this table alone does not enable participation.
