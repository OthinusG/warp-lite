//! Real companion Agent PTY, private MCP and connection ownership acceptance.
use remote_protocol::{managed::PROTOCOL_MAJOR, proto::*, protocol::*};
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use uuid::Uuid;

#[test]
#[ignore = "only the owned companion PTY launches this child"]
fn managed_agent_child() {
    #[cfg(unix)]
    assert_eq!(unsafe { libc::isatty(0) }, 1);
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::*;
        let mut mode = CONSOLE_MODE::default();
        GetConsoleMode(GetStdHandle(STD_INPUT_HANDLE).unwrap(), &mut mode).unwrap();
    }
    if std::env::var_os(warp_agent_bus::transport::CAPABILITY).is_some() {
        use std::io::{BufRead, Write};
        let mut bridge = std::process::Command::new(std::env::var_os("WARP_AGENT_BIN").unwrap())
            .arg("mcp")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut input = bridge.stdin.take().unwrap();
        for message in [
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"owned-remote-fixture","version":"1"}}}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ] {
            writeln!(input, "{message}").unwrap();
        }
        input.flush().unwrap();
        let mut output = std::io::BufReader::new(bridge.stdout.take().unwrap());
        for id in [1, 2] {
            let mut line = String::new();
            output.read_line(&mut line).unwrap();
            let response: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(response["id"], id);
            assert!(response.get("error").is_none());
            if id == 2 {
                assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 30);
            }
        }
        drop(input);
        bridge.wait().unwrap();
        use warp_agent_bus::transport::*;
        let registered = call(
            &std::env::var(ENDPOINT).unwrap(),
            &Request {
                protocol_major: PROTOCOL_MAJOR,
                terminal: std::env::var(TERMINAL).unwrap(),
                capability: std::env::var(CAPABILITY).unwrap(),
                run: None,
                defer_initial_ready: false,
                native_activity: None,
                directory: None,
                operation: warp_agent_bus::Operation::AgentRegister {
                    name: "fixture".into(),
                },
            },
        )
        .unwrap();
        println!("MCP_NATIVE_RUN={}", registered["run"].as_str().unwrap());
    }
    println!("REMOTE_ROOT={}", std::env::current_dir().unwrap().display());
    loop {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).unwrap() == 0 {
            break;
        }
        match line.trim_end() {
            "Unicode 空格;$(never-execute)" => println!("UNICODE_INPUT_VERIFIED"),
            "flood" => {
                println!("{}", "x".repeat(300_000));
                println!("BOUNDED_REPLAY_VERIFIED");
            }
            _ => panic!("Unexpected terminal input"),
        }
    }
}

struct Attachment {
    child: tokio::process::Child,
    input: Compat<tokio::process::ChildStdin>,
    output: Compat<tokio::process::ChildStdout>,
    fence: ManagedFence,
}

#[test]
#[ignore = "owned retained background child fixture"]
fn retained_background_child() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
    std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "retained_background_sleep",
            "--ignored",
            "--nocapture",
        ])
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !std::path::Path::new("retained-background-ready").exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
#[ignore = "owned retained background sleeper fixture"]
fn retained_background_sleep() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
    std::fs::write("retained-background-ready", "ready").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(30));
}

