//! Executed only against an explicitly provisioned isolated CI SSH fixture.
use remote_protocol::proto::{TerminalAction, TerminalControl, TerminalLaunch, TerminalState};
use uuid::Uuid;
use warp_agent_bus::ssh_remote::{HostClient, RemoteShell, SshProfile};

#[tokio::test]
#[ignore = "requires the controlled OpenSSH fixture"]
async fn controlled_ssh_uses_project_communication_fences() {
    let config = std::env::var_os("WARP_TEST_SSH_CONFIG").expect("Controlled SSH config");
    let root = std::env::var("WARP_TEST_REMOTE_ROOT").expect("Remote fixture root");
    let companion = std::env::var("WARP_TEST_COMPANION_PATH").expect("Owned companion path");
    let profile = SshProfile {
        target: "warpai-test".into(),
        config_file: Some(config.into()),
        remote_root: root,
        companion_path: companion,
        remote_shell: RemoteShell::Posix,
    };
    let mut first = HostClient::connect(&profile)
        .await
        .expect("Verified SSH companion");
    assert!(std::path::Path::new(&first.canonical_root).is_absolute());
    let mut second = HostClient::connect(&profile).await.unwrap();
    let one = first.fence().unwrap();
    let two = second.fence().unwrap();
    assert_ne!(one.connection_id, two.connection_id);
    assert_eq!(one.service_boot_id, two.service_boot_id);
    assert_eq!(one.service_id, two.service_id);
    assert_eq!(one.project_id, two.project_id);
    assert_eq!(first.account_id, second.account_id);
    assert_eq!(first.root_identity, second.root_identity);
    let query = warp_agent_bus::companion::TaskCommand::Panel(
        warp_agent_bus::transport::PanelQuery::default(),
    );
    let tasks = first.project_tasks(&query, 3).await.unwrap();
    assert_eq!(
        tasks["value"]["project"].as_str(),
        first.fence().map(|f| f.project_id.as_str())
    );
    assert!(tasks["value"]["tasks"].is_array());
    let second_tasks = second.project_tasks(&query, 4).await.unwrap();
    assert_eq!(tasks["value"]["tasks"], second_tasks["value"]["tasks"]);
    assert_eq!(
        first
            .project_tasks(
                &warp_agent_bus::companion::TaskCommand::Operator(
                    warp_agent_bus::Operation::AgentRegister {
                        name: "not-a-gui-operation".into()
                    }
                ),
                5
            )
            .await
            .unwrap_err(),
        warp_agent_bus::ssh_remote::ConnectionError::FeatureUnavailable
    );
    controlled_agent_terminal(&profile).await;
    controlled_two_agents(&profile, &profile, false).await;
    controlled_worktree_team(&profile).await;
    // The alias resolves through a controlled config, not local project canonicalization.
    let mut missing = profile.clone();
    missing.remote_root.push_str("/missing-root");
    assert!(HostClient::connect(&missing).await.is_err());
}

async fn controlled_agent_terminal(profile: &SshProfile) {
    let mut owner = HostClient::connect(profile).await.unwrap();
    assert!(owner.capabilities().iter().any(|c| c == "managed_agent"));
    let launch = TerminalLaunch { fence: owner.fence().cloned(), session_id: Uuid::new_v4().to_string(),
        executable: "/bin/sh".into(), arguments: vec!["-c".into(),
            "printf 'REMOTE_ROOT=%s\\n' \"$PWD\"; while IFS= read -r input; do printf 'RECEIVED=%s\\n' \"$input\"; done".into()],
        columns: 80, rows: 24, agent_program: None };
    let state = owner.terminal_launch(launch.clone()).await.unwrap();
    assert_eq!(
        owner.terminal_launch(launch).await.unwrap().run_id,
        state.run_id
    );
    let expected_root = format!("REMOTE_ROOT={}", owner.canonical_root);
    terminal_output(&mut owner, &state, &expected_root).await;
    let mut next = owner;
    let attached = state;
    let mut input = terminal_command(&next, &attached, TerminalAction::TerminalInput);
    input.input = "Remote Unicode 多语言;$(not-a-command)\r"
        .as_bytes()
        .to_vec();
    input.input_sequence = 1;
    next.terminal_control(input.clone()).await.unwrap();
    assert!(next.terminal_control(input).await.is_err());
    terminal_output(
        &mut next,
        &attached,
        "RECEIVED=Remote Unicode 多语言;$(not-a-command)",
    )
    .await;
    let stopped = next
        .terminal_control(terminal_command(
            &next,
            &attached,
            TerminalAction::TerminalStop,
        ))
        .await
        .unwrap();
    assert!(stopped.stop_requested);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let status = next
            .terminal_control(terminal_command(
                &next,
                &attached,
                TerminalAction::TerminalRead,
            ))
            .await
            .unwrap();
        if status.exit_code.is_some() && status.output_closed {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Remote Stop not observed"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    next.terminal_control(terminal_command(
        &next,
        &attached,
        TerminalAction::TerminalRelease,
    ))
    .await
    .unwrap();
    next.disconnect();
}

