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

For monochrome assets rendered by `warpui_core::elements::Icon`, use full red
(`#FF0000`) for opaque geometry and transparent empty space, matching existing
native icons. Both Metal and WGPU use the texture's red channel as the opacity
mask, then supply the theme foreground color. Gray/blue/black reference colors
reduce or erase the rendered icon; they are not the visible toolbar palette.
Keep brand colors only for assets rendered as ordinary images/SVGs. Inspect the
actual consumer before choosing its fill/stroke colors.

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

## Application branding — 2026-10-05

The owner selected the bound-page/terminal design, then requested a black base
and removal of the outer reflective ring. The canonical full-color RGBA master
is `app/assets/branding/warpai.png`. Application artwork is intentionally raster:
macOS ICNS and Windows ICO need multiresolution bitmap resources. Do not convert
this shaded image into a bitmap wrapped in SVG or use the red-channel toolbar
mask convention for it.

The built-in imagegen tool performed the material/color/rim edits. Preserve the
master bytes and alpha; only downsample/encode when exporting native formats.
Inspect alpha-composited QA previews on real light/dark backgrounds: RGB values
hidden beneath zero/near-zero alpha can look misleading in a raw preview.

Export with an existing development Python/Pillow installation:

```python
from pathlib import Path
import subprocess
import tempfile
from PIL import Image

master = Image.open("app/assets/branding/warpai.png").convert("RGBA")
master.save("app/assets/branding/warpai.ico", format="ICO",
            sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
with tempfile.TemporaryDirectory() as temp:
    iconset = Path(temp) / "Warpai.iconset"
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            suffix = "@2x" if scale == 2 else ""
            master.resize((size * scale, size * scale), Image.Resampling.LANCZOS).save(
                iconset / f"icon_{size}x{size}{suffix}.png")
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o",
                    "app/assets/branding/warpai.icns"], check=True)
for path in Path("app/channels").glob("*/icon/no-padding/*.png"):
    size = int(path.stem.split("x")[0])
    master.resize((size, size), Image.Resampling.LANCZOS).save(path)
```

The retained channel PNGs serve inherited Linux paths; this does not add Linux
desktop support. macOS packaging and the legacy channel bundle entry point copy
the shared ICNS; the Dock plugin copies the master. Windows resource embedding,
installer, portable app and Cargo metadata use the shared branding directory.
Keep legacy serialized AppIcon variants readable while resolving all to Warpai.
The desktop release DMG uses the same ICNS as its Finder volume icon and provides
an Applications shortcut. Preserve bundle metadata with native ditto when staging
the image; verify the mounted bundle signature and icon after packaging.
Both macOS packagers reuse `script/macos/create-dmg.sh` to set the custom icon
flag on the actual writable image volume before compression. A source-folder
flag alone is insufficient; inspect the compressed image's mounted volume.
Retired Dock PNG variants, per-channel ICOs and Icon Composer artwork are removed
after consumer migration. Validate every ICO/ICNS size, alpha, native icon decoding,
packaging paths, light/dark small-size previews and source-matched desktop checks.

The Windows wizard header is a 58px RGB BMP export of the icon, alpha-composited
on its native white surface; the 202 × 386 RGB sidebar BMP derives from
`app/assets/branding/installer-banner.png`. Its built-in imagegen edit preserves
the prior background/wordmark and replaces only the icon. Keep both BMP paths in
windows-installer.iss synchronized. No upstream marketing background is used
by the inherited macOS DMG entry.

## Companion installer variant — 1.1.0

The owner requested a connection-tool variant using the existing Warpai product
icon and supplied opposing curved arrows. The built-in imagegen edit creates
`app/assets/branding/warpai-companion.png`; ICNS/ICO exports use the unchanged
1254px RGBA master. Do not replace the desktop mark or use a toolbar opacity mask
for full-color installer artwork. Check transparent edges and small sizes on
light/dark backgrounds before wiring consumers.

Windows setup/uninstall use the companion ICO. macOS image volume and install
command use the companion ICNS (Finder custom resource fork retained by hdiutil).
Linux's self-contained CLI installer includes the master; there is no new Linux
desktop launcher. All variants install the same runtime at the stable account
location. CI verifies the native installer resources, installation and component
integrity before release. Keep artwork and consumer paths in the same source.
