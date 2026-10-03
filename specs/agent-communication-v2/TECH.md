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

Manual helper deployment is sufficient. Existing retained PTY/host-status code
may be reused where it shortens integration, but retention UI, file operations,
transfer machinery and automatic installation are independent future scope.

## Verification

Prove two remote fixture processes in one project exchange existing messages/tasks,
another project cannot read/write them, replaced roots/runs reject stale replay,
and disconnect/reconnect status is truthful. Run relevant GitHub-only Rust tests,
both desktop build checks and focused native screenshots/draft checks. Local
communication remains a required regression. Update PROGRESS per accepted item.

The detailed former manager architecture is archived in
legacy-ssh-project-manager/TECH.md and is not a delivery checklist.
