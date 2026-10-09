# SSH Agent Communication API

## Worktree orchestration contract (2026-10-08)

`PanelQuery.worktree` selects the Worktree projection; false preserves the Project
projection. Worktree projection includes bounded active `candidates`, durable
`roles`, registered `worktrees` and `coordinator_online`. Candidates identify the
native Agent, exact run and physical checkout; panel selection never supplies an
arbitrary process identity or changes a process working directory.

Private controller intents `worktree_coordinator` and `worktree_worker` carry
`root`, `agent`, `run`, `request_id`. The owning local Broker or SSH Companion
validates the active run, native repository/root. Multiple Agents may share the same checkout.
Explicit enrollment creates a separate team identity while preserving prior
Project history. An occupied Agent with unresolved Project work cannot enroll.
Selection does not spawn or restart a process. Any active Agent in any registered
checkout can become Coordinator; a new selection demotes the old Coordinator
without ending its run. Stale selection must fail. Coordinator state belongs to
the repository and run, independent of the panel's focused terminal. Schema v9
removes the checkout uniqueness constraint after a consistent v8 backup.
Active participants automatically join an enabled repository team. Messages and
Agent discovery are scoped to the same checkout or the selected Coordinator;
other workers in different checkouts are excluded. Project panel pagination
filters offline participants before applying its page limit.

`worktree_create` carries `root`, a bounded branch `name`, local `base` ref and
`request_id`. It creates a registered checkout through trusted private Git only;
it never starts an Agent or writes through the file protocol. Existing files or
branches are never reset or deleted. Coordinator authority is required before
workers can receive team assignments. Role changes are trusted panel operations;
ordinary Agent tools cannot nominate a Coordinator or claim another checkout.

`task_integrate` is a Coordinator-only Agent operation carrying an accepted
`task_id`, optional final integration `commit`, and `request_id`. It records the
Coordinator's integration choice; it does not execute Git. If a commit is supplied,
verify it in the Coordinator checkout. Existing task assignment, messages, evidence
and designated peer reviewer transitions remain the task/context/review engine.
Project mode does not impose these role restrictions.

> Current 1.1.0 iteration: [remote installation and terminal-driven connection](INSTALLATION.md) supersedes manual SSH alias/root/companion forms and raw companion delivery.

The active scope is PLAN S0–S5. The former manager contract is archived in
https://github.com/OthinusG/warpai/blob/47a2a5a/specs/agent-communication-v2/legacy-ssh-project-manager/API.md and is not an implementation checklist.

## Transport and authority

Use bounded, length-prefixed protobuf over a clean system SSH stdio connection.
No format auto-detection, public listener, device enrollment or federation.
Initialize verifies protocol major, private account/service/boot/connection IDs.
ProjectOpen binds a canonical native root and opaque project UUID. Every later
request/result carries that exact fence; replaced roots and late responses fail.
Removed host-status/list tags and detach/attach/generation fields are reserved.

Capabilities: project_open, managed_agent, project_tasks, project_mcp.

## Existing file tools over SSH

The 2026-10-07 self-contained-tools iteration reuses `project_git_review` and the
existing Git root/status/diff/branches/base-file operations. Explorer and Review
share a session/project attachment. Native installers resolve Git from the private
runtime named by companion-manifest.json; no system Git installation is required.
No wire schema or protocol-major change is introduced.

`project_files` admits only typed `ProjectFilesRequest` operations under the
current project fence: directory metadata, staged reads, staged saves, explicit
create/rename/delete and transfer release. Requests and results echo selection
generation. Metadata reuses the existing repository snapshot schema, including
non-Git directories. File bytes use SFTP, never a managed file-content payload.
Staged transfers are bounded and connection-owned; transfer IDs are not reusable
across attachments. Saves require the observed SHA-256 before replacement.
Root/path replacement, symlink escape and stale replies must fail before access.
An old Companion without this capability keeps existing Agent operations.

Managed file errors distinguish invalid input, missing path (`MANAGED_NOT_FOUND`,
appended as code 9), permission denial, conflict, capacity and stale attachment.
Stale/incompatible control and failed SFTP routes close the retained attachment;
they cannot silently retry against a desktop path or a newly connected host.
Service crash-transfer cleanup requires exclusive service ownership.

## Agent lifecycle and IO

TerminalLaunch carries a launch-intent UUID, absolute executable, bounded argv,
terminal dimensions and optional Agent program. A managed Agent uses the existing
session::launch adapter inside the service-owned native PTY. Fresh MCP endpoint,
capability and terminal binding are injected into its environment only; credentials
never enter control responses, arguments or preferences. The Broker run UUID equals
the owned native run. Ordinary commands receive no Agent authority.

