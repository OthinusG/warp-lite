# Agent readiness repair

## Acceptance

- `ready` means no task is executing. Native/rich input drafts do not change it, including when the user types and deletes text. Typing still cancels a delayed Enter to preserve user input.
- Report `activity`, `ready`, `has_draft`, `draft_state`, `can_auto_submit`, and `delivery_blockers` for each peer. `ready` describes an idle lifecycle; automatic prompt submission additionally requires empty input and a settled, live run.
- Distinguish startup/unknown, working, waiting for approval, waiting for user input, cancelled, idle, cooperative waiting, dispatching, and offline/revoked runs. Do not infer task completion from quiet output.
- Only actual native turns, submitted work, or successfully started tasks revoke readiness. Ordinary MCP queries, messages, acknowledgements, and claims of queued tasks do not. Submission/failure/cancellation completion restores readiness and explicitly reminds the agent to call final `agent_ready`; native turn completion and final `agent_ready` also restore readiness for local and peer work. Native busy/approval state still prevents automatic prompt injection.
- Codex completion and approval/user-input events must apply only to the owned thread, in transport order, including notifications the TUI otherwise opts out of.
- Preserve drafts, queued messages, run fencing, cancellation, one-time startup registration, and delayed Enter guards across every managed native MCP program.

## Implementation

Reuse the local broker and per-launch native relay. Add an authenticated internal lifecycle field to the broker request, not a public model tool. Keep idle evidence while editing; track only safe, known native editing operations, and fail closed with `draft_state: unknown` for unsupported editing/history/attachments. Rich input and permission overlays remain app-owned delivery guards and are reflected in peer status.

## Verification

Use the small `warp-agent-bus` crate on this machine, not a full app build or remote CI. Reproduce both reported transitions before fixing them. Exercise queued delivery, clearing drafts, MCP busy/ready cycles, repeated tasks, permission/user questions, cancellation, stale Enter, replacement, and native notification ordering. Use isolated broker storage and short-lived MCP/PTY processes; do not replace the installed app or existing agents. Separate simulated native event checks from real vendor acceptance. No upload, push or release is requested.

Local results (2026-10-02): shared lifecycle regressions pass using 20 short-lived stdio MCP clients, and the owned Codex notification regressions pass. These are shared transport and simulated lifecycle checks, not proof of real vendor task completion. The isolated native Codex launch timed out establishing its session connection; QoderCN discovered MCP but did not report completion within the test deadline. Real vendor two-turn acceptance and installed application behavior remain unverified. The broad library suite also reports three storage failures (history export, pool-claim exclusivity, operator-managed workspace/device operations); those are outside the readiness checks and require separate investigation.
