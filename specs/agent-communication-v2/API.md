# SSH Agent Communication API

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
