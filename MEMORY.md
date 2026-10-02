# Project Memory

## Product Boundary

- User platform decision on 2026-10-01: macOS and Windows are the only target platforms. Linux implementations and tests may be dropped; avoid new Linux compatibility work and Linux CI jobs. This does not require an unrelated wholesale rewrite of inherited platform code.

- Warpai is an independently maintained AGPL local-first terminal derived from warp-lite and Warp, targeting macOS and Windows.
- The default product excludes AI agents, telemetry, cloud account/login, billing, and related platform surfaces.
- Project Explorer is a desired terminal-adjacent feature and must be restored without reintroducing excluded product dependencies.
- User naming decision on 2026-10-01: this version is branded Warpai (application name), with warpai in product prose. Rename menus/settings/notifications, visible package metadata/executables, README, and installer artwork. Preserve existing storage/bundle identifiers and protocol/tool names for compatibility; real upstream URLs and attribution remain accurate. Branding and communication live directly in repository source.

## Theme, UI And Sleep Defaults

- Default theme decision on 2026-10-01: fresh installs start in a built-in `Claude Warm Light` theme matching the user's local custom theme palette. Implemented as `ThemeKind::ClaudeWarmLight` (now `#[default]`, placed after `Light`) with ANSI colors in `default_themes.rs`. The alternative of writing a YAML theme file on first launch was rejected: a missing file silently falls back to Dark. Existing saved theme settings and user theme files are unaffected; the settings default follows `ThemeKind::default()` automatically.

- Warp AI startup button bug on 2026-10-01: the legacy tab-bar AI entrypoint rendered whenever `AgentMode` was compile-time disabled (the Lite default). Clicking only set in-memory panel-open state while the panel render is disabled, so the button appeared on every launch and vanished on click. Fix: gate it on `AISettings::is_any_ai_enabled`, matching every other AI entrypoint; `warp_platform` builds keep upstream behavior.

- Keep-awake toggle on 2026-10-01: a session-scoped tab-bar button (`KeepAwake` singleton, `WorkspaceAction::ToggleKeepAwake`) holds a sleep-prevention guard only while enabled and a tracked CLI agent reports `InProgress`. `crates/prevent_sleep` was restored from history, trimmed to macOS `NSProcessInfo` UserInitiated activity plus a Windows dedicated-thread `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)`; the unused Stream wrapper, `futures`/`pin-project`/`cfg-if` dependencies and `ES_AWAYMODE_REQUIRED` were dropped. Display sleep and explicit user-initiated sleep remain allowed on both platforms; the toggle is intentionally not persisted and resets on relaunch.

## Native Agent Communication Design

- Cross-checkout reservation warnings require an explicit repository UUID and
  workspace mapping plus current membership of both owners. Snapshot sharing
  identities when the lease is created: joining or remapping must not disclose
  old private reservations. Leave suppresses warning disclosure without deleting
  physical leases. SQLite v4 adds nullable identities with a v3 backup; no Git
  remote matching or filesystem lock is inferred.

- Native static UI capture can reuse the retained warpui integration driver and
  GPU window-frame interface without the removed integration crate or OS
  accessibility permission. The debug-only `WARP_COLLABORATION_CAPTURE` entry
  uses a unique data profile and keeps HOME unchanged; worker subprocesses
  retain their normal entrypoint. Missing captures must fail the check, and
  image generation alone is never visual or keyboard acceptance.
  macOS runtime is proven by two exit-0 runs of the signed `8ea58e2` debug app;
  the optional CI `capture_ui` input uses the same path on both target OS jobs.

- Comparing a task response's state/version to its current row does not establish
  a new mutation: cached submission results can match after a later ordinary user
  turn starts. Check committed request identity while holding the broker mutex;
  cache replays must never update live readiness, even when the task row has not
  changed. Keep durable response replay separate from ephemeral lifecycle effects.

- Pool availability and execution ownership are distinct. Broadcast notices may
  remain visible in inboxes, but automatic wake must choose one eligible idle
  receiver in durable notification order. Once submitted, an unclaimed notice
  must suppress another automatic prompt; recheck at claim time so cached wake
  candidates cannot fan out. An unavailable run or failed submission allows
  reevaluation; atomic TaskClaim remains the execution-assignment authority.

