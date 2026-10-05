# Warpai 1.0.1 release

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
