# Warpai repository cover

## Asset and references

- Final asset: [warpai-social-preview.jpg](warpai-social-preview.jpg), 1280 × 640,
  233,415 bytes, opaque JPEG. Both READMEs use this file.
- Generated with the built-in imagegen tool on 2026-10-07; exported from its
  1774 × 887 PNG with native macOS `sips`, JPEG quality 90, without cropping.
- Identity reference: [approved Warpai icon](../../app/assets/branding/warpai.png).
- Style reference: [official Warp repository](https://github.com/warpdotdev/warp),
  specifically its README cover's blue/violet/teal grain and geometric framing.
  Upstream logos and application screenshots are not included.
- This is a marketing image, not a native application capture. Existing app
  icons and README screenshots are unchanged.
- Copy revision: the supporting line is now **Agent Management · Project Agent
  Communication**, emphasizing AI agent management and communication between
  agents rather than SSH or human team collaboration. The established visual
  style, icon, wordmark and headline are retained.

## Acceptance

Inspected the exported cover at 1280 × 640 and 640 × 320:

- Layout: left icon and right copy remain inside the safe margin.
- Identity: black tile, asymmetric white panels, six links, left slots and
  right terminal glyph remain recognizable; no crop or stretch.
- Typography: Warpai and both copy lines are spelled correctly and readable.
- Palette: blue/violet/teal gradient and fine grain follow the reference.
- Spacing: copy does not overlap the icon or frame; no clipped primary content.
- Intentional deviation: standalone icon and product copy replace Warp's
  screenshot composition; no simulated application UI is introduced.

## Social preview upload

The README reference and GitHub Social preview are separate settings. Current
agent tools can publish repository files but cannot perform the browser upload.
Open [repository Settings](https://github.com/OthinusG/warpai/settings), find
**Social preview → Edit → Upload an image**, and select the final JPEG above.
GitHub recommends 1280 × 640 and requires a file under 1 MB; this export meets
both. See [GitHub's official instructions](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview).

## Final text-edit prompt

```text
Edit this existing Warpai cover, changing ONLY the small supporting line at the bottom of the right text column. Replace "Agent management · Agent-to-agent communication" with EXACTLY "Agent Management · Project Agent Communication". Keep wordmark "Warpai", main two-line headline "The terminal built / around agents.", the approved notebook/terminal black icon with six binding links, its size and placement, blue-violet-teal grainy gradient, geometric framing, margins, colors and typography unchanged. Keep the replacement text on one line in the same location, readable light gray sans serif with sufficient right margin; adjust only this line's font size as needed. No clipping, no additional elements, no visual redesign. Output opaque landscape 2:1, ideally 1280x640 pixels.
```

## Original generation prompt

```text
Use case: ads-marketing / compositing.
Create a finished GitHub repository Social preview image, landscape exactly 1280 x 640 pixels, solid opaque background. It will also replace the small logo at the top of the Warpai README.
Reference image 1 (/Users/wqin/workplace/warpai/app/assets/branding/warpai.png) is the approved Warpai product icon. Preserve the recognizable black rounded square, asymmetric pearl-white bound notebook panels, exactly six binding links, two vertical slots on the left and terminal >_ glyph on the right. Do not redesign the mark.
Reference image 2 (/tmp/warpai-social-preview/warp-readme-reference.png) is STYLE ONLY: use its premium blue/purple/teal grainy gradient aesthetic and understated geometric framing, but do not include its Warp logo, wordmark, screenshot, or any interface.
Composition: spacious editorial horizontal brand cover. Icon on the left at about 330px tall, centered vertically, without stretching or cropping. Right half: large crisp white sans-serif wordmark "Warpai", below it two-line white tagline "The terminal built" / "around agents." Below this a smaller readable line "Local + SSH · Your CLI agents". Keep all content within a 70px safe margin. Blue-violet edges and a restrained muted teal center glow, very fine film grain, subtle thin framing lines. Strong contrast, professional and calm. No mock terminal UI, no invented product screenshots, no badges, no extra text, no upstream branding, no sparks or bright outer ring on the icon. Exact typography spelling: Warpai (capital W, lowercase arp ai). This is the actual final cover, not a mockup sitting on a desk.
```
