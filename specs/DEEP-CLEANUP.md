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
   full application regressions without packaging.
5. Verify native process behavior, desktop compilation and documentation links.
   Icon changes and release packaging are deferred by the user on 2026-10-05.
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
| Tracked generated Yarn install state | 1 | 1,236 |

The tracked Yarn `install-state.gz` is generated tool state with no package
manifest or build/runtime consumer. Remove it in the delivery cleanup; it is not
an executable/build input. The small upstream JavaScript completion demo remains
under the unrelated-terminal-code preservation rule, alongside the two unwired
Rust test files. Actual command signature data comes from the pinned external
`warp-command-signatures` dependency, not a replaced local implementation.

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

The following source `02d08d4` records are historical cleanup evidence. Identity,
settings migration, user-completed formatting/UI changes and the final vector
icons are covered by the integrated checkpoint below.

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

Executable/build source: `02d08d484dd7ece33f528383230801d78c1355f5`.

- [Remote acceptance run 37194454441](https://github.com/OthinusG/warpai/actions/runs/37194454441)
  passed on Linux/macOS/Windows, including strict Clippy, protocol/persistence,
  owned native PTYs and controlled Linux OpenSSH two-agent messaging/tasks.
  Downloaded companion manifests, capabilities, checksums and notices match.
- [Desktop acceptance run 37194453713](https://github.com/OthinusG/warpai/actions/runs/37194453713)
  passed macOS/Windows default and `warp_platform` checks, focused application
  regressions, native setup/actions and all 161 captures per OS. Release
  packaging and package inspection are pending.
- Both capture archives passed CRC and GitHub SHA-256 checks, with successful
  diagnostics naming the exact source above. Reviewed narrow/wide light/dark
  collaboration panels and six MCP settings scenes per OS. The two README PNGs
  are byte-identical to the reviewed native macOS originals, showing vertical
  tabs and Claude Warm Light; [provenance](../docs/images/README.md) records the
  capture set. No image recoloring or generated UI is used.
- Third-party agent management and terminal guardrail sources have no changes
  against the baseline. All locked package/version/source/checksum sets remain
  identical; only the three reviewed dependency edges change.

At that historical checkpoint, delivery changes after the source above were documentation,
original PNGs and deletion of unconsumed generated Yarn state. They do not change
Rust source, manifests, workflows or executable/resource build inputs.

### Integrated identity, cleanup and vector icon checkpoint

The identity follow-up removes 35 unconsumed obsolete cloud/Oz/synchronization
documents and four replaced/unused Rust source files: 39 files, 444,578 bytes.
Combined source/document/resource cleanup removes 278 files, 40,780,046 bytes;
engineering renames and the six PNG-to-SVG replacements are not counted as unused
file removals. Keep macOS UserDefaults for the required Apple press-and-hold OS
integration. Preserve independently installed Agent management, shared terminal
code, protocol compatibility, stored data and copyright/license notices.

The combined scope includes the Warpai package/executables/bundle identity,
canonical `.config/.warpai` root and non-destructive legacy import, the completed
user UI/format changes, native panel/settings consistency and six real vector
icons. Final production flow is recorded in [the icon workflow](../docs/ICON-WORKFLOW.md).
The remote acceptance fixture now atomically replaces heartbeats so terminating
the owned child cannot leave a truncated file and falsely report a live process.
No production stop behavior or acceptance assertion is weakened.

Executable/build source: `b86328644c8e77b76e612ae32ddae4695e4565fd`.

- [Remote run 37212494323](https://github.com/OthinusG/warpai/actions/runs/37212494323)
  passed Linux/macOS/Windows, including Windows owned-process disconnect and
  controlled Linux OpenSSH messaging/tasks/review. All three downloaded archives
  and binaries match their source manifests/digests, capabilities and notices.
- [Desktop run 37212494687](https://github.com/OthinusG/warpai/actions/runs/37212494687)
  was paused at the user’s request and is superseded by the native Codex
  correction and final gates below. It is not current-source acceptance.
  Updated icon screenshots and release package inspection belong to the next
  icon/packaging task.

### Native Codex adaptation and final cleanup — 2026-10-05

Removed the inherited hosted-model Codex promotion modal, its constructor/state,
rendering, URI/actions, preferred-hosted-model accessor and unused telemetry
variants. It is unrelated to third-party Codex CLI management, which is retained.
Removed the orphaned `codex_integration.png` (486,090 bytes). Current plugin
specifications now reference the generic native modal pattern.

Updated both README languages and the compatibility/technical documentation for
automatic per-invocation Codex MCP. Removed the superseded explicit-only launch
plan and command-alias description. Verification must include current native
session isolation, argument preservation, upgrade resolution and desktop builds.
No release package is built in this final cleanup gate.

Removed 18 obsolete enrolled-device/full SSH-manager design documents
(418,624 bytes). Current specifications link to their immutable Git history
where historical evidence is needed, rather than keeping abandoned backlogs
in the working tree. Retained the active SSH project/API/acceptance documents.

Removed the obsolete child-PATH alias stripping branch, an unused broker runtime
handle and the unused bridge directory setter left behind by proxy removal.
The simulated native-status helper is test-only; production uses terminal
lifecycle and final-action MCP readiness.

### Current verification

- Native macOS Codex 0.160.0: two simultaneous original clients, two model turns
  each, native MCP list/final-ready calls and independent busy/ready state passed.
  Cloud-built executable/bridge from 625904f; agent_bus inputs remain identical
  in f6fa053. The rerun includes the actual native-directory bridge mode. Both downloaded artifact CRCs and GitHub SHA-256 digests matched.
- Static checks passed: changed Rust syntax via rustfmt (no local Rust build),
  38 tracked Bash/Zsh scripts, workflow YAML, manifest/lock TOML and all local
  links in the current collaboration specifications and both README languages.
- Recomputed Cargo/module reachability: 116 target roots, 2,976 Rust files;
  only the two deliberately preserved unrelated upstream terminal/completer test
  files remain outside target reachability. The seven parser limitations are
  existing grammar/macro constructs; supported-target Cargo checks are required.
- Baseline tracked deletion inventory: 302 files, 41,706,081 bytes. This includes
  earlier audited removals, naming replacements and the final obsolete modal,
  marketing asset and retired design-document cleanup. It is not a claim that
  every remaining upstream compatibility type is unused.

- [Remote gate 37230080613](https://github.com/OthinusG/warpai/actions/runs/37230080613)
  passed for f6fa053 on Linux/macOS/Windows: protocol and strict remote-wire
  lint, local relay/readiness/native setup/history, owned native processes/PTYs
  and controlled Linux OpenSSH message/task/review flow.

Desktop run [37230078092](https://github.com/OthinusG/warpai/actions/runs/37230078092)
passed all macOS checks/regressions; Windows compilation passed but the new
PowerShell roundtrip regression exposed argument splitting. It is superseded by
the corrected source below. The isolated Windows diagnostic
[37233373477](https://github.com/OthinusG/warpai/actions/runs/37233373477) verified
60 launcher/shell/mode/path combinations with unchanged parsed TOML values.
The permanent application regression now covers those same boundary cases.
Corrected-source remote gate
[37233527936](https://github.com/OthinusG/warpai/actions/runs/37233527936) passed
for eab0ff0 on Linux/macOS/Windows, including controlled Linux OpenSSH acceptance.
The native-model acceptance inputs remain byte-identical to 625904f.
Final desktop gate
[37233525889](https://github.com/OthinusG/warpai/actions/runs/37233525889) passed
for eab0ff0 on macOS and Windows. Both default and `warp_platform` application
checks passed. Application regressions passed for original Codex argument and
expansion preservation, all 60 Windows launcher/shell/mode/path roundtrips,
both pending-launch Ctrl+C cancellation paths, dormant wake/draft protections,
native setup, settings and legacy-data/credential migration.

These gates did not produce desktop installers or capture new UI images. The
remaining delivery work is icon revision (including refreshed screenshots) and
packaging/release; no code or acceptance fix remains in this scope.

### Default branch and repository delivery — 2026-10-05

Renamed the GitHub repository to [OthinusG/warpai](https://github.com/OthinusG/warpai)
and the default branch to `main`. Fast-forward integrated
33e366fb85f804d89b9e356781c989199a259fb9, whose production/build inputs match the
accepted eab0ff0. Deleted this task's merged branch and the isolated diagnostic
branch locally and remotely; GitHub retains only `main`. Full history and release
tags remain. Updated origin and its HEAD; final delivery receipts are documentation
only. No icon revision, new desktop screenshots or installer/release was made.
