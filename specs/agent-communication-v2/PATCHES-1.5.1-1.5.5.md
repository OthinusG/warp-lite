# Collaboration patches 1.5.1 and 1.5.5

Owner instructions: b.txt, 2026-10-10. The duplicate item 7 is resolved:
items 1–7 (including Message/Assign selectors) belong to 1.5.1; account usage
belongs to 1.5.5. Companion remains 4.0.0 unless its source needs changes.

## Product and visual contract

macOS and Windows native desktop, both Project and Worktree, local and SSH.
Reuse the accepted 1.5.0 native panel, its theme/font tokens, 8px spacing,
native icon assets, menus, settings and scrolling primitives. Changes affect
only the requested surfaces; retain terminal focus, unsent drafts, project
identity, live-run Coordinator ownership and existing task/history contracts.
Static fixture captures gate live integration; review light/dark, 320/420px
panels, narrow windows and 1.25 zoom on both desktop platforms.

## 1.5.1 acceptance and tasks

1. Use the existing Collaborate toolbelt icon with tooltip/accessibility label
   instead of a special text button. Keep its selected-state treatment.
2. Expose management of registered local worktrees through Warp's native
   workspace system, next to creation. Removal addresses the original repository
   and exact registered checkout. Confirm the path; refuse the main checkout,
   active/open checkouts, locked worktrees and dirty/untracked/ignored contents.
   Never force removal or delete branches. Recheck immediately before mutation;
   refresh native catalog after success. SSH deletion is outside this request.
3. Select/change Coordinator through one native dropdown in the main panel.
   Candidates retain exact Agent run identity and checkout context; changing
   selection reuses the existing guarded operation/receipt/error path.
4. Mode heading is `Agents Collaboration Mode`.
5. Separate functional sections with theme-native thin borders/dividers and
   add native leading icons to section headings. Preserve one spacing rhythm.
6. History sits beside Message and opens its own secondary panel in both modes.
   Show separate selectable records with compact pagination; hide main team/
   task sections while viewing History. Back restores the main panel.
7. Message/Assign use dropdowns for enumerated choices (recipient, reviewer and
   selectable prerequisite tasks). Free-form descriptions/criteria remain
   editable. Display names/checkout context but submit original stable identities;
   revalidate scope and eligible live runs at submission. No protocol changes.
8. Show `Rescan agents` and `Remove Warpai MCP from all agents` inside the enabled
   Agent communication section only; hide both when its switch is off.
9. Remove the nonfunctional `App icon` selector and its search terms/actions.
   Preserve native app artwork and the separate Show Warpai in Dock switch.

Affected paths: workspace left panel/actions and worktree catalog;
agent_communication panel/controls and debug fixtures; existing dropdown/menu
and icons APIs; targeted application tests and native capture expectations.

## 1.5.5 acceptance and tasks

Inspect https://github.com/stablyai/orca at a recorded immutable revision and
retain its MIT notices for reused source. Inventory Warpai's supported Agents
against Orca providers. Reuse implemented providers only; investigate Qoder CN
and Antigravity separately. Unsupported sources show unsupported/unavailable,
never fabricated percentages. Provider balances without a known limit must
remain balances rather than an invented percentage.

Add a native settings entry for account setup and which accounts appear.
Provider authentication is explicitly authorized; Warp account/cloud AI,
telemetry and billing remain excluded. Store secrets through the existing OS
credential abstraction; never include secret values in settings, logs, errors,
fixtures, documents or Agent messages. Do not inspect personal credential files
during development. Use bounded requests, explicit provider hosts and synthetic
account fixtures. Do not restore an unrelated account-switching product.

Show a rounded, thin-bordered `Data usage` region at the bottom of the panel,
outside its main scroll area. Its heading stays visible; its own scroll area
shows at most four account rows. Each row has an Agent icon, readable account
label, percentage progress bar and clear unavailable/error state. Use the same
Agent icons in existing MCP communication settings and panel Agent rows.
Interpret the requested fixed position as a pinned footer, with a fixed header
inside that region; no overlay may obscure other controls.

## Verification and delivery boundaries

Before declaring completion: focused pure tests for selector eligibility,
history projection and worktree removal protections; GitHub application tests,
default/warp_platform builds and native fixed/live visual walkthroughs. No local
Rust build or personal application/provider installation. Use the existing
validation workflows and the owner's 30-minute cloud check cadence.

Keep separate source checkpoints for the two patch versions. Never modify
v1.5.0 or reuse its publication work as an acceptance receipt for changed UI.
Owner sequencing correction: validate the combined 1.5.5 implementation directly
instead of running a separate intermediate 1.5.1 walkthrough. Backport fixes to
shared panel behavior into the 1.5.1 checkpoint, then package/publish that exact
checkpoint with the normal release identity/installer gates. Data usage must
remain absent from the 1.5.1 source. Retain the 1.5.5 acceptance receipt separately.
Record provider support, source/license provenance and validation receipts here
and in MEMORY.md. Release/package only accepted version-specific source.
