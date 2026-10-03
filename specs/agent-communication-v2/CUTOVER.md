# SSH Remote Source Cutover and Removal Inventory

Date: 2026-10-03. Status: R0 source cutover checkpoint implemented; exact-source checks pending. Source inspected on the agent-communication branch after b529062. The native enrollment worker is removed and the device runtime is excluded from production; remaining dispositions below are not all complete. See IMPLEMENTATION.md and ACCEPTANCE.md. Use [PLAN.md](PLAN.md) R0/R7 for execution and [TECH.md](TECH.md) for the replacement architecture.

## Disposition definitions

- **Keep:** implementation/invariant directly serves local or remote project management.
- **Adapt:** retain audited primitives, change the authority/execution location or lifecycle before use.
- **Retire active behavior:** remove call paths, exported operations, workers and obsolete product UI once compatibility gates pass.
- **Preserve legacy data:** no new writes/authorization through obsolete roles; keep historical/unknown records readable/exportable. This does not mean keep obsolete runtime features forever.
- **Defer:** do not extend a feature beyond the requested same-project remote scope. Preserve already-working local behavior unless deletion has a concrete benefit and safe migration.

Code being implemented does not justify keeping the wrong product. Passing old tests proves an invariant under old authority, not that device federation belongs in SSH Remote. Conversely, deleting security/replay/unknown-execution protections to reduce code is not acceptable.

## D01 — Device enrollment and invitation flows: retire

**Actual source:** `crates/agent_bus/src/remote_auth.rs`; `ControllerOperation::InvitationCreate`, `DeviceList`, `DeviceRevoke`, `DeviceGrantUpdate` in `lib.rs`; corresponding dispatch/tests in storage/control modules.

**Why redundant:** A local user already authenticates to a remote account through SSH. The product does not pair independent native Warpai applications or grant another enrolled device a federation role.

**Remove:** invitation creation/consumption, durable device bearer credential issuance, device-generation grant editing and product Devices/invite/revoke flows. New connections identify SSH environments/projects, not enrolled application devices.

**Keep:** OS randomness for per-run capabilities, private endpoint permissions, constant-time verification where an actual capability verifier requires it, authorization-before-replay and scoped process identity. Do not remove an installed crypto dependency until every remaining caller is checked.

**Data/test gate:** Device/invitation/grant rows become non-operational, old invitation replies cannot authenticate, and old task/evidence references still display. Replace old invitation/grant acceptance tests with SSH account/project admission and stale attachment tests; retain migration fixtures for old rows.

## D02 — Gateway requiring a remote native app: retire and replace

**Actual source:** `crates/agent_bus/src/remote_control.rs` (`RunningController`, `AuthenticationFrame`, descriptor discovery/relay); `remote-stdio` command dispatch in `main.rs`; controller state in `transport.rs`.

**Why redundant:** The current gateway forwards frames to a running host Warpai native app and reports coordinator_unavailable when that app is absent. New remote project execution must work without a remote desktop GUI.

**Remove:** production relay-to-another-app entrypoint, enrolled-device Authenticate/Enroll dispatch, invitation controller discovery and planned remote-host opt-in settings. Do not rename the old gateway and claim it is a process/PTY/file service.

**Adapt:** codec limits, owned-child cleanup, safe diagnostics, nonce-owned socket cleanup, current-user Windows pipe DACL and connection replacement fencing. The new repository companion owns a project service/remote Store instead of forwarding to a remote GUI.

**Gate:** ordinary remote SSH/SFTP remains usable; new companion starts without the remote native app; old entrypoint fails safely or is removed from CLI/schema/docs. Compatibility fixtures cover older invocation without allowing an old frame to mutate new state.

## D03 — Native app enrollment worker: remove

**Actual source:** `app/src/agent_communication/remote_settings.rs`; `remote_pending`, `remote_cancel`, `remote_status`, initialization/polling and configure guard in `agent_communication.rs`; `remote_enrolling()`-based settings disables in `settings_view/agent_communication.rs`.

