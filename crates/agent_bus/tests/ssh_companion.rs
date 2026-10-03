//! Executed only against an explicitly provisioned isolated CI SSH fixture.
use remote_protocol::proto::{TerminalAction, TerminalControl, TerminalLaunch, TerminalState};
use uuid::Uuid;
use warp_agent_bus::sftp::{SftpClient, SftpError, UploadState};
use warp_agent_bus::ssh_remote::{HostClient, RemoteShell, SshProfile};

#[tokio::test]
#[ignore = "requires the controlled OpenSSH fixture"]
async fn controlled_ssh_uses_companion_fences_and_file_only_sftp() {
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
    let one = first.host_status(1).await.unwrap();
    assert_eq!(one.source, "companion_native");
    assert_eq!(one.query_generation, 1);
    let mut second = HostClient::connect(&profile).await.unwrap();
    let two = second.host_status(2).await.unwrap();
    assert_ne!(
        one.fence.as_ref().unwrap().connection_id,
        two.fence.as_ref().unwrap().connection_id
    );
    assert_eq!(
        one.fence.as_ref().unwrap().service_boot_id,
        two.fence.as_ref().unwrap().service_boot_id
    );
    assert_eq!(
        one.fence.as_ref().unwrap().service_id,
        two.fence.as_ref().unwrap().service_id
    );
    assert_eq!(
        one.fence.as_ref().unwrap().project_id,
        two.fence.as_ref().unwrap().project_id
    );
    assert_eq!(first.account_id, second.account_id);
    assert_eq!(first.root_identity, second.root_identity);
    let mut projection =
        remote_protocol::managed::HostObservation::new(two.fence.unwrap(), 2).unwrap();
    assert!(!projection.receive(one, std::time::Instant::now()));
    controlled_retained_terminal(&profile).await;
    // The alias resolves through a controlled config, not local project canonicalization.
    let mut missing = profile.clone();
    missing.remote_root.push_str("/missing-root");
    assert!(HostClient::connect(&missing).await.is_err());
    let mut file_only = profile;
    file_only.companion_path = None;
    let mut sftp = SftpClient::connect(&file_only)
        .await
        .expect("Independent SFTP subsystem");
    let listing = sftp.list("").await.unwrap();
    assert!(!listing.truncated);
    assert!(listing
        .entries
        .iter()
        .any(|entry| entry.name == "folder" && entry.attributes.is_directory()));
    assert!(listing
        .entries
        .iter()
        .any(|entry| entry.name == "outside-link" && entry.attributes.is_symlink()));
    let bytes = sftp.read("Unicode 空格;$(shell).bin").await.unwrap();
    let expected: Vec<u8> = (0..257).flat_map(|_| 0u8..=255).collect();
    assert_eq!(bytes, expected);
    assert!(matches!(
        sftp.stat("outside-link/passwd").await,
        Err(SftpError::PathEscape)
    ));
    assert!(matches!(
        sftp.read("../outside").await,
        Err(SftpError::InvalidName)
    ));
    assert!(matches!(sftp.remove("").await, Err(SftpError::InvalidName)));
    assert!(matches!(
        sftp.create_directory("../escape").await,
        Err(SftpError::InvalidName)
    ));
    let directory = format!("Upload 多语言 {}", Uuid::new_v4());
    sftp.create_directory(&directory).await.unwrap();
    let destination = format!("{directory}/binary ;$(never-execute).bin");
    let receipt = sftp.upload_intent(&destination, &expected).unwrap();
    sftp.prepare_upload(&receipt, &expected).await.unwrap();
    assert_eq!(
        sftp.reconcile_upload(&receipt).await.unwrap(),
        UploadState::Prepared
    );
    // Prepared content survives the original SSH attachment; reconcile before any retry.
    sftp.disconnect();
    let mut sftp = SftpClient::connect(&file_only).await.unwrap();
    assert_eq!(
        sftp.reconcile_upload(&receipt).await.unwrap(),
        UploadState::Prepared
    );
    sftp.commit_upload(&receipt).await.unwrap();
    assert_eq!(
        sftp.reconcile_upload(&receipt).await.unwrap(),
        UploadState::Confirmed
    );
    assert_eq!(sftp.read(&destination).await.unwrap(), expected);
    let competing = sftp
        .upload_intent(&destination, b"must not replace")
        .unwrap();
    sftp.prepare_upload(&competing, b"must not replace")
        .await
        .unwrap();
    assert!(matches!(
        sftp.commit_upload(&competing).await,
        Err(SftpError::Conflict)
    ));
    assert_eq!(sftp.read(&destination).await.unwrap(), expected);
    sftp.remove(&competing.partial).await.unwrap();
    let renamed = format!("{directory}/renamed.bin");
    sftp.rename_new(&destination, &renamed).await.unwrap();
    assert_eq!(sftp.read(&renamed).await.unwrap(), expected);
    sftp.remove(&renamed).await.unwrap();
    sftp.remove(&directory).await.unwrap();
    assert!(matches!(
        sftp.stat(&directory).await,
        Err(SftpError::Missing)
    ));
    sftp.disconnect();
    assert!(matches!(
        sftp.list("").await,
        Err(SftpError::ConnectionLost)
    ));
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
        attachment_generation: state.attachment_generation,
        action: action.into(),
        ..TerminalControl::default()
    }
}

async fn terminal_output(client: &mut HostClient, state: &TerminalState, expected: &str) {
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
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Missing remote terminal output"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
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
        columns: 80, rows: 24 };
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
