# SSH Agent Collaboration Native UI Checkpoint

Date: 2026-10-04. Active scope: [PLAN.md](PLAN.md) S4. The existing collaboration
panel, native editors/text buttons and theme/soft-wrapping rules are the visual
source. Earlier local and superseded manager evidence is
[archived](https://github.com/OthinusG/warpai/blob/47a2a5a/specs/agent-communication-v2/legacy-ssh-project-manager/UI-CHECKPOINT.md).

## 2026-10-09 Worktree correction — accepted

Visual source remains the existing native Appearance theme, Button/Flex components
and fixed Worktree fixtures. Desktop canvas checks use 320/600px panel widths,
light/dark themes and 1.0/1.25 zoom. Intentional changes:

| Area | Baseline issue | Expected corrected artifact |
| --- | --- | --- |
| Mode selection | Text controls have weak selected state | Equal-width native Accent/Secondary Project and Worktree buttons |
| Spacing | Rows, wrapping runs and sections use different gaps | Consistent 8px gaps and left alignment |
| Coordinator | Selector limits checkout and repeats names/actions | One compact selector with candidates from every checkout |
| Creation | Separate panel form and worker binding | Existing Warp worktree modal creates a terminal tab; participation is automatic |
| Daily actions | Storage, mapping, filtering and repeated task buttons crowd the panel | Message/Assign and contextual review actions; paths expand on demand |
| History | Protocol counters and export IDs dominate the view | Task descriptions/states and readable messages; offline names remain available |

The native walkthrough also switches Coordinator to another checkout and returns
to the original tab, asserting retained selection and the original unsent draft.
The empty-project capture checks automatic repository membership before Agent
registration; obsolete manual join/leave captures are retired.
Source 46248e08/[run 37943970413](https://github.com/OthinusG/warpai/actions/runs/37943970413)
passes macOS and Windows. Each source-matched diagnostic reports exit_code 0,
no failed assertions or panics, and 214 valid PNGs. The non-Git probe's Git
exit 128 is expected negative-path context, not a capture failure.

Rendered review confirms all six areas above: native selected-state colors and
equal button widths, 8px spacing, wrapped/scrollable Coordinator choices, the
original repository/branch modal with a real new tab, compact contextual actions,
and historical participant labels. Collapsed long branch names intentionally fade;
expanded paths wrap. The detail-scrolling fixture uses a 600px-high window to
produce real overflow with both platforms' font metrics and restores its bounds.
Coordinator switch/return captures retain the original unsent terminal draft.

About adds 24 frames per platform: light/dark, 800/1200px windows at 1.25 zoom,
and idle/checking/current/available/unavailable/error states. Reviewed all Windows
states and all macOS states at production-equivalent source 27a9f992, then checked
current-source narrow Download/error and wide checking captures. The earlier
narrow switch/attribution clipping is corrected with the existing shrinking-label
row and wrapped attribution; Update/Download/Releases remain visible. Startup
preference enable/disable assertions pass. These receipts supersede the earlier
capture failures; the baseline receipts below remain historical only.

## Static gate accepted

Source06aa074/run37122711581 produced native macOS/Windows captures. Reviewed
SSH connected/connecting/disconnected/error states, including narrow 320px at
1.25 zoom and wide 600px. Guidance wraps without overlap and the unsent terminal
draft survives. This permits live integration using the existing form controls.
No independent dashboard, resource metrics, file actions or session manager.

## Live gate accepted

Source4fcb0c3/[run37138929300](https://github.com/OthinusG/warpai/actions/runs/37138929300)
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
cross-device runs are not claimed. Final release packaging passed S5 in the same
source-matched desktop run. Current-source narrow 320px/1.25 zoom light connected
and dark disconnected fixtures were also reviewed on both OSes without overlap.