fn terminal_command(
    client: &HostClient,
    state: &TerminalState,
    action: TerminalAction,
) -> TerminalControl {
    TerminalControl {
        fence: client.fence().cloned(),
        session_id: state.session_id.clone(),
        run_id: state.run_id.clone(),
        action: action.into(),
        ..TerminalControl::default()
    }
}

async fn terminal_output(
    client: &mut HostClient,
    state: &TerminalState,
    expected: &str,
) -> TerminalState {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let result = client
            .terminal_control(terminal_command(
                client,
                state,
                TerminalAction::TerminalRead,
            ))
            .await
            .unwrap();
        if String::from_utf8_lossy(&result.output).contains(expected) {
            return result;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Missing remote terminal output"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

async fn controlled_worktree_team(profile: &SshProfile) {
    use warp_agent_bus::{companion::TaskCommand, ControllerOperation};
    // The provisioned loopback fixture deliberately shares this runner's filesystem.
    let owned = tempfile::tempdir_in(&profile.remote_root).unwrap();
    let main = owned.path().join("main checkout");
    let linked = owned.path().join("feature checkout");
    let unjoined = owned.path().join("unjoined");
    std::fs::create_dir(&main).unwrap();
    let git = |arguments: &[&str]| {
        let result = std::process::Command::new(warp_agent_bus::installation::git_executable().unwrap())
            .args(["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false", "-c", "core.hooksPath="])
            .args(arguments).current_dir(&main).output().unwrap();
        assert!(result.status.success(), "Controlled SSH Git fixture command succeeds");
    };
    git(&["init", "--initial-branch=main"]);
    std::fs::write(main.join("source.txt"), "base").unwrap();
    git(&["add", "source.txt"]);
    git(&["commit", "-m", "SSH worktree fixture"]);
    git(&["worktree", "add", "-b", "feature", linked.to_str().unwrap()]);
    git(&["worktree", "add", "-b", "unjoined", unjoined.to_str().unwrap()]);
    let mut first = profile.clone();
    first.remote_root = main.to_str().unwrap().into();
    let mut second = profile.clone();
    second.remote_root = linked.to_str().unwrap().into();
    for selected in [&first, &second] {
        let mut client = HostClient::connect(selected).await.unwrap();
        let joined = client.project_tasks(&TaskCommand::Controller(ControllerOperation::WorktreeJoin {
            root: client.canonical_root.clone(), request_id: Uuid::new_v4().to_string(),
        }), 1).await.unwrap();
        assert!(joined.get("value").is_some());
        client.disconnect();
    }
    controlled_two_agents(&first, &second, true).await;
}

async fn controlled_two_agents(profile: &SshProfile, receiver_profile: &SshProfile, worktree_team: bool) {
    use warp_agent_bus::Operation;
    let fixture =
        std::env::var("WARP_TEST_AGENT_FIXTURE").expect("Owned native Agent fixture executable");
    let mut one = HostClient::connect(profile).await.unwrap();
    let mut two = HostClient::connect(receiver_profile).await.unwrap();
    async fn launch(client: &mut HostClient, executable: &str) -> TerminalState {
        let state = client
            .terminal_launch(TerminalLaunch {
                fence: client.fence().cloned(),
                session_id: Uuid::new_v4().to_string(),
                executable: executable.into(),
                arguments: ["--exact", "managed_agent_child", "--ignored", "--nocapture"]
                    .map(str::to_owned)
                    .to_vec(),
                columns: 1000,
                rows: 24,
                agent_program: Some("fixture".into()),
            })
            .await
            .unwrap();
        terminal_output(client, &state, &format!("MCP_NATIVE_RUN={}", state.run_id)).await;
        state
    }
    let mut first = launch(&mut one, &fixture).await;
    let mut second = launch(&mut two, &fixture).await;
    if worktree_team {
        use warp_agent_bus::{ControllerOperation, companion::TaskCommand, transport::PanelQuery};
        let query = TaskCommand::Panel(PanelQuery { worktree:true, ..Default::default() });
        let panel = one.project_tasks(&query, 8).await.unwrap();
        let find = |name: &str| panel["value"]["candidates"].as_array().unwrap().iter()
            .find(|candidate| candidate["agent"]["name"].as_str() == Some(name)).unwrap().clone();
        let lead = find(&format!("fixture-{}", first.session_id));
        let worker = find(&format!("fixture-{}", second.session_id));
        assert!(one.project_tasks(&TaskCommand::Controller(ControllerOperation::WorktreeCoordinator {
            root:lead["root"].as_str().unwrap().into(), agent:lead["agent"]["id"].as_str().unwrap().into(),
            run:lead["run"].as_str().unwrap().into(), request_id:Uuid::new_v4().to_string(),
        }), 9).await.unwrap().get("value").is_some());
        assert!(one.project_tasks(&TaskCommand::Controller(ControllerOperation::WorktreeWorker {
            root:worker["root"].as_str().unwrap().into(), agent:worker["agent"]["id"].as_str().unwrap().into(),
            run:worker["run"].as_str().unwrap().into(), request_id:Uuid::new_v4().to_string(),
        }), 9).await.unwrap().get("value").is_some());
    }
    let receiver = format!("fixture-{}", second.session_id);
    ssh_fixture_operation(
        &mut one,
        &mut first,
        Operation::AgentSend {
            to: receiver.clone(),
            body: "Remote fixture message".into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: Uuid::new_v4().to_string(),
        },
    )
    .await;
    assert_eq!(
        ssh_fixture_operation(
            &mut two,
            &mut second,
            Operation::AgentInbox {
                cursor: None,
                limit: None,
            }
        )
        .await["has_message"],
        true
    );
    let assigned = ssh_fixture_operation(
        &mut one,
        &mut first,
        Operation::TaskAssign {
            to: receiver,
            description: "SSH fixture work".into(),
            acceptance: "Message and review pass".into(),
            reviewer: None,
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
            request_id: Uuid::new_v4().to_string(),
        },
    )
    .await;
    let task = assigned["id"].as_str().unwrap().to_owned();
    let started = ssh_fixture_operation(
        &mut two,
        &mut second,
        Operation::TaskStart {
            task_id: task.clone(),
            revision: 1,
            expected_version: None,
            request_id: Uuid::new_v4().to_string(),
        },
    )
    .await;
    assert_eq!(started["state"], "running");
    let submitted = ssh_fixture_operation(
        &mut two,
        &mut second,
        Operation::TaskSubmit {
            task_id: task.clone(),
            revision: 1,
            result: "Done".into(),
            evidence: "Actual SSH processes pass".into(),
            expected_version: None,
            attempt_id: Some(started["attempt_id"].as_str().unwrap().into()),
            request_id: Uuid::new_v4().to_string(),
            evidence_ids: vec![],
        },
    )
    .await;
    assert_eq!(submitted["state"], "submitted");
    assert_eq!(
        ssh_fixture_operation(
            &mut one,
            &mut first,
            Operation::TaskReview {
                task_id: task.clone(),
                revision: 1,
                accepted: true,
                feedback: "Verified".into(),
                expected_version: None,
                request_id: Uuid::new_v4().to_string(),
            }
        )
        .await["state"],
        "accepted"
    );
    let query = warp_agent_bus::companion::TaskCommand::Panel(
        warp_agent_bus::transport::PanelQuery { worktree:worktree_team, ..Default::default() },
    );
    let panel = one.project_tasks(&query, 10).await.unwrap();
    assert!(panel["value"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["id"] == task && entry["state"] == "accepted"));
    if worktree_team {
        let receiver_panel = two.project_tasks(&query, 10).await.unwrap();
        assert_eq!(panel["value"]["collaboration_scope"], receiver_panel["value"]["collaboration_scope"]);
        assert_ne!(one.fence().unwrap().project_id, two.fence().unwrap().project_id);
        assert_eq!(panel["value"]["worktree_branch"], "main");
        assert_eq!(receiver_panel["value"]["worktree_branch"], "feature");
        assert_eq!(receiver_panel["value"]["worktree_root"], two.canonical_root);
    }
    let mut other_profile = profile.clone();
    if worktree_team {
        other_profile.remote_root = std::path::Path::new(&profile.remote_root).parent().unwrap().join("unjoined").to_str().unwrap().into();
    } else { other_profile.remote_root.push_str("/isolated"); }
    std::fs::create_dir_all(&other_profile.remote_root).unwrap();
    let mut other = HostClient::connect(&other_profile).await.unwrap();
    let other_panel = other.project_tasks(&warp_agent_bus::companion::TaskCommand::Panel(warp_agent_bus::transport::PanelQuery::default()), 11).await.unwrap();
    if worktree_team {
        assert_eq!(other_panel["value"]["collaboration_scope"], panel["value"]["collaboration_scope"]);
        assert!(other_panel["value"]["tasks"].as_array().unwrap().iter().any(|entry| entry["id"] == task));
    } else {
        assert!(other_panel["value"]["tasks"].as_array().unwrap().is_empty());
    }
    assert!(other
        .terminal_control(terminal_command(
            &other,
            &first,
            TerminalAction::TerminalRead
        ))
        .await
        .is_err());
    other.disconnect();
    one.disconnect();
    two.disconnect();
}

async fn ssh_fixture_operation(
    client: &mut HostClient,
    state: &mut TerminalState,
    operation: warp_agent_bus::Operation,
) -> serde_json::Value {
    let mut input = terminal_command(client, state, TerminalAction::TerminalInput);
    input.input_sequence = state.accepted_input_sequence + 1;
    input.input = format!("{}\r", serde_json::to_string(&operation).unwrap()).into_bytes();
    *state = client.terminal_control(input).await.unwrap();
    let marker = format!("FIXTURE_RESULT_{}=", state.accepted_input_sequence);
    let output = terminal_output(client, state, &marker).await;
    let output = String::from_utf8_lossy(&output.output);
    serde_json::from_str(
        output
            .split(&marker)
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .trim(),
    )
    .unwrap()
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "requires the controlled OpenSSH fixture"]
async fn established_terminal_socket_probes_and_fences_the_companion() {
    use std::process::Stdio;
    use warp_agent_bus::ssh_remote::ConnectionError;
    let config = std::env::var_os("WARP_TEST_SSH_CONFIG").expect("Controlled SSH config");
    let root = std::env::var("WARP_TEST_REMOTE_ROOT").expect("Remote fixture root");
    let companion = std::env::var("WARP_TEST_COMPANION_PATH").expect("Owned companion path");
    // OpenSSH appends a temporary suffix; macOS Unix sockets have a 104-byte limit.
    let directory = tempfile::Builder::new()
        .prefix("warpai-ssh-")
        .tempdir_in("/tmp")
        .unwrap();
    let socket = directory.path().join("ssh-control");
    let mut master = tokio::process::Command::new("ssh")
        .arg("-F")
        .arg(&config)
        .arg("-M")
        .arg("-S")
        .arg(&socket)
        .args([
            "-N",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "warpai-test",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !socket.exists() {
            assert!(
                master.try_wait().unwrap().is_none(),
                "Controlled SSH master exited"
            );
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    let profile = SshProfile {
        target: "observed-remote-host".into(),
        config_file: Some(config.into()),
        remote_root: root,
        companion_path: companion,
        remote_shell: RemoteShell::Posix,
    };
    let client = HostClient::connect_session(&profile, &socket, None)
        .await
        .unwrap();
    let config_path = profile.config_file.as_ref().unwrap();
    let native_arguments = vec![
        "-p".into(),
        "22222".into(),
        "-i".into(),
        config_path
            .parent()
            .unwrap()
            .join("client")
            .to_str()
            .unwrap()
            .into(),
        "-o".into(),
        format!(
            "UserKnownHostsFile={}",
            config_path.parent().unwrap().join("known_hosts").display()
        ),
        "127.0.0.1".into(),
    ];
    let native = HostClient::connect_arguments(&profile, &native_arguments)
        .await
        .unwrap();
    assert_eq!(native.account_id, client.account_id);
    assert_eq!(
        native.fence().unwrap().project_id,
        client.fence().unwrap().project_id
    );
    drop(native);
    use warp_agent_bus::ssh_files::{RemoteFiles, SshConnection};
    let selected_files = RemoteFiles::connect(
        profile.clone(),
        SshConnection::Multiplexed {
            socket: socket.clone(),
            wsl: None,
        },
    )
    .await
    .unwrap();
    controlled_remote_files(&selected_files).await;
    let native_files = RemoteFiles::connect(
        profile.clone(),
        SshConnection::Native {
            arguments: vec![
                "-F".into(),
                config_path.to_str().unwrap().into(),
                "warpai-test".into(),
            ],
            session: Uuid::new_v4().to_string(),
        },
    )
    .await
    .unwrap();
    controlled_remote_files(&native_files).await;
    let account = client.account_id.clone();
    assert!(!account.is_empty());
    let mut missing = profile.clone();
    missing.companion_path.push_str("-missing");
    assert_eq!(
        HostClient::connect_session(&missing, &socket, None)
            .await
            .err(),
        Some(ConnectionError::CompanionUnavailable)
    );
    drop(client);
    master.kill().await.unwrap();
    master.wait().await.unwrap();
    assert!(selected_files
        .list(&selected_files.canonical_root, 99)
        .await
        .is_err());
    assert!(
        !selected_files.connected(),
        "Closed master disables retained file attachment"
    );
    assert!(
        HostClient::connect_session(&profile, &socket, None)
            .await
            .is_err(),
        "Closed terminal must not reconnect to another host"
    );
}

async fn controlled_remote_files(files: &warp_agent_bus::ssh_files::RemoteFiles) {
    use remote_protocol::proto::{ProjectFileAction, ProjectFilesRequest};
    use warp_agent_bus::ssh_remote::ConnectionError;
    let directory = format!("file-check-{}", Uuid::new_v4());
    let name = format!(
        "{directory}/{}",
        if cfg!(windows) {
            "literal ' [1] 多语言.md"
        } else {
            "literal ' [1]*? 多语言.md"
        }
    );
    let control = |action, path: &str| ProjectFilesRequest {
        action: action as i32,
        path: path.into(),
        ..Default::default()
    };
    files
        .control(control(
            ProjectFileAction::ProjectDirectoryCreate,
            &directory,
        ))
        .await
        .unwrap();
    files
        .control(control(ProjectFileAction::ProjectFileCreate, &name))
        .await
        .unwrap();
    let absolute = format!("{}/{name}", files.canonical_root.trim_end_matches('/'));
    let (cached, original_hash) = files.download(&absolute).await.unwrap();
    assert_eq!(std::fs::read(&cached).unwrap(), b"");
    let bytes = b"# SSH SFTP\r\n\0binary\xff";
    let hash = files.save(&absolute, bytes, &original_hash).await.unwrap();
    let (cached, observed_hash) = files.download(&absolute).await.unwrap();
    assert_eq!(std::fs::read(&cached).unwrap(), bytes);
    assert_eq!(hash, observed_hash);
    assert_eq!(
        files
            .document_link(&absolute, "./linked image.svg")
            .unwrap(),
        format!(
            "{}/{directory}/linked image.svg",
            files.canonical_root.trim_end_matches('/')
        )
    );
    assert!(files
        .document_link(&absolute, "../../../outside.md")
        .is_err());
    assert_eq!(
        files
            .document_link(&absolute, "linked%20image.svg#figure")
            .unwrap(),
        files.document_link(&absolute, "linked image.svg").unwrap()
    );
    assert!(files.document_link(&absolute, "%00.md").is_err());
    assert!(files
        .document_link(&absolute, "%2e%2e/%2e%2e/%2e%2e/outside")
        .is_err());

    files.disconnect();
    assert_eq!(
        files.download(&absolute).await.unwrap_err(),
        ConnectionError::ConnectionLost
    );
    files.reconnect().await.unwrap();
    assert_eq!(
        std::fs::read(files.download(&absolute).await.unwrap().0).unwrap(),
        bytes
    );

    assert_eq!(
        files
            .save(&absolute, b"must not overwrite", &original_hash)
            .await
            .unwrap_err(),
        ConnectionError::Conflict
    );
    assert_eq!(
        std::fs::read(files.download(&absolute).await.unwrap().0).unwrap(),
        bytes
    );
    let snapshot = files
        .list(&format!("{}/{directory}", files.canonical_root), 77)
        .await
        .unwrap();
    assert_eq!(snapshot.entries[0].subtree_metadata.len(), 1);
    assert!(files
        .download(&format!("{}/../outside", files.canonical_root))
        .await
        .is_err());
    files
        .control(control(ProjectFileAction::ProjectFileDelete, &directory))
        .await
        .unwrap();
    assert_eq!(
        files.download(&absolute).await.unwrap_err(),
        ConnectionError::NotFound
    );
}

#[tokio::test]
#[ignore = "requires the controlled OpenSSH fixture"]
async fn native_ssh_file_tools_save_conflict_preview_resources_and_reconnect() {
    use warp_agent_bus::ssh_files::{RemoteFiles, SshConnection};
    let config = std::env::var("WARP_TEST_SSH_CONFIG").expect("Controlled SSH config");
    let profile = SshProfile {
        target: "warpai-test".into(),
        config_file: None,
        remote_root: std::env::var("WARP_TEST_REMOTE_ROOT").unwrap(),
        companion_path: std::env::var("WARP_TEST_COMPANION_PATH").unwrap(),
        remote_shell: if cfg!(windows) {
            RemoteShell::PowerShell
        } else {
            RemoteShell::Posix
        },
    };
    let connection = SshConnection::Native {
        arguments: vec!["-F".into(), config, "warpai-test".into()],
        session: "owned-native-file-fixture".into(),
    };
    let files = RemoteFiles::connect(profile.clone(), connection.clone())
        .await
        .unwrap();
    let shared = RemoteFiles::connect(profile.clone(), connection.clone())
        .await
        .unwrap();
    assert!(
        std::sync::Arc::ptr_eq(&files, &shared),
        "Explorer and Review reuse the same attachment"
    );
    if let Ok(repository) = files.repository().await {
        assert_eq!(repository.identity, files.identity);
        let directory = format!("nested-cwd-{}", Uuid::new_v4());
        files
            .control(remote_protocol::proto::ProjectFilesRequest {
                action: remote_protocol::proto::ProjectFileAction::ProjectDirectoryCreate as i32,
                path: directory.clone(),
                ..Default::default()
            })
            .await
            .unwrap();
        let mut nested_profile = profile;
        nested_profile.remote_root =
            format!("{}/{directory}", files.canonical_root.trim_end_matches('/'));
        let nested = RemoteFiles::connect(nested_profile, connection)
            .await
            .unwrap();
        let parent = nested.repository().await.unwrap();
        assert_eq!(parent.identity, files.identity);
        assert_eq!(parent.canonical_root, files.canonical_root);
    }
    controlled_remote_files(&files).await;
}
