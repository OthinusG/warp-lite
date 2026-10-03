//! Executed only against an explicitly provisioned isolated CI SSH fixture.
use warp_agent_bus::ssh_remote::{HostClient, RemoteShell, SshProfile};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires the controlled OpenSSH fixture"]
async fn controlled_ssh_host_status_never_uses_local_paths_or_replaced_attachments() {
    let config = std::env::var_os("WARP_TEST_SSH_CONFIG").expect("Controlled SSH config");
    let root = std::env::var("WARP_TEST_REMOTE_ROOT").expect("Remote fixture root");
    let companion = std::env::var("WARP_TEST_COMPANION_PATH").expect("Owned companion path");
    let profile = SshProfile { id: Uuid::new_v4(), display_name: "Controlled SSH".into(), target: "warpai-test".into(),
        user: None, port: None, identity_file: None, config_file: Some(config.into()), jump_alias: None,
        remote_root: root, companion_path: Some(companion), remote_shell: RemoteShell::Posix };
    let mut first = HostClient::connect(&profile).await.expect("Verified SSH companion");
    assert!(std::path::Path::new(&first.canonical_root).is_absolute());
    let one = first.host_status(1).await.unwrap();
    assert_eq!(one.source, "companion_native");
    assert_eq!(one.query_generation, 1);
    let mut second = HostClient::connect(&profile).await.unwrap();
    let two = second.host_status(2).await.unwrap();
    assert_ne!(one.fence.as_ref().unwrap().connection_id, two.fence.as_ref().unwrap().connection_id);
    assert_ne!(one.fence.as_ref().unwrap().service_boot_id, two.fence.as_ref().unwrap().service_boot_id);
    let mut projection = remote_protocol::managed::HostObservation::new(two.fence.unwrap(), 2).unwrap();
    assert!(!projection.receive(one, std::time::Instant::now()));
    // The alias resolves through a controlled config, not local project canonicalization.
    let mut missing = profile.clone();
    missing.remote_root.push_str("/missing-root");
    assert!(HostClient::connect(&missing).await.is_err());
}
