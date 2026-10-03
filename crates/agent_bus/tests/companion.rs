//! Real process/stdio smoke test; no vendor account or SSH enrollment required.
use remote_protocol::{managed::PROTOCOL_MAJOR, proto::*, protocol::*};
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use uuid::Uuid;

#[tokio::test]
async fn standalone_companion_negotiates_clean_bounded_stdio_and_exits_on_disconnect() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_warpai-companion"))
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null()).kill_on_drop(true).spawn().unwrap();
    let mut input = child.stdin.take().unwrap().compat_write();
    let mut output = child.stdout.take().unwrap().compat();
    let request_id = Uuid::new_v4().to_string();
    let request = ClientMessage { request_id: request_id.clone(), message: Some(client_message::Message::Managed(
        ManagedRequest { operation: Some(managed_request::Operation::Initialize(ManagedInitialize { protocol_major: PROTOCOL_MAJOR })) },
    )) };
    write_message_with_limit(&mut input, &request, MAX_MANAGED_MESSAGE_SIZE).await.unwrap();
    let reply: ServerMessage = tokio::time::timeout(std::time::Duration::from_secs(10),
        read_message_with_limit(&mut output, MAX_MANAGED_MESSAGE_SIZE)).await.unwrap().unwrap();
    assert_eq!(reply.request_id, request_id);
    assert!(matches!(reply.message, Some(server_message::Message::Managed(ManagedResponse {
        result: Some(managed_response::Result::Initialized(_)),
    }))));
    input.get_mut().shutdown().await.unwrap();
    drop(input);
    assert!(tokio::time::timeout(std::time::Duration::from_secs(10), child.wait()).await.unwrap().unwrap().success());
}
