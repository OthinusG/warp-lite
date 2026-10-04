# SSH Agent Communication Acceptance

Date: 2026-10-04. Active scope: [PLAN.md](PLAN.md). Exact-source evidence and
acceptance updates are recorded in [PROGRESS.md](PROGRESS.md). Earlier full
SSH project-manager gates and local verification history are [archived](https://github.com/OthinusG/warpai/blob/47a2a5a/specs/agent-communication-v2/legacy-ssh-project-manager/ACCEPTANCE.md).

| Item | Completion check | State |
| --- | --- | --- |
| S0 | Redundant file/transfer/dashboard/metrics/device/session runtime removed; local Store migration, terminal and communication regressions preserved | Accepted |
| S1 | Same-account native root selects one private remote Broker/Store; different/replaced roots, account and stale authority are denied | Accepted |
| S2 | Real owned children initialize/discover MCP with fresh project/run credentials; stale or revoked capabilities cannot act or replay receipts | Accepted |
| S3 | Two actual native Agent fixtures exchange a message and complete assign/start/submit/review through controlled OpenSSH; other roots isolated and disconnect stops owned work | Accepted |
| S4 | Existing panel selects target/root/helper, reads real remote projections, sends human messages and assigns/opens a task; offline writes denied, terminal/form drafts preserved through reconnect and return to local | Accepted source4fcb0c3/run37138929300 on macOS and Windows, including eight reviewed live images per OS |
| S5 | Final relevant three-platform tests, both desktop default/platform checks, focused application tests and source-matched review packages; helper setup and evidence documented | Accepted source4fcb0c3: remote run37138932460 and desktop run37138929300; packages inspected |

S4 visual checks cover the existing native controls, narrow/wide themes and zoom,
readable remote Agent/task status, preserved terminal drafts, and no overlap or
hidden primary controls. The live panel uses real private companion and native
MCP processes; controlled Linux OpenSSH separately verifies the shared transport.
Neither desktop runner is described as a physical remote SSH host.

Rust compilation and execution run on GitHub only. Local formatting/parsing,
workflow and shell syntax checks do not substitute for native runtime evidence.
Record the tested SHA, run, platform, fixtures/actions and result before checking
each PLAN item. Unknown results remain unknown; no automatic mutation retry.

Manual companion placement and system SSH authentication are sufficient. Paid
vendor model sessions and physical cross-device SSH are not claimed by fixture
acceptance. No file manager, transfer queue, retained-session UX or eight-hour
manager soak is required by this delivery.

## Native Codex ownership and deep cleanup — 2026-10-05

Source f6fa053 passed remote run
[37230080613](https://github.com/OthinusG/warpai/actions/runs/37230080613) on
Linux/macOS/Windows, preserving local coordination and controlled SSH behavior.
The original macOS Codex 0.160.0 separately passed two simultaneous participants
and four completed model/MCP turns with source-matched agent_bus inputs. See
[compatibility coverage](../agent-communication/COVERAGE.md) and
[cleanup evidence](../DEEP-CLEANUP.md). Icon revision and release packaging are
explicitly deferred; historical package acceptance does not certify this source.
