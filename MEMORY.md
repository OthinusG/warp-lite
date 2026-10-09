# Project Memory

## About update checks — 2026-10-09

- Owner requests an Update button and Check for updates on startup in About.
  Use public OthinusG/warpai stable releases, existing native widgets/local
  settings/toasts, and existing HTTP/runtime dependencies. Keep upstream Warp
  cloud updating disabled. Startup checks default off and run once when enabled;
  checks never modify installed files. Owner correction: Update checks; a separate
  Download button opens the exact published installer for the current OS/arch.
  Match public asset names and construct fixed-repository URLs; unsupported or
  missing packages use a Releases fallback, never the wrong installer.
- Accept only stable application vMAJOR.MINOR.PATCH tags, with numeric comparison,
  fixed HTTPS endpoint, timeout and bounded response. Do not send Warp headers or
  trust response URLs. Version tests and native About fixed-state captures are
  added to desktop validation; current-source acceptance remains pending.
- Native capture totals must reflect retired manual join/leave screens (-4),
  new empty-project/switch/return screens (+3), and 24 About state frames. Update
  both platform count assertions together; the current fixed addition is 110.
- Source 3708d389 passed all remote workflow jobs, including controlled Linux SSH.
  Desktop checks found one About EventContext URL call; route it through the
  typed About action before continuing build/capture acceptance.
- Source 15927aa3 remote run 37920112411 passes Linux/macOS/Windows, including
  controlled SSH. Later changes affect only About, desktop mode controls and
  native screenshot expectations; backend source remains identical. Desktop
  source be5810d3 passes both default/platform builds in run 37921826849; app
  tests and native visual acceptance are still pending. Final 0dfcf3ac also makes
  clicks on the already-selected mode inert and needs desktop verification.
- macOS be5810d3 passes application tests, including version/installer selection,
  then fails the native live-detail scrolling assertion: the simplified 600px
  layout fits without overflow. Keep the real scrolling check at 320px/1.25 zoom
  instead of adding product clutter. Add native no-op and non-Git mode assertions;
  Worktree creation/switch/return already passed before that capture failure.
- Native visual review found raw attempt-owner UUIDs in task details. Reuse the
  existing participant label lookup, including offline historical names, instead
  of displaying protocol identity in that user-facing row.
- Native 2009d9cd passes the corrected narrow scrolling and mode edge assertions,
  then panics in a retired participant filter fixture that searches the online
  list for the ended capture-worker. Resolve its ID from historical participant
  names, keeping online-only product behavior. Non-Git probe Git exit128 is
  incidental diagnostic context, not that panic cause.
- Windows native creation captures stopped while BranchPicker was still loading;
  the debug fixture had bypassed the native workspace catalog and forced a branch.
  Populate the owned workspace through the existing folder-picker path, await
  the real main-branch selection, and allow native tab startup 45 seconds. Keep
  the actual new-checkout/Coordinator assertions and report their current names.
  Both platform native walkthroughs and About captures remain pending.

## Worktree behavior correction — 2026-10-09

- Audit correction: project identity must be established at native launch, before
  registration/tasks. Native worktrees live under the managed data directory,
  outside the main checkout; ancestry and basename cannot establish membership.
  Coordinator changes preserve identity/task ownership and provide handoff plus
  future task updates. A same-name restart cannot inherit Coordinator authority.
- Communication mode is a project setting independent of Coordinator selection,
  persisted with existing Store metadata. Panel reads do not change permissions;
  Project lists include all online project participants across checkouts. Remove
  remote candidate truncation and update MCP instructions to automatic membership.
- First source a5ed10d3 fails two protocol regressions (remote origin-ID matching
  after automatic enrollment, and noncanonical macOS fixture path). Corrections
  and the expanded identity/mode/handoff coverage await source-matched cloud runs.
- Source bed129ce passed the expanded identity/mode/handoff and online pagination
  tests on macOS/Windows. One legacy remote restart assertion still expected a
  private directory scope; align it with automatic membership and assert that
  restart grants only Worker authority. Application/UI checks remain pending.
- Coordinator handoff sends a bounded unfinished-task count and existing paginated
  tool instructions, rather than concatenating descriptions that can exceed the
  message limit and block selection. Include tasks beyond the first history page.
- Empty native worktree tabs resolve the existing repository project even when
  queried in Project mode before their first Agent starts. Keep legacy opted-out
  or active private sessions isolated; a read does not create checkout admission.
- Non-Git directories remain independent canonical-root Projects. Worktree reads
  report unavailable and restore Project mode; they neither inherit nor clear a
  repository's Coordinator. Include cross-project denial and return-to-repository
  regression coverage (owner reminder on 2026-10-09). Native Worktree mode
  controls must honor that availability flag before mutation, so non-Git folders
  keep Project communication instead of reporting a spurious connection error.
- Source ddfc5b7b passed all library tests (including non-Git, empty checkout and
  bounded handoff coverage) and coordination integration tests on macOS/Windows.
  Integration corrections remain: online pagination/readiness expectations and
  native-only name collision suffixing. Generic shared workspaces must retain
  their existing cross-checkout identity-reclaim denial. Desktop/UI checks pending.
- Source 0ab4c54d passed macOS/Windows remote jobs and all library/coordination
  tests. Cloud compilation identified a toolbar branch placed in close_button;
  move it to render_button. Controlled Linux SSH must now expect same-repository
  worktrees to share Project history, while retaining native terminal/file fences.

- Owner supersedes manual worker binding and exclusive checkout ownership: reuse
  Warp native worktree creation/tab configurations; one project spans the Git
  common-directory identity and all registered checkouts/subdirectories. Same
  checkout Agents communicate directly; cross-checkout messages go through the
  selected Coordinator. Keep separate clones and SSH authorities isolated.
- Any online Agent in any project checkout can become Coordinator at any time.
  Switching demotes the prior Coordinator without ending its process or tasks.
  Selection belongs to the project/native run and survives pane/tab/project
  focus changes; only process termination/revocation or app shutdown clears it.
- Project participant lists exclude offline Agents before pagination. Preserve
  history and original native checkout/file authority during enrollment.
- Owner requires prominent Project/Worktree controls, removal of nonessential
  option buttons and consistent row/section spacing. Reuse native theme/widgets
  and fixed narrow/light/dark capture fixtures; no second worktree manager.
- Schema v9 removes checkout uniqueness after a consistent pre-upgrade-v8 backup.
  Source parsing and diff checks are local; Rust/native verification stays on
  GitHub. Current implementation verification is pending.

## Website deployment — 2026-10-09

- https://othinusg.github.io/warpai/ is live on GitHub Pages from the orphan
  `gh-pages` branch, built natively by Pages. Bilingual landing page (commit
  16984f89): EN at `/`, ZH at `/zh/`, light/dark theme with manual toggle and
  no-flash inline script (localStorage `warpai-theme`; `?theme=dark|light` URL
  param for QA), hreflang/canonical/og tags, jekyll-sitemap. Responsive
  container tiers min(1180px,100%-40px) / 680px (801-1100) / 560px (≤800).
  Main branch and its workflows stay untouched; HTTPS enforced;
  deploy-from-branch. No Gemfile or workflow needed; Pages rebuilds on every
  push to `gh-pages` (no [skip ci] needed there — nothing else triggers on it).
- i18n: static dual pages; shared chrome copy in `_data/strings.yml` keyed by
  `page.lang`. Jekyll 3.9 layouts do NOT expose their `{% assign %}` variables
  to page content — each page must assign its own
  `s = site.data.strings[page.lang]` (missing this silently empties all copy).
- Fonts: self-hosted Glow Sans subsets (welai/glow-sans v0.93 SC-Normal, OFL),
  6 woff2 for weights 600/700/900 × latin/CJK (162KB total), split by CSS
  unicode-range so EN pages skip CJK files. Pipeline: otf2ttf, then pyftsubset
  on the TTF — subsetting the original CFF/OTF leaves ~112KB constant dead
  weight per file. Subsets only cover characters used on the current pages:
  re-subset and re-verify cmaps whenever site copy changes.
- Local Jekyll build (system ruby 2.6 on macOS): jekyll 3.9.5 installed
  `--user-install --ignore-dependencies` plus pinned pure-Ruby gems (sass
  3.4.25, jekyll-sass-converter 1.5.2, kramdown 1.17.0, jekyll-sitemap 1.4.0,
  i18n 0.9.5, addressable 2.8.7, public_suffix 5.1.1, concurrent-ruby 1.1.10),
  with em-websocket/jekyll-watch dependency lines removed from the installed
  jekyll gemspec (serve-only, native deps). Use `--baseurl ''` for local
  preview only; production absolute URLs require the configured `/warpai`.
- QA: Brave headless screenshots. Chromium headless enforces a 500px minimum
  window width; verify true 375px layouts via a 375px-wide iframe.
- Visual system replicates the getaigist.com layout/color/typography tokens;
  none of its copy or assets are reused.

## 1.3.1 public delivery — 2026-10-09

- Published v1.3.1 as latest with Companion 3.1.0. Immutable tagged source:
  0312ac0f9ed6d9781fcd0b52c9f8d0b4d1c0e4c9. Main contains the accepted
  Worktree collaboration and preparation; later commits record docs/receipts.
  Historical tags, releases, screenshots and branding remain unchanged.
- Formal macOS/three-platform Companion run 37881956251 succeeds on attempt 2;
  retry only the desktop job after first-attempt dependency executable/disk I/O
  errors. All Companion installer/runtime jobs pass on the first attempt.
  Windows run 37888146809 passes package/version/icon/bridge checks and publishes
  the complete draft at 2026-10-09 06:27:56 UTC.
- Downloaded all five installers and both SHA-256 lists; every GitHub digest and
  checksum matches, with exact five-installer coverage. Windows public bytes
  match the native CI artifact and tagged-source receipt. Public inventory is
  exactly five installers plus two checksum files; notes match the committed
  changelog and all five unauthenticated download HEAD requests return HTTP 200.
- Both read-only macOS DMGs pass verification, artwork/volume icon and payload
  checks. App 1.3.1 identity, strict deep signature and Agent bridge pass;
  Companion 3.1.0 native version/source/target and every payload checksum pass.
  Linux payload source/target/version and all file hashes pass. Desktop, bridge,
  Companion and private Git Mach-O binaries declare minimum macOS 11.0, matching
  bilingual README requirements for Apple silicon; macOS 11 hardware execution
  was not tested. No personal app, Agent or SSH configuration was installed.
- Only main remains on origin. Three completed development branches and retired
  Codex diagnostic workflow history were removed; two release and two validation
  workflows remain. Repository About describes Project/Worktree coordination
  and links to Warpai's current downloads. README integrates the modes into
  workflow/example/panel/setup/remote sections without an appended update list.

## 1.3.1 release authorization — 2026-10-09

- Owner requires minimum macOS version in README download requirements. Published
  macOS packages are Apple silicon only: report macOS 11.0 (Big Sur) or newer,
  rather than copying the generic 10.14 deployment/plist setting. Formal 3.1.0
  Companion and bundled Git Mach-O LC_BUILD_VERSION both declare minos 11.0.
  Confirm the desktop executable and bridge in the formal DMG before final
  delivery; this is a deployment minimum, not an older-OS runtime test.

- Owner authorizes merging accepted Worktree collaboration into main, removing
  completed branches and obsolete workflows, integrating feature descriptions
  into existing README sections, updating repository About, and publishing App
  1.3.1 with Companion 3.1.0. Keep protocol major 1 and historical tags/assets.
- Reuse the two native release workflows and two validation workflows: all four
  serve current gates. Retired diagnose-codex-arguments.yml is already absent;
  remove only its four obsolete diagnostic runs, not release/acceptance receipts.
  The two old remote-tools branches are already ancestors of main; remove their
  refs and the feature ref after integration. About homepage must use war pai's
  current repository URL, not the earlier warp-lite URL.
- README modes belong in product workflow, development example, panel/setup,
  remote and use-case sections, not an appended changelog. Published screenshots
  remain unchanged. Formal release/installer gates remain required after the
  component version bump. No local Rust build or personal installation.
- Main integration and immutable v1.3.1 tag both point to
  0312ac0f9ed6d9781fcd0b52c9f8d0b4d1c0e4c9. Formal macOS/three-platform
  Companion run 37881956251 started with auto_publish=true; Windows follows
  after success. Publication remains pending. Only main remains on origin;
  completed feature/remote-tools refs were removed after ancestor checks.
  Retired Codex diagnostic workflow disappeared after removing its four runs;
  the four current release/validation workflows remain. About now describes
  Project/Worktree coordination and links to the current repository's downloads.
- Formal run 37881956251 first attempt passes all three Companion 3.1.0
  installer/runtime jobs. macOS desktop fails in unrelated dependency builds:
  unicode-general-category build executable cannot execute (126) and ranlib
  reports an output-file I/O error. Retry only failed jobs on a fresh runner,
  preserving the immutable tag and successful Companion artifacts; no release
  is public yet. This is runner failure evidence, not an application type error.

## Cross-worktree collaboration — 2026-10-08

- Final acceptance recorded 2026-10-09: application/Companion source 06f0ab15
  passes all three remote targets in run 37830659603. Desktop run 37830653334
  passes macOS including 191 captures and Windows protocol/default/platform/
  focused application regressions; its overall failure is DLL preparation.
  Workflow-only fix 48f1ca4a passes focused Windows run 37841105097, including
  owned SSH/SFTP and 191 native captures. Downloaded diagnostics are clean on
  both desktops; narrow light/dark and actual create/unassigned/bind frames
  reviewed. Acceptance contract records exact receipts and the red full-run
  limitation rather than calling that run successful. SSH Worktree GUI-specific
  capture and paid vendor/model execution are not claimed; backend fixtures
  enforce remote orchestration. Feature remains unmerged/unreleased on
  feat/worktree-collaboration; no personal installation was changed.

- Owner requests focused cloud checks while debugging (2026-10-08): avoid
  repeating hour-long full builds. `native_windows_only` skips macOS and redundant
  protocol/history/default/platform checks while retaining native Windows and
  owned SSH/SFTP acceptance. Cache pinned-toolchain/lockfile Cargo outputs before
  builds and save before UI capture; finish with a complete verification run.
- Owner requests checking cloud runs once every 30 minutes, without frequent
  polling or progress messages. Focused run 37798319072 (49632d2a) fails after
  about 35 minutes; its native diagnostic artifact is not yet inspected because
  the local command tool cannot spawn processes (OS resource exhaustion).
  Command execution later recovered; downloaded diagnostics identify private Git
  stdout collection timing out (stage 1, unavailable), causing Join to remain
  unsubmitted. Use CREATE_NO_WINDOW for Windows background Git; verify through
  the existing failing native capture. The first 4.2 GB Rust cache was saved.
  Run 37811301758 (bfe81384) passes Windows Worktree enrollment, Coordinator,
  creation and binding; no Git read timeout recurs. It reaches 169 screenshots
  then the native remote fixture cannot start: focused mode omitted compilation
  of the managed_agent test executable. Build that fixture with --no-run while
  keeping the full protocol suite skipped.
  Focused run 37824647901 (06f0ab15) passes all Windows native capture assertions
  and owned SSH/SFTP acceptance: 191 PNGs, no failed steps, assertions or private
  Git errors. Reviewed narrow light/dark layouts and actual creation/selection/
  binding frames. Full desktop run 37830653334 and three-platform Companion run
  37830659603 were started for final verification. The Companion run passes
  on all three platforms; macOS passes the full desktop run. Windows protocol,
  default/platform checks and application regressions pass, but capture runtime
  preparation cannot overwrite vcruntime140.dll held by a prior test process.
  Reuse runtime DLLs only when SHA-256 matches the tracked source, preserving
  stale-file detection. Retry only focused Windows native acceptance; source
  application/Companion code is unchanged.

- Owner authorizes completing cross-worktree Agent collaboration locally and
  over SSH. Reuse existing worktree creation, messages, tasks, review and evidence;
  do not duplicate the communication stack. Active contract:
  specs/agent-communication-v2/WORKTREE-COLLABORATION.md.
- Owner correction: panel has Project / Worktree modes; Project preserves the
  previous behavior. In Worktree mode select a Coordinator from the project's
  active Agents. No active Agent means no Coordinator or coordination actions.
  Worktree creation never starts an Agent; users explicitly bind eligible active
  Agents in the panel to their actual worktrees. Coordinator assigns tasks,
  supplies context, collects results, requests peer review, chooses main-branch
  integration and makes the final commit. This supersedes automatic lead/worker
  launching; implemented cross-worktree messaging is only the foundation.
- Owner requires an uncluttered collaboration panel contained within GUI bounds.
  Use existing native components/themes; keep mode and Coordinator controls
  separate, show compact worker summaries, expand details on demand and verify
  narrow layouts, long labels, many workers and native macOS/Windows screenshots.