- Native debug review startup on 2026-10-02 exposed a Lite boundary mismatch:
  `ReferAFriend` is registered only under `warp_platform`, but the macOS app menu
  still constructed it. Its missing description panicked at startup in debug and
  produced a dead item in release. Keep menu construction under the same boundary;
  do not weaken `default_name` assertions to hide missing action registration.
- The continuation has no Windows SSH test machine or authorized model-call budget
  (user reply: unavailable). Local macOS UI automation also lacks accessibility
  permission. Keep C01/C05/C13 and real-model/device acceptance distinct from CI;
  `specs/agent-communication-v2/ACCEPTANCE.md` tracks every remaining PLAN item.

- Continuation audit on 2026-10-02: source `fb298cb` failed its dormant wake fixture
  because it expected `AgentReady` to permit delivery during an unfinished delegated
  task. Keep the executing-task readiness guard; submit/fail/confirm cancellation
  before testing a new idle wake. `PROGRESS.md` remains the engineering handover;
  static UI and real-device/model acceptance must remain separate from protocol CI.

- Continuation on 2026-10-02: the user authorized completing the v2 plan, GitHub pushes/builds and necessary operations without further permission questions, superseding the earlier readiness-repair no-upload restriction. Keep real-device/model acceptance separate from source/protocol tests. The native panel starts as an explicitly labeled static checkpoint (`WARP_COLLABORATION_PREVIEW=1`); live controller wiring waits for visual QA. The first executed v3 Windows suite exposed missing persisted assignee/reviewer updates and nested `RefCell` connection borrows in space/history reads. Cancellation-pending tasks must continue blocking another delegated start/claim until confirmed stopped.
- Review isolation: `WARP_DATA_PROFILE` works only in debug builds; release bundles ignore it. The packaging script's `--debug` mode keeps debug assertions, uses a separate review bundle identifier and supports GitHub-built native UI review. Match the app and companion: local IPC now checks protocol major 2 and registration features before exposing MCP tools. Confirmed cancellation must belong to the original attempt's run, not merely an agent name reclaimed by another session. Archive removes tasks from routine peer summaries; full history remains explicitly paginated.
- Reclaiming a disconnected agent identity does not prove its old process or daemon stopped. TaskStart must reject running/unknown recovery; the operator must explicitly fence the uncertain execution before retry creates a new revision. Keep unknown attempts' finish time unset and retain the recorded override risk. An application/terminal binding disappearing alone is not evidence of a stopped native background task.
- Locally verified evidence must reflect a real check: explicit operator file verification uses SHA-256 and checks the opened macOS/Windows handle remains inside its producing workspace before reading. Credential paths and files over 64 MiB are excluded. Git references require local objects with lazy fetching disabled; reported test labels never authorize command execution. Protocol-only workflow runs intentionally skip application/UI/package checks and must not be cited as complete release validation.
- Task response-cache rows are not permanent result history. Persist submitted result/evidence and review feedback in attempt-attributed events before clearing current fields for rework; paginate those events through participant-scoped task_history. Row-count pagination alone cannot bound long messages or evidence: account for serialized bytes and the outer MCP text envelope. At C10 scale (100k messages/10k completed tasks), source 2cccc64 measured indexed page p95 1.799 ms macOS / 2.863 ms Windows on GitHub runners; this is not a UI, disk-full or soak result.
- Preserve the original v1 `.pre-upgrade` snapshot when upgrading normalized v2 storage; the latter backup is `.pre-upgrade-v2`. A failed SQLite COMMIT can leave the transaction open: roll back before returning failure, so an identical retry starts cleanly. Start deadlines apply to a work revision, not to whether the task has any older attempts. Wake/unread inspection must use indexed, bounded reads instead of loading every pending body.

- User clarification on 2026-10-02: `ready` means no task is executing, regardless of an input draft. Typing/deleting text and ordinary MCP queries/messages/ACKs must not revoke idle readiness. Actual local/peer turn submission or task start sets it false; completion restores it and supplies a final `warp_agent_ready` reminder. Draft protection remains a separate automatic-submission guard. Claiming a delayed delivery is not task execution and must not leave ready false when input cancels it. Expose lifecycle, native state, readiness source, draft/permission/dispatch/settling blockers and task-start eligibility separately. The user authorized minimal local process verification for this repair and requested no upload, superseding the earlier GitHub-only verification preference for this task.

