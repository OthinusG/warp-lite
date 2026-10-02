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

pub fn validate_alias(alias: &str) -> Result<()> {
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
    Ok(())
}

fn build_ssh_command(executable: &Path, alias: &str) -> Result<Command> {
    validate_alias(alias)?;
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

// SSH stderr and peer messages can contain reflected credentials; retain only known codes.
fn remote_error(error: crate::DomainError) -> anyhow::Error {
    let code = match error.code.as_str() {
        "invalid_input"
        | "invalid_state"
        | "unauthorized"
        | "scope_denied"
        | "device_revoked"
        | "protocol_incompatible"
        | "feature_unavailable"
        | "capacity_exceeded"
        | "storage_full"
        | "coordinator_unavailable"
        | "request_conflict"
        | "request_epoch_expired"
        | "execution_unknown"
        | "stale_attempt"
        | "stale_revision"
        | "version_conflict"
        | "reservation_conflict"
        | "dependency_blocked"
        | "cursor_expired"
        | "dependency_cycle" => error.code.as_str(),
        _ => {
            return crate::domain(
                "invalid_input",
                "Remote collaboration request was rejected",
                false,
                None,
            )
        }
    };
    crate::domain(
        code,
        "Remote collaboration request was rejected",
        error.retryable,
        error.version,
    )
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
            AuthenticationFrame::Error { error } => Err(remote_error(error)),
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
            matches!(&response, AuthenticationFrame::EnrollResult { device_id, credential, generation, space_ids }
                if !device_id.is_nil() && *generation > 0 && !credential.is_empty() && credential.len() <= 512
                    && !space_ids.is_empty() && space_ids.len() <= 32
                    && space_ids.iter().all(|space| !space.is_nil())
                    && space_ids.iter().collect::<std::collections::HashSet<_>>().len() == space_ids.len()),
            invalid_input("Invalid enrollment response")
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
            !device_id.is_nil()
                && generation > 0
                && space_ids.iter().all(|space| !space.is_nil())
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

    /// The participant app supplies its reviewed mapping and verified native identities, never MCP arguments.
    pub async fn announce(
        &mut self,
        workspace: uuid::Uuid,
        space: uuid::Uuid,
        checkout: uuid::Uuid,
        native_session: uuid::Uuid,
        native_run: uuid::Uuid,
        program: &str,
        name: &str,
    ) -> Result<crate::storage::RemoteActor> {
        use crate::transport::remote_control::AuthenticationFrame;
        let device = self
            .principal
            .as_ref()
            .filter(|(_, _, spaces)| spaces.contains(&space))
            .map(|(device, _, _)| *device)
            .ok_or_else(|| crate::scope_denied("Authenticate with a grant for this space"))?;
        ensure!(
            !workspace.is_nil()
                && !checkout.is_nil()
                && !native_session.is_nil()
                && !native_run.is_nil(),
            invalid_input("Reviewed workspace and native identities are required")
        );
        let AuthenticationFrame::ActorAnnounced {
            connection_epoch,
            actor,
            mutation_epoch,
            expires_at,
        } = self
            .exchange(&AuthenticationFrame::ActorAnnounce {
                connection_epoch: self.connection_epoch,
                workspace_id: workspace,
                space_id: space,
                checkout_id: checkout,
                native_session,
                native_run,
                program: program.into(),
                name: name.into(),
            })
            .await?
        else {
            return Err(invalid_input("Unexpected actor admission response"));
        };
        ensure!(
            connection_epoch == self.connection_epoch
                && !mutation_epoch.is_nil()
                && expires_at > 0
                && uuid::Uuid::parse_str(&actor.id).is_ok_and(|id| !id.is_nil())
                && actor.terminal == format!("remote:{device}:{native_session}")
                && actor.project == format!("space:{space}")
                && actor.program == program,
            invalid_input("Remote actor admission does not match the reviewed native context")
        );
        Ok(crate::storage::RemoteActor {
            actor,
            epoch: mutation_epoch.to_string(),
            expires_at,
        })
    }

    /// Send an original intent only after the app has durably recorded it for reconciliation.
    pub async fn execute(
        &mut self,
        actor: &crate::storage::RemoteActor,
        operation: &crate::Operation,
    ) -> Result<serde_json::Value> {
        self.actor_request(actor, operation, false).await
    }

    /// Query the original receipt before replaying an intent whose response was lost.
    pub async fn reconcile(
        &mut self,
        actor: &crate::storage::RemoteActor,
        operation: &crate::Operation,
    ) -> Result<serde_json::Value> {
        ensure!(
            operation.request_id().is_some(),
            invalid_input("Reconcile an original mutation")
        );
        self.actor_request(actor, operation, true).await
    }

    async fn actor_request(
        &mut self,
        actor: &crate::storage::RemoteActor,
        operation: &crate::Operation,
        reconcile: bool,
    ) -> Result<serde_json::Value> {
        let (actor_id, mutation_epoch) = self.actor_authority(actor)?;
        if let Some(request) = operation.request_id() {
            ensure!(
                uuid::Uuid::parse_str(request).is_ok(),
                invalid_input("Original mutation request UUID is required")
            );
        }
        self.wire_operation(actor_id, mutation_epoch, operation, reconcile)
            .await
    }

    fn actor_authority(
        &self,
        actor: &crate::storage::RemoteActor,
    ) -> Result<(uuid::Uuid, uuid::Uuid)> {
        let space = actor
            .actor
            .project
            .strip_prefix("space:")
            .and_then(|space| uuid::Uuid::parse_str(space).ok())
            .ok_or_else(|| invalid_input("Remote actor scope is unavailable"))?;
        ensure!(
            self.principal
                .as_ref()
                .is_some_and(|(device, _, spaces)| spaces.contains(&space)
                    && actor
                        .actor
                        .terminal
                        .starts_with(&format!("remote:{device}:"))),
            crate::scope_denied("Actor belongs to another authenticated device or space")
        );
        let actor_id = uuid::Uuid::parse_str(&actor.actor.id)
            .map_err(|_| invalid_input("Invalid remote actor identity"))?;
        let mutation_epoch = uuid::Uuid::parse_str(&actor.epoch)
            .map_err(|_| invalid_input("Invalid original mutation epoch"))?;
        ensure!(
            !actor_id.is_nil() && !mutation_epoch.is_nil(),
            invalid_input("Original native identities are required")
        );
        Ok((actor_id, mutation_epoch))
    }

    pub async fn actor_heartbeat(&mut self, actor: &crate::storage::RemoteActor) -> Result<()> {
        use crate::transport::remote_control::AuthenticationFrame;
        let (actor_id, mutation_epoch) = self.actor_authority(actor)?;
        let response = self
            .exchange(&AuthenticationFrame::ActorHeartbeat {
                connection_epoch: self.connection_epoch,
                actor_id,
                mutation_epoch,
            })
            .await?;
        ensure!(
            matches!(response, AuthenticationFrame::ActorHeartbeatResult {
            connection_epoch, actor_id: received_actor, mutation_epoch: received_epoch,
        } if connection_epoch == self.connection_epoch && received_actor == actor_id
            && received_epoch == mutation_epoch),
            invalid_input("Native presence response does not match the run")
        );
        Ok(())
    }

    /// Send a previously staged intent without reconstructing or replacing its authority.
    pub async fn execute_intent(
        &mut self,
        intent: &crate::storage::RemoteIntent,
    ) -> Result<serde_json::Value> {
        self.intent_request(intent, false).await
    }

    pub async fn reconcile_intent(
        &mut self,
        intent: &crate::storage::RemoteIntent,
    ) -> Result<serde_json::Value> {
        self.intent_request(intent, true).await
    }

    async fn intent_request(
        &mut self,
        intent: &crate::storage::RemoteIntent,
        reconcile: bool,
    ) -> Result<serde_json::Value> {
        let space = uuid::Uuid::parse_str(&intent.space)
            .map_err(|_| invalid_input("Invalid original remote space"))?;
        ensure!(
            intent.coordinator == self.coordinator_id.to_string()
                && self
                    .principal
                    .as_ref()
                    .is_some_and(|(device, _, spaces)| intent.device == device.to_string()
                        && spaces.contains(&space)),
            crate::scope_denied("Intent belongs to another coordinator, device or space")
        );
        let actor = uuid::Uuid::parse_str(&intent.actor)
            .map_err(|_| invalid_input("Invalid original remote actor"))?;
        let epoch = uuid::Uuid::parse_str(&intent.epoch)
            .map_err(|_| invalid_input("Invalid original mutation epoch"))?;
        let operation: crate::Operation = serde_json::from_str(&intent.operation)
            .map_err(|_| invalid_input("Original intent payload is invalid"))?;
        ensure!(
            operation.request_id() == Some(intent.request_id.as_str())
                && uuid::Uuid::parse_str(&intent.request_id).is_ok_and(|id| !id.is_nil())
                && !actor.is_nil()
                && !epoch.is_nil(),
            invalid_input("Original intent identities do not match")
        );
        self.wire_operation(actor, epoch, &operation, reconcile)
            .await
    }

    async fn wire_operation(
        &mut self,
        actor_id: uuid::Uuid,
        mutation_epoch: uuid::Uuid,
        operation: &crate::Operation,
        reconcile: bool,
    ) -> Result<serde_json::Value> {
        use crate::transport::remote_control::AuthenticationFrame;
        let frame_id = uuid::Uuid::new_v4();
        let request = if reconcile {
            AuthenticationFrame::Reconcile {
                connection_epoch: self.connection_epoch,
                frame_id,
                actor_id,
                mutation_epoch,
                operation: operation.clone(),
            }
        } else {
            AuthenticationFrame::Operation {
                connection_epoch: self.connection_epoch,
                frame_id,
                actor_id,
                mutation_epoch,
                operation: operation.clone(),
            }
        };
        let (connection_epoch, received_id, received_epoch, result) =
            match self.exchange(&request).await? {
                AuthenticationFrame::OperationResult {
                    connection_epoch,
                    frame_id,
                    mutation_epoch,
                    result,
                } if !reconcile => (connection_epoch, frame_id, mutation_epoch, result),
                AuthenticationFrame::Reconciled {
                    connection_epoch,
                    frame_id,
                    mutation_epoch,
                    result,
                } if reconcile => (connection_epoch, frame_id, mutation_epoch, result),
                _ => return Err(invalid_input("Unexpected original-intent response")),
            };
        ensure!(
            connection_epoch == self.connection_epoch
                && received_id == frame_id
                && received_epoch == mutation_epoch,
            invalid_input("Operation response does not match the original request")
        );
        Ok(result)
    }

    pub async fn events(
        &mut self,
        space: uuid::Uuid,
        after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<serde_json::Value> {
        self.projection_request(
            crate::transport::remote_control::AuthenticationFrame::Events {
                connection_epoch: self.connection_epoch,
                frame_id: uuid::Uuid::new_v4(),
                space_id: space,
                after,
                limit,
            },
        )
        .await
    }

    pub async fn snapshot(
        &mut self,
        space: uuid::Uuid,
        after: Option<u64>,
        expected_sequence: Option<u64>,
        limit: Option<u32>,
    ) -> Result<serde_json::Value> {
        self.projection_request(
            crate::transport::remote_control::AuthenticationFrame::Snapshot {
                connection_epoch: self.connection_epoch,
                frame_id: uuid::Uuid::new_v4(),
                space_id: space,
                after,
                expected_sequence,
                limit,
            },
        )
        .await
    }

    pub async fn acknowledge_cursor(
        &mut self,
        space: uuid::Uuid,
        sequence: u64,
    ) -> Result<serde_json::Value> {
        self.projection_request(
            crate::transport::remote_control::AuthenticationFrame::CursorAck {
                connection_epoch: self.connection_epoch,
                frame_id: uuid::Uuid::new_v4(),
                space_id: space,
                sequence,
            },
        )
        .await
    }

    async fn projection_request(
        &mut self,
        request: crate::transport::remote_control::AuthenticationFrame,
    ) -> Result<serde_json::Value> {
        use crate::transport::remote_control::AuthenticationFrame;
        let (space, frame_id) = match &request {
            AuthenticationFrame::Events {
                space_id, frame_id, ..
            }
            | AuthenticationFrame::Snapshot {
                space_id, frame_id, ..
            }
            | AuthenticationFrame::CursorAck {
                space_id, frame_id, ..
            } => (*space_id, *frame_id),
            _ => return Err(invalid_input("Unexpected projection request")),
        };
        ensure!(
            self.principal
                .as_ref()
                .is_some_and(|(_, _, spaces)| spaces.contains(&space)),
            crate::scope_denied("Authenticate with a grant for this space")
        );
        let response = self.exchange(&request).await?;
        let (connection_epoch, received_id, result) = match (request, response) {
            (
                AuthenticationFrame::Events { .. },
                AuthenticationFrame::EventsResult {
                    connection_epoch,
                    frame_id,
                    result,
                },
            )
            | (
                AuthenticationFrame::Snapshot { .. },
                AuthenticationFrame::SnapshotResult {
                    connection_epoch,
                    frame_id,
                    result,
                },
            )
            | (
                AuthenticationFrame::CursorAck { .. },
                AuthenticationFrame::CursorAckResult {
                    connection_epoch,
                    frame_id,
                    result,
                },
            ) => (connection_epoch, frame_id, result),
            _ => return Err(invalid_input("Unexpected projection response")),
        };
        ensure!(
            connection_epoch == self.connection_epoch && received_id == frame_id,
            invalid_input("Projection response does not match the original request")
        );
        Ok(result)
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
