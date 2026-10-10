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
capture fixtures and settings compile only on Windows. Guest-only handlers compile
with the dedicated Linux WSL service feature, excluded from default SSH builds.
Future Linux desktop work is separate.

## Dedicated WSL component and Windows installation

Build warpai-wsl-companion with the opt-in wsl_companion feature, reusing service
code while default SSH builds exclude WSL-only handlers/capabilities. Default
Companion stays 4.0.0; dedicated WSL builds report 4.0.0. Install at
~/.config/.warpai/wsl/bin/warpai-wsl-companion. Use a separate service data namespace.
Desktop WSL routing never points to the SSH binary. Retain SSH behavior and state.
Windows app bundles the verified guest archive. The existing WSL MCP settings
checkbox installs/removes only the currently selected logged-in account via
wsl.exe stdin. No standalone manager, exported account list or new selector.

Bundle the verified Linux x64 payload in the Windows application with its
private Git runtime. Configuration targets are confirmed WSL accounts logged into
live Warpai terminals; resolve the selected Linux user/home from that session.
Probe/checksum/install/update/uninstall
through literal wsl.exe arguments, never distro name interpolation or Windows UNC
writes. Verify payload provenance/checksums and target architecture, reject links
at installation boundaries and preserve SSH installation/project/history data.
Reuse the existing staged Unix installer as a generated WSL-specific payload script.

Acceptance adds default SSH capability/version regression, separate WSL feature
build, bundled checkbox install/reinstall/detect/uninstall, wrong distro/user/link/
corrupt payload failures, and the full native Windows WSL/SSH workspace walkthrough.

## Native Agent MCP and settings

After `wsl`, the foreground shell and Agent are Linux processes. Do not prepend
Windows MCP environment or launch them in a managed Companion PTY. Install MCP
through the selected guest's native vendor configuration. For participating
interactive Codex invocations, append native --no-daemon once, preserving the
original argument tail, aliases/functions and batch commands. This is authorized
to bind MCP to the current foreground Agent rather than a shared daemon.
The background WSL service owns project brokers; bind MCP to the verified Linux
foreground command group leader, verified ancestor and start time, and actual
working directory. Reject detached/non-foreground owners; MCP/panel disconnect does not
clear a live Agent or Coordinator; actual process exit revokes that run.

Persist the independent WSL toggle and selected distribution/user in desktop settings.
Only enabling reveals the logged-in account dropdown; only selecting starts Agent
discovery. Guest MCP configuration belongs to distribution/user, not terminal focus.
The selected settings account is a maintenance target, not an exclusive active
account. Other logged-in accounts retain their participation and Coordinator.
Codex launch probes its own account read-only, including after desktop restart.
Use live TerminalView active bootstrapped sessions from all Warpai windows, not
the focus or restored history. Require Linux user/home and exclude pending/nested
SSH sessions; deduplicate by distribution/user. Reuse the native dropdown and WSL
execution-mode metadata to admit only mode 2, independently of package version.
All setup calls explicitly pass --distribution and --user; use the confirmed home
for the dedicated executable, not a login shell's inferred HOME. Recheck live
membership before setup and accepting asynchronous results. Closing the last
session removes the entry and invalidates setup results without deleting saved
MCP settings or Coordinator state. Saved selection never starts an unlogged guest.
Use distinct WSL maintenance actions, fenced by the same live selected account.
Rescan discovers Agents without writing guest preferences or MCP configuration.
Serialize account setup with a separate no-follow, nonblocking guest lock file.
Disable duplicate desktop operations while a request is active. A timeout or
transport error may follow partial writes: report uncertainty and require a rescan;
only an explicit missing-executable probe permits discovery-only fallback.
Remove all revokes guest participation first, then removes only owned WSL MCP
entries from saved and discovered Agents in that account; preserve unrelated
servers and all other accounts/distributions, Windows-local and SSH settings.

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