**Why redundant:** This worker consumes one-time device invitations and waits for expected coordinator UUIDs. The desired UI selects/authenticates SSH projects.

**Remove:** unused enrollment APIs/results/channel/watch fields, periodic enrollment poll and UI busy gates that exist solely for this worker. Do not leave unreachable scaffolding after the replacement starts working.

**Keep/adapt:** an owned cancellable async worker pattern is useful for SSH authentication/deployment. Reuse existing runtime primitives and notify/generation fences, not the credential-bearing Enrollment type. Preserve communication disablement semantics and local vendor setup gating.

**Gate:** disabling communication remains immediate; cancelled SSH work cannot later enable a project; local setup cleanup still works. Do not introduce a shared blocking worker that freezes terminal input.

## D04 — Enrollment profile preferences: deprecate, migrate metadata deliberately

**Actual source:** `setup::RemoteProfile` and `Preferences.remote_profiles` in `app/src/agent_communication/setup.rs`; profile serialization/validation test in remote_settings.rs.

**Remove:** new profile writes with coordinator/device UUID, generation and space grant fields; the old schema is not a SSH project profile.

**Keep:** alias validation and secret-free preference serialization, backward-compatible defaults and atomic preference replacement. Setup.rs is included independently by bridge tests, so new non-UI preference types must stay UI-independent.

**Migration:** preserve old profile metadata in a deprecated/read-only namespace for export/removal. A saved alias may be offered as an untrusted connection draft, never as an already verified account/root. Require a fresh SSH trust/project selection. Display cleanup status when old secure keys cannot be removed.

**Gate:** old preferences load, active local CLI selection survives, removed fields do not expose secrets or resurrect device authority, and no automatic network operation occurs during migration.

## D05 — Device credential helper and keys: remove feature, keep common fix

**Actual source:** `app/src/agent_communication/remote_credentials.rs`; shared `crates/warpui_extras/src/secure_storage/mac.rs` and Windows provider.

**Remove:** `agent-remote-{coordinator}-{device}` enrollment credential load/save paths and related production module wiring after ownership-aware cleanup/migration. No replacement device bearer secret is necessary for SSH authentication.

**Keep:** macOS mapping only errSecItemNotFound to absence; access/locked failures must not count as absent. Keep secure storage trait/providers and genuinely reused secret-handling checks. SSH private keys remain owned by system SSH, not copied into this provider.

**Cleanup:** remove only known keys derived from legacy saved UUID pairs, never enumerate/read arbitrary user credentials. If secure storage is locked, leave credentials inactive and report cleanup_pending. Never print/store secret values or treat failure as successful deletion.

**Gate:** old keys cannot authorize active product access, another application's/device's key remains untouched and the generic error classification regression remains in CI.

## D06 — Device grants and cross-machine actor authority: retire

**Actual source:** `remote_actors.rs`, remote grant authorization in `remote_auth.rs`, `ControllerOperation::RemoteWorkspaceMap`, `RemotePrincipal`, `RemoteActor`, actor admission/heartbeat dispatch in `remote_control.rs`.

**Remove:** participant-app device-qualified mapping, remote machine name collision/reclaim rules based on a device bearer, and coordinator grants for unrelated machines. No Devices/space permission editor is planned.

**Adapt:** bind remote actors directly to the remote project's own managed runs via its private MCP endpoint. Preserve current actor/run/revision checks; provenance still identifies the producing remote environment.

**Gate:** two remote processes in one project register separately; another root/account cannot impersonate them; old device frames cannot adopt new project actors. New GUI operator identity remains distinct from Agent identity.

## D07 — Remote participant private spool: adapt invariant, retire old context

**Actual source:** `crates/agent_bus/src/remote_pending.rs`, `RemoteIntent` fields coordinator/device/space/actor/epoch/request_id, `Connection::execute_intent` and reconciliation functions in `remote.rs`.

**Why old layout is wrong:** The former local participant app staged writes to another native coordinator. New remote Agents execute against their nearby remote Store; the local GUI stages only its own submitted operator/session/file intents.

