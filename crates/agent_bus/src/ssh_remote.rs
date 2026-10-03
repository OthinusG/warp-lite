//! System OpenSSH metadata and a clean, strictly verified project control channel.
use std::{path::PathBuf, process::Stdio, time::Duration};

use base64::Engine;
use remote_protocol::{
    managed::{valid_fence, PROTOCOL_MAJOR},
    proto::*,
    protocol::{read_message_with_limit, write_message_with_limit, MAX_MANAGED_MESSAGE_SIZE},
};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteShell {
    Posix,
    PowerShell,
}

/// Only metadata is persisted. Key contents and authentication stay with OpenSSH.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshProfile {
    pub id: Uuid,
    pub display_name: String,
    pub target: String,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    /// An existing OpenSSH config alias; jump routing remains system-managed.
    pub jump_alias: Option<String>,
    pub remote_root: String,
    pub companion_path: Option<String>,
    pub remote_shell: RemoteShell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionError {
    InvalidProfile,
    SshUnavailable,
    ConnectionLost,
    IncompatibleVersion,
    StaleAttachment,
    CompanionUnavailable,
    FeatureUnavailable,
    InvalidInput,
    Conflict,
    CapacityExceeded,
}

fn target(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:[]".contains(&b))
}

fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

impl SshProfile {
    pub fn validate(&self) -> Result<(), ConnectionError> {
        if self.id.is_nil()
            || !text(&self.display_name, 256)
            || !target(&self.target)
            || self.port == Some(0)
            || self.jump_alias.as_ref().is_some_and(|v| !target(v))
            || self.user.as_ref().is_some_and(|v| {
                !text(v, 256)
                    || v.starts_with('-')
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-\\".contains(&b))
            })
            || self
                .identity_file
                .as_ref()
                .is_some_and(|p| p.to_str().is_none_or(|v| !text(v, 4096)))
            || self
                .config_file
                .as_ref()
                .is_some_and(|p| p.to_str().is_none_or(|v| !text(v, 4096)))
            || !text(&self.remote_root, 4096)
            || self.companion_path.as_ref().is_some_and(|p| !text(p, 4096))
            || match self.remote_shell {
                RemoteShell::Posix => {
                    !self.remote_root.starts_with('/')
                        || self
                            .companion_path
                            .as_ref()
                            .is_some_and(|p| !p.starts_with('/'))
                }
                RemoteShell::PowerShell => {
                    !windows_absolute(&self.remote_root)
                        || self
                            .companion_path
                            .as_ref()
                            .is_some_and(|p| !windows_absolute(p))
                }
            }
        {
            return Err(ConnectionError::InvalidProfile);
        }
        Ok(())
    }

    /// Interactive authentication is an explicit terminal action, separate from protocol pipes.
    pub fn authentication_command(&self) -> Result<Command, ConnectionError> {
        let mut command = self.ssh_command(false)?;
        command.arg("-tt").arg("--").arg(&self.target);
        Ok(command)
    }

    pub(crate) fn ssh_command(&self, machine: bool) -> Result<Command, ConnectionError> {
        self.validate()?;
        let mut command = Command::new("ssh");
        crate::session::without_terminal_binding(&mut command);
        command.env_remove("VIBE_MCP_SERVERS");
        // Keep system authentication agent/config; never forward native terminal capabilities.
        command.args([
            "-o",
            "ForwardAgent=no",
            "-o",
            "ForwardX11=no",
            "-o",
            "PermitLocalCommand=no",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=2",
        ]);
        if machine {
            command.args([
                "-T",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
            ]);
        } else {
            command.args(["-o", "StrictHostKeyChecking=ask"]);
        }
        if let Some(user) = &self.user {
            command.arg("-l").arg(user);
        }
        if let Some(port) = self.port {
            command.arg("-p").arg(port.to_string());
        }
        if let Some(identity) = &self.identity_file {
            command.arg("-i").arg(identity);
        }
        if let Some(config) = &self.config_file {
            command.arg("-F").arg(config);
        }
        if let Some(jump) = &self.jump_alias {
            command.arg("-J").arg(jump);
        }
        command.kill_on_drop(true);
        Ok(command)
    }

