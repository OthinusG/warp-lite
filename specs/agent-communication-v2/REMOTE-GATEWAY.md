# Remote gateway implementation checkpoint

Status: enrollment/storage and SSH negotiation are verified subsets. An enrollment-only
controller and byte relay are implemented with real IPC checks pending CI. There
is no production application opt-in or remote task routing yet. This document
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
