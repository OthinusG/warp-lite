# SSH Remote Implementation Checkpoint

Date: 2026-10-03. Baseline: `b52906262bf4e785060780dfe37c633ecc692f32`.

## R0 cutover tasks and acceptance

1. Remove native device enrollment polling and its settings-only busy gates.
2. Deserialize old connection metadata into `legacy_remote_profiles`, never an
   active SSH profile. Remove only keys identified by saved coordinator/device
   UUIDs through the platform provider; retain visible cleanup failures.
3. Remove the shipped device gateway/controller/client. Keep old protocol code
   only in library test builds until migration invariants have replacement checks.
   An old `remote-stdio` invocation returns one bounded `feature_unavailable`
   frame without opening a descriptor, database, socket or SSH connection.
4. Upgrade v6 to v7 with a consistent `.pre-upgrade-v6` backup. Revoke legacy
   devices/invitations/runs and preserve tables, receipts and pending intents.
   Unfinished remote attempts become unknown without an invented finish time.
   Local attempts and all task/evidence history remain unchanged.
5. Reject old device controller operations before replay. Legacy intent storage
   exposes reads only in production; mutation helpers remain test fixtures.
6. Verify on GitHub: protocol/setup/migration suites on both desktop OSes,
   default/platform application checks and focused credential-cleanup tests.

This checkpoint does not complete R0's companion reuse gate or implement R1–R7.
Do not advertise SSH Remote based on this cutover.

## Audited reuse boundary

CodeGraph and caller inspection found no application caller of
`RunningController::start` or `enroll_remote`. The native singleton did poll an
unused enrollment receiver every 250 ms and gated local setup on it. The binary's
`remote-stdio` dispatch was the reachable device gateway. `Broker::control` is
the shared trusted GUI controller boundary and must reject retired operations.

`crates/remote_server` contains a protobuf client, manager, setup/installer and
protocol definitions, but no server binary. `RunCommand` buffers command output;
it does not implement retained PTYs or scoped process ownership. The existing
installer downloads upstream Oz. Preserve ordinary SSH/terminal code, but do not
use that installer for managed projects. A separately packaged compatible companion and explicit protocol scope/version
extensions are still required for V02. The repository already has server handlers
in `app/src/remote_server/server_model.rs`, with daemon/proxy dispatch through
`app/src/lib.rs` and `app/src/remote_server/unix/`. Reuse this implementation where
appropriate rather than assume that no server source exists. Its current daemon
starts a WarpUI headless application, is Unix-only, uses unbounded outbound
channels and has no admitted project/root boundary. It cannot establish V02 for
a small Linux/macOS/Windows companion unchanged. Its random `host_id` is generated
per daemon boot, not a verified durable SSH environment/account identity.

The new managed control path will extend the existing little-endian protobuf
envelope; it must not reuse the retired big-endian JSON device authorization.
Do not add an alternative generic transport or SFTP dependency before testing
the existing client/system primitives against the required contracts.

## Verification record

Local static checks and exact-source GitHub results are recorded in ACCEPTANCE.
Unchecked PLAN gates remain pending until their runtime checks actually pass.

### Application compilation repair

Source a9caea0/run 37040051387 passed both OS backend suites but macOS application
check failed with E0616 at settings_view/agent_communication.rs:149. The settings
view accessed a setup-private legacy profile field. Repair: expose only a boolean
pending-cleanup query on Preferences and use it from settings; keep profile
metadata private. Verify the query in the existing serialization regression,
then repeat both OS backend/default/platform/native-cleanup checks on GitHub.

## Existing remote/local path call sites

| Current source | Actual boundary | Managed-project disposition |
| --- | --- | --- |
| `crates/repo_metadata/src/repository_identifier.rs` | Remote root pairs a daemon HostId with a server-side StandardizedPath; local roots use a distinct variant | Keep location distinction; add verified environment/project/boot fencing, never identify a host by path text |
| `crates/warp_files/src/lib.rs` | register_remote_file routes save/delete through the matching client; file_path returns only local paths | Reuse routing; managed writes need selected-root containment, fingerprints and original receipts before enabling them |
| `app/src/remote_server/server_model.rs` | Navigation canonicalizes on the daemon OS; command cwd and file paths arrive in requests | Reuse remote canonicalization, but admission must pin the root/account before any request; do not expose the old unrestricted command/file handlers as Agent tools |
| `app/src/remote_server/ssh_transport.rs` | ControlMaster socket selects an existing SSH account; setup runs uname and upstream installation | Reuse clean stdio/lifetime handling after interactive trust; replace installer and Windows detection in the managed path |
| `app/src/terminal/view/ssh_file_upload.rs` | File upload inserts a shell SFTP here-string into a terminal | Keep ordinary terminal behavior; this is not a scoped transfer queue or safe filename batch contract |
| `crates/agent_bus/src/lib.rs` | normalize_workspace_path resolves the service host's filesystem and rejects escapes | Reuse on the remote producing service, never apply it on the GUI host to a RemoteFileRef |

