# Cross-worktree collaboration

Date: 2026-10-09. Status: behavior correction implemented; local/SSH backend
verified, final desktop regression and native visual acceptance pending.
App 1.3.1 / Companion 3.1.0 is the earlier released baseline, not this correction.
Formal tagged-package receipts are tracked in [the release contract](../RELEASE.md).

## 2026-10-09 correction — native worktrees and project lifetime

This revision supersedes the explicit worker binding and one-Agent-per-checkout
rules below. Reuse Warp's existing worktree modal, tab configurations and new-tab
flow; collaboration does not own a second worktree creation form. Git common-directory identity defines the parent project, including checkout subdirectories.
Any active Agent in any checkout can become Coordinator. Switching the selection
at any time demotes the previous Coordinator to an ordinary participant without
ending its run or losing its tasks. Agents automatically participate in all
registered checkouts of that repository before Coordinator selection.
Agents in the same checkout may communicate; across checkouts only messages to
or from the selected Coordinator are allowed. Separate clones and SSH authorities
remain isolated. Preserve private Project history and reject enrollment when
unfinished private work cannot safely change scope.

Native launches establish repository project membership before registration and
task creation. Mode changes and Coordinator changes preserve Agent IDs, native
runs, checkout bindings and task ownership. Legacy private history remains in its
original authority; migration is not the normal launch or mode-switch path.
Persist the active communication mode per project separately from Coordinator
selection: Project permits project peers; Worktree permits same-checkout peers
and Coordinator links. Reading the panel never changes the active mode. Exiting
the selected run clears its role so a same-name restart is an ordinary participant.
Coordinator changes notify the new selection about unfinished project tasks;
task provenance and designated reviewer remain intact. Project participant lists
include every online project Agent, regardless of the focused checkout. Remote
Coordinator candidates must have pagination or complete bounded native coverage.

Coordinator selection belongs to the project and exact native run, independent
of focused pane, tab, directory or panel visibility. Clear active selection only
on run termination/revocation or application shutdown; never elect a replacement.
Project participant lists contain online Agents, with pagination applied after
filtering. Offline history remains available through tasks/history.

Visual source: existing native panel components, Appearance theme and the fixed
Worktree layout fixtures. Use prominent equal-width Project/Worktree controls,
short action labels, a compact Coordinator row and checkout Agent summaries.
Remove nonessential option buttons; use one consistent row/section spacing.
Keep full paths and task details behind expansion. Static narrow/light/dark
fixture screenshots precede runtime capture acceptance.

Acceptance tasks:
- [ ] Native worktree action creates/opens a new tab without starting an Agent.
- [x] Automatic repository participation includes multiple Agents per checkout.
- [x] Same-checkout and Coordinator messages pass; cross-worker messages fail.
- [x] Focus/project/subdirectory changes retain Coordinator; process exit clears it.
- [x] Online filtering precedes pagination and preserves offline task/history data.
- [x] Schema upgrade preserves roles and produces a pre-upgrade backup.
- [x] Initial repository identity and task ownership survive both mode changes.
- [x] Non-Git directories remain independent Projects without inherited Worktree mode or Coordinator; returning to the repository retains its selection.
- [x] Same-name Coordinator restart remains a worker until explicit selection.
- [x] New Coordinator receives unfinished-task handoff and future task updates.
- [ ] Collaboration entry remains prominent after toolbelt state refresh.
- [ ] Focused protocol tests, desktop checks and native narrow/light/dark captures.

Backend receipts: macOS/Windows protocol suites in run 37921826849 and three-OS
remote run 37920112411. Desktop-only follow-ups do not change backend source.
Native creation, toolbar appearance and final source acceptance remain open.

Risks: preserve immutable checkout authority and private work during admission;
repository identity must never be inferred from a URL/name. Remote native
worktree creation must retain the SSH terminal's existing command/tab path rather
than invoking local filesystem creation.

## Product and acceptance

The collaboration panel has a top-level `Project` / `Worktree` switch. `Project`
is the default and preserves the previous release's project communication,
participants, assignments and history. `Worktree` provides parallel development
within one Git repository, locally on macOS/Windows or on one SSH host/account on
Linux/macOS/Windows. Changing the visible mode never silently migrates existing
runs or their private history.

After switching to Worktree mode, the user selects a Coordinator from the current
project's active Agents in the panel. With no active Agent, the selector is empty
and coordination/assignment actions are disabled; show guidance to start an Agent
through the existing Agent workflow. Selecting a Coordinator does not launch or
restart a process. Persist the explicit selection; never infer it from tab focus,
display name, launch order or an Agent's self-declaration.

