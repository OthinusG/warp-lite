use super::*;

#[test]
fn default_locations_follow_remote_native_home() {
    let (path, shell) = companion_path("/Users/Remote Person/多语言", "Darwin").unwrap();
    assert_eq!(
        path,
        "/Users/Remote Person/多语言/.config/.warpai/bin/warpai-companion"
    );
    assert!(matches!(shell, RemoteShell::Posix));
    assert_eq!(
        companion_path("/home/remote/", "Linux").unwrap().0,
        "/home/remote/.config/.warpai/bin/warpai-companion"
    );
    let (path, shell) = companion_path(r"C:\Users\Remote Person\多语言", "Windows").unwrap();
    assert_eq!(
        path,
        r"C:\Users\Remote Person\多语言\.config\.warpai\bin\warpai-companion.exe"
    );
    assert!(matches!(shell, RemoteShell::PowerShell));
    for (home, os) in [
        ("relative", "Linux"),
        ("/home/user", "Windows"),
        ("C:\\Users\\User", "Linux"),
        ("/home/user\n", "Linux"),
        ("/home/user", "unknown"),
    ] {
        assert!(companion_path(home, os).is_err());
    }
}

#[test]
fn probes_reject_wrong_protocol_and_unbounded_or_ambiguous_output() {
    assert!(verify_version(b"warpai-companion 1.1.0 protocol 1\n").is_ok());
    assert!(verify_version(b"warpai-companion v1.1.0 protocol 1\r\n").is_ok());
    for output in [
        b"warpai-companion 1.1.0 protocol 2".as_slice(),
        b"warpai-companion 1.1.0 protocol 1 extra",
        b"other 1.1.0 protocol 1",
        b"warpai-companion invalid protocol 1",
        b"login banner\nwarpai-companion 1.1.0 protocol 1",
        b"\xff",
    ] {
        assert!(verify_version(output).is_err());
    }
    assert!(verify_version(&[b'a'; 257]).is_err());
}
