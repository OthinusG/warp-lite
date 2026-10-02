use serde_json::Value;
use std::{path::PathBuf, thread, time::Duration};
use uuid::Uuid;
use warp_agent_bus::{
    transport::{self, Broker, Request, RunningBroker},
    Operation, Store,
};
fn request_id() -> String {
    Uuid::new_v4().to_string()
}
fn managed_programs() -> Vec<String> {
    // Use the restored detector as the coverage source; aliases map to its canonical prefix.
    let mut programs: Vec<String> = include_str!("../../../app/src/terminal/cli_agent.rs")
        .lines()
        .filter_map(|line| line.trim().strip_prefix("CLIAgent::"))
        .filter(|line| line.contains("=> &[\""))
        .filter_map(|line| line.split('"').nth(1).map(str::to_owned))
        .collect();
    programs.push("custom".into());
    programs
}
#[test]
fn every_managed_type_delegates_reviews_and_recovers() {
    let path: PathBuf =
        std::env::temp_dir().join(format!("warp-agent-test-{}.sqlite", request_id()));
    {
        let store = Store::open(path.to_str().unwrap()).unwrap();
        let reviewer = store
            .register("reviewer", "claude", "/project", "reviewer")
            .unwrap();
        let stranger = store
            .register("stranger", "codex", "/elsewhere", "stranger")
            .unwrap();
        let programs = managed_programs();
        assert!(programs.len() >= 19);
        for (index, program) in programs.iter().enumerate() {
            let worker = store
                .register(
                    &format!("terminal-{index}"),
                    program,
                    "/project",
                    &format!("worker-{index}"),
                )
                .unwrap();
            let assign = Operation::TaskAssign {
                dependencies: vec![],
                start_deadline: None,
                execution_timeout_seconds: None,
                review_timeout_seconds: None,
                to: worker.id.clone(),
                description: "Implement a focused change".into(),
                acceptance: "Run a representative check".into(),
                reviewer: None,
                request_id: request_id(),
            };
            let task = store.execute(&reviewer, "review-run", &assign).unwrap();
            assert_eq!(
                task,
                store.execute(&reviewer, "review-run", &assign).unwrap()
            );
            let id = task["id"].as_str().unwrap().to_owned();
            assert!(store
                .execute(
                    &stranger,
                    "stranger-run",
                    &Operation::TaskGet {
                        task_id: id.clone()
                    }
                )
                .is_err());
            assert!(store.next_work(&worker, "worker-run").unwrap().is_some());
            store
                .execute(
                    &worker,
                    "worker-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 1,
                        expected_version: None,
                        request_id: request_id(),
                    },
                )
                .unwrap();
            store.recover(&worker, "worker-run").unwrap();
            assert_eq!(store.task(&worker, &id).unwrap().attempts.len(), 1);
            let submission = Operation::TaskSubmit {
                evidence_ids: vec![],
                task_id: id.clone(),
                revision: 1,
                result: "Implemented".into(),
                evidence: "Focused test passed".into(),
                expected_version: None,
                attempt_id: None,
                request_id: request_id(),
            };
            assert!(store.execute(&worker, "obsolete-run", &submission).is_err());
            store
                .execute(&worker, "worker-run", &submission)
                .unwrap();
            let review = Operation::TaskReview {
                task_id: id.clone(),
                revision: 1,
                accepted: false,
                feedback: "Cover the failure case".into(),
                expected_version: None,
                request_id: request_id(),
            };
            assert!(store.execute(&worker, "worker-run", &review).is_err());
            let rework = store.execute(&reviewer, "review-run", &review).unwrap();
            assert_eq!(rework["revision"], 2);
            assert!(
                store
                    .execute(&worker, "worker-run", &submission)
                    .unwrap()["state"]
                    == "submitted"
            ); // Identical retry returns its original response.
            assert!(store
                .execute(
                    &worker,
                    "worker-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 1,
                        expected_version: None,
                        request_id: request_id()
                    }
                )
                .is_err());
            store
                .execute(
                    &worker,
                    "worker-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 2,
                        expected_version: None,
                        request_id: request_id(),
                    },
                )
                .unwrap();
            store
                .execute(
                    &worker,
                    "worker-run",
                    &Operation::TaskSubmit {
                        evidence_ids: vec![],
                        task_id: id.clone(),
                        revision: 2,
                        result: "Failure case covered".into(),
                        evidence: "Failure simulation passed".into(),
                        expected_version: None,
                        attempt_id: None,
                        request_id: request_id(),
                    },
                )
                .unwrap();
            assert_eq!(
                store
                    .execute(
                        &reviewer,
                        "review-run",
                        &Operation::TaskReview {
                            task_id: id,
                            revision: 2,
                            accepted: true,
                            feedback: "Verified".into(),
                            expected_version: None,
                            request_id: request_id()
                        }
                    )
                    .unwrap()["state"],
                "accepted"
            );
        }
        // A pane changing projects gets a distinct identity and cannot read prior messages.
        let moved = store
            .register("terminal-0", &programs[0], "/elsewhere", "moved")
            .unwrap();
        assert!(store.pending(&moved).unwrap().is_empty());
        let inbox = store
            .execute(&reviewer, "review-run", &Operation::AgentList)
            .unwrap();
        assert_eq!(inbox.as_array().unwrap().len(), programs.len() + 1);
    }
    let reopened = Store::open(path.to_str().unwrap()).unwrap();
    let reviewer = reopened
        .register("replacement-reviewer", "claude", "/project", "reviewer")
        .unwrap();
    assert!(
        reopened
            .execute(&reviewer, "new-run", &Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .len()
            >= 20
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
fn register(broker: &Broker, terminal: &str, program: &str) -> Request {
    let capability = broker.prepare(terminal).unwrap();
    broker
        .activate(terminal, program, "/project", false)
        .unwrap();
    let mut request = Request {
        protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
        terminal: terminal.into(),
        capability,
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: None,
        operation: Operation::AgentRegister {
            name: terminal.into(),
        },
    };
    let result = transport::call(&broker.endpoint, &request).unwrap();
    assert_eq!(result["protocol_major"], transport::PROTOCOL_MAJOR);
    assert_eq!(result["features"], serde_json::json!(transport::LOCAL_FEATURES));
    request.run = result["run"].as_str().map(str::to_owned);
    request
}

#[test]
fn native_workspace_and_pre_discovery_activity_are_not_heuristic_idle() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let project = warp_agent_bus::project_root(first.path()).unwrap();
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let mut issuer = register(&server.broker, "issuer", "codex");
    for state in [
        "native-context",
        "native-busy",
        "user-submit",
        "user-draft",
        "empty",
    ] {
        let capability = server.broker.prepare(state).unwrap();
        server
            .broker
            .activate(state, "codex", "/project", true)
            .unwrap();
        match state {
            "native-busy" => server.broker.readiness(state, false),
            "user-submit" => server.broker.user_input(state, true),
            "user-draft" => server.broker.user_input(state, false),
            _ => {}
        }
        let mut receiver = Request {
            protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
            terminal: state.into(),
            capability,
            run: None,
            defer_initial_ready: state == "native-context",
            native_activity: None,
            directory: (state == "native-context")
                .then(|| first.path().to_string_lossy().into_owned()),
            operation: Operation::AgentRegister { name: state.into() },
        };
        let registered = transport::call(&server.broker.endpoint, &receiver).unwrap();
        receiver.run = registered["run"].as_str().map(str::to_owned);
        if state == "native-context" {
            assert_eq!(registered["agent"]["project"], project);
            let capability = server.broker.prepare("native-issuer").unwrap();
            server
                .broker
                .activate("native-issuer", "codex", &project, false)
                .unwrap();
            let mut native_issuer = Request {
                protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
                terminal: "native-issuer".into(),
                capability,
                run: None,
                defer_initial_ready: false,
                native_activity: None,
                directory: None,
                operation: Operation::AgentRegister {
                    name: "native-issuer".into(),
                },
            };
            let registered = transport::call(&server.broker.endpoint, &native_issuer).unwrap();
            native_issuer.run = registered["run"].as_str().map(str::to_owned);
            native_issuer.operation = Operation::AgentSend {
                to: state.into(),
                body: "weekday".into(),
                subject: None,
                thread_id: None,
                reply_to: None,
                task_id: None,
                request_id: request_id(),
            };
            transport::call(&server.broker.endpoint, &native_issuer).unwrap();
            receiver.directory = Some(second.path().to_string_lossy().into_owned());
            receiver.operation = Operation::AgentList;
            assert!(
                transport::call(&server.broker.endpoint, &receiver).is_err(),
                "A registered identity cannot switch projects"
            );
            assert!(server
                .broker
                .peers("issuer")
                .iter()
                .all(|peer| peer.terminal != state));
            continue;
        }
        issuer.operation = Operation::AgentSend {
            to: state.into(),
            body: "weekday".into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: request_id(),
        };
        transport::call(&server.broker.endpoint, &issuer).unwrap();
    }
    thread::sleep(Duration::from_millis(850));
    let wakes = server.broker.wakeups();
    assert_eq!(wakes.len(), 1);
    assert_eq!(wakes[0].terminal, "empty");
    let legacy = serde_json::json!({"terminal":"legacy","capability":"placeholder","run":null,"operation":{"op":"agent_register","name":""}});
    let legacy: Request = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy.protocol_major, 0);
    let error = transport::call(&server.broker.endpoint, &legacy).unwrap_err();
    assert_eq!(error.downcast_ref::<warp_agent_bus::DomainError>().unwrap().code, "protocol_incompatible");
    assert!(!legacy.defer_initial_ready);
    assert!(legacy.directory.is_none());
    assert!(!serde_json::to_value(legacy)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("directory"));
}
#[test]
fn socket_wait_handoff_and_expired_run_rejection() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let sender = register(&server.broker, "sender", "codex");
    let mut worker = register(&server.broker, "worker", "agy");
    let broker = server.broker.clone();
    worker.operation = Operation::AgentWait;
    let waiting = worker.clone();
    let join = thread::spawn(move || transport::call(&broker.endpoint, &waiting).unwrap());
    thread::sleep(Duration::from_millis(80));
    let mut assignment = sender.clone();
    assignment.operation = Operation::TaskAssign {
        dependencies: vec![],
        start_deadline: None,
        execution_timeout_seconds: None,
        review_timeout_seconds: None,
        to: "worker".into(),
        description: "Inspect code".into(),
        acceptance: "Report evidence".into(),
        reviewer: None,
        request_id: request_id(),
    };
    transport::call(&server.broker.endpoint, &assignment).unwrap();
    assert_eq!(join.join().unwrap()["message"]["kind"], "assignment");
    let mut competitor = register(&server.broker, "competitor", "claude");
    competitor.run = None;
    competitor.operation = Operation::AgentRegister {
        name: "worker".into(),
    };
    assert!(transport::call(&server.broker.endpoint, &competitor).is_err());
    let claim_capability = server.broker.prepare("claimant").unwrap();
    server
        .broker
        .activate("claimant", "claude", "/project", false)
        .unwrap();
    let claim = Request {
        protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
        terminal: "claimant".into(),
        capability: claim_capability,
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: None,
        operation: Operation::AgentRegister {
            name: "worker".into(),
        },
    };
    assert!(transport::call(&server.broker.endpoint, &claim).is_err());
    let mut forged = worker.clone();
    forged.capability = "invalid".into();
    assert!(transport::call(&server.broker.endpoint, &forged).is_err());
    server.broker.end("worker");
    let recovered = transport::call(&server.broker.endpoint, &claim).unwrap();
    assert_eq!(recovered["agent"]["name"], "worker");
    assert!(transport::call(&server.broker.endpoint, &worker).is_err());
    server
        .broker
        .activate("worker", "agy", "/project", false)
        .unwrap();
    assert!(transport::call(&server.broker.endpoint, &worker).is_err());
    worker.operation = Operation::AgentRegister {
        name: "expired-claim".into(),
    };
    assert!(transport::call(&server.broker.endpoint, &worker).is_err());
    let schema = warp_agent_bus::mcp::tools();
    assert_eq!(schema.len(), 30);
    for tool in schema {
        assert_eq!(
            tool.input_schema.get("additionalProperties"),
            Some(&Value::Bool(false))
        );
        assert!(!tool.input_schema["properties"]
            .as_object()
            .unwrap()
            .contains_key("op"));
    }
    assert!(serde_json::from_value::<Operation>(serde_json::json!({"op":"agent_send","to":"worker","body":"hello","request_id":request_id(),"sender":"spoof"})).is_err());
}

