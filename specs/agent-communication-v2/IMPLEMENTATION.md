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
use that installer for managed projects. A repository-owned compatible server
and explicit protocol scope/version extensions are still required for V02.

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