- Planning request on 2026-10-01: document the complete next implementation for real execution acceptance, a collaboration panel, task cancellation/failure/dependencies, file/worktree coordination, threads/evidence, durable history and remote communication. The proposed specification package starts at `specs/agent-communication-v2/PLAN.md`, with PRODUCT, TECH and API companions. It proposes a single authoritative coordinator per explicit collaboration space over SSH, while each app retains local terminal/readiness control; macOS and Windows only. The user confirmed no Figma mock and requested the existing Warpai UI style. This is a plan, not an implemented feature or authorization to deploy remote services.

- Verification for source commit 632bd1c (GitHub run 36862604389): macOS and Windows GitHub protocol suites passed. Downloaded the macOS CI bridge and repeated native handshake/registration with Claude, QoderCN, Cursor, Antigravity, the repaired ordinary dsh-tui profile and Codex. All six passed; Antigravity used the compiled prelude directly, and Codex passed with inherited network proxies while both loopback exclusion variables were deliberately absent before launch. These checks did not replace the installed app or request model turns. Application/platform/package CI is a separate verification stage.


- Antigravity 1.2.14 probes `server/discover` as a request (numeric ID), then falls back to legacy initialization after MethodNotFound. The pinned rmcp unknown-method decoder loses that request ID by interpreting it as a custom notification and closes before initialize. A bounded shared stdio/relay prelude must reject the probe using its original ID and retain unread buffered input for rmcp; do not advertise stateless MCP support. Native temporary-HOME setup completed initialize, all twelve tools and broker registration with this fallback, even before login. Official fallback rules: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/draft/basic/transports/stdio.mdx.


- Local Cursor 2026.09.18 handshake verification on 2026-10-01: its native stdio transport filters ambient Warp binding variables. Dedicated MCP JSON entries need explicit `${env:WARP_AGENT_ENDPOINT}`, `${env:WARP_AGENT_CAPABILITY}`, and `${env:WARP_TERMINAL_SESSION_UUID}` references. Native list-tools with an approved temporary server passed initialize and twelve tools after these references were added. Preserve native server approval and never persist binding values. The installed DeepSeek TUI initially failed before MCP because identical `journal-hooks-codex` insertions existed in both the home and dsh-tui profile Cordis patches. The user requested repairing daily use as well: remove only the redundant profile insertion, retain the global journal loader and MCP entry, and back up the original profile patch. The repaired ordinary profile passed native initialize, twelve tools and broker registration using a temporary MCP overlay; no model turn was requested. This is a machine-local configuration repair, not a shipped synchronization script.

- User correction on 2026-10-01: QoderCN and Qoder are separate CLI tools, not aliases sharing configuration or selection. Use separate managed identities (`qodercn`/`qoderclicn` versus `qoder`/`qodercli`/`qoder-cli`), discovery rows, configuration roots and authorization. A Qoder entry script that reports a missing CLI must not hide the installed QoderCN client. Refresh legacy selected QoderCN program metadata after native discovery; reuse the existing Qoder prompt transport and artwork. Local handshake checks must cover installed vendor frontends before submitting this repair to GitHub.

- Codex startup proxy failure on 2026-10-01: the installed proxy-aware 0.159.2 sent Warpai's `ws://127.0.0.1:<port>` remote TUI connection through inherited proxies when loopback exclusions were absent, yielding `Handshake not finished`. An isolated native TUI probe received no local connection before the failure; adding `NO_PROXY`/`no_proxy` reached HTTP 101 and initialize. Set both spellings on the shared-mode TUI child only, preserving existing lists and falling back to the other spelling when absent. Do not disable provider proxies, edit personal configuration, restart the shared daemon or infer model acceptance from this transport probe.

