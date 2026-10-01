# Agent Collaboration v2 Implementation Progress

## Active continuation — 2026-10-02

The user authorized completing the plan, pushing source and GitHub builds, and
subsequently confirmed that necessary actions require no further permission questions.
This supersedes the earlier no-upload instruction for the readiness repair.

- Committed the inherited readiness fixes, reservation sequence fix and real-MCP
  regression harness as `4d5b859`; preserved all previous worktree changes.
- GitHub run `36901907456` compiled the Windows protocol package and ran 32 unit
  tests: 28 passed, 4 failed. It uncovered unpersisted task assignee/reviewer changes
  and nested SQLite connection borrows in space listing/history export. The new
  cancellation regression also used invalid request IDs. Fixes are being validated
  in the next run. The macOS runner failed executing dependency build scripts;
  native toolchain diagnostics were added, without changing pinned dependencies.
- Cancellation-pending work retains exclusive ownership: another start/claim is
  rejected until stop confirmation; assignment/pool wake notices do not bypass it.
- Second run `36903064933`: all 32 storage/session unit tests passed on both OSes.
  Integration failures exposed obsolete 13-tool assertions (the schema now has 29)
  and obsolete expectations that queries revoke idle readiness. These expectations
  were corrected. Added a real authenticated IPC concurrent-claim test and guards
  preventing old deduplicated start/submit responses from changing current readiness.
- Physical reservations now resolve symlink aliases and nearest existing parents,
  reject workspace escapes and Windows device/path aliases, and require the current
  authenticated run when reserving/renewing attempt-owned paths. Existing mock-only
  reservation tests now use a real disposable checkout.
- Added a native, explicitly gated static Tools Panel preview and fixtures for all
  B02/B04/B06 states. See `UI-CHECKPOINT.md`. Screenshot acceptance and live wiring
  remain pending; sample data is never represented as live agent activity.
- The ignored native two-turn harness now uses a disposable Git repository and a
  runtime bridge-path override. GitHub uploads the acceptance executable so it can
  run locally without Rust compilation. It remains a native lifecycle probe, not
  real task-edit/rework or actual app-UI acceptance.

The original handover below describes its historical checkpoint. M2 live operation,
M4 functional shared-space routing, M6 remote coordination and M7 acceptance are not
complete. No plan checkbox has been promoted solely from source inspection.

### Source `b58db4c` and runtime checkpoint

- GitHub run `36903982569` passed protocol, migration, real-MCP process, readiness
  and setup suites on both macOS and Windows, then entered application checks.
- Downloaded the matching macOS bridge and readiness acceptance executable;
  no local Rust compilation. Native lifecycle probes did not pass: Codex 0.159.2
  shared mode timed out before frontend connection/registration; QoderCN 1.1.65
  registered but did not restore readiness after the first submitted question
  within 60 seconds. These are failures, not execution or UI acceptance. A
  no-daemon Codex probe option was added for subsequent isolation checks.
- Downloaded the successful `e08b2ed` review app from run `36877709716` into
  `/tmp/warp-v2-baseline-e08b2ed/review`, verified its bundle display name and
  deep/strict code signature. This is the separate baseline app, not a replacement
  of `/Applications/Warpai.app` or a build of current source.
- Storage continuation replaces lifetime task/message ceilings with an active
  queue limit and per-recipient backpressure, preserves reserved cancellation
  notification/request capacity, and permits purge at the database budget.
  Reusable free SQLite pages are excluded from occupied-page quota accounting.
  Added a focused archival/full-inbox stop regression; CI validation pending.

### Source `882bd2e` and next validation

- Run `36905359670`: both OS protocol suites passed; Windows default/platform
  application checks passed; macOS default/platform checks passed and focused
  application tests are running. Packaging is not yet verified.
- The matching GitHub-built Codex 0.159.2 embedded-mode lifecycle probe also
  failed to register within 60 seconds. Its cleanup stalled; only its own child
  session was terminated. The harness now uses nonblocking PTY I/O, process-group
  cleanup and a bounded parent cleanup wait, and matches safe diagnostic markers
  across output chunks without retaining terminal transcripts.
- Added run-identity fencing to cancellation confirmation and a regression for
  a replacement run. Archived tasks are omitted from routine peer summaries,
  with direct/history reads retained. These changes await GitHub tests.
- Local IPC declares major 2, registration includes explicit local features,
  and the MCP bridge checks protocol/features before tool discovery. This does
  not advertise shared spaces or remote support. Legacy IPC fails closed with
  an actionable matching-companion error.
