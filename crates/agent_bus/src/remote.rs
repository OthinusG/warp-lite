//! System SSH startup for the internal collaboration channel.
use crate::{domain, invalid_input};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

const PROTOCOL_MAJOR: u16 = 2;
const FEATURES: &[&str] = &[
    "task_control",
    "dependencies",
    "threads",
    "reservations",
    "evidence_refs",
    "event_resume",
];

/// Negotiation is unauthenticated metadata and grants no operation authority.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NegotiationFrame {
    Hello {
        protocol_major: u16,
        protocol_minor: u16,
        features: Vec<String>,
        max_frame_bytes: u32,
    },
    HelloResult {
        protocol_major: u16,
        protocol_minor: u16,
        features: Vec<String>,
        max_frame_bytes: u32,
        coordinator_id: uuid::Uuid,
        connection_epoch: uuid::Uuid,
        presence_lease_ms: u32,
    },
}

impl NegotiationFrame {
    /// Refuse semantic downgrade before enrolling or admitting any operation.
    pub fn negotiate(self, coordinator_id: uuid::Uuid) -> Result<Self> {
        let Self::Hello {
            protocol_major,
            protocol_minor: _,
            features,
            max_frame_bytes,
        } = self
        else {
            return Err(invalid_input("Expected the initial hello frame"));
        };
        ensure!(
            protocol_major == PROTOCOL_MAJOR,
            domain(
                "protocol_incompatible",
                "Upgrade both applications to compatible collaboration versions",
                false,
                None
            )
        );
        ensure!(
            (4096..=crate::MAX_FRAME as u32).contains(&max_frame_bytes)
                && features.len() <= 32
                && features.iter().all(|feature| !feature.is_empty()
                    && feature.len() <= 64
                    && feature.bytes().all(|byte| byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || byte == b'_'))
                && features
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == features.len(),
            invalid_input("Invalid collaboration negotiation limits or features")
        );
        ensure!(
            FEATURES
                .iter()
                .all(|required| features.iter().any(|feature| feature == required)),
            domain(
                "feature_unavailable",
                "Upgrade the peer to support required shared-space semantics",
                false,
                None
            )
        );
        Ok(Self::HelloResult {
            protocol_major: PROTOCOL_MAJOR,
            protocol_minor: 0,
            features: FEATURES
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
            max_frame_bytes,
            coordinator_id,
            connection_epoch: uuid::Uuid::new_v4(),
            presence_lease_ms: 30_000,
        })
    }
}

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
