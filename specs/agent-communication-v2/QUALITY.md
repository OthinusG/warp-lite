# SSH communication quality and integration gate

Date: 2026-10-04. Baseline: accepted source `4fcb0c3`, documentation `1ae36e8`.
Scope: PLAN S0–S5, followed by repository-wide proven-unused-code cleanup and
fast-forward integration into `warp-lite/main`. Preserve all accepted behavior.

## Specification and tasks

| Item | Implementation decision | Verification |
| --- | --- | --- |
| S0 | Preserve completed removal of the redundant SSH manager/device runtime. Remove only source/dependencies proven unused across supported features. | Local communication, migration/history tests; default/platform checks. |
| S1 | Reuse the private account service, native root identity and existing Broker/Store. | Same/different root, root replacement, service identity and receipt isolation. |
| S2 | Keep the locked rmcp SDK and existing per-run environment/adapter contract. | Native initialize/tools discovery, revoked and replaced-run capability rejection. |
| S3 | Keep native PTY group/job ownership and existing vendor launch adapters. | Linux/macOS/Windows real process, background exit and controlled OpenSSH two-Agent message/task/review. |
| S4 | Keep existing native panel, transient SSH selection, original human intents and generation fences. | macOS/Windows native actions, static/live screenshots, drafts and reconnect. |
| S5 | Replace the handwritten request-ID varint length decoder with the already locked prost decoder. | Reject overflowing/truncated varints; retain valid multibyte IDs and trailing-corruption extraction; complete both workflows. |

The selected replacement is the public API
[`prost::encoding::decode_varint` at v0.14.3](https://github.com/tokio-rs/prost/blob/v0.14.3/prost/src/encoding/varint.rs).
No new dependency, wire format, database schema or UI design is required.
Checked integer conversion must reject lengths unrepresentable on the host.
The original decoder incorrectly discards high bits in the tenth varint byte,
allowing a malformed length to be reported as a short request identifier.

Other candidate implementations cannot directly preserve all accepted contracts:
the openssh crate is Unix-only; portable-pty termination tracks individual children
rather than the required complete group/job lifecycle; generic coordination servers
would change the task/receipt model and require migration. Candidate evidence and
the comparison viewer are temporary artifacts outside this repository.

## Acceptance and release order

1. Preserve the accepted worktree and provenance, record replacement decisions.
2. Replace the decoder and leave a focused runnable Rust regression.
3. Run remote and desktop GitHub validation for the exact final source; Rust
   compilation remains GitHub-only. Review native screenshots and package manifests.
4. Audit the repository for unused code. If cleanup changes executable source or
   build inputs, rerun affected gates before integration.
5. Fast-forward the existing default branch; never force-push over remote changes.
6. Delete obsolete branches only after preserving refs in a local Git bundle and
   proving their tips are included in the accepted default branch. The exact retired
   SSH cutover tip `b149e30` is also removable: its only unique changes are the
   obsolete metadata/SFTP manager and the ConPTY standard-handle correction
   already present in accepted source. Retain other unmerged work and active PR
   branches; remove obsolete CI branch triggers.

Completion means passing final-source protocol/real-process/local-regression
tests, both default/platform desktop checks, native panel actions and screenshot
review, plus matching default-branch source and audited branch cleanup. Paid vendor
sessions, physical-host acceptance and publishing a new tagged release are separate
from this engineering gate.

## Repository cleanup

The whole workspace consumer audit found no removable crate. Remove 27
unreferenced direct dependency edges from 17 manifests and only the newly
unreachable `line-span 0.1.5` lock entry; retain all other locked versions.
Also remove seven unconsumed workspace dependency declarations from the root
manifest: ashpd, backtrace, three objc2 platform aliases, parquet and warp.
These entries contribute no dependency edges or features to any member;
retain the actual direct/transitive platform dependencies and profile settings.
Preserve feature-driven, macro-generated and native linking dependencies,
including num-traits required by num-derive expansions in warpui_core and
log required by safe_info expansions in vim.

Remove seven unwired files: the old GetFiles executor, debug block model, queued
query types, empty legacy navigation file, invalid unused usage module, and
exact duplicate AppId/meta sources. Their active implementations/tests remain.
Preserve the unrelated upstream terminal test file. Keep the local CodeGraph
index on disk and ignore it in source delivery.

Update CI triggers and README to the accepted default branch. Re-run both
validation workflows for cleaned source before default-branch integration.

## Verification record

Replacement source `5ae1afd`: remote workflow
[37147192504](https://github.com/OthinusG/warp-lite/actions/runs/37147192504)
passed Linux/macOS/Windows protocol regression and clippy, owned native process
lifecycle and the controlled Linux OpenSSH message/task/review gate.
Desktop workflow
[37147195277](https://github.com/OthinusG/warp-lite/actions/runs/37147195277)
passed both OS default/platform checks, application regression, native actions,
155 screenshots per OS and both release-package steps. Eight live SSH states per
OS plus narrow/light/zoomed task views were visually reviewed before cleanup.

Cleaned executable/build source `8b8993801a696972ae0ec853d40f9c6cc8e0f582`:
- [Remote run 37152973196](https://github.com/OthinusG/warp-lite/actions/runs/37152973196)
  passed all three platforms, including protocol clippy with warnings denied,
  the overflowing-varint regression, owned process lifecycle and controlled Linux
  OpenSSH two-Agent message/task/review. 31 successful suites recorded 277 test
  executions, with no failures. All three companion source/digest/license
  manifests were checked.
- [Desktop run 37152975773](https://github.com/OthinusG/warp-lite/actions/runs/37152975773)
  passed macOS/Windows default and warp_platform checks, 45 successful suites
  recording 246 test executions, native actions, captures and release packaging.
  Both diagnostics match this source, exit zero and contain no failed steps.
  Each OS has 155 verified PNGs; the eight SSH states and narrow/zoomed task views
  preserve drafts, disable disconnected writes and display reconnect state.

Test counts are executions across platforms, not distinct test cases. Rust
compilation/testing ran only on GitHub. The optional long-soak jobs were not
requested in this quality gate. Documentation-only integration commits preserve
all executable source and build inputs from the tested source above.

Integration: `903a26e` fast-forwarded `warp-lite/main`. The remote now contains
only that branch. Deleted 59 obsolete remote branches and four local branches
after verifying a full-history bundle and exact-tip ref audit at
`/Users/wqin/.codex/backups/warp-lite/20261004/`. Release tags and the local
CodeGraph index remain. No other unmerged work or active PR heads existed.

Both final desktop packages were downloaded from the immutable desktop run.
The macOS bundle contains ARM64 application and Agent bridge, preserved bundle
identifier, signature files and AGPL notice. The Windows artifact ZIP passed CRC;
the portable package and installer contain valid executable headers, and the
application/bridge/ConPTY/OpenConsole binaries are x86_64. Required runtime DLLs,
bootstrap and resources are present. These are package-structure checks, not
installation on physical desktops.

Status: replacement, cleanup, final functional/native UI acceptance, artifact
inspection and default-branch/branch cleanup complete.
