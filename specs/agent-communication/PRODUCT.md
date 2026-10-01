# Native CLI Agent Communication

## Summary

All CLI agent sessions managed by Warpai running in Warpai can exchange messages, assign tasks, submit results, and accept or request changes. Warpai owns local routing and durable task state. External messaging products are not required.

## Behavior

1. Participation is controlled by Settings > Features > Agent communication, disabled by default. Enabling it discovers installed managed CLI agents. Checking an agent configures the bundled native MCP bridge and required environment passthrough in the background. Unchecking removes only Warpai-owned configuration; disabling communication revokes all live communication immediately and cleans up all owned configuration. Running clients may require restart to reload MCP configuration. Native discovery assigns unique project-local identities automatically.
2. Agents discover and communicate with other registered agents in the same project. Other projects are isolated by default.
3. A task names an assignee, a reviewer (the assigner by default), a description, and acceptance criteria. Names are resolved to stable identities when the task is created.
4. The assignee explicitly starts a queued task, then submits a result and verification evidence. Only its reviewer can accept the submitted revision or request changes with feedback. Submission does not imply acceptance.
5. Rework increments the revision. Stale operations, unauthorized state transitions, and conflicting retries fail without changing the task or its notifications.
6. Messages and task transitions survive an application restart. Live capabilities and run identities do not. Interrupted work requires an explicit restart acknowledgement; it is not silently marked complete.
7. During an active model turn, agents may optionally call `warp_agent_wait` to receive work directly. The normal dormant-prompt path uses queue polling and automatic submission; holding a wait call open is not required.
8. An agent that has finished its turn and is waiting at its input prompt must also start newly assigned work without a human prompt. Agents announce readiness with `warp_agent_ready` before ending a turn; installed native lifecycle listeners can also announce completion. Warpai submits a short inbox notification through the existing agent-specific prompt pipeline. Busy, permission-blocked, remotely controlled, exited, replaced, or manually edited terminals are not submitted to. Messages remain pending until the agent acknowledges or consumes them.
9. An agent already executing an assigned task does not receive another assignment through waiting, but can receive messages and review requests. Messages remain pending until explicitly acknowledged or consumed by the corresponding task transition.
10. Each supported CLI uses the same protocol. Registration is refused when the CLI no longer owns the terminal. A replacement process cannot reuse an old run's requests.
11. Communication remains local, adds no cloud/account/telemetry dependency, and does not start Warpai's disabled platform MCP client runtime.
12. Source is maintained directly in this independent repository. Builds and releases do not fetch upstream or replay patches.

## Settings and project communication acceptance

- Implement settings and reversible vendor configuration first, then replace manual peer selection with automatic project routing.
- Reuse native switches, checkboxes, and settings layout; all UI text is English.
- Discovery follows the managed enum and aliases. Installed tools without a verified native setup adapter are visibly unavailable, never falsely enabled. No agents are installed by Warpai.
- Configuration runs off the UI thread, reports success/failure per agent, preserves unrelated settings, rejects conflicting ownership, and never writes live capability values.
- Enable/disable operations are serialized. Only successfully configured agents are authorized. Disabling revokes active runs before cleanup; failed cleanup remains visible and retryable after restart.
- Same-project registered agents automatically discover, message, and delegate to each other. There is no peer picker, pair permission, or connection step.
- Canonical repository root scopes projects; subdirectories share the root, separate repositories and worktrees remain isolated. Offline/unloaded/disallowed clients cannot participate.
- Test configuration add/remove/idempotency/conflicts, project isolation, disable/re-enable and stale runs, existing task lifecycle, and dormant wake protection.
- GitHub checks default/platform builds and packages review artifacts. No local compilation; live vendor and screenshot validation limitations must be reported honestly.

## Delivery

## Receiver feedback and complete adapter coverage (2026-10-01)

- Every live receiver shows a native notification when peer work is queued, including while busy or awaiting a readiness signal. Once ready, every managed agent uses the same guarded automatic PTY submission path and visibly processes its inbox.
- MCP readiness is authoritative across all agent types. An opaque notification listener or stale presentation status must not veto it; explicit blocked events, user drafts, cancellation, and replaced runs still prevent submission. Opaque OSC notifications must not establish readiness because approval notifications use the same channel.
- Sending reports queue admission, not task completion or receiver acknowledgement. Peer listing exposes pending work and readiness so agents can report delivery honestly.
- Native setup must cover every named managed agent and alias, including Vibe, DeepSeek Harness, Qoder/QoderCN, Antigravity, Grok, and Trae. Custom commands retain capability probing. Preserve unrelated configuration and never persist runtime capabilities.
- Acceptance includes receivers with actual opaque and structured listeners, native setup add/remove for every vendor format, native pending notifications, busy queueing, and cancellation/replacement protection. Compile and run Rust checks only on GitHub.

The user approved implementation and requested GitHub-only compilation. Source changes are committed directly under app/src/agent_communication and crates/agent_bus, with independent remote validation workflows. Personal agent configuration and credentials are not inspected. MCP setup uses verified vendor configuration commands where available. Native MCP client support is required; no shell-tool fallback is included.

## First-release coverage

Coverage is derived from `CLIAgent` and its command recognition, including all aliases: Claude Code, Gemini, Codex, Amp, Droid, OpenCode, Copilot, Pi, oh-my-pi, Auggie, Cursor, Goose, Hermes, Mistral Vibe, Antigravity, Grok Build, DeepSeek Harness, Qoder, Trae, and user-configured custom agents (`Unknown`). Recognition includes `qodercn`, `traecn`, `cursor-agent`, and DeepSeek TUI-specific launch rules.

Every eligible managed type must pass the common registration, messaging, delegation, cooperative waiting, submission, acceptance, and rework contract. Native MCP configuration differs by vendor. Agents or installed versions without native MCP client support are excluded and listed in COVERAGE.md. Do not equate command recognition, protocol simulation, MCP compatibility, and authenticated live-model acceptance. Report each separately.


An upstream addition to the managed-agent enum must be assessed for native MCP support; eligible types use the native binding's canonical program name. Validation compares coverage against the restored enum rather than maintaining a four-program allowlist.

## Independent maintenance revision (2026-10-01)

Retire upstream synchronization, replay patches and restoration scripts. All feature/branding/platform changes are direct repository source. Keep independent GitHub validation and tagged macOS/Windows release workflows, including the communication companion. Linux is not a target platform. Historical upstream plans above are superseded by this revision. Receiver state acceptance is specified in RECEIVER-STATES.md.
