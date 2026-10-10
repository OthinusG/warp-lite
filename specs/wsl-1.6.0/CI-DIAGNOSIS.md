# Windows CI diagnosis

Run: https://github.com/OthinusG/warpai/actions/runs/38051265508
Source: ca2b4f657bb20fa899c3ff3a1d6a592d6c0ca9b4

## Failure and impact

The Windows default application check fails with E0603 at
app/src/agent_communication/wsl_settings.rs:352. Its wsl_command helper calls
warp_agent_bus::session::without_terminal_binding, which is declared pub(crate)
in crates/agent_bus/src/session.rs:30. The session module is already public;
the function's crate visibility prevents the application's external call.

Protocol, persistence, native socket/MCP tests and history checks passed.
remote_server passed all 20 unit tests. The platform application check, WSL2
provisioning, guest build, workspace assertions, bundled installation assertions
and native captures were not reached. macOS was skipped in this focused run.
Warnings are not the terminating error.

## Minimal fix plan

1. Following fix-errors guidance, change the existing function from pub(crate)
   to pub, with a brief public API comment explaining the isolation requirement.
   Keep its environment removal behavior and every caller intact. Do not remove
   the WSL call or duplicate the environment list.
2. Scope: the existing helper, its external WSL caller and existing CI gates.
   Internal SSH, companion and native PTY callers need no behavior changes.
   This is a Rust API visibility correction, with no wire/schema change.
3. Verify the focused diff and Rustfmt parsing locally; run Rust checks/tests
   only on GitHub under project policy. The Windows application build exercises
   this external call directly, so a duplicate visibility unit test is unnecessary.
4. Run source-matched Windows default and warp_platform checks, protocol/MCP and
   remote_server tests, then actual WSL workspace/install assertions and captures.
   Inspect captures and assertion results. Before delivery, complete both OS
   acceptance from the corrected source; a dispatched run is not acceptance.

## Completion criteria

E0603 is absent and both Windows application configurations compile. Existing
SSH regressions remain green, WSL executable assertions pass, and source-matched
native captures pass inspection. No release or merge is justified by this failed
run.

## Applied correction

The existing cleanup function is public with its environment removal behavior
unchanged. Internal callers and the WSL settings caller reuse the same helper.
Cloud acceptance remains pending until actual build, test and capture results
have been inspected. Monitor the source-matched run every 30 minutes.

## Archived guest packaging correction

Run 38052636231 at 54b02dc7 passes Windows default/platform checks and focused
application tests. WSL builds reach packaging, which fails because package-wsl.py
calls git rev-parse HEAD in an archived source directory without .git.

Resolve the SHA in the runner checkout, archive that exact revision and pass it
explicitly through the guest build to package-wsl.py. Validate the full commit SHA
before packaging and keep it in the checksummed manifest. Both validation and
release callers use verify-wsl-workspace.sh and receive the same correction.
The Python regression checks manifest provenance without a Git subprocess and
rejects malformed SHAs; existing installation/isolation tests remain required.
Rerun complete cloud acceptance from the corrected source.
