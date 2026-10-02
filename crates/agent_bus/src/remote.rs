//! System SSH startup for the internal collaboration channel.
use crate::{domain, invalid_input};
use anyhow::{ensure, Result};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

const GATEWAY_COMMAND: &str = "warp-agent remote-stdio";
const SSH_OPTIONS: &[&str] = &[
    "BatchMode=yes",
    "StrictHostKeyChecking=yes",
    "ForwardAgent=no",
    "ForwardX11=no",
    "ClearAllForwardings=yes",
    "PermitLocalCommand=no",
    "RemoteCommand=none",
    "RequestTTY=no",
    "StdinNull=no",
    "ConnectTimeout=10",
    "ServerAliveInterval=10",
    "ServerAliveCountMax=3",
    "ControlMaster=no",
    "ControlPath=none",
    "ControlPersist=no",
];

/// Uses system OpenSSH only; missing installations require explicit user setup.
pub fn ssh_executable() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let path = PathBuf::from("/usr/bin/ssh");
    #[cfg(windows)]
    let path = PathBuf::from(std::env::var_os("WINDIR").ok_or_else(|| {
        domain(
            "feature_unavailable",
            "System OpenSSH location is unavailable",
            false,
            None,
        )
    })?)
    .join("System32")
    .join("OpenSSH")
    .join("ssh.exe");
    ensure!(
        path.is_absolute() && path.is_file(),
        domain(
            "feature_unavailable",
            "Install the system OpenSSH client before connecting",
            false,
            None
        )
    );
    Ok(path)
}

/// Background connections never prompt on protocol stdin or change known hosts.
pub fn ssh_command(alias: &str) -> Result<Command> {
    build_ssh_command(&ssh_executable()?, alias)
}

fn build_ssh_command(executable: &Path, alias: &str) -> Result<Command> {
    ensure!(
        !alias.is_empty()
            && alias.len() <= 253
            && alias.as_bytes()[0].is_ascii_alphanumeric()
            && alias
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')),
        invalid_input(
            "Use an SSH host alias containing letters, numbers, dots, underscores or hyphens"
        )
    );
    let mut command = Command::new(executable);
    command.arg("-T");
    for option in SSH_OPTIONS {
        command.args(["-o", option]);
    }
    command
        .arg(alias)
        .arg(GATEWAY_COMMAND)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    Ok(command)
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
