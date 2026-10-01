# Agent readiness repair

## Acceptance

- Keep lifecycle readiness separate from native/rich input drafts. Typing must cancel a delayed Enter without discarding known idle evidence. Clearing a known draft restores delivery eligibility without a model turn or rediscovery.
- Report `activity`, `ready`, `has_draft`, `draft_state`, `can_auto_submit`, and `delivery_blockers` for each peer. `ready` describes an idle lifecycle; automatic prompt submission additionally requires empty input and a settled, live run.
- Distinguish startup/unknown, working, waiting for approval, waiting for user input, cancelled, idle, cooperative waiting, dispatching, and offline/revoked runs. Do not infer task completion from quiet output.
- Native lifecycle reports outrank model announcements. Ordinary MCP tools must not replace native status with a synthetic `agent_list` call. A final `agent_ready` releases clients without native events.
- Codex completion and approval/user-input events must apply only to the owned thread, in transport order, including notifications the TUI otherwise opts out of.
- Preserve drafts, queued messages, run fencing, cancellation, one-time startup registration, and delayed Enter guards across every managed native MCP program.

## Implementation

Reuse the local broker and per-launch native relay. Add an authenticated internal lifecycle field to the broker request, not a public model tool. Keep idle evidence while editing; track only safe, known native editing operations, and fail closed with `draft_state: unknown` for unsupported editing/history/attachments. Rich input and permission overlays remain app-owned delivery guards and are reflected in peer status.

## Verification

Use the small `warp-agent-bus` crate on this machine, not a full app build or remote CI. Reproduce both reported transitions before fixing them. Exercise queued delivery, clearing drafts, MCP busy/ready cycles, repeated tasks, permission/user questions, cancellation, stale Enter, replacement, and native notification ordering. Use isolated broker storage and short-lived MCP/PTY processes; do not replace the installed app or existing agents. Separate simulated native event checks from real vendor acceptance. No upload, push or release is requested.
