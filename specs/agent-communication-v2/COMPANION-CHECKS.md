# Companion verification matrix

Date: 2026-10-07. Scope: existing account service, managed Agent communication,
and the file-tool extension in [REMOTE-FILE-TOOLS.md](REMOTE-FILE-TOOLS.md).

## Acceptance rules

Every operation must prove its target and its effect. For failures, inspect the
original file, destination, buffer/task state and temporary resources, rather
than accepting an error message alone. Never replace a missing native runtime
receipt with a compile check. No personal SSH/configuration/credential fixtures,
cloud model calls or automatic retries of uncertain mutations are required.

`Implemented` below means a runnable check exists, not that every platform has
passed it. Outstanding native receipts and unfinished feature gates remain
explicit. These are release gates, not reasons to enable incomplete UI routes.

## Checks

| Area | Required cases and expected effects | Current evidence / remaining gate |
| --- | --- | --- |
| Installation and compatibility | Native platform/architecture selection; package manifest/hash; explicit executable path; version/protocol probe; corrupt, ambiguous, oversized and missing probe results; older Companion without optional file capabilities | Existing installation/package checks retained. Capability/probe tests implemented. Pinned Linux/macOS/Windows Companion, package, installer and native SSH gates pass in cloud run 37485335027. |
| Protocol framing | Split headers/payloads; EOF during a frame; malformed protobuf; zero/oversized payloads; invalid request UUID; request/response correlation; clean stdout; legacy command rejection; reserved field numbers | `remote_protocol` tests and framed Companion tests implemented. Invalid input never executes a partial operation. |
| Account and project binding | Uninitialized connection; foreign service/boot/connection/project; two connections to one project; same textual root on another host/account; renamed/replaced root; project switch success versus failure | Existing fence/identity tests plus new file tests implemented. A failed switch preserves the old attachment; a successful switch clears its transfers. |
| Service lifecycle | Exclusive owner; concurrent identity initialization; connection-owned runs/transfers; disconnect; idle exit; restart; crash leftovers; no takeover of another run; limited connection slots | Existing service/PTY checks retained. EOF cleanup and owned crash-transfer cleanup implemented; cleanup runs only after acquiring the account service lock. Saturated-slot and kill-during-commit runtime receipts remain pending. |
| Terminal and Agents | Native PTY/ConPTY; Unicode/binary IO; resize; input sequence replay; bounded output offsets; readiness; background child ownership; stop/release; two-Agent collaboration; private MCP revocation after disconnect | Existing native PTY, managed Agent and readiness integration suites retained and run locally. Windows ConPTY suites pass on the Windows runner. Installed vendor model calls remain explicitly opt-in. |
| Task storage | Project isolation; caller permissions; migration/backup; corrupt state; dependency cycles; lease/version conflicts; transaction rollback; disk-full simulation; uncertain attempts; retention/history pagination | Existing storage and coordination suites retained. The 100,000-message / 10,000-task history check is run explicitly rather than left silently ignored. |
| Path boundary | Root and absolute-path mutations; traversal including interior dot components; prefix lookalikes; control/NUL/overlong paths; spaces, quotes, Unicode, leading hyphens; descriptor-relative Unix traversal | File tests implemented. Root deletion is rejected. Remote paths never become desktop operation targets. |
| Native filesystem behavior | Symlink ancestors/leaves; FIFO/special files; denied permissions; replaced roots; Windows junction/reparse traversal; drive/UNC prefixes; alternate data streams, reserved devices and trimming aliases; no-overwrite rename | Unix checks run locally. Windows junction and naming tests added to the cross-platform suite; Windows naming/junction tests pass on the native runner. Windows rename uses native no-replace flags. |
| Listing and limits | Empty/non-Git directories; repeat/lazy listing; sorted names; unknown filename encoding; entry count and encoded byte budgets; enumeration errors; recursive deletion count/depth | Implemented. Enumeration failure cannot become a successful partial snapshot. Oversized known delete trees fail before any child is removed. Concurrent external tree changes may still cause a partial delete; surface failure and refresh, never replay automatically. |
| Content and saves | Empty/binary/CRLF content; immutable staged read; permissions; partial/oversized upload; wrong hash/path/transfer; external edit between open and save; successful commit; repeated commit; release/drop/project-switch cleanup | Implemented. Wrong/stale uploads preserve originals and destinations. A successful commit consumes its transfer. External processes are not transactionally excluded; detect observed conflicts without promising full filesystem CAS. |
| SSH routing and SFTP | Confirmed master only; no master fallback; native SSH argument preservation; ports/config/jumps/identities; SSH/SFTP flag collisions; literal batch quoting; missing executable; failed/stalled process; host/account/root validation; closed master | Argument checks, real native SFTP and controlled SSH/SFTP checks implemented. Both master and native-argument paths run locally. Missing/failed/stalled transfer process checks run with virtual time. Native Windows SSH/SFTP passes in cloud run 37485335027. Real macOS/Windows ProxyJump read/save/conflict/reconnect checks pass in run 37496901971. Actual missing-subsystem and WSL runtime receipts remain pending. |
| Resource and recovery behavior | Bounded control frames, metadata, file staging and subprocess duration; per-attachment transfer count; cleanup after normal/error/disconnect/restart; no automatic mutation replay; actionable missing/denied/conflict states | Implemented ceilings: 1 MiB control frame, 512 KiB metadata budget, 4,000 entries, 64 recursive levels, 16 MiB per file and 8 transfers per attachment. Preview cache is bounded to 128 MiB and 1,024 entries per attachment; active/dirty sources remain pinned. Mutation errors refresh the existing tree without replay. |
| Existing file UI | Ordinary SSH/cd follows cwd; stale result ignored; same-name files in separate sessions; open tabs retain identity; create/rename/delete/navigation; unsaved buffer survives failure; local OS actions unavailable remotely | Shared selection/tree/FileModel routes are implemented. Real SSH native captures exercise Explorer clicks, dirty editing, verified remote saves and preview; macOS and Windows native walkthroughs pass in run 37569663988 at 60ea2810. |
| Existing previews | Every supported in-app format; binary classification; encoding/newlines; remote relative Markdown resources/links; temporary asset lifetime; disconnected cached viewing | Existing editor/Markdown routes are implemented, with remote-relative resources and links, original save sources and stale-result fencing. The native Markdown/relative-image walkthrough passes on both desktops in run 37569663988; other supported viewers share the existing load pipeline. A plain cached file is insufficient proof of relative-resource correctness. |
| Existing Git Review | Repository versus non-repository versus missing Git; staged/unstaged/untracked/rename/delete/binary/conflict; unborn HEAD; worktrees; comparison modes; concurrent changes; bounded patches/base content; feedback and mutation target | Typed bounded Git backend and parity fixtures pass for HEAD/main/branch, staged/unstaged/untracked/rename/delete/binary/conflict, unborn HEAD and worktrees. Original Review rendering passes on both desktops in run 37569663988. No unrestricted remote command handler or desktop Git fallback is permitted. |
| Build and packaging | Pinned Rust and lockfile; default desktop/affected `warp_platform` consumers; standalone Companion on Linux/macOS/Windows; source-matched installers; workflow syntax; existing local/SSH/Agent regressions | Local Companion checks are runnable independently. Desktop check currently stops at missing Apple Metal compiler. Three-platform workflows have been dispatched and monitored. Linux/macOS/Windows Companion suites pass. Both desktop default and warp_platform checks pass at b73f6a57 and 9c8d2a4. Final native UI gates pass on both desktops in run 37569663988. |

