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
        let mut store = Store::open(path.to_str().unwrap()).unwrap();
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
            assert!(store.next_work(&worker, "worker-run").is_some());
            store
                .execute(
                    &worker,
                    "worker-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 1,
                        request_id: request_id(),
                    },
                )
                .unwrap();
            store.recover(&worker, "replacement-run").unwrap();
            assert!(store.next_work(&worker, "replacement-run").is_some());
            store
                .execute(
                    &worker,
                    "replacement-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 1,
                        request_id: request_id(),
                    },
                )
                .unwrap();
            let submission = Operation::TaskSubmit {
                task_id: id.clone(),
                revision: 1,
                result: "Implemented".into(),
                evidence: "Focused test passed".into(),
                request_id: request_id(),
            };
            assert!(store.execute(&worker, "worker-run", &submission).is_err());
            store
                .execute(&worker, "replacement-run", &submission)
                .unwrap();
            let review = Operation::TaskReview {
                task_id: id.clone(),
                revision: 1,
                accepted: false,
                feedback: "Cover the failure case".into(),
                request_id: request_id(),
            };
            assert!(store.execute(&worker, "replacement-run", &review).is_err());
            let rework = store.execute(&reviewer, "review-run", &review).unwrap();
            assert_eq!(rework["revision"], 2);
            assert!(
                store
                    .execute(&worker, "replacement-run", &submission)
                    .unwrap()["state"]
                    == "submitted"
            ); // Identical retry returns its original response.
            assert!(store
                .execute(
                    &worker,
                    "replacement-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 1,
                        request_id: request_id()
                    }
                )
                .is_err());
            store
                .execute(
                    &worker,
                    "replacement-run",
                    &Operation::TaskStart {
                        task_id: id.clone(),
                        revision: 2,
                        request_id: request_id(),
                    },
                )
                .unwrap();
            store
                .execute(
                    &worker,
                    "replacement-run",
                    &Operation::TaskSubmit {
                        task_id: id.clone(),
                        revision: 2,
                        result: "Failure case covered".into(),
                        evidence: "Failure simulation passed".into(),
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
        assert!(store.pending(&moved).is_empty());
        let inbox = store
            .execute(&reviewer, "review-run", &Operation::AgentList)
            .unwrap();
        assert_eq!(inbox.as_array().unwrap().len(), programs.len() + 1);
    }
    let mut reopened = Store::open(path.to_str().unwrap()).unwrap();
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
        terminal: terminal.into(),
        capability,
        run: None,
        defer_initial_ready: false,
        directory: None,
        operation: Operation::AgentRegister {
            name: terminal.into(),
        },
    };
    let result = transport::call(&broker.endpoint, &request).unwrap();
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
            terminal: state.into(),
            capability,
            run: None,
            defer_initial_ready: state == "native-context",
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
                terminal: "native-issuer".into(),
                capability,
                run: None,
                defer_initial_ready: false,
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
        terminal: "claimant".into(),
        capability: claim_capability,
        run: None,
        defer_initial_ready: false,
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
    assert_eq!(schema.len(), 12);
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
        assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 12);
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

#[test]
fn dormant_agents_wake_without_an_open_wait_call() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let mut issuer = register(&server.broker, "issuer", "codex");
    let mut workers = Vec::new();
    for (index, program) in managed_programs().iter().enumerate() {
        let terminal = format!("idle-{index}");
        let mut worker = register(&server.broker, &terminal, program);
        issuer.operation = Operation::TaskAssign {
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
        worker.operation = Operation::AgentInbox;
        let inbox = transport::call(&server.broker.endpoint, worker).unwrap();
        assert_eq!(
            inbox.as_array().unwrap().len(),
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
    worker.operation = Operation::AgentInbox;
    let inbox = transport::call(&server.broker.endpoint, &worker).unwrap();
    worker.operation = Operation::TaskStart {
        task_id: inbox[0]["task_id"].as_str().unwrap().to_owned(),
        revision: 1,
        request_id: request_id(),
    };
    transport::call(&server.broker.endpoint, &worker).unwrap();
    issuer.operation = Operation::AgentSend {
        to: worker.terminal.clone(),
        body: "Review follow-up".into(),
        request_id: request_id(),
    };
    transport::call(&server.broker.endpoint, &issuer).unwrap();
    worker.operation = Operation::AgentReady;
    transport::call(&server.broker.endpoint, &worker).unwrap();
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
            request_id: request_id(),
        };
        transport::call(&broker.endpoint, &left).unwrap();
    }
    right.operation = Operation::AgentInbox;
    assert_eq!(
        transport::call(&broker.endpoint, &right)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    third.operation = Operation::AgentInbox;
    assert_eq!(
        transport::call(&broker.endpoint, &third)
            .unwrap()
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
        terminal: "outside".into(),
        capability,
        run: None,
        defer_initial_ready: false,
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
            request_id: request_id(),
        };
        assert!(transport::call(&broker.endpoint, &left).is_err());
    }
    outside.operation = Operation::AgentSend {
        to: "left".into(),
        body: "Must stay isolated".into(),
        request_id: request_id(),
    };
    assert!(transport::call(&broker.endpoint, &outside).is_err());
    left.operation = Operation::TaskAssign {
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
                terminal: name.clone(),
                capability,
                run: None,
                defer_initial_ready: false,
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
                request_id: request_id(),
            };
            assert_eq!(
                transport::call(&broker.endpoint, &issuer).unwrap()["delivery"],
                "queued"
            );
            if state == "fresh" {
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
        worker.operation = Operation::AgentInbox;
        let inbox = transport::call(&broker.endpoint, worker).unwrap();
        assert_eq!(
            inbox.as_array().unwrap().len(),
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
            task_id: task_id.clone(),
            revision: 1,
            result: "Normal terminal report".into(),
            evidence: "Representative check passed".into(),
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
            request_id: request_id(),
        };
        transport::call(&broker.endpoint, &issuer).unwrap();
    }
    let mut original = register(broker, "recoverable", "codex");
    issuer.operation = Operation::TaskAssign {
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
        task_id: id.clone(),
        revision: 1,
        result: "Must not silently complete".into(),
        evidence: "Not recovered".into(),
        request_id: request_id(),
    };
    assert!(transport::call(&broker.endpoint, &replacement).is_err());
    replacement.operation = Operation::TaskStart {
        task_id: id.clone(),
        revision: 1,
        request_id: request_id(),
    };
    transport::call(&broker.endpoint, &replacement).unwrap();
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
    assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 12);
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
