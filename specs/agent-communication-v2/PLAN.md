# SSH Agent Communication Extension Plan

Date: 2026-10-03. Status: scope corrected by the user; implementation in progress.
This plan replaces the broad SSH project-manager delivery. The objective is to
extend working same-project Agent communication to Agents running through SSH,
with concise connection/Agent/communication status in the existing panel.

## Scope and completion

Reuse existing messages, tasks, reviews, history, native MCP and terminal UI.
Two Agents in separate SSH terminals of the same selected remote project share
one remote Broker/Store. Other remote roots/accounts/hosts remain isolated.
The remote machine needs only the repository companion and user-managed vendor
CLI/authentication. SSH credentials remain system-owned. Local communication
continues working. No cross-host/local-remote task federation is implied.

A simple remote target/root/helper selection and status section are sufficient.
A file manager, transfer queue, automatic installer, session dashboard, full
remote desktop manager, mandatory retained-session UX and broad vendor/physical
matrix are not prerequisites for this delivery. Existing unrelated terminal/file
functionality and in-progress foundations are preserved; no bulk rollback.

## Tasks and acceptance

- [ ] **S0 — Remove redundant implementation.** Delete newly added file/SFTP
  manager, independent Connections preview, host resource metrics and unnecessary
  session-management features and their protocol/UI/test/workflow plumbing.
  Preserve original terminal/file core and local communication. Complete focused
  regressions before accepting cleanup; an unused leftover is not completion.

- [x] **S1 — Remote communication authority.** Reuse existing private Broker/Store
  on the remote account, fixed to the selected native project. Same-project
  connections share authority; root replacement and old device authorization
  cannot adopt it. Accepted source 65e1b86, run 37110078158.
- [x] **S2 — Per-run private MCP foundation.** Fresh child credentials, fixed
  project/native run, existing MCP SDK and operation semantics; revoke after
  observed owned exit and reject replaced-root/stale-capability replay.
  Actual retained native child performs MCP initialize/tools discovery.
  Accepted source 9a8e692, run 37110965089 on Linux/macOS/Windows. This backend
  gate does not prove vendor launch UX or two-Agent execution yet.
- [ ] **S3 — Existing SSH terminal Agent workflow.** Provide the smallest explicit
  remote launch/binding path using current vendor adapters and terminal IO.
  Ordinary SSH commands continue working. No automatic credential copying or
  unsupported readiness/wake claims. Prove two real fixture processes exchange
  messages/tasks in one remote project and another root cannot participate.
- [ ] **S4 — Existing panel integration and concise status.** Select an SSH target,
  root and manual companion path; read remote Agent/message/task projections in
  the current collaboration panel and send explicit human messages. Show remote
  location, connected/disconnected/error, Agent run and last observation.
  Keep focus/drafts; disable writes offline and fence late project responses.
  Reuse reviewed native controls; static review precedes live integration.
- [ ] **S5 — Focused regression and delivery.** Local communication remains
  functional. Verify same/different project, fresh/stale run, disconnect/reconnect
  and exact response scope. GitHub-only Rust checks on relevant targets, both
  desktop builds and focused native UI review. Record exact-source evidence and
  update PROGRESS after each completed item; document explicit helper setup.

## Working rules

Specify → Plan → Execute → Verify, with the active PRODUCT/TECH/API contracts.
Use existing code, system OpenSSH and installed dependencies. Preserve original
mutation identity and unknown execution. No secret values in code/docs/logs.
No model-call budget was supplied: deterministic remote-process checks establish
engineering behavior; paid vendor validation is reported separately when needed.

The previous R0–R7 plan is archived in legacy-ssh-project-manager/PLAN.md.
Historical PROGRESS/ACCEPTANCE remains evidence, not a requirement to finish the
archived product. The earlier full-product time estimate does not apply here.
