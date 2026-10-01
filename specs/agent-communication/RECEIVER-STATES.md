# Receiver State Audit

Target platforms: macOS and Windows only. This audit covers every managed `CLIAgent` type and confirmed custom native MCP clients, rather than a four-vendor subset.

## Contract

The issuer becomes the coordinator when it delegates. It polls every outstanding task, receives progress messages and results, and reviews results before reporting completion. Peer instructions execute through the receiver's existing CLI prompt and PTY, so the CLI renders its normal work and final report. A broker response only confirms queue admission.

## Shared state matrix

| Receiver state | Required behavior | Verification |
| --- | --- | --- |
| Not installed or native adapter unavailable | Visible unsupported state when installed; never authorize or claim connected | Native adapter/discovery checks |
| Configured but MCP not loaded | Restart guidance; cannot be addressed as a live participant | Native discovery and recipient authentication checks |
| MCP discovery before terminal activation | Brief bounded retry for an unregistered valid binding; never retry expired runs or capabilities | Real child-process stdio startup race regression |
| Fresh bare interactive launch | Native discovery grants one initial readiness lease; no first human/model prompt needed | Every-program startup matrix and app PTY regression |
| Startup with task arguments | No inferred idle lease; require native idle event or final-action readiness | Every-program startup matrix |
| Batch/print or ACP/server mode | Refuse automatic prompt paste into noninteractive/protocol stdin even if MCP is present | Managed alias and native batch-mode regression |
| Startup busy, draft or cancellation before discovery | Registration cannot overwrite input/lifecycle invalidation | Every-program startup matrix |
| Busy model turn | Queue work; actual user/peer submissions and native turn-start events set ready false. Read-only MCP operations do not change it | Every-program real stdio readiness matrix |
| Native structured idle prompt | Establish readiness without changing presentation status | Listener forwards IdlePrompt to broker |
| Opaque completion notification | Presentation only; cannot establish readiness | Listener status contract |
| Opaque approval, edit approval, plan prompt or question | Known native prefixes block delivery; do not treat them as completion | Codex/Grok notification regression |
| Structured permission request or question | Revoke readiness and preserve the prompt | Native listener and app blocked-state regression |
| Empty input after completed turn | Poll pending work and submit once after readiness/output settle | Every-program dormant and app PTY regression |
| Rich input draft, saved draft or image attachment | Keep ready true while idle; preserve user input through a separate automatic-submission guard | Shared view guard and rich draft regression |
| Native input editing | Keep idle readiness unchanged; invalidate a scheduled Enter and protect the draft independently | Every-program real stdio readiness matrix |
| Native input cancellation | Pause automatic delivery until a new user submission; expose the pause separately from lifecycle readiness | Every-program cancellation regression |
| Read-only MCP query, inbox read, message send or ACK | Does not start a task and does not revoke readiness | Every-program real stdio readiness matrix |
| Queued task claimed | Preserve readiness until execution starts | Task execution readiness regression |
| Assigned task started | Set ready false only after successful transition; failures preserve prior state | Task execution readiness regression |
| Task submitted, failed or cancellation confirmed | Restore ready and instruct the agent to make `warp_agent_ready` its final tool action; native busy still protects the prompt | Task execution readiness regression |
| Automatic delivery claimed but not submitted | Preserve ready; expose dispatching and fence delayed Enter. Cancelled delivery cannot leave an idle receiver permanently busy | Shared wake guard |
| Repeated native discovery | Reuse identity; never re-arm a busy run | Every-program rediscovery matrix |
| Shell alias, abbreviation or function hiding launch arguments | Do not infer initial idle from a displayed bare alias | Shared startup guard |
| Cooperative wait | Return queued work directly; do not also paste into PTY | Socket wait handoff regression |
| Already executing a delegated task | Another assignment waits; messages/review requests remain available | Task state machine and next-work filtering |
| Several queued tasks | Start/submit/review each revision; task notifications cannot be discarded by ordinary ACK | Every-program lifecycle and task ACK regression |
| PTY submitted, not acknowledged | Keep message pending and avoid duplicate automatic submission | Dormant and startup matrix |
| Draft/busy/exit/replacement during delayed Enter | Invalidate the claim; never submit Enter to a different run | App delayed Enter and broker run regression |
| Receiver exits or pane closes | Revoke run; queued/running work survives; coordinator sees offline/interrupted | Run rejection, task runtime and persistence checks |
| Receiver returns in a new pane | Explicit offline identity reclaim is possible after automatic discovery; running work requires explicit start | Coordinator/recovery regression |
| Rename after acquiring work | Refuse identity abandonment | Coordinator/recovery regression |
| Communication disabled or agent unchecked | Revoke immediately; cleanup failure cannot restore authorization | Policy and reversible setup checks |
| Different repository/worktree or remote/shared terminal | Reject routing/submission | Project isolation and shared view guard |

## Native adapter coverage

The MCP peer snapshot separates `activity`, `native_activity`, `readiness_source`, `ready`, `has_draft`, `draft_state`, `paused`, `waiting`, `can_auto_submit`, `can_start_task`, and `delivery_blockers`. `ready` reports execution state; drafts, permissions, dispatching, cooperative waiting, settling and expired runs independently gate automatic PTY submission. Offline/revoked runs cannot be queried as live peers and remain visible through task recovery state.

All enum aliases pass the adapter contract checks. QoderCN's actual installed help confirms user-scope stdio add/get/remove syntax. Qoder entry directories, shell startup/current PATH, and interpreter PATH passthrough are included. Configuration ownership, malformed input, idempotency and preservation are checked with temporary files/executables, never personal credentials.

## Limits of verification

Protocol tests exercise actual local IPC and stdio MCP, with simulated vendor identities. App tests exercise every managed type's native PTY submission strategy. These tests do not prove authenticated vendor-model execution, version-specific startup dialogs, UI rendering or final-action instruction compliance. Native MCP discovery alone cannot prove that an undocumented vendor startup modal is absent. Live acceptance in a patched application must cover fresh launch, permission dialogs, busy queueing, normal screen output and reply delivery for each installed vendor. Agents started with arguments or versions without usable idle events retain the explicit final-action readiness contract.

The broker owns transport and task state; the vendor model follows coordination instructions. The app cannot guarantee that an arbitrary model keeps polling or submits results just by describing those tools. Do not report a simulated matrix as live vendor acceptance.
