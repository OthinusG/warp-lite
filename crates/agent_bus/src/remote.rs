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

/// Owns one system SSH child. Credentials and protocol payloads have no Debug representation.
pub struct Connection {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    output: tokio::process::ChildStdout,
    diagnostics: tokio::task::JoinHandle<()>,
    coordinator_id: uuid::Uuid,
    connection_epoch: uuid::Uuid,
    principal: Option<(uuid::Uuid, u64, Vec<uuid::Uuid>)>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum HelloResponse {
    Negotiation(NegotiationFrame),
    Authentication(crate::transport::remote_control::AuthenticationFrame),
}

impl Connection {
    pub fn coordinator_id(&self) -> uuid::Uuid {
        self.coordinator_id
    }
    pub fn connection_epoch(&self) -> uuid::Uuid {
        self.connection_epoch
    }

    /// The expected durable authority comes from explicit enrollment, never from host-name matching.
    pub async fn open(alias: &str, coordinator: uuid::Uuid) -> Result<Self> {
        Self::from_command(ssh_command(alias)?, coordinator).await
    }

    async fn from_command(mut command: Command, coordinator: uuid::Uuid) -> Result<Self> {
        use crate::transport::{receive, remote_control::AuthenticationFrame, send};
        use std::time::{Duration, Instant};
        ensure!(
            !coordinator.is_nil(),
            invalid_input("Expected coordinator identity is required")
        );
        command.kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|_| crate::coordinator_unavailable("SSH channel could not start"))?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| crate::invalid_state("SSH input unavailable"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| crate::invalid_state("SSH output unavailable"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| crate::invalid_state("SSH diagnostics unavailable"))?;
        // Discard diagnostics through Tokio's fixed-size copy buffer; never persist SSH output.
        let diagnostics = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
        });
        let mut connection = Self {
            child,
            input,
            output,
            diagnostics,
            coordinator_id: coordinator,
            connection_epoch: uuid::Uuid::nil(),
            principal: None,
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        send(
            &mut connection.input,
            &NegotiationFrame::Hello {
                protocol_major: PROTOCOL_MAJOR,
                protocol_minor: 0,
                features: FEATURES.iter().map(|feature| (*feature).into()).collect(),
                max_frame_bytes: crate::MAX_FRAME as u32,
            },
            deadline,
        )
        .await
        .map_err(|_| crate::coordinator_unavailable("SSH negotiation failed"))?;
        let response: HelloResponse = receive(&mut connection.output, deadline)
            .await
            .map_err(|_| crate::coordinator_unavailable("SSH negotiation failed"))?;
        match response {
            HelloResponse::Negotiation(NegotiationFrame::HelloResult {
                protocol_major,
                features,
                max_frame_bytes,
                coordinator_id,
                connection_epoch,
                ..
            }) => {
                NegotiationFrame::Hello {
                    protocol_major,
                    protocol_minor: 0,
                    features,
                    max_frame_bytes,
                }
                .negotiate(coordinator_id)?;
                ensure!(
                    coordinator_id == coordinator && !connection_epoch.is_nil(),
                    crate::scope_denied("Coordinator authority does not match enrollment")
                );
                connection.connection_epoch = connection_epoch;
            }
            HelloResponse::Authentication(AuthenticationFrame::Error { error }) => {
                return Err(error.into())
            }
            _ => return Err(invalid_input("Unexpected SSH negotiation response")),
        }
        Ok(connection)
    }

    async fn exchange(
        &mut self,
        frame: &crate::transport::remote_control::AuthenticationFrame,
    ) -> Result<crate::transport::remote_control::AuthenticationFrame> {
        use crate::transport::{receive, remote_control::AuthenticationFrame, send};
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        send(&mut self.input, frame, deadline)
            .await
            .map_err(|_| crate::coordinator_unavailable("SSH channel is unavailable"))?;
        let response = receive(&mut self.output, deadline)
            .await
            .map_err(|_| crate::coordinator_unavailable("SSH channel is unavailable"))?;
        match response {
            AuthenticationFrame::Error { error } => Err(error.into()),
            response => Ok(response),
        }
    }

    /// Delivery remains in memory. The app must persist through platform secure storage before use.
    pub async fn enroll(
        &mut self,
        invitation: String,
        name: String,
    ) -> Result<crate::transport::remote_control::AuthenticationFrame> {
        use crate::transport::remote_control::AuthenticationFrame;
        ensure!(
            self.principal.is_none(),
            crate::invalid_state("Already authenticated")
        );
        let response = self
            .exchange(&AuthenticationFrame::Enroll { invitation, name })
            .await?;
        ensure!(
            matches!(&response, AuthenticationFrame::EnrollResult { .. }),
            invalid_input("Unexpected enrollment response")
        );
        Ok(response)
    }

    pub async fn authenticate(&mut self, credential: String) -> Result<uuid::Uuid> {
        use crate::transport::remote_control::AuthenticationFrame;
        ensure!(
            self.principal.is_none(),
            crate::invalid_state("Already authenticated")
        );
        let AuthenticationFrame::Authenticated {
            device_id,
            generation,
            connection_epoch,
            space_ids,
        } = self
            .exchange(&AuthenticationFrame::Authenticate { credential })
            .await?
        else {
            return Err(invalid_input("Unexpected authentication response"));
        };
        ensure!(
            generation > 0
                && connection_epoch == self.connection_epoch
                && !space_ids.is_empty()
                && space_ids.len() <= 32
                && space_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == space_ids.len(),
            invalid_input("Invalid authentication context")
        );
        self.principal = Some((device_id, generation, space_ids));
        Ok(device_id)
    }

    pub async fn heartbeat(&mut self, space: uuid::Uuid) -> Result<()> {
        use crate::transport::remote_control::AuthenticationFrame;
        ensure!(
            self.principal
                .as_ref()
                .is_some_and(|(_, _, spaces)| spaces.contains(&space)),
            crate::scope_denied("Authenticate with a grant for this space")
        );
        let response = self
            .exchange(&AuthenticationFrame::Heartbeat {
                connection_epoch: self.connection_epoch,
                space_id: space,
            })
            .await?;
        ensure!(
            matches!(response, AuthenticationFrame::HeartbeatResult { connection_epoch }
            if connection_epoch == self.connection_epoch),
            invalid_input("Unexpected heartbeat response")
        );
        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.diagnostics.abort();
        let _ = self.child.start_kill();
    }
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
