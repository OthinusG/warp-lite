# Native remote routing implementation

Status: implementation in progress; production participation is not enabled.
Scope: M6.3–M6.7. PRODUCT.md, TECH.md and API.md remain authoritative.

## Delivery order and acceptance

1. Store credentials through the existing SecureStorage provider. Namespace keys
   by expected coordinator and enrolled device UUIDs. Reject overwrites, locked
   storage, no-op providers and failed readback. Persist only reviewed aliases,
   coordinator/device identities, grants and workspace mappings in preferences.
   Native enrollment uses a password editor, clears its input after transmission,
   and never logs credential-bearing errors. Check successful readback, failure,
   missing/locked storage and preservation of another device's key.
2. Add native Devices/Connections settings using existing editors, switches and
   button themes. Explicit hosting owns RunningController and stops it before
   RunningBroker. Participant enrollment verifies expected coordinator before
   sending an invitation. Validate static narrow/wide/light/dark fixtures before
   live integration. Unavailable system SSH and host prerequisites are visible.
3. Add reviewed remote admission alongside existing local shared admission.
   Bind canonical local checkout, remote workspace, coordinator/device/space,
   native session and current verified CLI run at app-controlled boundaries.
   Default/restored panes remain private; MCP cannot select these identities.
4. Reuse Connection and the coordinator Store engine for admitted remote MCP.
   Stage original mutations in private SQLite before network transmission; release
   the broker mutex during I/O. Recheck terminal capability, participation and run
   before accepting callbacks. Announce on reconnect to fence old connections,
   then reconcile original receipts without replacing request/run identities.
   Participant read projections never become authoritative task copies.
5. Route notifications naming only actor/run/message through the existing native
   wake path. Recheck readiness, unsent draft, approval, manual pause, process
   replacement and delayed Enter locally. Presence heartbeat alone grants no idle
   authority. Retain delivered-message identities during same-run reconnect.
6. Normalize participant paths using existing canonical checkout checks. Protect
   local/private/shared reservations atomically with remote pending intents before
   sending FileReserve. Retain unresolved provisional guards; an absent receipt
   is meaningful only behind the connection replacement fence. Remote evidence
   remains metadata unless opened from its authenticated producing local checkout.
7. Exercise isolated coordinator/participant apps on both GitHub OS runners,
   including dropped results, replay, reconnect, old frames, sleep, restart,
   revocation, stale callbacks and draft-safe wake. Rebuild exact final source,
   review native screenshots and record each PLAN package separately.

Existing retained capture drivers and private IPC/stdio fixtures provide
credential-free deterministic checks. They do not establish physical SSH across
macOS/Windows or real vendor model acceptance; those require unavailable user
runtime resources. Do not enable incomplete production routing to bypass a gate.
