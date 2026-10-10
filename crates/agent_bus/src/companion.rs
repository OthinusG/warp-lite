//! Private project communication and Agent IO, independent of desktop/UI and enrollment.
use std::{
    fs::File,
    path::{Path, PathBuf},
};

use remote_protocol::{
    managed::PROTOCOL_MAJOR,
    proto::*,
    protocol::{
        read_message_with_limit, write_message_with_limit, ProtocolError, MAX_MANAGED_MESSAGE_SIZE,
    },
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use uuid::Uuid;
#[cfg(test)]
#[path = "companion_files_tests.rs"]
mod file_tests;
#[path = "companion_files.rs"]
pub(crate) mod files;
#[path = "companion_identity.rs"]
pub(crate) mod identity;
#[path = "companion_service.rs"]
mod service;
#[path = "companion_tasks.rs"]
mod tasks;
#[path = "companion_terminals.rs"]
mod terminals;
#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
#[path = "companion_guests.rs"]
pub(crate) mod guests;
pub(crate) use tasks::decode_result as decode_task_result;
pub use tasks::Command as TaskCommand;

struct Project {
    root: PathBuf,
    handle: File,
}

/// One fenced attachment to account-owned project and terminal state.
pub struct Companion {
    identity: identity::Identity,
    fence: ManagedFence,
    initialized: bool,
    project: Option<Project>,
    terminals: std::sync::Arc<terminals::Terminals>,
    tasks: std::sync::Arc<tasks::Projects>,
    files: files::Files,
    peer_pid: Option<u32>,
}

impl Companion {
    pub fn new(data_directory: &Path) -> std::io::Result<Self> {
        let identity = identity::Identity::open(data_directory)?;
        Ok(Self {
            fence: ManagedFence {
                service_id: identity.service_id.clone(),
                service_boot_id: Uuid::new_v4().to_string(),
                connection_id: Uuid::new_v4().to_string(),
                project_id: String::new(),
            },
            initialized: false,
            project: None,
            terminals: std::sync::Arc::new(terminals::Terminals::default()),
            tasks: std::sync::Arc::new(tasks::Projects::new(data_directory)),
            files: files::Files::new(data_directory),
            peer_pid: None,
            identity,
        })
    }
}

impl Companion {
    /// Only the managed oneof reaches this handler; legacy commands never execute.
    pub fn handle(&mut self, request: ClientMessage) -> ServerMessage {
        let valid_request_id = Uuid::parse_str(&request.request_id).is_ok_and(|id| !id.is_nil());
        let result = if !valid_request_id {
            Err(ManagedErrorCode::ManagedInvalidInput)
        } else if let Some(client_message::Message::Managed(request)) = request.message {
            self.dispatch(request)
        } else {
            Err(ManagedErrorCode::ManagedFeatureUnavailable)
        };
        ServerMessage {
            // Never echo unvalidated peer strings, including oversized request identifiers.
            request_id: if valid_request_id {
                request.request_id
            } else {
                String::new()
            },
            message: Some(server_message::Message::Managed(Box::new(
                ManagedResponse {
                    result: Some(result.unwrap_or_else(|code| {
                        managed_response::Result::Error(ManagedError { code: code.into() })
                    })),
                },
            ))),
        }
    }

    fn dispatch(
        &mut self,
        request: ManagedRequest,
    ) -> Result<managed_response::Result, ManagedErrorCode> {
        match request.operation {
            Some(managed_request::Operation::Initialize(request)) => {
                if self.initialized {
                    return Err(ManagedErrorCode::ManagedInvalidInput);
                }
                if request.protocol_major != PROTOCOL_MAJOR {
                    return Err(ManagedErrorCode::ManagedIncompatibleVersion);
                }
                self.initialized = true;
                Ok(managed_response::Result::Initialized(ManagedInitialized {
                    protocol_major: PROTOCOL_MAJOR,
                    fence: Some(self.fence.clone()),
                    os: std::env::consts::OS.into(),
                    architecture: std::env::consts::ARCH.into(),
                    capabilities: vec![
                        "project_open".into(),
                        "managed_agent".into(),
                        #[cfg(feature = "wsl_companion")]
                        "managed_launch_cwd".into(),
                        "project_tasks".into(),
                        "project_mcp".into(),
                        "project_files".into(),
                        #[cfg(feature = "wsl_companion")]
                        "project_file_chunks".into(),
                        "project_git_review".into(),
                        "worktree_collaboration".into(),
                        "worktree_orchestration".into(),
                    ],
                    account_id: self.identity.account_id.clone(),
                }))
            }
            Some(managed_request::Operation::ProjectOpen(request)) => {
                self.check_fence(request.fence.as_ref(), false)?;
                if request.root.is_empty()
                    || request.root.len() > 4096
                    || request.root.chars().any(char::is_control)
                    || !Path::new(&request.root).is_absolute()
                {
                    return Err(ManagedErrorCode::ManagedInvalidInput);
                }
                let root = Path::new(&request.root)
                    .canonicalize()
                    .map_err(path_error)?;
                if !root.is_dir() || root.to_str().is_none() {
                    return Err(ManagedErrorCode::ManagedInvalidInput);
                }
                let handle = open_root(&root).map_err(path_error)?;
                let (id, root_identity) = self.identity.project(&handle).map_err(path_error)?;
                let canonical_root = root.to_str().unwrap().to_owned();
                self.terminals.disconnect(&self.fence.connection_id);
                self.files.clear();
                self.project = Some(Project { root, handle });
                self.fence.project_id = id;
                Ok(managed_response::Result::ProjectOpened(ProjectOpened {
                    fence: Some(self.fence.clone()),
                    canonical_root,
                    root_identity,
                }))
            }
            Some(managed_request::Operation::TerminalLaunch(request)) => {
                self.check_project(request.fence.as_ref())?;
                self.terminals
                    .launch(
                        request,
                        &self.fence,
                        &self.project.as_ref().unwrap().root,
                        &self.tasks,
                    )
                    .map(managed_response::Result::TerminalState)
            }
            Some(managed_request::Operation::TerminalControl(request)) => {
                self.check_project(request.fence.as_ref())?;
                self.terminals
                    .control(request, &self.fence)
                    .map(managed_response::Result::TerminalState)
            }
            Some(managed_request::Operation::ProjectTasks(request)) => {
                self.check_project(request.fence.as_ref())?;
                self.tasks
                    .execute(request, &self.fence, &self.project.as_ref().unwrap().root)
                    .map(managed_response::Result::ProjectTasks)
            }
            Some(managed_request::Operation::ProjectFiles(request)) => {
                self.check_project(request.fence.as_ref())?;
                self.files
                    .execute(request, self.project.as_ref().unwrap())
                    .map(|result| managed_response::Result::ProjectFiles(Box::new(result)))
            }
            Some(managed_request::Operation::GuestMcpBind(request)) => {
                self.check_project(request.fence.as_ref())?;
                #[cfg(all(feature = "wsl_companion", target_os = "linux"))]
                {
                    self.tasks.guests.bind(&self.tasks, &self.fence, &self.project.as_ref().unwrap().root,
                        self.peer_pid, request.agent_pid, &request.program)
                        .map(managed_response::Result::GuestMcpBound)
                }
                #[cfg(not(all(feature = "wsl_companion", target_os = "linux")))]
                { Err(ManagedErrorCode::ManagedFeatureUnavailable) }
            }
            None => Err(ManagedErrorCode::ManagedInvalidInput),
        }
    }

    fn check_project(&self, fence: Option<&ManagedFence>) -> Result<(), ManagedErrorCode> {
        self.check_fence(fence, true)?;
        if !same_root(self.project.as_ref().unwrap()).map_err(path_error)? {
            return Err(ManagedErrorCode::ManagedStaleAttachment);
        }
        Ok(())
    }

    fn check_fence(
        &self,
        fence: Option<&ManagedFence>,
        project: bool,
    ) -> Result<(), ManagedErrorCode> {
        let Some(fence) = fence else {
            return Err(ManagedErrorCode::ManagedStaleAttachment);
        };
        if !self.initialized
            || fence.service_id != self.fence.service_id
            || fence.service_boot_id != self.fence.service_boot_id
            || fence.connection_id != self.fence.connection_id
            || if project {
                self.project.is_none() || fence.project_id != self.fence.project_id
            } else {
                !fence.project_id.is_empty()
            }
        {
            return Err(ManagedErrorCode::ManagedStaleAttachment);
        }
        Ok(())
    }
}

fn path_error(error: std::io::Error) -> ManagedErrorCode {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        ManagedErrorCode::ManagedPermissionDenied
    } else if error.kind() == std::io::ErrorKind::NotFound {
        ManagedErrorCode::ManagedNotFound
    } else {
        ManagedErrorCode::ManagedUnavailable
    }
}

