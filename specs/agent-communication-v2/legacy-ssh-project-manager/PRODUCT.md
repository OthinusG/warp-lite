# Warpai SSH Remote Project Manager

Date: 2026-10-03. Status: revised target behavior, not a shipped capability. The user confirmed SSH with SFTP only and Linux/macOS/Windows remote environments. Local desktop targets remain macOS/Windows. This replaces the former enrolled-device collaboration product; historical behavior B01–B30 is archived in [legacy-machine-collaboration/PRODUCT.md](legacy-machine-collaboration/PRODUCT.md).

## Summary

Use one local Warpai GUI to connect to remote projects, manage remote files and transfers, open remote terminals, launch remotely installed CLI agents, and inspect/control their sessions and collaboration tasks. Commands and file operations run on the selected remote machine. Agents in the same remote project communicate without requiring a remote Warpai desktop application.

## Design reference

Figma: none provided. The previously confirmed visual authority is the existing Warpai interface: Tools Panel, Project Explorer, terminal tabs/splits, settings, editors, themes, buttons and keyboard conventions. Existing left Tab/Pane sidebar owns Agent/session management; new Connections/Projects and Transfers surfaces follow that authority. The collaboration panel supplies task/message/history detail on demand, not a second Agent manager. Existing collaboration screenshots establish reusable styling only; new remote surfaces need their own static review before live integration.

## Scope

SSH provides project access and execution; SFTP provides file transfer. Linux, macOS and Windows are remote environments; local Warpai runs on macOS/Windows. Remote vendor CLIs and their provider authentication remain user-managed. FTP/FTPS, cross-host task federation, device invitations, cloud accounts, automatic source synchronization, public server hosting and Linux desktop UI are outside this delivery.

## Behavior

### Connections and project selection

1. **SR01 — One local control surface.** The GUI shows saved connections and remote projects alongside local work. A user can connect, select a remote directory, open terminals and files, launch agents and inspect tasks without opening a second desktop app on the remote machine. Ordinary manually typed SSH commands continue to work, but do not silently become managed projects.

2. **SR02 — Connection profiles.** A profile records a display name and SSH alias or host/user/port, with optional SSH configuration/identity-file references and default project directory. Invalid inputs produce field-specific errors. Passwords, passphrases, private-key contents and provider credentials are not displayed as profile data or exported with profiles. Renaming a profile does not change project identity.

3. **SR03 — SSH authentication and host trust.** Existing system SSH configuration and user authentication methods remain available. First-time trust exposes the target and fingerprint for deliberate review. A changed host key blocks access until the user resolves it; reconnect never silently replaces trust or disables verification. Authentication prompts remain visible and separate from remote terminal/application protocol content. Cancelling authentication leaves no enabled session.

4. **SR04 — Capability-specific connection state.** Show connecting, authentication required, host verification required, connected, reconnecting, disconnected and failed with an actionable reason. Terminal, SFTP, project helper and Agent/MCP availability are separate facts. A shell can work while SFTP is unavailable; a SFTP-only connection can work while helper deployment is unavailable. Do not label a host fully managed merely because TCP/SSH connected.

5. **SR05 — Explicit remote project.** Opening a project selects a directory on the authenticated remote machine and account. Same-name paths, matching Git remotes and matching SSH host aliases do not implicitly merge projects. Managed terminals opened in the same verified project share its Agent/task scope; different roots, accounts and hosts remain isolated. Selecting a root does not move existing local or other-project tasks.

6. **SR06 — Visible location.** Explorer, terminal tabs, Agent rows, task details and destructive actions identify their remote connection/project. Users can distinguish a local file from a remotely opened same-named file. No remote command, read, save, test or Git action falls back to the local machine on failure.

7. **SR07 — Remote prerequisites.** Missing SSH/SFTP, unsupported OS/architecture, unavailable remote shell, permission denial and incompatible helper versions expose specific remediation. User-visible helper installation explains destination/version/ownership before deployment. Warpai does not silently install vendor agents, enable SSH servers, change firewall rules, request root privileges or download an upstream cloud CLI.