Worktrees are communication isolation boundaries. Creating a worktree through
Warp's existing modal opens a new terminal tab and never launches an Agent.
Active Agents automatically participate using their actual checkout; participation
cannot relocate a process or change its captured file authority. Multiple Agents
may share one checkout. The Coordinator can be selected from any checkout and
changed while both the previous and new Coordinator processes keep running. The panel shows the Coordinator and each worker's task, checkout,
branch and state; unassigned worktrees remain visible.

The Coordinator assigns tasks, passes context, collects results, asks one worker
to review another worker's result, and decides which reviewed results enter the
designated main/integration branch. Warpai exposes and enforces this authority;
it does not make these decisions on behalf of the Coordinator.

Workers implement and test their assignments in their bound checkouts, then
submit results and producing-branch commit/test evidence. The Coordinator collects and
reviews results, integrates accepted changes into the designated checkout, runs
combined checks, and creates the final integration commit. Task acceptance alone
never merges branches. Publishing, pushing and creating PRs require separate
authorization; branch commits used to hand off worker results are allowed.

Checkout admission is internal infrastructure. Selecting a Coordinator enables
repository participation for native Agents. Enrollment preserves each process,
physical checkout and prior Project history. Exiting a Coordinator run clears
its active authority; a replacement run never elects itself. Worker participation
is automatic after restart while the repository team remains active. Stale run events
cannot revoke a newer run or restore an old Coordinator. Leaving fences shared
access, including queued wake delivery, without claiming native processes stopped.

Git's actual common directory identifies a repository. Matching remote URLs,
repository names or branches never join separate clones. Only checkouts with the
same canonical, native common-directory identity can share the team. Worktrees
remain independent file/editor/Review roots. Local and SSH authorities, different
remote services/accounts and unrelated repositories remain isolated.

Reuse existing worktree creation/tab configuration, native Agent launch, Git
commands, tasks and reviews. No separate worktree manager, automatic cleanup,
Git writes over the file protocol or cross-host federation.

## Roles and ownership contract

- Persist the team, stable participant identity, explicit `coordinator` / `worker`
  role, native checkout identity and task ownership in the existing authority.
  Bind each active run to this record using its revocable run identity. Switching
  Coordinator demotes the previous selection to worker within one transaction.
- An operational team has exactly one explicitly selected active Coordinator.
  Without one, disable coordination actions. Each Agent retains its native
  checkout; several Agents may occupy it, including the Coordinator and workers.
- Automatic participation records native checkout/run identity separately from
  worktree creation and Agent launch. Validate both before admission; failure
  preserves the native process, private work and user files.
  Coordinator-authorized task/review assignments reference these explicit bindings.
- Worker restart preserves its task and checkout records and revokes the previous
  run; participation is automatic while the team is active. Coordinator exit disables its coordination authority and does not elect
  a replacement. Explicit restart/reselection validates the active run and fences
  the old authority before granting it to the selected Coordinator.
- Enforce team-management and assignment authority in Warpai's backend APIs,
  rather than relying on prompts. Vendor CLIs retain their OS-account access;
  role checks do not imply a filesystem or Git sandbox.
- Returning to Project mode changes the panel view, not the lifetime or ownership
  of a running team. Stop/restart actions remain explicit; preserve unfinished
  worktrees and uncommitted changes.

## Technical contract

- Add private controller `worktree_join` / `worktree_leave` operations with a
  physical checkout root and UUID request ID. These are never Agent MCP tools.
- Resolve and validate Git worktree/common directories through the existing
  bounded, no-hook, private-Git command runner. Use native directory identity,
  including birth time, to distinguish replacement directories.
- Reuse spaces, workspace bindings and producing-root evidence. Store worktree
  admission fingerprints separately from ordinary workspace metadata. Every
  authorization revalidates the admitted checkout/repository identity. Explicit
  project-space remapping replaces worktree admission and revokes its captured
  actors while preserving ordinary workspace access.
- Ordinary local and remote Agent launches retain their Project scope until a
  Coordinator enables the repository team. Admit active runs automatically while
  preserving process, native input state, checkout and prior private history.
- Remote physical project fences remain unchanged. A repository-owned
  Broker/Store serves explicitly enrolled runs; original private endpoints forward
  native lifecycle and communication without restarting a process. Scope-bound
  requests reject forms captured before a team change.
- Project and Worktree projections are independent. An unconfigured Worktree
  view shows eligible active Agents and registered checkouts, never private tasks.
  Non-Git projects show an empty Worktree view with guidance to switch to Project.
- Reuse native wrapping buttons, explicit selection forms and vertical scrolling.
  Worker details expand on demand. Internal Join/Leave operations remain private
  infrastructure; the ordinary panel selects only the Coordinator. Worktree
  creation dispatches Warp's existing modal/tab action.

## App 1.3.1 baseline orchestration acceptance

- [x] Top-level modes; Project behavior/history regression and retained drafts.
- [x] Active-Agent Coordinator selection, empty-project disabled actions and
      explicit reselection after exit; no implicit launch or election.
