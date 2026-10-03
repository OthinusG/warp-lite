//! Executed only against an explicitly provisioned isolated CI SSH fixture.
use uuid::Uuid;
use warp_agent_bus::sftp::{SftpClient, SftpError};
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
    assert_ne!(
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
    assert!(matches!(
        sftp.stat("outside-link/passwd").await,
        Err(SftpError::PathEscape)
    ));
    assert!(matches!(
        sftp.read("../outside").await,
        Err(SftpError::InvalidName)
    ));
    sftp.disconnect();
    assert!(matches!(
        sftp.list("").await,
        Err(SftpError::ConnectionLost)
    ));
}