Only the launch connection may read/input/resize/stop/release its run. Input is
bounded and sequenced; output is bounded and paged with explicit truncation.
Release requires observed full group/job exit and output EOF. Disconnect revokes
MCP and requests owned stop; no listing, detach/reattach or takeover. Failed native
activity observation retains ownership. Launch-intent replay reconciles only the
same connection and immutable payload.

## Existing communication domain

ProjectTasks carries a typed command (panel/operator/controller), query generation
and bounded JSON domain payload. The private remote Broker/Store is authoritative.
The GUI receives value/error envelopes, scoped projections and fixed failure
metadata. It cannot register Agents, claim readiness, start/submit Agent execution,
or access retired device operations. Existing messages/tasks/reviews/versions/
original request receipts remain unchanged. Changed-payload retries conflict;
unknown writes retain their original intent and are never automatically replayed.

Panel Agent rows include optional `run` from the currently observed Broker run.
It is projection metadata, separate from the persisted Agent identity, and is
absent when no active run is observed. Offline cached rows are explicitly stale.

## Explicit SSH terminal launch

`warpai-companion agent <absolute-root> <program> <absolute-vendor-executable>
[vendor-arguments...]` joins the same private service, forwards native terminal IO
and tracks resize. The companion supports mcp and the existing forward relay.
System SSH and vendor authentication remain user-managed on the remote account.

## Owned local Codex bridge invocation

`warpai-agent mcp --native-directory` captures its OS working directory and
supplies it through the existing authenticated request directory field before
MCP discovery. The flag takes no user-supplied path and does not change cwd.
The Codex per-invocation server uses this mode; other native adapters retain
`mcp`. Initial local scope follows the verified native directory, while already
registered or explicitly bound workspace identities cannot change scope.

### Existing Review over SSH

`project_git_review`, alongside `project_files`, admits typed, read-only `PROJECT_GIT_ROOT`,
`PROJECT_GIT_BRANCHES`, `PROJECT_GIT_STATUS`, `PROJECT_GIT_DIFF`, and
`PROJECT_GIT_PREPARE_BASE` actions. There is no caller-supplied command or argument
vector. `git_previous_path` admits a validated project-relative old path for
rename/copy patch selection; it is never a command argument override. Status carries porcelain-v2 records, the resolved base commit, and
name-status records for merge-base comparisons. Git metadata and patches are
bounded at 512 KiB and each subprocess has a 10-second deadline. Base file bytes
use the same connection-owned, hashed SFTP staging and release lifecycle.

Head mode includes staged, unstaged and untracked files. Branch comparisons use
local remote-host refs and merge bases; no implicit network fetch occurs. Git
write operations remain available in the remote terminal and are visibly disabled
in Review. Failed reads never fall through to desktop Git or desktop file paths.

Task list projections include a bounded 512-character `description` preview.
Task detail continues to carry the full description; mutation revisions remain
unchanged and are no longer displayed in the daily Collaboration view.

Remote Review verifies working-content hashes against the patch observation and
checks status/HEAD again before publishing. Rapid branch changes coalesce after
the bounded transfer; obsolete observations never replace the selected mode.
Remote cache sources stay referenced while Review or an editor uses them.
Explicit reconnect verifies the original account, canonical root and root identity;
it cannot replay an uncertain write or silently adopt a replaced project.
# Cross-worktree extension — 2026-10-08

Optional Companion capability `worktree_collaboration` adds private controller
operations `worktree_join` and `worktree_leave`, each with a canonical checkout
`root` and UUID `request_id`. Remote roots are derived from the current native
attachment; a payload cannot select another checkout. Agent MCP tools are unchanged.

Task commands may use `{ "kind": "scoped", "operation": { "scope": "...",
"command": { "kind": "operator", "operation": { "op": "..." } } } }`.
Only one wrapper is accepted. Team task writes require the scope returned by the
last panel projection; mismatches fail with `scope_denied`. Membership operations
pin the physical project and reconcile their original receipt in the repository
Store even after joining/leaving changes task routing. Private old clients retain
their unwrapped command contract. Desktop wraps commands only when the Companion
advertises this capability.

Panel responses keep `project` equal to the physical attachment's project ID for
existing client fence checks. Additive fields `collaboration_scope`,
`worktree_available`, `worktree_joined`, `worktree_root`, and `worktree_branch`
describe the selected team/checkout. Each participant row may carry its own
`worktree_branch` alongside existing `workspace`. Scope changes discard task
selection and pagination cursors. File requests, SFTP roots and ManagedFence are
unchanged. See [the acceptance contract](WORKTREE-COLLABORATION.md).
