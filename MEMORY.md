# Project Memory

## Product Boundary

- User platform decision on 2026-10-01: macOS and Windows are the only target platforms. Linux implementations and tests may be dropped; avoid new Linux compatibility work and Linux CI jobs. This does not require an unrelated wholesale rewrite of inherited platform code.

- Warpai is an independently maintained AGPL local-first terminal derived from warp-lite and Warp, targeting macOS and Windows.
- The default product excludes AI agents, telemetry, cloud account/login, billing, and related platform surfaces.
- Project Explorer is a desired terminal-adjacent feature and must be restored without reintroducing excluded product dependencies.
- User naming decision on 2026-10-01: this version is branded Warpai (application name), with warpai in product prose. Rename menus/settings/notifications, visible package metadata/executables, README, and installer artwork. Preserve existing storage/bundle identifiers and protocol/tool names for compatibility; real upstream URLs and attribution remain accurate. Branding and communication live directly in repository source.

## Native Agent Communication Design

- User clarification on 2026-09-30: the target is native communication between independent CLI agents running inside Warp Lite, including task delegation, result submission, acceptance/rework, and automatic handoff. hcom and Agent Mail are references, not a required choice or final architecture. The user first requested a design, then approved implementation. First-release coverage was expanded to every CLI agent managed by Warp Lite, including aliases; the user subsequently excluded agents without native MCP support.
- Implementation ownership: a Warp-owned local broker for identities, messages, and task state, a thin stdio MCP bridge over local IPC for CLI access, and automatic handoff through polling queued work and submitting to idle native prompts. The user explicitly rejected cooperative MCP waiting as the sole handoff path: waiting at the input prompt is idle and must be woken. A common final-action readiness tool covers native MCP clients without lifecycle hooks; installed completion/busy hooks also update readiness; it must not reactivate Warp's platform MCP client manager or cloud AI product.
- User runtime report on 2026-10-01: sending from Codex appeared successful but the receiving Codex showed no action. The shared wake guard required Success whenever any listener existed, incorrectly vetoing explicit MCP readiness for opaque Codex/Grok listeners and stale status for other clients. Use the broker readiness lease as authority across every agent, retain explicit blocked/draft/run protections, and never infer idle from opaque OSC notifications that also carry approval requests. Show queued work at the receiver and report queue admission separately from acknowledgement/completion.
- This revision adds native Trae YAML-list setup (including traecli recognition), DeepSeek Harness home Cordis insertion (including dsh-tui discovery), Grok TOML setup, and Vibe runtime native environment configuration. Vibe requires a new pane after selection because its Python stdio client filters ambient Warp variables; prepare native VIBE_MCP_SERVERS in memory with user/project entries at pane startup, never persist capability values. Later profile/directory/config changes require a fresh pane to refresh the snapshot.
- Qoder command recognition does not imply session-event support: Qoder currently has neither a session listener handler nor a Warp notification plugin manager. Existing CLI session events update terminal status/context; they are not an agent-to-agent task delivery mechanism.
- `TerminalView::submit_text_to_cli_agent_pty` already submits prompts using agent-specific PTY strategies. Reuse it behind explicit readiness, live-run identity checks, and task acknowledgement; its current return value does not prove that an agent consumed a prompt, and its delayed Enter path needs lifecycle revalidation for broker use.

## Build And Release

- Primary check: `cargo check -p warp --bin warp-oss`.
- App packaging: `script/build-warp-lite-app.sh` (macOS), `script/build-warp-lite-windows.ps1` (Windows x64).
- Release artifacts include `WarpLite.app.zip`, `WarpLite.dmg`, `WarpLiteSetup-x64.exe`, and `WarpLite-windows-x64.zip`.

## Maintenance

- Upstream changes are historically selected and applied with provenance rather than merged wholesale.
- User decision on 2026-10-01: retire automatic upstream synchronization, restoration scripts and replayable patches. Keep changes as direct committed source. Retain independent GitHub compilation, testing and tagged releases; do not remove provenance or licenses. GitHub fork-network metadata is separate from the source maintenance decision.
- `OthinusG/warp-lite` is a fork of `terzigolu/warp-lite`; both default to `warp-lite/main` and do not use `master`.
- Project Explorer and persisted ToolsPanel migration are maintained directly in source, alongside CLI integrations, platform fixes, branding and native agent communication.
- Workspace initialization restores the right-side Tools Panel button for older persisted toolbar configurations that omit it.
- Standalone macOS releases build an existing repository tag. Windows release automation reads that successful run's release-target artifact and builds the same tag rather than the latest branch or release.
- macOS packaging uses Warp's native 1024×1024 `app/DockTilePlugin/Resources/mono.png`; standalone tagged releases preserve older assets because GitHub's asset endpoint returned 404 when `gh release upload --clobber` tried to replace them.
- Antigravity (`agy`) uses the standard CLI Agent fallback without Warp AI/cloud dependencies or telemetry transport.
- DeepSeek Harness accepts only dsh-tui and dsh TUI profiles in managed CLI sessions; do not add telemetry or platform integration.
- DeepSeek Harness uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles; the standard monochrome icon renderer turned the original RGB PNG's opaque white background into a blank block.
- Qoder aliases qoder, qodercli, qoder-cli and qodercn are managed CLI sessions; native command detection is independent of notification hooks.
- Qoder CLI uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles, matching DeepSeek Harness.
- Trae aliases include traecli as well as trae, traecn, trae-cli and traecn-cli. Native MCP setup uses the documented YAML list; the icon follows the original-color transparent PNG path.
- Hermes includes the hermes-agent alias and uses a transparent PNG icon through original-color rendering.
- Keep both Cursor commands cursor-agent and agent in command recognition.
- The default Lite build does not register Warp MCP file watchers, file-based server management, or MCP gallery; it retains an inert `TemplatableMCPServerManager` singleton only for compiled Warp AI callers. The full MCP runtime remains available in `warp_platform` builds, while third-party CLI agents manage their own MCP configurations.
- Do not restore inherited internal upstream workflows, cloud integrations or synchronization. Retain only independent validation and macOS/Windows release workflows.
- Windows packaging uses the existing PowerShell build script and Inno Setup; installer/portable artifacts include the native communication companion.
- Preserve Windows autoupdate log replacements for removed telemetry and the OsStr conversion for the PowerShell history read command.

