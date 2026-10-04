# Collaboration UI consistency

## Scope and visual source

Unify the existing collaboration panel and Agent communication settings with
native Tools Panel and settings components. Global Search supplies the panel typography and wrapping patterns; the revised
panel title is 14px semibold primary. Settings defines semibold labels and 12px descriptions.
`settings_page::render_body_item` owns setting alignment and description spacing.
Keep existing theme/button variants, editors, events, focus and disabled rules.
No new surface, dependency, protocol or configuration behavior.

## Acceptance

- Panel title, section headings and secondary guidance have distinct text roles;
  the 14px semibold primary title outranks section headings (revised 2026-10-04).
- Related connection/navigation/form actions wrap at narrow widths. The SSH entry
  point is a Secondary button; refresh shares its row; state and guidance share one line.
- Form labels stay adjacent to their editor. Settings legacy-cleanup warnings use the
  theme warning color because they require user action (supersedes ordinary contrast).
- MCP controls align with other settings under an "Agents" sub-header, with status
  text matching 12px row descriptions. Panel spacing uses shared GAP_* constants.
- Existing native static/light/dark/zoom and live action captures pass on both OSes.
- README images use unmodified native Claude Warm Light captures. Settings captures
  also include this default palette; standard Light and Dark remain covered.
- Default and `warp_platform` application checks run on GitHub; no local Rust build.

## Tasks and verification

1. Reuse existing text roles and wrapping layout in the collaboration panel.
2. Reuse the shared setting row for MCP enablement and agent selection.
3. Rust formatting and focused review for unchanged callbacks/disabled states.
4. Run the existing source-matched desktop workflow with native captures.
5. Inspect narrow/wide light/dark forms and screenshots; record evidence below.
6. Shared post-step capture retries up to three fresh frame requests when a frame
   times out or its callback is dropped. Retry only capture, never the preceding
   UI action. Retain the nonempty/exact-count PNG gate; an exhausted capture must
   still fail acceptance. Verify using both native desktop capture suites.

## Evidence

- Full regression and review packaging:
  [run 37180907136](https://github.com/OthinusG/warpai/actions/runs/37180907136),
  source `33e717d26e18d60745c5cc87a73d828d38616e44`, passed on macOS and Windows.
- Final source `3b315664131933837c735896239ae750f3c674c1`:
  [run 37188935250](https://github.com/OthinusG/warpai/actions/runs/37188935250),
  passed on both OSes. Default and `warp_platform` checks, protocol/persistence/
  native socket/MCP tests, and native action assertions passed. Each OS captured
  161 nonempty PNGs with exit code 0 and no failed action assertions.
- Earlier source `a5699b9`,
  [run 37184297938](https://github.com/OthinusG/warpai/actions/runs/37184297938),
  reproduced Windows missing-frame failures twice, for different static PNGs.
  Shared capture previously discarded a single failed request. The final driver
  retries fresh frames up to three times without replaying UI actions; the exact
  image-count gate remains unchanged and now passes on both OSes.
- Earlier visual review confirmed 14px secondary panel titles, semibold section labels,
  12px secondary guidance, wrapping navigation/pagination/form actions and 4px
  label/editor spacing. Warnings retain ordinary contrast. Dynamic agent actions
  retain their bounded layout for long names. The title/warning roles in this
  historical review are superseded by the revised Acceptance above.
- Native settings rows align descriptions and controls with the existing Features
  page. The footer action is 12px below status. Light, Dark and Claude Warm Light
  settings at 100%/125%, and narrow 320px panel forms/tasks, remain readable with
  no overlapping controls; long content uses the existing scroll container.
- README images are byte-identical to the final macOS native warm captures.
  [English provenance](../../docs/images/README.md) and
  [Chinese provenance](../../docs/images/README.zh-CN.md) identify the source,
  sample data and original filenames. Both README render checks and all 91 local
  links/fragments in the bilingual documentation passed.

## Final repair checkpoint — 2026-10-04

Finish UI before final screenshots and packages, as explicitly corrected by the
user. Preserve the existing 14px primary title, Secondary SSH entry, wrapping
actions, shared GAP_* spacing, Agents sub-header and warning-color cleanup text.
Use the existing secondary note role for original reservation metadata and archive
guidance. Match MCP status colors to native settings descriptions using surface_1;
the explicit warning-color override remains unchanged. Callbacks, form values,
focus and disabled/submitting rules do not change. Final native review is pending.

| Visual point | Native reference / repair | Verification |
| --- | --- | --- |
| Title hierarchy | 14px primary semibold title above 12px section text | Static and live panel captures |
| Guidance | Existing secondary note role for archive and reservation metadata | Reservation confirmation capture and focused source review |
| Settings palette | surface_1 background for secondary status; theme warning override | Light/Dark/Claude Warm settings captures |
| Spacing | GAP_TIGHT/ROW/SECTION; 4px label/editor separation | Narrow forms and zoom captures |
| Actions | Existing button themes and wrapping rows | Native action assertions and narrow panels |
| Documentation | Raw macOS warm images with vertical tabs | Source/digest/provenance and full image review |
