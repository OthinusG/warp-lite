# Current direction: SSH Remote project management

Date: 2026-10-03. Documentation scope correction completed; remote implementation
and CUTOVER source deletion remain pending. Current PLAN uses R0–R7 and current
acceptance uses V01–V24. Earlier M0–M7/C01–C14 entries below are chronology only.

Confirmed by the user: SFTP over SSH only; remote Linux/macOS/Windows; local
macOS/Windows GUI. Agent processes, commands, tests and file IO execute remotely.
One local GUI manages project files/transfers/terminals/Agents/tasks. Full remote
Warpai GUI, enrolled devices, invitation/grant federation and FTP/FTPS are excluded.

Documentation changes: rewritten PLAN/PRODUCT/TECH/API, detailed CUTOVER D01–D16,
new pending acceptance matrix, retired remote gateway/routing pointers and archived
previous architecture. AGENTS/README/MEMORY align with the confirmed scope.
Existing local engine/UI/security work stays reusable; old device source is not
yet deleted and no existing CI result proves the new product complete.

The inherited remote_server client/transport/file-tree/SFTP upload is a reuse
candidate. Its Oz/cloud installer, Unix detection and compatible server source
must be audited before deployment. No remote environment was changed by writing
these specifications.

Documentation verification: all 24 inspected Markdown files have valid local
links; SR01–SR40, R0–R7 (61 tasks), D01–D16 and V01–V24 are unique/complete.
Tracked changes are Markdown only and git diff --check passes. Rust compilation,
remote runtime tests and code removal were not executed for this documentation task.

GUI ownership revision (2026-10-03), approved by the user: existing Tab/Pane
sidebar owns Agent/session status and lifecycle actions. R6 now reuses the
collaboration panel for compact on-demand tasks/messages/history, preserving saved
selection and keeping fresh profiles closed. CUTOVER D14 removes duplicate roster
and session controls after sidebar parity while retaining data APIs/attribution.
V14/V21/V22 require exact task/session links, detached rows, shared projections
and background updates that preserve focus with the task view closed. No new
dashboard or mandatory task board is planned; source changes remain pending.

## Historical progress follows

# Agent Collaboration v2 Implementation Progress

## Active continuation — 2026-10-02

### Recovered checkpoint for the current continuation

- Source `bd593d9`, run `36973482638`: both complete backend suites passed,
  including live-panel pagination/cursor/scope/presence checks. macOS debug UI
  compiled and static capture passed; Windows debug compilation is still running.
- Added native operator forms for assignment, cancellation, review, retry,
  reassignment, explicit uncertain-execution override and archival. Forms retain
  original version/request UUID and reject modified replay. Submission never
  schedules a later focus change; the result callback only refreshes data.
  Added deterministic native live/action capture extension (96 images), including
  typed-override rejection, unknown-effect cancellation and guarded retry. Native
  build/action/image acceptance and the rest of M2/M3/M4/M5 integration are pending.


- Live panel read integration added: ordinary Tools Panel entry, bounded background
  event wait, explicit agent/task pages, task selection, original attempts/evidence,
  readiness/draft/approval/offline metadata and a 200-event view tail. Context and
  generation guards reject stale callbacks; disablement clears the projection.
  Added real IPC checks for pagination, resumption, scope reset and independent
  execution certainty. Rust compilation and live native capture remain pending;
  human controls are not yet wired.


- Source `f6200db`, run `36971937538`: both complete protocol suites passed,
  including device/checkout identity, original-run resume and replacement fences,
  remap/departure revocation, atomic interrupted-attempt recovery and v5 migration.
  Remote production dispatch and UI remain pending.
- Source `3d66017`, run `36970541930`: both native capture jobs passed with
  74 images and keyboard/focus/draft checks each. Complete Windows fixture and
  full-resolution detail-end/restoration review passed; prior complete macOS
  review plus detail-end evidence completes M2.1/M2.2. Live integration begins.


- Source `f16496e`, run `36971306130`: v6 source compiled on both OSes,
  v5 backup/host-lease/coordinator identity migration passed, and existing backend
  checks passed. Registration tests exposed nested BEGIN from recover inside the
  admission transaction. Reuse a transaction-internal recovery helper so identity,
  membership, epoch and interrupted attempts remain atomic. Rerun pending.

- Added SQLite v6 device-qualified workspace/actor/run storage with a v5 backup
  and downgrade sentinel, immutable participant checkout identities, original-run
  resume, replacement/leave/remap fences and existing interruption recovery.
  Host filesystem roots remain separate; remote relative leases use opaque
  device/checkout keys. Added cross-device/name/run/migration checks; CI pending.
  Remote operation frames, participant lease mirroring, evidence/device metadata,
  repository warnings and production UI are not enabled by this subset.

- Source `25c80ed`, run `36961960929`: both OS gateway/backend checks passed;
  macOS native capture passed. Windows no longer panicked in wgpu, but its readback
  callback produced no PNGs. The capture harness deliberately clears its launcher
  environment variable before window creation, so the COPY_SRC guard still read
  false. Reuse the retained real-display integration flag instead; no new capture
  configuration or production renderer setting is needed. Native rerun pending.
- Full source `6a53a71`, run `36959134811`, passed both OS protocol suites,
  default/platform app checks and release review packaging. Newer gateway/client
  and renderer changes still need their exact-source full application validation.

- Source `07b12ef`, run `36962746574`: both OS full backend suites passed,
  including the owned-child/stdio participant session fixture: expected
  coordinator authority, authentication, explicit-space heartbeat and rejection
  of ungranted use. System SSH option probes remain offline; real SSH, secure
  participant persistence and remote native operation routing are still pending.
- Hardened peer enrollment contexts and error handling: bound credentials/grants,
  reject nil identities and retain only known error codes with static messages.
  Reflected peer messages never reach application errors. CI rerun pending.

