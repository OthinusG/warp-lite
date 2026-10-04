# Deep source cleanup

## Objective and boundaries

Remove unused source, resources, scripts and dependency declarations, including
obsolete bundled AI/cloud product remnants, without changing supported Warpai
behavior. The baseline is `378a67510367035c94bf7fef2bf8f927c5e2998c`.

Supported products are macOS/Windows desktop, existing local CLI/MCP collaboration,
and the Linux/macOS/Windows SSH companion. Preserve terminal rendering/input/model
guardrails, editor/project browsing, migrations and stored-data compatibility,
native runtimes, notices/licenses and default/`warp_platform` compilation.

The user explicitly reaffirmed that third-party AI agent management must remain:
launching, native setup, MCP configuration, notifications and coordination are
retained. README images must use native macOS vertical tabs and Claude Warm Light.
Set those preferences only in the isolated documentation capture profile; verify
that the vertical tab panel is visible before capturing the documentation scene.

## Removal rules

- Establish Cargo targets, Rust module/include reachability, conditional features,
  build-script resource consumption, scripts and documentation entrypoints before
  deletion. No single identifier search establishes that code is unused.
- Remove unreachable trees and their orphaned wiring/resources/dependencies.
- Trace shared consumers before trimming compiled historical product modules.
  Do not replace terminal or editor implementations with placeholders.
- Retain native linking, macro/derive requirements, migrations, active fixtures,
  resource manifests and current packaging/validation commands.
- Never inspect credentials or personal application settings. Rust builds/tests
  run on GitHub; local inspection, formatting and script checks are allowed.
- Update both README languages to describe the final product accurately. Network
  access by user commands, SSH and independently installed agents remains supported.

## Tasks and acceptance

1. Inventory reachable sources and build/resource/script edges; record removals
   and retained shared boundaries below.
2. Delete proven unused files, then remove newly orphaned dependencies and assets.
3. Review default and supported-feature module wiring and script/resource paths.
4. Run focused static checks and both existing GitHub validation workflows with
   full application regressions, native UI capture and packaging.
5. Verify source-matched desktop/companion artifacts, visual behavior and links.
6. Update project memory, fast-forward the accepted default branch, and remove
   this task's merged temporary branch.

Completion requires supported-target checks and functional/native acceptance to
pass after cleanup. A deletion count or successful compilation alone is insufficient.

## Audit and evidence

The audit enumerated 115 Cargo source targets and all 3,028 tracked Rust files,
following module declarations, inline/macro modules, path attributes and include
edges across all conditional branches. Negative results were checked against
source, resources and build/script consumers. The graph found 53 unreachable
Rust files: 51 permanently disabled AI-search implementation files and two
unwired upstream terminal/completer test files. The two test files are retained
under the unrelated-terminal-code preservation rule.

The three AI search roots already export independent compatibility types. Their
`cfg(any())` implementation subtrees never compile in any feature variant; delete
those subtrees and declarations while retaining the consumed compatibility types.

Embedded assets were checked against surviving source, icon maps, literal asset
macros, dynamic asset construction, tests, manifests, scripts and documentation.
The unused onboarding images do not include the theme images selected by the
login slide's dynamic intention/theme/orientation lookup. Fonts, syntax data,
bootstrap resources, native binaries and all referenced assets remain.

Upstream bundled skills are loaded only behind `BundledSkills`, absent from
supported desktop defaults, `warp_platform`, and packaging features. Current
local CLI/MCP coordination does not load these payloads. Remove these optional
upstream payloads and gated-skill distribution logic; preserve project/user skill
loading and the shared skill manager. Retained license entries still refer to
retained components; remove only the removed Claude API skill attribution.

All four inherited composite actions have no callers in the remaining workflows
or project entrypoints. The two Sentry upload/release scripts and the old channel
release-branch generator likewise have no callers. Retain current standalone
release workflows, build/run scripts, remote fixture setup and referenced triage
configuration. The installer now references `warpai-banner.bmp`; its old banner
has no consumer.

| Removal group | Files | Bytes |
| --- | ---: | ---: |
| Permanently disabled AI context/search/embedding sources | 51 | 310,196 |
| Unreferenced embedded image/icon assets | 51 | 37,953,798 |
| Unsupported upstream bundled/cloud skill payloads | 124 | 1,803,384 |
| Uninvoked upstream GitHub actions | 7 | 25,666 |
| Uninvoked channel/cloud release helpers | 4 | 6,970 |
| Replaced installer artwork | 1 | 234,218 |

### Dependency cleanup

Remove three unused direct dependency edges and their matching lockfile edges:
`command -> lazy_static`, `node_runtime -> sha2`, `syntax_tree -> warp_util`.
Their source/build/test targets have no consumers and the declarations add no
necessary native linking or feature contract. All locked package versions remain.
The broader token-negative audit was reviewed conservatively: retain macro
requirements (for example `safe_info! -> log` and `num-derive -> num-traits`),
SQLite bundled linking, explicit runtime/target feature unification and optional
compatibility-feature dependencies.

### Retained shared boundaries

Compiled AI/cloud modules still supply types, local CLI harnesses, editor/notebook
and terminal integration, persistence/migration schemas and `warp_platform`
consumers. Removing their directories would remove live consumers, rather than
unused code. Keep them and their necessary dependencies/notices. No claim is
made that all AI-named code or all networking is gone. Historical specifications
and provenance remain as documentation; they do not restore removed product paths.

### Verification

A first dependency pass incorrectly limited consumers to each crate directory.
Desktop setup-test compilation exposed the cross-directory `#[path]` import of
`app/src/agent_communication/setup.rs` by `warp-agent-bus/tests/setup.rs`.
Restore the test crate's `serde_yaml`/`toml` dependencies and follow each Cargo
root's transitive module/include closure across directories for dependency checks.
Do not count the failed preliminary desktop run as acceptance.


Local checks passed: all reviewed removal paths are absent, retained license
inputs exist, the surviving source graph has only the two preserved upstream
test files outside Cargo reachability, and shell syntax/diff checks pass. A
packaging dry run used a temporary Cargo command fixture (no Rust compilation)
to verify stale skill removal, retained license assembly, version metadata and
settings-schema command construction.

Pending source-matched GitHub desktop and remote acceptance, release-package
inspection and native macOS vertical-tab documentation screenshots.
