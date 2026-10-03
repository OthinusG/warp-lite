//! Real companion, native PTY and disconnect/replacement acceptance.
use remote_protocol::{managed::PROTOCOL_MAJOR, proto::*, protocol::*};
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use uuid::Uuid;

#[test]
#[ignore = "only the owned companion PTY launches this child"]
fn retained_terminal_child() {
    #[cfg(unix)]
    assert_eq!(unsafe { libc::isatty(0) }, 1);
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::*;
        let mut mode = CONSOLE_MODE::default();
        GetConsoleMode(GetStdHandle(STD_INPUT_HANDLE).unwrap(), &mut mode).unwrap();
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
async fn retained_terminal_survives_disconnect_and_fences_replay_input_and_owned_stop() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("Retained root 多语言");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let mut first = Attachment::open(&root).await;
    let launch = TerminalLaunch {
        fence: Some(first.fence.clone()),
        session_id: Uuid::new_v4().to_string(),
        executable: std::env::current_exe().unwrap().to_str().unwrap().into(),
        arguments: [
            "--exact",
            "retained_terminal_child",
            "--ignored",
            "--nocapture",
        ]
        .map(str::to_owned)
        .to_vec(),
        columns: 80,
        rows: 24,
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
    let replaced = second
        .command(second.control(&state, TerminalAction::TerminalAttach))
        .await;
    assert!(replaced.attachment_generation > state.attachment_generation);
    for action in [
        TerminalAction::TerminalInput,
        TerminalAction::TerminalResize,
        TerminalAction::TerminalStop,
    ] {
        assert!(matches!(
            first
                .call(managed_request::Operation::TerminalControl(
                    first.control(&state, action)
                ))
                .await,
            managed_response::Result::Error(_)
        ));
    }
    first.disconnect().await;
    let mut resize = second.control(&replaced, TerminalAction::TerminalResize);
    resize.columns = 120;
    resize.rows = 40;
    second.command(resize).await;
    let mut flood = second.control(&replaced, TerminalAction::TerminalInput);
    flood.input = b"flood\r".to_vec();
    flood.input_sequence = 1;
    second.command(flood).await;
    let bounded = second
        .wait_output(&replaced, "BOUNDED_REPLAY_VERIFIED")
        .await;
    assert!(bounded.output_truncated);
    assert!(bounded.output.len() <= 32 * 1024);
    assert!(bounded.output_end - bounded.output_offset <= 256 * 1024);
    let original_boot = second.fence.service_boot_id.clone();
    second.disconnect().await;
    let mut next = Attachment::open(&root).await;
    assert_eq!(next.fence.service_boot_id, original_boot);
    let managed_response::Result::TerminalStates(list) = next
        .call(managed_request::Operation::TerminalList(TerminalList {
            fence: Some(next.fence.clone()),
        }))
        .await
    else {
        panic!("Retained list")
    };
    let retained = list
        .sessions
        .into_iter()
        .find(|s| s.session_id == state.session_id)
        .unwrap();
    assert_eq!(retained.run_id, state.run_id);
    assert_eq!(retained.exit_code, None);
    let retained = next
        .command(next.control(&retained, TerminalAction::TerminalAttach))
        .await;
    let other_root = tempfile::tempdir().unwrap();
    let mut other = Attachment::open(other_root.path()).await;
    assert!(matches!(
        other
            .call(managed_request::Operation::TerminalControl(
                other.control(&retained, TerminalAction::TerminalAttach)
            ))
            .await,
        managed_response::Result::Error(_)
    ));
    other.disconnect().await;
    let stopped = next
        .command(next.control(&retained, TerminalAction::TerminalStop))
        .await;
    assert!(stopped.stop_requested);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let observed = next
            .command(next.control(&retained, TerminalAction::TerminalRead))
            .await;
        if observed.exit_code.is_some() && observed.output_closed {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "Stop did not exit");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    next.command(next.control(&retained, TerminalAction::TerminalRelease))
        .await;
    next.disconnect().await;
}