- Coordinator iteration adds schema v8 with an additive v7 backup, explicit
  run-bound roles, peer-review task reuse and Coordinator-only integration choices.
  Enrollment preserves native processes/checkout authority and prior Project
  records; unresolved Project work blocks enrollment. Remote original endpoints
  forward enrolled runs to the repository Broker and retain native lifecycle
  fencing. Worktree creation never launches a process. Compilation, regression
  tests, SSH/SFTP and native Companion packaging passed on macOS/Windows/Linux
  in run 37765513619 (19cb66d5), including exit, remote forwarding cleanup,
  disabled-program recovery, backward-compatible Project query serialization and
  retention of the native run's original captured checkout handle.
  Desktop default/platform builds and focused application tests passed before
  final panel-only refinements. Native macOS capture passed all 191 screenshots in run 37762499422
  (5c6848c9), and again in run 37774398871 (7ba8703f). Windows passes protocol
  and both application build variants but fails the native team-projection
  checkpoint; waiting for the real mode projection did not resolve it. Its
  screenshot reports invalid_input during Worktree reads. Cause remains
  unconfirmed. Diagnostics now export only source-owned state categories and
  Git exit codes, never paths, terminal payloads or credentials. Run 37782260820
  (f91cb6cb) failed both app builds because diagnostic code accessed a private
  Form field; corrected using its existing public submission state. Run
  37785380593 (926bc1dc) passes macOS and both Windows application checks.
  Windows diagnostics show a closed form but no admission, with successful Git
  reads and a connected panel. Normalize local native context paths to prevent
  Windows shell path-spelling updates from discarding a form; also assert the
  Join intent actually reaches submission. Run 37790608415 (37f60667) passes
  macOS but Windows shows the Join request was not submitted: panel.connected
  was false while the failure-time single query and Git commands succeeded.
  Path normalization alone is insufficient. Run 37796269498 (e5fdf6b3) retains
  the failing periodic-read category and private Git error/stage/exit codes;
  earlier single-query diagnostics did not retain the transient failure.
  Windows native acceptance remains pending. Local enrolled exits clear transient team binding;
  remote observed exits remove endpoint forwarding. Relaunch requires selection.
- Native Agents stay in Project on ordinary launch; panel enrollment explicitly
  assigns the live run to a repository team. Private history stays private. Git common-directory
  native identity separates independent clones; remote authority remains within
  one service/account/host, separate from local authority. File/Review roots stay
  physical and task acceptance does not automatically merge commits.
- Explicit project-space remapping replaces worktree admission and fences its
  captured actors; ordinary workspace admission remains usable. Commit evidence
  verification reuses the fenced private Git runner, including packaged SSH hosts.
- Three-platform remote protocol, worktree, persistence, private-runtime and Linux
  real-SSH acceptance pass in GitHub run 37723479969 (a20ead3e). Desktop build and
  native screenshot acceptance failed its capture fixture in run 37723483322;
  rechecked run 37740536908 also passes builds/tests but fails a stale workspace
  projection assumption at panel.rs:2982. The new iteration strengthens the
  capture assertion to wait for the named space's mapped checkout.
  Rust compilation stays on GitHub; no release or personal installation is implied.

## Repository cover — 2026-10-07

- Owner correction: cover copy must emphasize Agent management and communication
  between agents, not SSH, generic "Your CLI agents" or human team collaboration.
  Keep its visual style and use "Agent Management · Project Agent Communication"
  as the supporting line.
- Owner requests a shared README/Social preview cover incorporating the approved
  Warpai icon and the official Warp repository's gradient style. Both READMEs
  use docs/images/warpai-social-preview.jpg; application icons and native
  screenshots stay unchanged. Prompt and visual checks: docs/images/SOCIAL-PREVIEW.md.
- GitHub Social preview requires a separate Settings upload; current tools lack
  browser upload capability. A README image does not configure Social preview.

## 1.3.0 release authorization — 2026-10-07

- Owner authorizes packaging and public release of App 1.3.0 with Companion
  3.0.0, with release notes covering the net changes from v1.2.0 through v1.3.0.
  Desktop package versions derive from v1.3.0; component version is independent
  and protocol major stays 1. Supersedes the 1.2.2 trigger-only delivery scope.
- Reuse the existing macOS/three-platform Companion workflow and automatic
  Windows workflow with auto_publish=true. Preserve historical tags and assets;
  verify complete public delivery. No local Rust build or personal installation.
- Immutable v1.3.0 source: e2964f3b49923ab44e219b7ece0ae6071f9fe3a6.
  Formal macOS/Companion run 37606929158 passes all four jobs. Windows run
  37613962878 passes and publishes v1.3.0 as the latest public release.
  Four downloaded installer hashes pass. Read-only macOS DMG inspection passes
  App 1.3.0 identity/signature/Agent bridge/artwork and Companion 3.0.0 source,
  private runtime and all payload checksums; Linux payload provenance also passes.
  The Windows CI installer source receipt matches the immutable release tag.
  All five public installer digests match the checksum lists; Windows's published
  GitHub digest matches the downloaded verified CI installer. Public inventory
  contains exactly five installers and two SHA-256 lists, and notes match the
  committed v1.2.0-to-v1.3.0 changelog.

## Self-contained SSH project tools — 2026-10-07

- Owner authorizes original remote Review with the existing Explorer/Code View
  and preview, and corrects the dependency request: Companion installers must
  carry Warpai's own runtime dependencies, rather than desktop auto-deployment.
  Vendor Agent CLIs/authentication remain user-owned. System tray is explicitly
  deferred; keep on-demand startup and the existing 60-second idle shutdown.
- Active specification: specs/agent-communication-v2/SELF-CONTAINED-TOOLS.md.
  Reuse accepted original Review code and share exact-session/project RemoteFiles
  attachments. Native installers carry private Git without altering PATH or
  shell/SSH profiles. Subsequently published in App 1.3.0 with Companion 3.0.0;
  no installation on personal machines has been performed.
- GitHub run 37600588598 (f7c749a4) passes three-platform Companion protocol,
  files, native installation and private Git checks without system Git on PATH.
  Desktop run 37599409548 passes both default/platform builds, application tests
  and native UI assertions on macOS/Windows, producing 170 nonempty PNGs each.
  Its post-capture check remains red for an obsolete 171-image expectation;
  corrected workflow count passes against both downloaded actual artifacts.
  Review and Review-to-Code-View screenshots were inspected on both platforms.
  App sources did not change after their tested commit 65b5a9dc.

## Original Code Review entry restored — 2026-10-07

- Owner now requests restoring only the original Warp Code Review panel entry,
  superseding the earlier instruction to hide it. Enable the existing toolbar
  item under local_fs and align its native capture assertion. Keep the original
  panel implementation; do not restore the removed SSH Review extension.
- Source audit: restoring the entry does not enable SSH Git Review. The panel
  still uses path-keyed local DiffStateModel/Git commands, and its remote empty
  state says "Diffs only work for local workspaces." Remote Explorer/SFTP file
  opening is a separate data path. Native SSH Review was not runtime-verified.

## 1.2.2 release authorization — 2026-10-07

- Owner requests push and v1.2.2 release triggering only, with no subsequent
  monitoring. This supersedes manual package inspection for this release.
- Release-macos has an opt-in auto_publish input, default false. Its release
  target artifact carries the choice to the automatic Windows workflow; public
  publication occurs only after macOS/Companion success and Windows verification
  and upload. Dispatch v1.2.2 with auto_publish=true. Companion stays 2.0.0.
- Bilingual README download links and v1.2.2 notes target the new release.
  Triggering is not a claim that native builds or publication have completed.

## Code Review scope correction — 2026-10-07

- Owner explicitly does not want a Code Review panel. The SSH requirement is
  existing Explorer file management and clicking files into the existing editor
  and Markdown preview. Earlier notes treating SSH Review as required are
  superseded; do not re-enable Review to implement file tools.
- Restore Review source, right panel and toolbar to v1.0.1, which retains backing
  code but hides the toolbar entry. Remove added SSH Review wiring while keeping
  remote file backends, original save identity and shared protocol compatibility.
  Agent task acceptance in the collaboration panel remains a separate workflow.
- Current correction and audit are in
  specs/agent-communication-v2/REVIEW-BASELINE.md. Focused file/Companion/SFTP
  checks passed (23 tests); both default and warp_platform desktop checks are
  blocked by the missing local Metal compiler. Native GUI acceptance for this
  revision remains unverified; published 1.2.1 installers were not changed.

## Existing file tools over SSH — 2026-10-06

- Owner clarified the next feature: after ordinary SSH connection and `cd`, the
  existing file-management, editor/preview and Review surfaces should operate on
  the remote cwd. Reuse the current frontend panels and interaction patterns;
  do not design another Explorer, file manager or Review UI.
- Implementation proposal is in
  specs/agent-communication-v2/REMOTE-FILE-TOOLS.md. Existing remote repo metadata,
  FileModel backends and diff rendering are reuse points, but current Companion
  capabilities lack the file/Git backend and several UI events still carry only
  local paths. Removing remote guards alone is insufficient. File bytes must use
  SFTP per AGENTS.md; the old upstream headless remote daemon is not the portable
  Companion. This checkpoint changes documentation only and claims no runtime
  acceptance.

## Agent setup debug — 2026-10-06

- Installed Warpai reports 1.1.0. Isolated temporary-HOME setup using installed
  vendor CLIs passed Claude Code 2.1.267 add/get/remove and QoderCN 1.1.65
  add/get/remove with the exact generated scope/transport arguments. An isolated
  DSH 0.10.2 home-level `cordis.patch.yml` was loaded by `dsh --dump-config`.
  No personal agent configuration was read or changed.
- Settings setup was reducing adapter preflight, write and native CLI rejection
  errors to generic status text. Source now preserves actionable error context;
  the CLI adapter includes the vendor's rejection output. This source edit does
  not update the installed 1.1.0 app.
- Owner reports every non-Codex selection shows `setup failure, existing
  configuration preserved`. In 1.1.0 that exact generic status is emitted only
  when the read-only `configure(enable=false, owned=false)` preflight fails;
  vendor add commands have not run. The underlying per-agent error remains
  unknown because 1.1.0 discards it and personal configs are not inspected.
- Focused Rust setup tests could not reach the test binary because the local
  Command Line Tools lack Apple's `metal` shader compiler (`warpui/build.rs`).
  Rust formatting and `git diff --check` pass; rerun the focused suite on a full
  Xcode environment or CI before accepting the code change.

## Legacy agent MCP cleanup — 2026-10-06

- User confirmed the installed 1.1.0 app was finding MCP entries created under
  the previous Warp/Warp Lite name. A sanitized local scan found six reserved
  `warp-lite-communication` entries using the old `warp-agent mcp` command in
  Claude, Cursor, Gemini, QoderCN, Codex and DSH configuration. They were
  removed through each native CLI or a targeted structured edit; a follow-up
  scan found no remaining matching entries in those agent config locations.
- Settings now has a one-click cleanup action. It disables communication first,
  discovers installed supported agents even while disabled, and removes only
  the reserved entry when its command matches the old `warp-agent` or current
  `warpai-agent` bridge. Other servers and sibling DSH patch inserts are kept;
  failed cleanups remain visible for retry. Codex stays session-only for setup;
  its config is read only during this explicitly requested legacy cleanup.
  Ordinary global disable or agent deselection never reads or removes Codex's
  persistent MCP configuration; those controls only stop future session injection.
- Added a focused regression test for legacy JSON, Codex TOML and DSH YAML
  cleanup. The test is currently blocked before compilation because the local
  Command Line Tools do not include Apple's `metal` shader compiler; rerun with
  full Xcode or CI.

## Current delivery — 2026-10-05

- Owner superseded 1.0.1 release with **1.1.0** remote deployment iteration.
  Cancel Windows publication run 37290225101 and preserve the private 1.0.1
  draft/tag. Ship per-OS remote installers, not raw companion archives. Install
  at the remote account default Warpai directory; discover from actual remote
  home/OS and check protocol compatibility. No shell/PATH/SSH/credential edits
  and no new persistent service. Users connect via ordinary ssh and select the
  project with cd; remove the SSH alias/root/companion-path form. See
  specs/agent-communication-v2/INSTALLATION.md for plan and acceptance.

