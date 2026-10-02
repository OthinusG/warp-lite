//! Real local IPC checks; no vendor model, SSH session or user profile is involved.
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use warp_agent_bus::{
    transport::{self, Broker, Request, RunningBroker},
    ControllerOperation, Operation,
};

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn call(broker: &Broker, client: &Request, operation: Operation) -> anyhow::Result<Value> {
    let mut request = client.clone();
    request.operation = operation;
    transport::call(&broker.endpoint, &request)
}
fn space(broker: &Broker, root: &str, name: &str) -> String {
    broker
        .control(
            root,
            &ControllerOperation::SpaceCreate {
                name: name.into(),
                request_id: id(),
            },
        )
        .unwrap()["space_id"]
        .as_str()
        .unwrap()
        .into()
}
fn map(broker: &Broker, root: &str, space: &str) -> String {
    broker
        .control(
            root,
            &ControllerOperation::WorkspaceMap {
                space_id: space.into(),
                root: root.into(),
                repository_id: None,
                model: "same_branch".into(),
                branch: None,
                base_commit: None,
                request_id: id(),
            },
        )
        .unwrap()["workspace_id"]
        .as_str()
        .unwrap()
        .into()
}
fn client(
    broker: &Broker,
    terminal: &str,
    name: &str,
    root: &str,
    workspace: Option<&str>,
) -> Request {
    let capability = match workspace {
        Some(workspace) => broker.prepare_in_workspace(terminal, workspace),
        None => broker.prepare(terminal),
    }
    .unwrap();
    broker.activate(terminal, "codex", root, true).unwrap();
    let mut request = Request {
        protocol_major: transport::PROTOCOL_MAJOR,
        terminal: terminal.into(),
        capability,
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: Some(root.into()),
        operation: Operation::AgentRegister { name: name.into() },
    };
    request.run = transport::call(&broker.endpoint, &request).unwrap()["run"]
        .as_str()
        .map(str::to_owned);
    request
}
fn assign(broker: &Broker, from: &Request, to: &str) -> String {
    call(
        broker,
        from,
        Operation::TaskAssign {
            to: to.into(),
            reviewer: None,
            description: "Edit the owned source fixture".into(),
            acceptance: "Fixture check passes".into(),
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
            dependencies: vec![],
            request_id: id(),
        },
    )
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn reserve(path: &str) -> Operation {
    Operation::FileReserve {
        paths: vec![path.into()],
        mode: "exclusive".into(),
        task_id: None,
        attempt_id: None,
        ttl_seconds: None,
        request_id: id(),
    }
}
fn leases() -> Operation {
    Operation::FileReservations {
        path: None,
        cursor: None,
        limit: None,
        include_expired: false,
    }
}

#[test]
fn shared_ipc_preserves_private_work_and_original_evidence_checkout() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let root1 = warp_agent_bus::project_root(first.path()).unwrap();
    let root2 = warp_agent_bus::project_root(second.path()).unwrap();
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let b = &server.broker;
    let private_issuer = client(b, "private-issuer", "issuer", &root1, None);
    let private_worker = client(b, "private-worker", "worker", &root1, None);
    let private_task = assign(b, &private_issuer, "worker");
    let old_attempt = call(
        b,
        &private_worker,
        Operation::TaskStart {
            task_id: private_task.clone(),
            revision: 1,
            expected_version: None,
            request_id: id(),
        },
    )
    .unwrap();
    let group = space(b, &root1, "Shared fixtures");
    let ws1 = map(b, &root1, &group);
    let ws2 = map(b, &root2, &group);
    // Metadata alone does not join either old pane, nor does it allow rebinding its capability.
    assert!(b.prepare_in_workspace("private-worker", &ws1).is_err());
    let issuer = client(b, "shared-issuer", "issuer", &root1, Some(&ws1));
    let worker = client(b, "shared-worker", "worker", &root2, Some(&ws2));
    let domain = format!("space:{group}");
    assert_eq!(
        call(b, &issuer, Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        call(b, &private_issuer, Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(call(
        b,
        &worker,
        Operation::TaskGet {
            task_id: private_task.clone()
        }
    )
    .is_err());
    let task = assign(b, &issuer, "worker");
    assert!(call(
        b,
        &private_issuer,
        Operation::TaskGet {
            task_id: task.clone()
        }
    )
    .is_err());
    std::fs::write(second.path().join("source.txt"), b"owned fixture").unwrap();
    std::fs::write(first.path().join("source.txt"), b"different checkout").unwrap();
    for revision in 1..=2 {
        call(
            b,
            &worker,
            Operation::TaskStart {
                task_id: task.clone(),
                revision,
                expected_version: None,
                request_id: id(),
            },
        )
        .unwrap();
        let evidence = call(
            b,
            &worker,
            Operation::EvidenceAdd {
                task_id: task.clone(),
                kind: "file".into(),
                attempt_id: None,
                path: Some("source.txt".into()),
                hash: Some(format!("{:x}", Sha256::digest(b"owned fixture"))),
                commit: None,
                repository: None,
                branch: None,
                base: None,
                head: None,
                command: None,
                outcome: None,
                exit_code: None,
                summary: None,
                request_id: id(),
            },
        )
        .unwrap();
        let evidence_id = evidence["evidence_id"].as_str().unwrap();
        assert_eq!(
            b.control(
                &domain,
                &ControllerOperation::EvidenceVerify {
                    evidence_id: evidence_id.into(),
                    verified: true,
                    request_id: id(),
                }
            )
            .unwrap()["verified"],
            true
        );
        call(
            b,
            &worker,
            Operation::TaskSubmit {
                task_id: task.clone(),
                revision,
                result: "Owned fixture edited".into(),
                evidence_ids: vec![evidence_id.into()],
                evidence: "File hash verified".into(),
                expected_version: None,
                attempt_id: None,
                request_id: id(),
            },
        )
        .unwrap();
        call(
            b,
            &issuer,
            Operation::TaskReview {
                task_id: task.clone(),
                revision,
                accepted: revision == 2,
                feedback: "Check the failure case".into(),
                expected_version: None,
                request_id: id(),
            },
        )
        .unwrap();
    }
    assert_eq!(
        call(b, &issuer, Operation::TaskGet { task_id: task }).unwrap()["state"],
        "accepted"
    );
    let unchanged = call(
        b,
        &private_worker,
        Operation::TaskGet {
            task_id: private_task,
        },
    )
    .unwrap();
    assert_eq!(unchanged["state"], "running");
    assert_eq!(unchanged["attempts"], old_attempt["attempts"]);
    // Same physical checkout excludes conflicting private work without revealing its identity.
    call(b, &private_worker, reserve("private.txt")).unwrap();
    let conflict = call(b, &issuer, reserve("private.txt"))
        .unwrap_err()
        .to_string();
    assert!(!conflict.contains("private-worker"));
    let shared_lease = call(b, &issuer, reserve("shared.txt")).unwrap();
    assert_eq!(
        call(b, &issuer, leases()).unwrap()["reservations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let private_leases = call(b, &private_worker, leases()).unwrap();
    assert_eq!(private_leases["reservations"].as_array().unwrap().len(), 1);
    assert_eq!(private_leases["reservations"][0]["path"], "private.txt");
    call(
        b,
        &issuer,
        Operation::FileRenew {
            reservation_ids: vec![shared_lease["reservation_ids"][0].as_str().unwrap().into()],
            ttl_seconds: Some(60),
            request_id: id(),
        },
    )
    .unwrap();
}

#[test]
fn shared_capabilities_fence_directory_reclaim_departure_and_remapping() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let root1 = warp_agent_bus::project_root(first.path()).unwrap();
    let root2 = warp_agent_bus::project_root(second.path()).unwrap();
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let b = &server.broker;
    let group = space(b, &root1, "Shared");
    let ws1 = map(b, &root1, &group);
    let ws2 = map(b, &root2, &group);
    let owner = client(b, "owner", "owner", &root1, Some(&ws1));
    let peer = client(b, "peer", "peer", &root2, Some(&ws2));
    let domain = format!("space:{group}");
    let capability = b.prepare_in_workspace("drift", &ws1).unwrap();
    assert!(b.activate("drift", "codex", &root2, false).is_err());
    b.activate("drift", "codex", &root1, false).unwrap();
    let mut drift = owner.clone();
    drift.terminal = "drift".into();
    drift.capability = capability;
    drift.run = None;
    drift.directory = Some(root2.clone());
    drift.operation = Operation::AgentRegister {
        name: "drift".into(),
    };
    assert!(transport::call(&b.endpoint, &drift).is_err());
    let replay = reserve("keep.txt");
    call(b, &peer, replay.clone()).unwrap();
    let task = assign(b, &owner, "peer");
    call(
        b,
        &peer,
        Operation::TaskStart {
            task_id: task.clone(),
            revision: 1,
            expected_version: None,
            request_id: id(),
        },
    )
    .unwrap();
    b.control(
        &domain,
        &ControllerOperation::SpaceLeave {
            space_id: group.clone(),
            agent: "peer".into(),
            request_id: id(),
        },
    )
    .unwrap();
    assert!(call(b, &peer, replay).is_err());
    assert!(call(
        b,
        &peer,
        Operation::AgentRegister {
            name: "peer".into()
        }
    )
    .is_err());
    assert!(b.activate("peer", "codex", &root2, true).is_err());
    assert!(b.peers("owner").is_empty());
    assert_eq!(b.operator_task(&domain, &task).unwrap()["state"], "running");
    // Offline identities cannot be stolen by another physical checkout.
    let wrong_cap = b.prepare_in_workspace("wrong-reclaim", &ws1).unwrap();
    b.activate("wrong-reclaim", "codex", &root1, false).unwrap();
    let mut wrong = owner.clone();
    wrong.terminal = "wrong-reclaim".into();
    wrong.capability = wrong_cap;
    wrong.run = None;
    assert!(call(
        b,
        &wrong,
        Operation::AgentRegister {
            name: "peer".into()
        }
    )
    .is_err());
    // An explicit new admission in the original checkout can recover its uncertain attempt.
    let recovered = client(b, "recovered", "peer", &root2, Some(&ws2));
    assert_eq!(
        call(
            b,
            &recovered,
            Operation::TaskGet {
                task_id: task.clone()
            }
        )
        .unwrap()["state"],
        "running"
    );
    let other = space(b, &root1, "Other");
    map(b, &root2, &other);
    map(b, &root2, &group);
    assert!(call(b, &recovered, Operation::AgentList).is_err());
    assert!(b.activate("recovered", "codex", &root2, true).is_err());
    let other_ws = map(b, &root2, &other);
    let independent = client(b, "other-owner", "owner", &root2, Some(&other_ws));
    assert_eq!(
        call(b, &independent, Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
