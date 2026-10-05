//! Remote installation metadata comes from the authenticated shell session, never local paths.
use crate::ssh_remote::{ConnectionError, RemoteShell};

pub const RELEASE_VERSION: &str = "1.1.0";

/// Resolve the installer-owned executable without asking for a path or changing PATH.
pub fn companion_path(home: &str, os: &str) -> Result<(String, RemoteShell), ConnectionError> {
    if home.is_empty() || home.len() > 4000 || home.chars().any(char::is_control) {
        return Err(ConnectionError::InvalidProfile);
    }
    match os {
        "Linux" | "Darwin" | "MacOS" | "macos" | "linux" if home.starts_with('/') => Ok((
            format!(
                "{}/.config/.warpai/bin/warpai-companion",
                home.trim_end_matches('/')
            ),
            RemoteShell::Posix,
        )),
        "Windows" | "windows"
            if {
                let bytes = home.as_bytes();
                bytes.len() >= 3
                    && bytes[0].is_ascii_alphabetic()
                    && bytes[1] == b':'
                    && matches!(bytes[2], b'\\' | b'/')
            } =>
        {
            Ok((
                format!(
                    r"{}\.config\.warpai\bin\warpai-companion.exe",
                    home.trim_end_matches(['\\', '/'])
                ),
                RemoteShell::PowerShell,
            ))
        }
        _ => Err(ConnectionError::InvalidProfile),
    }
}

/// A successful executable probe must also identify the supported control protocol.
pub fn verify_version(output: &[u8]) -> Result<(), ConnectionError> {
    if output.len() > 256 {
        return Err(ConnectionError::IncompatibleVersion);
    }
    let output = std::str::from_utf8(output).map_err(|_| ConnectionError::IncompatibleVersion)?;
    let mut parts = output.split_whitespace();
    if parts.next() != Some("warpai-companion") {
        return Err(ConnectionError::IncompatibleVersion);
    }
    let version = parts
        .next()
        .ok_or(ConnectionError::IncompatibleVersion)?
        .trim_start_matches('v');
    if version.split('.').count() != 3
        || !version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        || parts.next() != Some("protocol")
        || parts.next().and_then(|part| part.parse::<u32>().ok())
            != Some(remote_protocol::managed::PROTOCOL_MAJOR)
        || parts.next().is_some()
    {
        return Err(ConnectionError::IncompatibleVersion);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
