# SSH Agent Communication Removal Inventory

Active scope is [PLAN.md](PLAN.md) S0–S5. The former manager inventory is
[archived](https://github.com/OthinusG/warpai/blob/47a2a5a/specs/agent-communication-v2/legacy-ssh-project-manager/CUTOVER.md) and is not an active backlog.

S0 removed the new SFTP/file manager, Connections UI/profile persistence, host
resource metrics, retained-session list/detach/reattach/takeover and retired
device enrollment/authentication/federation/pending runtime. Reserved protobuf
tags prevent accidental reuse. System SSH config owns authentication/routing.

Keep the original terminal/file core, local communication, native process
ownership and private IPC/root/capability checks. Legacy Store migrations retain
history, receipts and uncertain original intents without restoring old authority.

Evidence: backend source70a5e1d/run37123392723 and native desktop
source06aa074/run37122711581; see [PROGRESS.md](PROGRESS.md). Subsequent S4
changes and final dependency-only pruning are validated by S5.