- Source `e54dc28`, run `36962285181`: both OS complete backend suites passed,
  including actual controller/gateway enrollment/authentication/heartbeat/live
  revocation/restart identity, stale ownership, bounded/private discovery,
  malformed/oversize frame rejection and clean unavailable CLI stdio.
- Added an owned system-SSH client session with expected-authority validation,
  enrollment/authentication and grant-checked heartbeat. Added a credential-free
  real-child/stdio fixture check for authority and authentication; CI pending.
  This fixture is not real SSH authentication or platform credential persistence.

- Source `47284a9`, run `36961667419`: Windows full backend validation passed,
  including the new real controller/gateway enrollment/authentication/heartbeat/
  live revocation and restart identity check. macOS is rerunning the socket-name
  correction. New source `25c80ed`, run `36961960929`, also tests clean CLI failure
  framing, bounded/private discovery and stale controller ownership; pending.

- Source `47284a9`, run `36961667419`: macOS gateway source compiled and
  existing 48 tests passed; the new IPC check exposed a controller socket name
  exceeding macOS SUN_LEN inside the existing private runtime directory. Shorten
  only the random basename and remove owned socket files on controller drop.
- Source `1267827` macOS detail-end PNG reviewed: final reservation warning,
  human controls and uncertainty guidance are visible with the draft intact.
  This completes macOS detail scrolling evidence; Windows remains pending.

- Windows capture root cause: surface COPY_SRC was enabled only by the removed
  integration-test build feature, whereas the debug checkpoint uses the existing
  readback path. Enable it for the explicit debug capture environment, and reject
  unsupported texture usage/format before encoding. Native capture rerun pending.

- Source `fc603e4`, run `36959995622`: macOS and Windows protocol validation
  passed, including the Windows kernel DACL/current-user connection check.
- Source `1267827`, capture run `36958662888`: macOS native capture passed;
  Windows failed in wgpu initialization (`wgpu_core.rs:2653:18`) after runtime
  DLL staging. Windows static acceptance remains pending; no live panel is enabled.
- Added an explicitly started enrollment controller, bounded private discovery,
  fixed remote-stdio relay and a real IPC enrollment/authentication/heartbeat/
  revocation/disable/restart check. This is an enrollment-only implementation;
  remote actor/task dispatch and production UI remain pending. Rust CI pending.

- Windows pipe source `78f5cce`, run `36959439468`: macOS backend passed; Windows
  compiled and passed existing unit checks, but the ACL regression assumed SDDL
  always renders a numeric SID. Well-known accounts use aliases. Inspect and
  compare the actual kernel ACE SID instead, without logging account identifiers;
  the corrected OS-boundary check is pending.


- Added a current-user-only protected Windows pipe DACL for every native listener
  instance, rejecting remote clients and inherited handles. Reused the locked
  Windows bindings and Tokio constructor. A native kernel DACL/current-user
  connection regression is pending CI. This boundary is required before a
  separate controller endpoint carries enrollment responses.


- Source `1267827`, run `36958662888`: both OS complete protocol, migration,
  history and enrollment suites passed. Real authenticated shared IPC exercised
  delegation/start/file evidence in its producing checkout/submit/rework/accept,
  active private attempt preservation, cross-scope reservation privacy, directory
  drift denial, offline reclaim denial, leave/replay fencing and irreversible
  old-capability remap revocation. Both debug UI builds/captures are still running;
  backend success does not accept the shared-pane GUI or SSH gateway.


- Source `e1b2f4d`, run `36958248410`: both OS passed complete storage, migration
  and enrollment unit tests. Shared IPC reached AgentList correctly after the
  borrow fix; the new fixture incorrectly counted the caller as its own peer.
  Native AgentList intentionally excludes the caller. Fix the expected counts,
  preserving that behavior; delegation/evidence steps still need the rerun.


- Shared-space source `efaeb47`, run `36957643422`: both OS compiled all new
  code and passed storage/migration tests, but real IPC AgentList exposed a nested
  RefCell borrow in the new membership filter. Materialize rows and release the
  SQLite borrow before authorization queries; this fixes the common list path.
  Rerun pending. Windows debug capture has not yet reached its runtime step.
- Implemented enrollment persistence primitives: OS-random 256-bit invitations
  and distinct credentials, five-minute/single-use grants, hash-only storage,
  constant-time verifier comparison with pinned already-locked subtle, current
  grant-generation checks and operator revocation/listing. Invitation dedup stores
  only a safe receipt and never redisplays its secret. Added expiry/grant/replay/
  redaction/revocation tests. Gateway, secure participant storage and SSH enrollment
  remain separate unimplemented integration gates; Rust CI pending.


- Implemented local shared routing backend with fresh app-selected terminal
  admission, schema-v5 immutable workspace bindings, durable revocation, scoped
  task/message/event grouping, physical reservation conflict privacy and original
  evidence checkout resolution. Added real authenticated IPC delegation/rework,
  private-work preservation, directory/reclaim/leave/remap tests and v4 backup
  migration regression. Local Rust parsing and SQLite admission/revocation checks
  passed; both-OS Rust CI is pending. Shared-pane UI remains gated.
- Source `668c604`, run `36947705932`: complete macOS/Windows checks and packaging
  passed. Source `8c33356`, run `36950239278`: macOS complete checks, packaging and
  74 native images passed with draft/navigation/scroll assertions. Windows checks
  and packaging passed, but native capture exited 101 before any image; it is not
  an accepted screenshot gate. Failure-location diagnostics are being added.


- Added opt-in eight-hour deterministic backend soak orchestration: two
  sequential four-hour phases per OS, matching source/OS/duration, compilation
  excluded, metadata-only atomic reports and failure/timeout handling. Local
  stdlib self-check (mocked child/clock, no Rust) passed continuity, short-run
  labeling and failure cases; dry-run passed. Actual soak remains pending and
  does not stand in for a native UI/draft or real-model run.
