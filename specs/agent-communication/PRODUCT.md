# Native CLI Agent Communication

## Summary

All CLI agent sessions managed by Warp Lite running in Warp Lite can exchange messages, assign tasks, submit results, and accept or request changes. Warp owns local routing and durable task state. External messaging products are not required.

## Behavior

1. Participation is explicit. After connecting the bundled MCP bridge, an agent registers a project-local name. Its identity is bound to its live Warp terminal run; callers cannot select a sender identity.
2. Agents discover and communicate with other registered agents in the same project. Other projects are isolated by default.
3. A task names an assignee, a reviewer (the assigner by default), a description, and acceptance criteria. Names are resolved to stable identities when the task is created.
4. The assignee explicitly starts a queued task, then submits a result and verification evidence. Only its reviewer can accept the submitted revision or request changes with feedback. Submission does not imply acceptance.
5. Rework increments the revision. Stale operations, unauthorized state transitions, and conflicting retries fail without changing the task or its notifications.
6. Messages and task transitions survive an application restart. Live capabilities and run identities do not. Interrupted work requires an explicit restart acknowledgement; it is not silently marked complete.
7. During an active model turn, agents may optionally call `warp_agent_wait` to receive work directly. The normal dormant-prompt path uses queue polling and automatic submission; holding a wait call open is not required.
8. An agent that has finished its turn and is waiting at its input prompt must also start newly assigned work without a human prompt. Agents announce readiness with `warp_agent_ready` before ending a turn; installed native lifecycle listeners can also announce completion. Warp submits a short inbox notification through the existing agent-specific prompt pipeline. Busy, permission-blocked, remotely controlled, exited, replaced, or manually edited terminals are not submitted to. Messages remain pending until the agent acknowledges or consumes them.
9. An agent already executing an assigned task does not receive another assignment through waiting, but can receive messages and review requests. Messages remain pending until explicitly acknowledged or consumed by the corresponding task transition.
10. Each supported CLI uses the same protocol. Registration is refused when the CLI no longer owns the terminal. A replacement process cannot reuse an old run's requests.
11. Communication remains local, adds no cloud/account/telemetry dependency, and does not start Warp's disabled platform MCP client runtime.
12. Upstream synchronization replays an auditable downstream patch. Repeated application is harmless; partial application or conflicting upstream changes fail before publication.

## Delivery

The user approved implementation and requested GitHub-only compilation. Source changes ship inside `.github/patches/agent-communication.patch`, with replay integrated into the existing restoration script and remote validation workflows. Personal agent configuration and credentials are not inspected. MCP setup uses verified vendor configuration commands where available. Native MCP client support is required; no shell-tool fallback is included.

## First-release coverage

Coverage is derived from `CLIAgent` and its command recognition, including all aliases: Claude Code, Gemini, Codex, Amp, Droid, OpenCode, Copilot, Pi, oh-my-pi, Auggie, Cursor, Goose, Hermes, Mistral Vibe, Antigravity, Grok Build, DeepSeek Harness, Qoder, Trae, and user-configured custom agents (`Unknown`). Recognition includes `qodercn`, `traecn`, `cursor-agent`, and DeepSeek TUI-specific launch rules.

Every eligible managed type must pass the common registration, messaging, delegation, cooperative waiting, submission, acceptance, and rework contract. Native MCP configuration differs by vendor. Agents or installed versions without native MCP client support are excluded and listed in COVERAGE.md. Do not equate command recognition, protocol simulation, MCP compatibility, and authenticated live-model acceptance. Report each separately.


An upstream addition to the managed-agent enum must be assessed for native MCP support; eligible types use the native binding's canonical program name. Validation compares coverage against the restored enum rather than maintaining a four-program allowlist.