    /// SSH joins remote argv into a shell program; quote for that separate boundary.
    fn companion_command(&self) -> Result<String, ConnectionError> {
        let path = self
            .companion_path
            .as_ref()
            .ok_or(ConnectionError::CompanionUnavailable)?;
        let quoted = format!("'{}'", path.replace('\'', "'\\''"));
        Ok(match self.remote_shell {
            RemoteShell::Posix => format!("exec {quoted}"),
            RemoteShell::PowerShell => {
                let script = format!("& '{}'", path.replace('\'', "''"));
                let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
                format!(
                    "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand {}",
                    base64::engine::general_purpose::STANDARD.encode(utf16)
                )
            }
        })
    }
}

fn windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || path.starts_with("\\\\")
}

/// Mutable access serializes requests. Any timeout/malformed response closes the channel.
pub struct HostClient {
    child: Child,
    input: Compat<ChildStdin>,
    output: Compat<ChildStdout>,
    fence: Option<ManagedFence>,
    pub canonical_root: String,
    pub account_id: String,
    pub root_identity: String,
    alive: bool,
    capabilities: Vec<String>,
}

impl HostClient {
    pub async fn connect(profile: &SshProfile) -> Result<Self, ConnectionError> {
        let mut command = profile.ssh_command(true)?;
        command
            .arg("--")
            .arg(&profile.target)
            .arg(profile.companion_command()?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let child = command
            .spawn()
            .map_err(|_| ConnectionError::SshUnavailable)?;
        Self::open(child, &profile.remote_root).await
    }

    /// The companion CLI joins the same private account service as SSH control.
    pub async fn connect_local(root: &str) -> Result<Self, ConnectionError> {
        let executable =
            std::env::current_exe().map_err(|_| ConnectionError::CompanionUnavailable)?;
        let mut command = Command::new(executable);
        crate::session::without_terminal_binding(&mut command);
        command.env_remove("VIBE_MCP_SERVERS");
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| ConnectionError::CompanionUnavailable)?;
        Self::open(child, root).await
    }