#[tokio::test]
async fn real_stdio_mcp_negotiates_and_registers_every_managed_type() {
    use rmcp::{model::CallToolRequestParam, transport::TokioChildProcess, ServiceExt};
    use warp_agent_bus::transport::{CAPABILITY, ENDPOINT, TERMINAL};
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = server.broker.clone();
    let reviewer =
        tokio::task::spawn_blocking(move || register(&broker, "live-reviewer", "claude"))
            .await
            .unwrap();
    for (index, program) in managed_programs().iter().enumerate() {
        let terminal = format!("mcp-{index}");
        let capability = server.broker.prepare(&terminal).unwrap();
        server
            .broker
            .activate(&terminal, program, "/project", false)
            .unwrap();
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_warp-agent"));
        command
            .arg("mcp")
            .env(ENDPOINT, &server.broker.endpoint)
            .env(CAPABILITY, capability)
            .env(TERMINAL, &terminal);
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 30);
        let native = server
            .broker
            .peers("live-reviewer")
            .into_iter()
            .find(|peer| peer.terminal == terminal)
            .expect("Native discovery must register without a user prompt");
        assert!(server
            .broker
            .peers(&terminal)
            .iter()
            .any(|peer| peer.terminal == "live-reviewer"));
        let result = client
            .call_tool(CallToolRequestParam {
                name: "warp_agent_list".into(),
                arguments: None,
            })
            .await
            .unwrap();
        assert_ne!(
            result.is_error,
            Some(true),
            "MCP registration failed: {:?}",
            result.content
        );
        let listed = client
            .call_tool(CallToolRequestParam {
                name: "warp_agent_list".into(),
                arguments: None,
            })
            .await
            .unwrap();
        assert_ne!(
            listed.is_error,
            Some(true),
            "MCP list failed: {:?}",
            listed.content
        );
        client.cancel().await.unwrap();
        server.broker.end(&terminal);
    }
}

