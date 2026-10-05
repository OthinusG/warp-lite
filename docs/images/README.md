# README screenshot provenance

**English** | [简体中文](README.zh-CN.md) · [Product introduction](../../README.md)

These are unmodified PNG captures from the native macOS application, not mockups
or generated product images. Sample projects, agents and tasks were exercised by
the native review driver; they are not customer projects or authenticated vendor
model sessions.

- Tabs: native macOS vertical tab panel enabled.
- Theme: default **Claude Warm Light**, rendered by the native app.
- Source: `b41d07ba2896e8f3c755c9227ac3483d873c4e2b`.
- [GitHub desktop run 37259523352](https://github.com/OthinusG/warpai/actions/runs/37259523352).
- Artifact: `Warpai-collaboration-native-captures-macOS`.
- Capture set: `capture-1194/collaboration-static/2026-10-05T03-55-06`.

| Documentation image | Original capture | Shows |
| --- | --- | --- |
| [warpai-workspace.png](warpai-workspace.png) | `live-evidence-file-open.png` | Source editor and a live task detail panel. |
| [warpai-ssh-project.png](warpai-ssh-project.png) | `live-ssh-task.png` | Remote project, agent and task projections in the live SSH panel. |

## Visual review

Compared the raw captures with [the native UI contract](../../specs/agent-communication-v2/UI-CONSISTENCY.md).
The earlier `5883221` frames were rejected for faint toolbar icons; the corrected
source uses full-red opacity masks and native theme colors. The downloaded
macOS archive passed GitHub SHA-256 and ZIP CRC checks, all 161 PNGs decoded, and
its source-matched diagnostic reports exit code 0 with no failed native actions.

| Point | Expected / reviewed result |
| --- | --- |
| Palette | Native Claude Warm Light workspace and SSH frames; no recoloring. |
| Layout | Visible macOS vertical tabs beside the source/terminal and task panel; native scrolling retains long content. |
| Toolbar assets | Clear terminal Settings glyph and four-row Tools glyph, without the supplied enclosing ring/frame; Light/Dark and both tab layouts reviewed. |
| Typography | Primary semibold panel title remains distinct from section headings and secondary guidance. |
| Actions and spacing | Secondary SSH entry, adjacent refresh and wrapping navigation in the 320px/125% panel; no overlapping primary controls. |
| Settings | Agents sub-header, aligned native setting rows and readable descriptions in Claude Warm at 100%/125%, plus Light/Dark at 125%. |

Both documentation PNGs are byte-identical to their original captures at
1440 × 684 pixels. The screenshot-only review build is not a published installer.

These captures predate the owner-supplied app icon update. The README now uses
[the canonical Warpai artwork](../../app/assets/branding/warpai.png); no Dock or
installer branding is claimed by these workspace-only frames.
