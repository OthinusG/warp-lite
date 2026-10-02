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
the one-line fixture repair awaits its source run. Packaging was still running.
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
| M2.1 | Native panel source, persisted selection, nine fixed fixtures | Fixture app test and rendered checkpoint required |
| M2.2 | Signed `75f47f9` debug review app attempted locally | Startup failed on missing ReferAFriend binding; fix pending build. Accessibility unavailable; both-OS screenshots pending |
| M2.3 | Paginated operator task/event APIs exist | Live panel subscriptions, cursor recovery and detail integration not implemented |
| M2.4 | Distinct operator mutations share storage transitions | Live panel controls and terminal focus not implemented |
| M3.1 | Deadlines, exclusive cancellation and interrupted recovery tests | Native interrupt unsupported/unsupported-state UI and clock-jump acceptance pending |
| M3.2 | Stale run/attempt and explicit operator recovery tests | Retry/reassignment backend verified; panel pending |
| M3.3 | Dependency acceptance and cycle/foreign-edge rejection tests | Backend verified; failed prerequisite panel pending |
| M3.4 | Pool eligibility, authenticated 20-way claims, deterministic single-receiver wake with stale-candidate/draft/failed-delivery/offline checks passed on both OSes | Native presence acceptance remains distinct |
| M3.5 | MCP cooperation instructions and readiness/replay checks | Missing-acknowledgement panel and task controls pending |
| M4.1 | Space/workspace metadata controller tests | Functional shared-space routing and checkout-qualified agents not implemented |
| M4.2 | Controller join/leave metadata operations | Scope preview, new-session routing and UI not implemented |
| M4.3 | Exact/subtree, symlink, path escape, TTL and attempt-owner tests; explicit-scope overlap/migration regressions in `bfbc585` passed both OS | Backend overlap warnings verified; device-qualified workspace routing and UI pending |
| M4.4 | MCP reserve/renew/release and abandoned-owner metadata | UI conflicts, renewal and release pending |
| M5.1 | Thread participants, literal search, reply/task/subject validation tests | Backend verified |
| M5.2 | Attempt-scoped evidence, opened-file SHA-256 and local Git-object tests | Backend verified; file-view integration and remote-unavailable labels pending |
| M5.3 | Archive/quota, ordered export, purge preservation and long-rework history tests | Backend verified; user-facing export/purge preview controls pending |
| M5.4 | C10 benchmark: 100k messages/10k tasks; p95 1.799 ms macOS / 2.863 ms Windows in run 36912281466 | Per-recipient backpressure verified; native history/UI throughput pending |
| M6.1 | No remote transport implementation | System SSH paths, authentication and known-host checks pending |
| M6.2 | No remote gateway/control endpoint | Pending |
| M6.3 | Device APIs explicitly return feature_unavailable | Enrollment, secure credentials, grants and revocation pending |
| M6.4 | Local protocol major/features negotiation exists | Remote hello, device epochs, heartbeat, spool and resume pending |
| M6.5 | Existing guarded local wake and single storage state machine | Remote coordinator routing and wake notifications pending |
| M6.6 | Existing local communication settings only | Devices/connections/mapping/grant UI pending |
| M6.7 | Local stale run, replay and crash fixtures only | Remote fault injection and reconciliation pending |
| M7.1 | Both-OS protocol CI; default/platform checks and macOS review packaging | New Windows installer/portable validation added; complete source run pending; UI snapshots pending |
| M7.2 | Native handshake records only | RC artifacts plus real installed-client task execution pending |
| M7.3 | No eight-hour soak result | Eight-hour deterministic restart/history soak and real-model acceptance pending |
| M7.4 | PROGRESS, this ledger and MEMORY updated | Final shipped capability matrix, remote prerequisites and operating instructions await implementation |

The user reported no available Windows SSH test machine or model-call budget.
Do not invoke paid native model turns or claim C01/C13 acceptance. Missing runtime
access does not turn unimplemented M2/M4/M6 work into completed work.

The UI source checkpoint requires screenshot acceptance before live integration:
see [UI-CHECKPOINT.md](UI-CHECKPOINT.md). Do not enable real operations behind a
sample-data panel or silently mark failed/unsupported scenarios as passed.
