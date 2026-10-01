# warp-lite Agent Guide

## Purpose

Maintain Warpai as an independently maintained local-first terminal derived from Warp, excluding bundled cloud AI, telemetry, cloud account, and billing product surfaces.

## Project Rules

- Read `README.md` and `MEMORY.md` before changing the repository.
- Preserve the terminal core guardrails listed in `README.md`.
- Prefer small, auditable changes and existing upstream implementations.
- Do not restore bundled AI, telemetry, cloud account, billing, or login dependencies.
- Source is maintained directly in this repository. Do not recreate upstream synchronization, patch replay, or restoration scripts.
- Keep the default build and `warp_platform` build compiling when relevant.
- Use the package manager and toolchain pinned by repository lockfiles.
- Never store credentials or secret values in source, documentation, logs, or memory.
- Target macOS and Windows only. Linux is not a product target; do not add Linux implementations or tests. Linux-specific paths and CI jobs may be removed when they touch the requested work. Preserve unrelated upstream terminal code.

## Verification

- Run focused tests for changed logic.
- Run `cargo check -p warp --bin warp-oss` for Rust application changes when feasible.
- Validate shell scripts with `bash -n` or `zsh -n`, matching their shebang.
- Validate workflow YAML and dry-run automation paths when available.
- Preserve unrelated user changes in a dirty worktree.

## Release

- Reuse `script/build-warp-lite-app.sh`, `script/build-warp-lite-windows.ps1`, and the standalone GitHub Actions release workflows.
- GitHub builds tagged repository source directly; retain provenance, copyright and license notices.