- Source `8a5c9e1`, run `36951776747`: both OS complete protocol/migration/history
  suites passed, including fixed hello bytes, feature intersection and truncation.

- SSH startup/negotiation source `dd4425f`, run `36951069744`: both OS complete
  protocol/migration/history suites passed, including real system `ssh -G` and
  hello/partial-frame checks. Windows first failed the probe while its disposable
  configuration handle remained open; closing it before OpenSSH reads fixed the
  actual Windows path. This does not validate real authentication or enrollment.
  Added fixed hello wire bytes, unknown-feature intersection and truncated frame
  checks in `d6381b6`; these incremental checks await CI.

- M6.4 negotiation subset: strict typed hello/result frames, incompatible-major
  and missing-required-feature rejection, bounded unique capability names and
  frame limit, fresh per-connection epoch. Reused the existing length-prefixed
  Tokio codec; added partial-I/O, malformed/unknown/oversized/deadline/redaction
  tests. Negotiation alone grants no device or operation authority. CI pending;
  authentication, spool/resume and gateway wiring remain unimplemented.

- M6.1 startup spike: native system OpenSSH path, bounded host alias, fixed
  remote gateway command, strict known-host checks and noninteractive pipes;
  disable forwarding, local commands and shared control sockets. Local macOS
  `ssh -G` against an empty test config accepted all options without networking.
  Added both-OS system option/injection/path regressions; Rust CI pending. This
  does not expose a gateway or advertise remote task support.
- Corrected clock-jump regression passed both OS full protocol/migration/history
  suites: source `61a44cd`, run `36950035983`. Native interruption/UI gates remain.

- Added explicit-clock sweep regression for forward/backward UTC jumps: no
  resurrection, duplicate deadline events, reassignment, lost attempts or replay
  start. Production still supplies current UTC; no system clock is modified.
  Initial run `36949685579` caught a fixture unit error (database UTC is
  milliseconds, not seconds). Jumps now derive milliseconds from Duration;
  both-platform GitHub verification is pending.

- Native static panel keyboard/focus revision: pinned preview controls, native
  screen-reader state/help, Left/Right/Enter navigation, Page Up/Down scrolling
  and Escape return through existing Tools Panel focus actions. Ordinary opening
  still retains terminal focus. The isolated GPU harness seeds an unsent draft
  and checks focus, state changes and draft preservation. GitHub validation and
  native runtime acceptance are pending; live wiring remains gated.

- Source `c7033aa`, run `36939414552`: both OS full validation and packaging
  passed. Source `8ea58e2`, run `36942065295`: both OS full validation/packaging
  also passed with the repaired native capture driver. Signed debug artifact ran
  locally twice, each producing 72 PNGs and exit 0. Detail images show successful
  narrow/wide wrapping; complete screenshot/keyboard/scroll/Windows acceptance is
  not claimed. Added an explicit `capture_ui` workflow input to collect native
  images on both CI operating systems after their review packages build.

- Implemented the backend cross-checkout reservation warning subset of M4.3:
  explicit repository UUIDs, current shared membership, immutable lease-sharing
  snapshots, bounded merge-overlap metadata, unchanged physical rejection and
  lease expiry. SQLite v4 preserves a v3 backup and legacy private leases. Added
  privacy/join/leave/remap/shared/expiry/capacity and migration regressions.
  GitHub verification pending. This does not implement shared task routing or UI.
  The actual SQL schema/query also passed a local SQLite check for private-row
  exclusion, membership revocation and additive migration; Rust stays CI-only.
  Source `bfbc585`, run `36941810908`: both OS complete protocol/migration and
  representative-history suites passed, including the new warning/backup tests.
- Added authenticated IPC dependency acceptance/start regression covering both
  deterministic orders and simultaneous requests. Run `36947096207` caught an
  incorrect test assumption: accepting a prerequisite deliberately increments
  the dependent's version when changing blocked → queued. The corrected check
  requires old intents to fail, then refreshes the version/new request ID to grant
  one attempt; replay cannot create another. GitHub verification pending.

- Added an opt-in debug native static capture harness using the retained warpui
  driver and GPU frames, with 72 fixture/width/theme/zoom combinations, a unique
  profile/output directory and a 300-second startup-inclusive watchdog. No
  system accessibility permission, removed integration crate or HOME override
  is needed. Missing PNGs fail the harness. Compilation/render review is pending;
  live controller wiring remains gated by UI-CHECKPOINT.
  Source `e515973`, run `36940946118` caught ambiguous closure inference caused
  by unnecessary `Box` wrappers around three generic assertion callbacks. Removed
  the wrappers to use the driver's direct closure API; render is still pending.

- Latest inherited source: `fb298cb`; working tree contained only an untracked
  local `.codegraph/` index. Existing engineering progress is recorded below.
- Run `36915402177` failed on both platforms in
  `dormant_agents_wake_without_an_open_wait_call`: the fixture started a task,
  then expected model readiness to override its unfinished execution. The
  production guard correctly refused. The fixture now asserts refusal, submits
  the task, then checks draft protection and subsequent wake. Both OS Rust
  tests/checks run on GitHub; no local Rust compilation.
- Continue with M2 static visual acceptance before live integration, functional
  M4 shared-space routing, M6 remote coordination and the M0/M7 native acceptance
  gates. No missing real-device or real-model gate is marked passed.
- `bde9043`, run `36934356183`: both OS protocol suites passed the repaired
  dormant-wake fixture, page-full/COMMIT rollback and revision-deadline tests.
  Full application and package validation was still running at this checkpoint.
- Added both-order start/cancel, submit/cancel and review/retry control checks,
  including stale replay, and expanded authenticated concurrent claims to 20
  participants. These additions await the next GitHub run.