8. **SR08 — Restore and removal.** Restart may restore saved profiles and layouts but does not replay old commands, reuse old live capabilities or claim that a remote process is still running. Removing a connection prevents new automatic activity and stale callbacks; it preserves task/history data and reports any unresolved remote sessions or transfers. Removing a profile is distinct from deleting remote files or stopping remote processes.

### Remote files and SFTP

9. **SR09 — File browsing.** The remote Explorer lists directories/files with type, name and useful size/time metadata. It supports navigation, sorting, explicit refresh, hidden-file display and bounded large-directory loading. Loading, empty, unreadable, stale and disconnected states are distinct. Symlinks are identified rather than silently treated as ordinary folders.

10. **SR10 — File-only access.** Users can browse and transfer with SFTP even without the managed-project helper or Agent support. A file-only root remains explicit. No FTP/FTPS profile, password workflow or insecure transfer fallback is offered. SFTP reuses verified SSH connection identity and applies host trust rules.

11. **SR11 — Remote open and editing.** Opening a text file shows remote origin, loading and errors. Supported text can be edited through existing editor/viewer affordances; binary, unsupported encoding, read-only and large-file states are explicit. Merely opening a file does not execute it, mark its evidence verified or copy an entire project locally.

12. **SR12 — Concurrent save conflict.** Saving compares the file state originally opened with the current remote state. If an Agent/editor changed it, show reload, compare and deliberate overwrite rather than silently losing those changes. A missing destination, changed symlink or lost connection is not a successful save. An unknown save result remains unknown until checked; atomic replacement is advertised only when supported.

13. **SR13 — File operations.** Create file/folder, rename, move and delete operate on selected remote paths. Show exact affected locations before destructive operations and distinguish trash availability from permanent deletion. Avoid overwriting existing names silently. Operation success appears only after remote confirmation; partial directory operations list failed items.

14. **SR14 — Upload/download.** Users select local sources or remote sources and explicit destinations, including drag/drop with a destination preview. File and directory batches support binary content. Show queue/progress/completed/failed/cancelled states, transferred bytes and individual errors. A file selected for upload is not an authorization to upload unrelated configuration, credentials or project content.

15. **SR15 — Transfer interruption and resume.** Users can cancel, retry and pause/resume only where the backend supports it; unsupported pause is clearly disabled. A partial file is not displayed as complete. Resume checks that source/destination still correspond to the original transfer. If safe resume cannot be proven, offer restart without appending to an unrelated file. Cancelling removes only owned temporary files, with cleanup failures visible.

16. **SR16 — Overwrite and path containment.** Remote project/file actions remain within the selected root; paths escaping through traversal, symlinks, drives or account remapping are rejected. Upload/download overwrite conflicts require a deliberate choice. Same-named roots on separate machines are distinct. Different remote filesystem case semantics are respected.

17. **SR17 — Bounded transfer history.** A Transfers view records endpoint/path/progress/result metadata with explicit cleanup. It does not persist credentials or file contents as diagnostics. Clearing history does not delete files; retrying does not silently overwrite a changed destination. Large transfer activity does not freeze terminals or task controls.

### Remote terminals and unified session control

18. **SR18 — Remote execution location.** Open Terminal starts a shell in the chosen remote project. Agent commands, tests and Git work use the remote filesystem and runtime. Environment discovery and executable availability come from that remote machine, independently of local installations. The GUI displays remote shell/root and preserves terminal blocks, tabs, splits and normal interactive input.

19. **SR19 — Terminal usability.** Multiple terminals support resize, Unicode, IME, paste, keyboard shortcuts, Ctrl-C and interactive programs. Terminal output is visible to the user but is not copied into task history by default. A slow transfer or long command does not starve input, Agent status or task updates.

20. **SR20 — Session ownership.** Sessions have stable identity and a current process run. The existing Tab/Pane sidebar lists managed sessions and owns name/focus/status/reconnect/stop/detach actions. No parallel Agent dashboard or session manager is introduced. Multiple panes in one tab retain their individual run identity; retained detached sessions are reachable within the same project sidebar hierarchy. Identical process/Agent names do not confer ownership. A new tab or replacement process cannot inherit another run's tasks, readiness or permission to receive old input.

