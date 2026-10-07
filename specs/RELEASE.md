# Warpai releases

## Current delivery: 1.2.2

The owner authorizes pushing the Review baseline correction and triggering
v1.2.2, without monitoring afterward. Retain Companion 2.0.0 and protocol major
1. Use docs/releases/v1.2.2.md and versioned desktop installer names.

Dispatch release-macos.yml from main with release_tag=v1.2.2 and
auto_publish=true. The option defaults to false for other releases. The macOS
workflow creates a draft only after its desktop and three-platform Companion
checks pass; its release-target artifact carries the explicit publication
choice. Windows follows automatically, verifies and uploads its installer,
then publishes the complete draft as latest when requested. Failed builds leave
the release unpublished. This owner instruction supersedes manual monitoring
and downloaded-package inspection for this delivery. Preserve historical tags.

Local focused checks passed; desktop checks were blocked by the missing Metal
compiler. Triggering automation does not constitute completed native acceptance.

## Historical delivery: 1.2.1

Publish `v1.2.1` only after the tagged desktop/agent regressions, native UI
review and three-platform remote installation/transport checks pass. Build
committed tagged source with the existing release workflows; do not compile
Rust locally.
The formal companion jobs reuse disposable-runner native install/reinstall checks
and exercise Windows binary stdio against the installed optimized executable.

Deliver `Warpai-1.2.1-macos-arm64.dmg`, `WarpaiSetup-1.2.1-windows-x64.exe`,
`WarpaiCompanion-2.0.0-linux-x64.run`, `WarpaiCompanion-2.0.0-macos-arm64.dmg` and
`WarpaiCompanion-2.0.0-windows-x64-setup.exe`, with SHA-256 checksum files. No desktop
or raw companion ZIP is a public release asset. Remote installers own only the
account's Warpai component and metadata; follow the
[installation contract](agent-communication-v2/INSTALLATION.md).

The macOS workflow creates a draft. Windows attaches its version/icon-checked
installer and leaves that draft private. Inspect downloaded formal packages,
source manifests, checksums, native identities and artwork before publishing
the complete release. Preserve all prior release tags and assets.

README images must be unmodified native macOS captures with vertical tabs and
Claude Warm Light, with exact source/run provenance. Native Windows independent
SSH authentication uses system keys/agent; Warpai does not store passwords or
modify vendor commands, shell profiles or SSH configuration. macOS ad-hoc
signing and unsigned Windows installation remain documented.
The desktop DMG contains the signed app, an Applications shortcut and the
canonical Warpai Finder volume icon. Verify these in the mounted formal image.

The 1.2.1 delivery completes the existing SSH Explorer, in-app editor/preview
and original Git Review workflow, simplifies daily Agent collaboration, and
includes the legacy MCP cleanup and actionable setup errors prepared after 1.1.
Ordinary Codex communication remains session-only. The explicit cleanup preserves
unrelated MCP entries and reports unsupported or failed cleanup.

The public baseline is 1.1.0; the 1.1.5 preparation was not published. Explain
changes from 1.1 to 1.2 in docs/releases/v1.2.1.md. Update the existing README
feature/setup/remote/download sections in place, retaining the existing native
screenshots; do not add a README changelog. The banner identifies the delivered
version, both desktop targets and all three remote targets. Remote copy describes
local/remote workflow parity, without upgrade history or transport internals.

### 1.2.1 delivery tasks and acceptance

1. Preserve the unpublished v1.2.0 tag at 210ebb4a as requested. Release Warpai
   1.2.1 with Companion 2.0.0; protocol major remains 1. Retain accepted runtime
   logic from 60ea2810 (desktop/native run 37569663988) and the accepted Companion
   backend (three-platform run 37498107083). Verify the component-version change
   and versioned packaging, then create the immutable v1.2.1 tag.
2. Dispatch release-macos.yml from main with release_tag=v1.2.1. Its three native
   Companion jobs verify tagged builds and install/reinstall before the macOS
   desktop build creates a private draft. The existing Windows workflow follows
   automatically and attaches its version/icon-checked installer.
