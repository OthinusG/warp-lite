# Warpai 1.1.0 remote companion delivery

## Product contract

Supersedes the S4 manual target/root/companion form and 1.0.1 raw companion delivery.
Users connect in a terminal with ordinary system OpenSSH (`ssh user@host`, including
existing host nicknames and connection options), then select the project with `cd`.
The collaboration panel follows that confirmed remote session and working directory.
There are no required SSH alias, project-path or companion-path settings fields.

Remote installers contain the source-matched companion and install into the remote
account's stable `~/.config/.warpai/bin` directory (`%USERPROFILE%\.config\.warpai\bin`
on Windows). Detect the actual remote OS; derive the native executable from the remote
home directory. Never search the local filesystem for a remote companion.
Check installation and protocol compatibility before binding a project. Missing or
incompatible components show a platform-specific install/update guide and retry action.
No automatic download/execution without a user installation action.

Installation owns only Warpai components, permissions and its installation metadata.
Do not edit shell startup files, PATH, SSH settings, credentials or vendor commands.
Do not register a new persistent OS service: the companion retains its existing
on-demand private-service startup. Retain project/account isolation, fences, original
mutation receipts and unknown-work recovery.

## Implementation plan

1. Trace native SSH session/remote cwd/bootstrap and existing OpenSSH multiplexing.
   Reuse trusted connection/session metadata rather than parsing arbitrary output or
   asking users to reproduce connection details. Preserve system host-key review.
2. Build native per-OS installer packages (macOS, Linux, Windows) containing the
   correct release binary, checksums and source/version manifest. Installation must
   be idempotent, enforce Unix execute permissions and report an actionable failure.
3. Discover stable installed components from confirmed remote home/OS and perform
   a bounded compatibility probe. Fail closed on unsupported or unconfirmed sessions.
4. Replace the three-field form with terminal-driven binding, platform-specific
   install guidance and retry. Fence pending replies on cd/tab/host/account changes,
   disconnect and SSH exit; preserve local collaboration and unrelated terminal IO.
5. Update bilingual product/install docs and release pipelines to 1.1.0. Desktop
   delivery remains macOS DMG and Windows EXE only. Remote delivery is installer
   packages rather than raw companion ZIPs. Preserve old source tags and CI history.
6. Run cloud-only Rust checks, focused installer/discovery/isolation regressions,
   controlled OpenSSH tests and native macOS/Windows screenshot QA before packaging.
   Inspect actual release artifacts, checksums and manifests, then publish 1.1.0.

## Acceptance criteria

- Ordinary SSH login and remote cd bind the correct remote project without forms.
- All three remote systems install the matching component to the default location;
  paths with spaces and Unicode work, Unix modes are executable, reinstall is safe.
- No shell/SSH/credential/vendor configuration changes and no new OS service.
- Missing, incompatible, unsupported and failed SSH sessions have useful guidance;
  companion/probe output is bounded, deadlines enforced, and input shell-quoted.
- Same remote account/root shares existing authority; distinct roots/accounts/hosts
  and stale callbacks cannot adopt another project. Local collaboration still passes.
- Native UI has no manual connection form and matches existing theme/components.
- Public 1.1.0 release has desktop installers and all remote installer packages.

## Initial investigation

Existing POSIX SSH bootstrap already creates a system OpenSSH control socket and
reports remote shell, native home/OS, session identity and cwd events. The existing
RemoteCommandExecutor reuses that socket, including WSL routing. Prefer this route
for established native sessions; do not replace user SSH aliases or configuration.
Windows remote/local PowerShell coverage requires explicit investigation rather
than assuming the POSIX shell wrapper handles it.

## Verification status

Implementation and 1.1.0 acceptance pending. The owner changed the release scope
while 1.0.1 Windows packaging was running; cancel that publication and preserve the
private 1.0.1 draft/source tag. Prior 1.0.1 acceptance is historical evidence only.