21. **SR21 — Distinct close actions.** Closing a terminal view, disconnecting a connection, requesting process stop and removing a project/profile are separate actions with clearly described effects. For managed sessions, closing a view/disconnecting retains the remote process until explicit Stop or observed exit and shows how to reattach. Removing a project with active sessions offers Stop or Leave running explicitly. Ordinary raw SSH tabs without managed retention show that limitation before abandoning active work. Stop requested remains distinct from observed exit; unreachable work is not marked stopped.

22. **SR22 — Disconnection and reattachment.** Network loss displays offline/stale/unknown state and protects unsent local input. Reconnect may attach only to the same verifiably surviving remote session. Otherwise show ended/unknown and require explicit new launch. Reconnect never replays an arbitrary command, creates a duplicate Agent process or sends a delayed Enter into a replacement session.

### Remote Agents and tasks

23. **SR23 — Remote Agent discovery.** Discover eligible remotely installed CLIs and versions, show configured/MCP-connected/missing-authentication/unsupported states, and offer managed launch. Local and remote installations are distinct. Qoder and QoderCN remain separate clients. Warpai neither supplies a model nor copies local provider credentials to make the remote client work.

24. **SR24 — Project-local communication.** Agents launched into the same managed remote project can discover authorized peers, send messages and assign/review tasks across that project's tabs. A different SSH project does not gain access even on the same host. This release does not join local Agents to remote task scope or route tasks between unrelated hosts automatically.

25. **SR25 — Truthful Agent status.** Distinguish installed, configured, connected, idle, task-ready, busy, approval-blocked, waiting for user input, draft-protected, manually paused, offline and unknown. Use the existing sidebar CLI icon/status treatment with concise text/attention indicators; show state source/age and detailed reasons on demand. Task badges use the same task data as task details. Connected/idle status cannot stand in for task accepted/completed, and background status changes never open another panel. An SSH heartbeat or loaded MCP bridge is not proof of readiness or completed work. Unsupported native lifecycle observation disables automatic wake rather than guessing from arbitrary terminal output.

26. **SR26 — Protected automatic delivery.** An eligible remote Agent can receive queued work automatically only when both its remote state and the local controlling input/view are safe. Preserve drafts, attachments, permission prompts, manual pause/cancel and replacement processes. Recheck immediately before submission. Record queued, claimed/submitted, acknowledged, started, submitted-for-review and accepted as separate facts; lack of acknowledgement is visible and never converted to success.

27. **SR27 — On-demand task GUI.** Reuse the collaboration panel for project-scoped tasks, messages and history. It stays closed by default on fresh profiles and preserves existing saved panel selection; events never open it automatically. Start with compact active tasks and selected-task status/result/wait reason. Show revision/attempts/dependencies/deadlines/evidence/timeline and advanced retry/reassign/archive/export/purge controls on demand. Keep valid assign/cancel/review controls; ordinary Agent communication does not require a board or manual GUI workflow. Disabled controls explain stale connection, permission, version conflict or uncertain execution. A task result is not accepted by rendering it in the panel.

28. **SR28 — Lifecycle and review.** Reuse queued/blocked/running/submitted/cancel_requested/cancelled/failed/expired/accepted behavior. The current assignee submits an owned attempt and the authorized reviewer/operator accepts or requests revision. Rework retains prior evidence. A stale run/revision cannot finish its replacement; an Agent cannot approve its own task.

29. **SR29 — Claims and dependencies.** Explicit eligible Agents may claim FIFO ready work atomically. A task starts only with accepted prerequisites and no conflicting active attempt. Reject cycles and cross-project edges. One concurrent claim wins; network retry does not create another execution owner.

30. **SR30 — Cancellation and uncertainty.** Cancellation of running work requests stop, targets only the exact owned remote run and waits for explicit stopped outcome or verified process exit. Connection loss, timeout, idle and advisory lease expiry do not prove stop. Retry/reassignment of uncertain execution needs explicit operator risk acknowledgement, preserving the prior unknown attempt.

31. **SR31 — Reservations and GUI edits.** Same-project Agents can reserve exact files/subtrees with task/attempt/expiry. Overlapping exclusive reservations in the same physical remote checkout conflict. GUI save/upload/delete shows conflicting reservations. These are advisory records, not OS locks; expiry never proves a writer stopped. Cross-worktree grouping is not enabled merely by matching Git remotes.