#[tokio::test]
async fn stdio_discovery_probe_falls_back_without_losing_buffered_initialization() {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use warp_agent_bus::transport::{CAPABILITY, ENDPOINT, TERMINAL};
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = server.broker.clone();
    tokio::task::spawn_blocking(move || register(&broker, "probe-reviewer", "claude")).await.unwrap();
    for probe_id in [None, Some(serde_json::json!(1)), Some(serde_json::json!("discover"))] {
        let terminal = request_id();
        let capability = server.broker.prepare(&terminal).unwrap();
        server.broker.activate(&terminal, "agy", "/project", false).unwrap();
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_warp-agent"))
            .arg("mcp")
            .env(ENDPOINT, &server.broker.endpoint)
            .env(CAPABILITY, capability)
            .env(TERMINAL, &terminal)
            .stdin(Stdio::piped()).stdout(Stdio::piped())
            .kill_on_drop(true).spawn().unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut messages = Vec::new();
        if let Some(id) = &probe_id {
            messages.push(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"server/discover","params":{}}));
        }
        messages.extend([
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"native-probe","version":"1"}}}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}),
        ]);
        for message in messages {
            input.write_all(format!("{message}\n").as_bytes()).await.unwrap();
        }
        // Pipelined input verifies that the prelude retains the SDK's unread buffered bytes.
        for expected_id in probe_id.iter().cloned().chain([serde_json::json!(2), serde_json::json!(3)]) {
            let mut line = String::new();
            tokio::time::timeout(Duration::from_secs(5), output.read_line(&mut line)).await.unwrap().unwrap();
            let response: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(response["id"], expected_id);
            if probe_id.as_ref() == Some(&expected_id) {
                assert_eq!(response["error"]["code"], -32601);
            } else if expected_id == 2 {
                assert!(response["result"]["protocolVersion"].is_string());
            } else {
                assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 30);
            }
        }
        assert_eq!(server.broker.peers("probe-reviewer").len(), 1);
        drop(input);
        tokio::time::timeout(Duration::from_secs(5), child.wait()).await.unwrap().unwrap();
        server.broker.end(&terminal);
    }
}