- Added GitHub packaging of a debug native review bundle with its own identifier
  so visual acceptance can use the existing debug-only data-profile isolation.
  UI screenshot acceptance remains pending.

Date: 2026-10-01. Written by the implementation session that landed commit `2a9da86`.
This is a status and handover record, not an acceptance claim. [PLAN.md](PLAN.md)
checkboxes change only when their exit gates actually pass.

## Landed

### M3, M4, M5 backend and readiness gates — commit `2a9da86`

`feat: add agent bus task control, reservations, and readiness gates`, pushed to
`warp-lite/agent-communication`. 13 files, +6031/-1040.

- `crates/agent_bus/src/lib.rs`: new operations `TaskCreatePool`, `TaskClaim`,
  `TaskProgress`, `TaskCancel`, `TaskFinishCancel`, `TaskFail`, `TaskRetry`,
  `TaskReassign`, `TaskSetDependencies`, `EvidenceAdd`, `FileReserve`,
  `FileRenew`, `FileRelease`; `TaskAssign`/`TaskList`/`TaskSubmit` extended with
  dependencies, deadlines, timeouts, `expected_version`, `attempt_id` and
  `evidence_ids`; controller operations for spaces, workspaces, devices,
  archive, history export/purge and force cancel; `Reservation`/`Evidence`
  contract structs.
- `crates/agent_bus/src/storage.rs`: dependency edges with cycle/self/cross-project
  rejection and accepted-prerequisite gating; pool queues with explicit eligible
  agents and atomic single-winner claim; start/execution/review deadlines with
  sweep; exclusive expiring file reservations with renewal, release and an
  abandoned-owner warning; operator (local UI) read and mutation paths that span
  the whole project; thread/reply/search retrieval; attempt-scoped evidence with
  operator verification; cursor history export, purge preview and purge;
  v2→v3 additive schema upgrade with a pre-upgrade backup copy and a sentinel
  that makes older binaries reject an upgraded writable database.
- `crates/agent_bus/src/readiness.rs` (new): `Activity` enum and a bounded
  memory-only `Draft` tracker; paste sequences are appended, backspace truncates
  by grapheme, Enter proves submission only when the known draft is nonempty,
  control characters invalidate rather than guess.
- `crates/agent_bus/src/transport.rs`: per-run `native_activity`, draft and
  blocked tracking; `Broker::activity`/`input_bytes`/`input_guard`.
- `crates/agent_bus/src/session.rs`: turn/pending-turn tracking and
  `incoming_status()`; MCP tool count assertion 13 → 29.
- App wiring: approval-blocked CLI status reports `WaitingApproval`
  (`app/src/agent_communication.rs`), user input feeds `input_bytes`,
  `peer_input_guard` added in `app/src/terminal/view/use_agent_footer/mod.rs`.
- Tests: ten new protocol tests in `crates/agent_bus/src/storage.rs`
  (`dependencies_gate_starts_until_prerequisites_accept`,
  `dependency_cycles_and_foreign_edges_are_refused`,
  `pool_claims_admit_one_winner_and_only_eligible_agents`,
  `deadlines_expire_and_timeouts_request_stop`,
  `reservations_are_exclusive_expiring_and_releasable`,
  `evidence_is_attempt_scoped_and_operator_verified`,
  `threads_are_participant_scoped_and_search_is_literal`,
  `history_export_is_ordered_and_purge_preserves_unread_work`,
  `spaces_workspaces_and_devices_are_operator_managed`,
  `v2_databases_upgrade_additively_with_backup`) plus `coordination.rs` updates.
- `specs/agent-communication/READINESS.md` (new): acceptance criteria for
  separating lifecycle readiness from drafts.

## CI status for `2a9da86` (run 36893865896): red

Both jobs failed at compile time with one error and one warning:

- `error[E0609]: no field created_seq on type &Reservation` at
  `crates/agent_bus/src/storage.rs:3670`.
- `warning: unused import: MAX_SUBJECT` at `crates/agent_bus/src/storage.rs:9`.

The protocol tests have never executed on this commit. A fix for the error is in
the concurrent working tree described below; `MAX_SUBJECT` should also be removed
from the storage.rs import list when that fix is committed.

## Concurrent working-tree changes (do not revert)

While this record was written, a second active session held uncommitted edits in
five files. They are legitimate in-progress work and must be reviewed and
committed by their author, not reverted:

- `crates/agent_bus/src/lib.rs`, `storage.rs`: add and populate
  `Reservation.created_seq` (this is the E0609 fix).