- Downloaded and verified the separate debug UI review app from source `75f47f9`.
  Launching with an isolated data profile exited 101: `app_menus.rs:104` asserted
  because ReferAFriend had no registered Lite binding. Gate the menu item with
  the same `warp_platform` boundary as its binding; a menu-construction regression
  and workflow entry accompany the fix. The local accessibility permission is
  absent; no native screenshot/keyboard acceptance is claimed.
- Extended full validation to build the existing Windows installer/portable
  packages without publishing a release, using pinned packaging tools and the
  existing script. Windows builds now require `--locked`, like application checks.
- User supplied no Windows SSH machine or native model-call budget. C01/C13 and
  native Windows acceptance remain unverified. `ACCEPTANCE.md` records every
  PLAN item individually; metadata APIs are not functional shared-space routing.
- Pool wake audit found that broadcasting availability woke every idle candidate,
  even though the atomic claim granted just one owner. The broker now selects one
  eligible candidate by durable notification order, prevents fan-out after a
  submitted notice, and checks stale candidates again under the claim mutex.
  Draft/busy states exclude candidates; failed delivery and a removed run permit
  reevaluation. A focused IPC regression covers those transitions; CI pending.
- Source `17da1f0`, run `36935931793`: both OS protocol checks passed, including
  the 20-way claim, both-order controls and single-receiver pool wake regression;
  application and package checks were still running at this checkpoint.
- A cached submission with the current task version could still reset readiness
  during a later ordinary user turn. The broker now checks committed request
  identity under the same mutex before allowing any lifecycle side effect. A
  regression submits new user input then replays the old unchanged task result.
  Replay returns durable data without rewriting live readiness. CI pending.
- Source `e2c90a1`, run `36936534033`: both OS protocol/migration/scale suites
  and default/platform application checks passed. macOS application test
  compilation failed in the new menu fixture because `App::update` needs a mutable
  `App`; corrected the fixture closure to `mut app`. Windows packaging is running;
  neither the failed app-test stage nor incomplete packaging is counted passed.

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
- Run `36908172790` failed because a newly added integration assertion called a
  crate-private error conversion. It now inspects the public typed error rather
  than widening the library API. No tests on that source are recorded as passed.
- Recovery audit found that a replacement run could start running work without
  proving the old execution stopped. TaskStart now rejects that path with
  `execution_unknown`; disconnect records do not manufacture a finished time.
  Explicit operator fencing records `unknown`/`overridden`, rather than stopped
  execution, before a new revision can be retried. Migration/recovery fixtures
  are updated to verify the stricter boundary.
- Source `75f47f9` passed both OS protocol suites and entered application checks.
  Local verification of evidence now compares SHA-256 content and checks existing
  Git objects with fetching disabled. Opened-file handles are checked against the
  authorized workspace before reading; credential paths and files over 64 MiB
  are rejected. Tests include content mismatch and missing/malformed commits.
  This new verifier awaits CI. It reuses pinned sha2/libc/Windows dependencies.
- A workflow-dispatch `protocol_only` option supports focused cross-platform
  backend iteration without cancelling an unrelated full application build.
  Its success is protocol acceptance only; skipped app checks are not passed.
- Source `3aff31e`, protocol-only run `36910516148`: both OS suites passed,
  including native opened-handle checks, SHA-256 mismatch rejection, local Git
  object verification and explicit unknown-execution recovery. App checks were
  intentionally skipped. Downloaded matching native executables; Codex embedded
  still did not register (60.16 s), QoderCN still did not complete its first turn
  (62.95 s). The bounded cleanup now completes; neither probe is model acceptance.
- Added the explicit C10-scale disk-backed benchmark (100k acknowledged messages,
  10k archived completed tasks), a 150 ms indexed task-page p95 check, search and
  export pagination, and purge preservation of live task/message/dedup records.
  The fixture SQL passed local syntax/count checks; Rust measurements await CI.
- Source `2cccc64`, run `36912281466`: both OS protocol and C10-scale checks
  passed. Indexed task-page p95 was 1.799 ms on macOS and 2.863 ms on Windows,
  measured on GitHub runners, below the 150 ms target. Search/export pagination
  and live-work/dedup preservation through purge passed. Full-disk injection and
  eight-hour soak are separate outstanding gates.
- History audit found that prior result/evidence text disappeared on rework once
  response-cache retention ended. New submission/review events now retain that
  text durably. Task detail bounds recent attempts/feedback and signals truncation;
  a new participant-scoped `warp_task_history` exposes the ordered full event
  record. Inbox/thread/search/event/export pagination shares a serialized-byte
  budget, and evidence descriptors enforce a combined metadata limit. New
  regressions exercise 20 reworks, expired response cache, unauthorized history
  reads and maximum-length escaped messages. CI validation is pending.
- Source `94bc32d`, run `36913294715`: both OS protocol suites and scale benchmark
  passed with the new 30-tool contract, bounded history and durable result events.
- Additional storage fixes preserve version-specific upgrade backups, roll back
  a failed COMMIT before retry, and scope start deadlines to the current revision.
  Added real SQLite page-full and competing-reader commit-failure fixtures. SQL
  fixture checks passed locally; Rust verification of these changes is pending.
- Wake inspection now fetches one eligible pending row instead of loading an
  entire inbox; peer unread counts use the existing index. Explicit model readiness
  cannot override an active delegated attempt, and task-start eligibility accounts
  for work invisible to the requesting issuer. New readiness regression pending.

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


## Native shared admission and scheduling checkpoint (2026-10-02)

- Source `3d66017`, run `36970488111`: both target OS protocol/history, default/platform application checks and release packaging passed.
- Source `15460de`, run `36976343320`: both applications compiled; native capture failed after 94 images. Action acceptance exposed ordinary retry treating a force-cancellation override as authorization for replacement execution. The shared retry/reassignment guard is being corrected and revalidated; no acceptance is claimed for this run.
- Shared scope preview/create/map/leave controls and reviewed new-tab admission are implemented. Source `00c86d0` runs `36977526463`; native shared admission driver source `f822201` runs `36977759626`. Both are validation checkpoints, not completed M4 acceptance. The retained driver now additionally covers pool creation and deadline policy.
- Eight-hour backend soak dispatched on fixed baseline `3d66017`, run `36977886213`. This is an in-progress backend stability check, not real vendor or physical-device acceptance and not yet a passed soak.


