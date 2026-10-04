# Fork Notice — Warpai

**Warpai** derives from [terzigolu/warp-lite](https://github.com/terzigolu/warp-lite) and [warpdotdev/warp](https://github.com/warpdotdev/warp), the open-source Warp Terminal released by Denver Technologies, Inc. in 2026.

## Provenance

- **Upstream:** https://github.com/warpdotdev/warp
- **License (preserved):** AGPL-3.0-only (with the original `warpui` and `warpui_core` crates remaining MIT, as in upstream)
- **Original copyright:** Copyright (C) 2020–2026 Denver Technologies, Inc.
- **Fork copyright on modifications:** © 2026 warp-lite contributors. AGPL-3.0-only applies to all modifications.

This fork is published under AGPL-3.0-only as required by the upstream license. The fork **cannot** be relicensed (only Denver Technologies, the original copyright holder, may do that); modifications introduced by this fork are also AGPL-3.0-only.

## What was removed

Warpai excludes bundled cloud AI, cloud accounts, billing and telemetry product surfaces. The terminal, editor, third-party CLI agent management, native MCP configuration and local/SSH project collaboration remain supported. Shared source needed by these features and stored-data compatibility is retained. The [cleanup record](specs/DEEP-CLEANUP.md) distinguishes source deletion from disabled product surfaces.

The inherited fork's [commit history](https://github.com/terzigolu/warp-lite/commits/warp-lite/main) records earlier removal phases. Those historical changes do not describe every feature or source dependency of current Warpai.

## Independent maintenance

Warpai is independently maintained on `main`, with development changes on feature branches. Automatic upstream synchronization and patch replay were retired on 2026-10-01; source, tests and release packaging are maintained directly. Git history preserves provenance. Independent maintenance does not change the inherited copyright, licensing or derivative-work obligations, and does not by itself change GitHub's fork-network metadata.

## AGPL §13 disclosure

If this fork is offered over a network (e.g., remote pair-programming, hosted shell access), AGPL §13 obligates the operator to make the corresponding source code available to all interacting users. The canonical source is this repository.

## Trademarks

"Warp" is a trademark of Denver Technologies, Inc. Warpai uses the Warp source code under AGPL but **does not** claim affiliation, endorsement, or sponsorship by Denver Technologies. The independently maintained product is named *Warpai*; historical upstream names remain in attribution and compatibility records.

If the upstream rights-holders ask for a name change, we will rename promptly.