- Codex 0.159.2 compatibility confirmed on 2026-10-01: the installed CLI can reuse a shared app-server daemon, while native MCP `env_vars` are resolved from the server process environment. A daemon started outside Warpai cannot inherit a new pane's binding. The reported pane had all three binding variables and the installed bridge passed an isolated initialize check; the user confirmed that launching with `codex --no-daemon` changed the server from failed (0 tools) to connected (12 tools). This confirms bridge loading, not cross-pane delivery or automatic wake acceptance. Never persist capability values or restart a shared daemon to bind multiple panes to one identity. Upstream reference: https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/tui/src/startup_orchestration.rs and https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/rmcp-client/src/utils.rs.

- Follow-up runtime report: a `codex --no-daemon` recipient was online with queued work but `ready: false`. The startup lease checked the entire command against bare aliases, excluding the required flag. Use the installed native help option contract for configuration-only launches, including hidden Codex --yolo; check shell alias/function shadowing against the parsed executable. Unknown syntax and initial work await lifecycle readiness; user drafts, blocked states and repeated discovery cannot grant another startup lease. Source repair does not update an already installed app or re-arm existing runs; live wake verification requires a patched build and a fresh receiver.

- Shared-service repair on 2026-10-01: private launch aliases bind MCP through a per-launch local relay, without persisting capabilities or restarting existing Codex daemons. Codex uses its native daemon proxy with per-thread MCP overrides; embedded modes retain their native backend and use dedicated configuration overrides. Claude/Qoder use confirmed inline native configuration. Other vendor external daemons require verified session interfaces and are not proven by common relay simulations. Native MCP child working directories establish project scope before registration; later project changes fail closed. Native busy supersedes heuristic/model readiness. GitHub run 36826820613 passed both OS protocol suites, application checks, and macOS wake/setup tests; packaging was still running at this checkpoint. These source checks do not update the installed app or prove live model acceptance.

- Native Codex launch validation with GitHub-built d677305 bridge found that the TUI rejects remote URLs containing /rpc before MCP discovery. Its accepted local form is ws://host:port, while the control socket WebSocket handshake uses the root path. Corrected both endpoints. A GitHub-built 462dc3e bridge launched two real Codex TUI clients in the same trusted project against the existing daemon; both independently registered and reported native idle with correct workspace context. A different startup directory stopped at native trust selection and was not auto-confirmed. This probe used a synthetic local broker and no model turn: it does not prove patched Warpai PTY wake, message consumption, or model replies. The final GitHub-built 843681f bridge passed both shared and explicit --no-daemon native probes: two same-project clients independently registered in each mode; both shared clients also reported idle. Cleanup completed normally after native EOF. An earlier broad process-group signal was denied; use native exit/EOF for temporary CLI cleanup. Do not equate passing common relay tests with native frontend compatibility.

- Additional native probes with the 843681f bridge: two QoderCN interactive clients loaded the private inline MCP entry and independently registered with correct workspace context, without modifying personal MCP configuration. The actual missing-entry query reproduced exit code zero and the exact sentence. Claude clients stopped at native directory trust selection before MCP discovery; no trust or approval choice was auto-confirmed. These probes do not exercise model replies or the installed app settings UI.

- Upgrade race: restored panes can snapshot old selected-agent metadata before asynchronous native help discovery finishes. A missing option contract previously made a valid --yolo launch bypass the shared relay. Refresh an empty Codex contract through the resolved native executable at launch; keep CLI identity detection independent of cached readiness metadata. Two native clients with deliberately empty metadata passed --yolo/-C registration and idle checks using the GitHub-built a530fb2 bridge.

- Codex resume/fork grammar: --last shifts the first session positional into an initial prompt; without --last, the first positional selects the session and only a later positional supplies work. Reuse the installed option parser for both backend selection and this initial-work distinction. Empty resumed sessions can become ready after an owned native thread binds; supplied work waits for its first native turn. Source reference: https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/cli/src/main.rs. Native loaded-thread resumes can ignore configuration overrides while another client observes the thread or a turn is active; per-launch isolation is established for fresh threads/cold resumes, not concurrent attachments to one already-loaded conversation. Source reference: https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/app-server/src/request_processors/thread_processor.rs.

- QoderCN reports an absent user-scope MCP server with exit code zero and an exact not-found sentence. Treat only that exact response as absence; preserve conflicting user-owned entries. Reversible native setup regression covers add, recheck, remove, and conflicting output.

