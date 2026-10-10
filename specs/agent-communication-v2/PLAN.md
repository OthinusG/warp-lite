# SSH Agent Communication Extension Plan

> Approved 2026-10-08: [cross-worktree collaboration](WORKTREE-COLLABORATION.md)
> defines Project / Worktree panel modes. Worktree mode uses an explicitly selected
> active Coordinator and panel-selected worker/worktree bindings, locally and over
> same-host/account SSH. Creating a worktree never starts an Agent. The existing
> communication foundation preserves physical file fences and private history;
> Coordinator orchestration and local/SSH acceptance are implemented; exact-source
> desktop, Companion and native receipts are tracked in the cross-worktree contract.

> Current 1.1.0 iteration: [remote installation and terminal-driven connection](INSTALLATION.md) supersedes manual SSH alias/root/companion forms and raw companion delivery.

> Approved 2026-10-07 follow-up: [self-contained remote tools](SELF-CONTAINED-TOOLS.md)
> restores original SSH Review and bundles Companion's runtime dependencies,
> superseding the later Review exclusion below. No desktop auto-deployment.

Date: 2026-10-04. Status: S0–S5 accepted; source4fcb0c3 review builds verified.
Quality follow-up: cleaned source `8b89938` passed the three-platform remote and
two-platform desktop workflows. See [replacement decisions and acceptance](QUALITY.md).

This plan replaces the broad SSH project-manager delivery. The objective is to
extend working same-project Agent communication to Agents running through SSH,
with concise connection/Agent/communication status in the existing panel.

Approved follow-up: [existing file tools over SSH](REMOTE-FILE-TOOLS.md) extends
the current Explorer and editor/preview backends to terminal-selected
remote directories. Existing components are reused; its native acceptance and
source-matched cloud receipts are tracked in [COMPANION-CHECKS.md](COMPANION-CHECKS.md).
The owner's 2026-10-07 correction restores Code Review to v1.0.1 and excludes
the SSH Review extension; see [the current scope and acceptance](REVIEW-BASELINE.md).

## Scope and completion

Reuse existing messages, tasks, reviews, history, native MCP and terminal UI.
Two Agents in separate SSH terminals of the same selected remote project share
one remote Broker/Store. Other remote roots/accounts/hosts remain isolated.
The remote machine needs only the repository companion and user-managed vendor
CLI/authentication. SSH credentials remain system-owned. Local communication
continues working.

A simple remote target/root/helper selection and status section are sufficient.
A file manager, transfer queue, automatic installer, session dashboard, full
remote desktop manager, mandatory retained-session UX and broad vendor/physical
matrix are not prerequisites for this delivery. Existing unrelated terminal/file
functionality are preserved; redundant additions are deleted with focused regression checks.

## Tasks and acceptance

- [x] **S0 — Remove redundant implementation.** Delete newly added file/SFTP
  manager, independent Connections preview, host resource metrics and unnecessary
  session-management features and their protocol/UI/test/workflow plumbing.
  Preserve original terminal/file core and local communication. Complete focused
  regressions before accepting cleanup; an unused leftover is not completion.
  Accepted backend source 70a5e1d, run 37123392723, and desktop/capture source
  06aa074, run 37122711581. Later dependency-only pruning is rechecked in S5.

- [x] **S1 — Remote communication authority.** Reuse existing private Broker/Store
  on the remote account, fixed to the selected native project. Same-project
  connections share authority; root replacement and old device authorization
  cannot adopt it. Accepted source 65e1b86, run 37110078158.
- [x] **S2 — Per-run private MCP foundation.** Fresh child credentials, fixed
  project/native run, existing MCP SDK and operation semantics; revoke after
  observed owned exit and reject replaced-root/stale-capability replay.
  An actual owned native child performs MCP initialize/tools discovery.
  Accepted source 9a8e692, run 37110965089 on Linux/macOS/Windows. This backend
  gate does not prove vendor launch UX or two-Agent execution yet.
- [x] **S3 — Existing SSH terminal Agent workflow.** Provide the smallest explicit
  remote launch/binding path using current vendor adapters and terminal IO.
  Ordinary SSH commands continue working. No automatic credential copying or
  unsupported readiness/wake claims. Prove two real fixture processes exchange
  messages/tasks in one remote project and another root cannot participate.
  Accepted source 06aa074, run 37122708969 on all three remote targets, including
  actual Linux OpenSSH. Existing vendor adapters are reused; paid vendor sessions
  are not claimed by deterministic fixture acceptance.
- [x] **S4 — Existing panel integration and concise status.** Follow the confirmed
  SSH terminal and cd-selected root; discover the installed component and read remote Agent/message/task projections in
  the current collaboration panel and send explicit human messages. Show remote
  location, connected/disconnected/error, Agent run and last observation.
  Keep focus/drafts; disable writes offline and fence late project responses.
  Reuse reviewed native controls; static review precedes live integration.
  Original panel acceptance: source 4fcb0c3, run 37138929300; native actions passed on macOS and
  Windows; eight live SSH screenshots per OS reviewed with retained drafts.
  The 1.1.0 automatic-selection iteration is tracked in INSTALLATION.md.
- [x] **S5 — Focused regression and delivery.** Local communication remains
  functional. Verify same/different project, fresh/stale run, disconnect/reconnect
  and exact response scope. GitHub-only Rust checks on relevant targets, both
  desktop builds and focused native UI review. Record exact-source evidence and
  update PROGRESS after each completed item; document explicit helper setup.
  Accepted source 4fcb0c3: remote run 37138932460 and desktop run 37138929300
  succeeded. Both desktop release packages and all three companion artifacts
  were checked against source/digests; no paid vendor or physical-device claim.

## Working rules

Specify → Plan → Execute → Verify, with the active PRODUCT/TECH/API contracts.
Use existing code, system OpenSSH and installed dependencies. Preserve original
mutation identity and unknown execution. No secret values in code/docs/logs.
No model-call budget was supplied: deterministic remote-process checks establish
engineering behavior; paid vendor validation is reported separately when needed.

The previous R0–R7 plan is archived in https://github.com/OthinusG/warpai/blob/47a2a5a/specs/agent-communication-v2/legacy-ssh-project-manager/PLAN.md.
Historical PROGRESS/ACCEPTANCE remains evidence, not a requirement to finish the
archived product. The earlier full-product time estimate does not apply here.
