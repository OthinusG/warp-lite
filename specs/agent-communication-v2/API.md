# SSH Agent Communication API

The active scope is PLAN S0–S5. The former manager contract is archived in
legacy-ssh-project-manager/API.md and is not an implementation checklist.

## Transport and authority

Use bounded, length-prefixed protobuf over a clean system SSH stdio connection.
No format auto-detection, public listener, device enrollment or federation.
Initialize verifies protocol major, private account/service/boot/connection IDs.
ProjectOpen binds a canonical native root and opaque project UUID. Every later
request/result carries that exact fence; replaced roots and late responses fail.
Removed host-status/list tags and detach/attach/generation fields are reserved.

Capabilities: project_open, managed_agent, project_tasks, project_mcp.

## Agent lifecycle and IO

TerminalLaunch carries a launch-intent UUID, absolute executable, bounded argv,
terminal dimensions and optional Agent program. A managed Agent uses the existing
session::launch adapter inside the service-owned native PTY. Fresh MCP endpoint,
capability and terminal binding are injected into its environment only; credentials
never enter control responses, arguments or preferences. The Broker run UUID equals
the owned native run. Ordinary commands receive no Agent authority.

Only the launch connection may read/input/resize/stop/release its run. Input is
bounded and sequenced; output is bounded and paged with explicit truncation.
Release requires observed full group/job exit and output EOF. Disconnect revokes
MCP and requests owned stop; no listing, detach/reattach or takeover. Failed native
activity observation retains ownership. Launch-intent replay reconciles only the
same connection and immutable payload.

## Existing communication domain

ProjectTasks carries a typed command (panel/operator/controller), query generation
and bounded JSON domain payload. The private remote Broker/Store is authoritative.
The GUI receives value/error envelopes, scoped projections and fixed failure
metadata. It cannot register Agents, claim readiness, start/submit Agent execution,
or access retired device operations. Existing messages/tasks/reviews/versions/
original request receipts remain unchanged. Changed-payload retries conflict;
unknown writes retain their original intent and are never automatically replayed.

Panel Agent rows include optional `run` from the currently observed Broker run.
It is projection metadata, separate from the persisted Agent identity, and is
absent when no active run is observed. Offline cached rows are explicitly stale.

## Explicit SSH terminal launch

`warpai-companion agent <absolute-root> <program> <absolute-vendor-executable>
[vendor-arguments...]` joins the same private service, forwards native terminal IO
and tracks resize. The companion supports mcp and the existing forward relay.
System SSH and vendor authentication remain user-managed on the remote account.
