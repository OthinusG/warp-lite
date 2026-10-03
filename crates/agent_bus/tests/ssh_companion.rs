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
    controlled_two_agents(&profile).await;
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

async fn controlled_two_agents(profile: &SshProfile) {
    use warp_agent_bus::Operation;
    let fixture =
        std::env::var("WARP_TEST_AGENT_FIXTURE").expect("Owned native Agent fixture executable");
    let mut one = HostClient::connect(profile).await.unwrap();
    let mut two = HostClient::connect(profile).await.unwrap();
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
        warp_agent_bus::transport::PanelQuery::default(),
    );
    let panel = one.project_tasks(&query, 10).await.unwrap();
    assert!(panel["value"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["id"] == task && entry["state"] == "accepted"));
    let mut other_profile = profile.clone();
    other_profile.remote_root.push_str("/isolated");
    std::fs::create_dir_all(&other_profile.remote_root).unwrap();
    let mut other = HostClient::connect(&other_profile).await.unwrap();
    let panel = other.project_tasks(&query, 11).await.unwrap();
    assert!(panel["value"]["tasks"].as_array().unwrap().is_empty());
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
