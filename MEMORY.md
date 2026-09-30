# Project Memory

## Product Boundary

- warp-lite is an AGPL, local-first fork of Warp Terminal for macOS.
- The default product excludes AI agents, telemetry, cloud account/login, billing, and related platform surfaces.
- Project Explorer is a desired terminal-adjacent feature and must be restored without reintroducing excluded product dependencies.

## Native Agent Communication Design

- User clarification on 2026-09-30: the target is native communication between independent CLI agents running inside Warp Lite, including task delegation, result submission, acceptance/rework, and automatic handoff. hcom and Agent Mail are references, not a required choice or final architecture. The user first requested a design, then approved implementation. First-release coverage was expanded to every CLI agent managed by Warp Lite, including aliases; the user subsequently excluded agents without native MCP support.
- Implementation ownership: a Warp-owned local broker for identities, messages, and task state, a thin stdio MCP bridge over local IPC for CLI access, and automatic handoff through cooperative MCP waiting. The first automatic handoff uses cooperative waiting via MCP; dormant TUI injection remains unverified; it must not reactivate Warp's platform MCP client manager or cloud AI product.
- Qoder command recognition does not imply session-event support: Qoder currently has neither a session listener handler nor a Warp notification plugin manager. Existing CLI session events update terminal status/context; they are not an agent-to-agent task delivery mechanism.
- `TerminalView::submit_text_to_cli_agent_pty` already submits prompts using agent-specific PTY strategies. Reuse it behind explicit readiness, live-run identity checks, and task acknowledgement; its current return value does not prove that an agent consumed a prompt, and its delayed Enter path needs lifecycle revalidation for broker use.

## Build And Release

- Primary check: `cargo check -p warp --bin warp-oss`.
- App packaging: `script/build-warp-lite-app.sh` (macOS), `script/build-warp-lite-windows.ps1` (Windows x64).
- Release artifacts include `WarpLite.app.zip`, `WarpLite.dmg`, `WarpLiteSetup-x64.exe`, and `WarpLite-windows-x64.zip`.

## Maintenance

