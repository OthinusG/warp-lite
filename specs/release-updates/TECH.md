# About release updates

Date: 2026-10-09. Status: implementation and verification pending.

## Behavior and scope

- Add an Update button and a Check for updates on startup switch to the native
  About page. Reuse native Button/Switch, settings persistence and workspace toast.
- Check only OthinusG/warpai public stable GitHub Releases. Do not enable upstream
  Warp cloud updating, accounts, telemetry, polling or background installation.
- Manual Update checks the published version, reports current/failure status, and
  opens the available release page on the next Update click. A startup check runs
  once when enabled and notifies only when a newer version exists.
- Startup checking defaults off to preserve local-first startup. The local setting
  survives restarts; disabling it before a queued startup check cancels that check.
- Preserve installed files and running terminals. Users install the selected
  desktop release through its existing macOS/Windows packaging instructions.

## Implementation and constraints

Use one application model shared by About and startup. Use existing reqwest and
async-compat; apply a timeout, bounded response and fixed HTTPS endpoint. Accept
only non-draft, non-prerelease vMAJOR.MINOR.PATCH tags; construct download-page URLs
from validated tags rather than trusting a returned URL. Compare against the
packaged application version. Keep status text wrapping in the existing About
layout and disable duplicate checks. Register About typed actions and observe
the update/settings models.

## Verification

- Version checks cover newer/equal/older, invalid tags and prerelease/draft data.
- Check the local settings registration, startup gate and About action routing.
- Cloud checks compile both desktop targets and default/warp_platform builds.
- Native captures cover About status and startup switch alongside collaboration
  narrow/light/dark fixtures; accept only source-matched successful diagnostics.
