# Self-contained remote project tools

Date: 2026-10-07. Status: released in App 1.3.0 with Companion 3.0.0.

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

## Verification receipts

- GitHub Remote Companion run 37600588598, source f7c749a4: Linux, macOS and
  Windows all pass protocol/file tests, native packaging, repeated installation,
  runtime checksums and private Git status/diff with system Git absent from PATH.
- Local syntax checks, workflow YAML parsing, the staged Unix installer fixture
  and native capture diagnostic redaction check pass. Rust builds run on GitHub.
- Desktop run 37599409548 passes both default/platform application checks and
  application tests on macOS and Windows. Its application sources are unchanged
  by the packaging-only repair. Both native captures exit successfully with
  all assertions and 170 nonempty PNGs each, including SSH Review and Code View.
  The workflow's obsolete screenshot total fails after successful capture; the
  corrected total (fixtures * 8 + 66) passes against the downloaded real artifact.
  SSH Review and Code View screenshots were inspected on both desktop platforms.
  The original run remains red for that post-capture count only; application
  sources were not changed, and no redundant native rebuild was dispatched.
- Subsequently released from immutable v1.3.0 source through formal runs
  37606929158 and 37613962878, with all release jobs passing. No installation on
  personal machines was performed; the delivery receipt is in specs/RELEASE.md.
