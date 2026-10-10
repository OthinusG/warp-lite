# Configuration migration and interface localization

## Existing implementation

- warp_core::paths owns the desktop managed root, profiles and channels.
- app::user_data_migration imports missing legacy files without overwriting either
  destination data or recovery sources, rejects destination symlinks and marks
  completion after successful writes. Extend this path; do not create a manager.
- Public settings use settings.toml; legacy JSON and private preferences remain
  separate. define_settings_group!/Setting supplies typed defaults and Reset.
- Native UI strings are currently embedded in Rust. No existing locale catalog
  was found. Native Text/widgets must continue rendering user content verbatim.
- Companion/WSL installers and application discovery hardcode their managed paths.
  Inventory these references before changing the desktop root; preserve support
  for installed 4.0.0 companions and avoid silently breaking remote discovery.

## Tasks and verification

1. Inventory managed-root callers, installers, scripts, fixtures and persisted
   executable paths. Add a separate root-migration completion boundary before
   existing legacy import so an imported old marker cannot suppress migration.
   Preserve channels/profiles, permissions, conflict precedence and recovery data.
   Never execute this migration against the owner's files during development.
2. Read only confirmed nonsecret local UI/terminal preferences. Build a reviewed
   public preference baseline using existing SettingsValue conversion. Exclude
   private/account/path/server/history data, reject invalid typed values, and make
   normal Reset and fresh installs agree. Do not overwrite existing preferences.
3. Add a persisted typed language setting through existing settings registration
   and the native dropdown. Apply it at startup, requiring restart so cached menus
   and active drafts stay consistent. Keep locale identity independent of labels.
4. Establish explicit UI text catalogs for en, zh-Hans and zh-Hant. Mark display
   boundaries rather than translating arbitrary Text/terminal/editor content.
   Translate dynamic templates before interpolation; verify identical arguments.
   Inventory reachable screens and shared controls, including menus, errors,
   tooltips, notifications, accessibility and native platform menus.
5. Add source/catalog coverage checks and focused migration/default/template tests.
   Local verification is syntax/script/catalog only; Rust and runtime checks run
   on GitHub after 1.6.0 acceptance, with all three locales and representative
   narrow/high-zoom/light/dark screenshots. Preserve existing SSH/WSL assertions.
6. Incorporate later 1.6.0 fixes by their existing commits, then complete full
   source-matched acceptance and existing immutable-tag release gates for 1.6.2.

## Risks

Directory migration must not overwrite databases, partial writes or explicit new
preferences. A simultaneous older app must not create divergent active stores.
Symlinks and stale absolute MCP/Companion paths require explicit handling.
Local preferences may include platform-only fonts/options; retain existing safe
fallbacks. Chinese text can widen rows or change keyboard/search behavior. English
runtime fallback is not evidence that Chinese interface coverage is complete.
No new cloud translation runtime, telemetry, billing or bundled Agent capability.

## Implementation progress

The owner baseline contains 49 registered public preferences. Theme defaults use
built-in Claude Warm Light, Catppuccin Latte and Dark; the owner explicitly chose
Dark because the former LLM Dark file is unavailable. Saved explicit themes remain
unchanged. No machine-specific theme paths are bundled.

The explicit interface catalog currently covers marked static labels and selected
named templates. script/check-interface-locales.py validates catalog arguments and
common direct display boundaries, excluding reviewed brand/technical names and
terminal preview content. Passing this checker does not establish complete UI
coverage: indirect constants, optional help text, dynamic actions/statuses, search
terms and accessibility labels still require review and native locale walkthroughs.
The 1.6.2 workflow runs the checker and sets desktop version metadata to 1.6.2;
cloud execution remains gated on successful 1.6.0 acceptance.
