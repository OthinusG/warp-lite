use super::*;

fn native_shell() -> Command {
    let mut command = Command::new("cmd.exe");
    command.args(["/d", "/s", "/c"]).kill_on_drop(true);
    command
}

#[tokio::test]
#[ignore = "requires a companion installed on the disposable Windows runner"]
async fn native_windows_remote_shell_preserves_companion_protocol_bytes() {
    assert_eq!(std::env::var("GITHUB_ACTIONS").unwrap(), "true");
    let root = tempfile::Builder::new()
        .prefix("SSH project with spaces 多语言")
        .tempdir()
        .unwrap();
    let profile = SshProfile {
        target: "native-shell-fixture".into(),
        config_file: None,
        remote_root: root.path().to_str().unwrap().into(),
        companion_path: std::env::var("WARP_TEST_COMPANION_PATH").unwrap(),
        remote_shell: RemoteShell::PowerShell,
    };
    HostClient::probe_installed(&profile, native_shell())
        .await
        .expect("Native shell version probe");
    let mut client = HostClient::open_command(&profile, native_shell())
        .await
        .expect("Native shell binary handshake and project binding");
    let query = crate::companion::TaskCommand::Panel(Default::default());
    let result = client.project_tasks(&query, 1).await.unwrap();
    assert_eq!(
        result["value"]["project"].as_str(),
        Some(client.fence().unwrap().project_id.as_str())
    );
    assert!(result["value"]["tasks"].is_array());
}
