# SSH Task Panel Host Status

Date: 2026-10-03. User requirement: SR41. Status: specified, not implemented.

## Behavior and visual authority

Use the existing native Tools Panel, theme tokens, paragraphs and status controls.
The on-demand task view shows its producing remote host/project in a compact
header; expanded details stay inside this panel. Session navigation remains in
the Tab/Pane sidebar. Do not open the panel automatically.

| Header field | Display rule |
| --- | --- |
| Host/account/project | Exact verified environment and task scope; long names wrap |
| Connection | Connecting, authentication/host review, ready, reconnecting, offline or failed |
| SSH / SFTP / companion / MCP | Independent capability status, with remediation on failure |
| CPU | Remote utilization percentage; unavailable until two valid counter samples |
| Memory | Used/total physical memory with consistent byte units |
| Project disk | Free/total bytes on the volume containing the selected remote root |
| Observation | Source and age; old values explicitly marked stale/offline |

Expanded detail adds remote OS/architecture, uptime and metric availability
reasons. Neither host connectivity nor resource utilization proves Agent idle,
task completion, successful transfer or process exit. Unsupported data must not
render as zero or a fabricated healthy state.

## Remote read contract

`HostStatus` uses the selected managed protobuf control channel, after verified
SSH and project/service attachment. It is a GUI read, not an Agent MCP operation.
The companion collects its own OS-native counters; project disk capacity refers
to the remotely resolved root. Bare SSH/SFTP without a companion still reports
connection capabilities, with resource metrics unavailable.

The response carries environment/project/service boot identity, observation
sequence, remote sample time, source, OS/architecture, optional uptime seconds,
optional CPU utilization, optional memory used/total bytes and optional project
volume free/total bytes. Each absent metric carries an enumerated reason:
unsupported, permission_denied, warming_up or unavailable. No arbitrary shell
output, environment variables, process arguments, keys or filesystem listing is
part of this projection. Metrics are ephemeral; they are not copied into task
events or diagnostic exports by default.

Reuse existing connection health projections. Poll resources at most once every
five seconds while this view is visible, with one request in flight; stop polling
when hidden/offline. Mark a resource sample stale after fifteen seconds without
a new observation. Client receipt age uses its monotonic clock, not a comparison
between unsynchronized host clocks. Bound the response to 8 KiB and retain one
last sample per displayed environment/project. Health reads cannot starve
terminal input, task events or transfers.

Validate finite CPU values in [0,100], used memory <= total memory and free disk
<= total disk; refuse invalid samples rather than clamp them into healthy data.
Use integer byte counters and apply units only in presentation. CPU load average
is not utilization; a platform without a supported counter remains unavailable.
An unsupported metric does not block task access.

## Scope, stale state and acceptance

Fence every reply with the original environment/project/service boot and GUI
query generation. Switching projects, removing a profile or reconnecting rejects
old callbacks before rendering. Historical task detail uses the producing host;
it never substitutes the active tab's host or local machine counters. Offline
shows the last source/time and leaves existing unsafe mutation controls disabled.

1. Static native captures cover all header/capability/metric states, long names,
   narrow/wide, light/dark and 125% text; labels, units and age remain readable.
2. A controlled SSH companion sample is compared to known remote counters and
   remote project volume. Local machine values are deliberately different.
3. Drop the channel and advance receipt age: metrics become stale/offline without
   implying Agent readiness or altering task state/drafts.
4. Switch hosts/projects during an outstanding response and restart the service:
   old samples cannot appear under the replacement identity.
5. Verify denied/unsupported/invalid counters, one-in-flight/five-second bounds
   and closed-panel polling/focus behavior on both desktop targets.

These checks extend V05/V21/V22/V23 and remain pending. No synthetic sample is
presented as live host status.
