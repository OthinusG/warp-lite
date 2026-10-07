# Existing File Tools over SSH

Date: 2026-10-07. Status: implemented and accepted; packaging for the 1.2.0 release.

## Outcome and scope

After the user connects with ordinary `ssh user@host` and runs `cd`, the existing
Project Explorer follows the confirmed remote directory. Its existing file
management, editor, preview and Git Code Review features operate on that remote
workspace through the same controls used for local work.

This extends the existing components. It does not introduce another Explorer,
preview renderer, Review panel, connection form or transfer dashboard. The
existing panel layout, shortcuts, menus, editor tabs and preview preferences are
the interaction source. No new visual design is required for this extension.

This document implements the approved narrow extension to the accepted SSH scope in
[PLAN.md](PLAN.md). The previously removed standalone file manager remains
removed; archived remote-manager designs are not implementation instructions.
Existing Agent task acceptance/rework already works remotely and is separate
from the Git Code Review panel addressed here.

## Required behavior

1. **Terminal-driven selection.** Confirmed SSH session metadata selects the
   host, account and current directory. `cd` and active-terminal changes update
   Explorer and Review through the current workspace events. SSH command entry
   alone is not proof of a connected session or a remote working directory.
2. **Directory parity.** Explorer displays the selected directory, including
   ordinary directories outside Git repositories. Inside a repository, preserve
   the existing local root-selection behavior. Leaving a repository clears its
   Review selection without making file browsing unavailable.
3. **Existing interactions.** Folder expansion, file selection, path copying,
   open in pane/tab, create, rename, delete and terminal directory navigation use
   their current controls. Operations execute on the selected remote account.
   Local OS actions such as Reveal in Finder/Explorer are not offered for a
   remote path. Remote files must never be passed to a local external editor or
   system application as though they were local files.
4. **Existing viewing and editing.** Text/code opens in the current editor;
   Markdown and other currently supported in-app preview formats use the current
   viewers. This release does not promise new preview formats. Markdown-relative
   resources resolve against the remote document directory, not the desktop cwd.
   File classification uses remote metadata/content, never a local path probe.
5. **Remote saves.** Save writes the original remote file and reports success
   only after remote completion is confirmed. Changes observed since opening
   trigger the current conflict/reload treatment. Failed or disconnected saves
   retain the buffer and its unsaved state. Preserve encoding/newlines and
   applicable file permissions according to existing editor behavior.
6. **Stable open tabs.** An opened file retains its host/account/path identity
   after `cd` or terminal switching. New selections follow the new cwd; existing
   tabs cannot be rebound to an identically named file elsewhere.
7. **Existing Git Review.** Repository detection, changed-file lists, statistics,
   existing comparison modes, diff rendering, file navigation and review comments
   use the remote repository. Renames, additions, deletions, untracked files,
   binary changes, conflicts, unborn HEAD and worktrees receive the corresponding
   current treatment. A missing Git executable and a non-repository directory
   have distinct actionable states.
8. **Review actions keep their target.** Opening/editing a reviewed file uses
   the same remote file identity as Explorer. Review feedback reaches the
   intended remote terminal/Agent using the existing submission flow. Inventory
   all existing Review actions before implementation; file/Git mutations must
   use an explicit remote backend or remain visibly unavailable. No action may
   accidentally execute against a desktop repository. Existing confirmations
   and Agent permission behavior continue to apply.
9. **Loading and disconnection.** Use existing loading/error states and concise
   location/status text. On disconnect, stop new operations and label retained
   data stale. Cached previews remain viewable; saves and other mutations are
   disabled. Reconnection revalidates account/root identity before enabling
   operations. Late responses from another cwd, session or connection are ignored.
10. **Independence and platforms.** File tools do not depend on enabling Agent
    communication or on the collaboration panel being open. The existing remote
    Companion installation may be reused. Desktop targets remain macOS/Windows;
    remote targets are Linux/macOS/Windows. Plain SSH remains usable when helper,
    SFTP, permissions or cwd metadata are unavailable; show the actual missing
    prerequisite in the existing tool surface.

