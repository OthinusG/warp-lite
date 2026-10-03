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
