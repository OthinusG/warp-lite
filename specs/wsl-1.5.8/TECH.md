# WSL connection and file contract

## Boundaries

Extend the existing selected remote-session route with a WSL connection containing
explicit distribution/user. Launch the absolute Linux Companion through wsl.exe
--distribution <name> --user <user> --exec <path>. Reuse HostClient initialization,
project-open, account service, fences, task/store and file-control handling.
Require confirmed Linux cwd/home/user and prefer a nested SSH route when present.
No shell interpolation of distro, user, path or file content.

WSL scope includes distribution/user plus verified service/account/project identity.
Use Linux Git common-directory ownership; do not treat /mnt/c as Windows-local.
Host and guest authority remain separate. Nested SSH still uses its existing route.

Direct WSL desktop routing, wsl.exe transport, chunk-transfer client, native
capture fixtures and launch adaptation compile only on Windows. Keep additive
protocol fields, chunk service and native launch service cross-platform: the Linux
Companion inside WSL must implement them. Future Linux desktop work is separate.

## Agent launch directory

TerminalLaunch adds optional working_directory, gated by managed_launch_cwd.
It preserves the original shell cwd for relative native arguments while the
selected project and MCP authority follow Codex's verified --cd/-C option.
Resolve the option in Linux using the installed CLI help; reject invalid directories.
Replay compares the original cwd too. Existing callers omit the field.

## File API

Keep SSH/SFTP transfers unchanged. WSL adds bounded binary chunks to existing
ProjectFilesRequest/Result with new read/write transfer actions, byte offset,
bytes and completion state. Capability project_file_chunks gates use. Reads and
writes reference a connection-owned prepared transfer ID, never arbitrary paths.
Read snapshots and staged writes reuse Prepare/Release/Commit and hash/conflict
checks. Enforce direction, sequential writes, 16 MiB file and 64 KiB chunk limits.
Transfers are revoked on disconnect; never retry an uncertain mutation blindly.
Yield between chunks so other control operations can run. Preserve binary content,
permissions, private cache and atomic final replacement. Protocol major stays 1;
new fields/actions are additive, old clients retain their SSH/SFTP path.

## Tasks and verification

1. Add protocol chunks, service handling and adversarial synthetic tests.
2. Add WSL transport and confirmed selected-session integration.
3. Reuse file/UI/task routes; wire native guest Agent entry and setup errors.
4. Add Windows-only cloud selection and disposable WSL acceptance; run native
   default/platform checks and Windows SSH regressions from the same source.
5. Record actual receipts and limitations. Do not tag/publish 1.5.8 without a
   separate release instruction. Companion version is 5.0.0; no telemetry.

## Windows build correction

Run 38031271996 passed protocol/MCP and history checks, then failed application
compilation with E0716 in selected_session.rs. Retain HostInfo in a local binding
before borrowing os_category; preserve WSL Linux override and nested SSH routing.
Acceptance remains Windows default/platform builds, selected-session tests, real
WSL2 acceptance and native captures from the corrected source.
