# Warpai releases

## 1.6.0 delivery contract

Owner authorization 2026-10-11: after acceptance passes, trigger both desktop
release workflows for Warpai 1.6.0 with Companion 4.0.0. Add no telemetry.

1. Finish the Windows WSL capture correction and one source-matched complete
   macOS/Windows acceptance run. Inspect diagnostics and the 274 macOS / 280
   Windows PNGs, including all six WSL views and installation/removal isolation.
2. Prepare v1.6.0 notes and README downloads. Companion RELEASE_VERSION remains
   4.0.0 and protocol major remains 1; desktop versions derive from the tag.
3. Fast-forward main to the accepted source and release documentation, retaining
   unrelated worktree changes and historical tags/assets. This also installs the
   existing WSL-aware Windows release workflow on the default branch, required
   by its workflow_run trigger. Create immutable v1.6.0; never move older tags.
4. Dispatch release-macos.yml with release_tag=v1.6.0 and auto_publish=true.
   Windows follows from the same tag and publishes after the native package gates.
   Preserve all three Companion native installer/runtime gates and both desktop
   version/source/package checks. Monitor at the owner's 30-minute cadence.
5. Verify all five public installers, both SHA-256 lists, source provenance and
   public links before reporting delivery. Record receipts here and in MEMORY.md.
   No personal installation, local Rust compilation or credential changes.

## Sequential patch delivery: 1.5.1, then 1.5.5

Owner authorization 2026-10-10 supersedes the earlier single-publication limit.
Publish 1.5.1 first, then 1.5.5; both use unchanged Companion 4.0.0 and protocol 1.
Add no telemetry. Do not begin 1.6.0 until the owner supplies its scope.

- 1.5.1 source: immutable tag v1.5.1 at 30ad76bb. Formal macOS/Companion run
  38022801330 uses auto_publish=true; Windows follows and publishes after gates.
- 1.5.5 source is prepared on release/1.5.5, retaining 1.5.1 ancestry and accepted
  native provider/UI implementation. Usage fixture visibility correction needs
  capture acceptance in run 38022957210 before tagging/publication.
- Retain native source/version/installer/reinstall/runtime checks and all five
  installers/checksum receipts. Confirm 1.5.1 public delivery before dispatching
  the 1.5.5 release; only the second release becomes latest afterward.

## 1.5.1 preparation contract (superseded publication limit)

Owner authorization 2026-10-10: complete both patch scopes, validate combined
1.5.5 directly, then package/publish the independent 1.5.1 source checkpoint.
Data usage belongs to 1.5.5 and must remain absent from 1.5.1. Retain historical
releases and the active workflows; do not resume the stopped 1.5.0 audit.

1. Keep 1.5.1 UI source on `patch/1.5.1`, with shared fixes backported from the
   combined implementation. Verify the checkpoint has no usage module, settings,
   footer or provider/credential dependencies and no Companion source changes.
2. Accept combined macOS/Windows builds, focused tests and native walkthroughs;
   record the immutable source/run and visual results in the patch contract.
   Do not repeat an intermediate 1.5.1 walkthrough. Apply shared fixes to both.
3. Integrate accepted 1.5.1 and release preparation into main. Keep Companion
   RELEASE_VERSION 4.0.0 and protocol major 1. Publish immutable `v1.5.1` through
   release-macos.yml with auto_publish=true; Windows follows existing gates.
4. Retain native source/version/installer/reinstall/checksum/runtime acceptance.
   Monitor cloud jobs at the owner's 30-minute cadence. Verify five versioned
   installers and both checksum lists before recording public delivery. No local
   Rust build, personal installation or automatic 1.5.5 publication is authorized.

## Historical delivery: 1.5.0 with Companion 4.0.0

Owner authorization 2026-10-10: merge all accepted fixes into main, clean unused
branches/workflows, and package/publish desktop 1.5.0 with Companion 4.0.0.

