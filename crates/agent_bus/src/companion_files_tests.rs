//! Exercises file control through the public protocol, including failure effects.
use super::*;
use sha2::{Digest, Sha256};

#[test]
fn service_startup_reaps_only_owned_transfer_leftovers() {
    let state = tempfile::tempdir().unwrap();
    let directory = state.path().join("file-transfers");
    identity::private_directory(&directory).unwrap();
    let abandoned = directory.join(Uuid::new_v4().to_string());
    identity::private_directory(&abandoned).unwrap();
    identity::private_file(&abandoned.join("content")).unwrap();
    std::fs::write(directory.join("unrelated-note"), b"keep").unwrap();
    files::clean_abandoned(state.path()).unwrap();
    assert!(!abandoned.exists());
    assert_eq!(
        std::fs::read(directory.join("unrelated-note")).unwrap(),
        b"keep"
    );
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("content"), b"outside").unwrap();
        let link = directory.join(Uuid::new_v4().to_string());
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        assert!(files::clean_abandoned(state.path()).is_err());
        assert_eq!(
            std::fs::read(outside.path().join("content")).unwrap(),
            b"outside"
        );
        std::fs::remove_file(link).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn windows_junctions_and_native_name_aliases_cannot_escape_the_project() {
    let mut fixture = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("keep.txt"), b"outside").unwrap();
    let junction = fixture.root.path().join("junction");
    // Junctions exercise reparse traversal without requiring symlink privileges.
    let status = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&junction)
        .arg(outside.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "Controlled Windows junction fixture");
    for (action, path) in [
        (ProjectFileAction::ProjectFileList, "junction"),
        (
            ProjectFileAction::ProjectFilePrepareRead,
            "junction/keep.txt",
        ),
        (ProjectFileAction::ProjectFileCreate, "junction/new"),
        (ProjectFileAction::ProjectFileDelete, "junction/keep.txt"),
    ] {
        assert!(fixture.run(action, path).is_err());
    }
    fixture
        .run(ProjectFileAction::ProjectFileDelete, "junction")
        .unwrap();
    assert_eq!(
        std::fs::read(outside.path().join("keep.txt")).unwrap(),
        b"outside"
    );
    for path in [
        "CON",
        "NUL.txt",
        "one:stream",
        "trailing.",
        "trailing ",
        "C:/escape",
        "\\\\server\\share",
        "one\\..\\escape",
    ] {
        assert_eq!(
            fixture
                .run(ProjectFileAction::ProjectFileCreate, path)
                .unwrap_err(),
            ManagedErrorCode::ManagedInvalidInput
        );
    }
}

#[tokio::test]
async fn disconnect_cleans_staging_through_the_framed_service() {
    use remote_protocol::protocol::*;
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    async fn call(
        client: &mut tokio::io::DuplexStream,
        operation: managed_request::Operation,
    ) -> managed_response::Result {
        let request = ClientMessage {
            request_id: Uuid::new_v4().to_string(),
            message: Some(client_message::Message::Managed(ManagedRequest {
                operation: Some(operation),
            })),
        };
        write_message_with_limit(
            &mut (&mut *client).compat_write(),
            &request,
            MAX_MANAGED_MESSAGE_SIZE,
        )
        .await
        .unwrap();
        let reply: ServerMessage = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read_message_with_limit(&mut client.compat(), MAX_MANAGED_MESSAGE_SIZE),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(reply.request_id, request.request_id);
        let Some(server_message::Message::Managed(reply)) = reply.message else {
            panic!("Managed")
        };
        reply.result.unwrap()
    }
    let state = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("file"), b"original").unwrap();
    let directory = state.path().join("private");
    let tasks = std::sync::Arc::new(tasks::Projects::new(&directory));
    let (mut client, server) = tokio::io::duplex(4096);
    let service = tokio::spawn(async move {
        serve_channel(
            server,
            &directory,
            &Uuid::new_v4().to_string(),
            std::sync::Arc::new(terminals::Terminals::default()),
            tasks,
        )
        .await
    });
    let managed_response::Result::Initialized(init) = call(
        &mut client,
        managed_request::Operation::Initialize(ManagedInitialize {
            protocol_major: PROTOCOL_MAJOR,
        }),
    )
    .await
    else {
        panic!("Initialize")
    };
    let managed_response::Result::ProjectOpened(opened) = call(
        &mut client,
        managed_request::Operation::ProjectOpen(ProjectOpen {
            fence: init.fence,
            root: root.path().to_str().unwrap().into(),
        }),
    )
    .await
    else {
        panic!("Project")
    };
    let managed_response::Result::ProjectFiles(staged) = call(
        &mut client,
        managed_request::Operation::ProjectFiles(ProjectFilesRequest {
            fence: opened.fence,
            action: ProjectFileAction::ProjectFilePrepareRead as i32,
            path: "file".into(),
            ..Default::default()
        }),
    )
    .await
    else {
        panic!("Stage")
    };
    assert!(Path::new(&staged.transfer_path).exists());
    drop(client);
    tokio::time::timeout(std::time::Duration::from_secs(5), service)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!Path::new(&staged.transfer_path).exists());
    assert_eq!(
        std::fs::read(root.path().join("file")).unwrap(),
        b"original"
    );
}