## Baseline reuse map

This table records the pre-extension gaps that motivated the implementation.
Runtime acceptance is recorded separately in `COMPANION-CHECKS.md`.

| Existing component | Reuse | Original gap |
| --- | --- | --- |
| `app/src/workspace/active_session.rs:53` and `app/src/agent_communication/panel.rs:360` | Confirmed remote cwd, SSH transport arguments, account home/OS, Companion discovery | SSH selection is private to the collaboration panel; file tools need the same selection without its visibility/settings lifetime |
| `crates/repo_metadata/src/remote_model.rs:106` and `app/src/code/file_tree/view.rs:915` | Remote repository identifiers, snapshots, incremental updates and the existing tree renderer | Populate these models from the current Companion connection; support a selected non-Git directory |
| `app/src/workspace/view.rs:14868` | Existing `RemoteRepoNavigated` to `set_remote_root_directories` path | Current Companion navigation does not supply this entire file-tool path |
| `app/src/code/file_tree/view.rs:2300`, `:2348`, `:3110` | Current open/menu/editing actions | Explicit remote guards suppress opening, creating, renaming, deleting and directory navigation |
| `app/src/code/editor_management.rs:105` | Current editor creation and source handling | File-tree/link sources carry only `PathBuf`; opening must retain remote identity |
| `crates/warp_files/src/lib.rs:349`, `:717` | Remote `FileId`, central load/save events and existing remote save branch | Existing remote backend resolves only `RemoteServerManager`; add current SSH backend routing and version tracking |
| `app/src/code/local_code_editor.rs:1673` and `app/src/code/view.rs:853` | Existing disconnected editor/save handling | Connect those states to the current SSH attachment |
| `app/src/workspace/view/right_panel.rs:785`, `app/src/code_review/diff_state.rs:410` | Current Review view, diff data/parsers, comments and comparisons | Remote unavailable state, local repository discovery, local Git subprocesses and local path keys |
| `crates/agent_bus/src/companion.rs:91`, `crates/agent_bus/src/ssh_remote.rs:204` | Project/account identity, root-handle validation, bounded requests and response fencing | Managed capabilities currently cover project/Agent/tasks, not file-tree operations or Git Review |
| `app/src/remote_server/ssh_transport.rs:75` and `app/src/remote_server/mod.rs:25` | Reference lifecycle/client/model patterns | Upstream automatic installation is disabled and the old daemon path is not the portable Companion; simply enabling it is insufficient |

The existing remote-file models reduce the work, but removing UI guards alone
would route remote paths into local filesystem/Git code. Backend routing must be
available and verified before those guards are replaced with capability checks.

## Implementation approach

### 1. Share confirmed SSH selection

Extract the existing `selected_ssh` resolution and `SshConnection` transport
description into a small shared module under the existing remote integration.
Use the original terminal session and native cwd events rather than parsing
rendered terminal output or introducing a connection profile database.

Keep connection identity, verified account/root identity and selection generation
with each operation. Add only the session-scoped attachment ownership needed by
existing consumers. A file tab pins its original attachment; directory changes
create/select another project attachment rather than calling `ProjectOpen` on a
client still used by another file or Agent. Share resolution and lifecycle, not
the collaboration panel's refresh state. Stop background work when tools/files
no longer need it.

Explorer and Review continue to follow the most recently focused terminal's
confirmed working directory, including after SSH and `cd`. Opening an editor or
Markdown pane does not select another project. Exclude SSH cache paths from the
shared workspace working-directory aggregation so they never become local roots.
The native capture must perform a user-origin edit, wait for dirty state, save,
and verify the original remote file rather than use streaming system edits.

Review feedback from file-only tabs resolves the original confirmed SSH terminal
among open workspace tabs. Match the exact transport/session and require its
current cwd to remain inside the reviewed root. A closed, pending or retargeted
terminal fails visibly; never choose a local terminal or another SSH session.

### 2. Populate the existing tree model

