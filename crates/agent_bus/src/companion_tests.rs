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
fn companion_opens_native_project_and_fences_every_attachment() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let mut companion = Companion::new(&state.path().join("companion-state")).unwrap();
    let init = || {
        request(managed_request::Operation::Initialize(ManagedInitialize {
            protocol_major: PROTOCOL_MAJOR,
        }))
    };
    let managed_response::Result::Initialized(initialized) = result(companion.handle(init()))
    else {
        panic!("Expected handshake")
    };
    assert_eq!(
        initialized.capabilities,
        [
            "project_open",
            "managed_agent",
            "project_tasks",
            "project_mcp"
        ]
    );
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

#[test]
fn project_tasks_require_current_native_project_before_any_store_access() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let directory = state.path().join("remote-data");
    let mut companion = Companion::new(&directory).unwrap();
    let managed_response::Result::Initialized(initialized) = result(companion.handle(request(
        managed_request::Operation::Initialize(ManagedInitialize {
            protocol_major: PROTOCOL_MAJOR,
        }),
    ))) else {
        panic!("Handshake required")
    };
    let managed_response::Result::ProjectOpened(opened) = result(companion.handle(request(
        managed_request::Operation::ProjectOpen(ProjectOpen {
            fence: initialized.fence,
            root: root.path().to_str().unwrap().into(),
        }),
    ))) else {
        panic!("Project required")
    };
    let fence = opened.fence.unwrap();
    let mut stale = fence.clone();
    stale.connection_id = Uuid::new_v4().to_string();
    let command_json =
        serde_json::to_vec(&TaskCommand::Panel(crate::transport::PanelQuery::default())).unwrap();
    let mut query = ProjectTasksRequest {
        fence: Some(stale),
        query_generation: 9,
        command_json,
    };
    assert!(
        matches!(result(companion.handle(request(managed_request::Operation::ProjectTasks(query.clone())))), managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedStaleAttachment as i32)
    );
    assert!(!directory.join("projects").exists());
    query.fence = Some(fence);
    assert!(matches!(
        result(
            companion.handle(request(managed_request::Operation::ProjectTasks(
                query.clone()
            )))
        ),
        managed_response::Result::ProjectTasks(_)
    ));
    let selected = root.path().join("selected");
    std::fs::create_dir(&selected).unwrap();
    let managed_response::Result::ProjectOpened(next) = result(companion.handle(request(
        managed_request::Operation::ProjectOpen(ProjectOpen {
            fence: Some(ManagedFence {
                project_id: String::new(),
                ..companion.fence.clone()
            }),
            root: selected.to_str().unwrap().into(),
        }),
    ))) else {
        panic!("New project required")
    };
    std::fs::rename(&selected, root.path().join("moved")).unwrap();
    std::fs::create_dir(&selected).unwrap();
    query.fence = next.fence;
    assert!(
        matches!(result(companion.handle(request(managed_request::Operation::ProjectTasks(query)))), managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedStaleAttachment as i32)
    );
}
