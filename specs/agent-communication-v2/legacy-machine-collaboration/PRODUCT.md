# Reliable Local and Remote Agent Collaboration

Status: proposed implementation plan, 2026-10-01. This document describes future behavior, not shipped capabilities. Start with [PLAN.md](PLAN.md); implementation and verification live in [TECH.md](TECH.md), and public contracts in [API.md](API.md).

## Summary

Make collaboration between independently installed CLI agents observable, recoverable and safe to coordinate across files, worktrees and explicitly connected computers. Preserve the existing local-first terminal, native agent interfaces, task review ownership and user control of each terminal.

## Scope

Deliver real execution acceptance, a collaboration panel, cancellation/failure recovery, dependencies and task claiming, advisory file reservations, threaded messages and evidence references, durable searchable history, and remote collaboration over SSH. Target macOS and Windows, including either platform as the coordinator.

The first remote release supports a single trusted user's explicitly enrolled machines, with Warpai running on participating machines. Hosted accounts, automatic agent installation, unattended headless workers, automatic code synchronization, automatic merges, multiple writable coordinators and public unauthenticated endpoints are outside this release. Existing CLI provider accounts and permissions remain under the user's control.

## Design reference

Figma: none provided. The user confirmed on 2026-10-01 that the panel should follow Warpai's existing interface style. Use the existing Tools Panel, themes, controls and keyboard conventions. UI implementation requires a static native fixture and screenshot review before connecting live data.

## Behavior

### Participation and visibility

1. **B01 — Opt-in participation.** Existing communication settings continue to govern participation. Enabling a client authorizes native-discovered sessions in the current project; disabling it immediately revokes their communication. Routine local participation needs no peer-by-peer pairing.
2. **B02 — Truthful status.** Distinguish installed, configured, connected, ready, busy, permission-blocked, draft-protected, manually paused, offline and connection-unknown states. Unknown native lifecycle information must be shown as unknown. A connected bridge is not evidence that an agent executed a task.
3. **B03 — Collaboration panel.** An `Agent collaboration` Tools Panel entry shows agents, tasks and activity for the active collaboration space. Agent rows show name, CLI, device, workspace, readiness and last observed activity. Selecting a local agent can focus its existing terminal; a remote agent is clearly labeled and is not presented as a local terminal.
4. **B04 — Task inspection.** Users can filter tasks by state and assignee, open a task's description, acceptance criteria, attempts, dependencies, results and review feedback, and inspect its event timeline. Queued tasks explain why they cannot start, including dependencies, unavailable assignee, another active task, protected input and uncertain previous execution.
5. **B05 — Human control.** Users can assign, cancel, retry, reassign, review and archive eligible tasks from the panel. These actions have the same validation as agent operations and are attributed to the local human operator. Destructive history deletion and explicit overrides of uncertain execution require a concrete confirmation describing the affected work. Ordinary messaging and assignment do not gain extra approval dialogs.
6. **B06 — Focus and accessibility.** Opening or refreshing the panel must not steal terminal focus. All actions support keyboard navigation, visible focus, accessible labels and status text independent of color. Empty, loading, disconnected, stale, capacity-limited and failed states expose an appropriate next action. Long names, messages and narrow panels remain readable.

### Delivery and task ownership

7. **B07 — Observable delivery.** Queued, submitted to terminal, acknowledged, started, submitted for review and accepted are separate facts. A timeout or lost connection cannot turn an unknown outcome into success. Only native message acknowledgement or a corresponding task transition marks work consumed. Known, still-authorized offline participants may receive queued messages/tasks; unknown or revoked identities cannot. The sender sees that offline work has not started.
8. **B08 — Protected terminal input.** Automatic wake preserves drafts, attachments, permission prompts, manual cancellation and replacement sessions. A busy agent receives queued work only after verified readiness. Retrying a delivery cannot repeatedly submit the same prompt while the original outcome is unknown.
9. **B09 — Task lifecycle.** Tasks expose `blocked`, `queued`, `running`, `submitted`, `cancel_requested`, `cancelled`, `failed`, `expired` and `accepted` states. Offline/interrupted execution and overdue review are additional conditions, not proof of completion. Only the active assignee can submit an active attempt, and only the named reviewer or human operator can accept it. An agent cannot review its own work.
10. **B10 — Cancellation.** Cancelling unstarted or submitted work takes effect immediately. Cancelling running work first requests a stop; it becomes cancelled only after the receiver confirms stopping or the local controller observes that the owning process has exited. Sending an interrupt alone does not prove the work stopped. Reassignment remains blocked while the old execution outcome is uncertain, unless the user explicitly overrides it.
11. **B11 — Failure, deadlines and recovery.** An assignee can report failure with a reason and evidence. An optional start deadline expires unstarted tasks; an optional execution deadline requests cancellation of running work. Review delays show overdue status without accepting or discarding results. Retry/reassignment creates a new attempt and invalidates the previous attempt's authority. It never claims that previous filesystem or external side effects were undone.
12. **B12 — Restart behavior.** Messages, task state, review history and attempts survive restart. Live readiness and execution authority do not. Interrupted work requires explicit recovery or reassignment. A replacement terminal cannot inherit another process's permission, draft state or right to submit a stale result.

### Dependencies and concurrent work

