//! Separate, explicitly started local enrollment controller. No operator or PTY API is exposed.
use super::{receive, send, Broker, RunningBroker};
use crate::{
    coordinator_unavailable, domain, invalid_input, invalid_state, remote::NegotiationFrame,
    storage::RemotePrincipal, DomainError,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::watch,
};
use uuid::Uuid;

const DEADLINE: Duration = Duration::from_secs(5);
const PRESENCE: Duration = Duration::from_secs(30);

/// Sensitive frames deliberately have no Debug implementation or durable retry representation.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Frame {
    Negotiation(NegotiationFrame),
    Authentication(AuthenticationFrame),
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthenticationFrame {
    Enroll {
        invitation: String,
        name: String,
    },
    EnrollResult {
        device_id: Uuid,
        credential: String,
        generation: u64,
        space_ids: Vec<Uuid>,
    },
    Authenticate {
        credential: String,
    },
    Authenticated {
        device_id: Uuid,
        generation: u64,
        connection_epoch: Uuid,
        space_ids: Vec<Uuid>,
    },
    Heartbeat {
        connection_epoch: Uuid,
        space_id: Uuid,
    },
    HeartbeatResult {
        connection_epoch: Uuid,
    },
    Goodbye {
        connection_epoch: Uuid,
    },
    Error {
        error: DomainError,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    endpoint: String,
    pid: u32,
    protocol_major: u16,
    coordinator_id: Uuid,
    nonce: Uuid,
}

/// Owns only its listener and descriptor. Dropping it fences outstanding sessions under the broker mutex.
pub struct RunningController {
    broker: Broker,
    descriptor: PathBuf,
    nonce: Uuid,
    shutdown: watch::Sender<bool>,
    listener: tokio::task::JoinHandle<()>,
}
impl RunningController {
    pub fn start(server: &RunningBroker, descriptor: &Path) -> Result<Self> {
        let broker = server.broker.clone();
        let mut state = broker.store()?;
        ensure!(
            !state.remote_active,
            invalid_state("Remote controller is already running")
        );
        ensure!(
            !broker.shared.stopped.load(Ordering::Acquire),
            coordinator_unavailable("Coordinator application stopped")
        );
        let coordinator_id = state.store.coordinator_id()?;
        #[cfg(target_os = "macos")]
        let endpoint = server
            .directory
            .join(format!("controller-{}.sock", Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        #[cfg(windows)]
        let endpoint = format!(r"\\.\pipe\warp-agent-controller-{}", Uuid::new_v4());
        let listener = {
            let _entered = broker.runtime.enter();
            #[cfg(target_os = "macos")]
            let listener = tokio::net::UnixListener::bind(&endpoint)?;
            #[cfg(windows)]
            let listener = super::windows_pipe::create(&endpoint, true)?;
            listener
        };
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600))?;
        }
        let nonce = Uuid::new_v4();
        let ready = (|| -> Result<()> {
            state.store.invalidate_remote_sessions()?;
            publish(
                descriptor,
                &Descriptor {
                    endpoint: endpoint.clone(),
                    pid: std::process::id(),
                    protocol_major: 2,
                    coordinator_id,
                    nonce,
                },
            )
        })();
        if let Err(error) = ready {
            #[cfg(target_os = "macos")]
            let _ = std::fs::remove_file(&endpoint);
            return Err(error);
        }
        state.remote_active = true;
        state.remote_owner = Some(nonce);
        drop(state);
        let (shutdown, receiver) = watch::channel(false);
        let task_broker = broker.clone();
        let task = broker.runtime.spawn(async move {
            let mut shutdown = receiver;
            #[cfg(windows)]
            let mut listener = listener;
            let mut sessions = tokio::task::JoinSet::new();
            loop {
                let connection = tokio::select! {
                    _ = shutdown.changed() => break,
                    _ = sessions.join_next(), if !sessions.is_empty() => continue,
                    connection = async {
                        #[cfg(target_os = "macos")]
                        { listener.accept().await.map(|(stream, _)| stream) }
                        #[cfg(windows)]
                        {
                            listener.connect().await?;
                            let next = super::windows_pipe::create(&endpoint, false).map_err(std::io::Error::other)?;
                            Ok::<_, std::io::Error>(std::mem::replace(&mut listener, next))
                        }
                    } => connection,
                };
                let Ok(stream) = connection else { break };
                if sessions.len() >= 32 { continue; }
                let broker = task_broker.clone();
                let mut closed = shutdown.clone();
                sessions.spawn(async move {
                    tokio::select! {
                        _ = closed.changed() => {},
                        _ = session(stream, broker, coordinator_id, nonce) => {},
                    }
                });
            }
            if let Ok(mut state) = task_broker.store() {
                if state.remote_owner == Some(nonce) {
                    state.remote_active = false;
                    state.remote_owner = None;
                    let _ = state.store.invalidate_remote_sessions();
                }
            }
            #[cfg(target_os = "macos")]
            let _ = std::fs::remove_file(&endpoint);
            // JoinSet drop aborts owned connections; no detached endpoint survives disablement.
        });
        Ok(Self {
            broker,
            descriptor: descriptor.into(),
            nonce,
            shutdown,
            listener: task,
        })
    }
}
impl Drop for RunningController {
    fn drop(&mut self) {
        if let Ok(mut state) = self.broker.store() {
            if state.remote_owner == Some(self.nonce) {
                state.remote_active = false;
                state.remote_owner = None;
                let _ = state.store.invalidate_remote_sessions();
            }
        }
        let _ = self.shutdown.send(true);
        self.listener.abort();
        if read_descriptor(&self.descriptor).is_ok_and(|descriptor| descriptor.nonce == self.nonce)
        {
            let _ = std::fs::remove_file(&self.descriptor);
        }
    }
}

