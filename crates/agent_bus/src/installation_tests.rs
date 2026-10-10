use super::*;

#[test]
fn wsl_installation_is_separate_from_ssh() {
    let home = "/home/Guest Person/多语言/";
    let path = wsl_companion_path(home).unwrap();
    assert_eq!(path, "/home/Guest Person/多语言/.config/.warpai/wsl/bin/warpai-wsl-companion");
    assert_ne!(path, companion_path(home, "Linux").unwrap().0);
    for home in ["relative", "C:\\Users\\Guest", "/home/user\n", ""] {
        assert!(wsl_companion_path(home).is_err());
    }
    assert_eq!(RELEASE_VERSION, "4.0.0");
}

#[test]
fn private_git_runtime_is_required_for_installed_payloads() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        git_executable_in(directory.path()).unwrap(),
        std::path::PathBuf::from("git")
    );
    let runtime = format!("companion-runtime-{}", "a".repeat(64));
    let manifest = directory.path().join("companion-manifest.json");
    std::fs::write(
        &manifest,
        serde_json::json!({"runtime_directory": runtime}).to_string(),
    )
    .unwrap();
    assert!(git_executable_in(directory.path()).is_err());
    let binary = directory.path().join(&runtime).join(if cfg!(windows) {
        "git/cmd/git.exe"
    } else {
        "git/bin/git"
    });
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::write(&binary, b"fixture").unwrap();
    assert_eq!(git_executable_in(directory.path()).unwrap(), binary);
    for invalid in [
        "../git",
        "companion-runtime-../../git",
        "companion-runtime-short",
    ] {
        std::fs::write(
            &manifest,
            serde_json::json!({"runtime_directory": invalid}).to_string(),
        )
        .unwrap();
        assert!(git_executable_in(directory.path()).is_err());
    }
}

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
    assert!(
        verify_version(format!("warpai-companion {RELEASE_VERSION} protocol 1\n").as_bytes())
            .is_ok()
    );
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