1. Merge the accepted correction branch into main, preserving local website
   memory commits as well as accepted code. Preserve historical tags,
   release assets, screenshots and the active gh-pages website. Delete completed
   development refs only after main contains them; remove only clean, unused
   native worktrees. The two release/two validation workflows remain necessary;
   keep Pages deployment and acceptance/release evidence.
2. Set Companion RELEASE_VERSION to 4.0.0, retaining protocol major 1. Desktop
   version derives from the immutable v1.5.0 tag. Update both README download and
   About/update sections, and write docs/releases/v1.5.0.md against public 1.3.1.
   No unrelated refactor, new packaging flow, local Rust build or installation.
3. Verify focused version/packaging checks and scripts, commit main, and create
   v1.5.0 without altering older tags. Dispatch release-macos.yml with
   release_tag=v1.5.0 and auto_publish=true. Existing native Companion install/
   reinstall/version/source checks precede the macOS draft; Windows follows and
   publishes only after its installer/version/icon/Agent-bridge checks pass.
4. Monitor cloud jobs at the owner's 30-minute cadence. Diagnose and fix failed
   gates without bypassing checks. Verify the five versioned public installers,
   both SHA-256 lists, exact source provenance, signed macOS bundles/DMG contents
   and public download links. Record delivery in MEMORY.md and this contract.

Acceptance baseline: source 46248e08 passes both desktop protocol/build/native
walkthrough jobs in run 37943970413, with 214 valid PNGs per OS (24 About states).
Application unit suites pass at production-equivalent 27a9f992 in run 37939850502.
Remote backend is unchanged from the three-platform accepted 15927aa3 source,
run 37920112411. The new component version requires fresh tagged package gates.

## Historical delivery: 1.3.1 with Companion 3.1.0

Owner authorization 2026-10-09: merge accepted Worktree collaboration into main,
remove completed branches and obsolete diagnostic workflow history, integrate
the feature into existing bilingual README sections, update repository About,
and publish App 1.3.1 with Companion 3.1.0. Preserve historical tags/assets.

1. Fast-forward main to the accepted feature source and release preparation.
   Keep the two release and two validation workflows; remove the retired Codex
   diagnostic workflow's runs. Delete completed branch refs only after main
   contains their commits. No history rewrite, local Rust build or installation.
2. Set the independent Companion component version to 3.1.0; protocol major
   remains 1. Update existing download/install guidance and add release notes
   describing changes from 1.3.0. Tag immutable main source as v1.3.1.
3. Dispatch release-macos.yml with release_tag=v1.3.1 and auto_publish=true.
   Reuse all native installer/source/version/runtime checks. Windows follows
   automatically and publishes only after its package checks and upload pass.
4. Check cloud runs every 30 minutes. Verify five versioned public installers,
   both checksum lists, tagged source provenance and About/download links.
   Record completed delivery in MEMORY.md; fix failed gates without bypassing them.

Source acceptance before version bump: Companion run 37830659603 passes all
three remote platforms. Desktop run 37830653334 passes macOS and Windows
protocol/default/platform/application regressions; Windows runtime preparation
fails on an occupied identical DLL. Workflow-only fix 48f1ca4a passes focused
Windows native run 37841105097. Each desktop has 191 verified captures; the
overall full-run failure is retained in the Worktree contract.

### 1.3.1 public delivery receipt

