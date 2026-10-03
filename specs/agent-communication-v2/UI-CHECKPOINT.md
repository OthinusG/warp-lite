# Historical local collaboration panel static checkpoint

Date: 2026-10-02. Status: historical local static checkpoint accepted. This permits only the originally reviewed local integration. SSH Remote Connections/Projects/Explorer/Transfers, existing Tab/Pane Agent status/actions and on-demand task/message states require a new V21 checkpoint before live integration; see PLAN.md and ACCEPTANCE.md.

## Current GUI ownership gate

The existing left Tab/Pane sidebar is the sole Agent/session management surface.
Reuse its native CLI icon/status, row/context actions and pane navigation. The
collaboration panel becomes an on-demand task/message/history view; no duplicate
Agent roster/session controls or mandatory board. New V21/V22 fixtures cover
multiple panes, detached retained runs, exact task/session cross-links, shared
status/task badges and background updates with the task view closed. Existing
Agent-section captures below remain historical and do not approve duplicate UI.

## Visual authority

Use the existing Tools Panel header and resizable container, `Appearance::ui_builder()`,
the current theme font/color tokens, native text wrapping and the existing text-button
variant. No external artwork or design-system overrides are required.

The editable static artifact is `app/src/agent_communication/panel.rs`; representative
data is `panel-fixtures.json`. Start a separately installed review app with
`WARP_COLLABORATION_PREVIEW=1`, open Tools Panel, select Agent collaboration, and use
Next preview state. The preview is clearly labeled as sample data and writes no task
state. Ordinary app launches do not enable the preview.

## Acceptance

- Inspect agents, tasks, detail, empty, loading, disconnected, stale, capacity and failed states.
- Inspect macOS/Windows at 320px and 600px panel widths, light/dark themes and increased UI text size.
- Verify description, acceptance, status, guidance and evidence labels wrap without truncation or overlap.
- Verify scrolling reaches the last section and the next-state control remains usable.
- Verify preview entry and updates preserve the terminal draft and focus; keyboard navigation follows native controls.
- Only enable live reads and controller actions after this checkpoint passes.

Keyboard behavior uses the existing left/right panel focus actions according to
Tools Panel placement. Explicitly focusing the collaboration panel enables
Left/Right or Enter to change preview state, Page Up/Down to scroll, and Escape to
return to the current terminal. Ordinary opening/refresh retains terminal focus.
Provide native screen-reader state/help text. Keep the preview header/control
outside the scrollable detail, and test that an unsent terminal draft survives.

## Fidelity ledger

| Point | Reference | Implementation | Render evidence / status |
| --- | --- | --- | --- |
| Layout | Existing resizable Tools Panel | Same parent container, padded vertical sections | Pending |
| Typography | Appearance UI font and size | Native builder spans with soft wrapping | Pending |
| Palette | Current theme text and scrollbar tokens | Shared tokens; no fixed colors | Pending |
| Controls | Existing text buttons and panel icon buttons | Shared variants and mouse states | Pending |
| Long content | Native selectable, wrapping text | Long description fixture and scroll container | Pending |
| Truthful states | PRODUCT B02/B04/B06 | Explicit sample-data banner and next-action guidance | Fixture coverage check added; render pending |

No screenshot gate or live behavior is claimed by this source checkpoint.

## Native capture harness

The debug review application exposes an opt-in `WARP_COLLABORATION_CAPTURE`
output directory. It reuses the retained native integration driver and GPU frame
capture rather than the removed integration package or OS accessibility automation.
It must use a separate data profile, leave the user's home and daily application
untouched, enforce a bounded runtime, and fail if any requested image is missing.
Capture all nine fixture states at both widths, both themes and normal/increased
UI zoom. Capturing images is not screenshot review or keyboard acceptance.

Run a GitHub-built debug review executable directly with
`WARP_COLLABORATION_CAPTURE=/tmp/warp-collaboration-captures`. The harness creates
a fresh `capture-<pid>/collaboration-static/<timestamp>/` under that directory and
requires all 72 nonempty PNGs. Existing output cannot satisfy a later run. A
300-second watchdog covers application startup as well as capture. The temporary
preferences profile is named `collaboration-capture-<pid>`; no home override or
daily-profile modification is performed.

Source `8ea58e2`, GitHub run `36942065295`, passed both platform application
checks, focused macOS application tests and both OS packaging. The separately
downloaded/signature-verified debug app completed 72 macOS PNGs with exit 0; a
second run also completed with exit 0. Narrow/wide detail images show wrapped
description/evidence without overlap. The native capture invocation does not
require OS accessibility permission. Complete image review, bounded-viewport
scrolling, keyboard/focus behavior and Windows render evidence are still pending.

The validation workflow's explicit `capture_ui` input runs this same native
harness after packaging and uploads PNGs for both OS jobs. Compilation is separate
from the seven-minute runtime step. Missing screenshots fail the runtime step;
partial images are uploaded for diagnosis. CI capture must succeed and its images
must be reviewed before it can count toward the static checkpoint.

The keyboard revision captures 74 PNGs: the original 72 combinations plus
`detail-scrolled.png` and `detail-restored.png`. It checks Left/Right wrap, Enter
preview navigation, Page Down/Up scrolling and Escape focus restoration while
preserving an unsent draft. Resize each fixture to a bounded viewport rather
than relying on an initial size that startup restoration may replace. These
new checks remain pending until the exact debug artifact completes.

