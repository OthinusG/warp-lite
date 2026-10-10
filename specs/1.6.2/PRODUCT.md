# Warpai 1.6.2

Authorized 2026-10-11. Develop independently while 1.6.0 is validated/released.
Start 1.6.2 cloud acceptance only after 1.6.0 acceptance passes. Apply every later
1.6.0 correction to this branch before its acceptance; do not duplicate fixes.

## User outcomes

1. Desktop configuration and durable data use ~/.config/warpai instead of
   ~/.config/.warpai. Upgrades preserve preferences, themes, keybindings, sessions,
   collaboration history and recovery sources. Existing new-root data wins;
   repeated startup does not restore intentionally reset preferences.
2. Fresh-install and Reset defaults reflect the owner's current local UI and
   terminal preferences, as explicitly confirmed. Exclude accounts, credentials,
   project paths/history, MCP servers and other personal data. Preserve supported
   platform differences and existing users' explicit values.
3. Settings adds a native Language dropdown: English, 简体中文, 繁體中文. Persist
   the selection and apply it throughout the interface after restart, with the
   restart requirement shown beside the dropdown. English remains available.
4. Translate every reachable native page, menu, option, button, label, status,
   tooltip, placeholder, confirmation, error and accessibility label. Keep terminal
   output, commands, user text, file contents, paths, vendor/product names and
   technical identifiers intact. Documentation translation is outside this work.

## UI reference and acceptance

Reuse the native settings rows, dropdowns, buttons, layout and theme tokens.
No external visual mock was supplied; the existing native UI is the visual source.
Preserve focus, drafts, ownership, content and terminal behavior on language change.
Check Chinese font fallback, keyboard navigation, screen-reader labels, long text,
narrow panes, high zoom, light/dark themes and both desktop platforms.

Completion requires safe repeatable root migration tests, typed defaults checks,
translation coverage with no unclassified reachable UI strings, template argument
parity, both build configurations and real source-matched native walkthroughs in
all three languages. Inspect screenshots, including SSH/WSL files and Review.
Package and publish 1.6.2 only after these gates pass. Keep Companion 4.0.0 if its
runtime/package contract is unchanged; document any required component change
before release. No personal installation or live vendor credentials in acceptance.
