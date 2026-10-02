# Superseded Enrolled-Device Gateway Design

Date: 2026-10-03. Status: historical architecture, not the current SSH Remote
entrypoint design. The existing remote-stdio relay requires a running remote
native app and device credentials; it does not provide remote project terminals,
files or Agent process management.

The full historical contract is preserved in
[legacy-machine-collaboration/REMOTE-GATEWAY.md](legacy-machine-collaboration/REMOTE-GATEWAY.md).
Use [TECH.md](TECH.md) for the repository-owned project companion and
[CUTOVER.md](CUTOVER.md) D01/D02/D06/D09/D10 for retirement/reuse boundaries.
The current control protocol decision must audit the existing remote_server
client/protobuf substrate and compatible server source; do not relabel the old
JSON device relay as a completed SSH Remote service.

Keep safe SSH process lifetime/framing/diagnostics, current-user private IPC,
authorization-before-replay and connection fences. Retire invitation/grant/paired
app flows after legacy data/compatibility checks. Do not delete data, change SSH
server settings or enable a new remote service as part of this document change.