- `crates/agent_bus/src/transport.rs`: readiness refinement — `ready` means no
  task is executing; drafts no longer revoke readiness (typing still cancels a
  delayed Enter); new `Live` fields `paused`, `last_input`, `readiness_source`;
  task lifecycle drives activity (`TaskStart`/`TaskClaim` → Working,
  `TaskSubmit`/`TaskFinishCancel`/`TaskFail` → Idle with a final-turn reminder);
  `AgentReady` no longer forces Working; `agent_list` exposes
  `native_activity`, `readiness_source`, `paused`, `can_start_task`; wake
  eligibility gains `paused`/`expired`/epoch/`last_input`/`native_activity`
  guards.
- `crates/agent_bus/src/readiness.rs`: ESC no longer counts as cancel (Ctrl-C
  only).
- `specs/agent-communication/READINESS.md`: acceptance criteria rewritten to
  match the above.

## Verification status

- Project rule: no local Rust compilation; GitHub CI is the only compile/test
  authority. Local pre-commit checks that passed: `git diff --check` clean,
  test expectations audited line-by-line against each handler's branches and
  error codes (sweep, FileRenew ordering, TaskClaim checks, retry, purge
  preview, history export, v2 upgrade).
- M1 remains the only milestone with a green CI acceptance (run 36877709716,
  commit `e08b2ed`).

## Remaining work

- Land the concurrent fix, then re-run the workflow; this will be the first
  execution of the ten new protocol tests.
- M0.2–M0.5: credential-free acceptance harness and real-device fixture runs.
  Requires devices and installed/authenticated CLIs; not completable from source
  alone.
- M2: native collaboration panel. Not implemented. Design notes below.
- M4 UI: space join/leave and reservation conflict visibility in the panel
  (the MCP-side text and controller operations exist).
- M6: SSH remote coordination. Not started.
- M7: cross-platform release acceptance and documentation. Not started.

## M2 implementation notes (explored, not implemented)

Backend read surface is ready:
`Broker::operator_agents/operator_tasks/operator_task/operator_events`
(`crates/agent_bus/src/transport.rs` around line 939), mutations via
`Broker::operator(project, &Operation)` under the operator principal
(`warp:{project}` / name `operator`), controller operations via
`Broker::control(project, &ControllerOperation)`. The panel's project key comes
from `project_root(cwd)` of the pane's working directory, as existing wiring
already does in `app/src/agent_communication.rs`.

Tools Panel integration points:

- `ToolPanelView` enum: `app/src/workspace/view/left_panel.rs:99`.
- `LeftPanelDisplayedTab` (persisted) and the `From<ToolPanelView>` mapping:
  `app/src/app_state.rs:316`.
- Toolbelt button construction: `left_panel.rs` around line 377, plus
  `LeftPanelAction`, `MouseStateHandles` and `update_button_active_states`.
- View construction: `LeftPanelView::new` call site in
  `app/src/workspace/view.rs:2893`; available-view list in
  `compute_left_panel_views` (`view.rs:21684`).
- Events upward: `LeftPanelEvent` → `handle_left_panel_event`
  (`view.rs:6155`). Pane focus from a panel selection can dispatch
  `WorkspaceAction::FocusTerminalViewInWorkspace` (handled at `view.rs:23128`
  with local focus at `view.rs:5653`). Reverse bus-terminal → warpui view
  lookup: add a helper over `agent_communication::VIEWS`.
- Async pattern to reuse (must not block the UI thread): background
  `std::thread::spawn` + `mpsc::Receiver` + `ctx.spawn(Timer::after(250ms))`
  polling, as in `app/src/agent_communication.rs:279-401`.
- Components: `Appearance::ui_builder()` paragraph/button/checkbox/switch
  (`app/src/settings_view/agent_communication.rs`); `EditorView::single_line`
  plus `TextInput` for any text entry
  (`app/src/settings_view/privacy/add_regex_modal.rs`). Do not autofocus panel
  widgets; opening or refreshing the panel must not steal terminal focus.
- Required states per PRODUCT.md B02/B04/B06: agent rows (name, CLI, device,
  workspace, readiness, last activity), task filters and detail (attempts,
  dependencies, results, review feedback, event timeline, `wait_reason`), human
  operations (assign, cancel, retry, reassign, review, archive) attributed to
  the operator with confirmation for destructive or uncertain-execution
  overrides, and empty/loading/disconnected/stale/capacity-limited/failed
  states with a next action.
- Fixed fixtures for all B02/B04/B06 states can be built from captured
  projections; screenshot review (narrow/wide, light/dark, text size) remains
  real-device acceptance under C05.
