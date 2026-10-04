# SSH Agent Communication Extension Technical Scope

Date: 2026-10-04. Active scope follows the user correction and PLAN S0–S5.
All six items are accepted; exact-source evidence is in PROGRESS.md.

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

### S3 minimum launch path (accepted)

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

## Native CLI ownership correction — 2026-10-05

User acceptance rejects transparent replacement of installed Agent commands.
Warpai must not create vendor-named launchers, prepend a private launcher directory
to PATH, dispatch its bridge by a vendor executable name, or force Codex through
a custom daemon/WebSocket frontend. Ordinary `codex` keeps its native executable,
arguments, working directory, configuration and package-manager update path.

Remove automatic aliases/catalog publication and Codex daemon proxy interception.
Preserve independently installed Agent management, native MCP configuration,
private broker authentication and explicit remote companion launches. Remote
launches may supply the original client's documented MCP configuration arguments;
they must execute its original absolute binary and inherit the selected project
working directory. Unsupported native capabilities remain unsupported rather
than being implemented through command replacement.

Verify all former alias creation/PATH injection callers, original command
resolution, project directory and argument preservation, and a bridge regression
that cannot dispatch as `codex`. Rust checks/tests run on GitHub. The earlier
desktop/source acceptance is superseded; final screenshots/packages must use
this corrected source when the user resumes unified acceptance monitoring.


### Replacement plan: native, explicit, session-only Codex MCP

Use the official installed CLI directly. The installed 0.160.0 help confirms
repeatable `-c key=value` runtime overrides and `--no-daemon`. Reference:
https://developers.openai.com/codex/cli/reference and
https://developers.openai.com/codex/mcp.

1. Ordinary terminal commands remain ordinary commands. No aliases, executable
   copies, shell hooks, PATH interception, CODEX_HOME replacement, vendor file
   changes, or daemon proxy are permitted.
2. An explicit Warpai-managed launch resolves the current package-manager command
   path on each launch, probes native capabilities, and runs that executable in
   the selected project directory. Do not canonicalize a package-manager symlink
   into a cached versioned executable. Preserve user launch arguments and flags.
3. Supply only the communication server through native `-c` overrides for that
   invocation. Retain the user's other MCP servers, provider, authentication and
   permissions. For a version exposing `--no-daemon`, use it to isolate the
   managed session from an already-running daemon. A version without a verified
   session isolation contract runs normally but is not advertised as managed MCP.
4. Reuse the private broker and existing MCP stdio bridge. Relay credentials stay
   in private runtime state, never command arguments or vendor configuration.
   Disabling communication revokes broker authority without editing Codex files.
5. Do not inject parameters into arbitrary user-typed `codex` commands. Local
   managed launch UI integration is a separate implementation task after the
   rollback; it must be explicit, visible, and covered by native acceptance.

Rollback implementation removes the alias/catalog/proxy and makes the legacy
serialized Codex adapter a no-op so existing preferences still load. Previously
written vendor configuration is preserved rather than automatically edited.
Explicit remote launch already uses native per-invocation MCP options and the
selected project's cwd. The local managed UI is not yet delivered; ordinary
Codex remains available and does not automatically receive the communication MCP.

Acceptance must verify original executable resolution before/after a simulated
package-manager upgrade, two different project roots, two isolated simultaneous
sessions, unchanged vendor configuration bytes and shell PATH, original provider
and permissions, broker revocation, and model-level MCP calls. Old proxy probes
are historical evidence only and cannot certify this replacement.
