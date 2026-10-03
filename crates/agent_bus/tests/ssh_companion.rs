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
        id: Uuid::new_v4(),
        display_name: "Controlled SSH".into(),
        target: "warpai-test".into(),
        user: None,
        port: None,
        identity_file: None,
        config_file: Some(config.into()),
        jump_alias: None,
        remote_root: root,
        companion_path: Some(companion),
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
    controlled_retained_terminal(&profile).await;
    // The alias resolves through a controlled config, not local project canonicalization.
    let mut missing = profile.clone();
    missing.remote_root.push_str("/missing-root");
    assert!(HostClient::connect(&missing).await.is_err());
}

async fn controlled_retained_terminal(profile: &SshProfile) {
    let mut owner = HostClient::connect(profile).await.unwrap();
    assert!(owner
        .capabilities()
        .iter()
        .any(|c| c == "retained_terminal"));
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
    let boot = owner.fence().unwrap().service_boot_id.clone();
    owner.disconnect();
    let mut next = HostClient::connect(profile).await.unwrap();
    assert_eq!(next.fence().unwrap().service_boot_id, boot);
    let retained = next
        .terminal_list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.session_id == state.session_id)
        .unwrap();
    assert_eq!(retained.run_id, state.run_id);
    assert!(retained.exit_code.is_none());
    let attached = next
        .terminal_control(terminal_command(
            &next,
            &retained,
            TerminalAction::TerminalAttach,
        ))
        .await
        .unwrap();
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
