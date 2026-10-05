# Remote companion installation

[English](REMOTE-INSTALLATION.md) | [简体中文](REMOTE-INSTALLATION.zh-CN.md)

Warpai 1.1.0 uses the remote component installed for the SSH account. Install it
once on each remote machine, then connect from an ordinary Warpai terminal:

```sh
ssh user@host
cd /absolute/path/to/project
```

An existing SSH host nickname also works. There is no required SSH alias setting,
project-path form or companion-path field. System OpenSSH handles authentication,
jump hosts and host-key review. Warpai follows confirmed shell session metadata;
installation alone does not make an opaque SSH session report its working directory.
Use Warpai's SSH shell integration when the panel requests it. At a remote
PowerShell prompt, select **Integrate PowerShell** in the SSH terminal banner.
This loads integration only in that session; no profile file is written.

## Install on the remote machine

Choose the installer for the **remote** operating system and architecture.

| Remote system | Package | Installation |
| --- | --- | --- |
| Linux x64 | [Self-contained installer](https://github.com/OthinusG/warpai/releases/download/v1.1.0/WarpaiCompanion-linux-x64.run) | Run `sh WarpaiCompanion-linux-x64.run`. |
| macOS Apple silicon | [DMG](https://github.com/OthinusG/warpai/releases/download/v1.1.0/WarpaiCompanion-macos-arm64.dmg) | Open the image and double-click **Install Warpai Companion.command**. |
| Windows x64 | [EXE installer](https://github.com/OthinusG/warpai/releases/download/v1.1.0/WarpaiCompanion-windows-x64-setup.exe) | Run the installer under the remote SSH account. |

The packages contain the correct executable, checksum, version/source manifest and
license notices. No manual extraction or executable-path entry is required. Unix
installation sets executable permissions; Windows uses a per-account installer.
Both use the stable account location:

- Linux/macOS: `~/.config/.warpai/bin/warpai-companion`
- Windows: `%USERPROFILE%\.config\.warpai\bin\warpai-companion.exe`

Install using the same account you use for SSH. Reinstalling the newer package
updates the component in the same location. Keep the desktop and remote installer
from the same release; protocol compatibility is checked before project binding.

## Connect and troubleshoot

Select the SSH terminal and enter the project with `cd`. The collaboration panel
uses the confirmed remote home, OS and directory to locate and probe the companion.
If it is missing, cannot execute or has an incompatible protocol, install/update
with the matching package and select **Reconnect**. Changing host, account, terminal
or project fences outstanding responses; disconnected state is stale and cannot write.

The companion starts its existing private account service on demand. Installation
does not register another system service, change shell startup files or PATH,
copy credentials, edit SSH configuration or replace vendor commands. Install and
authenticate CLI agents with their own tools on the remote machine.

## Linux

Run the Linux installer under your SSH account. Open the SSH terminal and use
`cd` to select the project after shell integration confirms the remote session.

## macOS

Open the companion DMG under your SSH account and run its install command.
Connect in the terminal and select the project with `cd`.

## Windows

Run the per-account companion EXE. At the remote PowerShell prompt, select
**Integrate PowerShell** in Warpai's SSH terminal banner, then use `cd`.
Native Windows OpenSSH does not support ControlMaster; the companion control
connection uses the same destination/options with system key or SSH-agent
authentication. A password-only login cannot authenticate that independent
noninteractive connection. The terminal remains usable; the panel explains
authentication failure and offers reconnect. No password is stored.

Reference: [Microsoft Win32-OpenSSH scope](https://github.com/PowerShell/Win32-OpenSSH/wiki/Project-Scope).
