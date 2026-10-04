# Warpai identity and user-data migration

## Product requirements

Unify the maintained product's identity as Warpai. The application Cargo package,
library and default executable are `warpai`; the owned bridge executable is
`warpai-agent`; the macOS identifier is
`dev.warpai.Warpai`. Current installation, CLI entrypoints, settings UI, resource
metadata, owned repository links, build scripts and CI must use Warpai names.
Internal subcrate/type names and WARP_* protocol/environment names remain
compatible, as explicitly confirmed by the user. The owned MCP server identity
is a compatibility key, not a product brand label.

Rename the GitHub repository to `OthinusG/warpai` after accepted-source delivery.
Preserve upstream copyright, license notices, real provenance and external
package identities. Historical release filenames and compatibility/migration
inputs must describe the real old names rather than fabricate renamed history.

All maintained desktop settings/data live below `~/.config/.warpai` on macOS and
Linux, and `%USERPROFILE%\.config\.warpai` on Windows. This means the user's home,
not a literal `/home` directory. Keep state and caches in `state` and `cache`
subdirectories; development profiles stay below this root and cannot escape it.
Third-party agent settings remain in those agents' native locations.

Preserve current third-party agent launch/setup/MCP/notifications/collaboration,
terminal behavior and existing local/SSH authority and receipt semantics. No
network account, telemetry or bundled cloud AI product is restored.

## Technical plan and migration contract

1. Update package/executable consumers together: manifests and lockfile edges,
   entrypoint imports, scripts, installers, capture drivers and workflows. Keep
   pinned external package versions/sources unchanged. Audit owned names without
   blindly replacing third-party API names or legal attribution.
2. Centralize desktop config/data/state/cache paths in the existing core path
   module. Retain base config helpers used to locate other programs. Preserve
   debug capture isolation below the canonical root.
3. Before preferences or durable state are opened, import legacy home and
   platform application directories. Prioritize the former OSS product over
   shared upstream defaults. Never overwrite an existing destination or delete
   conflicts; preserve old files for recovery. Do not traverse source symlinks.
   Migration failures must not silently replace settings with defaults.
4. Reuse the existing JSON preferences backend on both desktop platforms,
   keeping editable `settings.toml` behavior. Import legacy macOS/Windows
   preferences without printing values or changing third-party credentials.
   New preference writes must not return to UserDefaults/Registry or old paths.
5. Update maintained documentation, package names, branding and repository
   links. Preserve historical source evidence and real external upstream links.
6. Run exact-source GitHub desktop and remote acceptance after all source/build
   edits; inspect native macOS warm vertical-tab captures and release packages.
   Only then integrate the default branch, rename the repository, update remotes
   and verify links/default-branch source. Remove only merged task branches.

## Acceptance checks

- Locked Cargo metadata resolves the `warpai` application/default target and
  both default/`warp_platform` application builds compile on macOS and Windows.
- Path tests verify every managed root, state/cache children and isolated profiles.
- Synthetic migration tests cover both legacy home roots, precedence, existing
  destinations, nested contents, symlinks, idempotency and representative errors.
- Native preferences migration retains representative public/private settings;
  subsequent writes and resets use only the new file store.
- Protocol/persistence/native launch/MCP/SSH regression tests pass, with no change
  to third-party agent configuration ownership or authentication boundaries.
- Native screenshots preserve vertical tabs and Claude Warm Light; package
  headers, identifiers, runtimes, resources and notices are verified.
- GitHub repository identity, remote URLs and accepted default-branch source match.

Rust compilation/tests/package builds run on GitHub only. Local checks use
source inspection, formatting, script syntax, fixtures and documentation links.

After identity changes, repeat the unused-source and obsolete-document audit.
Remove only unconsumed items; retain current specifications, migration context,
licenses and source-matched acceptance/provenance. Repair surviving links.

## Post-identity cleanup audit

Removed 35 unconsumed obsolete documents: unsupported bundled feedback/cloud AI,
Oz/cloud execution and sharing designs, the superseded MCP removal plan, old
upstream-sync plans, and the superseded Preview symlink migration specification.
They had no retained exact-path or Markdown-link consumers. Preserve useful
terminal/editor/settings/CLI agent specs even when not linked from README; link
absence alone does not make functional documentation useless.

Removed the three Preview-only migration implementation/test-hook files and
unused native project-directory/App Group path helpers. The single managed-root
migration supersedes them, preserves channel/profile isolation and keeps recovery
sources. New synthetic migration tests replace the obsolete symlink behavior.

The former Registry preference writer has no remaining constructors after
switching the app to file preferences. Move its generic missing-key constant
to the WSL consumer and remove the unused writer. Keep the macOS UserDefaults
helper: a compiled OS integration consumer needs Apple's per-application
press-and-hold setting, which a custom JSON file cannot implement. It is not the
managed product preferences backend. Legacy import reads saved macOS plist and
Windows keys through already-installed libraries without creating old stores.

Cleanup removes 39 additional tracked files (444,578 bytes), excluding engineering
script/specification renames. Combined with the earlier deep cleanup this is
278 removed files. User-completed panel and formatting changes are preserved
and included in the final combined-source GitHub gate.

## Final delivery sequence

The user's latest correction is authoritative: finish all identity, cleanup and
UI repairs before native screenshots and release packaging. Cancel the superseded
desktop run rather than build packages from interim UI. Run one complete desktop
and remote gate for the final combined source, inspect captures/packages, then
integrate the default branch, rename the repository and remove the merged task
branch. Preserve the completed user styling throughout.

Native migration must run only from `run_internal()`: the capture driver finishes
its temporary-home/profile setup while building the driver. An earlier `run()`
call bypassed that isolation and has been removed. Native CI must verify that
capture does not create an ordinary-home migration marker. Help/completion and
worker dispatch must not perform a desktop settings import.