Source `8c33356`, run `36950239278`: all macOS capture assertions passed,
including an unsent draft, explicit panel focus, Left/Right/Enter, Page Up/Down
and Escape. The 74 PNGs use a bounded 1200px-wide native viewport. Reviewed
narrow enlarged detail and wide enlarged task states wrap without overlap. The
first scrolled image stops short of the last reservation line; repeat Page Down
and Page Up three times to capture the entire end and restoration. Windows
checks/packages passed, but capture exited 101 with no PNGs. The debug executable
was launched without the portable package's DXC/ConPTY DLLs. Stage these existing
runtime assets beside it and preserve only panic source locations for diagnosis.
This explains a packaging defect; the rerun must prove it fixes Windows startup.

For faster isolated capture, dispatch `protocol_only=true,capture_ui=true`: both
OS protocol suites and debug UI builds/captures run, while application checks and
release packages stay skipped. This is screenshot evidence, not full app build
acceptance. No real-model task execution or daily application installation occurs.

Visual review of all 72 macOS source-`8c33356` fixture combinations completed:
320/600px, light/dark, 1/1.25 zoom, all nine states. Text, status/help and pinned
controls wrap without overlap or horizontal truncation. Narrow enlarged agent/task
lists and detail extend below the viewport and require scrolling; end-of-detail
proof remains pending from the three-Page-Down revision. Windows remains pending.
Diagnostic contact sheets are local review aids, not replacement native artifacts.

Source `1267827`, run `36958662888`: macOS capture passed all 74 images and
navigation/draft assertions. The three-Page-Down detail-end image was visually
reviewed: final reservation warnings, human controls and uncertain-execution
confirmation guidance are fully visible; the unsent draft remains intact.
Windows still failed in wgpu command encoding after runtime DLL staging. Inspection
of the retained readback path found COPY_SRC enabled only by `integration_tests`;
the explicit debug capture needs the same usage. Source `25c80ed`, run
`36961960929`, contains that correction and rejects unsupported texture usage or
format before GPU copy encoding. Windows rerun and screenshot review remain
required before live panel integration.

## Accepted static gate — 2026-10-02

Source `3d66017`, [run 36970541930](https://github.com/OthinusG/warp-lite/actions/runs/36970541930),
passed both native capture jobs: 74 PNGs per OS and keyboard/focus/draft assertions.
All 72 Windows combinations were visually reviewed at native dimensions through
diagnostic contact sheets; full-resolution detail-end and restored-detail images
were reviewed separately. Combined with the complete macOS fixture review and
source `1267827` detail-end review, the static gate passes. Both themes, widths
and zooms preserve wrapping, spacing, pinned controls and the unsent draft.

The fidelity ledger's layout, typography, palette, controls, long content and
truthful-state rows are accepted against those native artifacts. Vertical scrolling
is intentional for long content; final reservations, human controls and uncertainty
guidance are reachable. Live integration must retain this visual system and receive
its own data/interaction capture; static screenshots do not validate live behavior.

## Live read capture gate (pending)

The harness now adds 18 captures to the accepted 74 static captures: an actual
empty broker projection; task and original-attempt details at both widths, themes
and zooms; and detail-end scrolling. A deterministic native IPC client registers,
starts an operator-assigned task and disconnects in the isolated broker profile.
No vendor executable, model or filesystem edit is involved. Assert the receiver
is offline while task state remains running with interrupted/unknown execution,
then restore terminal focus and the original unsent draft. The workflow requires
all 92 nonempty PNGs. These new live assertions/captures require exact-source CI
and image review before M2.3 acceptance.

The operator revision extends the gate to 96 PNGs, adding a typed-override form,
actual controller cancellation with unknown effects, a rejected normal retry and
an explicitly confirmed retry. Verify a new revision retains the previous unknown
attempt, with no false stopped outcome and no terminal draft loss. Source and
platform render/action acceptance are pending.


## SSH task panel host status checkpoint (SR41, pending)

Reuse the existing Tools Panel/theme/paragraph/status controls as visual authority.
Add a compact host/project header and progressively disclosed system details,
without duplicating sidebar Agent/session management. Fixed fixtures must cover
healthy host, connecting/authenticating, SSH-only, missing SFTP/helper/MCP,
permission-denied/unsupported metrics, stale/offline last sample, long host/root,
high utilization and project switching with an old callback. Capture narrow/wide,
light/dark and 125% text; preserve the terminal draft and closed-panel default.
Inspect status labels, wrapping, metric units, sample age and task/header scope
agreement before live integration. Sample metrics stay explicitly labeled as
fixtures. The prior 115 captures do not pass this new checkpoint.


SR41 fixed fixtures now reuse the existing wrapping section renderer, with
producing host/account/root, independent capabilities, native-resource source/age
and system details. Six states cover ready, authentication, SFTP-only, partial
capability/metric failure, stale/offline and long multilingual roots with high
utilization. No live metrics are wired yet. The capture harness includes these
48 native combinations, bringing its expected total to 163; image review and
new GUI behavior remain pending. Existing roster fixtures remain historical and
are removed only after sidebar parity, as required by R6.

Connections/Projects now has a separate native Tools Panel preview using existing
wrapping text, theme, button, scrolling and accessibility components. Seven fixed
states cover empty/authenticating/host verification/ready/failed/reconnecting/
missing OpenSSH. Preview is available only in the isolated debug capture profile,
with no fabricated production connection. Capture matrix adds 56 PNGs (219 total
per OS) and checks the original unsent terminal draft. Exact-source native capture
and visual review remain pending; live profile/connection binding follows this gate.