#[cfg(unix)]
pub(crate) fn open_root(root: &Path) -> std::io::Result<File> {
    File::open(root)
}

#[cfg(windows)]
pub(crate) fn open_root(root: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x02000000)
        .open(root)
}

#[cfg(unix)]
fn same_root(project: &Project) -> std::io::Result<bool> {
    root_matches(&project.handle, &project.root)
}

#[cfg(unix)]
pub(crate) fn root_matches(handle: &File, root: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let owned = handle.metadata()?;
    let current = root.metadata()?;
    Ok(current.is_dir() && owned.dev() == current.dev() && owned.ino() == current.ino())
}

#[cfg(windows)]
fn same_root(project: &Project) -> std::io::Result<bool> {
    root_matches(&project.handle, &project.root)
}

#[cfg(windows)]
pub(crate) fn root_matches(handle: &File, root: &Path) -> std::io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };
    fn identity(file: &File) -> std::io::Result<(u32, u32, u32)> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // Both handles are owned directory handles valid throughout this native call.
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
            .map_err(std::io::Error::other)?;
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }
    Ok(identity(handle)? == identity(&open_root(root)?)?)
}

/// Serial bounded reads provide backpressure and at most one response in flight.
pub async fn serve_stdio() -> Result<(), ProtocolError> {
    service::proxy_stdio().await
}