#[test]
fn dormant_agents_wake_without_an_open_wait_call() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let mut issuer = register(&server.broker, "issuer", "codex");
    let mut workers = Vec::new();
    for (index, program) in managed_programs().iter().enumerate() {
        let terminal = format!("idle-{index}");
        let mut worker = register(&server.broker, &terminal, program);
        issuer.operation = Operation::TaskAssign {
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
            to: terminal,
            description: "Inspect code".into(),
            acceptance: "Report evidence".into(),
            reviewer: None,
            request_id: request_id(),
        };
        transport::call(&server.broker.endpoint, &issuer).unwrap();
        assert_eq!(server.broker.pending_work().len(), index + 1);
        assert!(
            server
                .broker
                .wakeups()
                .iter()
                .all(|wake| wake.terminal != worker.terminal),
            "Queued notification must not imply readiness"
        );
        worker.operation = Operation::AgentReady;
        transport::call(&server.broker.endpoint, &worker).unwrap();
        workers.push(worker);
    }
    thread::sleep(Duration::from_millis(850));
    let candidates = server.broker.wakeups();
    assert_eq!(candidates.len(), workers.len());
    for wake in &candidates {
        assert!(server.broker.claim_wake(wake));
        assert!(!server.broker.claim_wake(wake));
        assert!(server.broker.wake_valid(wake));
        server.broker.finish_wake(wake, true);
        assert!(!server.broker.wake_valid(wake));
    }
    assert!(server.broker.wakeups().is_empty());
    for worker in &mut workers {
        worker.operation = Operation::AgentInbox {
            cursor: None,
            limit: None,
        };
        let inbox = transport::call(&server.broker.endpoint, worker).unwrap();
        assert_eq!(
            inbox["messages"].as_array().unwrap().len(),
            1,
            "PTY submission must not acknowledge or discard work"
        );
    }
    let mut worker = workers[0].clone();
    // A failed delivery remains pending. Stop/busy events and manual drafts revoke the Enter lease.
    // Candidates are unordered; find this worker's message rather than relying on HashMap order.
    worker.operation = Operation::AgentAck {
        message_id: candidates
            .iter()
            .find(|wake| wake.terminal == worker.terminal)
            .unwrap()
            .message_id
            .clone(),
    };
    assert!(
        transport::call(&server.broker.endpoint, &worker).is_err(),
        "Acknowledging a task must not silently discard it"
    );
    worker.operation = Operation::AgentInbox {
        cursor: None,
        limit: None,
    };
    let inbox = transport::call(&server.broker.endpoint, &worker).unwrap();
    worker.operation = Operation::TaskStart {
        task_id: inbox["messages"][0]["task_id"]
            .as_str()
            .unwrap()
            .to_owned(),
        revision: 1,
        expected_version: None,
        request_id: request_id(),
    };
    transport::call(&server.broker.endpoint, &worker).unwrap();
    issuer.operation = Operation::AgentSend {
        to: worker.terminal.clone(),
        body: "Review follow-up".into(),
        subject: None,
        thread_id: None,
        reply_to: None,
        task_id: None,
        request_id: request_id(),
    };
    transport::call(&server.broker.endpoint, &issuer).unwrap();
    worker.operation = Operation::AgentReady;
    let readiness = transport::call(&server.broker.endpoint, &worker).unwrap();
    assert_eq!(readiness["ready"], false, "An executing task cannot announce readiness");
    thread::sleep(Duration::from_millis(850));
    assert!(server.broker.wakeups().is_empty());
    worker.operation = Operation::TaskSubmit {
        task_id: inbox["messages"][0]["task_id"].as_str().unwrap().to_owned(),
        revision: 1,
        result: "Inspected code".into(),
        evidence: "Focused inspection completed".into(),
        evidence_ids: vec![],
        expected_version: None,
        attempt_id: None,
        request_id: request_id(),
    };
    transport::call(&server.broker.endpoint, &worker).unwrap();
    worker.operation = Operation::AgentReady;
    server.broker.user_input(&worker.terminal, false);
    transport::call(&server.broker.endpoint, &worker).unwrap();
    thread::sleep(Duration::from_millis(850));
    assert!(
        server.broker.wakeups().is_empty(),
        "Readiness cannot override a user draft"
    );
    server.broker.user_input(&worker.terminal, true);
    transport::call(&server.broker.endpoint, &worker).unwrap();
    thread::sleep(Duration::from_millis(850));
    let wake = server.broker.wakeups().pop().unwrap();
    assert!(server.broker.claim_wake(&wake));
    server.broker.user_input(&worker.terminal, false);
    assert!(
        !server.broker.wake_valid(&wake),
        "Typing cancels delayed Enter"
    );
    server.broker.user_input(&worker.terminal, true);
    transport::call(&server.broker.endpoint, &worker).unwrap();
    thread::sleep(Duration::from_millis(850));
    let stale = server.broker.wakeups().pop().unwrap();
    server.broker.readiness(&worker.terminal, false);
    assert!(
        !server.broker.claim_wake(&stale),
        "Busy or permission-blocked terminals stay queued"
    );
    transport::call(&server.broker.endpoint, &worker).unwrap();
    thread::sleep(Duration::from_millis(850));
    let stale = server.broker.wakeups().pop().unwrap();
    assert!(server.broker.claim_wake(&stale));
    server
        .broker
        .activate(&worker.terminal, "custom", "/project", false)
        .unwrap();
    assert!(
        !server.broker.wake_valid(&stale),
        "A replacement run must never receive delayed Enter"
    );
    assert!(transport::call(&server.broker.endpoint, &worker).is_err());
}

