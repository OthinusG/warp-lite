# Project Memory

Updated 2026-10-10. Current decisions and unfinished work only. Read this with
[AGENTS.md](AGENTS.md) and [README.md](README.md). Detailed historical receipts
and superseded decisions are in [the archive](docs/history/MEMORY-2026-10-10.md);
consult it only when a specific historical question requires it.

## Working and delivery rules

- Rust compilation/tests and native desktop acceptance run on GitHub, not locally.
  Local Rustfmt syntax parsing, diff checks and script/document checks are allowed.
  Do not install on personal machines or change personal Agent/SSH configuration.
- Check cloud runs every 30 minutes, without frequent polling or empty updates.
  A user-reported error permits immediate diagnosis. Use focused retries while
  debugging, then one complete source-matched acceptance run before delivery.
- Starting CI is not acceptance. Report uninspected/running/failed gates accurately;
  finish authorized fixes and delivery without repeated permission questions.
- Keep historical assets/receipts and active gh-pages; remove only authorized,
  merged, clean, unused branches/worktrees. Retain four validation/release workflows.
- Release immutable tags through existing workflows; both desktops build the same
  source. Follow [release gates](specs/RELEASE.md), including five installers and
  checksums. Finish UI acceptance before screenshots/packages. Protocol major is 1.

## Current patch work: 1.5.1 and 1.5.5

- Last delivered baseline: Warpai 1.5.0 / Companion 4.0.0. The owner explicitly
  ended its audit; do not resume it. Current patch work is not yet accepted/released.
- b.txt has eight items (duplicate number 7): first seven UI items are 1.5.1;
  Data usage is 1.5.5. Keep Companion 4.0.0 if unchanged. Contract:
  [PATCHES-1.5.1-1.5.5.md](specs/agent-communication-v2/PATCHES-1.5.1-1.5.5.md).
- 1.5.1: icon toolbelt entry; native local worktree management/removal;
  Coordinator dropdown; `Agents Collaboration Mode` heading; section icons,
  dividers and consistent spacing; secondary selectable History beside Message;
  Message/Assign recipient/reviewer/prerequisite dropdowns in both modes.
- Also 1.5.1: show Rescan agents / Remove all MCP only inside enabled Agent
  communication settings. Remove the nonfunctional App icon selector, retaining
  package artwork and the independent Dock visibility setting.
- Validate the completed combined 1.5.5 source directly, backport shared fixes to
  the independent 1.5.1 checkpoint, then package/publish 1.5.1. Skip a redundant
  intermediate native walkthrough; retain tagged installer gates. Do not publish
  1.5.5 automatically or include Data usage in the 1.5.1 source.
- Combined branch: `feature/collaboration-patches-1.5.1-1.5.5`. Independent branch:
  `patch/1.5.1`, worktree `/private/tmp/warpai-patch-1.5.1`, checkpoint ca42a88e.
  Main remains the delivered baseline. Preserve both checkpoints until delivery.
- Both desktops failed run 38016331795 on redundant Paragraph::with_soft_wrap;
  fixed in 4974c5e6. Native compatibility fixes follow in 74649624.