32. **SR32 — Evidence location.** Task evidence identifies the producing project, host, checkout, run/attempt and reported command/outcome. Explicit Open fetches the referenced remote file only through that project's authenticated file access. Verification and currently displayed content are distinct. Disconnected/missing/out-of-scope content remains unavailable; a local same-named file is never substituted.

33. **SR33 — Reconciliation.** Lost operation replies retain original intent and show pending/unknown. Reconnect checks the original committed receipt before a retry, without replacing the request ID or inventing a new epoch. Read-only cache does not become authoritative. Already-running remote work can continue if its service/run survives, but the disconnected local GUI cannot claim fresh access or fabricate a stopped outcome.

### GUI consistency, migration and release

34. **SR34 — Consistent context.** Connection/project selection drives Explorer, Transfers, existing sidebar session/Agent rows, terminals and on-demand tasks without merging unrelated projects. A task can link to its exact owning session; a sidebar row can explicitly open its current task. Both use the same authoritative projections. Missing/retired/replaced run links show unavailable rather than selecting another same-named Agent. Tabs may retain separate contexts. Switching project or removing a profile invalidates stale callbacks, searches and action forms; a late response cannot populate or mutate the newly selected project.

35. **SR35 — Focus and accessibility.** Opening/refreshing panels does not steal terminal/editor focus. Actions support keyboard navigation, accessible names, visible focus, text-based status and existing themes. Long host/path/Agent names wrap or expose full values. Narrow panes and larger text retain reachable primary actions; background events preserve unsent drafts. Task-to-session navigation and sidebar-to-task opening change focus only on explicit user action; the task view cannot duplicate sidebar rename/reconnect/stop/close controls.

36. **SR36 — Durable bounded history.** Messages, threads, tasks, reviews, evidence and attempts survive service restart. Live input authority and readiness do not. History remains paginated/searchable; active/unknown/referenced records are not silently purged. Quota errors identify an actionable export/cleanup path. Local and remote histories are separately labeled.

37. **SR37 — Legacy cutover.** Existing local collaboration history remains usable. Former device-enrollment metadata is not automatically trusted as a new SSH project. Old unknown work/intents remain visible/exportable, and removal does not claim execution stopped. Backup/restore is explicit; downgrading an old writer cannot silently rewrite new data.

38. **SR38 — Privacy and deployment.** No Warpai cloud account, billing or telemetry is added. No automatic source synchronization, shell-history upload, full transcript retention, local credential forwarding or privileged/background OS service installation is introduced. The user can inspect helper version/location and remove owned files without deleting project data or unrelated processes.

39. **SR39 — Supported platforms.** The local GUI supports macOS/Windows and remote project environments support Linux/macOS/Windows through explicitly tested helper/shell/SFTP versions. Unsupported architecture/libc/shell configurations are reported. Linux remote support does not imply a Linux desktop release. Agent compatibility records name actual vendor versions rather than claiming universal support.

40. **SR40 — Honest acceptance.** Separate source build, protocol fixture, real SSH/SFTP, native GUI, deterministic remote edit/test and real vendor-model execution evidence. Old device-gateway tests are historical evidence only. Unavailable physical machines or model budget remain unverified and cannot be presented as successful SSH Remote release acceptance.


## Remote host status in the SSH task panel

41. **SR41 — Host status alongside remote tasks.** The on-demand SSH task panel
    includes a compact status header for its exact selected remote environment
    and project: host/account/root, connection state and separate SSH, SFTP,
    companion and MCP availability. Show remotely sampled CPU utilization,
    memory used/total and free/total space on the selected project's volume,
    with sample source and observation age; reveal OS/architecture and uptime
    in expanded detail. Unsupported or denied metrics are unavailable, never
    zero. Offline retains the last sample with a stale/offline label and cannot
    imply current health or Agent readiness. Background refresh neither opens
    the panel nor steals focus. A task viewed from history identifies its
    producing host rather than displaying another tab's currently selected host.

This requirement was added by the user on 2026-10-03. Its executable contract,
static states and verification are defined in [HOST-STATUS.md](HOST-STATUS.md).
It does not create another Agent/session dashboard or outbound telemetry service.