13. **B13 — Dependencies.** A task may depend on other tasks in the same space. It can start only when all dependencies are accepted. Cycles, self-dependencies and cross-space dependencies are rejected. Failed, cancelled or expired prerequisites keep dependents blocked until the prerequisite is retried or the issuer/human changes dependencies. Editing dependencies of running work is rejected.
14. **B14 — Assignment and claiming.** A task can name an assignee or be placed in a shared queue with explicit eligible agents. An idle eligible agent can claim it atomically; only one concurrent claim succeeds. The default queue order is creation order. An issuer can explicitly assign ready work; the system does not guess an agent's skills from its model or CLI name.
15. **B15 — Coordinator behavior.** The application enforces state, dependencies, deadlines and exclusive attempt ownership. The issuing model remains responsible for task decomposition and semantic review. The panel reports a stalled coordinator or reviewer instead of claiming that a prompt instruction guarantees continued model participation.
16. **B16 — File reservations.** An agent can declare files or directory subtrees it intends to change, with an expiry and task association. Overlapping exclusive declarations produce a visible conflict and require resolution before a new exclusive reservation is granted. Reservations coordinate cooperating agents; they do not prevent an arbitrary editor or CLI from writing files.
17. **B17 — Worktree collaboration.** Separate worktrees and repositories remain isolated by default. A user may explicitly join selected workspaces into a collaboration space. The panel distinguishes a physical checkout from the logical repository and branch/base commit. Same-file edits in different worktrees warn of merge overlap rather than pretending that the physical file is shared. Expired reservations do not prove that a former owner stopped editing.

### Context, evidence and history

18. **B18 — Threads.** Messages can have a subject, thread, reply target and optional task association. Replies preserve thread membership and visibility. Agents can fetch unread messages, thread history and paginated search results without receiving every terminal transcript. Task events appear in the same task detail regardless of the sender's CLI.
19. **B19 — Evidence.** Results can refer to repository-relative files, commits, diffs and recorded test commands/outcomes. Evidence identifies the producing device, workspace and attempt. A path reference is not a file upload; unavailable remote files are labeled unavailable. References and agent-reported test success are not silently promoted to independently verified facts.
20. **B20 — Bounded sharing.** Remote messages and evidence metadata are shared only inside explicitly granted spaces. Source files, environment variables, credentials, shell history and full agent transcripts are never automatically uploaded. Opening a reference does not execute it. An initial release shares commit/diff references and user-readable summaries; binary attachment transfer is a separate feature.
21. **B21 — History and capacity.** Completed work can be archived and searched without occupying the active task queue. Active work, unread messages, unresolved dependencies and current retry records cannot be silently discarded. Storage pressure is visible before writes are refused; users can export or explicitly delete eligible archived records. Export preserves ordering and task/thread relationships and excludes secrets.

### Remote collaboration

22. **B22 — Connection setup.** A user chooses an SSH host already reachable with their system SSH configuration, enrolls that device with the chosen coordinator, and explicitly grants collaboration spaces. The UI displays device identity, host verification, granted projects and connection status. Missing SSH, untrusted/changed host keys, authentication failure and incompatible versions are actionable states. Warpai does not automatically enable the OS SSH server or change firewall settings.
23. **B23 — Workspace mapping.** Each enrolled device explicitly maps its own local checkout to a collaboration space and logical repository. Matching directory names, Git remotes or repository content alone never grants access. Joining a space applies to newly selected sessions; existing private work is not silently moved or exported.
24. **B24 — Single authority.** Each shared space has exactly one coordinator that stores authoritative messages and task state. A connected machine controls only its own agents and terminals. A remote message cannot create a local terminal binding, impersonate another device or bypass local participation settings.
25. **B25 — Disconnection.** The UI distinguishes connected, reconnecting, offline and unknown execution. Disconnected participants cannot claim new remote work or finalize shared state. An already running model may continue locally; its unconfirmed updates wait for reconciliation. Private local spaces continue operating. Closing the coordinator pauses shared coordination and does not silently elect a replacement.
26. **B26 — Reconnection.** Reconnect resumes from the last confirmed event and reconciles pending operations with stable identities. Duplicate network delivery does not duplicate a task transition. Results from revoked, replaced or cancelled attempts remain historical evidence and cannot complete a new attempt.
27. **B27 — Revocation.** Removing a device or space grant stops new access immediately and invalidates its current remote session. Locally managed terminals stop receiving remote work. Revocation cannot retract information already received or forcibly stop an unreachable process; uncertain work remains visible and protected from silent reassignment.
28. **B28 — Remote execution experience.** Remote work appears and executes in the receiving machine's existing native agent terminal, subject to the same draft/permission/readiness rules as local work. The sender sees progress, evidence and review results through the collaboration panel. Communication does not synchronize files or merge branches.

### Compatibility and release behavior

29. **B29 — Upgrade.** Existing identities, tasks, acknowledgements and reviews migrate without inventing historical timestamps or execution evidence. Unsupported app/bridge/protocol combinations fail with upgrade guidance. A database rollback is explicit and warns about work created since the backup; older binaries must not silently open a writable stale copy.
30. **B30 — Acceptance claims.** Compatibility status separates configuration checks, native handshake, real model execution and automatic wake validation, records CLI/OS/app versions and dates, and marks unavailable combinations as unverified. A remote acceptance claim requires a real second machine; loopback simulation is reported as simulation.
