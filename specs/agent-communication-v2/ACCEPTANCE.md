# SSH Remote Acceptance and Historical Evidence

Date: 2026-10-03. Current scope: SSH/SFTP-only, local GUI macOS/Windows, remote environments Linux/macOS/Windows. All V01–V24 gates below are **pending**. Existing local protocol/native screenshot evidence is reusable regression evidence, not SSH Remote acceptance. Source cutover has begun; pending gates are not passed by documentation or source inspection.

## R0 cutover checkpoint — 2026-10-03

Source changes are described in [IMPLEMENTATION.md](IMPLEMENTATION.md). Local
Rust syntax parsing (rustfmt emit only), SQLite schema/cutover SQL, workflow YAML
and diff whitespace checks passed. These are static checks, not Rust compilation.
New checks cover read-only legacy profiles, cleanup without credential reads,
retired controller/gateway refusal and v6 backup/unknown-attempt/intent retention.
Source `4c27262`, [run 37038998741](https://github.com/OthinusG/warp-lite/actions/runs/37038998741),
passed macOS and Windows protocol/setup and representative-history checks.
Application checks were still running at the checkpoint. Subsequent macOS
provider deletion and stronger local-task/receipt/selection checks require a new
exact-source run. V01/V02/V18
and R0 remain pending; no SSH/process/SFTP/GUI gate is claimed.

## Current gate matrix

| Gate | Behavior / package | Smallest runnable success check | Required failure/boundary check |
| --- | --- | --- | --- |
| V01 Source/cutover inventory | SR37/SR38; R0/R7 | Trace D01–D16 to reachable code/callers; verify no obsolete active endpoint after removal | Old enroll/device/grant frames cannot write; unrelated terminal/local features remain |
| V02 Upstream reuse/provenance | SR07/SR38; R0/R2 | Launch source-built compatible companion via audited existing client without remote GUI | Missing server/cloud-only installer/version mismatch rejected; no Oz/account fallback |
| V03 SSH trust/authentication | SR02–SR04; R1 | Controlled SSH account login and fingerprint review on each local OS | Unknown/changed host, bad auth, cancelled prompt and jump-host failure; no auto-trust |
| V04 Environment/project identity | SR05/SR06/SR34; R1 | Same verified root across two managed tabs resolves one project; roots/accounts/hosts separate | Alias retarget, case/path collision and restored capability fail closed |
| V05 Capability reporting | SR04/SR07/SR10/SR39; R1/R2 | SSH+SFTP-only endpoint works; GUI separately reports absent helper/MCP | Missing SFTP, unsupported platform/shell/feature and unavailable helper give actionable states |
| V06 Companion deployment | SR07/SR38/SR39; R2 | Explicit install owned checksummed helper and activate matching version on approved remote targets | Wrong hash/platform/permission, partial upload and activation failure preserve old binary/project |
| V07 Service/process authority | SR20/SR24/SR33; R2/R4 | Two GUI attachments share one writer/project service; remote MCP is private/current-run | Duplicate writer, wrong account/root/boot/attachment, stale capability and process replacement denied |
| V08 Event/receipt recovery | SR33/SR36; R2/R7 | Drop task response, reconnect behind fence, read exact receipt/cursor without duplicate transition | Changed content/request identity, absent receipt under old connection, expired run, stale cursor and disk full |
| V09 Remote Explorer | SR09/SR11/SR16; R3 | Controlled SSH/SFTP list/open refresh matches actual remote tree including Unicode/space names | Denied directory, invalid encoding, large/binary file, malformed filename and symlink cycle |
| V10 Transfer bytes/progress | SR10/SR14/SR17; R3 | Upload/download owned binary and directory batch; compare complete source/destination bytes/hash | Overwrite refusal, source changes, disk full and per-file errors never show complete |
| V11 Transfer interruption | SR15/SR17/SR33; R3/R7 | Interrupt controlled transfer, reconcile partial and resume safely or restart owned temporary file | Lost rename reply, destination replacement, cancellation during final commit and cleanup failure |
| V12 Edit/containment/conflict | SR12/SR13/SR16/SR31; R3 | Remote text edit/save with original fingerprint and confirmed changed remote file | Concurrent Agent edit, path/junction/symlink escape, link replacement, Windows case/drive and unsupported atomic overwrite |
| V13 Real remote terminal | SR18/SR19; R4 | Native GUI terminal reports remote cwd/process marker; command changes only remote owned fixture | Local same-named root untouched; resize, Unicode/IME, paste, Ctrl-C and interactive program regression |
| V14 Session close/stop | SR20–SR22; R4 | Existing sidebar actions detach/close/stop exact owned session, including retained detached row and distinct exit observation | Unreachable/unknown process is not stopped; identical PID/name/new run cannot receive old stop |
| V15 Protected reconnect/input | SR22/SR26/SR35; R4/R5 | Seed local/remote draft, disconnect/reconnect exact retained run and preserve input | Replacement run, delayed Enter, stale attachment, approval/busy and unsupported reattach fail closed |
| V16 Remote Agent/MCP | SR23–SR26; R5 | Two deterministic remote CLIs load native MCP with distinct project/run bindings and exchange messages | Other root/account/host, local daemon/credential leakage and missing remote CLI/provider state |
| V17 Task engine remote regression | SR27–SR31/SR33; R5 | Same Store handles claim/start/progress/submit/rework/accept and visible dependencies/leases | Claim race, stale attempt/version, cancel-submit race, unknown reassign and cycle all rejected |
| V18 Legacy migration/removal | SR08/SR36/SR37; R0/R7 | Open v6 history/preferences, retain unknown intents, export and deliberate cleanup/backup restore | Old credentials/profiles cannot enable a project; locked cleanup, partial migration and downgrade writer are safe |
| V19 GUI-driven remote task | SR18/SR24/SR28/SR32; R5 | Issuer assigns tiny edit/test, receiver runs remotely, submits, revises and receives acceptance through actual GUI | Verify remote process/root/files/evidence and unchanged local fixture, not just MCP handshake |
| V20 Actual vendor execution | SR23/SR26/SR40; R5/R7 | Known eligible installed vendor versions perform both task roles on release candidate | Approval/draft/background daemon/version matrix; unavailable credentials/budget explicitly unverified |
| V21 Static remote GUI | SR01/SR06/SR34/SR35; R1/R3/R6 | Review editable native fixtures for Connections/Projects/Explorer/Transfers, existing Tab/Pane status/actions and on-demand tasks/messages on both local OSes | Narrow/wide, light/dark, 125% text, multiple panes/tab, detached run, closed-panel updates, keyboard labels/focus; no duplicate Agent manager |
| V22 Integrated GUI actions | SR27/SR32/SR34/SR35; R6 | Live sidebar/task data shares scoped projections; explicit task→exact session and sidebar→task links; producing-location evidence Open | Rapid scope switch/removal, replaced/missing run link, closed task panel, late response and version conflict never redirect actions or steal focus |
| V23 Eight-hour end-to-end soak | SR15/SR19/SR22/SR33/SR36; R7 | Controlled SSH/helper/terminal/task/transfer reconnect/restart for eight hours with bounded reports | Queue/storage pressure, local sleep, remote service loss, preserved drafts/unknown outcomes and bounded memory |
| V24 Exact-source build/release | SR38–SR40; R7 | GitHub builds/checks local default/platform/apps and every claimed remote helper artifact from final SHA | Provenance/version/compatibility manifest and install/remove smoke; missing platform evidence excludes claim |

Each gate records source SHA, executable versions, local OS, remote OS/architecture, fixture versus physical status, test name, result and artifact. Use whitelisted metadata and owned screenshots; do not retain credentials, raw authentication logs or arbitrary transcripts. Code inspection can complete an inventory but cannot pass a runtime gate.

## Platform and runtime matrix

| Local GUI | Remote project | Required coverage | Current SSH Remote status |
| --- | --- | --- | --- |
| macOS | Linux | Companion target/build, actual controlled SSH/SFTP, remote PTY/files/Agents/tasks | Pending |
| macOS | macOS | Same-host isolated SSH fixture where available plus real remote confirmation | Pending |
| macOS | Windows | Windows OpenSSH/SFTP/ConPTY/path behavior through local GUI | Pending; no physical Windows SSH host supplied |
| Windows | Linux | Controlled remote SSH/SFTP/helper fixture and native Windows GUI | Pending |
| Windows | macOS | Controlled/physical remote endpoint plus Windows GUI paths/lifecycle | Pending |
| Windows | Windows | Windows-to-Windows SSH/SFTP/session/file fixture | Pending; physical acceptance unavailable |

Initial helper artifact candidates: Linux x86_64/aarch64, macOS arm64/x86_64 and Windows x64. Pin Linux libc/distribution/shell baselines before implementation. A candidate is not a support claim. Windows ARM64 and other targets need their own evidence before inclusion. Linux remote helper builds/fixtures are authorized; Linux desktop builds are excluded.

Local GitHub runners, containers and loopback SSH are controlled runtime evidence and must be labeled accordingly. Physical cross-device SSH results and real vendor model execution are separate rows. The user supplied no Windows physical host or model budget; do not invoke paid model turns or fabricate acceptance. This does not block controlled implementations/tests.

## Product-to-gate coverage

- SR01–SR08: V01–V07, V18, V21/V22.
- SR09–SR17: V05, V09–V12, V21–V23.
- SR18–SR22: V07/V13–V15, V19, V23.
- SR23–SR33: V07/V08/V15–V20/V22/V23.
- SR34–SR40: V01/V02/V18/V20–V24.
- SR41 (remote host status): V05/V21/V22/V23; HOST-STATUS.md; implementation pending.

R0 exit: V01/V02/V18. R1: V03/V04/V05. R2: V06/V07/V08/V17. R3: V09–V12. R4: V13–V15. R5: V16/V17/V19/V20. R6: V21/V22. R7: every gate applicable to a published capability. A blocked vendor/physical row limits the release claim rather than turning unfinished engineering into completion.

## Historical M0–M7 evidence — not current implementation instructions

The chronology below is intentionally retained, including original failures and later superseding results. Its unchecked states and IDs refer to the archived design/date, not the new R0–R7 plan. In particular, old enrolled-device tests do not validate remote process launch, SFTP transfer, SSH task execution or the new GUI. Preserve the source-specific boundaries recorded here.

# Per-package verification ledger

Date: 2026-10-02. This ledger separates executable backend checks from native UI,
vendor-model and physical-device acceptance. A passing subset does not complete
the associated PLAN package. All Rust compilation and tests run on GitHub.

Baseline repair: `bde9043`, run [36934356183](https://github.com/OthinusG/warp-lite/actions/runs/36934356183).
Both OS protocol suites passed; full application/package results were pending at
this checkpoint. Source `17da1f0`, run [36935931793](https://github.com/OthinusG/warp-lite/actions/runs/36935931793),
passed both OS protocol suites including control-order, 20-claim and single-receiver
pool wake checks. Source `e2c90a1`, run [36936534033](https://github.com/OthinusG/warp-lite/actions/runs/36936534033),
passed both OS protocol/scale suites and default/platform app checks. The new
menu test initially failed to compile because its App binding was immutable;
the one-line fixture repair passed the complete run for `c7033aa`,
[36939414552](https://github.com/OthinusG/warp-lite/actions/runs/36939414552),
including macOS/Windows packaging.
Native UI/device/model gates remain unchanged.

Source `bfbc585`, run [36941810908](https://github.com/OthinusG/warp-lite/actions/runs/36941810908),
passed both OS complete protocol/migration and representative-history suites,
including explicit-scope overlap warnings and the v3→v4 backup/private-row tests.
The debug native capture driver requires its corrected application build and
actual image review before the static UI checkpoint can pass.

| PLAN item | Smallest available check / evidence | Remaining verification or implementation |
| --- | --- | --- |
| M0.1 | Separate `e08b2ed` app provenance and signature; PROGRESS | Completed historical baseline only |
| M0.2 | `native_clients_complete_two_turns`, disposable repository and bounded owned-child cleanup | Harness covers Codex/QoderCN lifecycle only; prior native probes failed |
| M0.3 | `every_managed_type_delegates_reviews_and_recovers` exercises deterministic task/rework | Actual UI-driven edit/test and real model roles not verified |
| M0.4 | `every_receiver_state_preserves_work_and_initial_readiness_is_one_shot`, readiness suite | Actual UI approval/crash/restart matrix not verified |
| M0.5 | Native handshake records in PROGRESS/COVERAGE | No Windows test device or model-call budget supplied; role matrix not verified |
| M1.1 | Real stdio MCP discovery, 30 schema-derived tools | Existing twelve tool names retained; backend verified |
| M1.2 | Storage migration and indexed pagination tests | Backend verified |
| M1.3 | `attempts_versions_and_events_commit_with_transitions`, replay tests | Backend verified; UI event integration remains M2 |
| M1.4 | `migration_preserves_v1_data_with_backup_and_restore`, migration failures, downgrade sentinel, SQLite-full rollback | Backend verified; native old-binary release gate remains distinct |
| M1.5 | `request_replay_conflicts_and_epoch_expiry`, retention and quota tests | Backend verified |
| M2.1 | Native panel, persisted selection and fixed fixtures; both OS static checkpoint source `3d66017`, run `36970541930` | Static gate accepted; live integration tracked separately |
| M2.2 | 74 native captures per OS in run `36970541930`; all 72 combinations and final detail/draft/focus reviewed | Static visual gate accepted; Windows readback correction verified |
| M2.3 | Bounded live operator projection, 50-item pages, 200-event tail, scoped resume and generation fencing; both OS source `bd593d9`, run `36973482638` | Live native captures source `15460de` rendered 18 read states; complete action gate still pending |
| M2.4 | Native original-intent forms, typed overlap confirmation and terminal-focus dispatch implemented | Both OS source `15460de` compiled; action driver exposed cancellation/retry risk authorization gap. Source `14d5e45` corrects shared guard and real IPC expectation; cloud revalidation pending |
| M3.1 | Deadlines, exclusive cancellation and interrupted recovery tests | Explicit-clock deadline regression passed both OS in run 36950035983; native interrupt unsupported-state UI pending |
| M3.2 | Stale run/attempt and explicit operator recovery tests; native retry/reassign forms implemented | Replacement of unknown execution now requires its own operator override. Native and IPC regression revalidation pending |
| M3.3 | Dependency/cycle/foreign-edge backend checks verified; native assign/pool forms expose prerequisites and detail wait reason | Updated native scheduling capture pending |
| M3.4 | Pool eligibility, authenticated 20-way claims, deterministic single-receiver wake with stale-candidate/draft/failed-delivery/offline checks passed on both OSes | Native presence acceptance remains distinct |
| M3.5 | MCP cooperation instructions, native task control and scheduling forms implemented | Missing-acknowledgement delivery UI and full native operation matrix pending |
| M4.1 | Schema-v5 shared IPC verified; new-tab options pin reviewed workspace/space/root, default and restored panes remain private | Native shared-tab checkpoint implemented; cloud verification pending |
| M4.2 | Backend departure/remap fencing verified; create/map/scope preview/confirm new tab/remove participant controls and real-IPC native driver implemented | Cloud native preview/admission/departure assertions pending; package not accepted |
| M4.3 | Exact/subtree, symlink, path escape, TTL and attempt-owner tests; explicit-scope overlap/migration regressions in `bfbc585` passed both OS | Backend overlap warnings verified; device-qualified workspace routing and UI pending |
| M4.4 | MCP reserve/renew/release and abandoned-owner metadata | UI conflicts, renewal and release pending |
| M5.1 | Thread participants, literal search, reply/task/subject validation tests | Backend verified |
| M5.2 | Attempt-scoped evidence, file SHA-256/Git-object backend checks verified; native full evidence descriptors distinguish remote metadata | Native file-view integration remains pending |
| M5.3 | Archive/quota, ordered export, purge preservation and long-rework history tests | Backend verified; user-facing export/purge preview controls pending |
| M5.4 | C10 benchmark: 100k messages/10k tasks; p95 1.799 ms macOS / 2.863 ms Windows in run 36912281466 | Per-recipient backpressure verified; native history/UI throughput pending |
| M6.1 | System SSH command builder and offline `ssh -G` option/injection/path checks passed both OS in run 36951069744 | Real SSH authentication, changed hosts and remote stdio remain pending |
| M6.2 | Opt-in enrollment controller and fixed remote-stdio relay added; Both OS real IPC enrollment/authentication/heartbeat/revocation/restart and malformed frame checks passed in run 36962285181 | Application opt-in and remote task dispatch pending |
| M6.3 | Hash-only single-use enrollment, expiry, grants and revocation passed both OS in run 36958662888 | Both OS controller IPC passed in run 36962285181; secure participant storage pending |
| M6.4 | Both OS gateway auth/live heartbeat in run 36962285181 and owned-child participant authority/auth/grant fixture in run 36962746574; golden/truncation checks passed | Real SSH, durable pending intent, reconnect/reconciliation and event resume pending |
| M6.5 | Existing guarded local wake and single storage state machine | Remote coordinator routing and wake notifications pending |
| M6.6 | Existing local communication settings only | Devices/connections/mapping/grant UI pending |
| M6.7 | Local stale run, replay and crash fixtures only | Remote fault injection and reconciliation pending |
| M7.1 | Both-OS protocol CI; default/platform checks and macOS review packaging | New Windows installer/portable validation added; complete source run pending; UI snapshots pending |
| M7.2 | Native handshake records only | RC artifacts plus real installed-client task execution pending |
| M7.3 | Opt-in two-phase eight-hour backend runner added; stdlib orchestration self-check and dry-run passed | Actual eight-hour backend result, native UI/draft soak and real-model acceptance pending |
| M7.4 | PROGRESS, this ledger and MEMORY updated | Final shipped capability matrix, remote prerequisites and operating instructions await implementation |

The user reported no available Windows SSH test machine or model-call budget.
Do not invoke paid native model turns or claim C01/C13 acceptance. Missing runtime
access does not turn unimplemented M2/M4/M6 work into completed work.

The UI source checkpoint requires screenshot acceptance before live integration:
see [UI-CHECKPOINT.md](UI-CHECKPOINT.md). Do not enable real operations behind a
sample-data panel or silently mark failed/unsupported scenarios as passed.

Source `1267827`, run [36958662888](https://github.com/OthinusG/warp-lite/actions/runs/36958662888):
both OS complete backend suites passed, including real-IPC shared delegation and
rework, private active attempts, physical leases, original evidence checkout,
reclaim/departure/remap/dedup fences and v4→v5 backup migration. Enrollment tests
passed single-use/expiry/hash-only receipts/grants/revocation. M4 admission UI and
M6 gateway/platform credential storage/real SSH remain pending. MacOS debug capture passed; Windows failed in readback validation. COPY_SRC
correction is awaiting the next native capture run.

Source `c6f4d6c`, run [36983313934](https://github.com/OthinusG/warp-lite/actions/runs/36983313934): macOS passed all 108 native captures, including shared admission/departure, replacement-risk authorization, task filters, literal search/thread history and opening local evidence from the producing checkout. Visual review is pending. Windows source `8303fd8` passed backend/application compilation and 99 captures, then failed shared-tab admission. Extended-prefix startup directory handling is being corrected; no Windows live-action acceptance is claimed.

Native history/capacity/export-page/aged-archive/purge forms are implemented. Original preview sequence, typed deletion and stale-preview rejection have dedicated backend and retained GUI regressions (112 expected captures). Rust compilation and native visual/action acceptance are pending cloud verification. Local SQLite query-plan checks confirm indexed parent/reply lookups; workflow YAML and capture-diagnostic redaction checks passed.

Source `5914ada`, run [36985609759](https://github.com/OthinusG/warp-lite/actions/runs/36985609759), passed both OS backend/application builds and all 108 native captures/action assertions. Windows canonical-path startup fix is verified. macOS source `c6f4d6c` 34 live captures reviewed, including full thread/file-view images; Windows updated image review remains pending. Source `b3a8b57`, run [36987013225](https://github.com/OthinusG/warp-lite/actions/runs/36987013225), passed both OS protocol and representative-history steps with indexed retention queries; native history build/capture remains pending. Earlier `d6b382c` backend tests passed both OS; its slower pre-index-query benchmark run was cancelled after replacing the query. Native reservation renewal/typed release and original-tuple/unknown-attempt regressions now extend expected captures to 115; cloud validation pending.

History native source `b3a8b57` passed both OS backend/benchmark steps, then application compilation rejected a fixture-only access to `PanelTask.archived`, which is absent from the bounded summary. Source `eeb8852` instead verifies complete archived/attempt fields through history export; 115-capture revalidation runs in [36998184843](https://github.com/OthinusG/warp-lite/actions/runs/36998184843). macOS `c6f4d6c` thread capture shows missing text prefixes after file-view/tab transitions; visual acceptance remains pending diagnosis. Windows `5914ada` full thread image is readable. Canonical `2fa749c` full run completed macOS packaging but failed the known old Windows shared-tab admission; the already verified Windows startup fix must be included in the next full source run.

Gateway task dispatch source `7adfbb9`, run [36999119942](https://github.com/OthinusG/warp-lite/actions/runs/36999119942), passed both OS full protocol/representative-history checks, including real-IPC actor admission, duplicate original-epoch task intent, granted event pagination and revoked-device rejection before replay. Production participant routing, persistence and guarded remote wake are still pending. Windows `5914ada` 34 live PNGs reviewed; full thread image is readable. macOS `5914ada` cloud thread and isolated local review app (108 captures, exit zero) did not reproduce the earlier `c6f4d6c` prefix anomaly; retain that anomaly until the new 115-capture gate is reviewed.


Source `eeb8852`, run [36998184843](https://github.com/OthinusG/warp-lite/actions/runs/36998184843), passed both OS protocol/history checks, native application builds and all 115 native action captures. The seven new history/reservation states and thread images were reviewed on both platforms: typed confirmation, stale purge refusal, protected archival, reservation renewal/release and unsent drafts are readable. The earlier macOS thread-prefix anomaly did not recur in this gate; its original evidence remains recorded above. This does not establish physical Windows or real vendor acceptance.

Remote controller mapping source `dfa6e65`, run [36999798167](https://github.com/OthinusG/warp-lite/actions/runs/36999798167), failed compilation because the existing schema generator lacks UUID support. The fix describes these serialized UUID fields as strings without adding dependencies; mapping and participant-client revalidation are pending.


Source `338473e`, run [37001169565](https://github.com/OthinusG/warp-lite/actions/runs/37001169565), passed both target OS protocol/history checks. This includes schema-compatible reviewed mapping, owned stdio client identity/correlation checks, original receipt lookup over real controller IPC, closed-run reconciliation, changed-content rejection and revoked-device denial. Source `20c4338` exposed retry hints retained from unknown peer errors; the shared sanitizer now rejects those hints and versions while preserving metadata for whitelisted errors. Participant intent persistence source `b3c3b93` is under separate CI; production native routing and secure-storage/wake integration remain pending.


Participant intent source `b3c3b93`, run [37001599015](https://github.com/OthinusG/warp-lite/actions/runs/37001599015), and staged-client source `2973016`, run [37001938230](https://github.com/OthinusG/warp-lite/actions/runs/37001938230), passed both OS protocol/history checks. The checks recover unchanged pending/confirmed receipts after SQLite reopen, reject changed content/epochs/results, preserve unknown outcomes, enforce row/byte limits, page retained intents and reject another coordinator or request identity before stdio transmission. Native application routing is still unfinished.


## Exact-source validation checkpoints on 2026-10-02

- dc46ca6 / run 36998413298: both target OSes passed protocol/representative
  history, default and warp_platform application checks, review release packaging
  and all 115 native capture/action checks. This source excludes later remote
  safety changes and clipboard/participant observation acceptance extensions.
- Remote backend additions passed both OS suites in runs 37002464526 (presence),
  37002776680 (grant review), 37003138274 (unknown on loss), 37003690031
  (snapshot/cursors), 37004266066 (shutdown/storage failure) and 37004317107
  (cursor reopen). Native production routing remains unfinished.
- 5b988d3 and cec947a: both backend suites passed, including original-owner
  explicit unknown completion. Native compilation failed on the capture-only
  clipboard accessor; b1bf828 fixes it in run 37009716224 (pending).
- c3250f2 / run 37010078182: macOS backend/history passed with native
  connection replacement and delayed-old-write fencing; Windows still pending.
- Secure storage fail-closed application check added; cloud verification pending.
  No physical SSH, real vendor model, native credential or full M6 acceptance
  is asserted by these checkpoints.


- Native capture sources 2ebdfdf / run 37012541720 and 497de9e / run
  37013071649 passed both target OS suites, native builds and all 115 assertions.
  Both-platform 2ebdfdf screenshots reviewed for participant metadata, history
  capacity/export and thread readability; drafts remain intact.
- e4bc0e6 / run 37016574533 passed new delivery phase checks but failed a test
  expecting AgentAck to accept a task notification. Preserve the established
  invalid_state boundary and correct the assertion; exact-source rerun pending.
- macOS secure storage must distinguish errSecItemNotFound from access/locked
  errors. The shared provider now preserves other errors, with a focused status
  classification check. Native Keychain/DPAPI runtime acceptance remains pending.


R0 follow-up source `a9caea0`, [run 37040051387](https://github.com/OthinusG/warp-lite/actions/runs/37040051387):
macOS and Windows protocol/setup and representative-history checks passed,
including unchanged local active attempts, retained original receipts and legacy
profile selection. Both default application checks failed with E0616 because the settings view
accessed a private legacy profile field; platform and focused native cleanup
checks were skipped. A boolean Preferences accessor fixes the ownership boundary;
exact-source revalidation is pending. Source was verified to
match the workspace's 17 implementation paths without altering the current index.
SSH Remote and SR41 host-status collection/rendering remain unimplemented; their
contracts and static/live acceptance requirements are recorded in HOST-STATUS.md.


R0 repair source `d71d8b1`, [run 37041826672](https://github.com/OthinusG/warp-lite/actions/runs/37041826672):
macOS and Windows protocol/setup, representative history, default application and
warp_platform application checks passed. Both OS legacy metadata/credential
cleanup tests and macOS native configuration, wake and Keychain error-classifier
tests and both review-package builds passed. Real credential-provider and SSH
runtime acceptance remain pending. These results validate the
cutover implementation, not R1–R7 or SR41 runtime behavior. The workspace's 17
implementation paths match this source; its current branch/index was preserved.


## Managed read and SFTP source checkpoint — 2026-10-03

Source `fc8964c`, [run 37090383877](https://github.com/OthinusG/warp-lite/actions/runs/37090383877),
passed shared protobuf, native host metrics/root fencing, standalone process,
SSH argument/shell encoding and SFTP parser checks on Linux/macOS/Windows.
A controlled Linux loopback OpenSSH account exercised actual companion status
and independent file-only SFTP with no companion path. Unicode/spaces/shell
metacharacters preserved exact binary bytes; traversal/symlink escape and reads
after disconnect were refused. All three review companions include source/target
provenance, checksums and inherited notices. This is partial V02/V04/V05/V09
evidence, not completed GUI, installation, transfer or retained-session gates.

Source `6ab9021` adds persistent native account/root identity and a flushed
retired-endpoint refusal after a native-suite failure exposed Tokio stdio's no-op
shutdown. Its exact-source identity/flush checks are pending. New SR41 native
fixtures are implemented, but captures and visual review remain pending. V01–V24
stay open; no paid vendor/physical-host acceptance is inferred.


## Retained terminal backend checkpoint — 2026-10-03

Source `064f5cb85192b5aeaffb286a51acbb0db739b1d2`,
[run 37104521972](https://github.com/OthinusG/warp-lite/actions/runs/37104521972),
passed Linux/macOS/Windows shared wire, companion, native PTY, profile persistence,
retained terminal and SFTP checks. The controlled Linux SSH account also passed
same-boot/session/run reattachment, literal Unicode input, stale/replaced input
refusal, observed stop/exit/EOF/release and streamed SFTP disk-byte checks.
The terminal owner bounds sessions at 32, replay at 256 KiB, read pages at 32 KiB
and queued input at sixteen 4-KiB packets. Accepted input is queue admission,
not execution evidence. Duplicate launch reconciles the original run; changed
launch content conflicts. Disconnect revokes input while retaining the PTY.
These backend checks do not accept remote terminal GUI, vendor execution,
remote task authority, deployment or complete V13–V15.

Source `28a97cb841974d77a17bf93f458e8fd0764258a1` adds owned background-child
lifecycle checks on all three platforms, Unix unreaped process-group identity,
Windows job-aware console closure, and deferred task-panel workspace actions.
[Focused run 37105299664](https://github.com/OthinusG/warp-lite/actions/runs/37105299664)
and [desktop/capture run 37105299449](https://github.com/OthinusG/warp-lite/actions/runs/37105299449)
are pending. Windows capture source `d1022f8` stopped at shared-tab confirmation
with a circular view update; existing framework deferred dispatch now addresses
that production path and sibling file/focus actions. Visual gates remain open.


R0.3 contract/inventory accepted: API sections 2/3, shared ManagedFence/terminal
schema and IMPLEMENTATION's remote/local call-site table cover identity and
storage ownership before new remote domain storage. Source064f5cb/run37104521972
supplies native-root, persistent account/service and replaced-attachment checks
on all three remote platforms. Live GUI environment mapping remains pending.


R4.3 accepted by source `3a37ed2`,
[run 37105738779](https://github.com/OthinusG/warp-lite/actions/runs/37105738779):
all Linux/macOS/Windows focused checks and actual controlled Linux SSH/SFTP
passed. Actual background children remain alive after observing leader exit,
then stop under the original owned group/job and reach observed exit/output EOF.
The native fixture inspects OS process state rather than equating a broken pipe
with process death. This closes the source28a97cb native regressions; production
GUI/capture verification remains separate.


## Remote task Store/API backend checkpoint — 2026-10-03

Source `550753b33a090b0d0531395f9ced8c8beeebb2fb`,
[run 37107398041](https://github.com/OthinusG/warp-lite/actions/runs/37107398041),
passed Linux/macOS/Windows complete focused suites and controlled Linux SSH/SFTP.
New task checks cover real private IPC actor registration, persisted pool task/
events, shared project intent replay, changed-content conflict, different and
replaced project isolation, no file creation on forbidden authority, stale/root
replacement fencing, sanitized errors and actual SSH scoped panel reads. This
is partial V08/V17 evidence. Local projection/GUI intent persistence, real remote
MCP process launch and GUI/vendor task workflows remain unverified.

### R0.4 — native enrollment worker removal

Accepted independently from SSH GUI integration. Current worker/settings/setup
and legacy-key cleanup sources are byte-identical to `d71d8b1`, verified by
[run 37041826672](https://github.com/OthinusG/warp-lite/actions/runs/37041826672)
on macOS/Windows default/platform builds and focused native cleanup/configuration
checks. Source inventory confirms no enrollment worker, invitation polling or
enrollment-only busy gate remains; v6 migration retains unknown work/history.
This does not accept all R0 removals or managed SSH cancellation behavior.

### Owned process activity and concurrent attachment acceptance — source 65e1b86

[Run 37110078158](https://github.com/OthinusG/warp-lite/actions/runs/37110078158)
passed Linux/macOS/Windows protocol/native/service tests and controlled Linux
SSH/SFTP. Source `65e1b8671ec860007a5c76d868d442c46c76cec4` proves leader exit and
PTY EOF cannot authorize release while owned background processes survive;
disconnect/reattach preserves that run, explicit Stop observes inactive ownership
and Release follows EOF. Failed/unreadable native activity retains ownership.
Concurrent attachment identity access waits within a five-second bound; eight
simultaneous opens share one persistent identity. The three-platform concurrent
retained fixtures exercise the actual service startup path.

Accepted as the lifecycle/backend follow-up only. R2.8 attempt recovery, R2.6
per-launch MCP, sidebar/live GUI and full fault acceptance remain pending. Later
MCP edits are not covered by this source gate.