Add bounded managed Companion operations for directory snapshots, lazy child
loading and metadata refresh. Reuse `RepoMetadataUpdate`, `FileTreeEntry` and
their existing conversion code to feed `RemoteRepoMetadataModel`. Map verified
remote identity to the current `HostId`/`RemoteRepositoryIdentifier` conventions;
include account/session scope so identical paths on different hosts/accounts do
not collide. Support a directory snapshot independently of Git detection.

Reuse `set_remote_root_directories` and the existing navigation notification
path. Replace the `has_remote_server` assumption with availability of the
appropriate existing or Companion file backend. Start with refresh after cd,
explicit refresh and successful operations plus a bounded refresh while visible;
reuse incremental-update handling when the Companion can supply changes. Do not
require a new watcher service or recursive full-project synchronization.

Use the admitted canonical cwd when expanding the tree inside its repository
root, including when the shell's cwd is a directory alias or symbolic link.

### 3. Route the existing file/editor/preview pipeline

Carry a remote location through the current open events and `CodeSource` rather
than converting it with `to_local_path_lossy`. Extend the current `FileModel`
backend dispatch for the Companion SSH attachment; keep its events and editor
integration. Retain the existing upstream remote backend for its current callers.

File-content transfer must use **SFTP over SSH**, as required by `AGENTS.md`.
Reuse system OpenSSH authentication and the selected transport. Existing SFTP
upload helpers are transport references, not a directory-listing implementation:
do not parse human-readable `ls` output or interpolate filenames into a shell.
Use bounded Companion metadata for tree/stat operations and system SFTP for
selected file bytes. Reuse installed tooling first; add a library only if a
concrete cross-platform transport gap cannot be covered reliably.

Build SFTP options from the confirmed SSH route (port, config/jump routing,
identity and the admitted master where present). SSH and SFTP option syntax is
not interchangeable. A closed master must not fall back to a fresh destination.
For native Windows without a master, preserve the existing verified native SSH
contract and validate that the transfer belongs to the same account/root before
using or committing it. Reject opaque/nested routes instead of guessing.

Use the current preview renderer. For viewers requiring a local asset path,
download only the requested file/resources to a private bounded cache, tagged
with their remote source. This cache is a rendering input, not the editor's save
target. Keep document-relative links remote-aware and clean temporary data when
no longer needed. Extend unsupported remote opening routes only as far as the
current in-app viewer/editor can handle them.

### 4. Route existing file-management actions

Add typed project-scoped create/rename/delete operations and staged-save
prepare/commit operations to the managed protocol. Upload content through SFTP
to the prepared temporary location, then validate the attachment and observed
file version immediately before committing the replacement. Use atomic replace
where supported; preserve permissions and keep the original on failed transfer.
Delete only explicitly selected entries using the current confirmations.

Share path validation for all operations. Reject traversal, accidental local
absolute paths, replaced roots, directory cycles, unsupported special files and
symlink/reparse-point escapes. Preserve remote native path semantics, including
Windows drives/case rules. Use handle-based checks where required to avoid
validation/open races. Listing may show links; following or modifying them must
remain inside the admitted root. Root changes and cross-filesystem renames fail
clearly rather than falling back to copying/deleting.

Remote files can also change through terminal commands or Agents. Track observed
content versions and expose reload/conflict behavior; do not claim transactional
exclusion against unrelated external writers. If a mutation's response is lost,
retain its original intent and reconcile remote state before offering retry.
Never automatically repeat a possibly completed rename/delete/save.

### 5. Feed the current Git Review model

Preserve the existing entrypoints and presentation: an Explorer file click opens
the existing CodeView/document viewer, and an existing Git Review action opens
the existing RightPanelView/CodeReviewView. SSH changes backend selection only;
do not add an SSH-only Review presentation or open Review automatically on cd.

Extend the current diff-state loading boundary to select local or remote Git
execution. Remote commands run in the Companion's admitted repository with typed
allowlisted operations and native argument arrays. Reuse current status/diff
parsers and `GitDiffData`/base-content conversion; do not implement a second diff
renderer or enable the legacy unrestricted `RunCommand` handler.

