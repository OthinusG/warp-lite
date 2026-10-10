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
    let mut other_profile = profile.clone();
    other_profile.remote_root = "/home/warpai-other/project".into();
    other_profile.companion_path = "/home/warpai-other/.config/.warpai/wsl/bin/warpai-wsl-companion".into();
    let mut other_account = HostClient::connect_wsl(&other_profile, &distribution, "warpai-other").await.unwrap();
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
    // Ordinary Linux Agent in its shell, never Companion TerminalLaunch or Agent wrapper.
    // Preserve only the owned fixture's output so startup failures identify the failed guard.
    let native_log = tempfile::NamedTempFile::new().unwrap();
    let mut native = tokio::process::Command::new("wsl.exe")
        .args(["--distribution", &distribution, "--user", &user, "--exec", "python3",
            "/home/warpai-test/native-agent-fixture.py", "--terminal", &profile.companion_path, &root])
        .stdin(std::process::Stdio::piped()).stdout(native_log.as_file().try_clone().unwrap())
        .stderr(native_log.as_file().try_clone().unwrap()).kill_on_drop(true).spawn().unwrap();
    let metadata_path = format!("{root}/native-session.json");
    let mut metadata = None;
    for _ in 0..100 {
        if let Ok((path, _)) = files.download(&metadata_path).await {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).unwrap()) {
                metadata = Some(value); break;
            }
        }
        if native.try_wait().unwrap().is_some() { break; }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let metadata = metadata.unwrap_or_else(|| {
        use std::io::Read;
        let mut output = Vec::new();
        native_log.reopen().unwrap().take(16384).read_to_end(&mut output).unwrap();
        panic!("Ordinary guest MCP must register: {}", String::from_utf8_lossy(&output));
    });
    let shell_pid = metadata["shell_pid"].as_u64().unwrap() as u32;
    use warp_agent_bus::{companion::TaskCommand, transport::PanelQuery, ControllerOperation,
        wsl_setup::{NativeAction, NativeRequest, NativeState}};
    let observe = |draft, blocked, epoch, submitted, cancelled| TaskCommand::GuestNative(NativeRequest {
        shell_pid, action: NativeAction::Observe { draft, blocked, input_epoch: epoch,
            submitted, cancelled, native_ready: None },
    });
    let native_command = |action| TaskCommand::GuestNative(NativeRequest { shell_pid, action });
    let state = client.project_tasks(&observe(false, false, 0, false, false), 1).await.unwrap();
    let run = state["value"]["run"].as_str().expect("Native process-owned run").to_owned();
    let panel = TaskCommand::Panel(PanelQuery { worktree: true, ..Default::default() });
    let mut observer = connection.connect(&profile).await.unwrap();
    let before = observer.project_tasks(&panel, 1).await.unwrap();
    let candidate = before["value"]["candidates"].as_array().unwrap().iter()
        .find(|candidate| candidate["run"].as_str() == Some(run.as_str())).expect("Live native guest candidate");
    let agent_id = candidate["agent"]["id"].as_str().unwrap().to_owned();
    let selected = observer.project_tasks(&TaskCommand::Controller(ControllerOperation::WorktreeCoordinator {
        root: root.clone(), agent: agent_id.clone(), run: run.clone(), request_id: uuid::Uuid::new_v4().to_string(),
    }), 2).await.unwrap();
    assert!(selected.get("error").is_none(), "{selected}");
    assert_eq!(observer.project_tasks(&panel, 3).await.unwrap()["value"]["coordinator_online"], true);
    drop(observer);
    let mut observer = connection.connect(&profile).await.unwrap();
    let refreshed = observer.project_tasks(&panel, 4).await.unwrap();
    assert_eq!(refreshed["value"]["coordinator_online"], true);
    let scope = refreshed["value"]["collaboration_scope"].as_str().unwrap().to_owned();
    let message = observer.project_tasks(&TaskCommand::Scoped { scope, command: Box::new(TaskCommand::Operator(
        warp_agent_bus::Operation::AgentSend { to: agent_id, body: "Native fixture work".into(), subject: None,
            thread_id: None, reply_to: None, task_id: None, request_id: uuid::Uuid::new_v4().to_string() }
    )) }, 5).await.unwrap();
    assert!(message.get("error").is_none(), "{message}");
    assert!(client.project_tasks(&observe(true, true, 1, false, false), 6).await.unwrap()["value"]["wake"].is_null());
    client.project_tasks(&observe(false, false, 2, false, false), 7).await.unwrap();
    async fn next_wake(client: &mut HostClient, command: &TaskCommand) -> warp_agent_bus::transport::Wake {
        for generation in 8..108 {
            let value = client.project_tasks(command, generation).await.unwrap();
            let state: NativeState = serde_json::from_value(value["value"].clone()).unwrap();
            if let Some(wake) = state.wake { return wake; }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("Ready native Agent must receive queued work")
    }
    let wake = next_wake(&mut client, &observe(false, false, 2, false, false)).await;
    assert_eq!(client.project_tasks(&native_command(NativeAction::Claim { wake: wake.clone() }), 109).await.unwrap()["value"]["valid"], true);
    // Manual editing and cancellation invalidate an already claimed delayed Enter.
    client.project_tasks(&observe(true, false, 3, false, true), 110).await.unwrap();
    assert_eq!(client.project_tasks(&native_command(NativeAction::Validate { wake: wake.clone() }), 111).await.unwrap()["value"]["valid"], false);
    client.project_tasks(&native_command(NativeAction::Finish { wake, submitted: false }), 112).await.unwrap();
    client.project_tasks(&observe(false, false, 4, true, false), 113).await.unwrap();
    use tokio::io::AsyncWriteExt;
    native.stdin.as_mut().unwrap().write_all(b"OWNED_READY\n").await.unwrap();
    let wake = next_wake(&mut client, &observe(false, false, 4, false, false)).await;
    assert_eq!(client.project_tasks(&native_command(NativeAction::Claim { wake: wake.clone() }), 114).await.unwrap()["value"]["valid"], true);
    assert_eq!(client.project_tasks(&native_command(NativeAction::Validate { wake: wake.clone() }), 115).await.unwrap()["value"]["valid"], true);
    native.stdin.as_mut().unwrap().write_all(format!("Warpai peer work is waiting (message {}). Call warp_agent_inbox.\n", wake.message_id).as_bytes()).await.unwrap();
    client.project_tasks(&native_command(NativeAction::Finish { wake, submitted: true }), 116).await.unwrap();
    let mut delivered = false;
    for _ in 0..100 {
        let (path, _) = files.download(&metadata_path).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        if value["wakes"] == 1 { delivered = true; break; }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(delivered, "Ordinary Linux Agent must consume and acknowledge peer work through MCP");
    drop(native.stdin.take());
    tokio::time::timeout(std::time::Duration::from_secs(10), native.wait()).await.unwrap().unwrap();
    let mut cleared = false;
    for generation in 117..167 {
        if observer.project_tasks(&panel, generation).await.unwrap()["value"]["coordinator_online"] == false {
            cleared = true; break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(cleared, "Only actual native Agent exit clears Coordinator liveness");
    files.disconnect();
    files.reconnect().await.unwrap();
    assert_eq!(files.download(&path).await.unwrap().1, updated);
    let mut invalid = profile;
    invalid.companion_path = "/nonexistent-owned-warpai-companion".into();
    assert!(HostClient::connect_wsl(&invalid, &distribution, &user).await.is_err());
}