**Remove:** coupling pending requests to device credentials/grants and participant-local reservations for another authority. Do not automatically replay old spool rows into the new service.

**Keep/adapt:** bounded immutable original request content, original epoch, retained committed result, no unresolved eviction, exact receipt reconciliation and connection replacement fence. Reuse the existing storage abstraction where schema/authority permit; do not build a second queue service.

**Gate:** new GUI spool uses explicit environment/project/service identity, and old unknown rows remain read-only/exportable. Lost command/process/file operations get their own receipt semantics; an Agent task receipt does not prove a shell command/file replacement happened.

## D08 — Schema-v6 remote tables: retire writes, preserve data first

**Actual tables in storage.rs:** `devices`, `device_spaces`, `invitations`, `cursors`, `remote_workspaces`, `remote_actor_bindings`, `remote_runs`, `remote_pending_intents`. `spaces.device` and `evidence.device` also need dependency inspection.

**Do not do:** decrement schema version, delete v6 migration/backup fixtures, truncate unknown intents, rewrite old remote agents as new local ones, drop provenance columns without replacing reads, or set old attempts to cancelled/success to simplify cleanup.

**Sequence:** inventory row/reference usage → backup consistent database and verify reopen → fence all legacy runtime writes → preserve read-only receipt/history/provenance → add new environment/project metadata only when needed → optional later table compaction/export with explicit deletion preview. A code removal does not require immediate physical table drop.

**Gate:** existing local tasks/messages/attempts/evidence, archive/search/export and request dedup survive. Interrupted old attempts stay unknown. An old binary cannot become a writable fallback for the new schema; restore is explicit and version-matched.

## D09 — SSH Connection implementation: reuse selected primitives, replace roles

**Actual source:** `crates/agent_bus/src/remote.rs` system SSH command builder, owned stdio connection, negotiation/error sanitizer, device authentication, remote task dispatch/snapshot/cursors.

**Keep:** safe alias/argv construction, no originating capability forwarding, owned child lifetime, bounded IO/frame negotiation, diagnostic whitelist, authorization before replay and stale connection fencing.

**Replace:** fixed gateway command expecting another app, expected-coordinator Hello, enroll/auth/device grants and remote participant actor announcement. Managed project channels authenticate with SSH/service attach and expose project/session/file capabilities.

**Integration:** prefer the existing remote_server client/transport as the single managed-control substrate. Extract/reuse a primitive only when a real new caller needs it. Do not retain both full old and new connection stacks indefinitely.

**Gate:** interactive authentication is not broken by blindly carrying BatchMode into initial login; reviewed ProxyJump is not blocked by old relay flags; machine-channel stdout stays clean; Windows does not inherit Unix-only ControlMaster assumptions.

## D10 — Upstream remote server installer: replace cloud coupling

**Actual source:** `crates/remote_server/src/setup.rs`, `install_remote_server.sh`, `app/src/remote_server/ssh_transport.rs`.

**Observed issue:** binary naming/path follows upstream Oz channels, installation derives a cloud download URL, and RemoteOs currently covers Linux/MacOs with uname detection. This is client-side source, not proof of an independently distributable server implementation.

**Remove/replace in the new managed path:** Oz/cloud download/channel requirement, implicit deployment and Windows-incompatible detection. Use repository-owned versioned/checksummed helper artifacts and explicit install destination/manual path.

**Keep:** remote manager/setup-state/client/protocol design where verified, timeouts and clean stdio; preserve unrelated ordinary SSH behavior.

**Gate:** Linux/macOS/Windows remote companion launches from built source; Windows detection does not require uname; there is no cloud account/install dependency. Old installer reachability is audited and fenced rather than accidentally revived.

Source checkpoint: the inherited automatic installer now returns a fixed disabled
error before network/file work. The Oz/cloud URL/template helpers, unused script
SSH helper and obsolete local Oz cross-deploy script are removed. Ordinary SSH
and manually provisioned legacy server checks/proxy behavior remain. Independent
read-companion review artifacts are built in GitHub; explicit deployment and full
managed service compatibility remain pending. No new companion is substituted
for the incompatible legacy proxy command.