Fetch repository metadata/changed-file lists first, then bounded patches and
needed base/current content. Return remote repository identity, comparison base,
HEAD and query generation so changes cannot mix across refreshes. Working-tree
diffs are live observations: detect intervening changes and refresh rather than
claiming a frozen repository snapshot. Missing branches/history get the current
error treatment; network fetches and Git mutations require their existing explicit
user action. Do not silently fetch merely to populate a remote Review panel.

Include remote identity in existing view/model cache keys. Route reviewed-file
opening and saves through the same `FileModel` as Explorer. Route review feedback
to its original remote terminal/Agent, with the same readiness and permission
checks. Audit current discard/restore/stash and Git-dialog callers: operations
kept on the remote surface need corresponding typed backend handling and the
existing confirmations; desktop-only integrations remain explicitly unavailable.
No local Git invocation may receive a remote `PathBuf`.

### 6. Protocol and lifecycle contract

Add optional managed `project_files` and `project_git_review` capabilities and
new unused protobuf field numbers. Keep reserved fields reserved. An older
Companion still provides terminal/Agent communication; file tools explain the
missing capability and reuse the existing installation guidance.

Each request/response carries the existing `ManagedFence`, selection generation
and relevant operation identity. Reuse `check_project` before filesystem/Git
access. Bound metadata, file sizes, patches, subprocess time and cache usage;
large/binary files get the current supported preview or clear unavailable state.
Use structured errors for permission denied, missing path/Git/SFTP, conflict,
unsupported file and lost connection. Do not add credentials, telemetry, cloud
account dependencies or another persistent service.

## Execution sequence and acceptance

| Phase | Work | Gate before continuing |
| --- | --- | --- |
| F0: trace and reproduce | On an isolated SSH fixture, record ordinary ssh/cd behavior; inventory existing file/preview/Review actions and every local-path caller. Update the active API contract with only required operations. | Evidence distinguishes connection/backend gaps from UI guards; reuse map is complete; no panel redesign |
| F1: selection and tree | Share SSH selection; add Companion metadata capability; populate existing remote model and follow cwd for Git/non-Git directories. | `ssh → cd A → cd B → exit` follows the correct tree; late A responses cannot replace B; two sessions with identical paths stay separate |
| F2: open and preview | Add remote source identity and FileModel load routing; SFTP content access; feed current editor and supported viewers. | Existing open pane/tab and Markdown/text preview work remotely; relative resources are correct; files never resolve on the desktop |
| F3: file-management parity | Route create/rename/delete, save and cd actions; version/conflict handling; refresh the same tree. | Each existing supported action changes the intended remote file only; failure/disconnect keeps buffers and original files; local behavior passes regression |
| F4: Review parity | Remote repo discovery/Git backend; populate current diff view; remote file opens, feedback and supported explicit actions. | The same known fixture produces equivalent local/remote diffs; actions target the correct remote repo/Agent; no local fallback |
| F5: verification and delivery | Focused backend checks, actual SSH/SFTP tests and native existing-panel walkthroughs; capability compatibility; documentation and source-matched packages. | All required behavior passes on the platform matrix; README capability claim updates only after acceptance |

F1 precedes F2/F4; F2 precedes F3 and reviewed-file editing; F3/F4 precede final
acceptance. Deliver one reviewed vertical path first (ssh, cd, tree, open text)
before expanding to all actions. Each phase changes existing components and only
the backend/protocol pieces needed for its gate.

## Verification plan

The [Companion verification matrix](COMPANION-CHECKS.md) inventories protocol,
installation, routing, native filesystem, lifecycle, resource, failure and
platform checks. It records executed receipts separately from remaining gates.

- **Backend boundaries:** malformed paths, Unicode/whitespace/leading hyphens,
  symlink and Windows reparse escapes, replaced roots, stale fences, late replies,
  permission denial, special files and bounded large directories/files.
- **Files and previews:** non-Git project; nested cd; text/Markdown and each
  currently supported in-app preview type; relative resources; create/rename/
  delete; save/reload conflict; identical remote/local paths; cd with a dirty tab;
  partial upload and lost commit response. Confirm changes with remote filesystem
  reads, not only UI success messages.
