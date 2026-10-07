# Review baseline and SSH file tools

Date: 2026-10-07. Status: source restored; focused checks passed; desktop build/runtime blocked locally.

## Required outcome

Restore Code Review presentation, entry points and application logic to the
committed `v1.0.1` baseline. The owner's SSH requirement is existing Explorer
file management and in-app file opening/preview parity. The SSH Git Review
extension and toolbar enablement added later are outside this scope.

This correction supersedes the Git Review requirements in REMOTE-FILE-TOOLS.md.
Agent task review/acceptance is a separate collaboration workflow and remains
within the accepted product scope.

## Tasks and constraints

- Restore `app/src/code_review/`, `workspace/view/right_panel.rs` and
  `workspace/header_toolbar_item.rs` to their v1.0.1 contents.
- Remove the SSH Review setup caller and native capture steps that open it.
- Retain confirmed SSH cwd selection, Explorer metadata/mutations, SFTP content
  transfer, original editor/save identity and existing preview/resource handling.
- Retain Git-root detection used by Explorer; do not remove shared file backends
  or protocol compatibility merely because Review no longer consumes them.
- Align current product documentation and record the owner correction in memory.

## Acceptance and verification

- Restored Review files match v1.0.1 byte for byte; no new SSH Review callers or
  capture steps remain. The toolbar support gate hides Code Review.
- Existing native Explorer/editor/dirty-save/Markdown-relative-image steps remain,
  and explicitly assert that Code Review has no supported toolbar entry.
- Trace remote cd, list/create/rename/delete, click/open, preview/resources and
  version-checked save through the existing local UI components.
- Run focused file-backend/Companion tests, diff hygiene and Rust formatting;
  attempt the application check. Record actual runtime/build limitations rather
  than treating earlier source acceptance as verification of this revision.

## Audit and results

- Review source, right panel and toolbar match the v1.0.1 tag byte for byte.
  The added SSH setup caller and Review capture steps are removed. The native
  Explorer checkpoint now asserts that the Review toolbar entry is unavailable.
- Explorer selects the confirmed SSH terminal cwd, obtains remote directory
  metadata, and applies it to the existing file tree. Poll generations reject
  stale results. Create, rename and delete use the existing remote file actions.
- Clicking remote code/text or Markdown downloads content over SFTP, registers
  its original remote identity with FileModel and enters open_file_with_target.
  Existing editor/layout/Markdown preference handling is reused. Markdown images
  and linked documents resolve against the original remote document.
- Saving uses the original remote attachment/path and observed content hash;
  changing terminal or cwd does not redirect an already-open document's save.
  Failed saves retain the editor buffer. Shared Companion Git protocol methods
  remain for compatibility and Explorer Git-root discovery, without a Review UI.
- This parity covers the built-in text/code editor and Markdown renderer. It
  does not establish remote support for every local external application or
  arbitrary binary format; remote paths must not be passed as local OS paths.
- `companion::file_tests`: 17 passed. `ssh_files::tests`: 6 passed, including
  system SFTP literal-path and binary-byte transfer. Diff hygiene and Rust parser
  checks passed; restored files were not reformatted away from the baseline.
- Default and warp_platform desktop cargo checks fail in warpui's build script because xcrun
  cannot find the Metal compiler. Native GUI checkpoints have not been run for
  this revision. Existing published installers were not changed.
