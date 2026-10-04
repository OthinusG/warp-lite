# Collaboration UI consistency

## Scope and visual source

Unify the existing collaboration panel and Agent communication settings with
native Tools Panel and settings components. Global Search defines the panel
14px secondary title; settings defines semibold labels and 12px descriptions.
`settings_page::render_body_item` owns setting alignment and description spacing.
Keep existing theme/button variants, editors, events, focus and disabled rules.
No new surface, dependency, protocol or configuration behavior.

## Acceptance

- Panel title, section headings and secondary guidance have distinct text roles.
- Related connection/navigation/form actions wrap at narrow widths.
- Form labels stay adjacent to their editor; warnings retain ordinary contrast.
- MCP controls align with other settings, with separate command/status lines.
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

## Evidence

Pending source-matched native capture and application checks.
