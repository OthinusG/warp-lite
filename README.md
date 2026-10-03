# warpai

**Application name: Warpai.** This development version renames the app, menus, settings, notifications, and installer assets. Existing settings remain compatible. Published `v0.5.7-lite` packages still use their historical WarpLite names; current review packages come from the [validation workflow](https://github.com/OthinusG/warp-lite/actions/workflows/validate-agent-communication.yml).

**A local-first block terminal for macOS and Windows, with project browsing and native communication between your own CLI agents. No bundled cloud AI, telemetry, account login, or billing.**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE-AGPL)
[![Latest release](https://img.shields.io/github/v/release/OthinusG/warp-lite)](https://github.com/OthinusG/warp-lite/releases/latest)
[![Platform: macOS](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://github.com/OthinusG/warp-lite/releases/latest)
[![Platform: Windows](https://img.shields.io/badge/Windows-x64-blue.svg)](https://github.com/OthinusG/warp-lite/releases/latest)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](rust-toolchain.toml)

Warpai is independently maintained in [this repository](https://github.com/OthinusG/warp-lite), derived from [terzigolu/warp-lite](https://github.com/terzigolu/warp-lite) and [Warp Terminal](https://github.com/warpdotdev/warp). It is a local-first, GPU-accelerated block terminal for macOS and Windows with no Warpai account login, no bundled cloud AI, no cloud onboarding, and no telemetry as a product requirement. Source changes are committed directly; no upstream synchronization or patch replay is required. The inherited licenses, attribution and terminal core remain preserved.

> Status: alpha, with published macOS and Windows x64 packages. The current build is **v0.5.7-lite**, a privacy-vetted August upstream sync with 193 integration commits covering terminal security, reliability, editor/Vim, tabs, performance, and platform compatibility. GitHub Releases includes macOS app/DMG downloads and Windows x64 installer/portable ZIP downloads.

Latest release:

- Download the newest published build from [releases/latest](https://github.com/OthinusG/warp-lite/releases/latest).
- Current build: `v0.5.7-lite`.
- macOS artifacts: `WarpLite.dmg`, `WarpLite.app.zip`.
- Windows x64 artifacts: `WarpLiteSetup-x64.exe`, `WarpLite-windows-x64.zip`.
- Agent-to-agent communication is under development on `warp-lite/agent-communication`; it is not included in the published `v0.5.7-lite` packages.

See [`FORK_NOTICE.md`](FORK_NOTICE.md) for upstream Warp attribution and licensing.

## What this fork adds

The original **warp-lite** keeps the upstream block terminal, GPU rendering, shell integration, tabs, panes, editor/Vim, themes, completions, and Markdown viewing. It removes or disables the upstream bundled AI platform, cloud/account/login flows, billing, onboarding, and telemetry surfaces. Those changes remain the foundation of this repository; the original release history and terminal guardrails are preserved below.

**This fork adds tools for working with local projects and independently installed CLI agents:**

| Improvement over terzigolu/warp-lite | What you get | Delivery |
| --- | --- | --- |
| Restored Project Explorer | Browse project files from the native Tools Panel and its toolbar button. | Available on `warp-lite/main` |
| Native Mono app icon | Uses the upstream Mono artwork for macOS icon assets. | Available on `warp-lite/main` |
| Broader CLI agent recognition | Added Antigravity (`agy`), DeepSeek Harness TUI, Qoder/QoderCN (`qodercn`), and Trae/TraeCN; improved Hermes and Cursor command aliases. Agent icons appear in the existing terminal UI. | Available on `warp-lite/main` |
| Windows x64 distribution | Download an installer or portable ZIP, with Windows-specific compilation fixes retained. | Published release assets |
| Direct source maintenance | Project browsing, CLI integrations, communication, branding and platform fixes live directly in the repository. Builds do not fetch or merge upstream. | Independent maintenance |
| Keep-awake toggle | A tab-bar button holds off idle system sleep while tracked CLI agents are working (display may still sleep). Works on macOS and Windows. | Available on `warp-lite/main` |
| Local agent-to-agent collaboration | Agents exchange messages, assign work, submit results, accept results, or send them back for revision. Busy agents keep a queue; once idle, Warpai submits the next inbox instruction automatically. | In development; see below |

Third-party CLI agents are installed and authenticated by you. Supporting them does not restore the upstream bundled AI service or require a Warpai account. Their own provider connections remain under their control.

### Agent collaboration: upcoming

The communication feature is being developed on [`warp-lite/agent-communication`](https://github.com/OthinusG/warp-lite/tree/warp-lite/agent-communication). It adds a bundled local bridge for agents with native MCP support, including eligible custom agents. QoderCN has been confirmed to support the required local connection. Agents without native MCP support are excluded rather than given a shell-based workaround.

Open **Settings > Features > Agent communication**, enable communication, and check the installed CLI agents you want to participate. Warpai configures the bundled native MCP bridge in the background, including Codex environment passthrough. Unchecking an agent or disabling communication revokes its live access immediately and removes only Warpai-owned configuration. Running agents may need a restart to load the change; setup failures and unsupported installed versions are shown explicitly.

Agents with a loaded bridge automatically discover other participating agents in the same project. No tab-menu pairing is required. The project boundary is the canonical repository root, so repository subdirectories share communication and separate repositories/worktrees remain isolated. Agents can exchange messages and delegate tasks from ordinary conversation or authorized work. The receiver submits results and verification evidence; its reviewer accepts or requests revision. Warpai polls queued work while busy and submits an inbox instruction after readiness is reported. Agents without completion hooks follow the readiness rule supplied by the bridge.

Automatic delivery preserves user drafts and does not answer permission requests. Messages stay pending until the receiving agent actually acknowledges or consumes them. Coordination stays on your machine; it adds no Warpai cloud account or hosted messaging service.

The source is maintained directly in this repository. Automated protocol checks and authenticated tests with real vendor agents are separate: **live vendor acceptance is still pending**, and the feature is not yet part of the published release. See [coverage and setup](specs/agent-communication/COVERAGE.md), [receiver state audit](specs/agent-communication/RECEIVER-STATES.md), [expected behavior](specs/agent-communication/PRODUCT.md), and the [GitHub validation workflow](https://github.com/OthinusG/warp-lite/actions/workflows/validate-agent-communication.yml).

### SSH Agent communication: in development

Extend the existing same-project communication to CLI Agents running through SSH.
Agents in the same remote project share a private Broker/Store on that account;
the existing collaboration panel shows messages, tasks and concise connection
and Agent status. System OpenSSH owns authentication. Remote accounts support
Linux, macOS and Windows; desktop targets remain macOS/Windows.

Manual placement of the repository companion is sufficient. A file manager,
transfer queue, host resource monitor, independent Connections dashboard and
retained-session manager are outside this delivery; their redundant additions
have been removed. Historical device enrollment remains disabled; migration keeps
history without restoring device authority.

Remote project authority, private per-run MCP and explicit Agent launch have passed
three-platform checks, including two native Agent processes exchanging a message
and completing a reviewed task through controlled OpenSSH. Native panel actions
and screenshot review passed on macOS and Windows at source4fcb0c3; final desktop
release packaging remains pending.

For the panel, enable Agent communication and open **Agent collaboration >
Connect SSH project**. Enter a system SSH alias, the absolute remote project root
and the absolute installed companion path. The selection lasts for that panel;
it does not save another SSH profile. Review the host fingerprint and establish
working noninteractive authentication with your system SSH client first. The
panel requires an already trusted host and does not prompt for passwords or keys.

Use **Reconnect** after a lost connection. The last projection is shown as stale,
writes are disabled offline, and unsent forms survive reconnect. A write with an
unknown outcome keeps its original intent; explicitly retry unchanged content to
reconcile it. Close the current form before changing the target or returning to
**Local**. Use the same canonical remote root in the panel and Agent launch.

Use the source-matched companion artifact for the remote host's OS/architecture
from the [remote validation workflow](https://github.com/OthinusG/warp-lite/actions/workflows/validate-remote-companion.yml).
Place it at a user-chosen executable path on that account; use the artifact's
manifest to match its source/version to the desktop review build.

For a manually provisioned remote account, open an ordinary SSH terminal and run:

```sh
/opt/warpai/warpai-companion agent /srv/project codex /usr/local/bin/codex
```

Use the companion and vendor executable paths installed on that remote host. Run
another managed Agent in a second SSH terminal with the same canonical project
root. Vendor authentication stays remote; no desktop credentials are copied.
The connection owns the run, so closing it stops the Agent. Native adapters and
cooperative MCP readiness retain their existing behavior; no blanket automatic
wake guarantee is added. The current companion is a source-matched review build,
not included in the published release. See the [active plan](specs/agent-communication-v2/PLAN.md) and
[acceptance evidence](specs/agent-communication-v2/PROGRESS.md).

## Why

Upstream Warp is excellent, but includes a large agentic-development and cloud surface that some users do not want in their terminal. This fork keeps the terminal core and progressively removes or disables the product surfaces around AI, cloud sync, billing, onboarding, telemetry, and account login.

Goals, in order:

1. **Local-first.** No Warpai account required. No login gate for opening a terminal.
2. **Terminal-first.** Preserve the block terminal, GPU renderer, shell integrations, tabs, tab groups, panes, settings, themes, command palette, editor basics, completions, and markdown rendering.
3. **Independent maintenance.** Keep source, tests and releases under this repository's control, preserving original provenance when selectively adopting external fixes.
4. **Lighter over time.** Remove AI/cloud code paths carefully without breaking terminal rendering or input.
5. **Honest status.** Some source modules are still present while the default build avoids their product paths. This README tracks that split explicitly.

What this fork is **not**: a closed-source repackage, an MIT relicense, or a project maintained by the upstream Warp team. The AGPL applies and cannot be downgraded.

## Looking for a Warpai alternative?

warpai is aimed at people who like Warpai's terminal UX but not the platform around it:

- You want Warpai's blocks, panes, tabs, command palette, and GPU-accelerated rendering — **without bundled Warpai AI** in your prompt. Your own CLI agents can still run in the terminal.
- You want a terminal that opens **without a login or account**, ever.
- You want **no cloud sync** and **no telemetry**: your commands and history stay on your machine.
- You searched for "Warpai terminal without AI" or "Warpai without login" and found mostly settings toggles — this fork removes those surfaces at the source level instead.
- You prefer **open-source (AGPL), Rust-based** terminal software you can audit and build yourself.
- You are fine with alpha software on macOS in exchange for a lighter, local-first terminal.

If you want the upstream bundled AI agents, cloud drive, and team features, upstream [Warp](https://github.com/warpdotdev/warp) is the right choice — this fork intentionally goes the other way.

## Install

Download the latest macOS or Windows x64 release from:

```text
https://github.com/OthinusG/warp-lite/releases/latest
```

On macOS, open `WarpLite.dmg` and drag `WarpLite.app` into `/Applications`. On Windows x64, run `WarpLiteSetup-x64.exe`, or extract `WarpLite-windows-x64.zip` and run `WarpLite.exe`.

New development packages use `Warpai.app` / `Warpai.app.zip` on macOS and `WarpaiSetup-x64.exe` / `Warpai-windows-x64.zip` on Windows. The application executable is named `Warpai` / `Warpai.exe`.

The packaged app uses:

- Bundle identifier: `dev.warp-lite.WarpLite`
- App name: `Warpai`
- Current bundle version: `0.5.7-lite`

## Current Shipped State

The current build is `v0.5.7-lite`.

| Area | State | Notes |
|---|---|---|
| Terminal core | Works | Core terminal view/input/model files are preserved. Do not wholesale stub them. |
| macOS app bundle | Works | `script/build-warp-lite-app.sh` builds `Warpai.app`. |
| DMG release | Works | `WarpLite.dmg` is published in GitHub Releases. |
| Windows x64 packages | Published | Installer and portable ZIP are available on GitHub Releases. |
| Platform product boundary | Enabled | Default Lite omits `warp_platform`; billing, referrals, rewards, pricing UI/model, and selected AI startup/background paths compile only for platform builds. |
| Warpai login gate | Disabled | `skip_firebase_anonymous_user` is enabled by default. Startup, "skip login", and visible account/billing menu entry points are hardened away from Warpai auth in the lite build. |
| Telemetry product goal | Removed/neutralized | Historical telemetry call-site cleanup is part of the fork; keep auditing before claiming perfect network silence. |
| Project Explorer / Tools Panel | Restored | The native Project Explorer and its top-right toolbar launcher are restored without reintroducing AI, account, or cloud product surfaces. |
| Context Panel | Removed from shipped UI | The experimental Context Panel was deleted from the app wiring in `v0.5.1-lite` after causing instability and stale data issues. |
| Codex / Claude Code notifications | Kept | These are intentionally preserved for the lite fork. |
| Warpai MCP manager | Disabled in Lite | Warpai's MCP config watcher, server runtime, gallery, and settings page are not started; third-party CLI agents retain their own MCP configuration. |
| Markdown viewer | Kept | `markdown_tables` and `markdown_mermaid` remain in defaults. |
| Bundled Warpai agent mode | Not a target | Warpai agent-mode product surfaces stay out of Lite; independently installed CLI agents are supported separately. |

## Inherited upstream release history

The history below records the original Warpai cleanup and upstream integration work. The downstream additions maintained by this repository are listed [above](#what-this-fork-adds).

### v0.5.7-lite — August upstream sync (2026-08)

- Integrated 193 vetted upstream-sync and Warpai adaptation commits while keeping AI/agent, cloud account, billing, team, remote-control, and new telemetry changes out of the default Lite product.
- Added security hardening for external links/downloads, command and SSH escaping, environment-aware blocklist checks, auth-log redaction, OSC 52 clipboard control, and dependency fixes.
- Improved terminal and shell reliability across PTY writes, wide-character resize, inline images, OSC hyperlinks, process-group cancellation, zsh/PowerShell bootstrap, SSH, and remote sessions.
- Expanded editor, Markdown, file-viewer, and Vim behavior, including autosave settings, non-ASCII find/replace, local image refresh, natural file sorting, and additional Vim motions/actions.
- Added and stabilized horizontal/vertical tab grouping and pinning, persistence, cross-window drag behavior, multi-pane headers, Quake-window focus, hotkey-window behavior, and macOS window chrome.
- Reduced avoidable Git/filesystem watcher work, process sampling, path canonicalization, glyph work, and duplicate font scanning; added Windows/WSL, old-Mesa Intel Xe, bootstrap, and build-script compatibility fixes.
- Verified default and `warp_platform` checks, focused crate suites, test compilation graphs, process-group regressions, script syntax, diff hygiene, and a fresh optimized release build. Full audit: [`WARP_LITE_SYNC_2026-08.md`](WARP_LITE_SYNC_2026-08.md).

### v0.5.6-lite — Product boundaries and measured slimming (2026-07)

- Added the positive `warp_platform` compile boundary while keeping it out of the default Lite feature set.
- Removed billing, referrals, rewards, pricing UI/model, upgrade modals, and pricing-dependent terminal allocations from the default Lite compilation path.
- Removed AI initialization/keybinding registrations, scheduled ambient-agent startup work, its schedule implementation, and the Agent status-bar tip singleton from Lite.
- Preserved terminal input/view/model and persistence contracts instead of replacing them with broad stubs.
- Added revision-aware static and native runtime benchmark harnesses under `script/`.
- Reduced the same-machine release binary from **268,410,752** to **267,208,032 bytes**: **1,202,720 bytes (about 1.15 MiB)**.
- Verified default and `warp_platform` checks, both test compilation graphs, release compilation, benchmark dry-run behavior, and diff hygiene.

### v0.5.5-lite — Upstream sync (2026-07)

A large, privacy-audited catch-up with upstream Warp. Fork point `bc3fffa` was **927 commits** behind upstream `d375729`; **139 improvements were cherry-picked** (`-x` for AGPL provenance) after a strict per-commit review. Selection rule: bugfix / performance / terminal-feature only, and **rejected** if the diff reintroduced any telemetry, outbound network client, or AI/cloud/auth/account surface. The applied diff was audited — no new `send_telemetry`, `reqwest`, Firebase, or GraphQL network calls were added.

- **New feature: vertical tab grouping** — group, rename, reorder, and move tabs between groups (upstream #11749, #11791, #11842, #11849, #11903).
- **Performance:** avoid cloning the whole file tree on view updates (#12221), async presentation on macOS (#11326), input hot-path cleanup (#10927), fewer redundant SVG rasterizations (#12104), fixed a `WeakModelHandle` zombie-handle leak (#11767).
- **Stability / crash fixes:** flat-storage `RowIterator` underflow after clear (#12085), secret redaction across multibyte UTF-8 (#9521), block up/down navigation (#10095), plus ~60 more fixes.
- **Terminal / shell:** tab CWD + git branch from OSC 7 escape sequences (#9279), empty zsh `RPROMPT` handling (#11868), Intel(R) HD Graphics 2500 added to the buggy-iGPU list (#11454).
- **Editor / files:** configurable code-editor line numbers (#10012), show-hidden-files toggle in Project Explorer (#9532).
- **Intentionally skipped (6):** upstream changes entangled with removed AI/cloud/auth code — e.g. horizontal tab-group rendering (needs removed AI imports), SSH/remote-auth transport, and the code-review discard-panic fix — were aborted rather than force-merged, to avoid dragging removed surfaces back in.
- Verified green with `cargo check -p warp --bin warp-oss` and launch-tested. Full lists: [`WARP_LITE_SYNC_GAP_2026-06.md`](WARP_LITE_SYNC_GAP_2026-06.md) and [`WARP_LITE_SYNC_APPLIED_2026-07.md`](WARP_LITE_SYNC_APPLIED_2026-07.md).

### v0.5.4-lite

- Removed the normal prompt's unsupported AI toolbar in the lite build, including Agent/Auto mode switching, `auto (cost-efficient)`, slash AI commands, `@` AI context, and AI file attach controls.
- Redirected hidden settings entry points such as Account, billing, teams, Warpai Drive, and Warpai Agent pages to the supported Appearance settings page.
- Kept CLI agent rich-input infrastructure separate so Codex/Claude Code notification and context surfaces can continue to work where they are explicitly supported.

### v0.5.3-lite

- Refreshed this README to match the real v0.5.2-lite state.
- Hardened the no-login path so the compiled lite feature bypasses auth onboarding even if runtime flags drift.
- Hid or no-op'd remaining visible sign-up, upgrade, referral, logout, and anonymous-user menu/actions in the lite build.

### v0.5.2-lite

- Restored `skip_firebase_anonymous_user` in default features.
- Fixed the regression where the welcome/sign-up modal still appeared and "Skip for now" attempted Warpai/Firebase auth.
- Rebuilt and published fresh `WarpLite.dmg` and `WarpLite.app.zip` release assets.

### v0.5.1-lite

- Slimmed default features.
- Removed the experimental Context Panel source/wiring from the shipped app path.
- Gated additional agent/cloud management UI surfaces.

### v0.5.0-lite and earlier

- Gutted large parts of codebase indexing and AI-adjacent background work.
- Deleted or stubbed several AI/cloud peripheral crates.
- Removed large telemetry call-site surface from earlier phases.
- Preserved the terminal renderer/input stack after a failed over-aggressive stub attempt proved that compile success is not enough.

## Removed From Source

These crates or app modules are no longer present in the current tree:

| Path | Status |
|---|---|
| `crates/integration` | Removed |
| `crates/firebase` | Removed |
| `crates/voice_input` | Removed |
| `crates/handlebars` | Removed |
| `crates/warp_js` | Removed |
| `crates/warp_graphql_schema` | Removed |
| `crates/command-signatures-v2` | Removed |
| `crates/serve-wasm` | Removed |
| `crates/managed_secrets_wasm` | Removed |
| `crates/app-installation-detection` | Removed |
| `app/src/onboarding` | Removed |

## Still Present And Needs Work

These modules still exist and should be treated as the next cleanup targets. Some are default-disabled, partially stubbed, or unreachable in normal lite flows, but they are not physically gone.

| Path | Why it matters | Suggested next move |
|---|---|---|
| `app/src/auth` | Login UI and auth flow still exist in source. Startup/skip/menu paths are hardened in the lite build, but the module is not physically gone. | Continue shrinking or feature-gating auth UI internals after verifying shared `AuthStateProvider` consumers. |
| `app/src/ai` | Large AI UI/product surface remains. | Continue surgical feature-gating and deletion; avoid terminal core wholesale stubs. |
| `crates/ai` | Still a major compiled/source dependency. | Continue reducing agent/indexing/ambient modules behind stable APIs. |
| `crates/onboarding` | Still present even though app onboarding module is gone. | Finish crate-level cleanup if consumers are gone or can be stubbed safely. |
| `app/src/billing` | Source remains for platform builds, but the module and reachable UI are excluded from default Lite. | Keep the `warp_platform` boundary compile-green during upstream syncs. |
| `app/src/voice` | Voice feature source remains although `crates/voice_input` is gone. | Remove dead app-side voice surfaces or gate them out. |
| `crates/graphql` (`warp_graphql`) | The GraphQL client remains in the resolved graph despite platform product UI cuts. | Remove only after its protected terminal/workspace consumers have neutral ownership boundaries. |
| `crates/websocket` | Network transport crate still exists. | Verify consumers, then stub or delete if no terminal feature needs it. |
| `crates/warp_server_client` | Warpai backend client remains. | Audit call sites and remove once auth/cloud dependencies are gone. |
| `crates/managed_secrets` | Cloud/secret product surface remains as a stub candidate. | Keep API only if required, otherwise delete. |
| `crates/warp_files` | Cloud/file integration residue. | Audit dependencies before removal. |

## Guardrails

The terminal works because its core was preserved. Keep these files off any broad deletion or wholesale-stub plan:

- `app/src/terminal/view.rs`
- `app/src/terminal/input.rs`
- `app/src/terminal/block_list_element.rs`
- `app/src/terminal/view/`
- `app/src/terminal/input/`
- `app/src/terminal/local_tty/terminal_manager.rs`
- `app/src/terminal/alt_screen/alt_screen_element.rs`
- `app/src/terminal/model/`

If a change makes the build green by replacing terminal rendering/input/model code with small stubs, that change is wrong for warpai. Verify with launch testing, not just `cargo check`.

## Build

The fork keeps upstream's macOS build prerequisites:

1. Full Xcode, not just Command Line Tools.
2. Metal Toolchain:

   ```sh
   sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
   xcodebuild -downloadComponent MetalToolchain
   ```

3. git-lfs:

   ```sh
   brew install git-lfs
   git lfs install
   git lfs pull
   ```

4. Rust toolchain pinned by `rust-toolchain.toml`.

Build directly from repository source and include the communication companion when packaging. Current development builds and validation run on GitHub for macOS and Windows. Commands for other contributors:

```sh
cargo check -p warp --bin warp-oss
CARGO_BUILD_JOBS=4 cargo build --release --bin warp-oss
CARGO_BUILD_JOBS=4 cargo build --release -p warp-agent-bus --bin warp-agent
script/build-warp-lite-app.sh
rm -f Warpai.dmg
hdiutil create -volname Warpai -srcfolder Warpai.app -ov -format UDZO Warpai.dmg
```

Verification used for the latest release:

```sh
cargo check -p warp --bin warp-oss
CARGO_BUILD_JOBS=4 cargo build --release --bin warp-oss
codesign --verify --deep --strict --verbose=2 WarpLite.app
hdiutil verify WarpLite.dmg
```

Known caveat: full `cargo fmt --check` can currently fail because the repository still references disabled/removed upstream files. Prefer targeted formatting/checks until that cleanup is complete.

## Independent Build and Release

`validate-agent-communication.yml` checks committed source on macOS and Windows and uploads a macOS review package. No build replays patches or fetches upstream source. Linux is an approved remote
companion/project target; Linux desktop builds remain outside the target scope.

Dispatch `release-macos.yml` with an existing repository version tag. It tests and builds that exact tag, packages `Warpai.app.zip` and `Warpai.dmg`, and creates a release without overwriting older assets. `release-windows-x64.yml` then reads the tag from that successful run's release-target artifact, checks out the same tag, and attaches the Windows installer and portable ZIP. The Windows workflow can also be dispatched for an explicit existing release. Release publishing remains separate from validation.

## Branch Structure

```text
origin/warp-lite/main         default branch; current shipped work
origin/warp-lite/agent-communication  development branch; upcoming agent collaboration
```

Historical upstream-tracking and synchronization branches may remain as provenance; they are not an active maintenance or build mechanism.

Historical phase branches and tags may still exist, but the public state should be read from `warp-lite/main`, the tags, and the GitHub Releases page.

## Release History

| Tag | Summary |
|---|---|
| `v0.5.7-lite` | Privacy-vetted August upstream sync: 193 integration commits for security, terminal/shell reliability, editor/Vim, grouped/pinned tabs, performance, and platform compatibility. |
| `v0.5.6-lite` | Added `warp_platform` compile boundaries, removed pricing/account and safe AI startup work from default Lite, and added measured release/runtime benchmark gates. |
| `v0.5.5-lite` | Large privacy-safe upstream sync: 139 cherry-picked bug/perf/terminal improvements, incl. vertical tab grouping, with all telemetry/network/AI/cloud changes rejected. |
| `v0.5.4-lite` | Removed unsupported prompt AI controls and redirected hidden settings pages away from Account/signup surfaces. |
| `v0.5.3-lite` | README refresh, no-login hardening, and remaining visible account/upsell action cleanup. |
| `v0.5.2-lite` | No-login hotfix; fresh DMG/app zip assets. |
| `v0.5.1-lite` | Default feature diet and Context Panel removal from shipped app path. |
| `v0.5.0-lite` | AI/codebase-index cleanup and bundle version bump. |
| `v0.4.0-lite` | Managed secrets/onboarding reduction work. |
| `v0.3.x-lite` | Context Panel experiments; later removed from shipped path. |
| `v0.2.x-lite` | Telemetry call-site cleanup, UI hiding, niche crate removals. |
| `v0.1.0-lite` | Initial default feature purge and first green lite build. |

## FAQ

**Is Warpai a Warp Terminal alternative?**
Yes. It is an independent open-source fork of Warp's AGPL source that keeps the block terminal, panes, tabs, and command palette, and removes the AI, cloud, account, and telemetry product surfaces. It is a Warp alternative for people who want the terminal without the platform.

**Does warpai require a login or account?**
No. There is no login gate, no sign-up prompt, and no Warpai/Firebase account flow in the lite build. The app opens straight into a terminal.

**Does warpai send telemetry?**
Telemetry removal is an explicit product goal: historical telemetry call sites have been cleaned up, and upstream changes that would reintroduce telemetry or outbound network calls are rejected during syncs. Auditing continues before claiming perfect network silence — see [Current Shipped State](#current-shipped-state) for the honest status.

**Does warpai work on Linux or Windows?**
Desktop targets are macOS and Windows x64. Planned SSH remote projects and their
companion also target Linux; Linux desktop/UI remains outside the product scope.

**Can I use Codex, Claude Code, QoderCN, or other CLI agents?**
Yes. Independently installed CLI agents can run in Warpai, and this fork extends command recognition and icons for additional agents and aliases. Upcoming agent-to-agent communication requires native MCP support and local setup; see [coverage and setup](specs/agent-communication/COVERAGE.md). It does not enable the upstream bundled AI service.

**Is the AI code completely gone from the source?**
Not yet. Some AI/cloud/auth modules still exist in the source tree but are disabled, gated, or unreachable in the shipped lite build. The [Still Present And Needs Work](#still-present-and-needs-work) table tracks this split honestly; removal continues incrementally.

**Is this project affiliated with Warp or Denver Technologies, Inc.?**
No. warpai is an independent AGPL fork and is not maintained, sponsored, or endorsed by the upstream Warp team. The upstream "Warp" trademark belongs to Denver Technologies, Inc. — see [`FORK_NOTICE.md`](FORK_NOTICE.md).

## License

- Source code: **AGPL-3.0-only** (see [`LICENSE-AGPL`](LICENSE-AGPL)).
- `warpui` and `warpui_core` retain their original **MIT** license (see [`LICENSE-MIT`](LICENSE-MIT)).

Trademark "Warp" belongs to Denver Technologies, Inc. See [`FORK_NOTICE.md`](FORK_NOTICE.md).