## D11 — FTP/FTPS proposal: exclude entirely

The user confirmed SFTP-only on 2026-10-03. No FTP/FTPS implementation was found in this feature work; this is a planning removal, not a claim that an FTP library was deleted.

**Do not add:** FTP connection profiles, password persistence, passive-mode/TLS/data-channel support, insecure transport warning workflow, FTP fallback or FTP-specific CI. SFTP shares SSH verification/account identity and supports helper-independent file access.

**Gate:** active specifications, dependency choices and GUI copy say SSH/SFTP. Historical context may mention the rejected alternative only as excluded scope.

## D12 — Local task/state/history engine: keep

**Actual source:** `lib.rs`, `storage.rs`, `mcp.rs`; task/attempt/version/idempotency/dependency/thread/evidence/reservation/history logic and real-IPC tests.

**Why needed:** The requested GUI includes task management and remote Agent communication. These transitions are reusable remotely and do not depend on device federation as a product.

**Keep:** task lifecycle/rework/review, atomic claim, dependency checks, uncertain-execution fencing, optimistic forms, audit/delivery facts, quotas, migration backups and exact request replay.

**Adapt only:** producing environment/project identity and authorized operator/remote-run admission. Do not fork a new task state machine or delete safety features just because the remote architecture changed.

**Gate:** original local scenarios pass unchanged; remote scenarios call the same engine and preserve failure/unknown outcomes.

## D13 — Local collaboration spaces/worktrees: preserve, stop expansion

**Actual source:** local SpaceList/Create/Join/Leave/WorkspaceMap, workspace bindings, shared-pane admission in app/terminal paths, `LOCAL-SPACES.md` and local_spaces tests.

**Disposition:** preserve implemented local behavior and durable data. Same managed remote root forms a private project scope by default. Do not require users to create/grant a cross-machine space to run two agents on that host.

**Defer:** cross-host/shared-space federation, automatic Git-remote grouping, remote-to-local task routing and multi-machine repository alignment. Existing advanced local space controls can remain under local collaboration; they are not the SSH connection wizard.

**Gate:** same remote project communicates across tabs, different roots remain isolated, normal/restored local panes stay private and local same-checkout leases still conflict.

## D14 — Sidebar/session ownership and collaboration panel: consolidate

**Actual source:** `app/src/workspace/view/vertical_tabs.rs`, `panel.rs`, `panel_controls.rs`, `settings_view/agent_communication.rs`, workspace Tools Panel integration.

**Keep:** task detail/timeline, filters, literal thread search, original-intent forms, typed risk confirmation, local terminal focus, history export/purge and existing themes/accessibility/draft protections.

**Adapt:** sidebar Tab/Pane rows own Agent/session status, remote identity and focus/rename/reconnect/stop/detach. Retained detached sessions use the same hierarchy, not a second list. Reuse collaboration panel for compact task/message/history detail on demand, with advanced controls expanded only when needed. Preserve persisted panel selection; fresh profiles default closed. Add explicit exact-run task/session cross-links and shared status/task projections; Connections/Projects and Transfers remain setup/file surfaces.

**Remove:** obsolete planned Devices/invitation/grants UX, coordinator-device form fields and enrollment-only busy gates. Also retire the collaboration panel's independent Agent roster/navigation and duplicate session focus/rename/reconnect/stop/close controls after sidebar parity is proven. Do not introduce a second Agent dashboard or mandatory task board. Preserve AgentList/SessionList data APIs, participant labels, assignee pickers, historical attribution and task→session navigation; GUI removal is not actor/history deletion.

**Gate:** static new states pass native screenshot/keyboard/focus review before live integration. Cover multiple Agent panes per tab, detached retained runs, exact cross-links, shared badges/details and closed-task-panel background updates. Remove obsolete Agent-section fixture expectations only after new sidebar/task tests replace their useful checks; historical screenshots remain evidence. Existing 115 captures do not automatically cover the new surfaces.

