# SSH Agent Communication Progress

Updated: 2026-10-04. Active scope: [PLAN.md](PLAN.md) S0–S5. All six items
are accepted. Earlier project-manager/M0–M7/R0–R7 chronology is
[archived](legacy-ssh-project-manager/PROGRESS.md), not an active backlog.

| Item | State | Accepted evidence |
| --- | --- | --- |
| S0 Redundant implementation removal | Accepted | source70a5e1d/run37123392723 and source06aa074/run37122711581 |
| S1 Remote private authority | Accepted | source65e1b86/[run37110078158](https://github.com/OthinusG/warpai/actions/runs/37110078158) |
| S2 Fresh per-run private MCP | Accepted | source9a8e692/[run37110965089](https://github.com/OthinusG/warpai/actions/runs/37110965089) |
| S3 SSH terminal communication | Accepted | source06aa074/run37122708969 |
| S4 Existing panel and concise status | Accepted | source4fcb0c3/[desktop run37138929300](https://github.com/OthinusG/warpai/actions/runs/37138929300) |
| S5 Final regression and delivery | Accepted | source4fcb0c3/[remote run37138932460](https://github.com/OthinusG/warpai/actions/runs/37138932460) and [desktop run37138929300](https://github.com/OthinusG/warpai/actions/runs/37138929300) |

Final Companion artifacts for Linux x64, macOS arm64 and Windows x64 match
source4fcb0c3 and their SHA256SUMS, with version/capabilities and license notices.
No paid vendor session or physical cross-device acceptance is claimed.

## User scope correction — 2026-10-03

The user clarified that delivery is the working same-project Agent communication
extended to SSH, preferably with a concise status panel. The previous full SSH
project manager was broader than this goal. PLAN/PRODUCT/TECH now define S1–S5;
previous documents are archived without discarding history or unrelated changes.
File-manager/transfer/automatic-install/session-dashboard work is no longer a
prerequisite. Source 9a8e692, run 37110965089, passed private run MCP and actual
native SDK discovery on all three remote targets. S1/S2 foundations are accepted;
S3 two-Agent launch workflow, S4 existing-panel integration and S5 focused delivery
remain pending. Source 65e1b86 native captures also passed both desktop OSes in
run 37110117064; visual/integration acceptance remains scoped and independent.

### User-mandated implementation pruning (pending validation)

The user explicitly requires no redundant code/features, beyond shrinking the
checklist. Removed new agent_bus SFTP module and streamed-transfer tests, standalone
Connections preview/profile storage/fixtures/toolbelt entry, and host CPU/memory/
disk/uptime collection, wire types, validators and sample states. Capture/workflow
expectations now follow the retained collaboration fixtures only. Existing
upstream file/terminal code is preserved. Session management simplification and
exact-source regressions remain pending; S0 is not accepted yet.

### S0 cleanup follow-up — pending exact-source verification

Removed session listing, detach/reattach and generation takeover from the service,
SSH client and protobuf schema; removed tags/actions are reserved. Runs now belong
to their launch connection. Disconnect revokes MCP, requests owned group/job stop
and retains native ownership until activity is observed ended; closed disconnected
owners are collected. Replaced retained-session acceptance with connection-owned
Agent IO/private-MCP/isolation checks. README follows the corrected scope.

Cleanup source 468b490 run 37113128671 failed at compilation because service idle
logic inherited Instant through the removed metrics import. Fixed with an explicit
service import. No cleanup acceptance is claimed before a successful rerun.

S0 also removes the superseded device enrollment/authentication, federation client,
remote actor registration, remote gateway and device pending-intent runtime modules
(about 5,000 lines). Keep schema migration, read-only provenance and stored original
intents; retired controller operations return feature_unavailable. Added a raw-v6
migration check proving uncertain attempts, receipts, pending intent and backup
survive without the deleted runtime. Native disconnect acceptance now observes the
actual child heartbeat stopping, in addition to refusal of another connection.
These changes are pending exact-source CI; S0 remains unchecked.

Source ce1f686 run 37121782406 passed macOS cleanup/local regression and the Linux
companion/Agent checks. Linux broader Store regression exposed the desktop-only
opened-evidence handle inspection; added the native /proc/self/fd equivalent and
Unix no-follow/nonblocking open. Windows and exact-source rerun remain pending.
S3 source now adds an explicit companion Agent CLI using the existing account
client, PTY IO and session::launch adapters. Private credentials remain child-only;
MCP relay uses the existing forward implementation. S3 is not accepted yet.

S3 acceptance additions (pending CI): two owned native Agent fixture processes
register distinct names through private child bindings, send/read a message,
assign/start/submit/review one task, and exercise the explicit companion Agent CLI
with actual terminal IO. Existing MCP SDK initialization/tools discovery remains
part of each managed child. Vendor credentials/model calls are not used.

Latest cleanup sources remove constant takeover-generation/attached wire fields and
unused fixture flood behavior. The active API contract now contains only project
communication and connection-owned Agent IO; the host-status design is archived.
Workflow YAML/Bash syntax, fixture JSON/unique states, Rust parsing and diff whitespace
checks passed locally. Native tests remain GitHub-only. Exact source 06aa074 is
running focused three-platform + real SSH checks (37122708969) and native desktop
protocol/capture checks (37122711581); no new acceptance mark until completion.

### S3 accepted — SSH terminal Agent communication

Exact source `06aa074b04c4f87687878c0ea82814789f2bd0a8`, [run 37122708969](https://github.com/OthinusG/warpai/actions/runs/37122708969), passed Linux/macOS/Windows focused checks and controlled Linux OpenSSH.
The explicit companion Agent CLI performs real terminal IO. Two native fixture
processes each initialize/discover the existing MCP SDK and use fresh run bindings
to send/read a message, assign/start/submit/review a task; another remote root
cannot see the task or control the run. Disconnect stops the actual child
heartbeat and another attachment cannot adopt it. The existing session/vendor
adapters are reused; paid authenticated vendor calls are a separate limitation.
PLAN S3 is checked. S0 desktop removal regression, S4 live panel and S5 final
delivery remain pending.

Further S0 pruning reduces SSH metadata to target/root/helper/shell plus system
config override for the isolated SSH fixture. User/port/key/jump routing uses
system SSH configuration; removed the redundant authentication helper and profile
IDs/display-name/persistence scaffolding. Exact-source follow-up is pending.

### S0 accepted — redundant implementation removed

Backend source `70a5e1d64d17b0c449814bc5a56b1ba6d051d255`,
[run 37123392723](https://github.com/OthinusG/warpai/actions/runs/37123392723), passed Linux/macOS/Windows focused and controlled OpenSSH checks.
Desktop cleanup source `06aa074b04c4f87687878c0ea82814789f2bd0a8`,
[run 37122711581](https://github.com/OthinusG/warpai/actions/runs/37122711581), passed both native desktop builds, protocol/local regressions and captures.
Removed SFTP manager, independent Connections UI/profile persistence, resource
metrics, retired device runtime and retained-session controls. Original local
communication, Store migrations and upstream terminal/file behavior remain.
Representative native SSH connected/connecting/disconnected/error screenshots
were reviewed at narrow/wide sizes with no clipped guidance or terminal draft
loss. PLAN S0 is checked. The later unused direct rand dependency removal and S4
changes still require their exact-source final regression; no S4 completion claim.

### S4 existing native panel integration — validation running

Source `3145ce496caa741ed595c96831839a49cbfed806`,
[remote run 37128408681](https://github.com/OthinusG/warpai/actions/runs/37128408681), passed all three remote targets, controlled Linux SSH and local regressions.
[desktop run 37128408745](https://github.com/OthinusG/warpai/actions/runs/37128408745) is checking default/platform builds, native form logic, packages and actual companion-driven panel captures.
Panel-only SSH selection, remote typed reads/mutations, explicit human messaging,
stale/offline write guards and draft-preserving reconnect are implemented. Original
request content/version/identity are retained; no automatic mutation retry.
S4/S5 remain unchecked until native action/capture and final checks pass.

S4 verification follow-up: source210a803/run37131968858 passed all native macOS
actions and produced 155 captures. Windows reached the real remote projection
assertion but exceeded its default 10-second step limit; the existing native
startup contract allows 30 seconds. Sourcefc0be7b/run37132942444 passed all three
remote targets after aligning managed-child startup checks with that contract.
Neither result accepts S4: macOS visual review found the Agent/task status below
the initial viewport. Compact connection controls and leading remote status remove
that obstruction; remote Send/Connect forms also omit unrelated navigation/task
context. The projection assertion now allows 45 seconds for startup and polling.

Exact source `de2a72e6321d6c002b60872db0c30123370f9ff0` is being checked in
[desktop run 37134464890](https://github.com/OthinusG/warpai/actions/runs/37134464890)
and [remote run 37134467333](https://github.com/OthinusG/warpai/actions/runs/37134467333).
Default/platform builds, native actions/screenshots and release packaging must
complete before S4/S5 are checked. Earlier source82e20c8 passed both desktop
default/platform checks and focused application tests, but its older native
capture harness failed; its package run is superseded, not final acceptance.

Run37134467333 completed successfully on Linux x64, macOS arm64 and Windows x64,
including controlled Linux OpenSSH and local Store/communication regressions.
Downloaded source-matched Companion review artifacts: all three manifests match
de2a72e, version0.1.0/protocol1 and the four active capabilities; SHA256SUMS matches
each actual binary and both attribution/license files are present. S4 native
actions/visual review and S5 desktop/package completion remain pending.

Desktop run37134464890 passed default/platform checks and focused application
tests on both desktop targets. Windows native capture now passed the real remote
Agent projection, then failed "remote Store confirms the human message". Source
`aaf731a60bebb4fd97110cf554073865bdec76fe`,
[native diagnostic run37136879691](https://github.com/OthinusG/warpai/actions/runs/37136879691),
adds only debug capture state flags and bounded operation error codes; no field,
credential or remote payload is recorded. Message submission versus projection
failure still needs runtime diagnosis. S4/S5 remain unchecked.

Diagnostic run37136879691 passed macOS native actions and all 155 captures;
Windows again failed human-message confirmation. Comparing the real Agent
captures with Broker registration exposed a fixture race: SDK discovery creates
`fixture-<random eight characters>` before the fixture finishes discovery and
registers `fixture-<terminal UUID>`. The harness copied the temporary name before
that rename; a later send correctly rejects the obsolete name. Its initial
projection assertion now waits for the fixture's final registration before any
message/task is addressed. No product mutation semantics or retry behavior changes.
The temporary debug state writer was omitted from the artifact whitelist and is
removed rather than retained as unused instrumentation. Final CI remains pending.

### S4 accepted — 2026-10-04

Source4fcb0c3, [run37138929300](https://github.com/OthinusG/warpai/actions/runs/37138929300),
passed native collaboration action assertions on macOS and Windows. Each artifact
contains 155 nonempty PNGs and a source-matched diagnostic with exit_code0 and no
failed steps. Reviewed all eight live SSH images per OS: three-field selection,
remote Agent/run/last observation, human message, task assignment/detail, offline
write denial, reconnect with the unchanged form draft, and return to local.
Terminal drafts remain visible; key status/actions are accessible without overlap.
The earlier Windows failure was fixture readiness: SDK discovery temporarily names
the Agent before final registration. Acceptance now waits for that final name,
without changing product mutation semantics or adding retry machinery.
S5 remains pending final desktop release packaging and artifact inspection.

### S5 packaging checkpoint — macOS passed, Windows pending

Source4fcb0c3 desktop run37138929300 completed the macOS job successfully,
including release compilation, bundle metadata and strict deep codesign checks.
Downloaded artifact11281186350 and verified nested archive CRC, executable modes,
Warpai/warp-agent, icon/signature and copyright metadata. Its SHA256 matches
GitHub's artifact digest: 16afaeb2b5273d0a1789599073b8f0354f900959aa32c2ae7a21a9c9bdaea73c.
Windows default/platform, application tests and native capture passed; release
packaging is still running. S5 is not accepted until that package is checked.

### S5 accepted — 2026-10-04

Tested source: 4fcb0c3c0f7f1354c7f79bb53d10f6a9ec31b66e. Remote
[run37138932460](https://github.com/OthinusG/warpai/actions/runs/37138932460)
succeeded on Linux x64, macOS arm64 and Windows x64, including controlled Linux
OpenSSH two-Agent messaging/task/review, project/run isolation and disconnect
checks. Desktop [run37138929300](https://github.com/OthinusG/warpai/actions/runs/37138929300)
succeeded on macOS and Windows: local bus/history regressions, default and
warp_platform checks, focused app tests, native action/capture and release packaging.
No eight-hour manager soak was requested or run.

Downloaded desktop review artifacts and verified source/digest provenance and
archive CRC. macOS artifact11281186350 contains executable Warpai/warp-agent,
icon, signature and correct bundle/copyright metadata; CI strict deep codesign
passed. Windows artifact11281363517 contains the installer and portable archive,
x64 Warpai/warp-agent, six runtime DLLs, bootstrap, icon, OpenConsole and bundled
resources (142 portable entries). Windows artifact SHA256 matches GitHub:
3d7b6c87397b00dd6548c463d12f9a8d58c39a2e7623176b621e8d8084e8a3d2.
All three companion binaries already match manifests and SHA256SUMS with licenses.
Reviewed current-source narrow 320px/1.25 zoom light connected and dark disconnected
SSH fixtures on both desktop OSes, in addition to S4 live image review.

README documents the existing panel's three transient fields, system SSH trust/
authentication, source-matched manual companion placement and explicit Agent launch.
S0–S5 are complete. Final acceptance documentation changes no tested Rust source
or workflow. Review packages are available from these runs; no release publication,
paid vendor model call or physical cross-device acceptance is claimed.