- **Git:** staged/unstaged/untracked, rename/delete, binary, conflict, worktree,
  unborn HEAD, available/missing comparison branch, Git missing and permissions.
  Compare fixture outputs against native Git. Concurrent Agent edits cause a
  truthful refresh; review feedback cannot reach another session.
- **Transport:** real OpenSSH/SFTP, master closure, port/config/jump options,
  native Windows SSH arguments, missing SFTP subsystem, timeout and reconnect.
  Use non-sensitive isolated fixtures and no personal credential/config reads.
- **Platforms:** both desktop platforms; Companion file/Git tests on Linux,
  macOS and Windows. Native SSH/SFTP routes must have actual receipts; compile
  checks or a POSIX fixture cannot stand in for Windows runtime evidence.
- **Existing UI:** reuse current screenshots/components as layout references.
  Native checks cover keyboard actions, existing menus, panel switching, editor
  focus, dirty buffers, previews, Review navigation and stale/error states.
  No new frontend scaffold or separate screenshot-design phase is needed.
- **Builds/regressions:** focused changed-logic tests, desktop `cargo check -p
  warpai --bin warpai`, relevant `warp_platform` consumers and three Companion
  builds. Normal local file tools, ordinary SSH and existing Agent communication
  remain required regression paths. Record exact-source results and limitations.

## Completion

On a supported, provisioned remote target, ordinary SSH and `cd` are sufficient
to use the existing file-management, in-app viewing/editing and Review surfaces.
The user does not select a second connection or configure another panel. Every
operation retains its remote identity; failures are visible and never trigger a
desktop-path fallback. Backend, native transport and existing-UI receipts are
recorded in COMPANION-CHECKS.md.

## Collaboration panel delivery

The daily view shows project, agent availability, task descriptions, assignees,
messages, results and review actions. Protocol identifiers, revision counters,
readiness provenance and lease bookkeeping stay out of the daily view. History
and workspace maintenance retain their existing explicit navigation. Empty
maintenance sections are hidden; failures and interrupted execution remain visible.

## Implemented behavior and delivery checks

Existing Explorer follows confirmed SSH cwd and session selection, polls visible
directories, and sends create/rename/delete through typed Companion control.
Clicks, pane/tab opens, code/text editing, Markdown preview, relative image/file
links and saves use the original remote identity. Local editor preferences cannot
redirect these files to an external desktop editor. Windows terminal directory
navigation uses native PowerShell quoting.

Existing Review discovers the remote repository and uses the current renderer
for HEAD, main and selected available refs. It compares native Git status/patch
observations and downloads immutable base content through SFTP. Review Git writes
are explicitly unavailable in the panel; users run Git actions in its remote
terminal. No implicit fetch, desktop Git invocation or unrestricted command API
is added. Companion advertises file and Git review capabilities separately.

Open editors keep their original connection across cd/session changes. Save
checks the observed SHA-256, preserves buffers on conflict/disconnect, and offers
explicit reconnect to the same account/project. A replaced root fails admission.
Transfers, patches, directory depth and rendering cache have bounded capacities.
Canceled in-flight transfers close their attachment; completed missing-path
errors do not disconnect unrelated open editors.

The native acceptance harness uses an owned loopback SSH/SFTP server with generated
fixture keys. It exercises existing Explorer file selection, in-app code and
Markdown, a relative image, dirty buffer/save and existing remote Review. Native
captures supplement backend and actual transport tests; exact cloud receipts are
recorded in COMPANION-CHECKS.md after the corresponding jobs complete.

Final desktop acceptance: [run 37569663988](https://github.com/OthinusG/warpai/actions/runs/37569663988),
source `60ea2810`, passes default/warp_platform builds, focused application tests
and complete native captures on macOS and Windows. The walkthrough uses original
Explorer clicks, remote editing/save, Markdown with a relative image, original
Review rendering and SSH exit/local restoration. The 1.2.0 release packages
matching desktop and Companion builds; update both components when upgrading.
