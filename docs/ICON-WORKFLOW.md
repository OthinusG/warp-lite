# Icon asset workflow

## Source of truth

Native interface and CLI Agent icons are vector assets in
`app/assets/bundled/svg/`. Prefer an existing repository SVG or an authorized
original vector. When only a raster reference exists, reconstruct simple shapes
with paths, circles and rounded rectangles; trace complex silhouettes into real
Bezier paths and compare the rendered result with the reference.
Use vector authoring/SVG tools for the final icon, rather than raster generation.

SVG must contain actual vector geometry, an explicit `viewBox` and transparent
background. Do not embed a PNG/JPEG, use external URLs, scripts or fonts, or wrap
bitmap data in an `<image>` element. Preserve intentional holes and brand colors.
Toolbar icons reuse native theme tint, active/disabled states and button behavior.
Remove borders/backgrounds only when requested; preserve unrelated brand assets.

## Implementation and verification

1. Record the reference, target entry and acceptance in the relevant UI spec.
2. Create the SVG in `app/assets/bundled/svg/`; use an existing enum when present.
3. Update the central path mapping in `crates/warp_core/src/ui/icons.rs` and any
   direct resource references, tests, packaging lists and technical documents.
   Check all callers, including horizontal and vertical tab layouts.
4. Parse the XML; reject bitmap embedding/external references. Render small and
   large previews, inspect light/dark backgrounds, alignment and clipping, and
   compare complex silhouettes with the original. Previews remain temporary QA
   artifacts rather than bundled product assets.
5. Search the entire tracked repository for every retired PNG filename. Delete
   the PNG only after all active references are migrated. Preserve README/native
   screenshots and application installer artwork that legitimately remain raster.
6. Format affected Rust files without unrelated changes, review the diff and
   commit the SVG assets with their code references. No runtime dependency is
   required for SVG authoring tools.
7. Finish all UI changes before starting the source-matched GitHub desktop and
   remote validation workflows. Review actual native toolbar and Agent icons,
   actions and light/dark/zoom captures before accepting release packages.

## Migration — 2026-10-04

The six migrated assets are `deepseek_harness.svg`, `qoder.svg`, `trae.svg`,
`hermes.svg`, `warpai-settings.svg` and `warpai-tool-panel.svg`. Four Agent logos
retain their colors and silhouettes; settings and Tools panel retain the supplied
terminal/list references without enclosing frames. Simple shapes use hand-authored
geometry; complex logos use Bezier contours from the existing reference alpha.
All are bundled through the existing asset embedding and native SVG renderer.
The corresponding six PNGs are removed after reference and rendering checks.
