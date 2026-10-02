use super::*;
use std::{ffi::OsStr, time::Duration};

fn hello() -> NegotiationFrame {
    NegotiationFrame::Hello {
        protocol_major: PROTOCOL_MAJOR,
        protocol_minor: 9,
        features: FEATURES
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        max_frame_bytes: 65536,
    }
}

#[test]
fn negotiation_rejects_downgrade_and_never_grants_identity() {
    use serde_json::{json, Value};
    let coordinator = uuid::Uuid::new_v4();
    let first = serde_json::to_value(hello().negotiate(coordinator).unwrap()).unwrap();
    let second = serde_json::to_value(hello().negotiate(coordinator).unwrap()).unwrap();
    assert_eq!(first["type"], "hello_result");
    assert_eq!(first["protocol_minor"], 0);
    assert_eq!(first["max_frame_bytes"], 65536);
    assert_eq!(first["coordinator_id"], coordinator.to_string());
    assert_eq!(first["features"], json!(FEATURES));
    assert_ne!(first["connection_epoch"], second["connection_epoch"]);
    assert!(first.get("device_id").is_none());
    assert!(first.get("grants").is_none());
    let mut newer = serde_json::to_value(hello()).unwrap();
    newer["features"]
        .as_array_mut()
        .unwrap()
        .push(json!("future_feature"));
    let negotiated = serde_json::to_value(
        serde_json::from_value::<NegotiationFrame>(newer)
            .unwrap()
            .negotiate(coordinator)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(negotiated["features"], json!(FEATURES));
    assert!(serde_json::from_value::<NegotiationFrame>(first)
        .unwrap()
        .negotiate(coordinator)
        .is_err());
    for (field, replacement, code) in [
        ("protocol_major", json!(99), "protocol_incompatible"),
        ("features", json!([]), "feature_unavailable"),
        (
            "features",
            json!(["task_control", "task_control"]),
            "invalid_input",
        ),
        ("features", json!(["bad feature"]), "invalid_input"),
        ("max_frame_bytes", json!(0), "invalid_input"),
        (
            "max_frame_bytes",
            json!(crate::MAX_FRAME + 1),
            "invalid_input",
        ),
    ] {
        let mut value: Value = serde_json::to_value(hello()).unwrap();
        value[field] = replacement;
        let error = serde_json::from_value::<NegotiationFrame>(value)
            .unwrap()
            .negotiate(coordinator)
            .err()
            .unwrap();
        assert_eq!(
            error.downcast_ref::<crate::DomainError>().unwrap().code,
            code
        );
    }
    let mut unknown = serde_json::to_value(hello()).unwrap();
    unknown["terminal_capability"] = json!("must-not-be-forwarded");
    assert!(serde_json::from_value::<NegotiationFrame>(unknown).is_err());
    assert!(serde_json::from_value::<NegotiationFrame>(json!({"type":"execute_command"})).is_err());
}

#[tokio::test]
async fn bounded_hello_frames_handle_partial_io_and_reject_untrusted_payloads() {
    use crate::transport::{receive, send};
    use std::time::Instant;
    use tokio::io::AsyncWriteExt;
    let payload = serde_json::to_vec(&hello()).unwrap();
    assert_eq!(payload, br#"{"type":"hello","protocol_major":2,"protocol_minor":9,"features":["task_control","dependencies","threads","reservations","evidence_refs","event_resume"],"max_frame_bytes":65536}"#);
    let mut framed = (payload.len() as u32).to_be_bytes().to_vec();
    framed.extend_from_slice(&payload);
    let (mut reader, mut writer) = tokio::io::duplex(64);
    let fragmented = tokio::spawn(async move {
        for chunk in framed.chunks(3) {
            writer.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    let frame: NegotiationFrame = receive(&mut reader, Instant::now() + Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(serde_json::to_vec(&frame).unwrap(), payload);
    fragmented.await.unwrap();
    for size in [0, crate::MAX_FRAME as u32 + 1] {
        let mut header = std::io::Cursor::new(size.to_be_bytes());
        assert!(
            receive::<NegotiationFrame>(&mut header, Instant::now() + Duration::from_secs(1))
                .await
                .is_err()
        );
    }
    for truncated in [vec![0, 0], vec![0, 0, 0, 20, b'{']] {
        assert!(receive::<NegotiationFrame>(
            &mut std::io::Cursor::new(truncated),
            Instant::now() + Duration::from_secs(1)
        )
        .await
        .is_err());
    }
    for invalid in [
        b"not-json".as_slice(),
        b"{\"type\":\"execute_command\",\"credential\":\"do-not-echo\"}".as_slice(),
    ] {
        let mut bytes = (invalid.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(invalid);
        let error = receive::<NegotiationFrame>(
            &mut std::io::Cursor::new(bytes),
            Instant::now() + Duration::from_secs(1),
        )
        .await
        .err()
        .unwrap();
        assert!(!error.to_string().contains("do-not-echo"));
    }
    let (mut stalled, _held_open) = tokio::io::duplex(16);
    assert!(
        receive::<NegotiationFrame>(&mut stalled, Instant::now() + Duration::from_millis(25))
            .await
            .is_err()
    );
    let oversized = "x".repeat(crate::MAX_FRAME + 1);
    assert!(send(
        &mut tokio::io::sink(),
        &oversized,
        Instant::now() + Duration::from_secs(1)
    )
    .await
    .is_err());
}

#[test]
fn ssh_arguments_preserve_paths_and_reject_option_or_shell_injection() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("OpenSSH with spaces").join("ssh.exe");
    let command = build_ssh_command(&executable, "workstation-test").unwrap();
    assert_eq!(command.as_std().get_program(), executable.as_os_str());
    let args: Vec<_> = command.as_std().get_args().collect();
    assert_eq!(args[0], OsStr::new("-T"));
    assert_eq!(
        &args[args.len() - 2..],
        &[OsStr::new("workstation-test"), OsStr::new(GATEWAY_COMMAND)]
    );
    for option in SSH_OPTIONS {
        assert!(args
            .windows(2)
            .any(|pair| pair == [OsStr::new("-o"), OsStr::new(option)]));
    }
    for alias in [
        "",
        "-oProxyCommand=anything",
        "host;command",
        "host name",
        "host\ncommand",
        "host$(command)",
        "host`command`",
        "user@host",
        "host/relative",
        "主机",
    ] {
        let error = build_ssh_command(&executable, alias).err().unwrap();
        assert_eq!(
            error.downcast_ref::<crate::DomainError>().unwrap().code,
            "invalid_input"
        );
        assert!(!error.to_string().contains(alias) || alias.is_empty());
    }
    assert!(build_ssh_command(&executable, &"a".repeat(254)).is_err());
}

#[tokio::test]
async fn system_ssh_accepts_strict_background_options_without_connecting() {
    let executable = match ssh_executable() {
        Ok(executable) => executable,
        Err(error) => {
            assert_eq!(
                error.downcast_ref::<crate::DomainError>().unwrap().code,
                "feature_unavailable"
            );
            eprintln!("System OpenSSH unavailable; real executable acceptance remains unverified");
            return;
        }
    };
    // Close the handle before OpenSSH opens the file, including on Windows.
    let empty_config = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    let channel = build_ssh_command(&executable, "collaboration-test.invalid").unwrap();
    let mut probe = Command::new(&executable);
    // -G parses options without networking; the empty config avoids user hooks/credentials.
    probe
        .arg("-G")
        .arg("-F")
        .arg(empty_config.as_os_str())
        .args(channel.as_std().get_args())
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(10), probe.output())
        .await
        .expect("SSH configuration probe timed out")
        .expect("SSH probe failed to start");
    let diagnostic = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    let named_options: Vec<_> = SSH_OPTIONS
        .iter()
        .filter(|option| {
            diagnostic.contains(&option.split('=').next().unwrap().to_ascii_lowercase())
        })
        .collect();
    assert!(output.status.success(),
        "System SSH rejected options: exit={:?}, named_options={named_options:?}, config_access_error={}",
        output.status.code(), diagnostic.contains("permission") || diagnostic.contains("open configuration"));
    let configuration = String::from_utf8(output.stdout).unwrap();
    for setting in [
        "batchmode yes",
        "stricthostkeychecking true",
        "forwardagent no",
        "forwardx11 no",
        "clearallforwardings yes",
        "permitlocalcommand no",
        "requesttty false",
        "stdinnull no",
        "connecttimeout 10",
        "serveraliveinterval 10",
        "serveralivecountmax 3",
        "controlmaster false",
        "controlpersist no",
    ] {
        assert!(
            configuration.lines().any(|line| line == setting),
            "Missing safe SSH setting: {setting}"
        );
    }
}

#[test]
fn owned_stdio_client_checks_authority_and_authenticated_space() {
    use crate::transport::remote_control::AuthenticationFrame;
    use std::{io::Write, process::Stdio};
    let mut script = tempfile::NamedTempFile::new().unwrap();
    script.write_all(br#"
import json, struct, sys, uuid

def read():
    size = struct.unpack('>I', sys.stdin.buffer.read(4))[0]
    return json.loads(sys.stdin.buffer.read(size))

def send(value):
    data = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('>I', len(data)) + data)
    sys.stdout.buffer.flush()

hello = read()
epoch, device, space = str(uuid.uuid4()), str(uuid.uuid4()), sys.argv[2]
send(dict(type='hello_result', protocol_major=2, protocol_minor=0,
          features=hello['features'], max_frame_bytes=hello['max_frame_bytes'],
          coordinator_id=sys.argv[1], connection_epoch=epoch, presence_lease_ms=30000))
while True:
    frame = read()
    if frame['type'] == 'enroll':
        send(dict(type='enroll_result', device_id=device, credential='fixture-credential',
                  generation=1, space_ids=[space]))
    elif frame['type'] == 'authenticate':
        send(dict(type='authenticated', device_id=device, generation=1,
                  connection_epoch=epoch, space_ids=[space]))
    elif frame['type'] == 'heartbeat':
        send(dict(type='heartbeat_result', connection_epoch=epoch))
    elif frame['type'] == 'actor_announce':
        actor = dict(id=str(uuid.uuid4()), terminal='remote:'+device+':'+frame['native_session'],
                     project='space:'+space, program=frame['program'], name=frame['name'])
        send(dict(type='actor_announced', connection_epoch=epoch, actor=actor,
                  mutation_epoch=str(uuid.uuid4()), expires_at=2000000000000))
    elif frame['type'] == 'operation':
        operation = frame['operation']
        if operation.get('task_id') == 'stale-version':
            send(dict(type='error', error=dict(code='version_conflict', message='fixture-reflection',
                                              retryable=False, version=7)))
        else:
            correlation = str(uuid.uuid4()) if operation.get('task_id') == 'wrong-correlation' else frame['frame_id']
            send(dict(type='operation_result', connection_epoch=epoch, frame_id=correlation,
                      mutation_epoch=frame['mutation_epoch'], result=dict(operation=operation)))
    elif frame['type'] == 'events':
        send(dict(type='events_result', connection_epoch=epoch, frame_id=frame['frame_id'],
                  result=dict(events=[], cursor=frame.get('after'))))
"#).unwrap();
    // Close the script handle before Python opens it on Windows.
    let script = script.into_temp_path();
    let coordinator = uuid::Uuid::new_v4();
    let space = uuid::Uuid::new_v4();
    let command = || {
        #[cfg(target_os = "macos")]
        let mut command = Command::new("python3");
        #[cfg(windows)]
        let mut command = Command::new("python");
        command
            .arg(&script)
            .arg(coordinator.to_string())
            .arg(space.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    };
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let rejected = Connection::from_command(command(), uuid::Uuid::new_v4()).await;
        assert!(rejected.is_err());
        let mut connection = Connection::from_command(command(), coordinator)
            .await
            .unwrap();
        assert!(connection.heartbeat(space).await.is_err());
        let AuthenticationFrame::EnrollResult {
            credential,
            device_id,
            ..
        } = connection
            .enroll("fixture-invitation".into(), "Fixture".into())
            .await
            .unwrap()
        else {
            panic!("Expected enrollment delivery")
        };
        assert_eq!(
            connection.authenticate(credential).await.unwrap(),
            device_id
        );
        assert!(connection.heartbeat(uuid::Uuid::new_v4()).await.is_err());
        connection.heartbeat(space).await.unwrap();
        let actor = connection
            .announce(
                uuid::Uuid::new_v4(),
                space,
                uuid::Uuid::new_v4(),
                uuid::Uuid::new_v4(),
                uuid::Uuid::new_v4(),
                "codex",
                "Fixture",
            )
            .await
            .unwrap();
        let operation = crate::Operation::AgentSend {
            to: "receiver".into(),
            body: "Original intent".into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        let first = connection.execute(&actor, &operation).await.unwrap();
        let replay = connection.execute(&actor, &operation).await.unwrap();
        assert_eq!(first, replay);
        assert_eq!(
            first["operation"],
            serde_json::to_value(&operation).unwrap()
        );
        assert_eq!(
            connection.events(space, Some(42), Some(2)).await.unwrap()["cursor"],
            42
        );
        assert!(connection
            .events(uuid::Uuid::new_v4(), None, None)
            .await
            .is_err());
        let mut foreign = actor.clone();
        foreign.actor.terminal =
            format!("remote:{}:{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        assert!(connection.execute(&foreign, &operation).await.is_err());
        let stale = connection
            .execute(
                &actor,
                &crate::Operation::TaskGet {
                    task_id: "stale-version".into(),
                },
            )
            .await
            .unwrap_err();
        let stale = stale.downcast_ref::<crate::DomainError>().unwrap();
        assert_eq!(stale.code, "version_conflict");
        assert_eq!(stale.version, Some(7));
        assert!(!stale.message.contains("fixture-reflection"));
        assert!(connection
            .execute(
                &actor,
                &crate::Operation::TaskGet {
                    task_id: "wrong-correlation".into(),
                }
            )
            .await
            .is_err());
    });
}

#[test]
fn peer_errors_never_preserve_reflected_credentials_or_unknown_codes() {
    for code in ["device_revoked", "untrusted_peer_value"] {
        let error = remote_error(crate::DomainError {
            code: code.into(),
            message: "sensitive-fixture-reflection".into(),
            retryable: true,
            version: None,
        });
        assert!(!error.to_string().contains("sensitive-fixture-reflection"));
        let safe = error.downcast_ref::<crate::DomainError>().unwrap();
        assert_eq!(
            safe.code,
            if code == "device_revoked" {
                code
            } else {
                "invalid_input"
            }
        );
        assert_eq!(safe.retryable, code == "device_revoked");
    }
}
