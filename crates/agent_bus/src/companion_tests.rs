use super::*;

fn request(operation: managed_request::Operation) -> ClientMessage {
    ClientMessage {
        request_id: Uuid::new_v4().to_string(),
        message: Some(client_message::Message::Managed(ManagedRequest {
            operation: Some(operation),
        })),
    }
}

fn result(reply: ServerMessage) -> managed_response::Result {
    let Some(server_message::Message::Managed(reply)) = reply.message else {
        panic!("Expected managed reply")
    };
    reply.result.unwrap()
}

#[test]
fn companion_reads_native_project_metrics_and_fences_every_attachment() {
    let root = tempfile::tempdir().unwrap();
    let mut companion = Companion::default();
    let init = || {
        request(managed_request::Operation::Initialize(ManagedInitialize {
            protocol_major: PROTOCOL_MAJOR,
        }))
    };
    let managed_response::Result::Initialized(initialized) = result(companion.handle(init()))
    else {
        panic!("Expected handshake")
    };
    assert_eq!(initialized.capabilities, ["project_open", "host_status"]);
    let mut fence = initialized.fence.unwrap();
    assert!(matches!(
        result(companion.handle(init())),
        managed_response::Result::Error(_)
    ));
    let managed_response::Result::ProjectOpened(opened) = result(companion.handle(request(
        managed_request::Operation::ProjectOpen(ProjectOpen {
            fence: Some(fence.clone()),
            root: root.path().to_str().unwrap().into(),
        }),
    ))) else {
        panic!("Expected project")
    };
    assert_eq!(
        Path::new(&opened.canonical_root),
        root.path().canonicalize().unwrap()
    );
    fence = opened.fence.unwrap();
    let status_request = |fence: ManagedFence| {
        request(managed_request::Operation::HostStatus(HostStatusRequest {
            fence: Some(fence),
            query_generation: 19,
        }))
    };
    let managed_response::Result::HostStatus(status) =
        result(companion.handle(status_request(fence.clone())))
    else {
        panic!("Expected native metrics")
    };
    assert!(valid_status(&status));
    assert_eq!(status.query_generation, 19);
    assert_eq!(status.source, "companion_native");
    assert!(matches!(
        status.cpu.unwrap().value,
        Some(cpu_metric::Value::Unavailable(_))
    ));
    let Some(disk_metric::Value::Bytes(disk)) = status.project_disk.unwrap().value else {
        panic!("Expected volume counters")
    };
    assert_eq!(disk.total, disk_bytes(root.path()).unwrap().total);
    assert!(disk.free <= disk.total);
    let managed_response::Result::HostStatus(cached) =
        result(companion.handle(status_request(fence.clone())))
    else {
        panic!("Expected bounded sample")
    };
    assert_eq!(cached.observation_sequence, 1);
    for field in 0..4 {
        let mut stale = fence.clone();
        match field {
            0 => stale.service_id = Uuid::new_v4().to_string(),
            1 => stale.service_boot_id = Uuid::new_v4().to_string(),
            2 => stale.connection_id = Uuid::new_v4().to_string(),
            _ => stale.project_id = Uuid::new_v4().to_string(),
        }
        assert!(
            matches!(result(companion.handle(status_request(stale))), managed_response::Result::Error(ManagedError { code })
            if code == i32::from(ManagedErrorCode::ManagedStaleAttachment))
        );
    }
    let legacy = ClientMessage {
        request_id: Uuid::new_v4().to_string(),
        message: Some(client_message::Message::RunCommand(RunCommandRequest {
            command: "must-never-execute".into(),
            ..Default::default()
        })),
    };
    assert!(
        matches!(result(companion.handle(legacy)), managed_response::Result::Error(ManagedError { code })
        if code == i32::from(ManagedErrorCode::ManagedFeatureUnavailable))
    );
    let malformed = ClientMessage {
        request_id: "\u{1b}[untrusted".into(),
        message: None,
    };
    assert!(companion.handle(malformed).request_id.is_empty());
    #[cfg(unix)]
    {
        let renamed = root.path().with_extension("replaced");
        std::fs::rename(root.path(), &renamed).unwrap();
        std::fs::create_dir(root.path()).unwrap();
        assert!(
            matches!(result(companion.handle(status_request(fence))), managed_response::Result::Error(ManagedError { code })
            if code == i32::from(ManagedErrorCode::ManagedStaleAttachment))
        );
        std::fs::remove_dir(renamed).unwrap();
    }
}
