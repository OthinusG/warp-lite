# Warpai branding

`warpai.png` is the owner's selected bound-page/terminal design, edited with the
built-in imagegen tool. Final directions: retain the white asymmetric panels,
six links, left slots and right terminal glyph; use a black liquid-material tile;
remove its outer ring and exterior sparkle; preserve transparent margins.

`warpai.icns` and `warpai.ico` are format/size exports of the same master, consumed
by macOS and Windows packaging. The master is 1254 × 1254 RGBA. See the
[production workflow](../../../docs/ICON-WORKFLOW.md) and
[acceptance contract](../../../specs/APP-ICON.md).

The material research used the
[Blender Liquid Glass proposal](https://devtalk.blender.org/t/proposal-liquid-glass-app-icon-for-macos-tahoe/44113)
as a reference. No Blender asset or mark is distributed. This is baked artwork,
not a native animated Liquid Glass implementation.

The native monochrome fallback uses real geometry in
`app/assets/bundled/svg/warpai-mark.svg`, with the full-red opacity-mask convention.
Welcome/About surfaces use the full-color master rather than tinting it.

`installer-banner.png` is the imagegen composition master: the prior installer
background and lowercase wordmark are retained while its flat logo is replaced
with the approved icon. The Windows wizard exports remain 58 × 58 (header) and
202 × 386 (sidebar), RGB BMP. The inherited macOS DMG entry no longer uses the
retired upstream marketing background.

## Companion connection-tool identity

`warpai-companion.png` is the installer variant: the approved black Warpai tile
and silver notebook/terminal mark with the owner's opposing curved-arrow
reference fused into its lower-right connection emblem. The built-in imagegen
edit preserved the product identity, six binding links, transparent margin and
rimless material. The ICO/ICNS exports downsample this unchanged master.

The prompt requested one square RGBA installer icon, retaining the approved
Warpai geometry/material and adding silver/white opposing arrows without text,
new colors, an outer ring or sparkles. Native exports and small-size light/dark
previews were inspected. macOS DMG/Finder and Windows installation verification
remain release gates. Desktop Warpai branding stays separate.