## Reproduction commands

```sh
cargo test -p remote_protocol --locked
cargo test -p warp-agent-bus --locked --no-fail-fast
cargo test -p warp-agent-bus --lib representative_history_pages_within_budget_and_preserves_live_work --locked -- --ignored --nocapture
cargo check -p warp-agent-bus --bins --locked
cargo check -p warpai --bin warpai --locked
```

Controlled real SSH checks reuse `tests/ssh_companion.rs` and the isolated fixture
in `.github/workflows/validate-remote-companion.yml`. The fixture provisions its
own keys and known-host entry and enables `Subsystem sftp internal-sftp`.
`WARP_TEST_SSH_CONFIG`, `WARP_TEST_REMOTE_ROOT`, `WARP_TEST_COMPANION_PATH` and
`WARP_TEST_AGENT_FIXTURE` designate owned test resources, never personal keys.
On macOS the test master socket uses a short `/tmp` path: OpenSSH adds a
temporary suffix and the native Unix socket path limit is 104 bytes.

```sh
python3 script/verify-ssh-file-tools.py
cargo test -p warp-agent-bus --test ssh_companion --locked -- --ignored
```

OpenSSH's quoted batch arguments already protect glob metacharacters; only
quotes/backslashes need escaping in the batch input. The real SFTP test guards
against double escaping. This behavior is checked against the
[OpenSSH batch parser](https://github.com/openssh/openssh-portable/blob/master/sftp.c).

## Local receipts

- Full `warp-agent-bus` regression: 134 passed, zero failed; its 11 ignored
  entries include owned child fixtures, opt-in network checks and the history
  benchmark. Both controlled SSH tests and the benchmark were also run
  explicitly; installed-vendor model calls were not started.
- Shared wire suite: 16 passed. Standalone Companion binaries pass `cargo check`.
- Native macOS managed protocol/file/service/Agent tests and actual SFTP binary
  round trips run successfully during implementation.
- An ephemeral loopback sshd with generated fixture keys passes both terminal
  master reuse and native-argument file read/write, conflict and closed-master
  checks. No personal SSH settings are changed.
- The explicit history-scale test passes: 100,000 messages, 10,000 completed
  tasks, indexed task page p95 approximately 1.75 ms in the recorded local run.
- The installed Homebrew Rust is 1.98.1; repository/CI is pinned to 1.92.0.
  Local receipts must not be labeled pinned-toolchain or cross-platform receipts.
- Strict package Clippy currently reports existing repository-wide warnings;
  desktop compilation is blocked before application type checking by missing
  `xcrun metal`. These checks are not marked passed.
- Source implementation acceptance includes the complete native remote-file
  walkthrough on both desktops, as recorded below. Native captures use simulated
  shell metadata and a real owned SSH/SFTP server; they do not invoke vendor models.
  Optional service saturation/crash stress cases remain separately identified.

## Pinned cloud receipts

- [Three-platform Companion acceptance](https://github.com/OthinusG/warpai/actions/runs/37485335027): source 4717e61f, all three jobs pass, including real native macOS/Windows SSH/SFTP and controlled Linux SSH.
- [Native ProxyJump acceptance](https://github.com/OthinusG/warpai/actions/runs/37496901971): source 2ca71cc4, all three Companion jobs pass; macOS and Windows additionally repeat file operations through an owned OpenSSH jump host.
- [Final Companion and route-identity regression](https://github.com/OthinusG/warpai/actions/runs/37498107083): source 60d54422, all three jobs pass, including the new exact-session/route identity check and real macOS/Windows direct and ProxyJump file acceptance.
- [Desktop build and native capture gates](https://github.com/OthinusG/warpai/actions/runs/37478223166): source 9c8d2a4e, default/warp_platform checks and selected application tests pass on both desktop targets; native capture failures remain under diagnosis and are not marked accepted.
- [Desktop application regression](https://github.com/OthinusG/warpai/actions/runs/37557583500): source 40210295, default/warp_platform checks and selected application tests pass on macOS and Windows, including remote filename/breadcrumb parsing for drive, UNC and POSIX paths. Native captures are a separate pending gate.

Windows SFTP must receive an absolute remote drive path (`/C:/...`) even from a
Unix client. The local transfer temporary file keeps its path but closes its
writer handle before native SFTP opens it; retaining that handle caused a
Windows sharing conflict. The cloud fixture uses the native `sftp-server.exe`.

- [Final desktop and native acceptance](https://github.com/OthinusG/warpai/actions/runs/37569663988):
  source `60ea2810`, both jobs pass default/warp_platform compilation, focused
  application suites and all native capture assertions. Explorer follows the
  confirmed SSH cwd; original file clicks open the editor and Markdown viewer;
  dirty/save checks verify the original remote file; relative images load;
  the original Review renders the remote diff; exit restores local selection.
  The existing layout/Agent workflow walkthrough also completes on both desktops.