- [x] Independent worktree creation and panel-selected worker bindings; reject
      wrong-root, duplicate, stale-run and cross-environment assignments.
- [x] Coordinator-only task/context/review orchestration; worker A reviews worker
      B; Coordinator chooses integration and performs the final commit.
- [x] Local and SSH backend enforcement, persistence and native panel acceptance.

## Panel layout and visual acceptance

- Keep the header limited to the Project / Worktree switch. In Worktree mode,
  place the Coordinator selector in its own row below the header; stack controls
  when the panel is narrow instead of imposing a wider application window.
- Use a compact worker list showing Agent, branch and state. Reveal task,
  checkout, context and review details on selection or expansion. Keep secondary
  actions in existing menus/forms instead of adding every action to each row.
- Use existing native components, button themes, typography and spacing. Keep
  visual hierarchy clear with consistent alignment and space between sections;
  avoid dense tables or smaller fonts to fit additional controls.
- Constrain content to the panel bounds and scroll long lists vertically. Wrap
  descriptive text; shorten long paths/branches in summaries with an accessible
  way to inspect their full values. Forms and menus must fit the GUI viewport;
  primary actions must remain reachable at supported small window sizes.
- Before integration, review a fixed-data layout using existing components.
  Native macOS/Windows screenshot acceptance covers narrow/default panel widths,
  light/dark themes, no active Agents, long names/paths, many workers, expanded
  details and open selectors/forms. Check clipping, overlap, focus and disabled
  states. Narrow light/dark layouts and actual Coordinator selection, creation
  without Agent launch, worker selection and binding frames were reviewed on both
  desktop platforms. Coordinator-to-peer-review authority is exercised by the
  deterministic backend fixtures.

## Tasks and verification

- [x] Git identity resolver and opt-in workspace admission/revocation.
- [x] Local run, panel, reservations and producing-root evidence routing.
- [x] Remote shared authority, explicit active-run enrollment and scope-bound
      operator requests.
- [x] Existing native panel actions and compatibility documentation.
- [x] Real Git main/linked-worktree IPC tests: messaging, task start/submit/review,
      separate same-path reservations and evidence, unjoined checkout/clone
      isolation, stale scope/run, leave/rejoin, replacement and reopen.
- [x] Companion tests with different physical project fences and shared team,
      private-history retention, independent file roots and service isolation.
- [x] GitHub pinned protocol suites on all remote platforms; default and
      `warp_platform` desktop checks on macOS/Windows; focused native UI capture.

Rust compilation stays on GitHub per the project's existing preference. Local
checks cover parsing, focused source review and diff hygiene. Native vendor/model
execution is distinct from deterministic native IPC/Companion acceptance.

## Acceptance receipts

- Application and Companion source: `06f0ab155871019f08801885e37830a3eb4128d2`.
  [Three-platform Companion run 37830659603](https://github.com/OthinusG/warpai/actions/runs/37830659603)
  passes protocol, persistence, repository/run authority, owned SSH/SFTP and
  private-runtime/packaging checks on Linux, macOS and Windows.
- [Desktop run 37830653334](https://github.com/OthinusG/warpai/actions/runs/37830653334)
  passes macOS default/platform checks, focused application regressions and all
  191 native screenshots. Windows protocol, default/platform checks and focused
  application regressions pass, but runtime preparation fails when overwriting
  an identical DLL held by a previous test process; the overall run is red.
- Workflow-only fix `48f1ca4a` reuses DLLs only when their SHA-256 matches tracked
  source. [Focused Windows run 37841105097](https://github.com/OthinusG/warpai/actions/runs/37841105097)
  passes compilation, owned SSH/SFTP and all 191 native screenshot assertions.
  Application and Companion code are unchanged from the full regression source.
  Both downloaded capture artifacts report exit code 0 and no failed steps,
  assertions, Worktree reads or private Git errors.
- Native captures exercise local Worktree panel orchestration and existing SSH
  panel/file workflows. SSH Worktree orchestration is verified by backend
  fixtures; no separate SSH Worktree GUI capture or paid vendor/model execution
  is claimed. Fixtures use real Git, IPC and owned processes rather than model
  decisions. Final integration uses the Coordinator's ordinary Git CLI.

## Focused cloud debugging

Use `native_windows_only=true` on `validate-agent-communication.yml` to debug
Windows native failures without repeating macOS, protocol/history suites or the
default/platform check variants. It still builds the native application and
Companion/bridge, runs the owned SSH/SFTP fixture and all native capture assertions.
Restore the pinned-toolchain/lockfile Rust outputs before compilation and save
them before capture so a failing UI assertion does not discard the build cache.
The first cache fill still requires compilation; later source changes rebuild
through Cargo. Run the complete desktop/remote gates once after the fix.