    async fn open(mut child: Child, root: &str) -> Result<Self, ConnectionError> {
        let input = child
            .stdin
            .take()
            .ok_or(ConnectionError::ConnectionLost)?
            .compat_write();
        let output = child
            .stdout
            .take()
            .ok_or(ConnectionError::ConnectionLost)?
            .compat();
        let mut client = Self {
            child,
            input,
            output,
            fence: None,
            canonical_root: String::new(),
            account_id: String::new(),
            root_identity: String::new(),
            alive: true,
            capabilities: Vec::new(),
        };
        let initialized = client
            .request(managed_request::Operation::Initialize(ManagedInitialize {
                protocol_major: PROTOCOL_MAJOR,
            }))
            .await?;
        let managed_response::Result::Initialized(initialized) = initialized else {
            return Err(ConnectionError::IncompatibleVersion);
        };
        let Some(fence) = initialized.fence else {
            return Err(ConnectionError::IncompatibleVersion);
        };
        if initialized.protocol_major != PROTOCOL_MAJOR
            || !valid_fence(&fence, false)
            || !text(&initialized.account_id, 256)
        {
            return Err(ConnectionError::IncompatibleVersion);
        }
        let opened = client
            .request(managed_request::Operation::ProjectOpen(ProjectOpen {
                fence: Some(fence.clone()),
                root: root.into(),
            }))
            .await?;
        let managed_response::Result::ProjectOpened(opened) = opened else {
            return Err(ConnectionError::CompanionUnavailable);
        };
        let Some(project_fence) = opened.fence else {
            return Err(ConnectionError::StaleAttachment);
        };
        if !valid_fence(&project_fence, true)
            || project_fence.service_id != fence.service_id
            || project_fence.service_boot_id != fence.service_boot_id
            || project_fence.connection_id != fence.connection_id
            || !text(&opened.canonical_root, 4096)
            || opened.root_identity.len() != 64
            || !opened.root_identity.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(ConnectionError::StaleAttachment);
        }
        client.fence = Some(project_fence);
        client.canonical_root = opened.canonical_root;
        client.account_id = initialized.account_id;
        client.root_identity = opened.root_identity;
        client.capabilities = initialized.capabilities;
        Ok(client)
    }

    pub fn fence(&self) -> Option<&ManagedFence> {
        self.fence.as_ref()
    }

    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    fn terminal_scope(&self, fence: &Option<ManagedFence>) -> Result<(), ConnectionError> {
        if !self.capabilities.iter().any(|c| c == "managed_agent") {
            return Err(ConnectionError::FeatureUnavailable);
        }
        if !self.alive || fence.is_none() || fence != &self.fence {
            return Err(ConnectionError::StaleAttachment);
        }
        Ok(())
    }

    /// Domain failure metadata is returned as `error`; lost writes retain the original intent.
    pub async fn project_tasks(
        &mut self,
        command: &crate::companion::TaskCommand,
        query_generation: u64,
    ) -> Result<serde_json::Value, ConnectionError> {
        if !self.capabilities.iter().any(|c| c == "project_tasks") {
            return Err(ConnectionError::FeatureUnavailable);
        }
        if !self.alive || self.fence.as_ref().is_none_or(|f| f.project_id.is_empty()) {
            return Err(ConnectionError::StaleAttachment);
        }
        let command_json =
            serde_json::to_vec(command).map_err(|_| ConnectionError::InvalidInput)?;
        if command_json.len() > 64 * 1024 {
            return Err(ConnectionError::CapacityExceeded);
        }
        let result = self
            .request(managed_request::Operation::ProjectTasks(
                ProjectTasksRequest {
                    fence: self.fence.clone(),
                    query_generation,
                    command_json,
                },
            ))
            .await?;
        if let managed_response::Result::ProjectTasks(result) = result {
            if result.fence == self.fence && result.query_generation == query_generation {
                if let Some(value) = crate::companion::decode_task_result(&result.result_json) {
                    let panel_matches = !matches!(command, crate::companion::TaskCommand::Panel(_))
                        || value.get("error").is_some()
                        || value["value"]["project"].as_str()
                            == self.fence.as_ref().map(|f| f.project_id.as_str());
                    if panel_matches {
                        return Ok(value);
                    }
                }
            }
        }
        self.close();
        Err(ConnectionError::StaleAttachment)
    }

    pub async fn terminal_launch(
        &mut self,
        request: TerminalLaunch,
    ) -> Result<TerminalState, ConnectionError> {
        if request.agent_program.is_some()
            && !self.capabilities.iter().any(|cap| cap == "project_mcp")
        {
            return Err(ConnectionError::FeatureUnavailable);
        }
        self.terminal_scope(&request.fence)?;
        let session = request.session_id.clone();
        let result = self
            .request(managed_request::Operation::TerminalLaunch(request))
            .await?;
        if let managed_response::Result::TerminalState(state) = result {
            if self.valid_terminal(&state) && state.session_id == session {
                return Ok(state);
            }
        }
        self.close();
        Err(ConnectionError::StaleAttachment)
    }

    pub async fn terminal_control(
        &mut self,
        request: TerminalControl,
    ) -> Result<TerminalState, ConnectionError> {
        self.terminal_scope(&request.fence)?;
        let session = request.session_id.clone();
        let run = request.run_id.clone();
        let result = self
            .request(managed_request::Operation::TerminalControl(request))
            .await?;
        if let managed_response::Result::TerminalState(state) = result {
            if self.valid_terminal(&state) && state.session_id == session && state.run_id == run {
                return Ok(state);
            }
        }
        self.close();
        Err(ConnectionError::StaleAttachment)
    }

    fn valid_terminal(&self, state: &TerminalState) -> bool {
        remote_protocol::managed::valid_terminal_state(state) && state.fence == self.fence
    }

    /// Disconnect stops this connection's owned Agent runs and revokes their MCP bindings.
    pub fn disconnect(&mut self) {
        self.close();
    }

    async fn request(
        &mut self,
        operation: managed_request::Operation,
    ) -> Result<managed_response::Result, ConnectionError> {
        if !self.alive {
            return Err(ConnectionError::ConnectionLost);
        }
        let request_id = Uuid::new_v4().to_string();
        let message = ClientMessage {
            request_id: request_id.clone(),
            message: Some(client_message::Message::Managed(ManagedRequest {
                operation: Some(operation),
            })),
        };
        let response = tokio::time::timeout(Duration::from_secs(10), async {
            write_message_with_limit(&mut self.input, &message, MAX_MANAGED_MESSAGE_SIZE).await?;
            read_message_with_limit::<ServerMessage>(&mut self.output, MAX_MANAGED_MESSAGE_SIZE)
                .await
        })
        .await;
        let Ok(Ok(response)) = response else {
            self.close();
            return Err(ConnectionError::ConnectionLost);
        };
        if response.request_id != request_id {
            self.close();
            return Err(ConnectionError::StaleAttachment);
        }
        let Some(server_message::Message::Managed(ManagedResponse {
            result: Some(result),
        })) = response.message
        else {
            self.close();
            return Err(ConnectionError::IncompatibleVersion);
        };
        match result {
            managed_response::Result::Error(error) => {
                Err(match ManagedErrorCode::try_from(error.code) {
                    Ok(ManagedErrorCode::ManagedIncompatibleVersion) => {
                        ConnectionError::IncompatibleVersion
                    }
                    Ok(ManagedErrorCode::ManagedStaleAttachment) => {
                        ConnectionError::StaleAttachment
                    }
                    Ok(ManagedErrorCode::ManagedFeatureUnavailable) => {
                        ConnectionError::FeatureUnavailable
                    }
                    Ok(ManagedErrorCode::ManagedInvalidInput) => ConnectionError::InvalidInput,
                    Ok(ManagedErrorCode::ManagedConflict) => ConnectionError::Conflict,
                    Ok(ManagedErrorCode::ManagedCapacityExceeded) => {
                        ConnectionError::CapacityExceeded
                    }
                    _ => ConnectionError::CompanionUnavailable,
                })
            }
            result => Ok(result),
        }
    }

    fn close(&mut self) {
        self.alive = false;
        self.fence = None;
        let _ = self.child.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_profile_rejects_options_and_encodes_remote_shell_separately() {
        let mut profile = SshProfile {
            id: Uuid::new_v4(),
            display_name: "Research".into(),
            target: "research-node".into(),
            user: Some("researcher".into()),
            port: Some(2222),
            identity_file: None,
            config_file: None,
            jump_alias: Some("jump-alias".into()),
            remote_root: "/srv/project with spaces/多语言".into(),
            companion_path: Some("/opt/it's $(danger)/companion".into()),
            remote_shell: RemoteShell::Posix,
        };
        assert!(profile.validate().is_ok());
        assert_eq!(
            profile.companion_command().unwrap(),
            "exec '/opt/it'\\''s $(danger)/companion'"
        );
        let command = profile.ssh_command(true).unwrap();
        let args: Vec<_> = command
            .as_std()
            .get_args()
            .filter_map(|arg| arg.to_str())
            .collect();
        assert!(args.contains(&"StrictHostKeyChecking=yes"));
        assert!(args.contains(&"BatchMode=yes"));
        assert!(!args.iter().any(|arg| arg.starts_with("ControlPath=")));
        for bad in [
            "-oProxyCommand=bad",
            "user@host",
            "host;touch",
            "host\nother",
            "host other",
            "$(bad)",
        ] {
            profile.target = bad.into();
            assert_eq!(profile.validate(), Err(ConnectionError::InvalidProfile));
        }
        profile.target = "research-node".into();
        profile.remote_shell = RemoteShell::PowerShell;
        profile.remote_root = "C:\\project with spaces".into();
        profile.companion_path = Some("C:\\it's $(danger)\\companion.exe".into());
        assert!(profile.validate().is_ok());
        let command = profile.companion_command().unwrap();
        let encoded = command.split_whitespace().last().unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(
            String::from_utf16(&words).unwrap(),
            "& 'C:\\it''s $(danger)\\companion.exe'"
        );
    }
}