## 2026-10-02 build and remote safety checkpoint

- Full source dc46ca6 passed macOS and Windows backend/history, default and
  warp_platform application checks, release packaging and 115 native captures
  in run 36998413298. This is a complete build checkpoint, not whole-plan acceptance.
- Presence, reviewed grants, unknown execution on connection loss, scoped
  snapshots/confirmed cursors, shutdown failure fencing and cursor restart
  passed both OS backend suites in runs 37002464526, 37002776680, 37003138274,
  37003690031, 37004266066 and 37004317107 respectively.
- Sources 5b988d3/cec947a passed both backend suites including explicit original
  owner completion of an unknown attempt. Their native builds failed because
  the capture assertion used App.clipboard instead of the existing ViewContext
  accessor; b1bf828 fixes it. Run 37009716224 rechecks native history export and
  participant metadata on both platforms. No successful new UI acceptance yet.
- Added coordinator-ordered native connection ownership before writes and absent
  receipt reconciliation. The real IPC check replaces a connection without
  changing its original epoch, obtains not_committed, then rejects the delayed
  old write and verifies the receipt is still absent. Exact-source CI pending.
- Baseline 3d66017 completed four-hour soak phase 1 on both platforms; phase 2
  remains running. This baseline does not cover later remote changes.
- Production native remote opt-in, secure credential persistence, participant
  MCP routing, guarded wake and physical checkout conflict guards remain unfinished.
  Physical cross-device and vendor model acceptance remain unavailable.


- c3250f2 / run 37010078182 completed successfully on both OSes, including
  replacement connection ownership and old-frame write denial after absent
  receipt reconciliation. Added a follow-up assertion that old session cleanup
  preserves the replacement connection's fresh presence lease; CI pending.
- b1bf828 native capture rerun 37009716224 is building both review apps;
  86e154c full repository rerun 37010763280 is checking both application variants.
- 54fe996 native enrollment/profiles source is pushed on its isolated validation
  ref; full two-platform build and focused application checks run 37011624211.
  Enrollment UI and production remote participation are not yet connected.


- Native source b1bf828 compiled successfully on macOS, then its first live
  projection assertion failed: the fixture compared checkout metadata against
  PanelQuery.project, an ephemeral worker input not retained in panel state.
  Compare the private fixture against the received snapshot.project instead.
  Existing real-IPC metadata checks already passed; full native rerun pending.
- Native enrollment source 54fe996 exposed a bridge setup-test dependency:
  setup.rs is included independently of UI modules. Move its serialized profile
  type into setup.rs rather than importing the native worker module. Fix committed
  and the same two-platform full validation ref rerun.


- Initial Hello error responses now use the same peer diagnostic sanitizer as
  authenticated exchanges; the owned-child fixture rejects reflected text and
  unknown retry/version hints before enrollment. CLI preference changes wait
  for pending enrollment while disablement remains available and cancels it.
  Exact-source cloud revalidation pending.


- Added durable native delivery observations and selected-task projection,
  distinguishing guarded prompt claimed/submitted/cancelled from receiver
  acknowledgement and execution. The real private-IPC check exercises retries,
  duplicate callbacks, unchanged queued task/version/attempts, unacknowledged
  inbox and restart history without restoring live authority. Cloud verification
  and new native projection screenshot review pending.
- SSH children now reuse the existing launcher binding scrubber and remove Vibe
  MCP configuration. Argument/environment checks ensure native capabilities and
  endpoints stay on the originating machine. Exact-source CI pending.


- Native capture sources 2ebdfdf / run 37012541720 and 497de9e / run
  37013071649 passed both target OS suites, native builds and all 115 assertions.
  Both-platform 2ebdfdf screenshots reviewed for participant metadata, history
  capacity/export and thread readability; drafts remain intact.
- e4bc0e6 / run 37016574533 passed new delivery phase checks but failed a test
  expecting AgentAck to accept a task notification. Preserve the established
  invalid_state boundary and correct the assertion; exact-source rerun pending.
- macOS secure storage must distinguish errSecItemNotFound from access/locked
  errors. The shared provider now preserves other errors, with a focused status
  classification check. Native Keychain/DPAPI runtime acceptance remains pending.


## Product direction correction — 2026-10-02

The user clarified SSH Remote project execution as the intended remote product.
Current M6 machine-to-machine native collaboration is a different scope; its
production UI/routing implementation is suspended pending specification alignment.
Local Warpai should manage remote project environments; agents execute commands
and file operations remotely and communicate within the selected remote scope.
Existing source and validation evidence are retained rather than removed.
Source b529062 is pushed on an isolated ref with full two-platform validation
run 37019498133; that run validates existing changes, not SSH Remote support.


## 2026-10-03 SSH Remote R0 source cutover

Removed native enrollment worker and enrollment-only busy gates; legacy profile
metadata moves to a read-only namespace with UUID-owned deletion-only credential
cleanup and a visible pending state. Production excludes the old device SSH
client/controller/presence route. Retired remote-stdio returns a fixed refusal and
all five device controller operations are blocked before replay. Schema v7 keeps
v6 backup/history/pending intents, revokes old grants/runs and preserves unfinished
execution as unknown. Legacy runtime fixtures remain test-only. Local parsing,
SQL, YAML and whitespace checks passed; GitHub Rust checks remain pending.
R1–R7 and R0's independently source-built server gate are still unimplemented.