#[test]
fn project_peers_communicate_without_selection_and_policy_revokes_runs() {
    use std::collections::HashSet;
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = &server.broker;
    let mut left = register(broker, "left", "codex");
    let mut right = register(broker, "right", "qoder");
    let mut third = register(broker, "third", "claude");
    left.operation = Operation::AgentList;
    let peers = transport::call(&broker.endpoint, &left).unwrap();
    assert_eq!(peers.as_array().unwrap().len(), 2);
    for recipient in ["right", "third"] {
        left.operation = Operation::AgentSend {
            to: recipient.into(),
            body: "No pairing required".into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: request_id(),
        };
        transport::call(&broker.endpoint, &left).unwrap();
    }
    right.operation = Operation::AgentInbox {
        cursor: None,
        limit: None,
    };
    assert_eq!(
        transport::call(&broker.endpoint, &right)
            .unwrap()["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    third.operation = Operation::AgentInbox {
        cursor: None,
        limit: None,
    };
    assert_eq!(
        transport::call(&broker.endpoint, &third)
            .unwrap()["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let capability = broker.prepare("outside").unwrap();
    broker
        .activate("outside", "codex", "/other-project", false)
        .unwrap();
    let mut outside = Request {
        protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
        terminal: "outside".into(),
        capability,
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: None,
        operation: Operation::AgentRegister {
            name: "outside".into(),
        },
    };
    let registered = transport::call(&broker.endpoint, &outside).unwrap();
    outside.run = registered["run"].as_str().map(str::to_owned);
    for to in ["outside", registered["agent"]["id"].as_str().unwrap()] {
        left.operation = Operation::AgentSend {
            to: to.into(),
            body: "Must stay isolated".into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: request_id(),
        };
        assert!(transport::call(&broker.endpoint, &left).is_err());
    }
    outside.operation = Operation::AgentSend {
        to: "left".into(),
        body: "Must stay isolated".into(),
        subject: None,
        thread_id: None,
        reply_to: None,
        task_id: None,
        request_id: request_id(),
    };
    assert!(transport::call(&broker.endpoint, &outside).is_err());
    left.operation = Operation::TaskAssign {
        dependencies: vec![],
        start_deadline: None,
        execution_timeout_seconds: None,
        review_timeout_seconds: None,
        to: "right".into(),
        description: "Inspect source".into(),
        acceptance: "Evidence".into(),
        reviewer: Some("third".into()),
        request_id: request_id(),
    };
    transport::call(&broker.endpoint, &left).unwrap();
    broker.set_programs(Some(HashSet::from(["codex".into()])));
    assert!(transport::call(&broker.endpoint, &right).is_err());
    assert!(broker
        .activate("right", "qoder", "/project", false)
        .is_err());
    left.operation = Operation::AgentList;
    assert!(transport::call(&broker.endpoint, &left)
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
    broker.set_programs(Some(HashSet::new()));
    assert!(transport::call(&broker.endpoint, &left).is_err());
    broker.set_programs(Some(HashSet::from(["codex".into()])));
    assert!(
        transport::call(&broker.endpoint, &left).is_err(),
        "Re-enable must not revive a stale run"
    );
    broker.activate("left", "codex", "/project", false).unwrap();
    assert!(
        transport::call(&broker.endpoint, &left).is_err(),
        "Fresh discovery is required"
    );
}

#[test]
fn concurrent_pool_claims_over_authenticated_ipc_have_one_winner() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = &server.broker;
    let mut issuer = register(broker, "issuer", "claude");
    let workers: Vec<_> = (0..20)
        .map(|index| register(broker, &format!("claimant-{index}"), "codex"))
        .collect();
    let eligible: Vec<_> = workers.iter().map(|worker| worker.terminal.clone()).collect();
    issuer.operation = serde_json::from_value(serde_json::json!({
        "op": "task_create_pool", "description": "Claim once", "acceptance": "Single owner",
        "eligible": eligible, "request_id": request_id()
    })).unwrap();
    let task = transport::call(&broker.endpoint, &issuer).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(workers.len() + 1));
    let children: Vec<_> = workers.into_iter().map(|mut worker| {
        worker.operation = serde_json::from_value(serde_json::json!({
            "op": "task_claim", "task_id": task["id"], "expected_version": task["version"],
            "request_id": request_id()
        })).unwrap();
        let endpoint = broker.endpoint.clone();
        let barrier = barrier.clone();
        thread::spawn(move || {
            barrier.wait();
            transport::call(&endpoint, &worker)
        })
    }).collect();
    barrier.wait();
    let outcomes: Vec<_> = children.into_iter().map(|child| child.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    issuer.operation = Operation::TaskGet { task_id: task["id"].as_str().unwrap().into() };
    let committed = transport::call(&broker.endpoint, &issuer).unwrap();
    assert_eq!(committed["version"], 2);
    assert!(!committed["assignee"].as_str().unwrap().is_empty());
}

#[test]
fn pool_wake_selects_one_idle_candidate_and_fences_stale_candidates() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = &server.broker;
    let mut issuer = register(broker, "issuer", "claude");
    let mut left = register(broker, "left", "codex");
    let mut right = register(broker, "right", "qodercn");
    issuer.operation = serde_json::from_value(serde_json::json!({
        "op":"task_create_pool", "description":"Wake one eligible model", "acceptance":"One claim",
        "eligible":["left","right"], "request_id":request_id()
    })).unwrap();
    let task = transport::call(&broker.endpoint, &issuer).unwrap();
    left.operation = Operation::AgentReady;
    right.operation = Operation::AgentReady;
    transport::call(&broker.endpoint, &left).unwrap();
    transport::call(&broker.endpoint, &right).unwrap();
    thread::sleep(Duration::from_millis(850));
    let candidates = broker.wakeups();
    assert_eq!(candidates.len(), 1);
    let earlier = candidates[0].clone();
    assert_eq!(earlier.terminal, "left", "Use durable notification order, not HashMap order");
    broker.user_input("left", false);
    let fallback = broker.wakeups().pop().unwrap();
    assert_eq!(fallback.terminal, "right", "Protected input must not block another eligible idle candidate");
    broker.user_input("left", true);
    transport::call(&broker.endpoint, &left).unwrap();
    thread::sleep(Duration::from_millis(850));
    let chosen = broker.wakeups().pop().unwrap();
    assert_eq!(chosen.terminal, "left");
    assert!(broker.claim_wake(&chosen));
    assert!(!broker.claim_wake(&fallback), "An older candidate cannot wake a second model");
    broker.finish_wake(&chosen, false);
    thread::sleep(Duration::from_millis(850));
    assert_eq!(broker.wakeups().len(), 1, "Failed submission stays eligible");
    let chosen = broker.wakeups().pop().unwrap();
    assert!(broker.claim_wake(&chosen));
    broker.finish_wake(&chosen, true);
    assert!(broker.wakeups().is_empty(), "Unacknowledged successful submission must not fan out");
    broker.end("left");
    assert_eq!(broker.wakeups()[0].terminal, "right", "An unavailable run releases pool delivery eligibility");
    right.operation = serde_json::from_value(serde_json::json!({
        "op":"task_claim", "task_id":task["id"], "expected_version":task["version"], "request_id":request_id()
    })).unwrap();
    transport::call(&broker.endpoint, &right).unwrap();
    let wake = broker.wakeups().pop().unwrap();
    assert_eq!(wake.terminal, "right");
    assert!(broker.claim_wake(&wake), "Claimed assignment replaces the retired pool notices");
}

#[test]
fn competing_task_controls_preserve_versions_in_both_orders() {
    use serde_json::json;
    for scenario in ["start_cancel", "submit_cancel", "review_retry"] {
        for reverse in [false, true] {
            let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
            let mut issuer = register(&server.broker, "issuer", "claude");
            let mut worker = register(&server.broker, "worker", "codex");
            issuer.operation = serde_json::from_value(json!({"op":"task_assign", "to":"worker",
                "description":"Control race", "acceptance":"Keep one owner", "request_id":request_id()})).unwrap();
            let mut task = transport::call(&server.broker.endpoint, &issuer).unwrap();
            if scenario != "start_cancel" {
                worker.operation = serde_json::from_value(json!({"op":"task_start", "task_id":task["id"],
                    "revision":1, "request_id":request_id()})).unwrap();
                task = transport::call(&server.broker.endpoint, &worker).unwrap();
            }
            if scenario == "review_retry" {
                worker.operation = serde_json::from_value(json!({"op":"task_submit", "task_id":task["id"],
                    "revision":1, "result":"Complete", "evidence":"Checked", "request_id":request_id()})).unwrap();
                task = transport::call(&server.broker.endpoint, &worker).unwrap();
            }
            let mut first = if scenario == "review_retry" { issuer.clone() } else { worker.clone() };
            let mut second = issuer.clone();
            let id = task["id"].clone();
            let version = task["version"].clone();
            first.operation = serde_json::from_value(match scenario {
                "start_cancel" => json!({"op":"task_start", "task_id":id, "revision":1,
                    "expected_version":version, "request_id":request_id()}),
                "submit_cancel" => json!({"op":"task_submit", "task_id":id, "revision":1,
                    "result":"Complete", "evidence":"Checked", "expected_version":version, "request_id":request_id()}),
                _ => json!({"op":"task_review", "task_id":id, "revision":1, "accepted":false,
                    "feedback":"Recheck", "expected_version":version, "request_id":request_id()}),
            }).unwrap();
            second.operation = serde_json::from_value(if scenario == "review_retry" {
                json!({"op":"task_retry", "task_id":id, "reason":"Retry",
                    "expected_version":version, "request_id":request_id()})
            } else {
                json!({"op":"task_cancel", "task_id":id, "reason":"Stop",
                    "expected_version":version, "request_id":request_id()})
            }).unwrap();
            let requests = if reverse { [&second, &first] } else { [&first, &second] };
            let outcomes: Vec<_> = requests.iter()
                .map(|request| transport::call(&server.broker.endpoint, request)).collect();
            assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1,
                "{scenario}, reverse={reverse}");
            issuer.operation = Operation::TaskGet { task_id: id.as_str().unwrap().into() };
            let current = transport::call(&server.broker.endpoint, &issuer).unwrap();
            assert_eq!(current["version"].as_u64(), Some(version.as_u64().unwrap() + 1));
            let expected = match (scenario, reverse) {
                ("start_cancel", false) => "running",
                ("start_cancel", true) => "cancelled",
                ("submit_cancel", false) => "submitted",
                ("submit_cancel", true) => "cancel_requested",
                _ => "queued",
            };
            assert_eq!(current["state"], expected);
            // Both a winning replay and a rejected stale intent leave current state unchanged.
            for request in requests {
                let _ = transport::call(&server.broker.endpoint, request);
            }
            assert_eq!(transport::call(&server.broker.endpoint, &issuer).unwrap(), current);
        }
    }
}

#[test]
fn prerequisite_acceptance_and_dependent_start_serialize_over_authenticated_ipc() {
    use serde_json::json;
    for order in ["start_first", "review_first", "concurrent"] {
        let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
        let endpoint = &server.broker.endpoint;
        let mut issuer = register(&server.broker, "issuer", "claude");
        let mut prerequisite_worker = register(&server.broker, "prerequisite", "codex");
        let mut dependent_worker = register(&server.broker, "dependent", "codex");
        issuer.operation = serde_json::from_value(json!({"op":"task_assign", "to":"prerequisite",
            "description":"Prepare input", "acceptance":"Reviewed input", "request_id":request_id()})).unwrap();
        let prerequisite = transport::call(endpoint, &issuer).unwrap();
        prerequisite_worker.operation = serde_json::from_value(json!({"op":"task_start",
            "task_id":prerequisite["id"], "revision":1, "request_id":request_id()}))
        .unwrap();
        transport::call(endpoint, &prerequisite_worker).unwrap();
        prerequisite_worker.operation = serde_json::from_value(json!({"op":"task_submit",
            "task_id":prerequisite["id"], "revision":1, "result":"Prepared", "evidence":"Checked",
            "request_id":request_id()}))
        .unwrap();
        let submitted = transport::call(endpoint, &prerequisite_worker).unwrap();
        issuer.operation = serde_json::from_value(json!({"op":"task_assign", "to":"dependent",
            "description":"Use input", "acceptance":"Depends on accepted input",
            "dependencies":[prerequisite["id"]], "request_id":request_id()}))
        .unwrap();
        let dependent = transport::call(endpoint, &issuer).unwrap();
        issuer.operation = serde_json::from_value(json!({"op":"task_review",
            "task_id":prerequisite["id"], "revision":1, "accepted":true, "feedback":"Accepted",
            "expected_version":submitted["version"], "request_id":request_id()}))
        .unwrap();
        dependent_worker.operation = serde_json::from_value(json!({"op":"task_start",
            "task_id":dependent["id"], "revision":1, "expected_version":dependent["version"],
            "request_id":request_id()}))
        .unwrap();
        let outcome = match order {
            "start_first" => {
                let outcome = transport::call(endpoint, &dependent_worker);
                assert!(outcome.is_err(), "Submitted input is not accepted input");
                transport::call(endpoint, &issuer).unwrap();
                outcome
            }
            "review_first" => {
                transport::call(endpoint, &issuer).unwrap();
                let outcome = transport::call(endpoint, &dependent_worker);
                assert!(outcome.is_err(), "Acceptance must fence the old blocked version");
                outcome
            }
            _ => {
                let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
                let workers: Vec<_> = [issuer.clone(), dependent_worker.clone()]
                    .into_iter()
                    .map(|request| {
                        let endpoint = endpoint.clone();
                        let barrier = barrier.clone();
                        thread::spawn(move || {
                            barrier.wait();
                            transport::call(&endpoint, &request)
                        })
                    })
                    .collect();
                barrier.wait();
                let mut outcomes = workers.into_iter().map(|worker| worker.join().unwrap());
                outcomes.next().unwrap().unwrap();
                outcomes.next().unwrap()
            }
        };
        issuer.operation = Operation::TaskGet {
            task_id: dependent["id"].as_str().unwrap().into(),
        };
        let before_retry = transport::call(endpoint, &issuer).unwrap();
        assert!(outcome.is_err());
        assert_eq!(before_retry["state"], "queued");
        assert_eq!(before_retry["version"].as_u64(), Some(dependent["version"].as_u64().unwrap() + 1));
        assert!(before_retry["attempts"].as_array().unwrap().is_empty());
        assert!(transport::call(endpoint, &dependent_worker).is_err());
        // Acceptance changes the dependent version; only a refreshed intent may start.
        dependent_worker.operation = serde_json::from_value(json!({"op":"task_start",
            "task_id":dependent["id"], "revision":1, "expected_version":before_retry["version"],
            "request_id":request_id()})).unwrap();
        let started = transport::call(endpoint, &dependent_worker).unwrap();
        assert_eq!(started["state"], "running");
        assert_eq!(started["version"], 3);
        assert_eq!(started["attempts"].as_array().unwrap().len(), 1);
        assert_eq!(
            transport::call(endpoint, &dependent_worker).unwrap(),
            started
        );
    }
}

#[test]
fn project_roots_share_subdirectories_but_isolate_worktrees() {
    let root = std::env::temp_dir().join(format!("warp-project-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("repo/.git")).unwrap();
    std::fs::create_dir_all(root.join("repo/subdir")).unwrap();
    std::fs::create_dir_all(root.join("worktree")).unwrap();
    std::fs::write(root.join("worktree/.git"), "gitdir: elsewhere").unwrap();
    assert_eq!(
        warp_agent_bus::project_root(&root.join("repo")).unwrap(),
        warp_agent_bus::project_root(&root.join("repo/subdir")).unwrap()
    );
    assert_ne!(
        warp_agent_bus::project_root(&root.join("repo")).unwrap(),
        warp_agent_bus::project_root(&root.join("worktree")).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_receiver_state_preserves_work_and_initial_readiness_is_one_shot() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = &server.broker;
    let mut issuer = register(broker, "matrix-issuer", "codex");
    let mut expected = Vec::new();
    let mut workers = Vec::new();
    for (index, program) in managed_programs().iter().enumerate() {
        for state in [
            "fresh",
            "arguments",
            "busy",
            "draft",
            "cancelled",
            "active",
            "rediscovered",
        ] {
            let name = format!("matrix-{index}-{state}");
            let capability = broker.prepare(&name).unwrap();
            broker
                .activate(&name, program, "/project", state != "arguments")
                .unwrap();
            match state {
                "busy" => broker.readiness(&name, false),
                "draft" | "cancelled" => broker.user_input(&name, false),
                _ => {}
            }
            let mut worker = Request {
                protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
                terminal: name.clone(),
                capability,
                run: None,
                defer_initial_ready: false,
                native_activity: None,
                directory: None,
                operation: Operation::AgentRegister { name: name.clone() },
            };
            let registered = transport::call(&broker.endpoint, &worker).unwrap();
            worker.run = registered["run"].as_str().map(str::to_owned);
            if matches!(state, "active" | "rediscovered") {
                worker.operation = Operation::AgentList;
                transport::call(&broker.endpoint, &worker).unwrap();
                worker.operation = Operation::AgentRegister {
                    name: String::new(),
                };
                transport::call(&broker.endpoint, &worker).unwrap();
            }
            issuer.operation = Operation::AgentSend {
                to: name.clone(),
                body: "Execute in your existing terminal and reply".into(),
                subject: None,
                thread_id: None,
                reply_to: None,
                task_id: None,
                request_id: request_id(),
            };
            assert_eq!(
                transport::call(&broker.endpoint, &issuer).unwrap()["delivery"],
                "queued"
            );
            if matches!(state, "fresh" | "active" | "rediscovered") {
                expected.push(name);
            }
            workers.push(worker);
        }
    }
    thread::sleep(Duration::from_millis(850));
    let wakes = broker.wakeups();
    assert_eq!(wakes.len(), expected.len());
    for wake in &wakes {
        assert!(expected.contains(&wake.terminal));
        assert!(broker.claim_wake(wake));
        assert!(!broker.claim_wake(wake));
        broker.finish_wake(wake, true);
    }
    assert!(broker.wakeups().is_empty());
    for worker in &mut workers {
        worker.operation = Operation::AgentInbox {
            cursor: None,
            limit: None,
        };
        let inbox = transport::call(&broker.endpoint, worker).unwrap();
        assert_eq!(
            inbox["messages"].as_array().unwrap().len(),
            1,
            "Blocked or submitted work must remain unacknowledged"
        );
    }
    let name = "not-discovered";
    broker.prepare(name).unwrap();
    broker.activate(name, "codex", "/project", true).unwrap();
    issuer.operation = Operation::AgentSend {
        to: name.into(),
        body: "Cannot bypass discovery".into(),
        subject: None,
        thread_id: None,
        reply_to: None,
        task_id: None,
        request_id: request_id(),
    };
    assert!(transport::call(&broker.endpoint, &issuer).is_err());
}

#[test]
fn coordinator_tracks_every_worker_and_recovers_interrupted_discovered_identity() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let broker = &server.broker;
    let mut issuer = register(broker, "coordinator", "codex");
    let mut workers = Vec::new();
    for (index, program) in managed_programs().iter().enumerate() {
        let worker = register(broker, &format!("child-{index}"), program);
        issuer.operation = Operation::TaskAssign {
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
            to: worker.terminal.clone(),
            description: "Perform delegated work".into(),
            acceptance: "Submit result and evidence".into(),
            reviewer: None,
            request_id: request_id(),
        };
        let task = transport::call(&broker.endpoint, &issuer).unwrap();
        workers.push((worker, task["id"].as_str().unwrap().to_owned()));
    }
    for (worker, task_id) in &mut workers {
        issuer.operation = Operation::AgentList;
        let peers = transport::call(&broker.endpoint, &issuer).unwrap();
        let peer = peers
            .as_array()
            .unwrap()
            .iter()
            .find(|peer| peer["name"] == worker.terminal)
            .unwrap();
        assert_eq!(peer["tasks"][0]["state"], "queued");
        worker.operation = Operation::TaskStart {
            task_id: task_id.clone(),
            revision: 1,
            expected_version: None,
            request_id: request_id(),
        };
        transport::call(&broker.endpoint, worker).unwrap();
        issuer.operation = Operation::TaskGet {
            task_id: task_id.clone(),
        };
        let running = transport::call(&broker.endpoint, &issuer).unwrap();
        assert_eq!(running["state"], "running");
        assert_eq!(running["interrupted"], false);
        worker.operation = Operation::TaskSubmit {
            evidence_ids: vec![],
            task_id: task_id.clone(),
            revision: 1,
            result: "Normal terminal report".into(),
            evidence: "Representative check passed".into(),
            expected_version: None,
            attempt_id: None,
            request_id: request_id(),
        };
        transport::call(&broker.endpoint, worker).unwrap();
        let submitted = transport::call(&broker.endpoint, &issuer).unwrap();
        assert_eq!(submitted["state"], "submitted");
        assert_eq!(submitted["result"], "Normal terminal report");
        issuer.operation = Operation::TaskReview {
            task_id: task_id.clone(),
            revision: 1,
            accepted: true,
            feedback: "Verified".into(),
            expected_version: None,
            request_id: request_id(),
        };
        transport::call(&broker.endpoint, &issuer).unwrap();
    }
    let mut original = register(broker, "recoverable", "codex");
    issuer.operation = Operation::TaskAssign {
        dependencies: vec![],
        start_deadline: None,
        execution_timeout_seconds: None,
        review_timeout_seconds: None,
        to: "recoverable".into(),
        description: "Interrupted work".into(),
        acceptance: "Recover explicitly".into(),
        reviewer: None,
        request_id: request_id(),
    };
    let assigned = transport::call(&broker.endpoint, &issuer).unwrap();
    let id = assigned["id"].as_str().unwrap().to_owned();
    original.operation = Operation::TaskStart {
        task_id: id.clone(),
        revision: 1,
        expected_version: None,
        request_id: request_id(),
    };
    transport::call(&broker.endpoint, &original).unwrap();
    broker.end("recoverable");
    issuer.operation = Operation::TaskGet {
        task_id: id.clone(),
    };
    let interrupted = transport::call(&broker.endpoint, &issuer).unwrap();
    assert_eq!(interrupted["assignee_online"], false);
    assert_eq!(interrupted["interrupted"], true);
    let mut replacement = register(broker, "new-pane", "codex");
    replacement.operation = Operation::AgentRegister {
        name: "recoverable".into(),
    };
    transport::call(&broker.endpoint, &replacement).unwrap();
    replacement.operation = Operation::TaskSubmit {
        evidence_ids: vec![],
        task_id: id.clone(),
        revision: 1,
        result: "Must not silently complete".into(),
        evidence: "Not recovered".into(),
        expected_version: None,
        attempt_id: None,
        request_id: request_id(),
    };
    assert!(transport::call(&broker.endpoint, &replacement).is_err());
    replacement.operation = Operation::TaskStart {
        task_id: id.clone(),
        revision: 1,
        expected_version: None,
        request_id: request_id(),
    };
    let error = transport::call(&broker.endpoint, &replacement).unwrap_err();
    assert_eq!(error.downcast_ref::<warp_agent_bus::DomainError>().unwrap().code, "execution_unknown");
    broker.control("/project", &warp_agent_bus::ControllerOperation::TaskForceCancel {
        task_id: id.clone(), reason: "Operator accepts unknown execution risk".into(),
        expected_version: None, request_id: request_id()
    }).unwrap();
    issuer.operation = serde_json::from_value(serde_json::json!({"op":"task_retry", "task_id":id,
        "reason":"Explicit recovery", "request_id":request_id()})).unwrap();
    let error = transport::call(&broker.endpoint, &issuer).unwrap_err();
    assert_eq!(error.downcast_ref::<warp_agent_bus::DomainError>().unwrap().code, "execution_unknown");
    let retry = serde_json::from_value(serde_json::json!({"op":"task_retry", "task_id":id,
        "reason":"Operator authorizes replacement despite unknown effects", "override_uncertain":true,
        "request_id":request_id()})).unwrap();
    broker.operator("/project", &retry).unwrap();
    replacement.operation = Operation::TaskStart {
        task_id: id.clone(), revision: 2, expected_version: None, request_id: request_id()
    };
    transport::call(&broker.endpoint, &replacement).unwrap();
    issuer.operation = Operation::TaskGet { task_id: id.clone() };
    assert_eq!(
        transport::call(&broker.endpoint, &issuer).unwrap()["interrupted"],
        false
    );
    replacement.operation = Operation::AgentRegister {
        name: "different-identity".into(),
    };
    assert!(
        transport::call(&broker.endpoint, &replacement).is_err(),
        "A receiver with work cannot abandon its identity"
    );
    assert!(transport::call(&broker.endpoint, &original).is_err());
}
#[tokio::test]
async fn native_discovery_waits_for_terminal_activation_without_reviving_stale_runs() {
    use rmcp::{transport::TokioChildProcess, ServiceExt};
    use warp_agent_bus::transport::{CAPABILITY, ENDPOINT, TERMINAL};
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let capability = server.broker.prepare("starting").unwrap();
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_warp-agent"));
    command
        .arg("mcp")
        .env(ENDPOINT, &server.broker.endpoint)
        .env(CAPABILITY, capability)
        .env(TERMINAL, "starting");
    let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    let broker = server.broker.clone();
    let activation = tokio::task::spawn_blocking(move || {
        thread::sleep(Duration::from_millis(250));
        broker
            .activate("starting", "codex", "/project", true)
            .unwrap();
    });
    assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 30);
    activation.await.unwrap();
    server.broker.end("starting");
    server
        .broker
        .activate("starting", "codex", "/project", true)
        .unwrap();
    assert!(
        client.list_tools(None).await.is_err(),
        "A discovered old bridge must not adopt a replacement run"
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn retired_remote_stdio_refuses_even_with_a_legacy_descriptor() {
    // Compatibility refusal never discovers a daily controller or accepts old credentials.
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_warp-agent"));
    command.arg("remote-stdio")
        .env("WARP_DATA_PROFILE", format!("gateway-test-{}", Uuid::new_v4()))
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(10), command.output())
        .await.unwrap().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.len() > 4 && output.stdout.len() <= 4096);
    let size = u32::from_be_bytes(output.stdout[..4].try_into().unwrap()) as usize;
    assert_eq!(output.stdout.len(), size + 4);
    let frame: Value = serde_json::from_slice(&output.stdout[4..]).unwrap();
    assert_eq!(frame["type"], "error");
    assert_eq!(frame["error"]["code"], "feature_unavailable");
    assert!(!frame["error"]["retryable"].as_bool().unwrap());
    assert!(!frame.to_string().contains("credential"));
}
