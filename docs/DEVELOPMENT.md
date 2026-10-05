# Developing Warpai

**English** | [简体中文](DEVELOPMENT.zh-CN.md) · [Product introduction](../README.md)

Read [AGENTS.md](../AGENTS.md) and [MEMORY.md](../MEMORY.md) before changing the
repository. The user-facing introduction and download choices are in
[README.md](../README.md).

## Scope

- Desktop applications: macOS and Windows x64.
- SSH project environments and the companion: Linux, macOS and Windows.
- Preserve terminal rendering, input, shell integration and the existing editor.
- Keep the default application and `warp_platform` checks passing when relevant.
- Do not restore bundled cloud AI, account/login, billing or telemetry surfaces.
- Source is maintained directly. Do not add upstream synchronization, patch
  replay or restoration steps to builds.

The terminal core guardrails remain in the
[README contributor section](../README.md#develop-and-contribute).

## Toolchain and builds

Use the Rust toolchain pinned by [rust-toolchain.toml](../rust-toolchain.toml) and
dependencies pinned by [Cargo.lock](../Cargo.lock). Fetch Git LFS assets before
building. macOS builds require full Xcode and its Metal toolchain.

This maintainer workspace runs Rust compilation, tests and packaging on GitHub.
Local documentation, script syntax, formatting and static checks do not require
a Rust build. Commands below describe the build steps for configured build hosts:

```sh
cargo check -p warpai --bin warpai --locked
cargo check -p warpai --bin warpai --features warp_platform --locked
cargo build --release -p warpai --bin warpai --locked
cargo build --release -p warp-agent-bus --bin warpai-agent --locked
```

For macOS packaging, reuse
[build-warpai-app.sh](../script/build-warpai-app.sh). For Windows, reuse
[build-warpai-windows.ps1](../script/build-warpai-windows.ps1) and the
[installer resources](../script/windows/README.md). Preserve existing bundle and
storage identifiers when changing visible branding.

## Validation

- [Desktop communication workflow](../.github/workflows/validate-agent-communication.yml):
  macOS/Windows application checks, focused regressions and optional native UI
  captures and debug review builds. `protocol_only` skips desktop checks unless
  `check_app` or `capture_ui` is enabled; `capture_ui` builds isolated debug apps,
  not installers. Validation does not package desktop releases.
- [Remote companion workflow](../.github/workflows/validate-remote-companion.yml):
  three-platform protocol/process tests, controlled Linux OpenSSH acceptance and
  companion artifacts with source/digest manifests.
- [Current acceptance record](../specs/agent-communication-v2/QUALITY.md):
  replacement decisions, cleanup scope and exact-source verification.

Run checks appropriate to the changed logic. Validate shell scripts with
`bash -n` or `zsh -n` according to their shebang. Use targeted Rust formatting:
repository-wide formatting can encounter retained references to disabled
upstream modules. Do not reformat unrelated source to repair a small change.

## Releases and provenance

The [macOS release workflow](../.github/workflows/release-macos.yml) builds an
existing repository tag, packages the app and three-platform release companions,
and creates a draft. The
[Windows workflow](../.github/workflows/release-windows-x64.yml) uses the same tag
and attaches verified installer/portable artifacts before publishing the draft;
it also supports explicit dispatch. Set `GIT_RELEASE_TAG=v1.0.1` for macOS
packaging or pass `-ReleaseTag v1.0.1` to the Windows builder. Product package
versions derive from that tag. See [the release contract](../specs/RELEASE.md).
Validation artifacts are debug review builds and captures, separate from tagged
published releases. Installer/app ZIP/DMG packaging belongs to these release
workflows.

The default branch is `main`. Preserve copyright, license notices and
historical provenance. Do not delete unrelated unmerged work or active PR heads.

Historical terminal integration audits remain in
[WARP_LITE_SYNC_2026-08.md](../WARP_LITE_SYNC_2026-08.md) and Git history. They are
engineering records, not the current product's feature list.
