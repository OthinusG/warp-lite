# SSH Agent Collaboration Native UI Checkpoint

Date: 2026-10-03. Active scope: [PLAN.md](PLAN.md) S4. The existing collaboration
panel, native editors/text buttons and theme/soft-wrapping rules are the visual
source. Earlier local and superseded manager evidence is
[archived](legacy-ssh-project-manager/UI-CHECKPOINT.md).

## Static gate accepted

Source06aa074/run37122711581 produced native macOS/Windows captures. Reviewed
SSH connected/connecting/disconnected/error states, including narrow 320px at
1.25 zoom and wide 600px. Guidance wraps without overlap and the unsent terminal
draft survives. This permits live integration using the existing form controls.
No independent dashboard, resource metrics, file actions or session manager.

## Live gate accepted

Source4fcb0c3/[run37138929300](https://github.com/OthinusG/warp-lite/actions/runs/37138929300)
passed real private companion/native MCP child action assertions on macOS and
Windows. Both source-matched diagnostics report exit_code0 and no failed steps;
each native artifact contains 155 nonempty PNGs. Reviewed all eight live SSH
images per OS: selection, Agent/run status, human message, assigned task, task
detail, disconnected draft, reconnected draft, and return to local authority.
Connection controls share one row and status precedes task pagination. Native
forms retain readable fields and accessible Close/Confirm controls; offline writes
are disabled and both terminal and form drafts survive. No overlap or hidden
primary control was observed in these states.

The shared client path separately passed controlled Linux OpenSSH in remote
run37138932460. Desktop captures use actual local native companion processes,
not a desktop SSH server or synthetic Store. Paid vendor sessions and physical
cross-device runs are not claimed. Final release packaging belongs to S5.