- Shared launcher lifecycle: the private alias directory is shared across panes, so per-pane command snapshots cannot be the current authority. Publish a nonsecret runtime catalog atomically on selection changes, retain retired executable aliases for native passthrough, and fail closed for stale communication snapshots. Initial activation still needs a fresh pane to inherit launch PATH; do not inject commands into an existing shell or restart vendor daemons.

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
- Qoder aliases qoder, qodercli and qoder-cli and the separate QoderCN aliases qodercn/qoderclicn are managed CLI sessions; native command detection is independent of notification hooks.
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

- User clarification: custom/downstream agents also qualify when native MCP is confirmed. `qodercn mcp add --help` confirms native stdio, and QoderCN now has a separate managed identity (superseding the earlier Qoder alias mapping). Do not exclude `Unknown` sessions categorically; verify their MCP capability individually.

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

- Readiness verification on 2026-10-02 separates shared IPC regressions from vendor acceptance: 20 short-lived MCP clients and owned Codex event checks pass, while isolated native Codex session setup times out and QoderCN completion is not observed before the deadline. The installed app was not replaced; do not treat these checks as verified real vendor task acceptance. A queued pool claim is not task execution and must preserve readiness until TaskStart.

- Receiver audit on 2026-10-01 covers every managed type, not only Codex. A fresh bare interactive launch needs a one-time readiness lease after native discovery, cancelled by user input/busy/blocked events. Regular MCP operations and rediscovery cannot re-arm it. Preserve task notifications until their authorized task transition; ordinary ACK must not discard assignment/review work. Main agents receive default coordination instructions and peer task summaries; TaskGet exposes offline/interrupted receivers. Explicit offline identity reclaim must work after automatic discovery but cannot abandon an identity that already has work. See specs/agent-communication/RECEIVER-STATES.md; simulated states and live vendor acceptance remain separate.

- Agent bus v2 storage landed on 2026-10-01 (M1): the v1 single-blob store was replaced by normalized SQLite tables in crates/agent_bus/src/storage.rs with versioned schema, a pre-v2 backup copy, a sentinel that makes older binaries reject an upgraded writable database, and migration tests for valid, malformed and orphaned v1 fixtures. The twelve original MCP tool names remain, with task_list as the thirteenth; all mutations route through the serde Operation contract with stable DomainError codes, optimistic expected_version checks and per-run attempt fencing. Events commit atomically with each transition and page by cursor. Mutation epochs expire after 7 days while request dedup rows retain 8 days (capped at 10000) so pruning cannot re-execute an old request; capacity is 1000 agents / 10000 tasks / 1000 pending / 100000 messages per project with cursor pagination (50 default, 200 max).

- M1 verification for source commit e08b2ed (GitHub run 36877709716): macOS and Windows protocol suites passed (migration, pagination, dedup, error-code, fencing tests), both application checks passed (`cargo check -p warp --bin warp-oss` default and `warp_platform`), and the macOS job also passed the dormant-wake, managed-configuration, setup-reversibility and notification lib tests and built the packaged review artifact. Exit criteria C03/C04/C10 are satisfied on both target platforms. These source checks do not update an installed app; local acceptance of migration and stale replay still requires the M0.2/M0.3 harness against a real UI.

- Agent bus v2 M3/M4/M5 backend landed on 2026-10-01 (commit 2a9da86, 13 files): task pools/claims/progress/cancel/retry/reassign/dependencies, deadlines and sweeps, expiring exclusive file reservations with abandoned-owner warning, spaces/workspaces/devices controller operations, threads/evidence/history export-purge, and draft/native-activity readiness tracking, plus ten new storage protocol tests. CI run 36893865896 is red on a single compile error (`Reservation.created_seq`, storage.rs:3670) and an unused-import warning (`MAX_SUBJECT`); the protocol tests have not yet executed on this commit. A second active session holds the fix plus a user-authorized readiness-semantics refinement uncommitted in lib.rs/storage.rs/transport.rs/readiness.rs/READINESS.md; review and commit those edits, never revert them. Status handover with M2 panel design notes: specs/agent-communication-v2/PROGRESS.md. Remaining: M0.2–M0.5 harness and real-device runs, M2 panel, M4 join/leave UI, M6 SSH, M7 release acceptance.

