# Collaboration panel static checkpoint

Date: 2026-10-02. Status: static native checkpoint accepted; live integration now permitted.

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
