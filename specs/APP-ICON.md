# Warpai app icon

## Product and visual contract

Use the owner's selected bound-page/terminal icon, with the final correction on
2026-10-05: change the gray tile to black, preserving the white asymmetric pages,
six links, two left slots and right `>_` glyph. Remove the outer reflective ring as subsequently requested. Keep the softened liquid material,
transparent exterior and restrained highlights; no exterior glitter or shadow.
The owner-selected geometry is authoritative. Blender's Liquid Glass proposal
is a material reference only; no Blender artwork is distributed.

## Implementation

- Store the built-in imagegen edit as the canonical RGBA master in
  `app/assets/branding/warpai.png`.
- Derive native ICNS and multiresolution ICO resources from that master. These
  raster OS resources intentionally differ from theme-tinted toolbar SVG masks.
  The appearance is baked for consistent macOS/Windows branding, not a promise
  of animated Icon Composer materials.
- Use the same resources for bundles, Dock plugin, Windows executable/installer,
  Cargo packaging metadata and bilingual README. Retain existing Linux icon
  paths as compatibility assets without adding Linux desktop work.
- Preserve serialized legacy AppIcon values, but resolve them to Warpai and show
  only the current icon in the picker. Remove superseded artwork after updating
  all consumers; retain copyright and provenance notices.

## Acceptance

Inspect shape, six links, terminal/face glyphs, black palette, transparent margin
and readability on light/dark surfaces at large and small sizes. Decode every
ICO/ICNS size and check dimensions/alpha. Audit packaging and legacy-icon
references, validate shell/Objective-C/Rust syntax as available, and run relevant
desktop checks on GitHub. The owner subsequently authorized Warpai 1.0.1
publication after acceptance; follow `specs/RELEASE.md` for the release gate.

## Static fidelity ledger

| Point | Reference / final result |
| --- | --- |
| Identity | Owner-selected two asymmetric panels, six links, two slots and `>_` retained |
| Base | Requested black/deep charcoal replaces the rejected pale-gray base |
| Edge | Outer reflective ring removed; rounded-square silhouette retained |
| Material | Soft liquid surface; white panels retain the approved treatment |
| Framing | Whole icon centered with transparent margins, no exterior cast shadow |
| Small sizes | ICO/ICNS exports retain the same composition and readable white symbol |

Image generation used the built-in imagegen edit flow. PNG master is preserved
without pixel cleanup/recoloring outside that tool. Native resource export only
resizes and encodes. Light/dark alpha-composited previews and native file decoding
verify actual display rather than hidden RGB in transparent margins.

Installer-specific review also found a legacy header bitmap and flat sidebar
logo. Both now use the approved rimless icon; the sidebar preserves the existing
wordmark/background. RGB BMP dimensions remain 58 × 58 and 202 × 386. Removed
the inherited macOS DMG marketing background. These packaging-artwork edits do
not change application, UI asset embedding or remote protocol inputs.

## Native acceptance — 2026-10-05

Source `5d2588a86bbdc05cc994ad816561c4626be9aac4` passed
[desktop run 37268540992](https://github.com/OthinusG/warpai/actions/runs/37268540992)
and [remote run 37268449581](https://github.com/OthinusG/warpai/actions/runs/37268449581).
Both desktop jobs passed default/warp_platform checks and focused regressions.
Each produced 161 decoded PNGs, source-matched diagnostics, exit code 0 and no
failed actions. Downloaded archives passed GitHub SHA-256 and ZIP CRC checks.
Native review covered warm vertical-tab live workspace/SSH, warm settings at
125%, narrow Light/Dark panels, toolbar masks and Windows native equivalents.
No overlapping primary controls were found; long content retains native scroll.
README images now use unmodified macOS frames from this run.