R0 follow-up source `a9caea0`, [run 37040051387](https://github.com/OthinusG/warp-lite/actions/runs/37040051387):
macOS and Windows protocol/setup and representative-history checks passed,
including unchanged local active attempts, retained original receipts and legacy
profile selection. Both default application checks failed with E0616 because the settings view
accessed a private legacy profile field; platform and focused native cleanup
checks were skipped. A boolean Preferences accessor fixes the ownership boundary;
exact-source revalidation is pending. Source was verified to
match the workspace's 17 implementation paths without altering the current index.
SSH Remote and SR41 host-status collection/rendering remain unimplemented; their
contracts and static/live acceptance requirements are recorded in HOST-STATUS.md.


R0 repair source `d71d8b1`, [run 37041826672](https://github.com/OthinusG/warp-lite/actions/runs/37041826672):
macOS and Windows protocol/setup, representative history, default application and
warp_platform application checks passed. Both OS legacy metadata/credential
cleanup tests and macOS native configuration, wake and Keychain error-classifier
tests and both review-package builds passed. Real credential-provider and SSH
runtime acceptance remain pending. These results validate the
cutover implementation, not R1–R7 or SR41 runtime behavior. The workspace's 17
implementation paths match this source; its current branch/index was preserved.


## Managed read and SFTP source checkpoint — 2026-10-03

Source `fc8964c`, [run 37090383877](https://github.com/OthinusG/warp-lite/actions/runs/37090383877),
passed shared protobuf, native host metrics/root fencing, standalone process,
SSH argument/shell encoding and SFTP parser checks on Linux/macOS/Windows.
A controlled Linux loopback OpenSSH account exercised actual companion status
and independent file-only SFTP with no companion path. Unicode/spaces/shell
metacharacters preserved exact binary bytes; traversal/symlink escape and reads
after disconnect were refused. All three review companions include source/target
provenance, checksums and inherited notices. This is partial V02/V04/V05/V09
evidence, not completed GUI, installation, transfer or retained-session gates.

Source `6ab9021` adds persistent native account/root identity and a flushed
retired-endpoint refusal after a native-suite failure exposed Tokio stdio's no-op
shutdown. Its exact-source identity/flush checks are pending. New SR41 native
fixtures are implemented, but captures and visual review remain pending. V01–V24
stay open; no paid vendor/physical-host acceptance is inferred.

## Task-by-task acceptance resumed — 2026-10-03

### R0.6 accepted: legacy storage cutover

Rechecked source `d71d8b1` and successful GitHub run
[37041826672](https://github.com/OthinusG/warp-lite/actions/runs/37041826672).
The current `storage.rs` is byte-identical to that accepted source. Both desktop
OS suites verified v6 backup, retained original intents/receipts/history, revoked
legacy device authority, unknown unfinished remote execution and unchanged local
active attempts. PLAN R0.6 is checked; the remaining R0 gates stay open.

### Retained terminal implementation and pending acceptance

The account service now owns bounded native PTYs independently of clean SSH
attachments. Immutable launch UUIDs, exact run/attachment fences, sequential
input, explicit attach/detach/Stop/release, output replay and active-run idle
fencing are implemented. HostClient exposes the same scoped operations; a
controlled Linux SSH test checks actual remote cwd/input/detach/reattach/Stop.

Source `18dfb73`, run `37094247053`, passed Linux/macOS; Windows exposed missing
ConPTY standard-handle initialization. Source `10cdf1d`, run `37094569241`, passed
Linux/macOS including actual SSH; Windows passed native IO but retained release
waited indefinitely for a ConPTY output EOF while its owner retained HPCON.
Source `d1022f8` closes the console only after observed exit. Three-platform
run `37094854747` and full desktop/native-capture run `37094856739` are pending.
No R2/R4/GUI/vendor gate is checked by this implementation checkpoint.

### Retained terminal backend accepted — source 064f5cb

[Run 37104521972](https://github.com/OthinusG/warp-lite/actions/runs/37104521972)
passed Linux/macOS/Windows wire, native PTY, real companion disconnect/reconnect,
immutable launch replay, replaced input lease, sequential input, bounded replay,
project isolation and owned Stop/observed-exit/release checks. Controlled Linux
OpenSSH exercised a real remote shell cwd, Unicode input, reconnect to the same
boot/run and explicit Stop; actual binary SFTP streams also passed. All three
review artifacts include the negotiated retained_terminal capability.

This accepts the read/session service and terminal identity primitives, not
terminal GUI, Agent MCP/task authority or complete V13–V15. A follow-up retains
Unix leader IDs until owner release so background children remain controllable;
that new process-group check needs exact-source validation. Desktop run
37094856739 passed Windows application checks but native capture exposed a
circular view update; macOS stopped at a crates.io DNS download failure.
Full new-source desktop/capture run 37104524458 is pending.


### Retained ownership and native view follow-up — source 28a97cb

Keep the Unix leader unreaped with `waitid(WNOWAIT)` until owner release, fencing
process-group ID reuse. Stop and release include still-owned background children.
Windows retains ConPTY until its owned job has no active processes; stop can
terminate that job after the original leader exits. A real child fixture verifies
leader exit, retained background output and eventual stop/EOF on each platform.

Downloaded Windows `d1022f8` capture evidence contains 147 PNGs and ends at the
reviewed shared-workspace image. The next confirm opens a new tab synchronously
while CollaborationPanel is removed from the view map; workspace visibility
updates re-enter it. Reuse `dispatch_typed_action_deferred` for panel-to-workspace
open/file/focus actions. Existing native shared-tab assertions cover this path.
Focused run 37105299664 and full desktop/capture run 37105299449 are pending.
No additional PLAN item is accepted before their checks complete.


### Background-child corrections — source 3a37ed2

Run 37105299664 passed Linux, while macOS exposed Darwin's EPERM for a group
containing only an unreaped zombie and Windows required another exit observation
to close ConPTY after job termination. Full run 37105299449 stopped at the same
native tests before application validation. No failed gate is marked complete.

Confirm the Darwin group contains only the exited leader through bounded native
`proc_listpids` before interpreting EPERM as already ended; other permission
failures remain errors. Drop reaps an observed leader even if no group signal
recipient remains. The background child now reports readiness before its parent
exits, ignores Unix hangup itself, and the check inspects the actual child's
process status before/after Stop rather than inferring death from a PTY pipe.
It also observes Windows owned-job exit until console EOF. Exact-source run
[37105738779](https://github.com/OthinusG/warp-lite/actions/runs/37105738779) is pending.
Apple's [group signal implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c)
filters zombies; its [native process listing](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/proc_info.c)
includes current and zombie processes. These native semantics are not replaced
with an unconditional permission-error fallback.


### R0.3 accepted: identity contract and path/migration inventory

API sections 2/3 and executable shared `ManagedFence` define separate account,
service, boot, clean connection and admitted native project identities; the
terminal contract separately identifies session, immutable run and attachment
generation. IMPLEMENTATION's call-site table distinguishes local PathBuf/file
access from inherited remote StandardizedPath/client routing. Legacy v6 device
rows/receipts remain read-only rather than becoming new SSH authority.
Source064f5cb/run37104521972 passed three-platform identity/native-root/fence and
replaced terminal attachment checks. This accepts R0.3's definition/inventory
work, not future profile/environment GUI deduplication, writable remote task
storage or cross-host grouping. PLAN R0.3 is checked independently of those gates.


### R4.3 accepted: retained run and attachment ownership

Source `3a37ed2904b849c8ca93479a5d9ecb2b925b789b`,
[run 37105738779](https://github.com/OthinusG/warp-lite/actions/runs/37105738779),
passed all three remote platforms and controlled Linux SSH/SFTP. Independent
session/run/connection/generation IDs gate actual queued input, resize, attach,
detach, stop and release. Replacement rejects former input/resize/stop; changed
launch retries conflict and exact launch reconciles the same run. Reader/writer
threads retain the owned native PTY across clean channel loss. Background-child
checks now confirm an actual surviving child after leader exit, then its stopped
OS status and closed output. Original Unix IDs remain unreaped until release;
Windows closes only the exhausted owned job's console. PLAN R4.3 is checked.
This does not accept terminal GUI, profile-removal review or local draft replay.


### Remote authoritative task foundation — source 4420e78 (pending)

Add typed `project_tasks` protobuf fields/capability and reuse existing human
PanelQuery/Operation/ControllerOperation serialization inside that explicit field.
Private per-project SQLite/Broker owners live on the companion host; native
project identity, persisted root binding and one account-service registry prevent
path collisions or duplicate GUI writers. Task/version/dependency/review/history
logic stays in Store. Deny GUI Agent registration/readiness/start/submit/file
capabilities, cross-root space mapping and all retired device operations before
opening storage; require expected versions for versioned GUI mutations.

Client checks exact fence/generation, opaque panel project and bounded responses;
remote error prose is replaced, unknown codes refused and retry/version hints
sanitized. Unit checks use actual private IPC registration, create/replay/change
an original pool intent, isolate another/replaced root, reopen history and deny
forbidden authority without creating files. Controlled Linux SSH now reads both
attachments' task projections and rejects GUI registration. New backend CI is
pending; no R2.7/R2.6/live-UI task is accepted from source inspection.

The remaining panel-control and Connections exit focus dispatches now also use
the shared deferred helper, covering all discovered sibling workspace actions.
Desktop/capture run37106025476 validates source3a37ed2 independently of this new
wire/store source; it cannot establish compilation of source4420e78.


### Remote project task backend accepted — source 550753b

Source `550753b33a090b0d0531395f9ced8c8beeebb2fb`,
[run 37107398041](https://github.com/OthinusG/warp-lite/actions/runs/37107398041),
passed all three remote platform suites and controlled Linux SSH/SFTP. Private
project Store reopening retains tasks/events; same-project connections reconcile
the original immutable intent, changed content conflicts, other/replaced project
identities have independent databases. GUI lifecycle/retired-device requests and
stale/native-replaced project fences are refused. Remote errors retain safe
code/version facts without peer prose or arbitrary retry hints. Controlled SSH
checks both attachments' task projections and refuses GUI Agent registration.

Source4420e78/run37107128686 failed only the new IPC assertion's return type;
the helper already unwraps successful JSON. The corrected source verifies the
registered actor name, and every new task check now passes. This accepts the
remote authoritative Store/API foundation; full R2.7 still requires local
projection/pending GUI-intent integration. R2.6 per-launch MCP and R5/R6 live
remote Agent/sidebar/task flows remain pending, as does native UI validation of
this exact source. No complete V08/V17 gate or real vendor compliance is claimed.


### Owned process activity follow-up (pending validation)

Task source550753b passed all focused gates. Further lifecycle review found that
Darwin PTY output may close while an ignored-hangup background child remains
alive: leader exit plus output EOF must not authorize Release or 60-second idle
owner exit. NativePty now provides group/job activity, using bounded native group
membership plus installed sysinfo status on Unix and existing owned-job accounting
on Windows. TerminalState exposes optional processes_active; Release and idle
use the same actual observation and retain ownership on observation failure.
Extend the existing real background-child check with activity before/after stop,
and shared-wire checks with impossible inactive/unobserved-exit rejection.
These changes still need exact-source remote verification; no R2.8 gate is closed.

### R0.4 accepted: native enrollment worker removal

Rechecked the current worker/settings/configuration boundary against source
`d71d8b1` and [run 37041826672](https://github.com/OthinusG/warp-lite/actions/runs/37041826672).
agent_communication.rs, settings_view/agent_communication.rs, setup.rs and
legacy_remote_credentials.rs are byte-identical to that accepted source.
The enrollment module, receiver/cancel/status fields, poll and enrollment-only
configuration gates are absent. Both default/platform application builds and
local configuration/wake plus legacy metadata/credential cleanup checks passed.
Schema-v6 compatibility/unknown-work fixtures preceded removal. Shared secure
storage hardening and local operator/setup behavior remain. PLAN R0.4 is checked;
SSH async cancellation/live connection integration and remaining cutover gates
are separate pending tasks.

### Owned process activity and concurrent attachment acceptance — source 65e1b86

[Run 37110078158](https://github.com/OthinusG/warp-lite/actions/runs/37110078158)
passed Linux/macOS/Windows protocol/native/service tests and controlled Linux
SSH/SFTP. Source `65e1b8671ec860007a5c76d868d442c46c76cec4` proves leader exit and
PTY EOF cannot authorize release while owned background processes survive;
disconnect/reattach preserves that run, explicit Stop observes inactive ownership
and Release follows EOF. Failed/unreadable native activity retains ownership.
Concurrent attachment identity access waits within a five-second bound; eight
simultaneous opens share one persistent identity. The three-platform concurrent
retained fixtures exercise the actual service startup path.

Accepted as the lifecycle/backend follow-up only. R2.8 attempt recovery, R2.6
per-launch MCP, sidebar/live GUI and full fault acceptance remain pending. Later
MCP edits are not covered by this source gate.

### Per-launch remote MCP implementation (pending validation)

TerminalLaunch optionally selects a non-operator Agent program. The account
service creates fresh private IPC credentials and pins native root/run ownership,
then injects only that binding into the native child's environment. Ordinary
terminals receive no Agent binding. The actual retained run UUID is the Broker
run, and private credential verification now uses the installed constant-time
comparison. Root replacement/directory escape is rejected before operation or
receipt replay. Observed full group/job exit and owner removal revoke the binding;
idle scans observe every owner, even while another process remains active.

The companion's explicit mcp subcommand reuses Bridge and the pinned SDK.
An actual retained native child exercises initialize/tools discovery and reports
its nonsecret run identity; private IPC regression covers original receipts,
root replacement and fresh credentials after revocation. Exact-source CI is
pending. No R2.6, vendor adapter, automatic wake or live GUI gate is accepted yet.

### R2.5 accepted: single account/project service authority

Source `65e1b8671ec860007a5c76d868d442c46c76cec4`,
[run 37110078158](https://github.com/OthinusG/warp-lite/actions/runs/37110078158),
passed all three remote native targets and controlled Linux OpenSSH. Concurrent
SSH attachments share service/boot/native account/project/root identities and
project task projections while receiving distinct connection UUIDs. A native
exclusive service.lock permits one account-service owner; at most 32 admitted
project Brokers own private Stores and reject root-domain replacement. Native
IPC uses private Unix permissions/current-user Windows pipe ACLs, no remote GUI,
public listener or installer. Disconnect/reattach and stale/cross-project access
checks verify service/input ownership. PLAN R2.5 is checked. Private per-launch
MCP changes, deployment GUI and physical SSH-host matrix remain separate gates.

## User scope correction — 2026-10-03

The user clarified that delivery is the working same-project Agent communication
extended to SSH, preferably with a concise status panel. The previous full SSH
project manager was broader than this goal. PLAN/PRODUCT/TECH now define S1–S5;
previous documents are archived without discarding history or unrelated changes.
File-manager/transfer/automatic-install/session-dashboard work is no longer a
prerequisite. Source 9a8e692, run 37110965089, passed private run MCP and actual
native SDK discovery on all three remote targets. S1/S2 foundations are accepted;
S3 two-Agent launch workflow, S4 existing-panel integration and S5 focused delivery
remain pending. Source 65e1b86 native captures also passed both desktop OSes in
run 37110117064; visual/integration acceptance remains scoped and independent.

### User-mandated implementation pruning (pending validation)

The user explicitly requires no redundant code/features, beyond shrinking the
checklist. Removed new agent_bus SFTP module and streamed-transfer tests, standalone
Connections preview/profile storage/fixtures/toolbelt entry, and host CPU/memory/
disk/uptime collection, wire types, validators and sample states. Capture/workflow
expectations now follow the retained collaboration fixtures only. Existing
upstream file/terminal code is preserved. Session management simplification and
exact-source regressions remain pending; S0 is not accepted yet.

### S0 cleanup follow-up — pending exact-source verification

Removed session listing, detach/reattach and generation takeover from the service,
SSH client and protobuf schema; removed tags/actions are reserved. Runs now belong
to their launch connection. Disconnect revokes MCP, requests owned group/job stop
and retains native ownership until activity is observed ended; closed disconnected
owners are collected. Replaced retained-session acceptance with connection-owned
Agent IO/private-MCP/isolation checks. README follows the corrected scope.

Cleanup source 468b490 run 37113128671 failed at compilation because service idle
logic inherited Instant through the removed metrics import. Fixed with an explicit
service import. No cleanup acceptance is claimed before a successful rerun.

S0 also removes the superseded device enrollment/authentication, federation client,
remote actor registration, remote gateway and device pending-intent runtime modules
(about 5,000 lines). Keep schema migration, read-only provenance and stored original
intents; retired controller operations return feature_unavailable. Added a raw-v6
migration check proving uncertain attempts, receipts, pending intent and backup
survive without the deleted runtime. Native disconnect acceptance now observes the
actual child heartbeat stopping, in addition to refusal of another connection.
These changes are pending exact-source CI; S0 remains unchecked.

Source ce1f686 run 37121782406 passed macOS cleanup/local regression and the Linux
companion/Agent checks. Linux broader Store regression exposed the desktop-only
opened-evidence handle inspection; added the native /proc/self/fd equivalent and
Unix no-follow/nonblocking open. Windows and exact-source rerun remain pending.
S3 source now adds an explicit companion Agent CLI using the existing account
client, PTY IO and session::launch adapters. Private credentials remain child-only;
MCP relay uses the existing forward implementation. S3 is not accepted yet.
