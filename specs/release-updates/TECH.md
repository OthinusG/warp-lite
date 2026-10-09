# About release updates

Date: 2026-10-09. Status: implementation and native verification accepted.

## Behavior and scope

- Add an Update button and a Check for updates on startup switch to the native
  About page. Reuse native Button/Switch, settings persistence and workspace toast.
- Check only OthinusG/warpai public stable GitHub Releases. Do not enable upstream
  Warp cloud updating, accounts, telemetry, polling or background installation.
- Manual Update checks the published version, reports current/failure status, and
  shows a separate Download button for the current OS and architecture. Download
  opens the matching published installer directly in the browser. Update always
  checks again. Unsupported platforms or missing installers show a Releases
  fallback instead of offering the wrong package. A startup check runs
  once when enabled and notifies only when a newer version exists.
- Startup checking defaults off to preserve local-first startup. The local setting
  survives restarts; disabling it before a queued startup check cancels that check.
- Preserve installed files and running terminals. Users install the selected
  desktop release through its existing macOS/Windows packaging instructions.

## Implementation and constraints

Use one application model shared by About and startup. Use existing reqwest and
async-compat; apply a timeout, bounded response and fixed HTTPS endpoint. Accept
only non-draft, non-prerelease vMAJOR.MINOR.PATCH tags; match installer names in
the release assets and construct fixed-repository download URLs from validated
tags rather than trusting returned URLs. Current packages are macOS arm64 DMG
and Windows x64 EXE; do not select Companion, archive or checksum assets. Compare against the
packaged application version. Keep status text wrapping in the existing About
layout and disable duplicate checks. Register About typed actions and observe
the update/settings models.

## Verification

- Version checks cover newer/equal/older, invalid tags and prerelease/draft data.
- Installer selection covers both desktop targets, unsupported architectures,
  missing installers and Companion-only releases.
- Check the local settings registration, startup gate and About action routing.
- Cloud checks compile both desktop targets and default/warp_platform builds.
- Native captures cover About status and startup switch alongside collaboration
  narrow/light/dark fixtures; accept only source-matched successful diagnostics.
- At 800px/1.25 zoom, the startup label shrinks/wraps beside its native switch;
  attribution wraps within the page. Clicks on the label or switch toggle the
  same saved preference. Screenshot QA must verify both controls remain visible.

## Acceptance receipts

- Both desktop application unit suites, including version and installer selection,
  pass at 27a9f992 in run 37939850502. Its Windows capture fails only the old
  detail-scrolling viewport; subsequent changes affect that native fixture only.
- Source 46248e08/[run 37943970413](https://github.com/OthinusG/warpai/actions/runs/37943970413)
  passes both default/platform builds and both complete native walkthroughs, with
  214 valid PNGs and successful source-matched diagnostics per OS.
- Reviewed all six About states at both widths/themes on macOS and Windows;
  current-source narrow/wide checks confirm the same accepted production layout.
  Startup preference enable/disable assertions pass. Fixtures avoid real update
  downloads and public release changes. No release or personal installation.