- Collaboration static preview uses the existing Tools Panel directional focus actions; explicit focus enters its keyboard handler, while ordinary opening/refresh preserves terminal input. Preview header controls stay outside the scrolling detail. The isolated native capture harness seeds an unsent draft before checking navigation and focus restoration; these checks require a GitHub-built debug executable, not local Rust compilation. Static acceptance still gates live integration.

- Remote collaboration startup uses system OpenSSH, a validated host alias and fixed `warp-agent remote-stdio` command, with strict known-host verification, BatchMode and separate pipes. Disable forwarding, local command hooks and control-socket reuse. Offline `ssh -G` tests use an empty disposable configuration and never read credentials or connect; this startup spike does not establish enrollment, grants or remote task support.

- Windows OpenSSH offline probes must close a disposable configuration file before spawning ssh; keeping its NamedTempFile handle open rejected the probe on the Windows runner. Using a retained TempPath after closing the handle passed macOS/Windows system option parsing in run 36951069744. Keep diagnostics limited to fixed option names/status, without dumping configuration output.

- Eight-hour deterministic backend soak is opt-in through the validation workflow and split into two four-hour jobs per target OS. The Python stdlib runner excludes compilation time, requires matching source/OS/duration across phases, discards raw test output and writes atomic metadata-only reports. Short smoke runs never set eight_hour_pass. Backend repetition does not establish an eight-hour native UI/draft session or vendor-model acceptance.

- Local shared routing uses fresh app-selected terminal capabilities and SQLite v5 agent-to-workspace bindings. Reuse the existing immutable workspace root rather than duplicating paths. Tasks/events use a space domain; file conflicts and evidence use producing checkouts. Metadata joining alone leaves old private work untouched. Leaving/remapping durably revokes access and wake without claiming effects stopped; no shared UI or remote support is enabled from backend tests alone.

- Windows debug UI capture must stage the same DXC/ConPTY/runtime DLLs, PowerShell bootstrap and OpenConsole files used by the portable package beside its executable. A bare target/debug binary is not a runnable review package. Keep capture failure diagnostics limited to source locations/status; use protocol_only plus capture_ui for debug captures without recompiling release packages.

- SQLite RefCell borrows can live through chained iterator adapters after load; materialize rows before filtering with authorization queries. Shared AgentList exposed this on both OS real-IPC tests.
- Remote invitation retries must persist only the safe receipt, not the raw one-time invitation. Replay cannot redisplay the secret. Enrollment uses OS-random 256-bit credentials, SHA-256 verifiers and locked subtle constant-time comparison; participant persistence must use the existing platform secure storage when gateway wiring lands.

- Windows named-pipe defaults include Everyone/Anonymous read access. Native/control listeners should use a protected current-user SID DACL, reject network clients and disallow handle inheritance; inspect the actual kernel DACL in Windows CI. This OS boundary complements capabilities/device grants and does not replace them.

- Enrollment controller discovery is non-secret and nonce-owned; reuse the broker runtime and mutex, and invalidate device generations when participation stops/restarts. A gateway only relays framed bytes and never opens the database. Enrollment/heartbeat IPC does not establish remote native task routing or production opt-in.

- Windows native screenshot readback needs surface COPY_SRC independently of the old integration_tests feature. Enable only for the explicit debug collaboration capture or integration tests; reject unsupported usage/format before GPU copy encoding.

- macOS controller sockets share the existing /tmp broker runtime directory; the full Unix socket path must remain below SUN_LEN. Use a short random basename, not a long descriptive prefix, and remove only the nonce-owned socket on controller drop.

- SSH participant sessions must verify the expected durable coordinator UUID before sending enrollment/authentication secrets, and derive connection epochs from server replies. Own and kill only the SSH child, drain stderr without persistence, and keep one-time credentials in memory until platform secure storage succeeds. Real-child stdio fixtures do not prove real SSH or platform credential acceptance.

- The native capture harness clears WARP_COLLABORATION_CAPTURE before app window creation so worker children take normal entrypoints. GPU readback configuration must use the retained WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS flag (debug-only), rather than the cleared launcher variable.

