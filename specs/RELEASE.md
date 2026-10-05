# Warpai 1.0.1 release

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
   validated installer/portable packages and publishes the complete draft.
5. Verify public release assets, checksums, identities and documentation links;
   record acceptance and delivery in MEMORY.md.

## Packaging checks and risks

- macOS ZIP/DMG must contain Warpai.app, `dev.warpai.Warpai`, version 1.0.1,
  the reviewed ICNS and the local MCP bridge. Verify ad-hoc signing and DMG.
- Windows installer and portable ZIP must contain Warpai.exe, required native
  resources and the local MCP bridge. PE numeric/string versions and installer
  version must be 1.0.1; the portable icon must match the reviewed ICO.
- Remote packages must identify actual Rust target and tagged source, pass their
  native version invocation, and have verified SHA-256 manifests.
- Ad-hoc macOS signing and unsigned Windows installers can require OS approval;
  describe actual installation behavior without promising notarization.
- A failed packaging job leaves a draft rather than a partially public release.
  Preserve existing assets/history and fix failures before publication.

## Verification receipt

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
- Packaging-version source `ade83082b101ac624b62d90ea2c25d1a46b4fc65` passed
  [desktop run 37270362536](https://github.com/OthinusG/warpai/actions/runs/37270362536)
  on macOS and Windows, including both application configurations and all retained
  focused regressions. Subsequent pre-tag changes affect docs/screenshots and
  release-only archive checks, not application or companion runtime inputs.
