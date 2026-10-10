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

The owner baseline contains 51 registered public preferences. Theme defaults use
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

Native captures now accept explicit language codes through the existing SSH/file
acceptance driver. Each native process loads its language from a newly created
public TOML file in its own protected capture profile before interface creation;
a startup assertion verifies the stored and active language agree. The workflow
runs English, Simplified Chinese and Traditional Chinese sequentially against the
same owned SSH fixture, retaining separate process/profile artifacts and the
existing WSL cases. No production language override or personal installation is
introduced. These runtime checks have not run yet; the 1.6.0 gate still applies.

Focused cloud checks cover reordered confirmation arguments, preserved literal
user braces, typed public owner defaults and bilingual settings search. Native
capture setup persists each locale before application initialization and asserts
that both the typed setting and active catalog match. Dropdown labels used for
selection share the same translation path. Internal collaboration section IDs
remain English; translation is applied only to the rendered headings.

Additional literal action descriptions are checked at BindingDescription and
EditableBinding construction sites. The catalog includes terminal/editor actions
and compiled debug labels without changing action IDs or key sequences. Language
selector updates subscribe to the settings model so Reset and external preference
reloads reflect the persisted language; restart still applies the interface locale.

All nonempty inventoried settings keyword groups have simplified/traditional
search aliases. Communication forms translate their labels and validation errors
at display boundaries, retaining original enum identities and literal confirmation
tokens (ALLOW OVERLAP, RELEASE RESERVATION, DELETE HISTORY). Companion response
status text is translated on the host without changing Companion 4.0.0 payloads.
Heading icons recognize their translated titles and have a focused three-locale
regression check. Font weight dropdown creation and selection use identical labels.

Task state labels are translated at the task detail, task list, worktree expansion
and task selector boundaries. Query state values and comparisons retain their
protocol identity; user descriptions and task IDs remain unchanged. Catalog tests
cover every current task lifecycle state in all three interface languages.

Constant-based find tooltips/placeholders, secret-redaction descriptions and MCP
empty states are localized at their callers. The catalog checker resolves local
constant and LazyLock references used by explicit localization calls, with
self-checks for both declaration forms; it does not translate arbitrary content.

The accessibility audit also covers the help argument of announcements and views,
including native menus, workflow selection, find/replace, command input, update
and notification guidance. Command/output content stays literal. Shared command
input labels are localized at consumers, retaining the original constant values.

Update availability and toast/link labels are localized. Static update status is
translated when rendered so the existing failure-prefix check that exposes the
manual release link retains its behavior. Privacy page titles use the selected
locale without changing SettingsSection identity.

Literal MenuItemFields labels are localized at construction sites, and that
constructor family is included in the strict display inventory. Platform reveal
actions and notebook block/heading labels share translated display values; file
paths, shell names, branch names and shortcut labels remain literal.

Tooltip, placeholder, toast link and formatted text fragment constructors are
included in the display inventory. Commands, regular expressions, config file
names and branch-name examples stay verbatim. Linux desktop updater UI is
explicitly excluded; its implementation is outside this release's desktop scope.
The macOS backend initializes application settings before building its menu bar,
so the selected locale is available when native menu labels are constructed.

Toast notifications and constant-based labels, including commit/find/keybinding
placeholders, theme dialogs, prompt setup and settings import, are localized only
at display consumers. Constant declarations retain their original values. Pane
kind badges and fallback titles translate fixed kinds; user titles stay literal.

Local preflight at 7c9109bf: all 301 changed Rust files parse under their crate's
declared 2021/2024 edition. Strict catalog checks cover 2,330 translation pairs
and 2,378 marked display sites; Python, YAML and defaults JSON syntax checks
pass. Companion sources and Linux desktop updater sources match the 1.6.0
acceptance baseline c3497dd4. Cloud compilation, tests and three-locale native
visual acceptance remain gated on successful 1.6.0 acceptance.

Expand native acceptance to all seven additional visible settings sections:
Appearance, Features, both Code subpages, Keyboard shortcuts, Shared blocks and
Privacy. Capture each in light/dark at 800/1200 width and 125% zoom; assert the
actual selected section before capturing. Also capture the open Language menu
in each Appearance variant. Existing About state captures remain in place.
This adds 32 images per locale (macOS 918 total; Windows with WSL 936 total).
Use isolated profiles and the existing 300-second watchdog; runtime and visual
results must pass the cloud gate before release.