#[tokio::test]
async fn fragmented_invalid_and_oversized_control_frames_have_no_partial_effects() {
    use remote_protocol::protocol::*;
    use tokio::{
        io::AsyncWriteExt,
        time::{timeout, Duration},
    };
    use tokio_util::compat::TokioAsyncReadCompatExt;
    for mode in ["fragmented", "truncated", "oversized", "invalid-protobuf"] {
        let state = tempfile::tempdir().unwrap();
        let directory = state.path().join("private");
        let tasks = std::sync::Arc::new(tasks::Projects::new(&directory));
        let (mut client, server) = tokio::io::duplex(4096);
        let service = tokio::spawn(async move {
            serve_channel(
                server,
                &directory,
                &Uuid::new_v4().to_string(),
                std::sync::Arc::new(terminals::Terminals::default()),
                tasks,
            )
            .await
        });
        let mut frame = Vec::new();
        if mode == "fragmented" {
            let request = ClientMessage {
                request_id: Uuid::new_v4().to_string(),
                message: Some(client_message::Message::Managed(ManagedRequest {
                    operation: Some(managed_request::Operation::Initialize(ManagedInitialize {
                        protocol_major: PROTOCOL_MAJOR,
                    })),
                })),
            };
            write_message_with_limit(&mut frame, &request, MAX_MANAGED_MESSAGE_SIZE)
                .await
                .unwrap();
            for byte in frame {
                client.write_all(&[byte]).await.unwrap();
            }
            let reply: ServerMessage = timeout(
                Duration::from_secs(5),
                read_message_with_limit(&mut (&mut client).compat(), MAX_MANAGED_MESSAGE_SIZE),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(reply.request_id, request.request_id);
        } else {
            frame.extend_from_slice(
                &(if mode == "oversized" {
                    MAX_MANAGED_MESSAGE_SIZE as u32 + 1
                } else {
                    2u32
                })
                .to_le_bytes(),
            );
            if mode == "invalid-protobuf" {
                frame.extend_from_slice(&[0xff, 0xff]);
            } else if mode == "truncated" {
                frame.push(0xff);
            }
            client.write_all(&frame).await.unwrap();
        }
        client.shutdown().await.unwrap();
        drop(client);
        let result = timeout(Duration::from_secs(5), service)
            .await
            .unwrap()
            .unwrap();
        match mode {
            "oversized" => assert!(matches!(result, Err(ProtocolError::MessageTooLarge { .. }))),
            "invalid-protobuf" => assert!(matches!(result, Err(ProtocolError::Decode(..)))),
            _ => assert!(result.is_ok()),
        }
    }
}

struct Fixture {
    root: tempfile::TempDir,
    _state: tempfile::TempDir,
    companion: Companion,
    fence: ManagedFence,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let mut companion = Companion::new(&state.path().join("private")).unwrap();
        let managed_response::Result::Initialized(init) = Self::call(
            &mut companion,
            managed_request::Operation::Initialize(ManagedInitialize {
                protocol_major: PROTOCOL_MAJOR,
            }),
        ) else {
            panic!("Initialize")
        };
        let managed_response::Result::ProjectOpened(opened) = Self::call(
            &mut companion,
            managed_request::Operation::ProjectOpen(ProjectOpen {
                fence: init.fence,
                root: root.path().to_str().unwrap().into(),
            }),
        ) else {
            panic!("Project open")
        };
        Self {
            root,
            _state: state,
            companion,
            fence: opened.fence.unwrap(),
        }
    }
    fn git(&self, arguments: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args([
                "-c",
                "core.hooksPath=",
                "-c",
                "user.name=Warpai Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(arguments)
            .current_dir(self.root.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "Controlled Git fixture failed");
        String::from_utf8(output.stdout).unwrap()
    }
    fn call(
        companion: &mut Companion,
        operation: managed_request::Operation,
    ) -> managed_response::Result {
        let reply = companion.handle(ClientMessage {
            request_id: Uuid::new_v4().to_string(),
            message: Some(client_message::Message::Managed(ManagedRequest {
                operation: Some(operation),
            })),
        });
        let Some(server_message::Message::Managed(reply)) = reply.message else {
            panic!("Managed reply")
        };
        reply.result.unwrap()
    }
    fn query(&self, action: ProjectFileAction, path: &str) -> ProjectFilesRequest {
        ProjectFilesRequest {
            fence: Some(self.fence.clone()),
            query_generation: 17,
            action: action as i32,
            path: path.into(),
            ..Default::default()
        }
    }
    fn execute(
        &mut self,
        query: ProjectFilesRequest,
    ) -> std::result::Result<ProjectFilesResult, ManagedErrorCode> {
        match Self::call(
            &mut self.companion,
            managed_request::Operation::ProjectFiles(query),
        ) {
            managed_response::Result::ProjectFiles(value) => {
                assert_eq!(value.fence.as_ref(), Some(&self.fence));
                Ok(*value)
            }
            managed_response::Result::Error(error) => {
                Err(ManagedErrorCode::try_from(error.code).unwrap())
            }
            _ => panic!("File reply"),
        }
    }
    fn run(
        &mut self,
        action: ProjectFileAction,
        path: &str,
    ) -> std::result::Result<ProjectFilesResult, ManagedErrorCode> {
        self.execute(self.query(action, path))
    }
    fn write(&self, path: &str, content: &[u8]) {
        std::fs::write(self.root.path().join(path), content).unwrap();
    }
}

#[test]
fn file_requests_reject_uninitialized_stale_and_foreign_attachments() {
    let mut one = Fixture::new();
    let two = Fixture::new();
    for fence in [
        None,
        Some(two.fence),
        Some(ManagedFence {
            service_boot_id: Uuid::new_v4().to_string(),
            ..one.fence.clone()
        }),
    ] {
        let mut query = one.query(ProjectFileAction::ProjectFileCreate, "not-created");
        query.fence = fence;
        assert_eq!(
            one.execute(query).unwrap_err(),
            ManagedErrorCode::ManagedStaleAttachment
        );
        assert!(!one.root.path().join("not-created").exists());
    }
    let mut query = one.query(ProjectFileAction::ProjectFileCreate, "not-created");
    query.action = i32::MAX;
    assert_eq!(
        one.execute(query).unwrap_err(),
        ManagedErrorCode::ManagedInvalidInput
    );
    assert!(!one.root.path().join("not-created").exists());
}

#[test]
fn files_enforce_path_and_resource_limits_without_partial_creation() {
    let mut fixture = Fixture::new();
    for path in [
        "",
        "..",
        "../escape",
        "a/../../escape",
        "/absolute",
        "nul\0name",
        "new\ncommand",
        "tab\tname",
    ] {
        assert!(
            fixture
                .run(ProjectFileAction::ProjectFileCreate, path)
                .is_err(),
            "{path:?}"
        );
    }
    assert!(fixture
        .run(ProjectFileAction::ProjectFileCreate, &"x".repeat(4097))
        .is_err());
    let large = File::create(fixture.root.path().join("large.bin")).unwrap();
    large.set_len(files::MAX_FILE_BYTES + 1).unwrap();
    assert_eq!(
        fixture
            .run(ProjectFileAction::ProjectFilePrepareRead, "large.bin")
            .unwrap_err(),
        ManagedErrorCode::ManagedCapacityExceeded
    );
    assert!(fixture.companion.files.transfers_for_test().is_empty());
    let first = fixture.run(ProjectFileAction::ProjectFileList, "").unwrap();
    assert_eq!(first.query_generation, 17);
    assert_eq!(
        first.snapshot.unwrap().repo_path,
        fixture
            .root
            .path()
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
    );
    assert!(fixture
        .run(ProjectFileAction::ProjectFilePrepareRead, "missing")
        .is_err());
}

#[test]
fn transfers_are_bounded_private_nonreplayable_and_connection_owned() {
    let mut fixture = Fixture::new();
    fixture.write("zero.txt", b"");
    let mut transfers = Vec::new();
    for _ in 0..8 {
        let transfer = fixture
            .run(ProjectFileAction::ProjectFilePrepareRead, "zero.txt")
            .unwrap();
        assert_eq!(transfer.sha256, format!("{:x}", Sha256::digest(b"")));
        assert_eq!(transfer.size, 0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&transfer.transfer_path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
            assert_eq!(
                std::fs::metadata(Path::new(&transfer.transfer_path).parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
        }
        transfers.push(transfer);
    }
    assert_eq!(
        fixture
            .run(ProjectFileAction::ProjectFilePrepareRead, "zero.txt")
            .unwrap_err(),
        ManagedErrorCode::ManagedCapacityExceeded
    );
    let mut second = Fixture::new();
    let mut foreign = second.query(ProjectFileAction::ProjectFileRelease, "");
    foreign.transfer_id = transfers[0].transfer_id.clone();
    assert_eq!(
        second.execute(foreign).unwrap_err(),
        ManagedErrorCode::ManagedInvalidInput
    );
    let mut release = fixture.query(ProjectFileAction::ProjectFileRelease, "");
    release.transfer_id = transfers[0].transfer_id.clone();
    fixture.execute(release.clone()).unwrap();
    assert_eq!(
        fixture.execute(release).unwrap_err(),
        ManagedErrorCode::ManagedInvalidInput
    );
    assert!(!Path::new(&transfers[0].transfer_path).exists());
    let transfer = fixture
        .run(ProjectFileAction::ProjectFilePrepareRead, "zero.txt")
        .unwrap();
    drop(fixture);
    for transfer in transfers.into_iter().chain(Some(transfer)) {
        assert!(!Path::new(&transfer.transfer_path).exists());
    }
}

#[test]
fn explicit_mutations_preserve_destinations_and_reject_root_deletion() {
    let mut fixture = Fixture::new();
    fixture.write("source.txt", b"source");
    fixture.write("destination.txt", b"destination");
    let mut rename = fixture.query(ProjectFileAction::ProjectFileRename, "source.txt");
    rename.destination = "destination.txt".into();
    assert!(fixture.execute(rename.clone()).is_err());
    assert_eq!(
        std::fs::read(fixture.root.path().join("source.txt")).unwrap(),
        b"source"
    );
    assert_eq!(
        std::fs::read(fixture.root.path().join("destination.txt")).unwrap(),
        b"destination"
    );
    rename.destination = "renamed '$(literal)' 多语言.txt".into();
    fixture.execute(rename).unwrap();
    assert_eq!(
        std::fs::read(fixture.root.path().join("renamed '$(literal)' 多语言.txt")).unwrap(),
        b"source"
    );
    assert!(!fixture.root.path().join("source.txt").exists());
    assert!(fixture
        .run(ProjectFileAction::ProjectFileDelete, "")
        .is_err());
    assert!(fixture.root.path().is_dir());
    assert!(fixture
        .run(ProjectFileAction::ProjectFileCreate, "destination.txt")
        .is_err());
    assert_eq!(
        std::fs::read(fixture.root.path().join("destination.txt")).unwrap(),
        b"destination"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_special_files_and_denied_permissions_never_escape_or_block() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let mut fixture = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("keep.txt"), b"outside").unwrap();
    symlink(outside.path(), fixture.root.path().join("directory-link")).unwrap();
    symlink(
        outside.path().join("keep.txt"),
        fixture.root.path().join("file-link"),
    )
    .unwrap();
    for (action, path) in [
        (ProjectFileAction::ProjectFileList, "directory-link"),
        (ProjectFileAction::ProjectFilePrepareRead, "file-link"),
        (
            ProjectFileAction::ProjectFileCreate,
            "directory-link/new.txt",
        ),
        (
            ProjectFileAction::ProjectFileDelete,
            "directory-link/keep.txt",
        ),
    ] {
        assert!(fixture.run(action, path).is_err());
    }
    fixture
        .run(ProjectFileAction::ProjectFileDelete, "directory-link")
        .unwrap();
    assert_eq!(
        std::fs::read(outside.path().join("keep.txt")).unwrap(),
        b"outside"
    );
    let fifo = std::ffi::CString::new(fixture.root.path().join("fifo").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let started = std::time::Instant::now();
    assert!(fixture
        .run(ProjectFileAction::ProjectFilePrepareRead, "fifo")
        .is_err());
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    fixture.write("denied", b"data");
    std::fs::set_permissions(
        fixture.root.path().join("denied"),
        std::fs::Permissions::from_mode(0),
    )
    .unwrap();
    if unsafe { libc::geteuid() } != 0 {
        assert_eq!(
            fixture
                .run(ProjectFileAction::ProjectFilePrepareRead, "denied")
                .unwrap_err(),
            ManagedErrorCode::ManagedPermissionDenied
        );
    }
}

#[cfg(unix)]
#[test]
fn root_replacement_rejects_staged_commit_and_preserves_both_roots() {
    let mut fixture = Fixture::new();
    fixture.write("notes.txt", b"original");
    let mut query = fixture.query(ProjectFileAction::ProjectFilePrepareWrite, "notes.txt");
    query.expected_hash = format!("{:x}", Sha256::digest(b"original"));
    let transfer = fixture.execute(query).unwrap();
    std::fs::write(&transfer.transfer_path, b"edited").unwrap();
    let moved = fixture
        .root
        .path()
        .with_extension(Uuid::new_v4().to_string());
    std::fs::rename(fixture.root.path(), &moved).unwrap();
    std::fs::create_dir(fixture.root.path()).unwrap();
    fixture.write("notes.txt", b"replacement");
    let mut commit = fixture.query(ProjectFileAction::ProjectFileCommitWrite, "notes.txt");
    commit.expected_hash = format!("{:x}", Sha256::digest(b"edited"));
    commit.transfer_id = transfer.transfer_id;
    assert_eq!(
        fixture.execute(commit).unwrap_err(),
        ManagedErrorCode::ManagedStaleAttachment
    );
    assert_eq!(std::fs::read(moved.join("notes.txt")).unwrap(), b"original");
    assert_eq!(
        std::fs::read(fixture.root.path().join("notes.txt")).unwrap(),
        b"replacement"
    );
    std::fs::remove_dir_all(moved).unwrap();
}

#[test]
fn staging_snapshots_binary_content_and_save_preserves_permissions() {
    let mut fixture = Fixture::new();
    let bytes = b"\0binary\xff\r\n";
    fixture.write("binary.dat", bytes);
    let read = fixture
        .run(ProjectFileAction::ProjectFilePrepareRead, "binary.dat")
        .unwrap();
    fixture.write("binary.dat", b"later");
    assert_eq!(std::fs::read(&read.transfer_path).unwrap(), bytes);
    assert_eq!(read.size, bytes.len() as u64);
    let mut release = fixture.query(ProjectFileAction::ProjectFileRelease, "");
    release.transfer_id = read.transfer_id;
    fixture.execute(release).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            fixture.root.path().join("binary.dat"),
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
    }
    let mut prepare = fixture.query(ProjectFileAction::ProjectFilePrepareWrite, "binary.dat");
    prepare.expected_hash = format!("{:x}", Sha256::digest(b"later"));
    let transfer = fixture.execute(prepare).unwrap();
    std::fs::write(&transfer.transfer_path, bytes).unwrap();
    let mut commit = fixture.query(ProjectFileAction::ProjectFileCommitWrite, "binary.dat");
    commit.transfer_id = transfer.transfer_id;
    commit.expected_hash = format!("{:x}", Sha256::digest(bytes));
    assert_eq!(
        fixture.execute(commit.clone()).unwrap().sha256,
        commit.expected_hash
    );
    assert_eq!(
        std::fs::read(fixture.root.path().join("binary.dat")).unwrap(),
        bytes
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(fixture.root.path().join("binary.dat"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }
    assert_eq!(
        fixture.execute(commit).unwrap_err(),
        ManagedErrorCode::ManagedInvalidInput
    );
    assert!(!Path::new(&transfer.transfer_path).exists());
    assert!(std::fs::read_dir(fixture.root.path())
        .unwrap()
        .all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".warpai-save-")));
}

#[test]
fn failed_and_successful_project_switches_have_distinct_transfer_lifetimes() {
    let mut fixture = Fixture::new();
    fixture.write("keep", b"original");
    let transfer = fixture
        .run(ProjectFileAction::ProjectFilePrepareRead, "keep")
        .unwrap();
    let missing = fixture.root.path().join("missing");
    let account_fence = ManagedFence {
        project_id: String::new(),
        ..fixture.fence.clone()
    };
    assert!(matches!(
        Fixture::call(
            &mut fixture.companion,
            managed_request::Operation::ProjectOpen(ProjectOpen {
                fence: Some(account_fence.clone()),
                root: missing.to_str().unwrap().into()
            })
        ),
        managed_response::Result::Error(_)
    ));
    assert!(Path::new(&transfer.transfer_path).exists());
    fixture.run(ProjectFileAction::ProjectFileList, "").unwrap();
    let other = tempfile::tempdir().unwrap();
    let managed_response::Result::ProjectOpened(opened) = Fixture::call(
        &mut fixture.companion,
        managed_request::Operation::ProjectOpen(ProjectOpen {
            fence: Some(account_fence),
            root: other.path().to_str().unwrap().into(),
        }),
    ) else {
        panic!("Project switch")
    };
    assert!(!Path::new(&transfer.transfer_path).exists());
    assert_eq!(
        fixture
            .run(ProjectFileAction::ProjectFileCreate, "wrong-root")
            .unwrap_err(),
        ManagedErrorCode::ManagedStaleAttachment
    );
    fixture.fence = opened.fence.unwrap();
    let mut release = fixture.query(ProjectFileAction::ProjectFileRelease, "");
    release.transfer_id = transfer.transfer_id;
    assert_eq!(
        fixture.execute(release).unwrap_err(),
        ManagedErrorCode::ManagedInvalidInput
    );
    assert!(!other.path().join("wrong-root").exists());
    assert_eq!(
        std::fs::read(fixture.root.path().join("keep")).unwrap(),
        b"original"
    );
}

#[test]
fn invalid_commits_preserve_original_and_keep_staging_explicitly_releasable() {
    let mut fixture = Fixture::new();
    fixture.write("one", b"one");
    fixture.write("two", b"two");
    let mut prepare = fixture.query(ProjectFileAction::ProjectFilePrepareWrite, "one");
    prepare.expected_hash = "not-a-hash".into();
    assert_eq!(
        fixture.execute(prepare.clone()).unwrap_err(),
        ManagedErrorCode::ManagedConflict
    );
    assert!(fixture.companion.files.transfers_for_test().is_empty());
    prepare.expected_hash = format!("{:x}", Sha256::digest(b"one"));
    let transfer = fixture.execute(prepare).unwrap();
    let mut commit = fixture.query(ProjectFileAction::ProjectFileCommitWrite, "two");
    commit.transfer_id = transfer.transfer_id.clone();
    commit.expected_hash = format!("{:x}", Sha256::digest(b""));
    assert_eq!(
        fixture.execute(commit.clone()).unwrap_err(),
        ManagedErrorCode::ManagedInvalidInput
    );
    commit.path = "one".into();
    File::create(&transfer.transfer_path)
        .unwrap()
        .set_len(files::MAX_FILE_BYTES + 1)
        .unwrap();
    assert_eq!(
        fixture.execute(commit.clone()).unwrap_err(),
        ManagedErrorCode::ManagedCapacityExceeded
    );
    std::fs::write(&transfer.transfer_path, b"incomplete upload").unwrap();
    assert_eq!(
        fixture.execute(commit).unwrap_err(),
        ManagedErrorCode::ManagedConflict
    );
    assert_eq!(
        std::fs::read(fixture.root.path().join("one")).unwrap(),
        b"one"
    );
    assert_eq!(
        std::fs::read(fixture.root.path().join("two")).unwrap(),
        b"two"
    );
    assert!(Path::new(&transfer.transfer_path).exists());
    let mut release = fixture.query(ProjectFileAction::ProjectFileRelease, "");
    release.transfer_id = transfer.transfer_id;
    fixture.execute(release).unwrap();
    assert!(!Path::new(&transfer.transfer_path).exists());
}

#[test]
fn directory_snapshots_are_complete_repeatable_and_bounded_on_the_wire() {
    let mut fixture = Fixture::new();
    assert!(fixture
        .run(ProjectFileAction::ProjectFileList, "")
        .unwrap()
        .snapshot
        .unwrap()
        .entries[0]
        .subtree_metadata
        .is_empty());
    fixture
        .run(ProjectFileAction::ProjectDirectoryCreate, "nested")
        .unwrap();
    fixture.write("nested/-literal.txt", b"x");
    for _ in 0..3 {
        let snapshot = fixture
            .run(ProjectFileAction::ProjectFileList, "nested")
            .unwrap()
            .snapshot
            .unwrap();
        assert_eq!(snapshot.entries[0].subtree_metadata.len(), 1);
        assert!(matches!(&snapshot.entries[0].subtree_metadata[0].node,
            Some(repo_node_metadata::Node::File(file)) if Path::new(&file.path).ends_with(Path::new("nested").join("-literal.txt"))));
    }
    for index in 0..4001 {
        fixture.write(&format!("item-{index}"), b"");
    }
    assert_eq!(
        fixture
            .run(ProjectFileAction::ProjectFileList, "")
            .unwrap_err(),
        ManagedErrorCode::ManagedCapacityExceeded
    );
    for index in 0..4001 {
        std::fs::remove_file(fixture.root.path().join(format!("item-{index}"))).unwrap();
    }
    // Below entry-count ceiling but above encoded-metadata budget.
    for index in 0..2500 {
        fixture.write(&format!("{index}-{}", "x".repeat(190)), b"");
    }
    assert_eq!(
        fixture
            .run(ProjectFileAction::ProjectFileList, "")
            .unwrap_err(),
        ManagedErrorCode::ManagedCapacityExceeded
    );
}

#[test]
fn oversized_nested_delete_fails_before_removing_known_children() {
    let mut fixture = Fixture::new();
    std::fs::create_dir(fixture.root.path().join("tree")).unwrap();
    for directory in ["first", "second"] {
        std::fs::create_dir(fixture.root.path().join("tree").join(directory)).unwrap();
        for index in 0..2000 {
            fixture.write(&format!("tree/{directory}/{index}"), b"keep");
        }
    }
    assert_eq!(
        fixture
            .run(ProjectFileAction::ProjectFileDelete, "tree")
            .unwrap_err(),
        ManagedErrorCode::ManagedCapacityExceeded
    );
    for directory in ["first", "second"] {
        assert_eq!(
            std::fs::read_dir(fixture.root.path().join("tree").join(directory))
                .unwrap()
                .count(),
            2000
        );
    }
}

#[cfg(unix)]
#[test]
fn altered_stage_links_fifos_and_modes_are_rejected_without_touching_original() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let mut fixture = Fixture::new();
    fixture.write("original", b"safe");
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("keep");
    std::fs::write(&outside_file, b"outside").unwrap();
    for kind in ["symlink", "fifo", "permissions"] {
        let mut prepare = fixture.query(ProjectFileAction::ProjectFilePrepareWrite, "original");
        prepare.expected_hash = format!("{:x}", Sha256::digest(b"safe"));
        let transfer = fixture.execute(prepare).unwrap();
        if kind == "permissions" {
            std::fs::set_permissions(
                &transfer.transfer_path,
                std::fs::Permissions::from_mode(0o644),
            )
            .unwrap();
        } else {
            std::fs::remove_file(&transfer.transfer_path).unwrap();
            if kind == "symlink" {
                symlink(&outside_file, &transfer.transfer_path).unwrap();
            } else {
                let name = std::ffi::CString::new(transfer.transfer_path.as_str()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
        }
        let mut commit = fixture.query(ProjectFileAction::ProjectFileCommitWrite, "original");
        commit.transfer_id = transfer.transfer_id.clone();
        commit.expected_hash = format!("{:x}", Sha256::digest(b"outside"));
        let started = std::time::Instant::now();
        assert!(fixture.execute(commit).is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(
            std::fs::read(fixture.root.path().join("original")).unwrap(),
            b"safe"
        );
        assert_eq!(std::fs::read(&outside_file).unwrap(), b"outside");
        let mut release = fixture.query(ProjectFileAction::ProjectFileRelease, "");
        release.transfer_id = transfer.transfer_id;
        fixture.execute(release).unwrap();
    }
}

#[test]
fn git_review_uses_native_status_patch_and_immutable_sftp_base() {
    let mut fixture = Fixture::new();
    let git_root = fixture.root.path().to_owned();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args([
                "-c",
                "core.hooksPath=",
                "-c",
                "user.name=Warpai Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(args)
            .current_dir(&git_root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Controlled Git fixture command failed"
        );
        String::from_utf8(output.stdout).unwrap()
    };
    git(&["init", "--initial-branch=main"]);
    fixture.write("document 多语言.md", b"original\n");
    git(&["add", "--", "."]);
    git(&["commit", "-m", "fixture base"]);
    let head = git(&["rev-parse", "HEAD"]).trim().to_owned();
    git(&["branch", "comparison"]);
    fixture.write("document 多语言.md", b"staged\n");
    git(&["add", "--", "."]);
    fixture.write("document 多语言.md", b"working\n");
    fixture.write("new.txt", b"new\n");
    let status = fixture
        .run(ProjectFileAction::ProjectGitStatus, "")
        .unwrap();
    assert_eq!(
        status.git_output,
        git(&[
            "--no-optional-locks",
            "status",
            "--porcelain=2",
            "-z",
            "--untracked-files=all",
            "--",
            "."
        ])
    );
    assert_eq!(status.git_base, head);
    let patch = fixture
        .run(ProjectFileAction::ProjectGitDiff, "document 多语言.md")
        .unwrap();
    assert_eq!(
        patch.git_output,
        git(&[
            "--no-optional-locks",
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            &head,
            "--",
            "document 多语言.md"
        ])
    );
    let base = fixture
        .run(
            ProjectFileAction::ProjectGitPrepareBase,
            "document 多语言.md",
        )
        .unwrap();
    assert_eq!(std::fs::read(&base.transfer_path).unwrap(), b"original\n");
    let mut comparison = fixture.query(ProjectFileAction::ProjectGitStatus, "");
    comparison.destination = "@main".into();
    let comparison = fixture.execute(comparison).unwrap();
    assert_eq!(comparison.git_base, head);
    assert!(comparison.git_name_status.contains("document 多语言.md"));
    let branches = fixture
        .run(ProjectFileAction::ProjectGitBranches, "")
        .unwrap();
    assert!(branches.git_output.lines().any(|line| line == "comparison"));
    let root = fixture.run(ProjectFileAction::ProjectGitRoot, "").unwrap();
    assert_eq!(
        Path::new(root.git_output.trim()).canonicalize().unwrap(),
        fixture.root.path().canonicalize().unwrap()
    );
    assert!(fixture
        .run(ProjectFileAction::ProjectGitDiff, "../outside")
        .is_err());
    let mut malicious = fixture.query(ProjectFileAction::ProjectGitDiff, "new.txt");
    malicious.destination = "--output=outside".into();
    assert!(fixture.execute(malicious).is_err());
    let mut release = fixture.query(ProjectFileAction::ProjectFileRelease, "");
    release.transfer_id = base.transfer_id;
    fixture.execute(release).unwrap();
    assert!(!Path::new(&base.transfer_path).exists());
}

#[test]
fn git_review_handles_unborn_rename_delete_binary_conflict_and_worktree() {
    let mut fixture = Fixture::new();
    assert!(fixture
        .run(ProjectFileAction::ProjectGitStatus, "")
        .is_err());
    fixture.git(&["init", "--initial-branch=main"]);
    fixture.write("unborn.txt", b"first\n");
    let unborn = fixture
        .run(ProjectFileAction::ProjectGitStatus, "")
        .unwrap();
    assert!(unborn.git_base.is_empty());
    assert!(unborn.git_output.contains("? unborn.txt"));
    fixture.write("delete.txt", b"delete\n");
    fixture.write("rename.txt", b"rename\n");
    fixture.write("binary.dat", b"\0binary\xff");
    fixture.git(&["add", "--", "."]);
    fixture.git(&["commit", "-m", "fixture base"]);
    fixture.git(&["mv", "--", "rename.txt", "renamed.txt"]);
    fixture.git(&["rm", "--", "delete.txt"]);
    fixture.write("binary.dat", b"\0changed\xff");
    let status = fixture
        .run(ProjectFileAction::ProjectGitStatus, "")
        .unwrap();
    assert_eq!(
        status.git_output,
        fixture.git(&[
            "status",
            "--porcelain=2",
            "-z",
            "--untracked-files=all",
            "--",
            "."
        ])
    );
    for path in ["delete.txt", "renamed.txt", "binary.dat"] {
        let patch = fixture
            .run(ProjectFileAction::ProjectGitDiff, path)
            .unwrap();
        assert_eq!(
            patch.git_output,
            fixture.git(&[
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "HEAD",
                "--",
                path
            ])
        );
    }
    let mut missing = fixture.query(ProjectFileAction::ProjectGitStatus, "");
    missing.destination = "missing-comparison".into();
    assert!(fixture.execute(missing).is_err());
    fixture.git(&["add", "--", "."]);
    fixture.git(&["commit", "-m", "fixture changes"]);
    fixture.git(&["branch", "other"]);
    fixture.write("unborn.txt", b"main line\n");
    fixture.git(&["commit", "-am", "main change"]);
    fixture.git(&["checkout", "other"]);
    fixture.write("unborn.txt", b"other line\n");
    fixture.git(&["commit", "-am", "other change"]);
    let merge = std::process::Command::new("git")
        .args(["-c", "core.hooksPath=", "merge", "main"])
        .current_dir(fixture.root.path())
        .output()
        .unwrap();
    assert!(!merge.status.success());
    let conflict = fixture
        .run(ProjectFileAction::ProjectGitStatus, "")
        .unwrap();
    assert_eq!(
        conflict.git_output,
        fixture.git(&[
            "status",
            "--porcelain=2",
            "-z",
            "--untracked-files=all",
            "--",
            "."
        ])
    );
    assert!(conflict.git_output.contains("u UU"));
    fixture.git(&["merge", "--abort"]);
    let worktree = Fixture::new();
    fixture.git(&[
        "worktree",
        "add",
        "--detach",
        worktree.root.path().to_str().unwrap(),
        "HEAD",
    ]);
    let mut worktree = worktree;
    let status = worktree
        .run(ProjectFileAction::ProjectGitStatus, "")
        .unwrap();
    assert_eq!(status.git_base, worktree.git(&["rev-parse", "HEAD"]).trim());
    assert!(status.git_output.is_empty());
}
