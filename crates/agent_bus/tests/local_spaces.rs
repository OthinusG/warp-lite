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
fn panel_reservation_pages_preserve_scope_and_checkout() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let root1 = warp_agent_bus::project_root(first.path()).unwrap();
    let root2 = warp_agent_bus::project_root(second.path()).unwrap();
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let b = &server.broker;
    let group = space(b, &root1, "Lease scope");
    let workspace1 = map(b, &root1, &group);
    let workspace2 = map(b, &root2, &group);
    let shared = client(b, "lease-shared", "shared", &root1, Some(&workspace1));
    let private = client(b, "lease-private", "private", &root1, None);
    let other = client(b, "lease-other", "other", &root2, Some(&workspace2));
    call(b, &private, reserve("private-path.txt")).unwrap();
    call(b, &other, reserve("other-checkout.txt")).unwrap();
    for index in 0..51 {
        call(b, &shared, reserve(&format!("shared-{index}.txt"))).unwrap();
    }
    let mut query = warp_agent_bus::transport::PanelQuery {
        project: root1.clone(),
        terminal: Some("lease-shared".into()),
        ..Default::default()
    };
    let first_page = b.operator_panel(&query).unwrap();
    assert_eq!(first_page["reservations"].as_array().unwrap().len(), 50);
    query.scope = first_page["project"].as_str().map(str::to_owned);
    query.reservation_after = first_page["reservation_cursor"].as_u64();
    let second_page = b.operator_panel(&query).unwrap();
    assert_eq!(second_page["reservations"].as_array().unwrap().len(), 1);
    let paths: std::collections::HashSet<_> = first_page["reservations"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second_page["reservations"].as_array().unwrap())
        .map(|lease| lease["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths.len(), 51);
    assert!(!paths.contains("private-path.txt"));
    assert!(!paths.contains("other-checkout.txt"));
    query.project = root2;
    query.terminal = Some("lease-other".into());
    query.reservation_after = None;
    assert_eq!(
        b.operator_panel(&query).unwrap()["reservations"][0]["path"],
        "other-checkout.txt"
    );
    query.project = root1;
    query.terminal = Some("lease-private".into());
    query.reservation_after = Some(u64::MAX);
    assert_eq!(
        b.operator_panel(&query).unwrap()["reservations"][0]["path"],
        "private-path.txt"
    );
}

#[test]
fn reviewed_workspace_is_pinned_and_visible_before_agent_discovery() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::create_dir(fixture.path().join(".git")).unwrap();
    let nested = fixture.path().join("src");
    std::fs::create_dir(&nested).unwrap();
    let root = warp_agent_bus::project_root(&nested).unwrap();
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let b = &server.broker;
    let first = space(b, &root, "Alpha reviewed");
    let second = space(b, &root, "Beta remapped");
    let workspace = map(b, nested.to_str().unwrap(), &first);
    let reviewed = warp_agent_bus::WorkspaceBinding {
        id: workspace.clone(),
        space: first.clone(),
        root: root.clone(),
    };
    b.validate_workspace(&reviewed).unwrap();
    b.prepare_bound_workspace("pending-shared", &reviewed)
        .unwrap();
    let snapshot = b
        .operator_panel(&warp_agent_bus::transport::PanelQuery {
            project: root.clone(),
            terminal: Some("pending-shared".into()),
            spaces: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(snapshot["project"], reviewed.domain());
    assert_eq!(snapshot["admission"], "shared");
    assert_eq!(snapshot["spaces"][1]["workspaces"][0]["root"], root);
    let mut cursor = None;
    let mut names = Vec::new();
    loop {
        let page = b
            .control(
                &root,
                &ControllerOperation::SpaceList {
                    cursor,
                    limit: Some(1),
                },
            )
            .unwrap();
        assert_eq!(page["spaces"].as_array().unwrap().len(), 1);
        names.push(page["spaces"][0]["name"].as_str().unwrap().to_owned());
        cursor = page["cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(names, ["Private", "Alpha reviewed", "Beta remapped"]);
    map(b, &root, &second);
    assert!(b.validate_workspace(&reviewed).is_err());
    assert!(b
        .prepare_bound_workspace("stale-review", &reviewed)
        .is_err());
    let revoked = b
        .operator_panel(&warp_agent_bus::transport::PanelQuery {
            project: root.clone(),
            terminal: Some("pending-shared".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(revoked["project"], reviewed.domain());
    assert_eq!(revoked["admission"], "revoked");
    b.prepare("ordinary-private").unwrap();
    assert_eq!(
        b.operator_panel(&warp_agent_bus::transport::PanelQuery {
            project: root.clone(),
            terminal: Some("ordinary-private".into()),
            ..Default::default()
        })
        .unwrap()["project"],
        root
    );
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
    let scoped_panel = b
        .operator_panel(&warp_agent_bus::transport::PanelQuery {
            project: root1.clone(),
            scope: Some(root1.clone()),
            terminal: Some("shared-issuer".into()),
            selected_task: Some(private_task.clone()),
            event_after: Some(u64::MAX),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(scoped_panel["project"], domain);
    assert!(
        scoped_panel["task"].is_null(),
        "scope changes discard private selection"
    );
    assert!(
        !scoped_panel["events"].as_array().unwrap().is_empty(),
        "scope changes reset cursor"
    );
    assert_eq!(
        call(b, &issuer, Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        call(b, &private_issuer, Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
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
            b.local_evidence_file(&domain, evidence_id).unwrap(),
            second.path().join("source.txt").canonicalize().unwrap()
        );
        assert!(b.local_evidence_file(&root1, evidence_id).is_err());
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
        0
    );
}

#[test]
fn panel_pages_resume_and_keep_presence_separate_from_execution() {
    use warp_agent_bus::transport::PanelQuery;
    let directory = tempfile::tempdir().unwrap();
    let root = warp_agent_bus::project_root(directory.path()).unwrap();
    let server = RunningBroker::start(&directory.path().join("panel.sqlite")).unwrap();
    let b = &server.broker;
    let issuer = client(b, "panel-issuer", "issuer", &root, None);
    let worker = client(b, "panel-worker", "worker", &root, None);
    let task = assign(b, &issuer, "worker");
    call(
        b,
        &worker,
        Operation::TaskStart {
            task_id: task.clone(),
            revision: 1,
            expected_version: Some(1),
            request_id: id(),
        },
    )
    .unwrap();
    b.input_guard("panel-worker", true, true);
    let mut query = PanelQuery {
        project: root.clone(),
        scope: Some(root.clone()),
        terminal: Some("panel-issuer".into()),
        selected_task: Some(task.clone()),
        ..Default::default()
    };
    let snapshot = b.operator_panel(&query).unwrap();
    assert_eq!(snapshot["task"]["state"], "running");
    assert_eq!(snapshot["task_runtime"]["online"], true);
    assert_eq!(snapshot["task_runtime"]["interrupted"], false);
    let worker_row = snapshot["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["agent"]["name"] == "worker")
        .unwrap();
    assert_eq!(worker_row["blocked"], true);
    assert_eq!(worker_row["device"], "local");
    assert_eq!(worker_row["workspace"], root);
    assert!(worker_row["last_observed_ms"].as_u64().is_some());
    assert_eq!(worker_row["observation_source"], "local observation");
    assert_eq!(
        worker_row["draft"], "present",
        "rich drafts protect delivery too"
    );
    query.event_after = snapshot["event_cursor"].as_u64();
    assert!(query.event_after.is_some());
    let unchanged = b.operator_panel(&query).unwrap();
    assert!(unchanged["events"].as_array().unwrap().is_empty());
    b.expire_epoch("panel-worker");
    let expired = b.operator_panel(&query).unwrap();
    assert_eq!(expired["task_runtime"]["online"], false);
    assert_eq!(expired["task_runtime"]["interrupted"], true);
    b.end("panel-worker");
    let offline = b.operator_panel(&query).unwrap();
    assert_eq!(
        offline["task"]["state"], "running",
        "exit does not claim task effects stopped"
    );
    assert_eq!(offline["task_runtime"]["online"], false);
    assert_eq!(offline["task_runtime"]["interrupted"], true);
    assert!(!offline["agents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["agent"]["name"] == "worker"));
    assert_eq!(offline["participant_names"][offline["task"]["assignee"].as_str().unwrap()], "worker");

    query.task_state = Some("accepted".into());
    assert!(b.operator_panel(&query).unwrap()["tasks"]
        .as_array()
        .unwrap()
        .is_empty());
    query.task_state = Some("running".into());
    assert_eq!(
        b.operator_panel(&query).unwrap()["tasks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    query.task_assignee = Some(id());
    assert!(b.operator_panel(&query).unwrap()["tasks"]
        .as_array()
        .unwrap()
        .is_empty());
    query.task_state = None;
    query.task_assignee = None;

    for index in 0..51 {
        client(
            b,
            &format!("panel-{index:02}"),
            &format!("member-{index:02}"),
            &root,
            None,
        );
    }
    let first = b.operator_panel(&query).unwrap();
    assert_eq!(first["agents"].as_array().unwrap().len(), 50);
    query.agent_after = first["agent_cursor"].as_str().map(str::to_owned);
    let second = b.operator_panel(&query).unwrap();
    assert_eq!(second["agents"].as_array().unwrap().len(), 3);
    assert!(second["agent_cursor"].is_null());
    let names: std::collections::HashSet<_> = first["agents"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["agents"].as_array().unwrap())
        .map(|row| row["agent"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 53);

    query.agent_after = None;
    loop {
        let batch = b.operator_panel(&query).unwrap();
        if batch["events"].as_array().unwrap().is_empty() {
            break;
        }
        query.event_after = batch["event_cursor"].as_u64();
    }
    query.wait = true;
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = b.clone();
    let waiter = std::thread::spawn(move || sender.send(reader.operator_panel(&query)).unwrap());
    b.operator(
        &root,
        &Operation::TaskCancel {
            task_id: task,
            reason: "Operator requested cancellation".into(),
            expected_version: None,
            request_id: id(),
        },
    )
    .unwrap();
    let changed = receiver
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap()
        .unwrap();
    assert!(changed["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["kind"]
            .as_str()
            .is_some_and(|kind| kind.contains("cancel"))));
    waiter.join().unwrap();
}

#[test]
fn native_delivery_observations_never_acknowledge_or_start_tasks() {
    use warp_agent_bus::transport::PanelQuery;
    let directory = tempfile::tempdir().unwrap();
    let root = warp_agent_bus::project_root(directory.path()).unwrap();
    let database = directory.path().join("native-delivery.sqlite");
    let server = RunningBroker::start(&database).unwrap();
    let b = &server.broker;
    let issuer = client(b, "delivery-issuer", "issuer", &root, None);
    let worker = client(b, "delivery-worker", "worker", &root, None);
    let task = assign(b, &issuer, "worker");
    let initial = b.operator_task(&root, &task).unwrap();
    let query = PanelQuery {
        project: root.clone(),
        scope: Some(root.clone()),
        selected_task: Some(task.clone()),
        ..Default::default()
    };
    let mut delivered = None;
    for submitted in [false, true] {
        std::thread::sleep(std::time::Duration::from_millis(800));
        let wake = b
            .wakeups()
            .into_iter()
            .find(|wake| wake.terminal == worker.terminal)
            .unwrap();
        assert!(b.claim_wake(&wake));
        assert_eq!(
            b.operator_panel(&query).unwrap()["task_runtime"]["delivery_phase"],
            "claimed"
        );
        b.finish_wake(&wake, submitted);
        b.finish_wake(&wake, submitted);
        let panel = b.operator_panel(&query).unwrap();
        assert_eq!(
            panel["task_runtime"]["delivery_phase"],
            if submitted { "submitted" } else { "cancelled" }
        );
        assert_eq!(panel["task_runtime"]["delivery_retained"], true);
        let current = b.operator_task(&root, &task).unwrap();
        assert_eq!(current["state"], "queued");
        assert_eq!(current["version"], initial["version"]);
        assert!(current["attempts"].as_array().unwrap().is_empty());
        let inbox = call(
            b,
            &worker,
            Operation::AgentInbox {
                cursor: None,
                limit: Some(50),
            },
        )
        .unwrap();
        assert!(inbox["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["id"] == wake.message_id && message["acknowledged"] == false));
        delivered = Some(wake.message_id);
    }
    let events = b.operator_events(&root, None, Some(50)).unwrap();
    let phases: Vec<_> = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["kind"] == "native_delivery_observed")
        .collect();
    assert_eq!(phases.len(), 4);
    assert_eq!(
        phases
            .iter()
            .map(|event| event["payload"]["phase"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["claimed", "cancelled", "claimed", "submitted"]
    );
    for event in phases {
        assert_eq!(event["resource"], task);
        assert_eq!(event["payload"]["acknowledgement_implied"], false);
        assert_eq!(event["payload"]["execution_implied"], false);
        assert!(event["payload"].get("body").is_none());
    }
    let ack_error = call(
        b,
        &worker,
        Operation::AgentAck {
            message_id: delivered.unwrap(),
        },
    )
    .unwrap_err();
    assert_eq!(
        ack_error
            .downcast_ref::<warp_agent_bus::DomainError>()
            .unwrap()
            .code,
        "invalid_state"
    );
    assert_eq!(b.operator_task(&root, &task).unwrap()["state"], "queued");
    drop(server);
    let reopened = RunningBroker::start(&database).unwrap();
    assert_eq!(
        reopened.broker.operator_task(&root, &task).unwrap()["state"],
        "queued"
    );
    assert_eq!(
        reopened
            .broker
            .operator_events(&root, None, Some(50))
            .unwrap()["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["kind"] == "native_delivery_observed")
            .count(),
        4
    );
    let panel = reopened.broker.operator_panel(&query).unwrap();
    assert_eq!(panel["task_runtime"]["online"], false);
    assert!(panel["task_runtime"]["delivery_phase"].is_null());
}

#[test]
fn retired_device_controls_cannot_issue_or_replay_authority() {
    let server = RunningBroker::start(std::path::Path::new(":memory:")).unwrap();
    let operations = [
        ControllerOperation::DeviceList,
        ControllerOperation::InvitationCreate { space_ids: vec![id()], ttl_seconds: None, request_id: id() },
        ControllerOperation::DeviceRevoke { device_id: id(), request_id: id() },
        ControllerOperation::DeviceGrantUpdate { device_id: id(), expected_generation: 1,
            space_id: id(), mode: Some("write".into()), request_id: id() },
        ControllerOperation::RemoteWorkspaceMap { device_id: Uuid::new_v4(), expected_generation: 1,
            space_id: Uuid::new_v4(), checkout_id: Uuid::new_v4(), label: "Legacy".into(),
            repository_id: None, request_id: id() },
    ];
    for operation in operations {
        for _ in 0..2 {
            let error = server.broker.control("/fixture", &operation).unwrap_err();
            assert_eq!(error.downcast_ref::<warp_agent_bus::DomainError>().unwrap().code, "feature_unavailable");
        }
    }
    server.broker.control("/fixture", &ControllerOperation::SpaceCreate {
        name: "Local spaces still work".into(), request_id: id(),
    }).unwrap();
}
