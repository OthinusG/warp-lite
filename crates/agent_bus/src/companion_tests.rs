use super::*;

fn file_attachment(root: &Path, state: &Path) -> (Companion, ManagedFence) {
    let mut companion = Companion::new(state).unwrap();
    let managed_response::Result::Initialized(init) = result(companion.handle(request(
        managed_request::Operation::Initialize(ManagedInitialize { protocol_major: PROTOCOL_MAJOR }),
    ))) else { panic!("Initialize") };
    let managed_response::Result::ProjectOpened(opened) = result(companion.handle(request(
        managed_request::Operation::ProjectOpen(ProjectOpen { fence: init.fence, root: root.to_str().unwrap().into() }),
    ))) else { panic!("Project open") };
    (companion, opened.fence.unwrap())
}

#[test]
fn project_files_stage_content_and_refuse_changed_original_or_wrong_upload() {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("notes.md"), b"original").unwrap();
    let (mut companion, fence) = file_attachment(root.path(), &state.path().join("private"));
    let query = |action, path: &str| ProjectFilesRequest {
        fence: Some(fence.clone()), query_generation: 4, action: action as i32, path: path.into(), ..Default::default()
    };
    let managed_response::Result::ProjectFiles(read) = result(companion.handle(request(
        managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectFilePrepareRead, "notes.md")),
    ))) else { panic!("Read snapshot") };
    assert_eq!(std::fs::read(&read.transfer_path).unwrap(), b"original");
    assert_eq!(read.sha256, format!("{:x}", Sha256::digest(b"original")));
    let mut prepare = query(ProjectFileAction::ProjectFilePrepareWrite, "notes.md");
    prepare.expected_hash = read.sha256;
    let managed_response::Result::ProjectFiles(write) = result(companion.handle(request(
        managed_request::Operation::ProjectFiles(prepare),
    ))) else { panic!("Write staging") };
    std::fs::write(&write.transfer_path, b"edited").unwrap();
    let mut commit = query(ProjectFileAction::ProjectFileCommitWrite, "notes.md");
    commit.transfer_id = write.transfer_id;
    commit.expected_hash = format!("{:x}", Sha256::digest(b"wrong-upload"));
    assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(commit.clone())))), managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedConflict as i32));
    assert_eq!(std::fs::read(root.path().join("notes.md")).unwrap(), b"original");
    std::fs::write(root.path().join("notes.md"), b"external edit").unwrap();
    commit.expected_hash = format!("{:x}", Sha256::digest(b"edited"));
    assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(commit.clone())))), managed_response::Result::Error(ManagedError { code }) if code == ManagedErrorCode::ManagedConflict as i32));
    std::fs::write(root.path().join("notes.md"), b"original").unwrap();
    assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(commit)))), managed_response::Result::ProjectFiles(_)));
    assert_eq!(std::fs::read(root.path().join("notes.md")).unwrap(), b"edited");
    assert!(!Path::new(&write.transfer_path).exists());
    drop(companion);
    assert!(!Path::new(&read.transfer_path).exists());
}

#[test]
fn project_file_tree_and_mutations_are_root_scoped() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let (mut companion, fence) = file_attachment(root.path(), &state.path().join("private"));
    let query = |action, path: &str| ProjectFilesRequest { fence: Some(fence.clone()), action: action as i32, path: path.into(), ..Default::default() };
    for path in ["../outside", "/outside", "a\ncommand"] {
        assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectFileCreate, path))))), managed_response::Result::Error(_)));
    }
    assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectDirectoryCreate, "data 多语言"))))), managed_response::Result::ProjectFiles(_)));
    assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectFileCreate, "data 多语言/a.txt"))))), managed_response::Result::ProjectFiles(_)));
    let managed_response::Result::ProjectFiles(list) = result(companion.handle(request(managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectFileList, ""))))) else { panic!("Tree") };
    assert_eq!(list.snapshot.unwrap().entries[0].subtree_metadata.len(), 1);
    #[cfg(unix)] {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
        assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectFileCreate, "escape/a.txt"))))), managed_response::Result::Error(_)));
        assert!(!outside.path().join("a.txt").exists());
    }
    assert!(matches!(result(companion.handle(request(managed_request::Operation::ProjectFiles(query(ProjectFileAction::ProjectFileDelete, "data 多语言"))))), managed_response::Result::ProjectFiles(_)));
    assert!(!root.path().join("data 多语言").exists());
}

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
            "project_mcp",
            "project_files"
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
            matches!(result(companion.handle(request(managed_request::Operation::ProjectTasks(ProjectTasksRequest { fence: Some(fence), query_generation: 1, command_json: serde_json::to_vec(&TaskCommand::Panel(crate::transport::PanelQuery::default())).unwrap() })))), managed_response::Result::Error(ManagedError { code })
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