R0.3's identity shapes are defined in API section 2, but are not executable new
admission types yet. No old device UUID, alias, daemon boot UUID or identical
local/remote path is promoted to a trusted SSH identity by this checkpoint.


The accessor repair was verified by source d71d8b1/run 37041826672: both OS
backend/history/default/platform/legacy-cleanup checks and macOS native
configuration/wake/Keychain-classification tests passed. No SSH capability or
host-status runtime acceptance is asserted. Packaging is tracked separately.


## Managed protocol and companion foundation

The user requested uninterrupted completion of the entire plan. R0 validation
run 37041826672 completed successfully, including both review packages.

Extract the existing protobuf schema and little-endian codec into
`remote_protocol`, with no WarpUI, cloud installer or desktop dependency.
`remote_server` re-exports the same protocol/generated symbols, preserving
ordinary SSH consumers. Both the GUI client and the companion use this single
schema/codec. Legacy channels retain their existing 64 MiB bound; managed control
uses the same codec with a 1 MiB ceiling and host-status payloads stay <=8 KiB.
No protocol autodetection, JSON device revival or parallel framing implementation.

Tasks and checks:
1. Move schema/codec/tests/build generation; keep public paths unchanged.
2. Reject oversized writes before allocation and reads before payload allocation;
   verify framing recovery and managed bounds on Linux/macOS/Windows in GitHub.
3. Add versioned managed initialization/project attachment and typed host status,
   pinning account/service/project/boot/connection identities before operations.
4. Build the repository companion without desktop crates, collect native remote
   metrics and verify real root identity; then integrate managed SSH client/UI
   through the static UI gates in PLAN. Do not mark full R2/R6 complete early.


Managed read source now adds the independent `warpai-companion` binary to the
existing agent_bus package, reusing the shared protobuf and installed sysinfo.
It advertises only project admission and native host-status reads. Directory
handles detect root replacement; every read checks boot/connection/project.
CPU starts warming_up; memory and selected-volume bytes use native counters.
Sampling is bounded to five seconds. Received projections reject invalid values,
stale fences/generations and repeated sequences, using local receipt age.
This attachment currently exits on stdio disconnect; it advertises no durable
sessions/task service and does not satisfy R2 retention or SR41 GUI acceptance.
The three-platform workflow runs real standalone process and native-root tests.
First shared-codec run 37088358866 passed macOS/Windows but Linux protoc 3.6
required the proto3 optional flag; the build script now explicitly supplies it.
Exact new-source tests are pending.


Source `861bd6c`, GitHub run 37088825313, passed shared wire, scoped native
metrics and actual standalone stdio tests on Linux/macOS/Windows. This proves
read-channel primitives only. Safe SSH profile metadata and system SSH client
are now in source: separate interactive ask/clean batch paths, strict machine
host checking, no terminal capability or Vibe configuration forwarding, bounded
requests, remote shell encoding and exact-response fencing. The Linux workflow
provisions its own loopback SSH daemon/keys and checks an actual remote companion.
These keys exist only in the isolated runner, are never printed, and are removed
after the check. Controlled SSH checks, new static SR41 captures and default/
platform app checks for the extended protobuf remain pending.


On-demand panel visibility now stops new local projection reads and fences
pending callbacks when closed or when another tool is selected. Drafts/forms
remain intact. The native harness exercises closing without terminal draft
changes; saved tool selection and fresh-profile closed state are preserved.
Source 2a66fe0 passed all three OS codec/native/standalone and SSH argv tests;
controlled Linux SSH failed before handshake. Fixture key placement now uses
an owned 0700 directory under the runner home so OpenSSH StrictModes need
not be disabled; safe fixed diagnostics identify any remaining auth issue.
Windows app capture initially failed before building because its protocol
step did not install protoc; that prerequisite now precedes all bridge tests.
Both fixes and visibility checks await exact-source CI.


Run 37089722741's fixed SSH diagnostic identified the Linux runner account as
password-locked: UsePAM=no rejects it before public-key authentication. The
isolated daemon now uses system PAM account checks with password/interactive
authentication still disabled. This is fixture provisioning, not a production
trust relaxation. Companion artifacts now carry source/target/debug provenance,
version, SHA256SUMS and inherited license/notices. They remain read-only review
artifacts, with durable service, installation and release acceptance pending.


