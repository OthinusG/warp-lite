# Direct WSL workspace — Warpai 1.6.0 / Companion 4.0.0

Owner approved direct local WSL access and implementation on 2026-10-10.
Owner revision: Windows Warpai bundles the separate WSL component. Its existing
WSL MCP settings contain one install checkbox; no standalone Windows manager.
Keep existing Linux SSH Companion behavior/version 4.0.0. WSL Companion is 4.0.0.
Validation is Windows-only, including the dedicated component executing in WSL.
Both desktop release versions are 1.6.0. macOS retains 1.5.5 functionality; WSL
desktop code/resources compile and package only for Windows. SSH and WSL component
versions are both 4.0.0, with separate binaries, installation and state namespaces.

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
The Windows app includes the WSL component and private Git payload. A Companion
installation checkbox in WSL MCP settings detects/installs/removes it in the
currently selected logged-in account; no separate installer or account picker.
Do not install distributions or overwrite/remove SSH Companion or project data.
Windows Warpai starts/connects WSL Companion on demand from confirmed sessions;
users enter ordinary Linux Agent commands and cd. Never rewrite those commands
through `companion agent`; native Agent MCP clients launch the guest bridge.
Participating interactive Codex uses native --no-daemon to keep MCP owned by
the current Agent; existing flags and all other original arguments are preserved.
Settings first shows an independent WSL communication switch. Enabling it reveals
a dropdown of WSL 2 accounts already logged into live Warpai terminal sessions,
labelled by distribution and Linux user. Selecting an account discovers its Agents;
selection and MCP installation are independent of Windows-local communication.
Explain that users must log into WSL in Warpai before configuring it. Do not scan
or start unlogged distributions, substitute a default user, or enroll restored
historical/unconfirmed sessions. Multiple panes of one account produce one entry.
WSL Rescan and Remove all act only on the selected distribution/account. Rescan
is read-only; removal preserves other integrations, distributions, users, Windows
local and SSH configurations.
Support the WSL 2 execution mode without an upper bound on the WSL software package
version, including 3.x preview packages. Record the package version actually tested.
Missing/old WSL Companion opens the dedicated installation path. SSH behavior
and desktop account-usage configuration remain unchanged.

## Verification

Windows default/platform application checks; protocol, staged-file, traversal,
conflict and transfer-boundary tests; real WSL2 stdio/files/Git/Agent workflow;
existing Windows SSH regressions and native UI assertions. Synthetic accounts
and disposable projects only. Missing runner WSL support must be reported rather
than counted as successful WSL acceptance. No macOS validation in this iteration.