fn active(broker: &Broker, owner: Uuid) -> Result<std::sync::MutexGuard<'_, super::State>> {
    let state = broker.store()?;
    ensure!(
        state.remote_active
            && state.remote_owner == Some(owner)
            && !broker.shared.stopped.load(Ordering::Acquire),
        coordinator_unavailable("Remote participation is disabled")
    );
    Ok(state)
}
fn parse_uuid(value: &serde_json::Value) -> Result<Uuid> {
    value
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| invalid_state("Enrollment identity is unavailable"))
}
fn grants(broker: &Broker, principal: &RemotePrincipal, owner: Uuid) -> Result<Vec<Uuid>> {
    active(broker, owner)?.store.remote_grants(principal)
}
async fn session<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    broker: Broker,
    coordinator: Uuid,
    owner: Uuid,
) -> Result<()> {
    let result = session_inner(&mut stream, &broker, coordinator, owner).await;
    if let Err(error) = &result {
        let safe = error
            .downcast_ref::<DomainError>()
            .cloned()
            .unwrap_or_else(|| {
                domain(
                    "coordinator_unavailable",
                    "Controller session is unavailable",
                    true,
                    None,
                )
                .downcast::<DomainError>()
                .unwrap()
            });
        let _ = send(
            &mut stream,
            &Frame::Authentication(AuthenticationFrame::Error { error: safe }),
            Instant::now() + DEADLINE,
        )
        .await;
    }
    result
}
async fn session_inner<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    broker: &Broker,
    coordinator: Uuid,
    owner: Uuid,
) -> Result<()> {
    active(broker, owner)?;
    let hello: NegotiationFrame = receive(stream, Instant::now() + DEADLINE).await?;
    let result = hello.negotiate(coordinator)?;
    let epoch = match &result {
        NegotiationFrame::HelloResult {
            connection_epoch, ..
        } => *connection_epoch,
        _ => unreachable!(),
    };
    send(stream, &result, Instant::now() + DEADLINE).await?;
    let mut principal = None;
    loop {
        let frame: AuthenticationFrame = receive(
            stream,
            if principal.is_some() {
                Instant::now() + PRESENCE
            } else {
                Instant::now() + DEADLINE
            },
        )
        .await?;
        let response = match frame {
            AuthenticationFrame::Enroll { invitation, name } if principal.is_none() => {
                let enrolled = active(broker, owner)?
                    .store
                    .enroll_remote(&invitation, &name)
                    .map_err(crate::classify_storage)?;
                let spaces = enrolled["space_ids"]
                    .as_array()
                    .ok_or_else(|| invalid_state("Enrollment grants unavailable"))?
                    .iter()
                    .map(parse_uuid)
                    .collect::<Result<Vec<_>>>()?;
                AuthenticationFrame::EnrollResult {
                    device_id: parse_uuid(&enrolled["device_id"])?,
                    credential: enrolled["credential"]
                        .as_str()
                        .ok_or_else(|| invalid_state("Enrollment delivery unavailable"))?
                        .into(),
                    generation: 1,
                    space_ids: spaces,
                }
            }
            AuthenticationFrame::Authenticate { credential } if principal.is_none() => {
                let authenticated = active(broker, owner)?
                    .store
                    .authenticate_remote(&credential)
                    .map_err(crate::classify_storage)?;
                let spaces = grants(broker, &authenticated, owner)?;
                let response = AuthenticationFrame::Authenticated {
                    device_id: Uuid::parse_str(&authenticated.device)?,
                    generation: authenticated.generation,
                    connection_epoch: epoch,
                    space_ids: spaces,
                };
                principal = Some(authenticated);
                response
            }
            AuthenticationFrame::Heartbeat {
                connection_epoch,
                space_id,
            } => {
                ensure!(
                    connection_epoch == epoch,
                    invalid_input("Connection epoch does not match")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before using this channel"))?;
                active(broker, owner)?.store.authorize_remote(
                    principal,
                    &space_id.to_string(),
                    false,
                )?;
                AuthenticationFrame::HeartbeatResult {
                    connection_epoch: epoch,
                }
            }
            AuthenticationFrame::Goodbye { connection_epoch } => {
                ensure!(
                    principal.is_some() && connection_epoch == epoch,
                    crate::unauthorized("Authenticated connection required")
                );
                return Ok(());
            }
            _ => return Err(invalid_input("Unexpected controller frame")),
        };
        send(stream, &response, Instant::now() + DEADLINE).await?;
    }
}

fn read_descriptor(path: &Path) -> Result<Descriptor> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    let file = options
        .open(path)
        .map_err(|_| coordinator_unavailable("Coordinator application is unavailable"))?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= 4096,
        invalid_input("Invalid controller descriptor")
    );
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0,
            invalid_input("Controller descriptor is not private")
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            invalid_input("Controller descriptor cannot be a reparse point")
        );
    }
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4096,
        invalid_input("Invalid controller descriptor")
    );
    let descriptor: Descriptor = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_input("Invalid controller descriptor"))?;
    ensure!(
        descriptor.protocol_major == 2
            && descriptor.pid != 0
            && descriptor.pid <= i32::MAX as u32
            && descriptor.endpoint.len() <= 256,
        invalid_input("Invalid controller descriptor")
    );
    #[cfg(target_os = "macos")]
    ensure!(
        Path::new(&descriptor.endpoint).is_absolute() && descriptor.endpoint.ends_with(".sock"),
        invalid_input("Invalid controller endpoint")
    );
    #[cfg(windows)]
    ensure!(
        descriptor
            .endpoint
            .starts_with(r"\\.\pipe\warp-agent-controller-"),
        invalid_input("Invalid controller endpoint")
    );
    Ok(descriptor)
}
fn publish(path: &Path, descriptor: &Descriptor) -> Result<()> {
    if path.exists() {
        let previous = read_descriptor(path)?;
        ensure!(
            !process_alive(previous.pid),
            invalid_state("Another controller owns this descriptor")
        );
        ensure!(
            read_descriptor(path)?.nonce == previous.nonce,
            invalid_state("Controller ownership changed")
        );
        std::fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    let result = serde_json::to_writer(&file, descriptor)
        .map_err(anyhow::Error::from)
        .and_then(|()| file.sync_all().map_err(anyhow::Error::from));
    if result.is_err() {
        let _ = std::fs::remove_file(path);
    }
    result
}
fn process_alive(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe {
            libc::kill(pid as i32, 0) == 0
                || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
    }
    #[cfg(windows)]
    {
        use windows::{
            core::Owned,
            Win32::System::Threading::{
                GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        };
        let process = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            Ok(process) => process,
            Err(error) => return error.code() != windows::core::HRESULT::from_win32(87),
        };
        let process = unsafe { Owned::new(process) };
        let mut status = 0;
        unsafe { GetExitCodeProcess(*process, &mut status).is_ok() && status == 259 }
    }
}

pub fn descriptor_path() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let root = PathBuf::from(
        std::env::var_os("HOME")
            .ok_or_else(|| coordinator_unavailable("Local app data directory unavailable"))?,
    )
    .join("Library/Application Support");
    #[cfg(windows)]
    let root = PathBuf::from(
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| coordinator_unavailable("Local app data directory unavailable"))?,
    );
    let profile = std::env::var("WARP_DATA_PROFILE").unwrap_or_else(|_| "daily".into());
    ensure!(
        !profile.is_empty()
            && profile.len() <= 64
            && profile
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
        invalid_input("Invalid application profile")
    );
    Ok(root
        .join("Warpai/remote")
        .join(profile)
        .join("controller.json"))
}

