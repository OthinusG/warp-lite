# Superseded Native Remote Routing Plan

Date: 2026-10-03. Status: retired implementation direction. The former plan routed
MCP/wake between participating native Warpai applications using device enrollment
and coordinator grants. Do not implement its remaining steps. Its source
checkpoint and previous instructions are preserved in
[the archived plan](legacy-machine-collaboration/NATIVE-REMOTE-ROUTING.md).

The current product is one local Warpai GUI managing SSH remote project execution,
with SFTP and a small remote companion rather than a second desktop application.
Use [PLAN.md](PLAN.md), [TECH.md](TECH.md), [API.md](API.md) and
[CUTOVER.md](CUTOVER.md), especially D01–D09/D15. Existing guarded delivery and
request-reconciliation invariants are reused under new project/run authority;
old enrollment/profile/spool APIs must not be wired into the new GUI as-is.

Production device federation remains disabled. This document update does not
remove its code or erase old unknown tasks/intents.