- Remote routing storage uses additive SQLite v6 tables rather than extending host workspaces' canonical-root uniqueness. Remote physical identity is enrolled device plus participant checkout UUID; display paths are not host roots. Native run UUIDs map to durable server mutation epochs. Preserve closed-run tombstones and uncertain attempts, and fence remap/departure before replay. This backend foundation does not enable production remote routing.

- Store::transaction owns an explicit BEGIN/COMMIT and is not nestable. Atomic remote admission must call a transaction-internal recovery helper; preserve the public recovery wrapper for ordinary caller-owned operations. Do not split identity/run creation and interrupted-attempt recovery into separate commits.

- Collaboration static gate passed on both native target platforms (source 3d66017, run 36970541930): 74 captures each, all fixture combinations reviewed, detail end reachable and draft/focus preserved. Live integration is now permitted but needs separate native data/action verification.

- Native panel reads use the broker's existing condition variable off the UI thread with 50-item pages, a 200-event view tail and scoped sequence cursors. A one-second timeout refreshes volatile readiness independently of durable history. Scope/pane changes reset selection and cursors; callbacks also check a local generation before applying results.

- Native human task forms retain the original scope, revision/version and request UUID. After submission, changed fields require a new explicit intent; do not retry an old ID with new content or silently substitute a version. Uncertain ownership requires typed ALLOW OVERLAP and still records unknown effects. Async mutation callbacks refresh data without a delayed focus change.

- Shared-pane admission is a reviewed workspace/space/canonical-root tuple passed through NewTerminalOptions, never an environment selector. Revalidate the exact tuple when preparing the new capability; a remap must not silently change the user's reviewed scope. Restored panes remain private. The panel derives pending admission from the prepared binding before native-agent discovery, and keeps revoked scope history distinguishable from participation.

- An overridden outcome does not prove stopped execution. A force-cancel override belongs to that cancellation intent; retry/reassignment must independently authorize the current revision's unknown attempt. Older uncertainty remains recorded after the replacement revision is authorized. Native action acceptance exposed this distinction beyond backend transition coverage.

- Native capture diagnostics copy only failed step names found in the committed capture source, plus exit status and source SHA. Raw stdout/stderr remain runner-local. This makes both-platform GUI failures diagnosable without exporting application payloads or credentials.

- Trusted local operator thread/search reads may inspect the selected domain; native actor reads remain participant-scoped. Keep the operator program reserved at registration, retain original message bodies and append replies rather than editing history. Panel search state stays transient and bounded to existing 50-record pages.

- Local evidence viewing and verification share the producing-checkout lookup. File viewing rechecks current canonical containment and file type, refuses remote descriptors, and never substitutes the active pane's same-named file. Opening current content is distinct from hash verification. Late file-open callbacks require the original selected task/pane and enabled communication.

- Windows PTY startup must simplify canonical extended-prefix paths before both CreateProcessW and bootstrap environment construction. Broker scope identity stays canonical; PowerShell location-provider paths use the existing dunce helper. Native shared-pane acceptance exposed this boundary.

- History eligibility must query all persisted attempts rather than the 16-attempt task-detail window. Force-cancel/overlap overrides retain unknown effects and cannot authorize archival or purge. Preview/deletion share predicates; referenced message roots survive acknowledged-message cleanup.

- Native history purge pins the original preview sequence and typed DELETE HISTORY intent. Purge selects task/message eligibility before deletions can expose more rows, records an audit event and replays the original result. Reply/thread retention checks use separate indexed lookups rather than one OR join. Native history JSON export is an explicit scoped clipboard page; no automatic filesystem write or eviction.

- Human reservation maintenance pins the original page owner, checkout and expiry. Renewal requires a current authorized owner and confirmed active linked attempt; explicit advisory release retains uncertain attempts and never claims OS execution stopped. Existing agent MCP ownership/run checks stay intact.

- Native task list projection intentionally contains only id/state/revision/version/assignee. Checks requiring description, archive markers or attempts must use the existing selected-task/history-export full records rather than inventing summary fields. The Rust compiler caught this in retained GUI fixtures; local Rust verification remains GitHub-only.

- Remote gateway task dispatch separates connection/correlation epochs from the coordinator-assigned original mutation epoch. Resolve the device-bound actor and current generation/grants before the existing Store request ledger can replay. Native readiness/wait and controller APIs remain excluded until guarded participant presence/wake is connected.