Published [Warpai 1.3.1 with Companion 3.1.0](https://github.com/OthinusG/warpai/releases/tag/v1.3.1)
as latest on 2026-10-09. Immutable tagged source:
`0312ac0f9ed6d9781fcd0b52c9f8d0b4d1c0e4c9`.

- [macOS and three-platform Companion release](https://github.com/OthinusG/warpai/actions/runs/37881956251)
  succeeds on attempt 2, preserving successful first-attempt Companion installers
  while retrying the desktop job after dependency executable/disk I/O failures.
  No source tag was changed and no check was bypassed.
- [Windows desktop release](https://github.com/OthinusG/warpai/actions/runs/37888146809)
  passes application/installer version, icon, Agent bridge and package checks,
  then uploads and publishes the complete release.
- All five downloaded installers and two SHA-256 lists match GitHub digests.
  Windows public bytes match the native CI artifact and immutable source receipt.
  The checksum lists cover exactly all five installers. Public notes match the
  committed changelog; all five unauthenticated download URLs return HTTP 200.
- Read-only macOS DMGs pass identity, version, deep signature, icons, launcher,
  source/target and payload checks; Linux source/target/version and all payload
  hashes pass. macOS App, bridge, Companion and private Git binaries declare
  minimum macOS 11.0. Bilingual README requirements specify Big Sur or newer on
  Apple silicon; this deployment minimum is not an older-OS execution receipt.
- Completed branches and the obsolete Codex diagnostic workflow were removed.
  The four current release/validation workflows remain; historical releases,
  tags, screenshots and branding are preserved. About and bilingual README
  describe both modes within the existing product sections. No personal-machine
  installation, local Rust compilation or credential/configuration edit occurred.

## Historical delivery: 1.3.0 with Companion 3.0.0

The owner authorizes packaging and public release of App 1.3.0 and Companion
3.0.0. Summarize the net changes from the v1.2.0 source baseline in
docs/releases/v1.3.0.md. Preserve all earlier tags, drafts and release assets.

1. Integrate the accepted self-contained remote tools into main, update the
   component version and bilingual download links, and tag committed source
   as v1.3.0. Desktop bundle/PE versions derive from this tag; protocol major
   remains 1. No local Rust compilation or personal-machine installation.
2. Dispatch release-macos.yml with release_tag=v1.3.0 and auto_publish=true.
   Reuse native packaging, checksums, three-platform install/reinstall and
   private Git checks. Windows follows automatically from the same tag and
   publishes only after its package identity checks and uploads pass.
3. Verify completed jobs, the public five-installer inventory, matching versions,
   source provenance and checksum files. Record delivery in MEMORY.md. Fix any
   packaging failure before publishing; do not bypass existing release gates.

Existing desktop builds/tests and native SSH Review/Code View assertions passed
on macOS/Windows. The corrected 170-image count passes against actual captures.
Three-platform self-contained Companion acceptance passed before this version
bump. Tagged release checks verify the new component version and formal packages.

### 1.3.0 public delivery receipt

Published [Warpai 1.3.0 with Companion 3.0.0](https://github.com/OthinusG/warpai/releases/tag/v1.3.0)
as the latest formal release on 2026-10-07. Immutable tagged source:
`e2964f3b49923ab44e219b7ece0ae6071f9fe3a6`.

- [macOS and three-platform Companion release](https://github.com/OthinusG/warpai/actions/runs/37606929158):
  all four jobs pass, including source-matched native installers, install/reinstall,
  private Git without system Git on PATH and Windows installed release protocol.
- [Windows desktop release](https://github.com/OthinusG/warpai/actions/runs/37613962878):
  application numeric/string versions, installer ProductVersion 1.3.0, icon,
  Agent bridge and package contents pass before automatic public delivery.
- Read-only inspection of downloaded macOS DMGs passes App version/identity,
  strict deep signature, Agent bridge, canonical artwork and Applications link;
  Companion source/target/version, every payload checksum and Finder launcher
  icon pass. Linux source/target/version and complete payload checksums pass.
- Public delivery contains five versioned installers and two SHA-256 lists.
  All public GitHub installer digests agree with those lists; Windows's public
  digest matches the downloaded verified CI installer and its tagged-source
  receipt. The public notes match the committed changelog exactly.
  Release notes cover the v1.2.0 baseline through v1.3.0; earlier tags and assets
  are preserved. No local Rust build or personal-machine installation occurred.

## Historical delivery: 1.2.2

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