## D15 — Native readiness, launch adapters and evidence: keep safeguards, adapt location

**Actual source:** `readiness.rs`, `session.rs`, `launch.rs`, native CLI session code, `use_agent_footer/mod.rs`, evidence opening/verification and local delivery observations.

**Keep:** per-run capability, shared-daemon isolation, Qoder/QoderCN separation, permission/draft/manual-pause gates, delayed Enter recheck and no acknowledgement/task-start inference.

**Adapt:** remote executable/version detection, remote private endpoint and lifecycle signals; local view and remote run both guard automatic delivery. Keep ordinary remote-session rejection until a verified managed route exists.

**Remove/defer:** assumptions that a remotely paired native app will relay the wake, host-only unavailable evidence as the permanent design, and any proposal to infer readiness solely from remote heartbeat. New explicit evidence fetching uses the producing SSH project.

**Gate:** remote draft/approval/busy/replaced-run negative cases fail closed; unsupported adapters remain manual/unverified. Open evidence never substitutes local same-named files.

## D16 — Tests, workflows and documentation: remap, then prune

**Actual source:** `remote_tests.rs`, remote module inline tests, `tests/local_spaces.rs`, native capture driver, validation/build scripts and old M0–M7/C01–C14 documents.

**Keep:** generic framing/injection/replay/unknown-attempt/storage/pipe permissions/owned-child/host-key safety fixtures; default/platform checks; native panel captures; provenance and migration tests.

**Replace:** old device enrollment/grant/coordinator matrix as current product acceptance with V01–V24 controlled SSH/SFTP/remote-run/GUI checks. Retain legacy-only fixture tests until their schema migration gate passes. Do not delete tests merely because they expose a new regression.

**Build split:** desktop macOS/Windows only; remote companion Linux/macOS/Windows. Existing Linux desktop exclusions remain; new Linux helper fixtures/builds are authorized and separately named.

**Docs:** old design is archived, current PLAN/PRODUCT/TECH/API govern, ACCEPTANCE marks history, PROGRESS stays chronological, README describes proposed versus shipped, and MEMORY/AGENTS record scope. No old eight-hour backend soak is relabeled SSH/session/transfer acceptance.

## Ordered removal commits and review gates

1. **Fence and inventory:** record legacy schema/profile/key ownership, entrypoint reachability, source baseline and local regression evidence. Add compatibility reading/export without dropping records.
2. **App-only enrollment removal:** remove D03 worker/polling/unused calls and mark D04 profiles inactive; keep shared secure-storage fix and local setup behavior.
3. **Replacement foundation:** implement verified SSH environment/project/companion and receipt/ownership contracts; prove no remote GUI dependency.
4. **Retire old active APIs:** remove/fail-closed D01/D02/D06 runtime roles and device-controller tools, adjust all callers and generated schemas. Keep legacy history reads.
5. **Simplify connection/storage:** consolidate D07/D09 under the new actual control stack; no blind replay or dual authoritative task copies. Table physical cleanup is optional later, separate from code deletion.
6. **Deployment and test cleanup:** replace D10 cloud installer, remove obsolete feature tests only after invariant/migration replacements, update workflows/docs and validate every supported target.

Each commit must be independently reviewable and preserve default/warp_platform/local task regressions. Do not mix a broad terminal rewrite or data purge into cleanup. Do not delete more than this task's audited obsolete feature paths.

## Removal completion checks

- No live code path accepts old enrollment/device/grant frames or uses legacy device credentials as new authorization.
- No stale old profile auto-connects/enables a project. Cleanup failures remain visible and inactive.
- Old unknown intents/attempts cannot be executed, claimed successful or silently erased.
- Local tasks, history, spaces, terminal core, native vendor setup and shared secure storage remain functional.
- New project/helper works without a remote Warpai desktop or cloud CLI.
- Active GUI/docs/dependencies contain SSH/SFTP only and no Linux desktop expansion.
- Every retained invariant has a corresponding current or legacy migration test, with exact source evidence.
