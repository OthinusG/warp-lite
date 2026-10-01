# Collaboration panel static checkpoint

Date: 2026-10-02. Status: native fixture implemented; screenshot acceptance pending.

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
