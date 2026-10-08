# Cross-worktree collaboration

Date: 2026-10-08. Status: implementation in progress.

## Product and acceptance

Extend the existing messages, assignments, reviews, reservations and evidence to
explicitly joined Git worktrees, locally on macOS/Windows and on one SSH remote
host/account on Linux/macOS/Windows. Shared-directory collaboration stays the
default. A user joins each checkout from the existing collaboration panel; fresh
Agent runs in that checkout use the repository's worktree team. Existing runs and
private task history retain their original scope. Leaving fences shared access,
including queued wake delivery, without claiming that native processes stopped.

Git's actual common directory identifies a repository. Matching remote URLs,
repository names or branches never join separate clones. Only checkouts with the
same canonical, native common-directory identity can share the team. Worktrees
remain independent file/editor/Review roots. Local and SSH authorities, different
remote services/accounts and unrelated repositories remain isolated.

Reuse existing worktree creation/tab configuration and Git commands. Agents
submit commit/base/head and test evidence through existing task tools. The lead
integrates accepted commits sequentially and validates the combined result;
task acceptance never merges branches automatically. No new worktree manager,
automatic cleanup, Git writes over the file protocol or cross-host federation.

## Technical contract

- Add private controller `worktree_join` / `worktree_leave` operations with a
  physical checkout root and UUID request ID. These are never Agent MCP tools.
- Resolve and validate Git worktree/common directories through the existing
  bounded, no-hook, private-Git command runner. Use native directory identity,
  including birth time, to distinguish replacement directories.
- Reuse spaces, workspace bindings and producing-root evidence. Store worktree
  admission fingerprints separately from ordinary workspace metadata. Every
  authorization revalidates the admitted checkout/repository identity.
- Local activation captures an explicitly joined worktree binding. Never remap
  an already registered run. Panel routing follows the captured run when one is
  present; new panes show the joined team's projection.
- Remote physical project fences remain unchanged. A separate repository-owned
  Broker/Store serves joined worktrees; unjoined roots retain the original
  per-project Store. Reuse the account-owned service, never desktop-path identity.
  Remote launch and operator projection resolve the same admission. Scope-bound
  requests reject forms captured before a team change.
- UI uses the existing wrapping text-button row and confirmation form, with
  `Join worktree team` / `Leave worktree team`. Show fresh-run guidance and each
  participant's actual checkout. Preserve drafts and fence late responses.

## Tasks and verification

- [ ] Git identity resolver and opt-in workspace admission/revocation.
- [ ] Local run, panel, reservations and producing-root evidence routing.
- [ ] Remote shared authority, fresh launches and scope-bound operator requests.
- [ ] Existing native panel actions and compatibility documentation.
- [ ] Real Git main/linked-worktree IPC tests: messaging, task start/submit/review,
      separate same-path reservations and evidence, unjoined checkout/clone
      isolation, stale scope/run, leave/rejoin, replacement and reopen.
- [ ] Companion tests with different physical project fences and shared team,
      private-history retention, independent file roots and service isolation.
- [ ] GitHub pinned protocol suites on all remote platforms; default and
      `warp_platform` desktop checks on macOS/Windows; focused native UI capture.

Rust compilation stays on GitHub per the project's existing preference. Local
checks cover parsing, focused source review and diff hygiene. Native vendor/model
execution is distinct from deterministic native IPC/Companion acceptance.