Source 7035082/run 37089944901 passed actual loopback Linux SSH companion
handshake/project/host-status reads after the PAM fixture repair, including
replaced connection/boot rejection and missing remote root. macOS and Windows
wire/native/stdio/profile tests and all three source-matched review artifacts
passed. This is controlled Linux remote-process evidence, not physical host or
remote macOS/Windows SSH acceptance. New binary SFTP source adds only bounded
structured list/stat/read, remotely canonicalized root containment and a clean
file-only connection without a helper path. CI now checks exact binary bytes in
a Unicode/shell-metacharacter filename and refuses a symlink escape. SFTP writes,
transfers, GUI integration and Windows root mapping remain pending.


Native capture run 37089753788 (source3f722f5) failed macOS build with E0117:
three legacy repo_metadata conversion impls became foreign-to-foreign after
protobuf extraction. The mapping bodies are preserved as explicit conversion
functions in remote_server, with the one application sender updated. The shared
protocol remains free of desktop metadata dependencies. All callers were searched;
default/platform application builds and native captures must revalidate the fix.


Sourcefc8964c/run37090383877 passed all three OS wire/native/stdio/profile/
SFTP parser suites and Linux real SSH + independent SFTP: binary bytes match
for Unicode/metacharacter names, symlink escape/traversal are rejected, and
explicit disconnect prevents new requests. Review artifacts passed.
Native capture run37090488200 failed its macOS backend suite before app build
because the retired stdio refusal occasionally exited without a flushed frame.
Pinned Tokio1.47.1 Blocking::poll_shutdown returns immediately; flush explicitly
awaits the blocking write. The refusal now flushes and a twenty-process test
checks the actual stdout frame. No raw stderr/payload is included in failures.
Persistent service/account/root identity source now replaces boot-random read
labels, with native ownership/private permissions and file-lock serialization.
Tests cover stable reopen, replaced root, concurrent lock, corrupt UUID and
Unix symlink rejection. Unsupported directory birth identity fails admission.
Service boot/connection still change; retained task/PTYS remain unimplemented.
These latest identity/flush changes require new exact-source CI.

Upstream cloud installer removal: disable the inherited install_binary network
deployment boundary, remove its Oz download template/helper and developer deploy
script, retain ordinary SSH and manually provisioned legacy protocol support.
The new companion is a different bounded protocol and must not be substituted
into the inherited proxy command. New managed installation remains pending.
Identity source6ab9021 passed Windows native private-directory identity tests and
20 real retired-endpoint refusal repetitions. macOS/Linux failed only test cleanup
when the state directory lived inside the replaced project root; isolate test
state from the root before revalidation.

Structured SFTP mutations checkpoint (CI pending): create private directories,
nonrecursive selected-entry deletion, non-overwriting rename, bounded exclusive
partial upload, SHA-256 reread verification and original-intent reconciliation.
New-file upload never replaces an existing destination. Transport uncertainty
retains commit_unknown; no automatic mutation retry or partial cleanup. Atomic
overwrite/editor save/resume, streaming large files, queue/GUI and Windows native
SFTP root acceptance remain pending. Linux controlled SSH test now checks actual
binary upload, reconnect/reconcile, competing destination, rename and delete.

Account-service checkpoint (CI pending): the clean stdio executable proxies to
one private background owner using the same bounded protobuf codec. The native
service lock prevents duplicate owners; current-user IPC and 32-attachment bound
reuse existing OS helpers. Boot identity survives proxy/SSH disconnects; idle
exit occurs only after 60 seconds without attachments. Current capabilities remain
read-only, with no retained PTY/task claim. Actual subprocess tests verify boot
reuse, fresh connection IDs and duplicate-owner rejection; Linux SSH tests now
require one shared service boot. Task Store/PTY retention still require integration.
SFTP source4104bae/run37091911301 passed all three focused platform suites and
controlled Linux byte upload/reconnect/conflict/rename/delete checks.

Native Connections/Projects source is now a separate existing Tools Panel view
with seven fixed design states. It is exposed only in the isolated debug preview
until its visual gate passes; no mock connection is a production capability.
Capture adds 56 width/theme/zoom combinations and draft assertions (219 PNGs
per OS). Live profile forms and remote IO remain pending this native image review.
Account-service source31a21d3/run37092447246 passed all three focused platform
suites, real stdio detach/reattach and duplicate-owner checks, and actual Linux
SSH shared-boot plus SFTP mutation checks. No PTY retention claim follows.