- Latest full combined validation dispatched:
  [38018432379](https://github.com/OthinusG/warpai/actions/runs/38018432379), source
  74649624, check_app/capture_ui enabled. Result has not yet been inspected.
  Require default/warp_platform builds, focused tests and 274 native PNGs per OS
  (60 new usage/settings frames), including focus/cache and independent scrolling.
- Remaining: inspect/fix cloud results, review native captures, propagate shared
  fixes to 1.5.1, update acceptance/release receipts, then release exact 1.5.1.
  Local syntax/diff/license checks do not establish runtime or provider acceptance.

## Project, worktree and Agent ownership

- Warp's native worktree system owns creation, catalog and tabs; creation opens a
  new tab without starting an Agent. No separate worktree registry/session manager.
- Canonical Git common-directory identity joins registered checkouts/subdirectories,
  including empty tabs and worktrees outside main. Ancestry, basename, branch and
  remote URL cannot establish membership.
- Non-Git folders are independent canonical-root projects with Worktree unavailable.
  They neither inherit nor clear a Git project's Coordinator. Preserve separate
  clones and SSH host/account/service authorities.
- Project mode lists online project participants across checkouts, filtering before
  pagination. Worktree mode permits same-checkout direct messages; cross-checkout
  communication goes through the selected Coordinator. File authority stays local
  to each checkout. Panel reads must not change admission/permissions.
- Mode belongs to the project independently of Coordinator. Any eligible online
  Agent can become Coordinator; switching demotes the previous one without ending
  its process or changing task ownership.
- Coordinator belongs to the project and exact native run, surviving pane/tab/
  project/focus changes; only process termination/revocation or app shutdown clears
  it. A same-name restart cannot inherit authority. Handoffs remain bounded and
  include unfinished-task counts plus paginated instructions.
- Remove local worktrees via native Git: confirm/recheck exact registered path;
  protect main/open/locked/dirty/untracked/ignored checkouts; never force/delete
  branches. SSH removal is outside this patch.
- Sidebar owns sessions; panel owns project messages/tasks/reviews/history. Reuse
  native widgets, prominent modes and 8px spacing. Default closed; events must
  preserve focus/drafts and never open the panel automatically.
- Derive Agent coverage/icons from managed CLIAgent, preserving aliases and custom
  native-MCP-capable sessions. No duplicate vendor allowlist or shell-tool fallback.

## Usage accounts and native Orca port

- Orca source: https://github.com/stablyai/orca at
  `3e0b82835856dde57f43a47661cf736e3e3dd90d`. Preserve MIT notices in both bundles.
  [Support and compatibility matrix](specs/agent-communication-v2/USAGE-PROVIDERS.md)
  owns provider/auth details; do not invent unsupported adapters or quota values.
- Port to native reqwest/TLS roots/system proxy, secure storage, CLI discovery and
  widgets; no Electron/Node/preload/React or temporary-clone runtime dependency.
- Accounts/visibility persist application-wide; manually connected credentials
  use OS secure storage, never account metadata/IPC/logs. Readings are cached in
  the application singleton. Project/worktree/tab/pane/focus changes do not reset
  them. Restart restores accounts and queries fresh readings.
- CLI login means desktop vendor login; expired OAuth requires vendor sign-in.
  Use synthetic fixtures, never personal credentials; they do not prove live
  authenticated vendor availability.
- Data usage is pinned outside main scrolling: fixed header, independent scroll,
  at most four visible account rows, native Agent icons and percentage bars.
  Unknown limits stay unavailable; verified Zen USD balance stays a balance.
- Preserve request parity, fixed HTTPS hosts, sensitive headers, bounded IO and
  generation guards. Tests cover twelve HTTP contracts and representative payloads.
- Antigravity uses the plain resolved agy executable, version >=1.1.11 and the
  exact free /usage metadata flags. Accept successful usage envelopes with log
  noise; stop subsequent polling if a model turn is detected. Other refreshes
  preserve that guard and hidden caches. Update agy/restart to restore eligibility.
- Background CLI processes prepend the resolved binary directory to PATH and
  suppress Windows console windows. Qoder CN remains without a verified safe
  background usage adapter; do not submit a potentially billed model prompt.

## SSH, task integrity and files

- Active scope: [PLAN](specs/agent-communication-v2/PLAN.md),
  [API](specs/agent-communication-v2/API.md),
  [Worktree contract](specs/agent-communication-v2/WORKTREE-COLLABORATION.md).
  Archived enrollment designs are historical. Cross-host task federation is not
  a goal/acceptance/follow-up item; do not call out its absence in product copy.
- Native SSH/cd selects the confirmed remote project; opening files must not
  change project context. Pending/unconfirmed SSH never falls back to a local root.
  Same selected remote project shares one account-owned Companion Broker/Store.
- Reuse Explorer/Code View/previews/original Review. Companion carries private Git
  and runtime dependencies; vendor CLI/auth remain user-owned. Keep on-demand
  startup/60-second idle shutdown; no tray or desktop auto-deployment requirement.
- Fence SFTP/control by confirmed host/account/root/session/generation. Reject
  traversal/aliases; keep atomic save, no-replace rename, bounded staging/deletion
  and stale-save checks. Never repeat uncertain writes.
- Keep original request UUID/content, task revision/run identity and receipts.
  PTY submission is not Agent acknowledgement or execution. Disconnect/view loss
  does not prove a process stopped; unknown attempts block new ownership until
  explicit guarded reconciliation/override. Preserve unresolved history/export.
- Wake polls until idle; manual input/cancel/run replacement fences delayed Enter.
  Cancel pauses wake until resumed input; closing a view invalidates its run.
  Evidence/test labels never authorize command execution or secret-file access.
- Back up schema migrations consistently; no downgrade or automatic legacy
  identity migration. Preserve read-only legacy metadata, uncertainty and receipts.
  macOS sockets require short private /tmp paths; Windows uses bounded asynchronous
  native named pipes, with a new listener before handing off a connection.

## Product defaults, CLI integration and website

- Identity: warpai package/binary, Warpai app, dev.warpai.Warpai bundle,
  OthinusG/warpai repository; retain internal/protocol/WARP_* compatibility.
  Data/settings: ~/.config/.warpai on both desktops. Migration preserves new
  settings/recovery files and runs after isolated capture profile setup.
- Fresh installs use built-in Claude Warm Light; preserve saved themes. Keep-awake
  is session-only, holds a guard only during tracked Agent activity, and permits
  display/user-initiated sleep. No cloud AI/login/billing/telemetry restoration.
- Ordinary Codex invocations (including --yolo/resume) adapt at Warpai's execution
  event while communication is enabled: official per-session MCP and --no-daemon,
  preserving original CLI/args/cwd/PATH and package-manager ownership. Do not proxy
  its daemon, impersonate vendor commands or write persistent Codex configuration.
  Explicit legacy MCP cleanup is the sole read/edit exception for reserved entries.
- Legacy cleanup disables communication, removes only verified reserved bridge
  entries, preserves other servers and reports failures. Ordinary disable/deselect
  stops future Codex session injection without reading/removing its persistent MCP.
- Keep native Windows argv tests; TOML Unicode escaping precedes shell quoting.
  WSL guest commands stay native without Windows host bridge injection.
- About Update checks stable releases; Download opens the exact platform installer
  or Releases fallback. Startup check defaults off/runs once; no installation writes.
- Docs are bilingual and Agent-first; update existing README sections, not a
  changelog. Keep native screenshots/provenance and approved bundled SVG branding.
- Website: https://othinusg.github.io/warpai/ from independent gh-pages branch,
  EN / and ZH /zh/. Preserve its worktree and app branding assets. Jekyll pages
  assign their own translated strings; self-hosted font subsets must be updated
  and cmap-checked when copy changes. Use configured /warpai baseurl in production.

## Memory maintenance

Keep this file under 200 lines / 12 KB. Record durable decisions and current
unfinished work once; replace superseded statements instead of appending a
second timeline. Put detailed CI/debug/release receipts in their owning spec or
release document. Archive only when historical evidence is still useful, and
never read the entire historical archive as routine startup context.
