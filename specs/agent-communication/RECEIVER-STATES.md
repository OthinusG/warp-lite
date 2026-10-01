# Receiver State Audit

Target platforms: macOS and Windows only. This audit covers every managed `CLIAgent` type and confirmed custom native MCP clients, rather than a four-vendor subset.

## Contract

The issuer becomes the coordinator when it delegates. It polls every outstanding task, receives progress messages and results, and reviews results before reporting completion. Peer instructions execute through the receiver's existing CLI prompt and PTY, so the CLI renders its normal work and final report. A broker response only confirms queue admission.

## Shared state matrix

| Receiver state | Required behavior | Verification |
| --- | --- | --- |
| Not installed or native adapter unavailable | Visible unsupported state when installed; never authorize or claim connected | Native adapter/discovery checks |
| Configured but MCP not loaded | Restart guidance; cannot be addressed as a live participant | Native discovery and recipient authentication checks |
| Fresh bare interactive launch | Native discovery grants one initial readiness lease; no first human/model prompt needed | Every-program startup matrix and app PTY regression |
| Startup with task arguments | No inferred idle lease; require native idle event or final-action readiness | Every-program startup matrix |
| Batch/print or ACP/server mode | Refuse automatic prompt paste into noninteractive/protocol stdin even if MCP is present | Managed alias and native batch-mode regression |
| Startup busy, draft or cancellation before discovery | Registration cannot overwrite input/lifecycle invalidation | Every-program startup matrix |
| Busy model turn | Queue work; regular MCP operations/user submissions invalidate readiness | Every-program startup matrix and dormant queue regression |
| Native structured idle prompt | Establish readiness without changing presentation status | Listener forwards IdlePrompt to broker |
| Opaque completion notification | Presentation only; cannot establish readiness | Listener status contract |
| Opaque approval, edit approval, plan prompt or question | Known native prefixes block delivery; do not treat them as completion | Codex/Grok notification regression |
| Structured permission request or question | Revoke readiness and preserve the prompt | Native listener and app blocked-state regression |
| Empty input after completed turn | Poll pending work and submit once after readiness/output settle | Every-program dormant and app PTY regression |
| Rich input draft, saved draft or image attachment | Preserve user input; do not submit | Shared view guard and rich draft regression |
| Native input editing or cancellation | Revoke lease; readiness cannot override a draft; resume requires user submit | Dormant queue and stale Enter regression |
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

All enum aliases pass the adapter contract checks. QoderCN's actual installed help confirms user-scope stdio add/get/remove syntax. Qoder entry directories, shell startup/current PATH, and interpreter PATH passthrough are included. Configuration ownership, malformed input, idempotency and preservation are checked with temporary files/executables, never personal credentials.

## Limits of verification

Protocol tests exercise actual local IPC and stdio MCP, with simulated vendor identities. App tests exercise every managed type's native PTY submission strategy. These tests do not prove authenticated vendor-model execution, version-specific startup dialogs, UI rendering or final-action instruction compliance. Native MCP discovery alone cannot prove that an undocumented vendor startup modal is absent. Live acceptance in a patched application must cover fresh launch, permission dialogs, busy queueing, normal screen output and reply delivery for each installed vendor. Agents started with arguments or versions without usable idle events retain the explicit final-action readiness contract.

The broker owns transport and task state; the vendor model follows coordination instructions. The app cannot guarantee that an arbitrary model keeps polling or submits results just by describing those tools. Do not report a simulated matrix as live vendor acceptance.