/// Framed bytes only; application authentication remains on the controller, never in argv.
pub async fn gateway<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    descriptor: &Path,
    mut input: R,
    mut output: W,
) -> Result<()> {
    let descriptor = read_descriptor(descriptor)?;
    ensure!(
        process_alive(descriptor.pid),
        coordinator_unavailable("Coordinator application is unavailable")
    );
    #[cfg(target_os = "macos")]
    let stream = tokio::time::timeout(
        DEADLINE,
        tokio::net::UnixStream::connect(&descriptor.endpoint),
    )
    .await
    .map_err(|_| coordinator_unavailable("Controller connection timed out"))?
    .map_err(|_| coordinator_unavailable("Coordinator application is unavailable"))?;
    #[cfg(windows)]
    let stream = tokio::net::windows::named_pipe::ClientOptions::new()
        .open(&descriptor.endpoint)
        .map_err(|_| coordinator_unavailable("Coordinator application is unavailable"))?;
    let (mut reader, mut writer) = tokio::io::split(stream);
    let upstream_closed = {
        tokio::select! {
            result = tokio::io::copy(&mut input, &mut writer) => { result?; true },
            result = tokio::io::copy(&mut reader, &mut output) => { result?; false },
        }
    };
    if upstream_closed {
        writer.shutdown().await?;
        tokio::time::timeout(DEADLINE, tokio::io::copy(&mut reader, &mut output))
            .await
            .map_err(|_| coordinator_unavailable("Controller close timed out"))??;
    }
    output.shutdown().await?;
    Ok(())
}

