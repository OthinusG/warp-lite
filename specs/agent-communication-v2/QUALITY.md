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

The whole workspace consumer audit found no removable crate. Remove 29
unreferenced direct dependency edges from 18 manifests and only the newly
unreachable `line-span 0.1.5` lock entry; retain all other locked versions.
Preserve feature-driven, macro-generated and native linking dependencies.

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
has passed both OS default/platform checks, application regression and native
actions/capture steps. Each OS produced 155 verified PNGs with exit code zero,
no failed native steps and the matching source SHA. Eight live SSH states per OS
plus narrow/light/zoomed task views were visually reviewed. Release-package
steps continue for that immutable source while cleaned source is verified on a
separate branch.

Status: replacement functional acceptance complete; cleaned-source verification pending.