- Upstream changes are historically selected and applied with provenance rather than merged wholesale.
- Automating upstream sync must retain privacy/product-boundary checks before merging or publishing.
- `OthinusG/warp-lite` is a fork of `terzigolu/warp-lite`; both default to `warp-lite/main` and do not use `master`.
- Project Explorer source remains in the tree. Warp Lite disabled only its `ToolsPanel` entry point; its restoration and the native Mono packaging icon are stored in `.github/patches/project-explorer.patch` and replayed by `script/restore-project-explorer.sh` after every upstream merge.
- Workspace initialization restores the right-side Tools Panel button for older persisted toolbar configurations that omit it.
- `.github/workflows/sync-upstream-warp-lite.yml` syncs only the latest upstream `v*-lite` release tag, verifies before push, and publishes ad-hoc-signed macOS ZIP/DMG artifacts under the same tag name.
- macOS packaging uses Warp's native 1024×1024 `app/DockTilePlugin/Resources/mono.png`; manual workflow dispatches create a run-specific release tag and preserve older assets because GitHub's asset endpoint returned 404 when `gh release upload --clobber` tried to replace them.
- Antigravity CLI (`agy`) support is preserved across upstream syncs by `.github/patches/antigravity-cli.patch`; it uses the standard CLI Agent fallback and does not add telemetry transport or Warp AI/Cloud dependencies.
- DeepSeek Harness TUI support is preserved across upstream syncs by `.github/patches/deepseek-harness.patch`; only `dsh-tui` and `dsh` TUI profiles enter CLI Agent management, with no listener, plugin, or telemetry upload added.
- DeepSeek Harness uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles; the standard monochrome icon renderer turned the original RGB PNG's opaque white background into a blank block.
- Qoder CLI support is preserved across upstream syncs by `.github/patches/qoder-cli.patch`; `qoder`, `qodercli`, `qoder-cli`, and domestic China release `qodercn` commands enter CLI Agent management with no listener, plugin, or telemetry upload added.
- Qoder CLI uses a transparent RGBA bundled PNG through the original-color image path for tab/status circles, matching DeepSeek Harness.
- Trae CLI support is preserved across upstream syncs by `.github/patches/trae-cli.patch`; `trae`, `traecn`, `trae-cli`, and `traecn-cli` commands enter CLI Agent management with no listener, plugin, or telemetry upload added. It uses a transparent 60x60 RGBA bundled PNG (`trae.png`) rendered through the original-color image path for tab/status circles.
- Hermes CLI agent support is enhanced and preserved across upstream syncs by `.github/patches/hermes-agent.patch`; it adds the official `hermes-agent` command alias, one-off shell keyword bypass, and a transparent 60x60 RGBA bundled PNG (`hermes.png`) rendered through the original-color image path for tab/status circles.
- `script/restore-upstream-cli-agents.sh` reads `warpdotdev/warp` master during sync, restores the vetted third-party CLI Agent patch, and automatically imports newly introduced non-Warp CLI Agent commits only through audited CLI-integration paths. `WarpTui`, product/network additions, and telemetry send calls are rejected; DeepSeek Harness, Qoder CLI, and Trae remain independent downstream patches.
- Cursor's CLI starts with `cursor-agent`, while upstream Warp currently recognizes only `agent`; `.github/patches/cursor-cli-command.patch` adds the official command without removing the legacy alias and is replayed by the CLI Agent sync script.
- The default Lite build does not register Warp MCP file watchers, file-based server management, or MCP gallery; it retains an inert `TemplatableMCPServerManager` singleton only for compiled Warp AI callers. The full MCP runtime remains available in `warp_platform` builds, while third-party CLI agents manage their own MCP configurations.
- Upstream Warp GitHub Actions workflows (such as internal release pipelines targeting GCS/Sentry/Slack, internal repo-sync, Oz AI agent bots for triage/implementation, and proprietary multi-platform CI) were removed from `.github/workflows/`; only the fork's release and synchronization workflow (`sync-upstream-warp-lite.yml`) is retained.
- `.github/workflows/release-windows-x64.yml` triggers automatically via `workflow_run` when `Sync Warp Lite and release` finishes. It inspects the release, skips redundant runs if Windows assets already exist, compiles `warp-oss` for `x86_64-pc-windows-msvc`, builds `WarpLiteSetup-x64.exe` (via Inno Setup) and `WarpLite-windows-x64.zip`, and attaches them to the GitHub release.
- Windows compilation fixes are preserved by `.github/patches/windows-compilation-fixes.patch` and replayed by `script/restore-project-explorer.sh`; it replaces dead `crate::();` telemetry removal leftovers in `app/src/autoupdate/windows.rs` with logging and corrects the `&OsStr` argument type to `powershell_read_all_text_command` in `app/src/terminal/model/session.rs`.

- Agent communication source changes must be delivered as a replayable downstream patch integrated into upstream synchronization. The user explicitly requires GitHub compilation and no local builds.
- Communication coverage must follow the managed `CLIAgent` enum automatically, not a separate four-vendor allowlist. The user explicitly excludes agents without native MCP support: list them, do not add shell-tool fallbacks. Protocol simulation and real vendor model acceptance are separate verification levels.

- macOS Unix socket endpoints must use a short private directory under `/tmp`; the system `TMPDIR` path plus a UUID can exceed macOS's 104-byte socket path ceiling. Protocol compilation/tests run on GitHub, not locally.

- User clarification: custom/downstream agents also qualify when native MCP is confirmed. `qodercn mcp add --help` confirms native stdio, and its recognized alias maps to Qoder. Do not exclude `Unknown` sessions categorically; verify their MCP capability individually.

- Native communication reuses interprocess 1.2.1's Tokio transport, already used by `crates/ipc`. Avoid sync `PIPE_NOWAIT` polling: Windows pipe semantics do not match Unix socket polling. Bound IO with async timeouts and acknowledge received frames before closing pipe connections.
