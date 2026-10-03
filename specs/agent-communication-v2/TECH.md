# SSH Agent Communication Extension Technical Scope

Date: 2026-10-03. Active scope follows the user correction and PLAN S1–S5.

## Reuse and authority

Reuse warp-agent-bus Broker/Store/MCP and existing terminal/vendor adapters.
System OpenSSH authenticates the account. A repository-owned companion exposes
bounded typed control over a clean SSH pipe to one account service. Selected
native root identity chooses the private remote project Store. Local GUI reads
projections; it does not become another authoritative task database.

A private run capability is created by the companion and injected only into the
owned Agent child. Broker run equals actual native run. Root/capability/run checks
precede operations and receipt replay. Project replacement, run revocation and
service restart reject old authority; ordinary terminals get no Agent binding.
Existing messages, tasks, versions, receipts, reviews and unknown-work recovery
remain the only domain engine. Existing typed protobuf framing remains explicit;
project_tasks JSON is a typed field, never protocol auto-detection.

## Minimum user workflow

Use existing SSH terminal IO and launch adapters for an explicit managed Agent
launch. Add only the remote target/root/manual helper selection needed by the
current collaboration panel. Adapt panel refresh/operator calls to choose either
the existing local Broker or exact remote project client. Generation-fence results
and preserve input/focus. Display a compact status section in that panel, without
creating a separate session manager. Static native review precedes live binding.

Manual helper deployment is sufficient. Agent PTYs belong to their SSH connection: disconnect revokes MCP and stops the
owned group/job. Session list, detach/reattach and generation takeover are removed.
Bounded IO buffering and native ownership checks remain required for Agent IO.

## Verification

Prove two remote fixture processes in one project exchange existing messages/tasks,
another project cannot read/write them, replaced roots/runs reject stale replay,
and disconnect/reconnect status is truthful. Run relevant GitHub-only Rust tests,
both desktop build checks and focused native screenshots/draft checks. Local
communication remains a required regression. Update PROGRESS per accepted item.

The detailed former manager architecture is archived in
legacy-ssh-project-manager/TECH.md and is not a delivery checklist.

### S3 minimum launch path (implementation pending validation)

Inside an ordinary SSH terminal, run `warpai-companion agent <absolute-root>
<program> <absolute-vendor-executable> [vendor-arguments...]`. The CLI joins the
same private account service and forwards native PTY IO; the owned child reuses
`session::launch` and its existing vendor MCP/readiness adapters. The companion
also exposes the existing `forward` relay entrypoint required by those adapters.
TTY mode is restored on normal/error return; resize follows the original terminal.
No credentials are passed in arguments or copied from the desktop. Service launch
validates a program, absolute vendor executable and at most 61 vendor arguments.
Disconnect ends the owned run; no retained-session takeover is available.

Linux remote evidence uses the opened file's `/proc/self/fd` path, followed by the
same root/credential/regular-file/size/hash checks as the existing desktop engine.
Unix evidence opens use O_NOFOLLOW/O_NONBLOCK to refuse symlinks and avoid FIFOs.

### S4 visual source

The existing native Collaboration panel and its existing editor/button components
are the visual source. Added four fixed SSH states (connecting, connected,
disconnected, error) to panel-fixtures.json, with target/root/companion and observed
Agent/task rows. Review native captures before wiring live remote projections.
No independent dashboard, resource metrics, file actions or session manager.

Remote selection is panel-local and transient. Reuse the existing human-intent
form for the system SSH alias, absolute remote root and companion path. A single
serialized HostClient handles reads and explicit mutations; no local remote Store
or saved credential/profile manager is introduced. Connection failure retains the
last projection and original intent, disables writes and requires explicit
reconnect. Selection/visibility generations reject late reads; reconnect keeps
drafts, while switching targets requires closing the current form. Remote file
and terminal focus actions cannot use local desktop paths or terminal IDs.
