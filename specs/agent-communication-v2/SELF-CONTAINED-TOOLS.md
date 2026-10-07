# Self-contained remote project tools

Date: 2026-10-07. Status: implementation in progress.

## Approved outcome

After installing Companion, ordinary SSH and cd select the project for the
existing Explorer, editor/preview, Code Review and Agent communication. Do not
require users to install Git, Python, Node, tmux, socat or a separate MCP bridge
for Warpai's infrastructure. Vendor Agent CLIs and their authentication remain
user-owned. Existing SSH/SFTP server and the OS shell are transport prerequisites.
This supersedes the exclusion of SSH Review in REVIEW-BASELINE.md. Automatic
desktop deployment is not requested by the corrected scope.

## Implementation and contract

- Reuse the previously verified original Review renderer and remote Git protocol.
  Explorer and Review share RemoteFiles attachments for the exact SSH session and
  project, without an independent connection configuration or panel.
- Ship a private Git runtime with native Companion installers. Prefer its absolute
  executable path, without changing global PATH or Git installation. Existing
  unpackaged development/test binaries may use system Git.
- Build Unix Git from pinned upstream source; Linux Git links its runtime statically,
  macOS uses OS libraries only. Windows carries verified MinGit and its DLLs.
  Preserve third-party notices and source provenance; verify every runtime file.
- Agent communication/MCP/PTY and filesystem operations already run inside the
  compiled Companion. Text/Markdown previews remain desktop rendering over SFTP.
- Keep session shell integration and existing SSH authentication. Do not modify
  shell profiles, SSH configuration, vendor settings or unrelated files.
- Remote Git writes remain in the terminal until supported by the remote protocol;
  never execute local Git writes against a remote rendering cache.

## Tasks and acceptance

1. Restore the accepted remote Review adapter, sharing the Explorer connection.
2. Package private Git and native runtime dependencies for all three remote OSes.
3. Install/check the complete payload and expose a runtime self-check.
4. Verify Review cases, connection/project isolation and no-Git-on-PATH execution.
5. Run three-platform Companion and macOS/Windows application checks on GitHub,
   plus the existing native UI capture for SSH Review. Local checks are syntax,
   packaging fixtures and static review; they do not replace native acceptance.

Risk controls: checksums before activation, staged runtime replacement, bounded
Git output/timeouts, stale response rejection, and original SFTP save identity.
No new wire operations or dependencies are needed for the existing Review adapter.

## Packaging diagnosis and repair plan

Run 37599403450 passes Windows native install/runtime and all three platforms'
core communication/file tests. Unix packaging needs two focused repairs:
- Exclude source-archive symlinks for optional Git GUI subprojects; Python's safe
  extraction correctly rejects their external targets, and the core-only build
  does not use them. Keep the extraction safety filter.
- Keep macOS's OS-provided iconv for its filename normalization code. Disable
  iconv only for the static Linux runtime.
Re-run native packaging on all three platforms; desktop compilation/capture is a
separate running check. Include the runtime checksum list in the payload checksum.