- Repository: [OthinusG/warpai](https://github.com/OthinusG/warpai), default branch
  `main`; origin is `git@github.com:OthinusG/warpai.git`. The local workspace path
  remains unchanged so active tooling and project skill paths keep working.
- Latest runtime/icon source is 5d2588a86bbdc05cc994ad816561c4626be9aac4;
  native and remote acceptance passed (see App icon checkpoint). Release
  preparation ade8308 derives versions from tags and packages companions.
  All task/diagnostic branches
  were deleted locally/remotely; GitHub retains only `main`. Preserve full
  history, licenses and release tags. Final verification receipts are docs only.
- Accepted core source is eab0ff0: desktop run 37233525889 and remote run
  37233527936 passed. Integration receipts changed documentation only; the
  screenshot follow-up fixes two toolbar opacity masks and debug capture
  readiness. Production Rust behavior, protocols and manifests remain unchanged. Native macOS original Codex model acceptance
  remains source-matched for the bridge/backend; Windows has native shell/argv
  and process-fixture coverage. Current UI receipts are recorded below.
- Source 8bc3c6e also passed full desktop run 37255520526 and remote run
  37255520544. Capture run 37255524294 passed all 161 macOS frames but stopped
  Windows at the exact shared-tab root check (132 frames); its preceding frame
  still showed PowerShell Core starting. Do not count that Windows job as native
  acceptance. The capture-only follow-up b41d07b gives that assertion 45 seconds
  and captures after it succeeds, without weakening canonical-root equality or
  changing product callbacks. Run 37259523352 passed both desktop native checkpoints: 161 decoded frames
  per platform, source-matched diagnostics, exit code 0 and no failed actions.
- Automatic session-only Codex MCP, bilingual README, audited deep cleanup and
  main/repository delivery are complete. The screenshot follow-up has refreshed native
  README images and removed unused workflow steps; icon revision is accepted,
  and formal 1.0.1 packaging/release is in progress. Main pushes run routine verification but
  do not create desktop installers.
- Native review of source 5883221 found faint toolbar SVGs despite successful
  capture assertions. Metal/WGPU monochrome Icon rendering uses the texture red
  channel as opacity. Use opaque #FF0000 geometry for these theme-tinted masks,
  not the reference gray-blue color. Ordinary image/SVG brand assets retain their
  colors. Record this consumer distinction in docs/ICON-WORKFLOW.md; refreshed
  README frames must come from the corrected source after native visual review.
- README images now use unmodified source 5d2588a macOS captures from run
  37268540992, with visible vertical tabs and Claude Warm Light. Native review
  confirms clear theme-tinted toolbar glyphs in both tab layouts and Light/Dark,
  the primary title/Secondary SSH entry, wrapping narrow actions and native MCP
  settings alignment at 100%/125%. Both PNGs retain their original bytes; the
  archive digest, CRC, all 161 images and source/action diagnostics were checked.
- After replacing the README frames, removed duplicate validation installer/release
  packaging and its unnecessary Inno Setup setup step. Retired the two-phase
  eight-hour repetition input/jobs and both orphaned Python orchestration files;
  retained the 35 existing Rust check/test commands and capture diagnostics. Keep
  desktop validation, remote validation, macOS release and Windows release.
  The source-less temporary Codex argv workflow is disabled in GitHub; accepted
  run history remains available. Cleaned-workflow protocol-only run 37262514339
  passed macOS and Windows for d3619fe: protocol/history tests and native bridge/
  readiness artifacts passed, desktop/native capture steps correctly skipped.
  Final receipts change documentation only. At that checkpoint, formal icon revision/release was
  deferred; the accepted icon and current release preparation supersede it.
- Earlier dated sections are historical evidence; current ownership, scope,
  identity and build instructions supersede their old commands/assumptions.

## 1.1.0 implementation checkpoint

- Specification: specs/agent-communication-v2/INSTALLATION.md. First source pass
  removes the manual SSH connection form, derives default remote component paths
  from native session home/OS/cwd and probes version/protocol through the existing
  OpenSSH master. Socket closure cannot fall back to a different host; WSL routes
  through the guest SSH. Pending/unconfirmed SSH must never bind a local project.
- Native installer sources live under script/companion: Linux self-extracting run,
  macOS DMG with install command and Windows per-account Inno EXE. Unix installer
  checks platform/checksum and stages executable/manifest before atomic replacement;
  it never changes PATH, shell startup or SSH configuration. Disposable CI tests
  exercise installation, repeat installation, manifest/hash/mode and corruption.
- Source 55acb81 passed all three remote Rust test suites and real controlled
  OpenSSH, including version/default-path and established-master/closed-master
  checks. Linux native installation/reinstallation/integrity also passed in run
  37301043343. Windows packaging caught an invalid Inno home constant (use the
  documented environment constant); macOS hit transient EAGAIN verifying the
  freshly created image. Correct both and rerun, keeping verification strict.
- Native Windows fallback preserves original literal SSH transport/authentication
  arguments in memory (redacted from Session debug output), uses system OpenSSH
  key/agent authentication and reports unavailable auth without saving passwords.
  Pending/unconfirmed or nested SSH cannot silently bind local/other-host roots.
  PowerShell transient bootstrap now has an explicit static dot-source branch,
  avoiding the previous unsupported-shell panic and any rc/profile-file writes.
- Remaining: run cloud Rust/native installer gates; complete and verify ordinary
  PowerShell SSH/session integration (POSIX bootstrap alone is insufficient),
  platform-specific guidance, remote cwd/tab/exit isolation and real native UI QA;
  then finalize 1.1.0 docs/assets/installers/release. These changes are not accepted
  or published yet; 1.0.1 remains a private historical draft.

## App icon checkpoint — 2026-10-05

- Owner supplied the independent bound-page/terminal logo, selected its softened
  material version, then requested a black base and no outer reflective ring.
  Built-in imagegen edits implement these exact corrections; preserve the six
  links, white panels, two slots and terminal glyph. Canonical master is
  app/assets/branding/warpai.png (1254px RGBA); ICNS/ICO exports preserve alpha.
- Use one brand asset set for README, welcome/About surfaces, macOS bundle/Dock
  plugin, Windows PE icon,
  installer/portable app and Cargo metadata. Old AppIcon enum values remain
  serialized-compatible but resolve to Warpai; the picker offers Warpai only.
  Removed superseded Dock variants, channel ICOs and Icon Composer artwork.
  Existing channel PNG paths retain the same artwork for inherited Linux
  consumers; no Linux desktop feature was added.
- The app material is baked artwork, not an animated Icon Composer implementation.
  Native UI/Agent SVGs retain their existing vector/mask conventions. See
  specs/APP-ICON.md and docs/ICON-WORKFLOW.md. Local asset/format/alpha, shell,
  Objective-C syntax and focused Rust formatting checks passed. Cloud desktop/
  native run 37268540992 and remote run 37268449581 passed for 5d2588a. Both
  OSes produced 161 valid PNGs with exit code 0 and no failed actions. Archive
  digests/CRC, all decodes and native visual review passed. README images now
  use unmodified macOS frames from this source, retaining Claude Warm Light,
  vertical tabs and the capture driver's 125% UI zoom.

## Release preparation — 2026-10-05

- Final delivery format: publish macOS DMG and Windows EXE installer only, plus
  Linux x64/macOS ARM64/Windows x64 companion ZIPs and checksum files. Desktop
  ZIPs are excluded. Preserve immutable v1.0.1 source 2bb4791.
- Windows packaging run 37282889244 compiled successfully but exposed an invalid
  installer numeric file-version assertion: Inno defaults that field to 0.0.0.0.
  Verify installer product text version from AppVersion; retain strict application
  PE string/numeric checks. Preserve installers as CI artifacts before the gate.

- Owner requested the first renamed release after acceptance and corrected the
  final product version to **1.0.1** (`v1.0.1` tag, `Warpai 1.0.1` title).
  Both READMEs must introduce the entire product, not compare upstream changes.
- Reuse the retained macOS/Windows release workflows. macOS prepares a draft
  with source-matched release companions; Windows validates and attaches its
  artifacts before publishing. Package and PE versions derive from the tag,
  not the historical hardcoded 0.5.7/1.0.0 values. Protocol versions are separate.
- Icon source 5d2588a passed remote run 37268449581 on all three platforms.
  Desktop/native run 37268540992 passed, as did preparation-source remote run
  37270362606. Bilingual README is a complete product introduction, with stable
  1.0.1 download links and source-matched warm vertical-tab images. Tagged package
  checks still gate publication; see specs/RELEASE.md.
- Packaging-version source ade8308 passed full desktop run 37270362536 on both
  OSes. Tag preparation then changes product docs, source-matched screenshots
  and release-only package checks; it preserves accepted runtime/build inputs.
- Release review caught macOS's old workflow compiling without release bundle
  options. Use `release_bundle,extern_plist`, as the existing macOS OSS bundle
  script does, to retain IME marked text and use the installed plist. Superseded
  run 37273812566 before publishing; companion packages from its immutable 2bb4791
  source passed manifest/checksum checks on all platforms and native macOS version
  invocation. Do not rewrite the v1.0.1 source tag for a workflow-only correction.

- macOS/companion release run 37277364402 passed. Actual downloaded bundle
  version/identity/canonical icon/bridge/signing and DMG checksum/image checks
  passed; all three companion manifests/checksums match immutable source 2bb4791.
  Installer-only workflow correction 594f015 passed static checks and removed
  the private desktop ZIP asset; Windows rerun 37290225101 is pending.

## Quality comparison and integration — 2026-10-04

- User authorized direct per-feature implementation comparison for PLAN S0–S5,
  compatible replacements, unified acceptance, proven-unused-code cleanup,
  default-branch integration and obsolete-branch removal. The actual default
  branch was `warp-lite/main` at that checkpoint; preserve terminal core and
  `warp_platform` consumers.
- Request-ID recovery must use the locked prost varint decoder plus checked
  integer conversion. The old handwritten decoder truncated overflowing tenth
  bytes into short lengths. Preserve partial field-1 extraction when later
  protobuf bytes are corrupt. No new protocol or dependency is required.
- portable-pty child termination is not a substitute for the companion's full
  native group/job exit observation. The openssh crate is Unix-only; keep system
  OpenSSH for both desktop platforms. Do not change the accepted task/receipt
  engine just to replace it with a generic messaging server.
- `search-before-build-compare` was installed with only SKILL.md. Its complete
  references, decision kernel and viewer were found in the skills-manager cached
  package; comparison artifacts remain in the OS temporary directory.

- Repository cleanup after the replacement functional gate removes only 27
  unused dependency edges, seven unconsumed root dependency declarations and
  seven unwired/duplicate files. No whole crate is
  unused: for example input_classifier still uses natural_language_detection.
  Preserve feature/macro/native-link side effects and all remaining lock versions.
  The local CodeGraph index remains on disk and is ignored in source delivery.

- Dependency audits must include macro expansion: num-derive needs num-traits
  in warpui_core, and safe_info needs log in vim. Plain identifier search does
  not establish that a dependency is unused; keep supported-feature CI gates.
- Cleaned executable/build source `8b8993801a696972ae0ec853d40f9c6cc8e0f582`
  passed remote run 37152973196 on Linux/macOS/Windows and desktop run
  37152975773 on macOS/Windows, including default and warp_platform checks.
  Documentation-only follow-ups preserve those executable/build inputs.
- Branch cleanup preserves full Git history and a ref audit under
  `/Users/wqin/.codex/backups/warp-lite/20261004/`; tags are retained.
  Only included tips and exact retired SSH cutover b149e30 may be deleted;
  preserve unrelated unmerged work and active PR heads.
- Integration at `903a26e` fast-forwarded `warp-lite/main`. Deleted 59 obsolete
  remote branches and four local branches after verifying the full-history
  `branches-before-cleanup-903a26e82cf9.bundle` and `branch-audit.json` in the
  backup directory above. Only the default branch remains; release tags remain.
  Adding remote.origin.url with git -c appends a URL rather than replacing it;
  use an explicit destination URL for a single push target. An atomic deletion
  succeeded at the first URL and a duplicate attempt returned stale tips.
  Both Git and the GitHub branch API confirmed the deletion result.
- Exact-source final artifacts were checked: all three companion digests/licenses;
  155 native PNGs per desktop with source-matched successful diagnostics; ARM64
  macOS application/bridge and bundle notices; Windows artifact CRC, x86_64
  application/bridge/ConPTY/OpenConsole, portable runtime files and installer
  header. Paid vendor sessions and physical-host installation are not claimed.

## Product introduction — 2026-10-04

- User wants README to introduce Warpai as a product, with images and concrete
  advantages, rather than comparing additions against upstream. Lead with the
  terminal workspace, user-owned CLI collaboration and scoped SSH projects.
  Keep maintenance/build records in docs/DEVELOPMENT.md. Preserve README core
  guardrails and license/provenance links.
- User requested corresponding Chinese documentation and working navigation.
  Maintain README.zh-CN.md, docs/DEVELOPMENT.zh-CN.md and screenshot provenance
  alongside their English counterparts. The Chinese agent usage guide is
  docs/AGENTS.zh-CN.md, linked bidirectionally to the English technical coverage.
  Keep App menu names and commands unchanged for matching actual UI.
  Chinese README uses explicit stable anchors; technical/license originals are
  linked with their language identified rather than silently presented as Chinese.
- README screenshots are unmodified native macOS vertical-tab Claude Warm Light
  captures from source 8bc3c6e; sample-data provenance is recorded in docs/images/README.md. Distinguish
  published v0.5.7-lite terminal packages from current collaboration review builds.
- User reports an independently designed Warpai icon, but it was not found in
  this checkout or post-fork image history. Packaging currently retains upstream
  Mono artwork on macOS and the OSS icon on Windows; do not invent a replacement
  logo or describe branding assets as fully synchronized.

## Deep cleanup and documentation capture — 2026-10-04

- User authorized deeper removal of unused historical AI/cloud source and files
  when supported behavior is unaffected. Explicitly preserve third-party agent
  management, launching, native setup, MCP configuration, notifications and
  coordination. README screenshots must use native macOS vertical tabs and the
  default Claude Warm Light theme. Apply these screenshot preferences only in the
  isolated capture profile; do not alter the user's normal settings.
- The source audit found 51 old AI search/embedding implementation files behind
  permanently false `cfg(any())` subtrees. Their three roots already supply
  independent compatibility types used by surviving modules. Remove the old
  trees/declarations while retaining those shared types. Do not delete entire
  AI/cloud directories: live terminal/editor/CLI consumers and persistence
  compatibility still cross those boundaries.
- Upstream bundled skill payloads are gated by `BundledSkills`, absent from the
  supported desktop default/platform and packaging configurations. Current
  project/user skill loading and native agent MCP setup are separate. Resource
  packagers must clear stale bundled skills on reused output directories; retain
  generated version metadata, settings schema and notices for retained code.
- Dependency consumers must be traced from each Cargo target through the full
  module/include graph across directories. `warp-agent-bus/tests/setup.rs`
  imports `app/src/agent_communication/setup.rs` with `#[path]`, requiring the
  test crate's TOML/YAML dependencies even though its own directory has no parser
  calls. Preserve them; directory-scoped identifier searches miss this edge.
- Cleanup removes 239 proven-unused tracked files (40,335,468 bytes) and three
  unused direct dependency edges without changing locked package versions.
  Source 02d08d484dd7ece33f528383230801d78c1355f5 passed remote run
  37194454441 on Linux/macOS/Windows, including controlled OpenSSH tests.
  Desktop run 37194453713 passed both application variants, regressions and
  161 native captures per OS; release packaging remains pending. Both README
  images now use the reviewed original macOS warm vertical-tab captures.
  Generated Yarn install state is removed with documentation delivery; it has
  no package manifest or executable/build consumer.

## Warpai identity and settings migration — 2026-10-04

- User explicitly requested Cargo application package/library/default target
  `warpai`, macOS Bundle ID `dev.warpai.Warpai`, all owned product/build/install
  entrypoints unified as Warpai, and final GitHub repository rename to
  `OthinusG/warpai`. User confirmed internal subcrate/type names and WARP_*
  compatibility protocols/environment variables should remain unchanged.
- Managed desktop settings/data use the user's home `.config/.warpai` on all
  platforms, including Windows `%USERPROFILE%\.config\.warpai`. This is not
  a literal `/home` path. State/cache and isolated debug profiles remain beneath
  this root. Third-party agent native settings directories remain provider-owned.
- Import old home/platform files without overwriting new settings or deleting
  recovery sources. Native macOS/Windows preferences are imported into JSON;
  future public/private preference writes use separate files, avoiding two
  cached stores overwriting each other's updates. Migration runs before startup
  opens preferences/state. Preserve existing settings.toml behavior.
- Additional UI and formatting changes appeared during this task. The user
  confirmed that parallel editing is finished and asked this task to continue.
  Preserve those changes and verify the final combined source; do not revert them.
- After name unification, repeat source/document cleanup and repair links before
  full GitHub desktop/remote acceptance and default-branch/repository integration.

## Product Boundary

- Current platform scope (user correction 2026-10-03): local desktop GUI targets macOS/Windows; SSH Remote environments and the repository-owned companion target Linux/macOS/Windows. Linux helper builds and focused remote tests are authorized; Linux desktop/UI remains excluded. This supersedes the broader Linux exclusion recorded on 2026-10-01.

- Warpai is an independently maintained AGPL local-first terminal derived from warp-lite and Warp, targeting macOS and Windows.
- The default product excludes bundled AI, telemetry, cloud account/login, billing, and related platform surfaces.
- Project Explorer is a desired terminal-adjacent feature and must be restored without reintroducing excluded product dependencies.
- User naming decision on 2026-10-01: this version is branded Warpai (application name), with warpai in product prose. Rename menus/settings/notifications, visible package metadata/executables, README, and installer artwork. Preserve existing storage/bundle identifiers and protocol/tool names for compatibility; real upstream URLs and attribution remain accurate. Branding and communication live directly in repository source.

## Theme, UI And Sleep Defaults

- Default theme decision on 2026-10-01: fresh installs start in a built-in `Claude Warm Light` theme matching the user's local custom theme palette. Implemented as `ThemeKind::ClaudeWarmLight` (now `#[default]`, placed after `Light`) with ANSI colors in `default_themes.rs`. The alternative of writing a YAML theme file on first launch was rejected: a missing file silently falls back to Dark. Existing saved theme settings and user theme files are unaffected; the settings default follows `ThemeKind::default()` automatically.

- Warp AI startup button bug on 2026-10-01: the legacy tab-bar AI entrypoint rendered whenever `AgentMode` was compile-time disabled (the Lite default). Clicking only set in-memory panel-open state while the panel render is disabled, so the button appeared on every launch and vanished on click. Fix: gate it on `AISettings::is_any_ai_enabled`, matching every other AI entrypoint; `warp_platform` builds keep upstream behavior.

- Keep-awake toggle on 2026-10-01: a session-scoped tab-bar button (`KeepAwake` singleton, `WorkspaceAction::ToggleKeepAwake`) holds a sleep-prevention guard only while enabled and a tracked CLI agent reports `InProgress`. `crates/prevent_sleep` was restored from history, trimmed to macOS `NSProcessInfo` UserInitiated activity plus a Windows dedicated-thread `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)`; the unused Stream wrapper, `futures`/`pin-project`/`cfg-if` dependencies and `ES_AWAYMODE_REQUIRED` were dropped. Display sleep and explicit user-initiated sleep remain allowed on both platforms; the toggle is intentionally not persisted and resets on relaunch.

## Native Agent Communication Design

- Cross-checkout reservation warnings require an explicit repository UUID and
  workspace mapping plus current membership of both owners. Snapshot sharing
  identities when the lease is created: joining or remapping must not disclose
  old private reservations. Leave suppresses warning disclosure without deleting
  physical leases. SQLite v4 adds nullable identities with a v3 backup; no Git
  remote matching or filesystem lock is inferred.

- Native static UI capture can reuse the retained warpui integration driver and
  GPU window-frame interface without the removed integration crate or OS
  accessibility permission. The debug-only `WARP_COLLABORATION_CAPTURE` entry
  uses a unique data profile and keeps HOME unchanged; worker subprocesses
  retain their normal entrypoint. Missing captures must fail the check, and
  image generation alone is never visual or keyboard acceptance.
  macOS runtime is proven by two exit-0 runs of the signed `8ea58e2` debug app;
  the optional CI `capture_ui` input uses the same path on both target OS jobs.

- Comparing a task response's state/version to its current row does not establish
  a new mutation: cached submission results can match after a later ordinary user
  turn starts. Check committed request identity while holding the broker mutex;
  cache replays must never update live readiness, even when the task row has not
  changed. Keep durable response replay separate from ephemeral lifecycle effects.

- Pool availability and execution ownership are distinct. Broadcast notices may
  remain visible in inboxes, but automatic wake must choose one eligible idle
  receiver in durable notification order. Once submitted, an unclaimed notice
  must suppress another automatic prompt; recheck at claim time so cached wake
  candidates cannot fan out. An unavailable run or failed submission allows
  reevaluation; atomic TaskClaim remains the execution-assignment authority.

- Native debug review startup on 2026-10-02 exposed a Lite boundary mismatch:
  `ReferAFriend` is registered only under `warp_platform`, but the macOS app menu
  still constructed it. Its missing description panicked at startup in debug and
  produced a dead item in release. Keep menu construction under the same boundary;
  do not weaken `default_name` assertions to hide missing action registration.
- The continuation has no Windows SSH test machine or authorized model-call budget
  (user reply: unavailable). Local macOS UI automation also lacks accessibility
  permission. Keep C01/C05/C13 and real-model/device acceptance distinct from CI;
  `specs/agent-communication-v2/ACCEPTANCE.md` tracks every remaining PLAN item.

- Continuation audit on 2026-10-02: source `fb298cb` failed its dormant wake fixture
  because it expected `AgentReady` to permit delivery during an unfinished delegated
  task. Keep the executing-task readiness guard; submit/fail/confirm cancellation
  before testing a new idle wake. `PROGRESS.md` remains the engineering handover;
  static UI and real-device/model acceptance must remain separate from protocol CI.

- Continuation on 2026-10-02: the user authorized completing the v2 plan, GitHub pushes/builds and necessary operations without further permission questions, superseding the earlier readiness-repair no-upload restriction. Keep real-device/model acceptance separate from source/protocol tests. The native panel starts as an explicitly labeled static checkpoint (`WARP_COLLABORATION_PREVIEW=1`); live controller wiring waits for visual QA. The first executed v3 Windows suite exposed missing persisted assignee/reviewer updates and nested `RefCell` connection borrows in space/history reads. Cancellation-pending tasks must continue blocking another delegated start/claim until confirmed stopped.
- Review isolation: `WARP_DATA_PROFILE` works only in debug builds; release bundles ignore it. The packaging script's `--debug` mode keeps debug assertions, uses a separate review bundle identifier and supports GitHub-built native UI review. Match the app and companion: local IPC now checks protocol major 2 and registration features before exposing MCP tools. Confirmed cancellation must belong to the original attempt's run, not merely an agent name reclaimed by another session. Archive removes tasks from routine peer summaries; full history remains explicitly paginated.
- Reclaiming a disconnected agent identity does not prove its old process or daemon stopped. TaskStart must reject running/unknown recovery; the operator must explicitly fence the uncertain execution before retry creates a new revision. Keep unknown attempts' finish time unset and retain the recorded override risk. An application/terminal binding disappearing alone is not evidence of a stopped native background task.
- Locally verified evidence must reflect a real check: explicit operator file verification uses SHA-256 and checks the opened macOS/Windows handle remains inside its producing workspace before reading. Credential paths and files over 64 MiB are excluded. Git references require local objects with lazy fetching disabled; reported test labels never authorize command execution. Protocol-only workflow runs intentionally skip application/UI/package checks and must not be cited as complete release validation.
- Task response-cache rows are not permanent result history. Persist submitted result/evidence and review feedback in attempt-attributed events before clearing current fields for rework; paginate those events through participant-scoped task_history. Row-count pagination alone cannot bound long messages or evidence: account for serialized bytes and the outer MCP text envelope. At C10 scale (100k messages/10k completed tasks), source 2cccc64 measured indexed page p95 1.799 ms macOS / 2.863 ms Windows on GitHub runners; this is not a UI, disk-full or soak result.
- Preserve the original v1 `.pre-upgrade` snapshot when upgrading normalized v2 storage; the latter backup is `.pre-upgrade-v2`. A failed SQLite COMMIT can leave the transaction open: roll back before returning failure, so an identical retry starts cleanly. Start deadlines apply to a work revision, not to whether the task has any older attempts. Wake/unread inspection must use indexed, bounded reads instead of loading every pending body.

- User clarification on 2026-10-02: `ready` means no task is executing, regardless of an input draft. Typing/deleting text and ordinary MCP queries/messages/ACKs must not revoke idle readiness. Actual local/peer turn submission or task start sets it false; completion restores it and supplies a final `warp_agent_ready` reminder. Draft protection remains a separate automatic-submission guard. Claiming a delayed delivery is not task execution and must not leave ready false when input cancels it. Expose lifecycle, native state, readiness source, draft/permission/dispatch/settling blockers and task-start eligibility separately. The user authorized minimal local process verification for this repair and requested no upload, superseding the earlier GitHub-only verification preference for this task.

- Planning request on 2026-10-01: document the complete next implementation for real execution acceptance, a collaboration panel, task cancellation/failure/dependencies, file/worktree coordination, threads/evidence, durable history and remote communication. The proposed specification package starts at `specs/agent-communication-v2/PLAN.md`, with PRODUCT, TECH and API companions. It proposes a single authoritative coordinator per explicit collaboration space over SSH, while each app retains local terminal/readiness control; macOS and Windows only. The user confirmed no Figma mock and requested the existing Warpai UI style. This is a plan, not an implemented feature or authorization to deploy remote services.

- Verification for source commit 632bd1c (GitHub run 36862604389): macOS and Windows GitHub protocol suites passed. Downloaded the macOS CI bridge and repeated native handshake/registration with Claude, QoderCN, Cursor, Antigravity, the repaired ordinary dsh-tui profile and Codex. All six passed; Antigravity used the compiled prelude directly, and Codex passed with inherited network proxies while both loopback exclusion variables were deliberately absent before launch. These checks did not replace the installed app or request model turns. Application/platform/package CI is a separate verification stage.


- Antigravity 1.2.14 probes `server/discover` as a request (numeric ID), then falls back to legacy initialization after MethodNotFound. The pinned rmcp unknown-method decoder loses that request ID by interpreting it as a custom notification and closes before initialize. A bounded shared stdio/relay prelude must reject the probe using its original ID and retain unread buffered input for rmcp; do not advertise stateless MCP support. Native temporary-HOME setup completed initialize, all twelve tools and broker registration with this fallback, even before login. Official fallback rules: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/draft/basic/transports/stdio.mdx.


- Local Cursor 2026.09.18 handshake verification on 2026-10-01: its native stdio transport filters ambient Warp binding variables. Dedicated MCP JSON entries need explicit `${env:WARP_AGENT_ENDPOINT}`, `${env:WARP_AGENT_CAPABILITY}`, and `${env:WARP_TERMINAL_SESSION_UUID}` references. Native list-tools with an approved temporary server passed initialize and twelve tools after these references were added. Preserve native server approval and never persist binding values. The installed DeepSeek TUI initially failed before MCP because identical `journal-hooks-codex` insertions existed in both the home and dsh-tui profile Cordis patches. The user requested repairing daily use as well: remove only the redundant profile insertion, retain the global journal loader and MCP entry, and back up the original profile patch. The repaired ordinary profile passed native initialize, twelve tools and broker registration using a temporary MCP overlay; no model turn was requested. This is a machine-local configuration repair, not a shipped synchronization script.

- User correction on 2026-10-01: QoderCN and Qoder are separate CLI tools, not aliases sharing configuration or selection. Use separate managed identities (`qodercn`/`qoderclicn` versus `qoder`/`qodercli`/`qoder-cli`), discovery rows, configuration roots and authorization. A Qoder entry script that reports a missing CLI must not hide the installed QoderCN client. Refresh legacy selected QoderCN program metadata after native discovery; reuse the existing Qoder prompt transport and artwork. Local handshake checks must cover installed vendor frontends before submitting this repair to GitHub.

- Codex startup proxy failure on 2026-10-01: the installed proxy-aware 0.159.2 sent Warpai's `ws://127.0.0.1:<port>` remote TUI connection through inherited proxies when loopback exclusions were absent, yielding `Handshake not finished`. An isolated native TUI probe received no local connection before the failure; adding `NO_PROXY`/`no_proxy` reached HTTP 101 and initialize. Set both spellings on the shared-mode TUI child only, preserving existing lists and falling back to the other spelling when absent. Do not disable provider proxies, edit personal configuration, restart the shared daemon or infer model acceptance from this transport probe.

- Codex 0.159.2 compatibility confirmed on 2026-10-01: the installed CLI can reuse a shared app-server daemon, while native MCP `env_vars` are resolved from the server process environment. A daemon started outside Warpai cannot inherit a new pane's binding. The reported pane had all three binding variables and the installed bridge passed an isolated initialize check; the user confirmed that launching with `codex --no-daemon` changed the server from failed (0 tools) to connected (12 tools). This confirms bridge loading, not cross-pane delivery or automatic wake acceptance. Never persist capability values or restart a shared daemon to bind multiple panes to one identity. Upstream reference: https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/tui/src/startup_orchestration.rs and https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/rmcp-client/src/utils.rs.

- Follow-up runtime report: a `codex --no-daemon` recipient was online with queued work but `ready: false`. The startup lease checked the entire command against bare aliases, excluding the required flag. Use the installed native help option contract for configuration-only launches, including hidden Codex --yolo; check shell alias/function shadowing against the parsed executable. Unknown syntax and initial work await lifecycle readiness; user drafts, blocked states and repeated discovery cannot grant another startup lease. Source repair does not update an already installed app or re-arm existing runs; live wake verification requires a patched build and a fresh receiver.

- Shared-service repair on 2026-10-01: private launch aliases bind MCP through a per-launch local relay, without persisting capabilities or restarting existing Codex daemons. Codex uses its native daemon proxy with per-thread MCP overrides; embedded modes retain their native backend and use dedicated configuration overrides. Claude/Qoder use confirmed inline native configuration. Other vendor external daemons require verified session interfaces and are not proven by common relay simulations. Native MCP child working directories establish project scope before registration; later project changes fail closed. Native busy supersedes heuristic/model readiness. GitHub run 36826820613 passed both OS protocol suites, application checks, and macOS wake/setup tests; packaging was still running at this checkpoint. These source checks do not update the installed app or prove live model acceptance.

- Native Codex launch validation with GitHub-built d677305 bridge found that the TUI rejects remote URLs containing /rpc before MCP discovery. Its accepted local form is ws://host:port, while the control socket WebSocket handshake uses the root path. Corrected both endpoints. A GitHub-built 462dc3e bridge launched two real Codex TUI clients in the same trusted project against the existing daemon; both independently registered and reported native idle with correct workspace context. A different startup directory stopped at native trust selection and was not auto-confirmed. This probe used a synthetic local broker and no model turn: it does not prove patched Warpai PTY wake, message consumption, or model replies. The final GitHub-built 843681f bridge passed both shared and explicit --no-daemon native probes: two same-project clients independently registered in each mode; both shared clients also reported idle. Cleanup completed normally after native EOF. An earlier broad process-group signal was denied; use native exit/EOF for temporary CLI cleanup. Do not equate passing common relay tests with native frontend compatibility.

- Additional native probes with the 843681f bridge: two QoderCN interactive clients loaded the private inline MCP entry and independently registered with correct workspace context, without modifying personal MCP configuration. The actual missing-entry query reproduced exit code zero and the exact sentence. Claude clients stopped at native directory trust selection before MCP discovery; no trust or approval choice was auto-confirmed. These probes do not exercise model replies or the installed app settings UI.

- Upgrade race: restored panes can snapshot old selected-agent metadata before asynchronous native help discovery finishes. A missing option contract previously made a valid --yolo launch bypass the shared relay. Refresh an empty Codex contract through the resolved native executable at launch; keep CLI identity detection independent of cached readiness metadata. Two native clients with deliberately empty metadata passed --yolo/-C registration and idle checks using the GitHub-built a530fb2 bridge.

- Codex resume/fork grammar: --last shifts the first session positional into an initial prompt; without --last, the first positional selects the session and only a later positional supplies work. Reuse the installed option parser for both backend selection and this initial-work distinction. Empty resumed sessions can become ready after an owned native thread binds; supplied work waits for its first native turn. Source reference: https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/cli/src/main.rs. Native loaded-thread resumes can ignore configuration overrides while another client observes the thread or a turn is active; per-launch isolation is established for fresh threads/cold resumes, not concurrent attachments to one already-loaded conversation. Source reference: https://github.com/openai/codex/blob/rust-v0.159.2/codex-rs/app-server/src/request_processors/thread_processor.rs.

- QoderCN reports an absent user-scope MCP server with exit code zero and an exact not-found sentence. Treat only that exact response as absence; preserve conflicting user-owned entries. Reversible native setup regression covers add, recheck, remove, and conflicting output.

- Shared launcher lifecycle: the private alias directory is shared across panes, so per-pane command snapshots cannot be the current authority. Publish a nonsecret runtime catalog atomically on selection changes, retain retired executable aliases for native passthrough, and fail closed for stale communication snapshots. Initial activation still needs a fresh pane to inherit launch PATH; do not inject commands into an existing shell or restart vendor daemons.

- User clarification on 2026-09-30: the target is native communication between independent CLI agents running inside Warp Lite, including task delegation, result submission, acceptance/rework, and automatic handoff. hcom and Agent Mail are references, not a required choice or final architecture. The user first requested a design, then approved implementation. First-release coverage was expanded to every CLI agent managed by Warp Lite, including aliases; the user subsequently excluded agents without native MCP support.
- Implementation ownership: a Warp-owned local broker for identities, messages, and task state, a thin stdio MCP bridge over local IPC for CLI access, and automatic handoff through polling queued work and submitting to idle native prompts. The user explicitly rejected cooperative MCP waiting as the sole handoff path: waiting at the input prompt is idle and must be woken. A common final-action readiness tool covers native MCP clients without lifecycle hooks; installed completion/busy hooks also update readiness; it must not reactivate Warp's platform MCP client manager or cloud AI product.
- User runtime report on 2026-10-01: sending from Codex appeared successful but the receiving Codex showed no action. The shared wake guard required Success whenever any listener existed, incorrectly vetoing explicit MCP readiness for opaque Codex/Grok listeners and stale status for other clients. Use the broker readiness lease as authority across every agent, retain explicit blocked/draft/run protections, and never infer idle from opaque OSC notifications that also carry approval requests. Show queued work at the receiver and report queue admission separately from acknowledgement/completion.
- This revision adds native Trae YAML-list setup (including traecli recognition), DeepSeek Harness home Cordis insertion (including dsh-tui discovery), Grok TOML setup, and Vibe runtime native environment configuration. Vibe requires a new pane after selection because its Python stdio client filters ambient Warp variables; prepare native VIBE_MCP_SERVERS in memory with user/project entries at pane startup, never persist capability values. Later profile/directory/config changes require a fresh pane to refresh the snapshot.
- Qoder command recognition does not imply session-event support: Qoder currently has neither a session listener handler nor a Warp notification plugin manager. Existing CLI session events update terminal status/context; they are not an agent-to-agent task delivery mechanism.
- `TerminalView::submit_text_to_cli_agent_pty` already submits prompts using agent-specific PTY strategies. Reuse it behind explicit readiness, live-run identity checks, and task acknowledgement; its current return value does not prove that an agent consumed a prompt, and its delayed Enter path needs lifecycle revalidation for broker use.

## Build And Release

- Primary check (GitHub only in this workspace): `cargo check -p warpai --bin warpai --locked`; also check `warp_platform`.
- App packaging: `script/build-warpai-app.sh` (macOS), `script/build-warpai-windows.ps1` (Windows x64).
- Historical published artifacts retain their WarpLite filenames. Current source targets Warpai; icon revision and packaging/release are deferred.

## Maintenance

- Upstream changes are historically selected and applied with provenance rather than merged wholesale.
- User decision on 2026-10-01: retire automatic upstream synchronization, restoration scripts and replayable patches. Keep changes as direct committed source. Retain independent GitHub compilation, testing and tagged releases; do not remove provenance or licenses. GitHub fork-network metadata is separate from the source maintenance decision.
- Historical fork provenance: `OthinusG/warp-lite` derived from `terzigolu/warp-lite`, originally on `warp-lite/main`. Current repository identity/default-branch integration is recorded in the final delivery section.
- Project Explorer and persisted ToolsPanel migration are maintained directly in source, alongside CLI integrations, platform fixes, branding and native agent communication.
- Workspace initialization restores the right-side Tools Panel button for older persisted toolbar configurations that omit it.
- Standalone macOS releases build an existing repository tag. Windows release automation reads that successful run's release-target artifact and builds the same tag rather than the latest branch or release.
- macOS packaging uses Warp's native 1024×1024 `app/DockTilePlugin/Resources/mono.png`; standalone tagged releases preserve older assets because GitHub's asset endpoint returned 404 when `gh release upload --clobber` tried to replace them.
- Antigravity (`agy`) uses the standard CLI Agent fallback without Warp AI/cloud dependencies or telemetry transport.
- DeepSeek Harness accepts only dsh-tui and dsh TUI profiles in managed CLI sessions; do not add telemetry or platform integration.
- DeepSeek Harness uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles; the standard monochrome icon renderer turned the original RGB PNG's opaque white background into a blank block.
- Qoder aliases qoder, qodercli and qoder-cli and the separate QoderCN aliases qodercn/qoderclicn are managed CLI sessions; native command detection is independent of notification hooks.
- Qoder CLI uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles, matching DeepSeek Harness.
- Trae aliases include traecli as well as trae, traecn, trae-cli and traecn-cli. Native MCP setup uses the documented YAML list; the icon follows the original-color transparent PNG path.
- Hermes includes the hermes-agent alias and uses a transparent PNG icon through original-color rendering.
- Keep both Cursor commands cursor-agent and agent in command recognition.
- The default Lite build does not register Warp MCP file watchers, file-based server management, or MCP gallery; it retains an inert `TemplatableMCPServerManager` singleton only for compiled Warp AI callers. The full MCP runtime remains available in `warp_platform` builds, while third-party CLI agents manage their own MCP configurations.
- Do not restore inherited internal upstream workflows, cloud integrations or synchronization. Retain only independent validation and macOS/Windows release workflows.
- Windows packaging uses the existing PowerShell build script and Inno Setup; installer/portable artifacts include the native communication companion.
- Preserve Windows autoupdate log replacements for removed telemetry and the OsStr conversion for the PowerShell history read command.

- Agent communication is ordinary repository source under app/src/agent_communication and crates/agent_bus. The user still requires GitHub compilation and no local builds.
- Communication coverage must follow the managed `CLIAgent` enum automatically, not a separate four-vendor allowlist. The user explicitly excludes agents without native MCP support: list them, do not add shell-tool fallbacks. Protocol simulation and real vendor model acceptance are separate verification levels.

- macOS Unix socket endpoints must use a short private directory under `/tmp`; the system `TMPDIR` path plus a UUID can exceed macOS's 104-byte socket path ceiling. Protocol compilation/tests run on GitHub, not locally.

- User clarification: custom/downstream agents also qualify when native MCP is confirmed. `qodercn mcp add --help` confirms native stdio, and QoderCN now has a separate managed identity (superseding the earlier Qoder alias mapping). Do not exclude `Unknown` sessions categorically; verify their MCP capability individually.

- Native communication uses the existing Tokio dependency's Unix sockets and Windows named pipes directly. The pinned interprocess wrapper failed real Windows IO tests; avoid sync `PIPE_NOWAIT` polling and wrapper-specific readiness semantics. Keep a new listening pipe instance alive before handing off a connection, bound IO with async timeouts, and acknowledge received frames before closing pipe connections.

- Dormant wake delivery must poll while busy and submit automatically once idle. Reuse the existing agent-specific prompt submission strategies, but guard delayed Enter against run replacement and manual input. PTY submission is not message acknowledgement; pending work is consumed through MCP. Do not depend on the code-review feature flag to access managed CLI sessions.

- User clarification: busy delivery is polling, then automatic submission once idle. Manual cancellation pauses automatic submission until the user resumes with a new input; pruning a closed view must also invalidate its broker run.

- Superseded README direction (before the user requested a product introduction): distinguish the original terzigolu/warp-lite removals and terminal preservation from OthinusG downstream additions. Preserve original release history; document restored Project Explorer, extra CLI integrations/aliases, Windows distribution, independent maintenance, and unreleased agent communication separately. Published v0.5.7-lite assets include both macOS and Windows x64; do not retain the old macOS-only FAQ.

- Full application test compilation exposed an inherited cloud-agent-management test referencing a field gated by `agent_management_view`. Gate that test with the same feature rather than restoring the disabled cloud UI or silently skipping native wake verification.

- User rejected the manual communication setup UX: configuring MCP flags, manually registering agent names, and requesting readiness in each prompt is too complex. The product needs a Warp-owned opt-in entry that prepares native MCP connections, assigns terminal identities, and supplies cooperation instructions; ordinary task delegation should be initiated from conversation or a terminal context action. Existing running agents may need an explicit restart to load a newly configured server. Earlier test artifacts exposed only the manual setup path; the settings revision below replaces it.

- Superseded by the settings/project-scoped revision below: user originally specified the communication entry: right-click an Agent tab, choose **Select communication peers**, then select an available agent to establish a reciprocal connection. Afterwards ordinary dialogue and agent-initiated task delegation use the same connection. Menu text and related UI messages must be English. Native MCP tool discovery registers identity automatically; connection choices are scoped to current process runs, and selected agents are restricted to their chosen peers. Native MCP setup is still a prerequisite; unloaded clients must never be falsely shown as connected.

- User revision on 2026-09-30: implement Settings-based communication enablement and automatic reversible vendor MCP/environment setup first; then remove all manual peer selection and allow every enabled native-discovered agent within the same project to communicate. Source delivery now uses direct commits; perform compilation/packaging on GitHub only.
- Settings are machine-local and default off. Track only the dedicated `warp-lite-communication` configuration and non-secret cleanup metadata. Revoke broker authorization before cleanup, retain failed cleanup for retry without restoring permission, preserve unrelated/user-modified configuration, and require vendor reload/restart where native hot-loading is unavailable.
- Maintain LF for Rust/configuration/packaging source across macOS and Windows checkouts. The old patch-specific whitespace exception is retired.

- Hermes filters stdio subprocess environments; configure runtime `${WARP_AGENT_*}` and `${WARP_TERMINAL_SESSION_UUID}` references explicitly. Vibe native stdio support alone does not establish dynamic environment passthrough; automatic setup stays unavailable until a compatible contract is verified, rather than persisting capabilities or pretending a connection exists.

- Readiness verification on 2026-10-02 separates shared IPC regressions from vendor acceptance: 20 short-lived MCP clients and owned Codex event checks pass, while isolated native Codex session setup times out and QoderCN completion is not observed before the deadline. The installed app was not replaced; do not treat these checks as verified real vendor task acceptance. A queued pool claim is not task execution and must preserve readiness until TaskStart.

- Receiver audit on 2026-10-01 covers every managed type, not only Codex. A fresh bare interactive launch needs a one-time readiness lease after native discovery, cancelled by user input/busy/blocked events. Regular MCP operations and rediscovery cannot re-arm it. Preserve task notifications until their authorized task transition; ordinary ACK must not discard assignment/review work. Main agents receive default coordination instructions and peer task summaries; TaskGet exposes offline/interrupted receivers. Explicit offline identity reclaim must work after automatic discovery but cannot abandon an identity that already has work. See specs/agent-communication/RECEIVER-STATES.md; simulated states and live vendor acceptance remain separate.

- Agent bus v2 storage landed on 2026-10-01 (M1): the v1 single-blob store was replaced by normalized SQLite tables in crates/agent_bus/src/storage.rs with versioned schema, a pre-v2 backup copy, a sentinel that makes older binaries reject an upgraded writable database, and migration tests for valid, malformed and orphaned v1 fixtures. The twelve original MCP tool names remain, with task_list as the thirteenth; all mutations route through the serde Operation contract with stable DomainError codes, optimistic expected_version checks and per-run attempt fencing. Events commit atomically with each transition and page by cursor. Mutation epochs expire after 7 days while request dedup rows retain 8 days (capped at 10000) so pruning cannot re-execute an old request; capacity is 1000 agents / 10000 tasks / 1000 pending / 100000 messages per project with cursor pagination (50 default, 200 max).

- M1 verification for source commit e08b2ed (GitHub run 36877709716): macOS and Windows protocol suites passed (migration, pagination, dedup, error-code, fencing tests), both application checks passed (`cargo check -p warp --bin warp-oss` default and `warp_platform`), and the macOS job also passed the dormant-wake, managed-configuration, setup-reversibility and notification lib tests and built the packaged review artifact. Exit criteria C03/C04/C10 are satisfied on both target platforms. These source checks do not update an installed app; local acceptance of migration and stale replay still requires the M0.2/M0.3 harness against a real UI.

- Agent bus v2 M3/M4/M5 backend landed on 2026-10-01 (commit 2a9da86, 13 files): task pools/claims/progress/cancel/retry/reassign/dependencies, deadlines and sweeps, expiring exclusive file reservations with abandoned-owner warning, spaces/workspaces/devices controller operations, threads/evidence/history export-purge, and draft/native-activity readiness tracking, plus ten new storage protocol tests. CI run 36893865896 is red on a single compile error (`Reservation.created_seq`, storage.rs:3670) and an unused-import warning (`MAX_SUBJECT`); the protocol tests have not yet executed on this commit. A second active session holds the fix plus a user-authorized readiness-semantics refinement uncommitted in lib.rs/storage.rs/transport.rs/readiness.rs/READINESS.md; review and commit those edits, never revert them. Status handover with M2 panel design notes: specs/agent-communication-v2/PROGRESS.md. Remaining: M0.2–M0.5 harness and real-device runs, M2 panel, M4 join/leave UI, M6 SSH, M7 release acceptance.

- Collaboration static preview uses the existing Tools Panel directional focus actions; explicit focus enters its keyboard handler, while ordinary opening/refresh preserves terminal input. Preview header controls stay outside the scrolling detail. The isolated native capture harness seeds an unsent draft before checking navigation and focus restoration; these checks require a GitHub-built debug executable, not local Rust compilation. Static acceptance still gates live integration.

- Remote collaboration startup uses system OpenSSH, a validated host alias and fixed `warp-agent remote-stdio` command, with strict known-host verification, BatchMode and separate pipes. Disable forwarding, local command hooks and control-socket reuse. Offline `ssh -G` tests use an empty disposable configuration and never read credentials or connect; this startup spike does not establish enrollment, grants or remote task support.

- Windows OpenSSH offline probes must close a disposable configuration file before spawning ssh; keeping its NamedTempFile handle open rejected the probe on the Windows runner. Using a retained TempPath after closing the handle passed macOS/Windows system option parsing in run 36951069744. Keep diagnostics limited to fixed option names/status, without dumping configuration output.

- Historical eight-hour deterministic backend repetition used two four-hour jobs per target OS. It was retired during the 2026-10-05 workflow cleanup because it is outside current acceptance and repeats the retained protocol/history suites. It never established an eight-hour native UI/draft session or vendor-model acceptance.

- Local shared routing uses fresh app-selected terminal capabilities and SQLite v5 agent-to-workspace bindings. Reuse the existing immutable workspace root rather than duplicating paths. Tasks/events use a space domain; file conflicts and evidence use producing checkouts. Metadata joining alone leaves old private work untouched. Leaving/remapping durably revokes access and wake without claiming effects stopped; no shared UI or remote support is enabled from backend tests alone.

- Windows debug UI capture must stage the same DXC/ConPTY/runtime DLLs, PowerShell bootstrap and OpenConsole files used by the portable package beside its executable. A bare target/debug binary is not a runnable review package. Keep capture failure diagnostics limited to source locations/status; use protocol_only plus capture_ui for debug captures without recompiling release packages.

- SQLite RefCell borrows can live through chained iterator adapters after load; materialize rows before filtering with authorization queries. Shared AgentList exposed this on both OS real-IPC tests.
- Remote invitation retries must persist only the safe receipt, not the raw one-time invitation. Replay cannot redisplay the secret. Enrollment uses OS-random 256-bit credentials, SHA-256 verifiers and locked subtle constant-time comparison; participant persistence must use the existing platform secure storage when gateway wiring lands.

- Windows named-pipe defaults include Everyone/Anonymous read access. Native/control listeners should use a protected current-user SID DACL, reject network clients and disallow handle inheritance; inspect the actual kernel DACL in Windows CI. This OS boundary complements capabilities/device grants and does not replace them.

- Enrollment controller discovery is non-secret and nonce-owned; reuse the broker runtime and mutex, and invalidate device generations when participation stops/restarts. A gateway only relays framed bytes and never opens the database. Enrollment/heartbeat IPC does not establish remote native task routing or production opt-in.

- Windows native screenshot readback needs surface COPY_SRC independently of the old integration_tests feature. Enable only for the explicit debug collaboration capture or integration tests; reject unsupported usage/format before GPU copy encoding.

- macOS controller sockets share the existing /tmp broker runtime directory; the full Unix socket path must remain below SUN_LEN. Use a short random basename, not a long descriptive prefix, and remove only the nonce-owned socket on controller drop.

- SSH participant sessions must verify the expected durable coordinator UUID before sending enrollment/authentication secrets, and derive connection epochs from server replies. Own and kill only the SSH child, drain stderr without persistence, and keep one-time credentials in memory until platform secure storage succeeds. Real-child stdio fixtures do not prove real SSH or platform credential acceptance.

- The native capture harness clears WARP_COLLABORATION_CAPTURE before app window creation so worker children take normal entrypoints. GPU readback configuration must use the retained WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS flag (debug-only), rather than the cleared launcher variable.

- Remote routing storage uses additive SQLite v6 tables rather than extending host workspaces' canonical-root uniqueness. Remote physical identity is enrolled device plus participant checkout UUID; display paths are not host roots. Native run UUIDs map to durable server mutation epochs. Preserve closed-run tombstones and uncertain attempts, and fence remap/departure before replay. This backend foundation does not enable production remote routing.

- Store::transaction owns an explicit BEGIN/COMMIT and is not nestable. Atomic remote admission must call a transaction-internal recovery helper; preserve the public recovery wrapper for ordinary caller-owned operations. Do not split identity/run creation and interrupted-attempt recovery into separate commits.

- Collaboration static gate passed on both native target platforms (source 3d66017, run 36970541930): 74 captures each, all fixture combinations reviewed, detail end reachable and draft/focus preserved. Live integration is now permitted but needs separate native data/action verification.

- Native panel reads use the broker's existing condition variable off the UI thread with 50-item pages, a 200-event view tail and scoped sequence cursors. A one-second timeout refreshes volatile readiness independently of durable history. Scope/pane changes reset selection and cursors; callbacks also check a local generation before applying results.

- Native human task forms retain the original scope, revision/version and request UUID. After submission, changed fields require a new explicit intent; do not retry an old ID with new content or silently substitute a version. Uncertain ownership requires typed ALLOW OVERLAP and still records unknown effects. Async mutation callbacks refresh data without a delayed focus change.

- Shared-pane admission is a reviewed workspace/space/canonical-root tuple passed through NewTerminalOptions, never an environment selector. Revalidate the exact tuple when preparing the new capability; a remap must not silently change the user's reviewed scope. Restored panes remain private. The panel derives pending admission from the prepared binding before native-agent discovery, and keeps revoked scope history distinguishable from participation.

- An overridden outcome does not prove stopped execution. A force-cancel override belongs to that cancellation intent; retry/reassignment must independently authorize the current revision's unknown attempt. Older uncertainty remains recorded after the replacement revision is authorized. Native action acceptance exposed this distinction beyond backend transition coverage.

- Native capture diagnostics copy only failed step names found in the committed capture source, plus exit status and source SHA. Raw stdout/stderr remain runner-local. This makes both-platform GUI failures diagnosable without exporting application payloads or credentials.

- Trusted local operator thread/search reads may inspect the selected domain; native actor reads remain participant-scoped. Keep the operator program reserved at registration, retain original message bodies and append replies rather than editing history. Panel search state stays transient and bounded to existing 50-record pages.

- Local evidence viewing and verification share the producing-checkout lookup. File viewing rechecks current canonical containment and file type, refuses remote descriptors, and never substitutes the active pane's same-named file. Opening current content is distinct from hash verification. Late file-open callbacks require the original selected task/pane and enabled communication.

- Windows PTY startup must simplify canonical extended-prefix paths before both CreateProcessW and bootstrap environment construction. Broker scope identity stays canonical; PowerShell location-provider paths use the existing dunce helper. Native shared-pane acceptance exposed this boundary.

- History eligibility must query all persisted attempts rather than the 16-attempt task-detail window. Force-cancel/overlap overrides retain unknown effects and cannot authorize archival or purge. Preview/deletion share predicates; referenced message roots survive acknowledged-message cleanup.

- Native history purge pins the original preview sequence and typed DELETE HISTORY intent. Purge selects task/message eligibility before deletions can expose more rows, records an audit event and replays the original result. Reply/thread retention checks use separate indexed lookups rather than one OR join. Native history JSON export is an explicit scoped clipboard page; no automatic filesystem write or eviction.

- Human reservation maintenance pins the original page owner, checkout and expiry. Renewal requires a current authorized owner and confirmed active linked attempt; explicit advisory release retains uncertain attempts and never claims OS execution stopped. Existing agent MCP ownership/run checks stay intact.

- Native task list projection intentionally contains only id/state/revision/version/assignee. Checks requiring description, archive markers or attempts must use the existing selected-task/history-export full records rather than inventing summary fields. The Rust compiler caught this in retained GUI fixtures; local Rust verification remains GitHub-only.

- Remote gateway task dispatch separates connection/correlation epochs from the coordinator-assigned original mutation epoch. Resolve the device-bound actor and current generation/grants before the existing Store request ledger can replay. Native readiness/wait and controller APIs remain excluded until guarded participant presence/wake is connected.

- Remote client replies must match the original actor/device/space, connection, correlation and mutation epochs. Transport correlation IDs may change while the original mutation request UUID/content stays unchanged. Preserve whitelisted structured conflict versions without retaining reflected peer messages. Existing schemars UUID support is absent; describe UUID wire fields as strings instead of adding a dependency feature.

- Remote reconciliation is a read of the original request ledger, never an executing replay. A retained matching receipt can resolve a closed run; an absent receipt under a closed/expired run cannot authorize replacement execution. Recheck device generation, space grants and actor membership before revealing receipts, and pin original operation content as well as actor/epoch/request UUID.

- Participant remote intents use a separate bounded table in the existing private SQLite Store, not authoritative task copies or plaintext credential preferences. Original identity/content and confirmed results are immutable; unresolved rows survive restart/expiry and cannot be removed. Native routing must stage before network writes without holding the broker mutex across SSH I/O.

- Remote native online state is a connection-bound monotonic receipt-time lease, separate from seven-day mutation epochs and durable tasks. Revalidate generation/grants/run before reporting online or allowing a remote pool claim; disconnect/shutdown clear only ephemeral presence. A heartbeat does not assert idle, empty draft, approval clearance or stopped execution, and cannot enable native wake.

- Device grant review is a trusted local controller mutation pinned to the original generation. Every read/write/removal change increments generation atomically with the grant/audit, fencing live sessions before replay while preserving tasks and execution uncertainty. DeviceList keeps compatible space_ids and exposes explicit roles; remote MCP/gateway operations cannot edit grants.

- Remote disconnect/lease loss must persist unknown certainty for active attempts of the exact actor/epoch, with one audit/version change and no finished timestamp or task outcome. Online projection alone is insufficient. Reuse this transition for lease expiry, session cleanup and controller shutdown; a heartbeat cannot restore execution authority or erase the original uncertainty.

- Remote snapshots reuse bounded history export with an original high-water fence across pages; concurrent changes require a fresh scoped snapshot. Confirmed device/space cursors cannot exceed coordinator high-water or move backwards. Grant removal clears that scoped cursor, and current authorization precedes every snapshot/event/cursor read or write.

- Controller deactivation must acquire the broker mutex and fence remote access before attempting expiry/audit writes. A full disk or rejected uncertainty write cannot keep participation enabled. Retain failed loss metadata for the next sweep/activation; session cleanup likewise expires a failed lease instead of dropping the only retry record. Listener exit and explicit drop share this deactivation path.

- Unknown connectivity does not grant another execution owner. The unchanged original actor/current run may explicitly confirm completion, failure or stopped cancellation on the same unresolved attempt/revision. Shared outcome checks accept only active/unknown attempts with no outcome/finish timestamp; overridden/replaced epochs and revisions stay fenced. Progress/start/claim/renewal do not gain this exception.

- Native history clipboard acceptance is restricted to isolated GitHub runners. Capture the exact page before explicit CopyHistory and compare only that owned fixture result, including scope/cursor/order/linkage and unchanged draft; local capture mode never reads or overwrites the daily user's clipboard.

- Native participant rows show qualified device, physical checkout and monotonic observation age without equating connectivity/activity with readiness or task outcome. Remote ages come from validated receipt leases; offline ages stay unavailable. Local focus remains limited to existing VIEWS matches. Preserve native labels and scroll/wrapping when adding these metadata facts.


- Native remote reconnect must fence the device-qualified native connection before absent-receipt reconciliation. Keep server-ordered owner tombstones after disconnect, reject older announcements and heartbeats, and guard every write before ledger replay. Otherwise a delayed old frame can commit after a new connection reports not_committed. Retain original mutation epochs and mark superseded execution unknown.
- Native capture assertions access clipboard through an existing ViewContext, not App. Explicit history export verification runs only on isolated GitHub runners; local captures must not inspect or replace the daily clipboard.


- Remote credentials reuse the existing SecureStorage trait with UUID-only namespaced keys. Empty/no-op providers must fail closed, enrollment must not overwrite existing keys, and successful writes require exact readback. Delete only the newly owned key after failed verification; verify absence after removal. Synthetic provider checks do not prove native Keychain/DPAPI acceptance.


- Native remote enrollment transfers credential results only in memory to the existing secure provider. Persist reviewed non-secret profile metadata only after readback; preserve older settings with an empty profile list. Cancel network futures through an owned watch channel and drop their SSH child; dropping only the result receiver leaves an unnecessary child alive until timeout. Metadata persistence alone must not enable remote MCP participation.


- app/agent_communication/setup.rs is included verbatim by the bridge integration tests without the native UI modules. Keep serialized connection profile types in setup.rs (or another UI-independent source), and keep secure provider/ModelContext/SSH worker orchestration in remote_settings.rs. A setup preference reference to its native sibling breaks both OS bridge tests.


- Sanitize AuthenticationFrame::Error during initial Hello as well as authenticated exchanges. A peer can send an error before enrollment; returning that raw DomainError bypasses the shared diagnostic whitelist. The owned stdio fixture now checks reflected Hello text and unknown retry/version hints without transmitting a credential.
- Native CLI preference changes wait for pending enrollment, except disablement which cancels immediately. Otherwise a settings worker can invalidate a consumed invitation or overwrite newly persisted connection metadata.


- Native claimed/submitted/cancelled prompt observations belong to the event history, with original receiver/run/message/task revision metadata only. Never infer AgentAck, TaskStart or execution from a PTY submission. Commit the claimed observation before input, expose failed completion audit retention, and never reconstruct live delivery authority from persisted observations after restart.
- SSH children reuse session::without_terminal_binding and strip VIBE_MCP_SERVERS. Removing originating capability/endpoint/native-launch environment prevents user SSH SendEnv rules from forwarding local MCP bindings; retain ordinary system SSH authentication environment.


- macOS SecureStorage must map only Security.framework errSecItemNotFound
  (-25300) to NotFound. Treating locked/denied access as absence lets enrollment
  overwrite a credential after unlock. Shared error conversion retains other
  failures; the focused regression uses status codes without reading user keys.


## SSH Remote product correction — 2026-10-02

The user clarified that the intended remote feature is a project environment
managed by the local Warpai app over SSH: agent processes, commands and file
operations execute on the remote machine, and agents in that remote environment
communicate. The existing v2 M6 design instead requires participating native apps
on separate machines with enrolled devices, grants and a coordinator; it does not
satisfy SSH Remote project execution. Stop extending that production routing
until the specifications are realigned. Preserve reusable local task/history/UI
and SSH framing work; do not equate a shared SSH host with automatic cross-project
access. The follow-up on 2026-10-03 resolved remote scope: Linux/macOS/Windows with SSH/SFTP only; no remote desktop GUI requirement.


## SSH Remote specifications and cutover — 2026-10-03

The user requested a documentation rewrite, including engineering work and explicit
removal of already-developed excess features. Active PLAN/PRODUCT/TECH/API now
describe SSH project execution, SFTP files/transfers and one local GUI for remote
terminals/Agents/tasks. R0–R7 and V01–V24 replace historical M0–M7/C01–C14 as
current gates. Previous machine-collaboration specs are archived; ACCEPTANCE and
PROGRESS retain historical source-specific evidence. No code was removed or remote
service enabled during this documentation task.

CUTOVER D01–D16 plans retirement of invitations/device grants, paired-native-app
gateway, enrollment worker/profiles/keys and obsolete device actor/spool roles.
Keep local task/history/UI, secure-storage fixes, private IPC permissions, original
receipt fencing and draft-safe wake. Legacy unknown intents/attempts/history must
remain read-only/exportable; no automatic identity migration or schema downgrade.

Existing remote_server client/transport/protobuf/file-tree/SFTP-upload source is a
reuse candidate, not a complete verified SSH Remote backend. Its installer names
upstream Oz/cloud downloads and platform detection lacks Windows remote support.
Audit compatible server source and replace deployment with reviewed repository
artifacts before enabling. No FTP/FTPS dependency or workflow is needed.


## SSH Remote GUI ownership discussion — 2026-10-03

The user pointed out that the existing left vertical Tab sidebar already serves
as the Agent/session management surface and asked why another Agent/task GUI is
needed. The sidebar already renders CLI icons/status and tab/pane naming/close
controls. Do not justify a duplicate Agent dashboard merely from backend roster
availability. Recommended design: existing sidebar owns session navigation/state
and session actions; the collaboration panel owns project-level task/message
relationships, evidence and durable history, with links back to those sessions.
The user approved this direction and requested its application to the plan.
PLAN/PRODUCT/TECH/API/CUTOVER/ACCEPTANCE and the UI checkpoint now define the
sidebar as the sole session manager and the collaboration panel as on-demand
task/message/history detail. Keep existing saved panel selection; fresh profiles
default closed, and events never open/focus it. Preserve actor/session read APIs,
assignee pickers and attribution while retiring duplicate Agent roster/controls.
Exact task/session links and detached runs use the same project/sidebar hierarchy.
These are specification changes; source implementation/removal remains pending.


## SSH Remote cutover implementation — 2026-10-03

- R0 checkpoint removes the native device-enrollment worker and its setup busy
  gates. Old `remote_profiles` deserialize as `legacy_remote_profiles`; only
  saved UUID-owned secure keys are removed. Cleanup never reads credential
  values and failed platform deletion stays visible/pending. Metadata is retained.
- Production builds exclude the old SSH device client/controller and presence
  routing. `remote-stdio` emits a fixed, non-retryable compatibility refusal;
  Broker::control rejects all five retired device operations before replay.
  Historical controller fixtures remain library-test-only until R0/R7 replacement.
- Schema v7 backs up v6 to `.pre-upgrade-v6`, revokes legacy authorization and
  marks unfinished remote attempts unknown without a finish/outcome. Original
  pending intents, receipts, actor/evidence provenance and local work survive.
  This supersedes old notes describing enrollment as the active remote direction.
- The inherited remote_server crate has no source-built server executable and
  its installer downloads Oz. R1–R7 need a repository-owned server using the
  existing protobuf control substrate; a device gateway cannot substitute for it.
- Checkpoint scope/verification: specs/agent-communication-v2/IMPLEMENTATION.md.
  Local Rust parsing, SQL and whitespace checks passed; exact-source GitHub
  protocol/application/runtime acceptance is pending. SSH Remote is not delivered.

- User requirement on 2026-10-03: the SSH task panel must include remote host
  status. SR41/HOST-STATUS.md pins host/account/project and separate SSH/SFTP/
  companion/MCP status, remote CPU/memory/project-volume space, source/age and
  expanded OS/architecture/uptime. Unsupported/offline values are unavailable or
  stale; no local fallback, readiness inference or background focus change.
  Specification is updated; the remote status source/UI is not implemented yet.

- R0 validation source 4c27262/run 37038998741 passed both desktop OS protocol/
  setup and representative-history steps; application builds were in progress.
  Follow-up shared macOS deletion uses the pinned security-framework direct
  delete API: no password retrieval and OS deletion failures are propagated.
  Stronger local-attempt/receipt/selection checks and this provider change await
  a fresh exact-source run. Native keychain runtime acceptance remains pending.


R0 follow-up source `a9caea0`, [run 37040051387](https://github.com/OthinusG/warp-lite/actions/runs/37040051387):
macOS and Windows protocol/setup and representative-history checks passed,
including unchanged local active attempts, retained original receipts and legacy
profile selection. Both default application checks failed with E0616 because the settings view
accessed a private legacy profile field; platform and focused native cleanup
checks were skipped. A boolean Preferences accessor fixes the ownership boundary;
exact-source revalidation is pending. Source was verified to
match the workspace's 17 implementation paths without altering the current index.
SSH Remote and SR41 host-status collection/rendering remain unimplemented; their
contracts and static/live acceptance requirements are recorded in specs/agent-communication-v2/HOST-STATUS.md.


Remote-server reuse audit: the lightweight client crate has no binary, but actual
server handlers exist under app/src/remote_server/server_model.rs and Unix
proxy/daemon dispatch. The daemon depends on WarpUI headless app services;
Windows dispatch is unsupported. Its host_id is a boot-random UUID, not stable
SSH account/server identity. Reuse verified handlers/codec instead of inventing
another full stack; packaging, root admission, bounded queues and multi-platform
service ownership still require work before managed SSH activation.


R0 repair source `d71d8b1`, [run 37041826672](https://github.com/OthinusG/warp-lite/actions/runs/37041826672):
macOS and Windows protocol/setup, representative history, default application and
warp_platform application checks passed. Both OS legacy metadata/credential
cleanup tests and macOS native configuration, wake and Keychain error-classifier
tests and both review-package builds passed. Real credential-provider and SSH
runtime acceptance remain pending. These results validate the
cutover implementation, not R1–R7 or SR41 runtime behavior. The workspace's 17
implementation paths match this source; its current branch/index was preserved.


Continuous R0–R7 implementation is explicitly requested on 2026-10-03. Extract
only protobuf generation/framing into remote_protocol so the independently built
companion can target Linux without desktop/UI dependencies. Keep the existing
remote_server::{proto, protocol} public paths as re-exports. Ordinary inherited
channels keep 64 MiB; managed channels use the same codec with a 1 MiB limit,
checked before allocations. No second codec, transport interface or cloud
installer is introduced. New remote-platform tests remain GitHub-only.

The managed read companion initially advertises only project_open/host_status;
its boot-scoped attachment is not durable service/environment identity. Keep
that distinction until per-account private service ownership and retained PTYs
are implemented. CI Linux's older protoc needs experimental_allow_proto3_optional
for the inherited schema. Native volume checks compare total/bounds rather than
exact free bytes, which can change between samples.

Read companion source 861bd6c/run 37088825313 passed Linux/macOS/Windows wire,
native-root/metrics and real process stdio tests. No durable PTY/task or GUI/live
SSH acceptance is implied. Managed machine SSH must not inherit VIBE_MCP_SERVERS;
it removes that variable alongside the existing native binding scrubber.

Controlled SSH fixture: GitHub's runner password is locked. UsePAM=no rejects
public-key login before authentication; UsePAM=yes respects system account checks
without unlocking the user or enabling password/interactive auth. Source7035082
run37089944901 passed Linux SSH read checks and all three companion review artifact
builds. Loopback Linux does not prove physical-host or remote macOS/Windows SSH
behavior. Structured SFTP uses the native SSH subsystem and never ls/batch
filename interpolation; file-only profiles need no companion path.

Tokio1.47.1 stdio poll_shutdown is a no-op; short-lived protocol refusals must
flush().await before returning an error. Native run37090488200 reproduced a
missing stdout frame; fixed in the shared retired endpoint and checked by real
process repetition. Sourcefc8964c/run37090383877 passed all three read-companion
suites and actual Linux SSH/SFTP bytes/escape/disconnect checks.
Persistent identity uses nonsecret account-local metadata with native private
permissions and std File::try_lock (supported by pinned Rust1.92). Root identity
requires native directory identity plus creation time; unsupported birth identity
must not silently become path-string equality. This does not add retained runs.

Upstream cloud installer removal: disable the inherited install_binary network
deployment boundary, remove its Oz download template/helper and developer deploy
script, retain ordinary SSH and manually provisioned legacy protocol support.
The new companion is a different bounded protocol and must not be substituted
into the inherited proxy command. New managed installation remains pending.
Identity source6ab9021 passed Windows native private-directory identity tests and
20 real retired-endpoint refusal repetitions. macOS/Linux failed only test cleanup
when the state directory lived inside the replaced project root; isolate test
state from the root before revalidation.

Structured SFTP mutations checkpoint (CI pending): create private directories,
nonrecursive selected-entry deletion, non-overwriting rename, bounded exclusive
partial upload, SHA-256 reread verification and original-intent reconciliation.
New-file upload never replaces an existing destination. Transport uncertainty
retains commit_unknown; no automatic mutation retry or partial cleanup. Atomic
overwrite/editor save/resume, streaming large files, queue/GUI and Windows native
SFTP root acceptance remain pending. Linux controlled SSH test now checks actual
binary upload, reconnect/reconcile, competing destination, rename and delete.

Account-service checkpoint (CI pending): the clean stdio executable proxies to
one private background owner using the same bounded protobuf codec. The native
service lock prevents duplicate owners; current-user IPC and 32-attachment bound
reuse existing OS helpers. Boot identity survives proxy/SSH disconnects; idle
exit occurs only after 60 seconds without attachments. Current capabilities remain
read-only, with no retained PTY/task claim. Actual subprocess tests verify boot
reuse, fresh connection IDs and duplicate-owner rejection; Linux SSH tests now
require one shared service boot. Task Store/PTY retention still require integration.
SFTP source4104bae/run37091911301 passed all three focused platform suites and
controlled Linux byte upload/reconnect/conflict/rename/delete checks.

Native Connections/Projects source is now a separate existing Tools Panel view
with seven fixed design states. It is exposed only in the isolated debug preview
until its visual gate passes; no mock connection is a production capability.
Capture adds 56 width/theme/zoom combinations and draft assertions (219 PNGs
per OS). Live profile forms and remote IO remain pending this native image review.
Account-service source31a21d3/run37092447246 passed all three focused platform
suites, real stdio detach/reattach and duplicate-owner checks, and actual Linux
SSH shared-boot plus SFTP mutation checks. No PTY retention claim follows.

Native terminal primitive checkpoint (CI pending): reuse audited Unix
openpty/setsid/TIOCSCTTY and Windows argv/ConPTY process-attribute sequences in
agent_bus without desktop services. Windows uses the OS ConPTY API and an owned
job assigned before child resume; no external desktop DLL. Structured absolute
executable/root, bounded argv, inherited binding scrubber, size validation and
observed exit are explicit. The real process check verifies an actual terminal,
owned multilingual cwd, Unicode input and resize on each OS. Service session
retention/replay/generation guards are not implemented by this primitive alone.

SSH profile metadata uses a versioned 64-profile/128-KiB bounded JSON file,
UUID selection and strict SshProfile validation. Invalid input is never saved;
corrupt storage reports a fixed failure rather than silently overwriting. Native
no-follow entry opens prevent reading symlink/reparse targets; atomic saves reuse
the existing configuration helper. Tests include actual isolated persistence and
unchanged targets on rejected saves. Live UI/profile integration remains pending.
Windows native PTY source58804df/run37094628621 revalidates the shared ConPTY
STARTF_USESTDHANDLES setup after the real child test detected inherited CI stdin.
Linux/macOS PTY checks passed source892621b; Windows rerun remains pending.

SFTP streaming now separates the 16-MiB editor bound from a caller-limited
1-TiB transfer ceiling. Both directions use 32-KiB buffers; uploads pin the
source digest before IO and validate it again before remote reread/commit. No
resume or conditional overwrite is claimed. Download SHA-256 describes received
bytes; coarse SFTP size/mtime checks do not lock external writers. Controlled
CI now checks disk bytes, local writer failure, 16-MiB-plus transfer and changed
source refusal. This checkpoint is pending exact-source GitHub verification.


Retained terminal backend source064f5cb/run37104521972 passed focused checks on
Linux/macOS/Windows and controlled Linux SSH/SFTP. The account service owns PTYs;
SSH attachments own revocable generation-fenced input leases. Launch UUID retries
reconcile the same immutable run, with bounded replay and explicit queue admission
versus execution. ConPTY must supply child standard handles via STARTF_USESTDHANDLES
(as in the desktop primitive) and remain owned until all job processes exit, then
close so retained output can reach EOF. Unix retains the exited leader with WNOWAIT
until owner release to avoid signaling a reused process-group ID. Background-child
follow-up source28a97cb is pending exact-source verification.

Panel-to-workspace actions that open tabs or change active sessions must use the
existing deferred typed dispatch: a synchronous action can update panel visibility
while the originating panel is temporarily removed from the app's view map.
Windows native capture exposed this at shared-tab confirmation. Source28a97cb uses
the helper for workspace open/file/focus actions; capture verification remains open.


Background ownership source3a37ed2/run37105738779 is verified on all three remote
platforms plus controlled Linux SSH/SFTP. Darwin excludes zombies from group
signal recipients and can return EPERM for a zombie-only group; bounded libproc
membership must confirm only the exited leader before classifying that error as
already ended. Permission failures with other members remain errors. Native tests
observe child readiness/status and repeat job exit observation for ConPTY EOF.
R0.3 identity contract/inventory and R4.3 session/run/attachment ownership are
accepted independently; live terminal/sidebar/task integration remains pending.


Remote task Store/API source550753b/run37107398041 is verified on all three
remote platforms plus controlled Linux SSH/SFTP. A native project UUID selects
an owned private SQLite/Broker instance on the companion host, reusing Store;
GUI attachments share its authority and never upload/open a local desktop Store.
Persist the canonical root binding because the existing engine keys private
projects by path: an externally renamed root must fail visibly rather than hide
history under a fresh path domain. Explicit root-domain migration is pending.
GUI task control denies Agent lifecycle/file capabilities and retired federation;
R2.6 fresh remote MCP and R2.7 local pending-intent projections remain pending.

- Remote lifecycle evidence (2026-10-03): Darwin PTY EOF and leader exit can precede
  owned background-process exit. Release and idle exit must use bounded native
  group/job activity, retaining ownership on observation failure. Source 65e1b86,
  GitHub run 37110078158, passed all three remote targets. Concurrent identity-file
  contention is transient: wait within a bound rather than rejecting attachment.

- User correction, 2026-10-03: SSH work means extending the already working
  same-project Agent communication to remote SSH Agents, plus concise status.
  Do not make a complete SSH project/file/transfer/session-management product a
  prerequisite. Active specs use S1–S5; former R0–R7 manager specs are archived.
  Reuse existing Broker/MCP, terminal adapters and collaboration panel; manual
  companion placement is sufficient. Private run MCP source 9a8e692 passed
  Linux/macOS/Windows in GitHub run 37110965089; live workflow remains pending.

- User correction, 2026-10-03: scope reduction requires deleting previously
  produced redundant code/features and wiring, not leaving dormant foundations.
  Keep only what is required for SSH same-project Agent communication and concise
  existing-panel status. Preserve original terminal/local communication behavior.

- SSH pruning, 2026-10-03: connection-owned Agent IO replaces retained session
  management. Disconnect revokes MCP and stops the owned native group/job; failed
  activity/stop observation cannot authorize owner release. Removed device runtime
  must not remove v6 migration, original pending intents or read-only provenance.
  New cleanup/migration checks are pending GitHub verification.

- SSH Agent workflow source 06aa074, run 37122708969, passed Linux/macOS/Windows
  and actual controlled OpenSSH: two owned native Agent fixtures exchange a
  message and complete assign/start/submit/review; other roots are isolated.
  `warpai-companion agent <root> <program> <absolute-executable> [args...]` reuses
  session::launch, native PTY IO and the existing private MCP/forward relay.
  Disconnect ends the child heartbeat and prevents adoption. S3 accepted; live
  panel/final desktop acceptance and paid vendor calls are not covered.

- Remote Linux evidence must inspect the opened file descriptor via /proc/self/fd,
  then apply the same root/credential/regular-file/bounds/hash checks. Unix opens
  use O_NOFOLLOW/O_NONBLOCK. Source 06aa074 verified this alongside Store migration
  and local communication regressions; desktop-only handle inspection fails remote
  task evidence even when SSH/MCP themselves work.

- S0 pruning accepted using backend source70a5e1d/run37123392723 and desktop
  source06aa074/run37122711581. Removed the extra SFTP/Connections/metrics/device
  runtime/session manager, preserving local Store migration and original terminal
  core. Native SSH status fixtures retain terminal drafts. S4 selection belongs
  to the existing panel only, transient, with system SSH authentication and one
  serialized exact-project client; reconnect keeps original human intent.

- SSH panel authentication remains system-owned: use an existing SSH alias with
  trusted host fingerprint and working noninteractive authentication. The control
  pipe uses BatchMode/StrictHostKeyChecking; do not add password/key storage or
  automatic trust. Manually deploy a source-matched Companion artifact and use
  the same canonical remote root for the panel and explicit Agent launch. Remote
  review artifacts provide version/source/checksum/capability manifests. The old
  V01–V24 SSH-manager acceptance matrix is archived; active acceptance is S0–S5.

- S4 accepted on 2026-10-04: source4fcb0c3/run37138929300 passed macOS/Windows
  native panel actions and eight live SSH image reviews per OS. SDK tools discovery
  registers a temporary Agent name before the fixture's explicit final registration;
  native acceptance must wait for that final name before addressing a human message.
  Keep product request identity/fencing unchanged; do not retry an obsolete name.
  S5 release packaging is still pending.

- SSH communication scope S0–S5 completed on 2026-10-04 at tested source
  4fcb0c3c0f7f1354c7f79bb53d10f6a9ec31b66e. Remote run37138932460 passed
  Linux/macOS/Windows plus controlled Linux OpenSSH two-Agent messaging/tasks.
  Desktop run37138929300 passed macOS/Windows default/platform, app/local
  regressions, native actions/screenshots and release packaging. Downloaded
  package digests/source match GitHub; macOS bundle and Windows installer/
  portable runtime contents checked. Final acceptance edits are documentation only.
  Use README's transient existing-panel selection and manual source-matched helper
  setup. Paid vendor sessions, physical cross-device tests and release publication
  are not claimed. Earlier pending manager/checkpoint notes are historical.

- Collaboration UI consistency uses Global Search panel text roles and the shared
  `settings_page::render_body_item` for local MCP rows. Keep related native actions
  in wrapping groups and field labels 4px from their editors; preserve all focus,
  disabled/submitting guards and warnings. Native capture now includes six
  communication-settings images (light/dark/Claude Warm Light, zoom 1/1.25),
  with fixed unavailable CLI rows only in the isolated debug profile and no vendor
  configuration calls. Source 33e717d passed full macOS/Windows regression and
  packaging run 37180907136. Final source 3b31566 passed both application variants,
  protocol tests and all 161 native captures per OS in run 37188935250.

- Native screenshot requests can time out or lose their callback on CI. Retry up
  to three fresh frame requests in the shared post-step capture handler, without
  replaying UI actions. Preserve exact-count/nonempty PNG acceptance; exhausting
  retries must not produce an accepted run. This resolved two different Windows
  missing-frame failures reproduced at source a5699b9.

- User documentation preference on 2026-10-04: README screenshots must show the
  default Claude Warm Light theme. Reset the native capture theme before the
  documentation scenarios; use source-matched raw PNGs and update both language
  captions/provenance. Do not recolor screenshots or change the application's
  default theme to compensate for the capture driver's earlier Dark selection.

- User's Warpai product positioning (2026-10-06): present it as an agent-first
  workflow built on a full-featured terminal, with Agent sessions as the work
  units and a lead Agent delegating, collecting, reviewing and integrating work.
  Explain how Agent management, terminal, project files/editor/preview and code
  review reduce app switching; include research, writing and other non-code
  workflows, and describe Rust/UI/rendering stack. Avoid user-facing launch/MCP
  mechanics. Reuse existing README screenshots. Accuracy boundary: the
  companion-based SSH workflow supports Agent messaging, tasks and task-result
  reviews; Project Explorer and Git Code Review remain local-workspace tools.

- Collaboration UI polish on 2026-10-04: panel title is now the strongest text role (14px semibold primary); SSH entry uses a Secondary button and shares a row with Refresh; panel spacing uses GAP_TIGHT/ROW/SECTION in panel.rs. MCP settings group agents under an "Agents" sub-header and show legacy-cleanup warnings in ui_warning_color, superseding the earlier ordinary-contrast rule. Native captures/README images must be regenerated on GitHub.

- Identity cleanup checkpoint: retained the user-completed panel/format changes;
  removed 35 unconsumed obsolete documents and four replaced/unused source files
  (444,578 bytes). Kept macOS UserDefaults for the required Apple press-and-hold
  OS integration. Canonical file stores separate public/private preferences;
  migration imports missing legacy entries once and preserves recovery sources.
  Package/executable identity and owned bridge are Warpai/warpai/warpai-agent;
  the final repository/default branch will be OthinusG/warpai and main.
  Static manifest/lock/workflow/shell/documentation checks passed; full GitHub
  acceptance of this combined source remains pending.

- Final delivery ordering reaffirmed by the user: finish identity/cleanup
  acceptance, then repair remaining UI mismatches against UI-CONSISTENCY.md and
  verify native captures before pushing the default branch. Preserve the existing
  user styling rather than replacing already-correct controls. Remote source
  e9a02ca passed Linux/macOS/Windows run 37205656289; desktop gate remains pending.

- Startup isolation correction: `run()` must not migrate user data before the
  capture driver configures its temporary home/profile. Keep the single migration
  call at the beginning of `run_internal()`, after `Builder::build()` runs setup
  and before logging/preferences/state. Completion/help/worker entrypoints also
  avoid unintended migration. Both native CI captures now reject creation of an
  ordinary-home migration marker. The e9a02ca desktop gate is superseded by this
  correction; its remote acceptance remains valid historical evidence only.

- User corrected delivery ordering: finish UI before screenshots and installation
  package builds. Cancel interim desktop run 37205655815; its completed checks
  are partial evidence, not final acceptance. Final UI uses existing secondary
  notes for archive/reservation metadata and native settings surface_1 status
  colors, preserving warning overrides and interaction guards. Finish all source
  changes before dispatching the single combined-source final gate.

- User supplied a terminal >_ glyph for the main workspace top-right Settings
  entry. Recreated it as a transparent 1254px PNG without the enclosing ring.
  Use the existing bundled icon renderer and theme tint in both tab-bar variants;
  omit the anonymous avatar's circular background. Preserve Settings actions,
  other settings glyphs and application bundle branding. Include this final icon
  in the unified native screenshot, functional and package acceptance gate.

- The top-right Tools panel entry also uses a user-supplied icon: four rows of
  circles and rounded bars, reconstructed at 1254px with transparent background
  and no rectangular frame. Replace both horizontal/vertical Tools entry glyphs,
  preserving the separate vertical Tabs panel menu and existing actions/state.
  Supersede desktop run 37209810520; remote run 37209810625 passed but must be
  regenerated for matching final source. Finish icon changes before new captures.

- Icon format correction: all six icons (DeepSeek, Qoder, Trae, Hermes, workspace
  Settings and Tools panel) must be real SVG assets under bundled/svg, never PNG
  or raster embedded in SVG. Central mappings and technical references migrate
  together before the six PNGs are deleted. Follow docs/ICON-WORKFLOW.md for new
  icons: reference, vector geometry, rendering/reference QA, all-call-site update,
  PNG removal, commit, then native screenshots/acceptance. Desktop run 37210399364
  is superseded by this change; remote 37210399494 is historical passed evidence.

- Windows remote run 37211788439 exposed an acceptance-fixture race: killing the
  owned Agent between heartbeat truncation and write can leave an empty file,
  which the unchanged nonempty/stability assertion treats as still alive. Use
  same-directory NamedTempFile writes and atomic persist for fixture heartbeats;
  preserve production process-stop behavior, deadlines and ownership assertions.
  Rerun both gates from the same final source after this fixture correction.

- User correction: starting CI is not completing acceptance. Keep working through
  desktop regressions, native screenshot review, package inspection and accepted
  default-branch/repository integration. A running compilation must be reported
  as pending, never as passed, and must not end the authorized delivery work.

- Codex ownership correction (2026-10-05): never impersonate vendor commands,
  prepend app aliases to PATH, proxy Codex's daemon/frontend, pin package-manager
  version paths, or write Codex user configuration for MCP. Preserve third-party
  Agent management. Use explicit original-CLI launches with verified native
  per-invocation MCP options and session isolation; user-typed commands stay
  untouched. The legacy Codex adapter must load but never read/write its file.
  Local managed-launch integration and new native model-level acceptance remain
  pending. Older installed app binaries can still contain the withdrawn behavior
  until replaced; do not interrupt active user sessions. Unified CI monitoring
  was explicitly paused by the user; focused rollback verification is separate.

- Rollback verification: source 48072b641f77cd150438984370b0f5703b329ea5
  passed GitHub run 37219398675 on Linux/macOS/Windows. Included session relay,
  native adapter, local communication, managed-process/PTY and controlled Linux
  SSH regressions. Codex configuration no-create/no-write test passed on all
  platforms; argv[0] impersonation regression passed on both Unix platforms.
  README/coverage updates are documentation-only successors. This is rollback
  verification, not new local managed-launch or model-level MCP acceptance.

- Latest Codex UX correction: automatically adapt ordinary Codex invocations
  inside Warpai, including --yolo and resume, while selected communication is
  enabled. The explicit-launch-only/pseudo-command plan is superseded. Adapt only
  the app execution event with official session MCP and --no-daemon; preserve
  original arguments and cwd, package-manager commands and all user files/PATH.
  Continue implementation and verification instead of stopping after a plan.

- Final delivery scope (2026-10-05): finish automatic native Codex adaptation,
  bilingual documentation, verified deep cleanup and main/repository integration.
  Leave icon modification and packaging/release to the next task. Use the new
  check_app gate with protocol_only=true to verify desktop builds without making
  installers. Remove inherited hosted-model Codex promotion UI, not third-party
  CLI management. Superseded explicit-only launch text is no longer current.

- Main-branch pushes verify desktop and protocol code without producing installers.
  Packaging remains an explicit workflow_dispatch operation; icon and release
  work is deferred. Windows PowerShell always uses legacy native quote handling
  even if a variable named PSNativeCommandArgumentPassing is defined.

- Native Codex acceptance passed on 2026-10-05: original Homebrew 0.160.0,
  two simultaneous --no-daemon clients, two real model/MCP turns each and
  independent readiness. Used cloud-built 625904f artifacts whose SHA-256/CRC
  matched; agent_bus sources match f6fa053, including actual native cwd binding. Only test-process trust for the
  owned fixture was overridden. No vendor command/config/PATH/CODEX_HOME changes.

- Native Codex parser rejects duplicate --no-daemon. Reuse installed option arity
  to preserve an existing flag without adding another; values/prompts named
  --no-daemon remain data. Both local and remote launch apply the rule. WSL guest
  commands stay native rather than probing/injecting a Windows host CLI/bridge.

- Windows native argv correction (2026-10-05): full application regression found
  PowerShell 5 splitting an inline MCP path containing spaces; pwsh Standard
  through .cmd also interpreted metacharacters. Encode whitespace/metacharacters
  in compact TOML basic-string values with standard Unicode escapes before shell
  quoting. Preserve decoded values, original command/arguments and user shell
  preferences. Real CI fixtures cover cmd/ps1, powershell/pwsh, all three modes,
  plain/spaced/apostrophe/metacharacter/percent/Unicode paths. The isolated quick
  diagnostic passed 60 cases; it does not replace the application regression.

- Final executable-source acceptance (2026-10-05): eab0ff0 passed desktop run
  37233525889 on macOS/Windows (default + warp_platform checks and focused app
  regressions, including 60 native Windows argument cases and both Ctrl+C paths)
  and remote run 37233527936 on Linux/macOS/Windows with controlled OpenSSH
  messaging/task/review. Original macOS Codex 0.160.0 model acceptance retains
  byte-identical agent_bus inputs. These are code/functional gates, not new UI
  captures or installers. Remaining user delivery scope is icon revision and
  packaging/release. Temporary Windows diagnostic branch/files were deleted.

- Companion artwork correction: keep the Warpai project glyph/base and merge
  the owner-supplied opposing arrows to distinguish the remote connection tool.
  Finish code first; regenerate native installer icons before final packaging.
- Installer source e2fe59e passed all three native installation/reinstallation
  gates and remote regressions in GitHub run 37302612346. Native Windows client
  ControlMaster is unsupported; use original system SSH key/agent authentication
  without credential storage. Add session-only PowerShell integration to the
  terminal SSH banner, never user profiles. Final desktop/UI acceptance pending.

- Companion identity now has its own `warpai-companion.png` master and ICNS/ICO
  exports. Imagegen merges the supplied opposing arrows with the approved black
  Warpai tile. Preserve the desktop icon. macOS volume/install command use native
  Finder icons via AppKit/osascript; Windows setup/uninstall use companion ICO,
  Linux CLI installer bundles the master without a desktop launcher. Small-size
  light/dark previews and a real temporary Finder resource-fork check passed.
- Native SSH source 6786490 passed remote run 37307186273 on all three systems
  plus controlled Linux OpenSSH original-arguments/master/project fencing.
  Desktop application tests and final icon-integrated native capture are pending.

- Desktop run 37307185988 passed both OS default/platform application checks,
  but lib-test compilation rejected two test-only Session constructors missing
  the new redacted SSH argument field. Fixed both constructors, then added
  inheritance/nested-host/redaction and cross-platform remote-path regressions.
  Remote directory typing must use confirmed remote OS before desktop fallback;
  macOS can connect to Windows PowerShell. WSL without its confirmed guest master
  cannot replay SSH arguments through a Windows host client. Rerun required.

- Icon-integrated installer source 5767d96 passed all three native installer
  suites and remote regressions in run 37312251498. Downloaded macOS DMG is
  checked for source-matched manifest/hash, companion volume/Finder icon and
  included master. Add a native Windows cmd/PowerShell byte-protocol checkpoint
  using only the installed CI-owned component; do not assume shell quoting alone
  proves binary stdio. Nested/opaque SSH cannot adopt any desktop-side master.

- Source bee1cf3 passed remote run 37316825893 on Linux/macOS/Windows, including
  native Windows cmd/PowerShell version probe, binary protocol handshake, remote
  project binding and task query using the installed CI component. The system
  shell retains protocol bytes; no alternate transport or credential layer is
  needed. Add focused frontend cases for compact SSH options and PowerShell
  doubled apostrophes, then run final native screenshots and desktop acceptance.

- Final native capture adds explicit SSH PowerShell integration entry frames in
  Claude Warm Light and Dark, preserving terminal drafts and using a static
  banner without opening a network connection. Exact native frame count is now
  163 per OS. Review these alongside the existing 161 frames; do not count a
  successful screenshot request alone as visual acceptance. Final gate pending.

- Final remote source 770741b passed run 37320176774 on Linux/macOS/Windows,
  including native installers/reinstall, Windows binary stdio and controlled
  OpenSSH. Full desktop/native capture run 37320844750 remains pending. Release
  workflows now retain the 1.1.0 draft after Windows upload; explicitly publish
  only after inspecting all downloaded formal assets. The 1.0.1 draft stays private.

- Desktop run 37320844750 passed both configurations/regressions and produced
  163 frames per OS, but visual review rejected the new PowerShell frames: the
  static banner was attached to an idle empty block and never painted. Simulate
  only the block preexec state (no SSH process), assert command/banner/draft,
  restore the block after capture and reserve native height for the added action
  row. Rerun the corrected native capture before release; CI success alone is
  not visual acceptance. Final review installer manifests/icons match 770741b.
- Native capture must echo command characters before preexec: HeaderGrid preexec
  freezes already echoed input rather than inserting its command payload. Start
  the static block and use the existing ANSI Handler, never execute sample SSH.
- Native screenshot assertions need the renderer's read-only block banner state.
  Run 37337077745 found its terminal-only getter inaccessible to the capture
  module. Widen only this existing getter to crate visibility; no state mutation
  or protocol API changes. Keep the failure recorded and rerun exact-source CI.
- Native DMG staging check found that SetFile on the source folder does not
  propagate its custom-icon flag to the image volume. Share create-dmg.sh between
  desktop/companion: mount a temporary writable image, set/check its actual Finder
  volume flag, detach and compress. A read-only remount proved flag/icon/shortcut
  preservation. Formal installers also run install/reinstall and installed Windows
  byte-protocol checks on disposable runners before draft asset upload.
- Windows source 6ec6755 passed full run 37340966290: 163 PNGs, matching native
  diagnostics, GitHub archive SHA-256/CRC and decoded frames. New PowerShell
  action is visible in warm/dark with no overlap. Review found a misleading Oz
  claim and upstream SSH guide in the shared banner; remove that text/link, then
  capture final wording with native checks. Existing control/launch logic remains
  unchanged; macOS full regressions are still pending.

- 1.1.0 full macOS/Windows regressions passed in run 37340966290 (6ec6755).
  Final 7ebbd3e native run 37349315123 passed both platforms; only SSH caption
  and help-link strings changed after full regressions. Both archives contain
  163 decoded source-matched PNGs and successful action diagnostics. README
  uses byte-identical Claude Warm Light macOS vertical-tab frames. Three-OS
  installer/transport gate 37348866696 passed; formal tagged package inspection
  and public release remain the delivery gates.

- Formal 1.1.0 macOS and three-platform companion build/install checks passed
  in run 37355772889. Windows run 37362390293 built correct 1.1.0 assets but
  failed because Inno Setup pads ProductVersion with whitespace. Trim only that
  installer comparison; preserve strict app checks and immutable v1.1.0 source
  574b2f9. Dispatch the corrected main workflow against that same tag.

- Warpai v1.1.0 published as latest on 2026-10-06 after formal macOS/companion
  run 37355772889 and corrected Windows run 37411727676 passed. Immutable tag
  source is 574b2f9; only the main workflow's padded installer-version verifier
  changed after tagging. All five downloaded installers and two checksum files
  match GitHub digests. Windows native artifact/source match, macOS strict bundle
  signature/identity/bridge, both DMG Finder volume flags/icons, companion
  manifests/hashes and installed native protocol checks passed. Public assets
  contain DMG/EXE desktops plus Linux .run/macOS DMG/Windows EXE remote installers,
  no raw ZIPs. Historical private 1.0.1 draft remains unpublished.

## 2026-10-06 — Remote file tools and Companion verification

- User explicitly approved `REMOTE-FILE-TOOLS.md` implementation and requires
  comprehensive Companion checks before later integration. Track receipts and
  remaining release gates in `specs/agent-communication-v2/COMPANION-CHECKS.md`;
  file/preview/Git UI parity remains unfinished until its native acceptance runs.
- Existing SSH selection is shared with file tools. Typed fenced file metadata
  and mutations use the account Companion; content uses system SFTP staging.
  Never enable upstream unrestricted command handlers or desktop-path fallback.
- Real OpenSSH batch tests exposed double escaping of quoted glob characters.
  The batch parser already escapes these; quote/backslash escaping alone passes
  literal-path binary transfers. Keep the actual subsystem round-trip regression.
- Windows std filesystem rename can replace a destination. Remote rename uses
  explicit native no-replace flags; file save uses explicit atomic replace.
  Reject Windows device/stream/trimming aliases and reparse traversal.
- Pin Unix parent descriptors, bound metadata/staging and recursive delete depth,
  and preflight known deletion budgets before mutations. Stale/error transport
  closes control to release connection-owned transfers; crash cleanup occurs
  only under the service's exclusive lock. Do not claim exclusion of external
  filesystem writers or automatically repeat an uncertain mutation.
- Remote rendering caches use hashed names with a safe extension, preventing
  remote drive/reserved-name/case aliases from becoming desktop paths. Remote
  relative assets and original display identity still require UI integration.
- macOS real SSH fixtures require a short master socket path: OpenSSH adds a
  temporary suffix against the 104-byte Unix socket limit. An owned loopback
  fixture passed master/native-argument SFTP operations and closure fencing.
- QoderCN fallback regression was affected by this machine's installed CLI in
  PATH. The lookup accepts an explicit environment-path iterator for isolated
  tests; production path precedence and user configuration stay unchanged.
- Local verification uses installed Homebrew Rust 1.98.1; pinned 1.92.0 and
  Windows/Linux native receipts remain separate gates. The desktop check cannot
  reach application type checking without `xcrun metal`; strict package Clippy
  also reports pre-existing broad lint failures. No cloud jobs were monitored.

## 2026-10-06 — Complete existing SSH file surfaces and simplify collaboration

- User explicitly requires remote Explorer clicks to reuse the in-app code/text
  editor and Markdown preview, alongside remote management and Git Review. Do not
  stop delivery at a tested Companion backend. Cloud push/verification is now
  explicitly authorized, superseding the earlier hold on cloud monitoring.
- Existing components are extended without new panels. Remote files retain an
  original SSH save source behind a private rendering cache; cd does not retarget
  dirty editors. Relative Markdown resources use the same SFTP admission checks.
  Keep source references while Review still uses cached paths, including deleted
  rows, so closing an editor cannot turn a Review open into a desktop-path lookup.
- Cancellation cleanup must distinguish canceled transfers from completed domain
  errors: missing/invalid paths do not disconnect sibling editors. Coalesce rapid
  Review refreshes instead of aborting shared file transfers. Explicit reconnect
  verifies account, canonical root and root identity; never replay uncertain writes.
- Native Git fixture commands must supply their own identity for merges as well
  as commits. A missing CI global identity can abort before creating a conflict,
  producing a misleading merge-conflict regression failure.
- Collaboration's daily view retains Agents, task descriptions, conversations,
  results and actionable approvals. Hide protocol/run identifiers, readiness
  sources, maintenance reservations and revision counters from that view.
- Owned native SSH/SFTP acceptance now has a reproducible repository script and
  existing-panel capture steps. Pinned three-platform/cloud receipts remain
  separate from local Homebrew Rust checks and are recorded after job completion.
- Pinned cloud default and `warp_platform` application checks pass on both macOS
  and Windows at b73f6a57. Native UI acceptance remains a separate gate: the
  capture finish callback must preserve an earlier failing step instead of
  replacing it with a missing-later-screenshot assertion.
- Windows canonical paths may carry a verbatim prefix. Normalize the public
  remote file root before joining cwd children; Windows verbatim paths reject
  mixed separators. Keep native identity comparison on the original HostClient.
- Owned Windows SSH acceptance uses the system OpenSSH server as a disposable
  SYSTEM service, SYSTEM-owned generated host keys, separate icacls owner/remove
  commands, and the native sftp-server subsystem. Never change personal or
  default service configuration for this fixture.
- A read-only Markdown refresh must not advance a dirty editor's original save
  hash. Preserve explicit in-app editor targets and fence delayed preview loads
  against the current document/generation.
- Pinned Linux/macOS/Windows Companion acceptance passes at 4717e61f (run
  37485335027), including actual native Windows SSH/SFTP. Close local staging
  writer handles before SFTP opens those paths on Windows; keep TempPath cleanup
  ownership. Unix-to-Windows SFTP needs `/C:/...` remote drive syntax.
- Native UI diagnosis at 474c3c7f confirmed that the capture's streaming append
  was a system edit, not a user edit. Use the existing user-origin `apply_edits`
  path and await dirty state before saving; do not alter production dirty-state
  logic merely to satisfy a capture.
- User correction (2026-10-07): Explorer and Review retain terminal-driven cwd
  selection after SSH and `cd`. Opening a file must not switch project context.
  Removed file-source overrides from ActiveSession; remote file load/save still
  uses the existing FileModel association. Exclude SSH caches from local roots.
  Hide desktop roots for pending and confirmed remote terminals even when
  remote metadata is incomplete; show an actionable SSH/Companion/SFTP state.
  Native UI acceptance must establish confirmed SSH shell metadata before
  opening files, rather than injecting a remote tree into a local terminal.
- Remote Review feedback from a file-only tab searches open workspace terminals
  by the exact original SSH connection/session and admitted cwd. Exclude hidden
  closed panes, pending SSH transitions, disconnected attachments and terminals
  that left the reviewed root; retain feedback and report failure if no match.
- Real macOS/Windows ProxyJump read/save/conflict/reconnect acceptance passes at
  2ca71cc4 (run 37496901971), alongside all three Companion platform jobs. Reuse
  the isolated native fixture and its generated host settings for both hops.
- Final Companion regression at 60d54422 passes on Linux/macOS/Windows (run
  37498107083), including exact feedback route identity and direct/ProxyJump
  native macOS/Windows file operations. Local agent-bus tests: 134 passed,
  11 opt-in ignored; explicit history-budget and 16 protocol tests also pass.

- Native Windows file capture at 9c4e7f56 verifies Explorer opens, user edits,
  confirmed remote saves and Markdown resources, but is not a completed runtime
  receipt. Screenshots exposed the legacy toolbar support gate hiding Review;
  enable the existing CodeReview item under local_fs while leaving bundled AI
  and notification items disabled. Native assertions must check visibility as
  well as model statistics. Reset fixture panel width and exit the owned SSH
  session before the separate local SSH-banner walkthrough.
- Native captures must focus the owned terminal and use LeftPanelView's current
  file-tree handle, never the first cached FileTreeView across tabs. Open the
  existing tools panel through its normal visibility action before asserting
  rows. Verify the copied Companion discovery installation over SSH before GUI
  launch; only fixed ConnectionError variants may enter capture diagnostics.
- Native application checks at a176dd9f pass in both desktop configurations on
  macOS and Windows (run 37514277064). Windows renders the actual remote Review
  panel after Explorer editing/saving and Markdown image preview; this remains
  a partial receipt because later lifecycle assertions fail. macOS Explorer
  failure reproduces using that cloud-built app against an owned local SSH server.
  Simulated remote shell metadata on a local PTY triggers PtyController's
  InitShell subscription, which writes a real bootstrap into the local shell and
  replaces the simulated SSH selection with local hooks. The isolated capture
  intercepts only its simulated InitShell event while retaining model session
  metadata and forwarding other events; production bootstrap remains unchanged.
  Use the owned SSH master and disable generators in the isolated capture. Do not change production selection to
  ignore legitimate terminal events to make the fixture pass.
- Capture setup clears WARP_COLLABORATION_CAPTURE for normal worker startup;
  debug diagnostics must use the retained preview flag. Windows GUI stderr can
  be absent even on failure, so persist only allowlisted assertion names under
  the owned capture artifact directory and filter them again before publication.
- Remote preview filename/breadcrumb parsing uses the existing typed-path
  dependency so Windows drive/UNC paths render correctly on macOS and POSIX
  literal backslashes remain part of the filename. Explorer uses the Companion's
  admitted canonical cwd before widening to a Git root; an SSH cwd alias must
  not fail a relative-path check against its canonical directory.
- User correction (2026-10-07): keep Warp's original file-click and Review
  interactions. Explorer file clicks open CodeView/document viewers; Git Review
  already uses RightPanelView/CodeReviewView. The SSH backend must reuse the same
  rendering and actions, without a separate SSH Review presentation or automatic
  Review opening on cwd changes.
- Remote Review now shares the original header, file navigation, maximize/close
  actions and visible-only lifecycle. Closing or changing SSH selection calls
  CodeReviewView::on_close; pending SSH never exposes a local repository. Native
  captures require the current confirmed SSH cwd, not just cached remote rows.
- Cloud run 37563576965 at c455e4a6 passes the complete macOS native walkthrough
  and both desktop build/application suites. Windows passes remote Explorer,
  editing/save, Markdown image and original Review assertions, then its local
  SSH-banner fixture panics because simulated input omitted Reset Grid OSC on
  a fresh Windows grid. Supply the real reset hook in that fixture; do not
  disable production grid checks. Full Windows capture remains pending.
- Final source acceptance (2026-10-07): run 37569663988 at 60ea2810 succeeds
  on macOS and Windows, including default/warp_platform builds, application tests,
  complete native file/editor/save/Markdown-image/original-Review walkthroughs,
  local restoration and retained Agent panel captures. The prior Windows grid
  failure was confined to simulated banner input and is fixed by its real reset
  hook. Source acceptance preceded packaging (public baseline was 1.1.0); no release
  or local installation was requested for this delivery.

## 2026-10-07 — Warpai 1.2.0 release preparation

- User explicitly requests packaging and public release of the accepted current
  code as 1.2, with Release notes covering changes from 1.1 and README updates
  integrated into the existing feature sections, not a separate update list.
- GitHub currently exposes 1.1.0 as latest; no v1.1.5 tag or public release exists.
  Earlier source documentation incorrectly described 1.1.5 as published. The
  1.2.0 release includes its prepared cleanup fixes and the accepted remote work.
- Use immutable v1.2.0 source, existing macOS/Windows workflows and all three
  Companion installers. Retain old screenshots and verify formal assets before
  making the draft public. Companion component/protocol versions are independent
  of the product tag; use matching release manifests/capabilities for upgrades.
- Owner correction: README must show the delivered release, both desktop and
  all remote platforms, an understandable delegation flow and explicit remote
  file-management/editor/preview parity. Remote feature copy must describe user
  outcomes, not upgrade/version instructions or transport internals.
- Owner requires versioned public installer names and Companion component 2.0.0.
  Keep protocol major 1. Installer ProductVersion and names derive from the
  compiled component's manifest version, independently of the desktop tag.
- Owner explicitly chooses preserving the unpublished v1.2.0 tag rather than
  replacing it. Cancelled its incomplete run 37575377385; release Warpai 1.2.1
  with Companion 2.0.0 from a new tag. No published historical tag changes.
- Packaging verification passed locally: Python/YAML/Bash syntax, component-2.0.0
  Linux payload naming/manifest preservation, and both desktop public filename
  and SHA-256 mappings in isolated temporary fixtures. These are packaging smoke
  checks, not native installer/runtime acceptance; tagged cloud gates remain
  required. All existing screenshots remain byte-identical.

## 2026-10-07 — Warpai 1.2.1 public delivery

- Published v1.2.1 as latest with Companion 2.0.0 after all formal gates pass.
  Immutable source: 4788cbde9a74987b53a3bd032b340cc38bd09fb7. Preserve v1.2.0 at
  210ebb4a; it remains unpublished. No historical tag or public asset was replaced.
- Formal macOS/three-platform Companion run 37577830960 and Windows desktop run
  37581995715 pass. Native installers verify component 2.0.0, desktop 1.2.1,
  repeated installation, source/hash and Windows optimized binary protocol.
- Downloaded all five versioned installers and both checksum files; all GitHub
  digests and checksum entries match. Windows desktop bytes match the accepted
  native artifact and tag-source receipt. Both mounted read-only DMGs pass
  identity, version, artwork and Finder flags; desktop deep signing and bundled
  Agent bridge pass. Actual macOS Companion reports 2.0.0 protocol 1; Linux
  manifest/binary/license/artwork checks pass. SHA-256 parsing must handle the
  standard Windows binary-mode `*` marker.
- Release notes describe 1.1-to-1.2 changes. README feature sections now describe
  outcomes and remote/local workflow parity, with static 1.2.1 and remote-platform
  badges and an explicit assignment/report/review flow. Old screenshots are
  unchanged. Desktop downloads carry 1.2.1; all three Companion downloads carry
  2.0.0. No local application or personal Agent/SSH configuration was installed
  or changed during release verification. All five unauthenticated public
  download HEAD requests return HTTP 200 after publication.
