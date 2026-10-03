//! Executed only against an explicitly provisioned isolated CI SSH fixture.
use uuid::Uuid;
use warp_agent_bus::sftp::{SftpClient, SftpError, UploadState};
use warp_agent_bus::ssh_remote::{HostClient, RemoteShell, SshProfile};

#[tokio::test]
#[ignore = "requires the controlled OpenSSH fixture"]
async fn controlled_ssh_uses_companion_fences_and_file_only_sftp() {
    let config = std::env::var_os("WARP_TEST_SSH_CONFIG").expect("Controlled SSH config");
    let root = std::env::var("WARP_TEST_REMOTE_ROOT").expect("Remote fixture root");
    let companion = std::env::var("WARP_TEST_COMPANION_PATH").expect("Owned companion path");
    let profile = SshProfile {
        id: Uuid::new_v4(),
        display_name: "Controlled SSH".into(),
        target: "warpai-test".into(),
        user: None,
        port: None,
        identity_file: None,
        config_file: Some(config.into()),
        jump_alias: None,
        remote_root: root,
        companion_path: Some(companion),
        remote_shell: RemoteShell::Posix,
    };
    let mut first = HostClient::connect(&profile)
        .await
        .expect("Verified SSH companion");
    assert!(std::path::Path::new(&first.canonical_root).is_absolute());
    let one = first.host_status(1).await.unwrap();
    assert_eq!(one.source, "companion_native");
    assert_eq!(one.query_generation, 1);
    let mut second = HostClient::connect(&profile).await.unwrap();
    let two = second.host_status(2).await.unwrap();
    assert_ne!(
        one.fence.as_ref().unwrap().connection_id,
        two.fence.as_ref().unwrap().connection_id
    );
    assert_eq!(
        one.fence.as_ref().unwrap().service_boot_id,
        two.fence.as_ref().unwrap().service_boot_id
    );
    assert_eq!(
        one.fence.as_ref().unwrap().service_id,
        two.fence.as_ref().unwrap().service_id
    );
    assert_eq!(
        one.fence.as_ref().unwrap().project_id,
        two.fence.as_ref().unwrap().project_id
    );
    assert_eq!(first.account_id, second.account_id);
    assert_eq!(first.root_identity, second.root_identity);
    let mut projection =
        remote_protocol::managed::HostObservation::new(two.fence.unwrap(), 2).unwrap();
    assert!(!projection.receive(one, std::time::Instant::now()));
    // The alias resolves through a controlled config, not local project canonicalization.
    let mut missing = profile.clone();
    missing.remote_root.push_str("/missing-root");
    assert!(HostClient::connect(&missing).await.is_err());
    let mut file_only = profile;
    file_only.companion_path = None;
    let mut sftp = SftpClient::connect(&file_only)
        .await
        .expect("Independent SFTP subsystem");
    let listing = sftp.list("").await.unwrap();
    assert!(!listing.truncated);
    assert!(listing
        .entries
        .iter()
        .any(|entry| entry.name == "folder" && entry.attributes.is_directory()));
    assert!(listing
        .entries
        .iter()
        .any(|entry| entry.name == "outside-link" && entry.attributes.is_symlink()));
    let bytes = sftp.read("Unicode 空格;$(shell).bin").await.unwrap();
    let expected: Vec<u8> = (0..257).flat_map(|_| 0u8..=255).collect();
    assert_eq!(bytes, expected);
    let local = tempfile::tempdir().unwrap();
    let partial = local.path().join("download.partial");
    let mut output = tokio::fs::File::create(&partial).await.unwrap();
    let digest = sftp
        .read_to("Unicode 空格;$(shell).bin", &mut output, 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(digest.size, expected.len() as u64);
    use sha2::{Digest, Sha256};
    assert_eq!(digest.sha256, <[u8; 32]>::from(Sha256::digest(&expected)));
    assert_eq!(tokio::fs::read(&partial).await.unwrap(), expected);
    assert_eq!(
        sftp.read_to("Unicode 空格;$(shell).bin", &mut tokio::io::sink(), 16)
            .await,
        Err(SftpError::CapacityExceeded)
    );
    let (mut closed, receiver) = tokio::io::duplex(1);
    drop(receiver);
    assert_eq!(
        sftp.read_to("Unicode 空格;$(shell).bin", &mut closed, 1024 * 1024)
            .await,
        Err(SftpError::LocalIo)
    );
    assert_eq!(
        sftp.read("Unicode 空格;$(shell).bin").await.unwrap(),
        expected
    );
    assert!(matches!(
        sftp.stat("outside-link/passwd").await,
        Err(SftpError::PathEscape)
    ));
    assert!(matches!(
        sftp.read("../outside").await,
        Err(SftpError::InvalidName)
    ));
    assert!(matches!(sftp.remove("").await, Err(SftpError::InvalidName)));
    assert!(matches!(
        sftp.create_directory("../escape").await,
        Err(SftpError::InvalidName)
    ));
    let directory = format!("Upload 多语言 {}", Uuid::new_v4());
    sftp.create_directory(&directory).await.unwrap();
    let destination = format!("{directory}/binary ;$(never-execute).bin");
    let receipt = sftp.upload_intent(&destination, &expected).unwrap();
    sftp.prepare_upload(&receipt, &expected).await.unwrap();
    assert_eq!(
        sftp.reconcile_upload(&receipt).await.unwrap(),
        UploadState::Prepared
    );
    // Prepared content survives the original SSH attachment; reconcile before any retry.
    sftp.disconnect();
    let mut sftp = SftpClient::connect(&file_only).await.unwrap();
    assert_eq!(
        sftp.reconcile_upload(&receipt).await.unwrap(),
        UploadState::Prepared
    );
    sftp.commit_upload(&receipt).await.unwrap();
    assert_eq!(
        sftp.reconcile_upload(&receipt).await.unwrap(),
        UploadState::Confirmed
    );
    assert_eq!(sftp.read(&destination).await.unwrap(), expected);
    let competing = sftp
        .upload_intent(&destination, b"must not replace")
        .unwrap();
    sftp.prepare_upload(&competing, b"must not replace")
        .await
        .unwrap();
    assert!(matches!(
        sftp.commit_upload(&competing).await,
        Err(SftpError::Conflict)
    ));
    assert_eq!(sftp.read(&destination).await.unwrap(), expected);
    sftp.remove(&competing.partial).await.unwrap();
    let renamed = format!("{directory}/renamed.bin");
    sftp.rename_new(&destination, &renamed).await.unwrap();
    assert_eq!(sftp.read(&renamed).await.unwrap(), expected);
    sftp.remove(&renamed).await.unwrap();
    sftp.remove(&directory).await.unwrap();
    // Transfer exceeds the editor ceiling without an in-memory file buffer.
    use tokio::io::AsyncReadExt;
    let size = 16 * 1024 * 1024 + 13;
    let mut hash = Sha256::new();
    for _ in 0..512 {
        hash.update([0x5au8; 32768]);
    }
    hash.update([0x5au8; 13]);
    let digest = warp_agent_bus::sftp::FileDigest {
        size,
        sha256: hash.finalize().into(),
    };
    let large = format!("streamed-{}.bin", Uuid::new_v4());
    let intent = sftp.upload_intent_digest(&large, digest.clone()).unwrap();
    sftp.prepare_upload_from(&intent, &mut tokio::io::repeat(0x5a).take(size))
        .await
        .unwrap();
    sftp.commit_upload(&intent).await.unwrap();
    assert_eq!(sftp.read(&large).await, Err(SftpError::CapacityExceeded));
    assert_eq!(
        sftp.read_to(&large, &mut tokio::io::sink(), size)
            .await
            .unwrap(),
        digest
    );
    sftp.remove(&large).await.unwrap();
    let changed = sftp
        .upload_intent("source-changed.bin", b"original")
        .unwrap();
    assert_eq!(
        sftp.prepare_upload_from(&changed, &mut &b"modified"[..])
            .await,
        Err(SftpError::Conflict)
    );
    assert!(matches!(
        sftp.stat(&directory).await,
        Err(SftpError::Missing)
    ));
    sftp.disconnect();
    assert!(matches!(
        sftp.list("").await,
        Err(SftpError::ConnectionLost)
    ));
}
