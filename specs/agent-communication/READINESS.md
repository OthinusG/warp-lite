# Agent readiness repair

## Acceptance

- `ready` means no task is executing. Native/rich input drafts do not change it, including when the user types and deletes text. Typing still cancels a delayed Enter to preserve user input.
- Report `activity`, `ready`, `has_draft`, `draft_state`, `can_auto_submit`, and `delivery_blockers` for each peer. `ready` describes an idle lifecycle; automatic prompt submission additionally requires empty input and a settled, live run.
- Distinguish startup/unknown, working, waiting for approval, waiting for user input, cancelled, idle, cooperative waiting, dispatching, and offline/revoked runs. Do not infer task completion from quiet output.
- Only actual native turns, submitted work, or successfully started tasks revoke readiness. Ordinary MCP queries, messages, acknowledgements, and claims of queued tasks do not. Submission/failure/cancellation completion restores readiness and explicitly reminds the agent to call final `agent_ready`; native turn completion and final `agent_ready` also restore readiness for local and peer work. Native busy/approval state still prevents automatic prompt injection.
- Codex uses its original native backend with --no-daemon. Structured app lifecycle and final-action MCP readiness apply to the bound terminal; no daemon/proxy notification adapter is used.
- Preserve drafts, queued messages, run fencing, cancellation, one-time startup registration, and delayed Enter guards across every managed native MCP program.

## Implementation

Reuse the local broker and per-launch native relay. Add an authenticated internal lifecycle field to the broker request, not a public model tool. Keep idle evidence while editing; track only safe, known native editing operations, and fail closed with `draft_state: unknown` for unsupported editing/history/attachments. Rich input and permission overlays remain app-owned delivery guards and are reflected in peer status.

## Verification

Run the focused agent_bus readiness and session relay tests on GitHub, then both
desktop application checks. Preserve drafts, queued delivery, permission/user
questions, cancellation and stale-run rejection. Run authenticated native Codex
acceptance with the cloud-built test executable and original installed client;
never replace user commands, configuration or CODEX_HOME. Compilation and
simulated relay tests are separate from model-level acceptance.

Historical 2026-10-02 proxy probes are superseded by the native session adaptation.
Current verification is recorded in ../DEEP-CLEANUP.md and
../agent-communication-v2/ACCEPTANCE.md.
