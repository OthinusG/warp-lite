//! Opt-in real Windows-to-WSL acceptance against a disposable Linux fixture.
#![cfg(windows)]
use remote_protocol::proto::*;
use warp_agent_bus::{ssh_files::{RemoteFiles, SshConnection}, ssh_remote::{HostClient, RemoteShell, SshProfile}};

#[tokio::test]
#[ignore = "requires a provisioned disposable WSL guest"]
async fn real_wsl_workspace_preserves_files_conflicts_git_and_guest_authority() {
    let distribution = std::env::var("WARP_TEST_WSL_DISTRIBUTION").expect("Controlled WSL distribution");
    let user = std::env::var("WARP_TEST_WSL_USER").expect("Controlled WSL user");
    let root = std::env::var("WARP_TEST_WSL_ROOT").expect("Controlled Linux project");
    let companion = std::env::var("WARP_TEST_WSL_COMPANION").expect("Source-matched Linux Companion");
    let profile = SshProfile { target: "wsl".into(), config_file: None,
        remote_root: root.clone(), companion_path: companion, remote_shell: RemoteShell::Posix };
    let connection = SshConnection::Wsl { distribution: distribution.clone(), user: user.clone() };
    let files = RemoteFiles::connect(profile.clone(), connection.clone()).await.unwrap();
    assert!(files.identity.contains(&connection.scope_key()));
    let snapshot = files.list(&root, 1).await.unwrap();
    assert!(!snapshot.entries.is_empty());
    let mut other_account = HostClient::connect_wsl(&profile, &distribution, "warpai-other").await.unwrap();
    let original_account = connection.connect(&profile).await.unwrap();
    assert_ne!(other_account.account_id, original_account.account_id);
    assert_ne!(other_account.root_identity, original_account.root_identity);
    other_account.disconnect();
    let path = format!("{root}/binary.dat");
    let (local, hash) = files.download(&path).await.unwrap();
    assert_eq!(std::fs::read(local).unwrap(), b"original\0bytes\xff");
    let (base, _) = files.download_base(&path, "HEAD").await.unwrap();
    assert_eq!(std::fs::read(base).unwrap(), b"original\0bytes\xff");
    let bytes = (0..150_000).map(|index| (index % 256) as u8).collect::<Vec<_>>();
    let updated = files.save(&path, &bytes, &hash).await.unwrap();
    let (local, observed) = files.download(&path).await.unwrap();
    assert_eq!(observed, updated);
    assert_eq!(std::fs::read(local).unwrap(), bytes);
    assert_eq!(files.save(&path, b"stale", &hash).await.unwrap_err(), warp_agent_bus::ssh_remote::ConnectionError::Conflict);
    assert!(files.download(&format!("{root}/../outside")).await.is_err());
    files.control(ProjectFilesRequest { action: ProjectFileAction::ProjectGitStatus as i32, ..Default::default() }).await.unwrap();
    let mut client = connection.connect(&profile).await.unwrap();
    assert!(client.account_id.starts_with("uid:"));
    let boot = client.fence().unwrap().service_boot_id.clone();
    drop(client);
    client = connection.connect(&profile).await.unwrap();
    assert_eq!(client.fence().unwrap().service_boot_id, boot);
    // A separate control attachment must not own or terminate the Agent's PTY.
    let state = client.terminal_launch(TerminalLaunch {
        fence: client.fence().cloned(), session_id: uuid::Uuid::new_v4().to_string(),
        executable: "/bin/sh".into(), arguments: vec!["-c".into(), "printf 'LAUNCH_CWD=%s\\n' \"$PWD\"; exec /bin/cat".into()], columns: 80, rows: 24, agent_program: None, working_directory: Some("/home/warpai-test".into()),
    }).await.unwrap();
    let other = connection.connect(&profile).await.unwrap();
    drop(other);
    client.terminal_control(TerminalControl { fence: client.fence().cloned(), session_id: state.session_id.clone(),
        run_id: state.run_id.clone(), action: TerminalAction::TerminalInput as i32, input: b"WSL owned input\n".to_vec(),
        input_sequence: 1, ..Default::default() }).await.unwrap();
    let mut seen = false;
    for _ in 0..50 {
        let reply = client.terminal_control(TerminalControl { fence: client.fence().cloned(), session_id: state.session_id.clone(),
            run_id: state.run_id.clone(), action: TerminalAction::TerminalRead as i32, ..Default::default() }).await.unwrap();
        if String::from_utf8_lossy(&reply.output).contains("WSL owned input")
            && String::from_utf8_lossy(&reply.output).contains("LAUNCH_CWD=/home/warpai-test") { seen = true; break; }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(seen);
    client.terminal_control(TerminalControl { fence: client.fence().cloned(), session_id: state.session_id,
        run_id: state.run_id, action: TerminalAction::TerminalStop as i32, ..Default::default() }).await.unwrap();
    let agent = client.terminal_launch(TerminalLaunch {
        fence: client.fence().cloned(), session_id: uuid::Uuid::new_v4().to_string(),
        executable: "/home/warpai-test/managed-agent-fixture".into(),
        arguments: ["--exact", "managed_agent_child", "--ignored", "--nocapture"].map(str::to_owned).to_vec(),
        columns: 240, rows: 24, agent_program: Some("fixture".into()), working_directory: None,
    }).await.unwrap();
    let mut registered = false;
    for _ in 0..450 {
        let reply = client.terminal_control(TerminalControl { fence: client.fence().cloned(),
            session_id: agent.session_id.clone(), run_id: agent.run_id.clone(),
            action: TerminalAction::TerminalRead as i32, ..Default::default() }).await.unwrap();
        if String::from_utf8_lossy(&reply.output).contains(&format!("MCP_NATIVE_RUN={}", agent.run_id)) {
            registered = true; break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(registered, "WSL Agent must complete native MCP registration");
    use warp_agent_bus::{companion::TaskCommand, transport::PanelQuery, ControllerOperation};
    let panel = TaskCommand::Panel(PanelQuery { worktree: true, ..Default::default() });
    let mut observer = connection.connect(&profile).await.unwrap();
    let before = observer.project_tasks(&panel, 1).await.unwrap();
    let candidate = before["value"]["candidates"].as_array().unwrap().iter()
        .find(|candidate| candidate["run"].as_str() == Some(&agent.run_id)).expect("Live guest candidate");
    let selected = observer.project_tasks(&TaskCommand::Controller(ControllerOperation::WorktreeCoordinator {
        root: root.clone(), agent: candidate["agent"]["id"].as_str().unwrap().into(),
        run: agent.run_id.clone(), request_id: uuid::Uuid::new_v4().to_string(),
    }), 2).await.unwrap();
    assert!(selected.get("error").is_none(), "{selected}");
    assert_eq!(observer.project_tasks(&panel, 3).await.unwrap()["value"]["coordinator_online"], true);
    drop(observer);
    let mut observer = connection.connect(&profile).await.unwrap();
    assert_eq!(observer.project_tasks(&panel, 4).await.unwrap()["value"]["coordinator_online"], true);
    client.terminal_control(TerminalControl { fence: client.fence().cloned(), session_id: agent.session_id,
        run_id: agent.run_id, action: TerminalAction::TerminalStop as i32, ..Default::default() }).await.unwrap();
    let mut cleared = false;
    for generation in 5..55 {
        if observer.project_tasks(&panel, generation).await.unwrap()["value"]["coordinator_online"] == false {
            cleared = true; break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(cleared, "Only actual Agent exit clears Coordinator liveness");
    files.disconnect();
    files.reconnect().await.unwrap();
    assert_eq!(files.download(&path).await.unwrap().1, updated);
    let mut invalid = profile;
    invalid.companion_path = "/nonexistent-owned-warpai-companion".into();
    assert!(HostClient::connect_wsl(&invalid, &distribution, &user).await.is_err());
}