3. Download all five installers and both checksum files. Check download hashes,
   exact source manifests, target/version identity, bundled Agent bridge, license
   notices and existing artwork. Mount both macOS DMGs read-only to check their
   Finder icons and contents. Retain native Windows installation/version receipts.
4. Publish the complete draft as latest only after these gates pass. Verify the
   public asset inventory and README download links, and record the immutable
   tag, jobs and delivery receipt in MEMORY.md. Preserve older releases.

## 1.2.1 public delivery receipt

Published [Warpai 1.2.1 with Companion 2.0.0](https://github.com/OthinusG/warpai/releases/tag/v1.2.1)
as latest on 2026-10-07. Immutable tagged source:
`4788cbde9a74987b53a3bd032b340cc38bd09fb7`.
The unpublished v1.2.0 tag remains at 210ebb4a as requested.

- [Formal macOS and three-platform Companion release](https://github.com/OthinusG/warpai/actions/runs/37577830960):
  all four jobs pass, including component-version probes, native installation/
  reinstallation, Windows Companion installer ProductVersion 2.0.0 and installed
  optimized Windows binary stdio. macOS desktop default/platform checks and the
  tagged release build pass.
- [Formal Windows desktop release](https://github.com/OthinusG/warpai/actions/runs/37581995715):
  native PE numeric/string versions, installer ProductVersion 1.2.1, icon, Agent
  bridge and archive contents pass. The downloaded public installer is byte-for-byte
  identical to its CI artifact; the source receipt matches the immutable tag.
- All five installers and both checksum files match GitHub SHA-256 digests.
  Both checksum files cover exactly the public installer inventory. Parse standard
  SHA-256 text/binary mode markers, including Windows's `*` marker.
- Both DMGs pass verification and read-only inspection. Desktop bundle identity,
  1.2.1 version, strict deep code signature, Agent bridge, canonical icon,
  Applications shortcut and Finder volume icon flag pass. macOS Companion's
  executable reports 2.0.0 protocol 1; its manifest, source/target/hash, launcher
  resource fork and artwork pass. Linux payload source/target/version, binary
  checksum, license notices and artwork pass.
- Release notes describe changes from 1.1 to 1.2. Bilingual READMEs update existing
  feature sections, identify local/remote platform support, explain remote file
  workflow parity and retain the original screenshots. All five download links
  match the public versioned installer names. No raw ZIP is a public asset.

| Public installer | SHA-256 |
| --- | --- |
| `Warpai-1.2.1-macos-arm64.dmg` | `84838a232ada10551ca24568be684f5b517f6e26772e9865ce0ab2605ed9afc9` |
| `WarpaiCompanion-2.0.0-linux-x64.run` | `460ae0c1a9f8fddf28179c4a339cbd7f08a1819a1e85c86e7483e672d9658fe0` |
| `WarpaiCompanion-2.0.0-macos-arm64.dmg` | `831a44fc5f1247c3a9e7a7e69978af8af53ec8b1a9b6de8828fcda07ad9fac8b` |
| `WarpaiCompanion-2.0.0-windows-x64-setup.exe` | `7099a0f2846a520e755e8f143b48ff993089ec5781c51bebd41c09f550743d36` |
| `WarpaiSetup-1.2.1-windows-x64.exe` | `bcf911bf21222995d74a50da7920434f91441a7b70ccb2dfa2ac89f06dc2f78c` |

## Historical releases

## Historical 1.0.1 preparation (not published)

> Superseded before publication by the owner's [1.1.0 iteration](agent-communication-v2/INSTALLATION.md). Keep the 1.0.1 draft private and preserve its source tag.

## Scope and acceptance

Publish the first renamed Warpai release as `Warpai 1.0.1`, tag `v1.0.1`.
The owner authorized publication after desktop/native and remote acceptance.
Rewrite both READMEs as complete product introductions with native screenshots,
features, installation, agent setup, SSH setup and stable release download links.
Preserve terminal guardrails, licenses and historical tags.

## Delivery plan

1. Accept the current icon source with full macOS/Windows checks and native
   screenshot review, plus Linux/macOS/Windows companion verification.
2. Reuse the two release workflows; build only tagged repository source. Derive
   bundle, installer and PE versions from the same release tag. No local Rust
   compilation and no changes to user CLI installations or configuration.
3. Build source-matched release companions for Linux x64, macOS ARM64 and Windows
   x64. Each archive includes its target, commit, binary version and checksums.
   Companion protocol versions remain independent of the product release version.
4. Create a draft after macOS packages and companions pass. Windows attaches its
   validated EXE installer and publishes the complete draft.
5. Verify public release assets, checksums, identities and documentation links;
   record acceptance and delivery in MEMORY.md.

## Packaging checks and risks

- macOS DMG must contain Warpai.app, `dev.warpai.Warpai`, version 1.0.1,
  the reviewed ICNS and the local MCP bridge. Verify ad-hoc signing and DMG.
- Windows installer must contain Warpai.exe, required native
  resources and the local MCP bridge. Application PE numeric/string versions and installer product
  version must be 1.0.1; the packaged icon must match the reviewed ICO.
- Remote packages must identify actual Rust target and tagged source, pass their
  native version invocation, and have verified SHA-256 manifests.
- Ad-hoc macOS signing and unsigned Windows installers can require OS approval;
  describe actual installation behavior without promising notarization.
- A failed packaging job leaves a draft rather than a partially public release.
  Preserve existing assets/history and fix failures before publication.

## Verification receipt

### Windows gate correction and installer-only delivery

The owner requested desktop installer images only: macOS DMG and Windows EXE,
plus all three companion archives. Remove desktop app/portable ZIP release assets
and update both READMEs, notes and checksum lists; retain the immutable source tag.

Run 37282889244 built the Windows packages but failed an incorrect installer
numeric file-version assertion. Inno's `VersionInfoVersion` defaults to 0.0.0.0;
its product text version defaults to `AppVersion`. See the official
[file version](https://jrsoftware.org/ishelp/topic_setup_versioninfoversion.htm)
and [product version](https://jrsoftware.org/ishelp/topic_setup_versioninfoproducttextversion.htm)
contracts. The application PE string-version assertions already passed.

Fix plan (diagnose-ci-failures / fix-errors):

1. Keep application PE numeric/string versions strict; verify the installer
   product version against the release version. Log actual version fields.
2. Preserve source-matched installer/checksum artifacts before the gate, allowing
   diagnosis without losing successfully compiled packages.
3. Publish only the EXE from Windows; remove the existing private desktop ZIP,
   correct checksums and notes, and keep DMG/companions already validated.
4. Validate workflow syntax, rerun native Windows checks, then inspect the public
   asset list/checksums and publish a complete release. No CLI installation edits.

- Icon/runtime source `5d2588a86bbdc05cc994ad816561c4626be9aac4` passed
  [desktop/native run 37268540992](https://github.com/OthinusG/warpai/actions/runs/37268540992):
  both application configurations, focused regressions, 161 images per OS,
  source-matched diagnostics and all native action assertions. Archive SHA-256,
  ZIP CRC and every PNG decode passed locally; selected native frames were reviewed.
- [Remote run 37268449581](https://github.com/OthinusG/warpai/actions/runs/37268449581)
  passed Linux/macOS/Windows. Packaging preparation source `ade8308` also passed
  [remote run 37270362606](https://github.com/OthinusG/warpai/actions/runs/37270362606).
- Release workflow actionlint, YAML, Bash and embedded Python syntax checks passed.
  Temporary macOS packaging smoke checked version 1.0.1, bundle identity, canonical
  ICNS, signing and invalid-version rejection using copied system placeholders.
  This smoke is a packaging check, not application/runtime acceptance.
- Publication additionally requires the cloud tagged-package checks above.
- macOS distribution uses `release_bundle,extern_plist`, matching the existing
  `script/macos/bundle` OSS path. This preserves release IME marked-text behavior
  and uses the actual installed Info.plist rather than the development embedded
  plist. Native UI acceptance already uses `release_bundle`. The release workflow
  executes from main while checking out the immutable tagged application source.
- Packaging-version source `ade83082b101ac624b62d90ea2c25d1a46b4fc65` passed
  [desktop run 37270362536](https://github.com/OthinusG/warpai/actions/runs/37270362536)
  on macOS and Windows, including both application configurations and all retained
  focused regressions. Subsequent pre-tag changes affect docs/screenshots and
  release-only archive checks, not application or companion runtime inputs.

- Tagged macOS/companion packaging passed [run 37277364402](https://github.com/OthinusG/warpai/actions/runs/37277364402). Downloaded macOS app inspection verified version 1.0.1, bundle identity, canonical ICNS, bridge and strict signing; the retained DMG passed SHA-256 and native image verification. All three companion archive manifests match immutable tag source `2bb47910ceadcf4ea47b4370166a6df7a34d2b96`, with valid binary/archive checksums.
- Installer-only workflow correction `594f015` passed actionlint, YAML, shell syntax and bilingual download-link checks. The private draft now contains only DMG, three companions and their checksum file; Windows rerun [37290225101](https://github.com/OthinusG/warpai/actions/runs/37290225101) must pass before publication.

### 1.1.0 Windows installer string normalization

Run 37362390293 built the tagged Windows application and installer with product
version 1.1.0, but Inno Setup pads its ProductVersion resource with spaces.
Normalize only that installer string with Trim before exact comparison; keep
application string/numeric checks and native resource checks strict. Dispatch
the corrected workflow from main while checking out the unchanged immutable
v1.1.0 source tag. Rerun the Windows release gate before uploading or publishing.

## 1.1.0 public delivery receipt

Published [Warpai 1.1.0](https://github.com/OthinusG/warpai/releases/tag/v1.1.0)
as the latest non-prerelease. Immutable tag source:
`574b2f97772cbd0f932128662445a686dd196a5e`.

- Full application regressions: run 37340966290; final native UI/actions:
  run 37349315123, 163 frames per platform with archive integrity and visual QA.
- Tagged macOS desktop and three native companion installers: run 37355772889.
  All remote install/reinstall checks passed; installed optimized Windows
  companion binary stdio tests passed.
- Corrected Windows desktop release: run 37411727676. Application string/numeric
  versions, normalized installer version, icon, bridge and archive contents passed.
- Downloaded all five formal installers and both checksum files. All hashes
  match GitHub asset digests; Windows installer is byte-identical to its native
  CI artifact and release-source receipt matches the immutable tag.
- Both DMGs passed native checksum verification and read-only mounted inspection.
  Desktop bundle identity/version, executable, strict deep code signature,
  reviewed ICNS, local MCP bridge, Applications shortcut and actual Finder custom
  volume-icon flag passed. Companion source/target/version manifests, payload
  hashes, artwork, launcher resource fork and actual volume-icon flag passed.
- Linux payload licenses, target/source/version manifest, executable checksum
  and companion artwork matched the tagged source. Windows setup PE/version
  resources passed; native CI performed the platform-specific installation checks.

| Public installer | SHA-256 |
| --- | --- |
| `Warpai.dmg` | `0ae508be02dc210b30d0c86dc2667b1518cc1afd5287cd45eda011573aa11f2a` |
| `WarpaiCompanion-linux-x64.run` | `85e4ba3b7d68195b1b9e6d1b631717c0eabe741183e5dec538f4f3dead08954e` |
| `WarpaiCompanion-macos-arm64.dmg` | `32b7c5cddadafbcdffd42d883b8ec7a288de776df1e47b1965077e5ca2da7d27` |
| `WarpaiCompanion-windows-x64-setup.exe` | `4ef9e059c87b8c1d4331135f35d21dc3b291907208720e874a336db22dc32148` |
| `WarpaiSetup-x64.exe` | `3893afe0e9285a24f0147391df035dbe5a8cd68eaa60e68361e32e31861546fb` |

No desktop or raw companion ZIP is published. Ad-hoc macOS signing and unsigned
Windows installation remain the documented first-launch constraints.
