# warp-lite Agent Guide

## Purpose

Maintain a local-first Warp fork that preserves the terminal experience while excluding AI, telemetry, cloud account, and billing product surfaces.

## Project Rules

- Read `README.md` and `MEMORY.md` before changing the repository.
- Preserve the terminal core guardrails listed in `README.md`.
- Prefer small, auditable changes and existing upstream implementations.
- Do not restore AI, telemetry, cloud account, billing, or login dependencies when syncing upstream features.
- Keep the default build and `warp_platform` build compiling when relevant.
- Use the package manager and toolchain pinned by repository lockfiles.
- Never store credentials or secret values in source, documentation, logs, or memory.

## Verification

- Run focused tests for changed logic.
- Run `cargo check -p warp --bin warp-oss` for Rust application changes when feasible.
- Validate shell scripts with `bash -n` or `zsh -n`, matching their shebang.
- Validate workflow YAML and dry-run automation paths when available.
- Preserve unrelated user changes in a dirty worktree.

## Release

- Reuse `script/build-warp-lite-app.sh` and the existing GitHub Actions release workflow.
- Keep upstream synchronization auditable; prefer explicit commits or patches over opaque generated rewrites.