- Agent communication is ordinary repository source under app/src/agent_communication and crates/agent_bus. The user still requires GitHub compilation and no local builds.
- Communication coverage must follow the managed `CLIAgent` enum automatically, not a separate four-vendor allowlist. The user explicitly excludes agents without native MCP support: list them, do not add shell-tool fallbacks. Protocol simulation and real vendor model acceptance are separate verification levels.

- macOS Unix socket endpoints must use a short private directory under `/tmp`; the system `TMPDIR` path plus a UUID can exceed macOS's 104-byte socket path ceiling. Protocol compilation/tests run on GitHub, not locally.

- User clarification: custom/downstream agents also qualify when native MCP is confirmed. `qodercn mcp add --help` confirms native stdio, and its recognized alias maps to Qoder. Do not exclude `Unknown` sessions categorically; verify their MCP capability individually.

- Native communication uses the existing Tokio dependency's Unix sockets and Windows named pipes directly. The pinned interprocess wrapper failed real Windows IO tests; avoid sync `PIPE_NOWAIT` polling and wrapper-specific readiness semantics. Keep a new listening pipe instance alive before handing off a connection, bound IO with async timeouts, and acknowledge received frames before closing pipe connections.

- Dormant wake delivery must poll while busy and submit automatically once idle. Reuse the existing agent-specific prompt submission strategies, but guard delayed Enter against run replacement and manual input. PTY submission is not message acknowledgement; pending work is consumed through MCP. Do not depend on the code-review feature flag to access managed CLI sessions.

- User clarification: busy delivery is polling, then automatic submission once idle. Manual cancellation pauses automatic submission until the user resumes with a new input; pruning a closed view must also invalidate its broker run.

- README must distinguish the original terzigolu/warp-lite removals and terminal preservation from OthinusG downstream additions. Preserve original release history; document restored Project Explorer, extra CLI integrations/aliases, Windows distribution, independent maintenance, and unreleased agent communication separately. Published v0.5.7-lite assets include both macOS and Windows x64; do not retain the old macOS-only FAQ.

- Full application test compilation exposed an inherited cloud-agent-management test referencing a field gated by `agent_management_view`. Gate that test with the same feature rather than restoring the disabled cloud UI or silently skipping native wake verification.

- User rejected the manual communication setup UX: configuring MCP flags, manually registering agent names, and requesting readiness in each prompt is too complex. The product needs a Warp-owned opt-in entry that prepares native MCP connections, assigns terminal identities, and supplies cooperation instructions; ordinary task delegation should be initiated from conversation or a terminal context action. Existing running agents may need an explicit restart to load a newly configured server. Earlier test artifacts exposed only the manual setup path; the settings revision below replaces it.

- Superseded by the settings/project-scoped revision below: user originally specified the communication entry: right-click an Agent tab, choose **Select communication peers**, then select an available agent to establish a reciprocal connection. Afterwards ordinary dialogue and agent-initiated task delegation use the same connection. Menu text and related UI messages must be English. Native MCP tool discovery registers identity automatically; connection choices are scoped to current process runs, and selected agents are restricted to their chosen peers. Native MCP setup is still a prerequisite; unloaded clients must never be falsely shown as connected.

- User revision on 2026-09-30: implement Settings-based communication enablement and automatic reversible vendor MCP/environment setup first; then remove all manual peer selection and allow every enabled native-discovered agent within the same project to communicate. Source delivery now uses direct commits; perform compilation/packaging on GitHub only.
- Settings are machine-local and default off. Track only the dedicated `warp-lite-communication` configuration and non-secret cleanup metadata. Revoke broker authorization before cleanup, retain failed cleanup for retry without restoring permission, preserve unrelated/user-modified configuration, and require vendor reload/restart where native hot-loading is unavailable.
- Maintain LF for Rust/configuration/packaging source across macOS and Windows checkouts. The old patch-specific whitespace exception is retired.

- Hermes filters stdio subprocess environments; configure runtime `${WARP_AGENT_*}` and `${WARP_TERMINAL_SESSION_UUID}` references explicitly. Vibe native stdio support alone does not establish dynamic environment passthrough; automatic setup stays unavailable until a compatible contract is verified, rather than persisting capabilities or pretending a connection exists.

- Receiver audit on 2026-10-01 covers every managed type, not only Codex. A fresh bare interactive launch needs a one-time readiness lease after native discovery, cancelled by user input/busy/blocked events. Regular MCP operations and rediscovery cannot re-arm it. Preserve task notifications until their authorized task transition; ordinary ACK must not discard assignment/review work. Main agents receive default coordination instructions and peer task summaries; TaskGet exposes offline/interrupted receivers. Explicit offline identity reclaim must work after automatic discovery but cannot abandon an identity that already has work. See specs/agent-communication/RECEIVER-STATES.md; simulated states and live vendor acceptance remain separate.
