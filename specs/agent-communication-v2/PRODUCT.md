# SSH Agent Communication Extension

Date: 2026-10-04. Status: scoped engineering acceptance complete; review builds available.

Extend existing same-project Agent communication to CLI Agents running through
SSH. Keep the current terminal dialogue and collaboration messages/tasks/history;
add concise remote connection/Agent/communication status in that same panel.

## User behavior

1. The user selects an SSH target, remote project root and installed companion
   path, using system SSH authentication and host-key review.
2. Agents launched in separate SSH terminals of that remote project receive
   private run-bound MCP access to one remote Broker/Store. Existing messaging,
   tasks and reviews work with the same semantics as local communication.
3. Different roots/accounts/hosts are separate projects. An ordinary unmanaged
   SSH tab does not silently receive another project's communication identity.
4. The existing collaboration panel can read the selected remote project's
   Agents/messages/tasks and send explicit human messages. Its status section
   shows remote location, connection state, Agent/run and observation age.
5. Disconnect shows unavailable/stale status and prevents unsafe writes. A
   reconnect validates the same project/authority; old callbacks cannot change
   the current selection or lose a draft. Unknown work is retained.
6. Existing local communication and normal SSH terminals keep working. Vendor
   CLIs and provider authentication stay user-managed on the remote machine.

## Delivery boundary

No new file manager, transfer queue, automatic companion installer, independent
Agent dashboard or complete session-management product is required. Reuse existing
native components and adapters; manual companion placement is sufficient.
Automatic wake is claimed only where existing native safety evidence applies.
The remote companion supports Linux/macOS/Windows; local desktop remains
macOS/Windows. Deterministic remote fixtures do not claim paid vendor compliance.

The former full SSH project-manager design is archived in
legacy-ssh-project-manager/PRODUCT.md.
