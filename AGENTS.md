# Warpai Agent Guide

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
- Local desktop targets are macOS and Windows. SSH Remote project environments and the remote companion target Linux, macOS and Windows, explicitly authorized on 2026-10-03. Linux desktop/UI implementations remain out of scope; Linux remote-service builds and focused tests are allowed. Preserve unrelated upstream terminal code.
- SSH remote file transfer uses SFTP over SSH only. Direct local WSL uses bounded Companion stdio file chunks, authorized 2026-10-10. Do not add FTP/FTPS profiles, dependencies or fallback. The active SSH Remote scope is defined in specs/agent-communication-v2/PLAN.md; archived enrolled-device collaboration designs are not implementation instructions.

## Verification

- Run focused tests for changed logic.
- Run `cargo check -p warpai --bin warpai` for Rust application changes when feasible.
- Validate shell scripts with `bash -n` or `zsh -n`, matching their shebang.
- Validate workflow YAML and dry-run automation paths when available.
- Preserve unrelated user changes in a dirty worktree.

## Release

- Reuse `script/build-warpai-app.sh`, `script/build-warpai-windows.ps1`, and the standalone GitHub Actions release workflows.
- GitHub builds tagged repository source directly; retain provenance, copyright and license notices.