Native wake uses the existing fenced ProjectTasks channel, keyed by the Linux
shell PID reported by Windows-only bootstrap code. The guest verifies shell and
foreground Agent start times and the exact run; observe/claim/validate/finish
reuse Broker readiness, draft, approval and delivery receipts. Windows observes
all live terminals independently of focus. User-input epochs, current shell/root,
shared sessions and rich drafts fence both text and delayed Enter. Validate again
in the guest before Enter; interrupted or uncertain submission is not retried.
Only a fixed inbox instruction and validated message UUID enter the native PTY.

1. Add protocol chunks, service handling and adversarial synthetic tests.
2. Add WSL transport and confirmed selected-session integration.
3. Reuse file/UI/task routes; wire native guest Agent entry and setup errors.
4. Add Windows-only cloud selection and disposable WSL acceptance; run native
   default/platform checks and Windows SSH regressions from the same source.
5. Record actual receipts and limitations. Do not tag/publish 1.6.0 without a
   separate release instruction. Companion version is 4.0.0; no telemetry.

## Windows build correction

Run 38031271996 passed protocol/MCP and history checks, then failed application
compilation with E0716 in selected_session.rs. Retain HostInfo in a local binding
before borrowing os_category; preserve WSL Linux override and nested SSH routing.
Acceptance remains Windows default/platform builds, selected-session tests, real
WSL2 acceptance and native captures from the corrected source.

Run 38032799233 passed Windows default/platform builds, protocol/MCP, focused
application tests and WSL2 provisioning. Source export failed because the guest
lacked git-lfs required by checkout attributes. Install git-lfs in the disposable
guest and assert its availability before archive; preserve hydrated source inputs.
Real WSL workspace and native captures still require acceptance.

## Revision implementation and remaining acceptance

The revision now has a separate feature-gated WSL guest binary, installation path,
service namespace and version; default SSH capabilities/version remain 4.0.0.
Native vendor MCP configuration adapters moved to agent_bus for host/guest reuse.
Guest MCP binds through the private service with Linux socket-peer ancestry,
UID and process start time checks. Service-owned runs survive MCP attachment loss;
exit/PID reuse or explicit communication disable revokes them. Synthetic tests
cover lifecycle, ancestry, guest Codex persistence and preservation of user servers.
Windows settings persist an independent switch and logged-in WSL 2 account selection,
then discover/configure only that confirmed account. Stale discovery results cannot
replace a newer selection. The package major version is not an admission condition.
Codex launch performs a read-only lookup in its own confirmed account and adds
native --no-daemon once for participating interactive commands. Settings selection
no longer suppresses other logged-in accounts' panels. Foreground kernel identity
tests cover launcher descendants, detached rejection and process-owned lifetimes.

Implementation is complete for native wake/readiness, edit epochs, draft/approval
guards and actual process-owned exit, plus bundled install/reinstall/detection and
owned-runtime removal. Windows-only acceptance covers ordinary Linux-shell MCP,
claim cancellation, wake acknowledgement and Coordinator retention/exit. Native
captures add WSL settings, guest-image Markdown preview and original Review.
Build the guest payload in Ubuntu 22.04 WSL for the same libc baseline as SSH.
Combined source-matched Windows CI, executable assertions and capture inspection
remain the acceptance gates; parsing and older architecture runs are not receipts.
Test WSL package 3.x previews when available without claiming unrun compatibility.

## Native SSH regression correction

Run 38050697525 at 8efd3867 passed the bus unit and coordination suites, then
failed managed_agent_fences_input_project_and_owned_stop: the default SSH service
correctly rejects the WSL-only launch-directory capability, while the test expected
WSL path validation. Keep production admission unchanged. Assert feature-unavailable
for default SSH and invalid input for relative paths in the WSL feature build;
also cover a valid directory rejected by default SSH. Rerun the Windows regression,
then source-matched default/platform checks, real WSL acceptance and native captures.
