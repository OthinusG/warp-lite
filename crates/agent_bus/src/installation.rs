//! Remote installation metadata comes from the authenticated shell session, never local paths.
use crate::ssh_remote::{ConnectionError, RemoteShell};

pub const RELEASE_VERSION: &str = "4.0.0";

/// Native installers own a private Git runtime; unpackaged development builds use system Git.
pub fn git_executable() -> std::io::Result<std::path::PathBuf> {
    git_executable_in(std::env::current_exe()?.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Companion directory unavailable",
        )
    })?)
}

fn git_executable_in(directory: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let manifest = match std::fs::read(directory.join("companion-manifest.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok("git".into()),
        Err(error) => return Err(error),
    };
    let manifest: serde_json::Value = serde_json::from_slice(&manifest)?;
    let runtime = manifest["runtime_directory"]
        .as_str()
        .filter(|name| {
            name.strip_prefix("companion-runtime-").is_some_and(|hash| {
                hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        })
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Companion runtime")
        })?;
    let executable = directory.join(runtime).join(if cfg!(windows) {
        "git/cmd/git.exe"
    } else {
        "git/bin/git"
    });
    if !executable.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Companion Git runtime unavailable",
        ));
    }
    Ok(executable)
}

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

/// Resolve the dedicated WSL installation without sharing the SSH executable.
#[cfg(any(windows, feature = "wsl_companion", test))]
pub fn wsl_companion_path(home: &str) -> Result<String, ConnectionError> {
    // Reuse the native home validation without pointing WSL to the SSH installation.
    companion_path(home, "Linux")?;
    Ok(format!("{}/.config/.warpai/wsl/bin/warpai-wsl-companion", home.trim_end_matches('/')))
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
#[path = "installation_tests.rs"]
mod tests;