/// Emit a framed, redacted failure instead of mixing diagnostics into protocol stdout.
pub async fn gateway_stdio() -> Result<()> {
    let result = match descriptor_path() {
        Ok(path) => gateway(&path, tokio::io::stdin(), tokio::io::stdout()).await,
        Err(error) => Err(error),
    };
    if result.is_err() {
        let error = coordinator_unavailable("Coordinator application is unavailable")
            .downcast::<DomainError>()
            .unwrap();
        send(
            &mut tokio::io::stdout(),
            &AuthenticationFrame::Error { error },
            Instant::now() + DEADLINE,
        )
        .await?;
        return Err(coordinator_unavailable(
            "Coordinator application is unavailable",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ControllerOperation;

    #[test]
    fn gateway_enrollment_revoke_and_disable_use_real_local_ipc() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("bus.sqlite");
        let descriptor = directory.path().join("controller.json");
        let server = RunningBroker::start(&database).unwrap();
        let controller = RunningController::start(&server, &descriptor).unwrap();
        assert!(RunningController::start(&server, &descriptor).is_err());
        let coordinator = read_descriptor(&descriptor).unwrap().coordinator_id;
        let space = server
            .broker
            .control(
                "/fixture",
                &ControllerOperation::SpaceCreate {
                    name: "Gateway fixture".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap()["space_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let invitation = server
            .broker
            .control(
                "/fixture",
                &ControllerOperation::InvitationCreate {
                    space_ids: vec![space.clone()],
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap()["invitation"]
            .as_str()
            .unwrap()
            .to_owned();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let (mut client, relay) = tokio::io::duplex(4096);
            let path = descriptor.clone();
            let task = tokio::spawn(async move {
                let (input, output) = tokio::io::split(relay);
                gateway(&path, input, output).await
            });
            let hello = NegotiationFrame::Hello {
                protocol_major: 2,
                protocol_minor: 0,
                max_frame_bytes: crate::MAX_FRAME as u32,
                features: [
                    "task_control",
                    "dependencies",
                    "threads",
                    "reservations",
                    "evidence_refs",
                    "event_resume",
                ]
                .iter()
                .map(|value| (*value).into())
                .collect(),
            };
            send(&mut client, &hello, Instant::now() + DEADLINE)
                .await
                .unwrap();
            let NegotiationFrame::HelloResult {
                connection_epoch,
                coordinator_id,
                ..
            } = receive(&mut client, Instant::now() + DEADLINE)
                .await
                .unwrap()
            else {
                panic!("Expected hello result")
            };
            assert_eq!(coordinator_id, coordinator);
            send(
                &mut client,
                &AuthenticationFrame::Enroll {
                    invitation,
                    name: "Participant".into(),
                },
                Instant::now() + DEADLINE,
            )
            .await
            .unwrap();
            let AuthenticationFrame::EnrollResult {
                credential,
                device_id,
                ..
            } = receive(&mut client, Instant::now() + DEADLINE)
                .await
                .unwrap()
            else {
                panic!("Expected enrollment")
            };
            send(
                &mut client,
                &AuthenticationFrame::Authenticate { credential },
                Instant::now() + DEADLINE,
            )
            .await
            .unwrap();
            let AuthenticationFrame::Authenticated { .. } =
                receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
            else {
                panic!("Expected authentication")
            };
            send(
                &mut client,
                &AuthenticationFrame::Heartbeat {
                    connection_epoch,
                    space_id: Uuid::parse_str(&space).unwrap(),
                },
                Instant::now() + DEADLINE,
            )
            .await
            .unwrap();
            let AuthenticationFrame::HeartbeatResult { .. } =
                receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
            else {
                panic!("Expected heartbeat")
            };
            server
                .broker
                .control(
                    "/fixture",
                    &ControllerOperation::DeviceRevoke {
                        device_id: device_id.to_string(),
                        request_id: Uuid::new_v4().to_string(),
                    },
                )
                .unwrap();
            send(
                &mut client,
                &AuthenticationFrame::Heartbeat {
                    connection_epoch,
                    space_id: Uuid::parse_str(&space).unwrap(),
                },
                Instant::now() + DEADLINE,
            )
            .await
            .unwrap();
            let AuthenticationFrame::Error { error } =
                receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
            else {
                panic!("Expected revocation")
            };
            assert_eq!(error.code, "device_revoked");
            let _ = task.await.unwrap();
        });
        drop(controller);
        assert!(!descriptor.exists());
        runtime.block_on(async {
            let (input, _) = tokio::io::duplex(128);
            assert!(gateway(&descriptor, input, tokio::io::sink())
                .await
                .is_err());
        });
        let controller = RunningController::start(&server, &descriptor).unwrap();
        assert_eq!(
            read_descriptor(&descriptor).unwrap().coordinator_id,
            coordinator
        );
        drop(controller);
        drop(server);
        let restarted = RunningBroker::start(&database).unwrap();
        let _controller = RunningController::start(&restarted, &descriptor).unwrap();
        assert_eq!(
            read_descriptor(&descriptor).unwrap().coordinator_id,
            coordinator
        );
    }
}
