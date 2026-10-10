# Direct WSL workspace — Warpai 1.5.8 / Companion 5.0.0

Owner approved direct local WSL access and implementation on 2026-10-10.
Validation is Windows-only, including Linux Companion execution inside WSL.

## Behavior and acceptance

Use the existing Windows WSL terminal entry and confirmed shell metadata. Follow
its Linux directory in Explorer, editor, preview, Review and collaboration.
Connect through wsl.exe without an SSH daemon, ports, keys or UNC file fallback.
Use the actual distribution and Linux user. Reject incomplete/pending selections.
Same environment and Git common directory share project mode and Coordinator;
different distributions/users and Windows-local projects remain isolated.
Preserve focus, drafts, exact Agent run identity and uncertain operations across
view switches and transport failures. Terminating WSL invalidates old run authority.
Agent CLIs and authentication remain Linux-owned; no Windows bridge injection.
Missing/old Companion produces actionable installation guidance. SSH behavior
and desktop account-usage configuration remain unchanged.

## Verification

Windows default/platform application checks; protocol, staged-file, traversal,
conflict and transfer-boundary tests; real WSL2 stdio/files/Git/Agent workflow;
existing Windows SSH regressions and native UI assertions. Synthetic accounts
and disposable projects only. Missing runner WSL support must be reported rather
than counted as successful WSL acceptance. No macOS validation in this iteration.
