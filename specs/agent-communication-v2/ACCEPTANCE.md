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