pub async fn serve_account_service() -> Result<(), ProtocolError> {
    service::serve().await
}

#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
pub async fn serve_guest_mcp(program: String) -> anyhow::Result<()> {
    guests::serve_mcp(program).await
}

fn data_directory() -> std::io::Result<PathBuf> {
    let directory = dirs::data_local_dir()
        .ok_or_else(|| std::io::Error::other("Companion data directory unavailable"))?
        .join("warpai")
        .join(if cfg!(feature = "wsl_companion") { "wsl-remote" } else { "remote" });
    #[cfg(all(feature = "wsl_companion", target_os = "linux"))]
    let directory = {
        let distribution = std::env::var("WSL_DISTRO_NAME")
            .map_err(|_| std::io::Error::other("WSL distribution identity unavailable"))?;
        directory.join(guests::distribution_namespace(&distribution)?)
    };
    Ok(directory)
}

#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
pub(crate) fn guest_preferences_path() -> std::io::Result<PathBuf> {
    Ok(data_directory()?.join("mcp-settings.json"))
}

async fn serve_channel<S>(
    stream: S,
    directory: &Path,
    boot: &str,
    terminals: std::sync::Arc<terminals::Terminals>,
    tasks: std::sync::Arc<tasks::Projects>,
    peer_pid: Option<u32>,
) -> Result<(), ProtocolError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = reader.compat();
    let mut writer = writer.compat_write();
    let mut companion = Companion::new(directory)?;
    companion.fence.service_boot_id = boot.into();
    companion.terminals = terminals;
    companion.tasks = tasks;
    companion.peer_pid = peer_pid;
    loop {
        let request =
            match read_message_with_limit::<ClientMessage>(&mut reader, MAX_MANAGED_MESSAGE_SIZE)
                .await
            {
                Ok(request) => request,
                Err(ProtocolError::UnexpectedEof) => return Ok(()),
                Err(error) => return Err(error),
            };
        // Native spawn/lease operations cannot block the async executor or Agent IO.
        let (owner, reply) = tokio::task::spawn_blocking(move || {
            let reply = companion.handle(request);
            (companion, reply)
        })
        .await
        .map_err(std::io::Error::other)?;
        companion = owner;
        write_message_with_limit(&mut writer, &reply, MAX_MANAGED_MESSAGE_SIZE).await?;
    }
}

impl Drop for Companion {
    fn drop(&mut self) {
        self.terminals.disconnect(&self.fence.connection_id);
    }
}

#[cfg(test)]
#[path = "companion_tests.rs"]
mod tests;
