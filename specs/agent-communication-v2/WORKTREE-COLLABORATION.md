# Cross-worktree collaboration

Date: 2026-10-08. Status: explicit modes and Coordinator orchestration implemented;
three-platform remote protocol verified, desktop and native visual acceptance pending.

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

Worktrees are worker isolation boundaries. Creating a worktree never launches an
Agent. In the panel, the user explicitly binds an eligible active Agent to a
worktree in the same repository and execution environment. A binding must match
the Agent's actual checkout; it cannot relocate a running process or silently
change its captured file authority. If no eligible Agent exists, leave the
worktree unassigned and guide the user to start an Agent there using the existing
workflow. The panel shows the Coordinator and each worker's task, checkout,
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

Explicit checkout admission is an internal communication foundation. Ordinary
Agent launches stay in Project mode until selected through the trusted panel.
Enrollment changes communication authority while preserving the native process,
physical checkout and private history. Exiting an enrolled run leaves its durable
role offline; a replacement run requires explicit selection. Stale run events
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
  Bind each active run to this record using its revocable run identity.
- An operational team has exactly one explicitly selected active Coordinator.
  Without one, disable coordination actions. A worker has exactly one owned
  worktree; two active workers cannot own the same worktree. The Coordinator's integration
  checkout cannot also be assigned to an active worker.
- Panel-selected worker binding records participant/checkout ownership separately
  from worktree creation and Agent launch. Validate actual checkout and active run
  before binding; failure leaves the worktree unassigned and preserves user files.
  Coordinator-authorized task/review assignments reference these explicit bindings.
- Worker restart preserves its role, task and checkout, and revokes the previous
  run. Coordinator exit disables its coordination authority and does not elect
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
- Local activation captures an explicitly joined worktree binding. Never remap
  an already registered run. Panel routing follows the captured run when one is
  present; new panes show the joined team's projection.
- Remote physical project fences remain unchanged. A separate repository-owned
  Broker/Store serves joined worktrees; unjoined roots retain the original
  per-project Store. Reuse the account-owned service, never desktop-path identity.
  Remote launch and operator projection resolve the same admission. Scope-bound
  requests reject forms captured before a team change.
- The implemented foundation uses the existing wrapping text-button row and
  confirmation form, with
  `Join worktree team` / `Leave worktree team`. Show fresh-run guidance and each
  participant's actual checkout. Preserve drafts and fence late responses.

## Orchestration acceptance still pending

- [ ] Top-level modes; Project behavior/history regression and retained drafts.
- [ ] Active-Agent Coordinator selection, empty-project disabled actions and
      explicit reselection after exit; no implicit launch or election.
- [ ] Independent worktree creation and panel-selected worker bindings; reject
      wrong-root, duplicate, stale-run and cross-environment assignments.
- [ ] Coordinator-only task/context/review orchestration; worker A reviews worker
      B; Coordinator chooses integration and performs the final commit.
- [ ] Local and SSH backend enforcement, persistence and native panel acceptance.

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
  states as well as the Coordinator-to-peer-review flow. Visual acceptance is
  pending; documenting this layout does not establish that the UI passes.

## Tasks and verification

- [x] Git identity resolver and opt-in workspace admission/revocation.
- [x] Local run, panel, reservations and producing-root evidence routing.
- [x] Remote shared authority, fresh launches and scope-bound operator requests.
- [x] Existing native panel actions and compatibility documentation.
- [x] Real Git main/linked-worktree IPC tests: messaging, task start/submit/review,
      separate same-path reservations and evidence, unjoined checkout/clone
      isolation, stale scope/run, leave/rejoin, replacement and reopen.
- [x] Companion tests with different physical project fences and shared team,
      private-history retention, independent file roots and service isolation.
- [ ] GitHub pinned protocol suites on all remote platforms; default and
      `warp_platform` desktop checks on macOS/Windows; focused native UI capture.

Rust compilation stays on GitHub per the project's existing preference. Local
checks cover parsing, focused source review and diff hygiene. Native vendor/model
execution is distinct from deterministic native IPC/Companion acceptance.