#[tokio::test]
async fn background_activity_blocks_release_until_owned_stop() {
    let root = tempfile::tempdir().unwrap();
    let mut first = Attachment::open(root.path()).await;
    let managed_response::Result::TerminalState(state) = first
        .call(managed_request::Operation::TerminalLaunch(TerminalLaunch {
            fence: Some(first.fence.clone()),
            session_id: Uuid::new_v4().to_string(),
            executable: std::env::current_exe().unwrap().to_str().unwrap().into(),
            arguments: [
                "--exact",
                "retained_background_child",
                "--ignored",
                "--nocapture",
            ]
            .map(str::to_owned)
            .to_vec(),
            columns: 100,
            rows: 24,
            agent_program: None,
        }))
        .await
    else {
        panic!("Background owner required")
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let ended = loop {
        let current = first
            .command(first.control(&state, TerminalAction::TerminalRead))
            .await;
        if current.exit_code.is_some() {
            break current;
        }
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert_eq!(ended.processes_active, Some(true));
    assert!(
        matches!(first.call(managed_request::Operation::TerminalControl(first.control(&ended, TerminalAction::TerminalRelease))).await, managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedConflict as i32)
    );
    first
        .command(first.control(&ended, TerminalAction::TerminalStop))
        .await;
    loop {
        let current = first
            .command(first.control(&ended, TerminalAction::TerminalRead))
            .await;
        if current.processes_active == Some(false) && current.output_closed {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Owned background process did not stop"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    first
        .command(first.control(&ended, TerminalAction::TerminalRelease))
        .await;
    first.disconnect().await;
}
impl Attachment {
    async fn open(root: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_warpai-companion"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut attachment = Self {
            input: child.stdin.take().unwrap().compat_write(),
            output: child.stdout.take().unwrap().compat(),
            child,
            fence: ManagedFence::default(),
        };
        let managed_response::Result::Initialized(initialized) = attachment
            .call(managed_request::Operation::Initialize(ManagedInitialize {
                protocol_major: PROTOCOL_MAJOR,
            }))
            .await
        else {
            panic!("Expected initialize")
        };
        let managed_response::Result::ProjectOpened(opened) = attachment
            .call(managed_request::Operation::ProjectOpen(ProjectOpen {
                fence: initialized.fence,
                root: root.to_str().unwrap().into(),
            }))
            .await
        else {
            panic!("Expected project")
        };
        attachment.fence = opened.fence.unwrap();
        attachment
    }
    async fn call(&mut self, operation: managed_request::Operation) -> managed_response::Result {
        let id = Uuid::new_v4().to_string();
        let request = ClientMessage {
            request_id: id.clone(),
            message: Some(client_message::Message::Managed(ManagedRequest {
                operation: Some(operation),
            })),
        };
        write_message_with_limit(&mut self.input, &request, MAX_MANAGED_MESSAGE_SIZE)
            .await
            .unwrap();
        let reply: ServerMessage = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            read_message_with_limit(&mut self.output, MAX_MANAGED_MESSAGE_SIZE),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(reply.request_id, id);
        let Some(server_message::Message::Managed(reply)) = reply.message else {
            panic!("Managed reply")
        };
        reply.result.unwrap()
    }
    fn control(&self, state: &TerminalState, action: TerminalAction) -> TerminalControl {
        TerminalControl {
            fence: Some(self.fence.clone()),
            session_id: state.session_id.clone(),
            run_id: state.run_id.clone(),
            attachment_generation: state.attachment_generation,
            action: action.into(),
            ..TerminalControl::default()
        }
    }
    async fn command(&mut self, command: TerminalControl) -> TerminalState {
        let managed_response::Result::TerminalState(state) = self
            .call(managed_request::Operation::TerminalControl(command))
            .await
        else {
            panic!("Terminal state")
        };
        state
    }
    async fn wait_output(&mut self, state: &TerminalState, marker: &str) -> TerminalState {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let result = self
                .command(self.control(state, TerminalAction::TerminalRead))
                .await;
            let mut offset = result.output_offset;
            let mut bytes = result.output.clone();
            let mut found = String::from_utf8_lossy(&bytes).contains(marker);
            while offset + (bytes.len() as u64) < result.output_end {
                offset += bytes.len() as u64;
                let mut command = self.control(state, TerminalAction::TerminalRead);
                command.output_offset = offset;
                let page = self.command(command).await;
                if page.output.is_empty() {
                    break;
                }
                bytes = page.output;
                found |= String::from_utf8_lossy(&bytes).contains(marker);
            }
            if found {
                return result;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Missing terminal marker"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }
    async fn disconnect(mut self) {
        self.input.get_mut().shutdown().await.unwrap();
        drop(self.input);
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(10), self.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
    }
}

#[tokio::test]
async fn managed_agent_fences_input_project_and_owned_stop() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("Retained root 多语言");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let mut first = Attachment::open(&root).await;
    let launch = TerminalLaunch {
        fence: Some(first.fence.clone()),
        session_id: Uuid::new_v4().to_string(),
        executable: std::env::current_exe().unwrap().to_str().unwrap().into(),
        arguments: ["--exact", "managed_agent_child", "--ignored", "--nocapture"]
            .map(str::to_owned)
            .to_vec(),
        columns: 240,
        rows: 24,
        agent_program: Some("fixture".into()),
    };
    let managed_response::Result::TerminalState(state) = first
        .call(managed_request::Operation::TerminalLaunch(launch.clone()))
        .await
    else {
        panic!("Launch")
    };
    let managed_response::Result::TerminalState(repeated) = first
        .call(managed_request::Operation::TerminalLaunch(launch.clone()))
        .await
    else {
        panic!("Replay launch")
    };
    first
        .wait_output(&state, &format!("MCP_NATIVE_RUN={}", state.run_id))
        .await;
    assert_eq!(state.run_id, repeated.run_id);
    let mut changed = launch;
    changed.rows = 30;
    assert!(
        matches!(first.call(managed_request::Operation::TerminalLaunch(changed)).await,
        managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedConflict as i32)
    );
    first
        .wait_output(&state, &format!("REMOTE_ROOT={}", root.display()))
        .await;
    let mut input = first.control(&state, TerminalAction::TerminalInput);
    input.input = "Unicode 空格;$(never-execute)\r".as_bytes().to_vec();
    input.input_sequence = 1;
    first.command(input.clone()).await;
    assert!(matches!(
        first
            .call(managed_request::Operation::TerminalControl(input))
            .await,
        managed_response::Result::Error(_)
    ));
    first.wait_output(&state, "UNICODE_INPUT_VERIFIED").await;
    let mut second = Attachment::open(&root).await;
    assert_eq!(first.fence.service_boot_id, second.fence.service_boot_id);
    assert!(
        matches!(second.call(managed_request::Operation::TerminalControl(
        second.control(&state, TerminalAction::TerminalRead))).await,
        managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedStaleAttachment as i32)
    );
    let other_root = tempfile::tempdir().unwrap();
    let mut other = Attachment::open(other_root.path()).await;
    assert!(matches!(
        other
            .call(managed_request::Operation::TerminalControl(
                other.control(&state, TerminalAction::TerminalRead)
            ))
            .await,
        managed_response::Result::Error(_)
    ));
    other.disconnect().await;
    let mut resize = first.control(&state, TerminalAction::TerminalResize);
    resize.columns = 120;
    resize.rows = 40;
    first.command(resize).await;
    let stopped = first
        .command(first.control(&state, TerminalAction::TerminalStop))
        .await;
    assert!(stopped.stop_requested);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let observed = first
            .command(first.control(&state, TerminalAction::TerminalRead))
            .await;
        if observed.processes_active == Some(false) && observed.output_closed {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "Stop did not exit");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    first
        .command(first.control(&state, TerminalAction::TerminalRelease))
        .await;
    first.disconnect().await;
    second.disconnect().await;
}

#[tokio::test]
async fn disconnected_agent_cannot_be_adopted_by_another_connection() {
    let root = tempfile::tempdir().unwrap();
    let mut first = Attachment::open(root.path()).await;
    let launch = TerminalLaunch {
        fence: Some(first.fence.clone()),
        session_id: Uuid::new_v4().to_string(),
        executable: std::env::current_exe().unwrap().to_str().unwrap().into(),
        arguments: ["--exact", "managed_agent_child", "--ignored", "--nocapture"]
            .map(str::to_owned)
            .to_vec(),
        columns: 100,
        rows: 24,
        agent_program: Some("fixture".into()),
    };
    let managed_response::Result::TerminalState(state) = first
        .call(managed_request::Operation::TerminalLaunch(launch.clone()))
        .await
    else {
        panic!("Launch")
    };
    first
        .wait_output(&state, &format!("MCP_NATIVE_RUN={}", state.run_id))
        .await;
    first.disconnect().await;
    let mut next = Attachment::open(root.path()).await;
    for action in [
        TerminalAction::TerminalRead,
        TerminalAction::TerminalInput,
        TerminalAction::TerminalStop,
    ] {
        assert!(matches!(
            next.call(managed_request::Operation::TerminalControl(
                next.control(&state, action)
            ))
            .await,
            managed_response::Result::Error(_)
        ));
    }
    next.disconnect().await;
}
