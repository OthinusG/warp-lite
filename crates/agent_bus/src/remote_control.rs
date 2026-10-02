//! Explicitly started authenticated agent controller. No operator or PTY API is exposed.
use super::{receive, send, Broker, RunningBroker};
use crate::{
    coordinator_unavailable, domain, invalid_input, invalid_state,
    remote::NegotiationFrame,
    storage::{RemotePrincipal, RemoteWorkspace},
    Agent, DomainError, Operation,
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

pub(super) struct ActorPresence {
    principal: RemotePrincipal,
    epoch: Uuid,
    connection: Uuid,
    seen: Instant,
}
impl ActorPresence {
    pub(super) fn age_ms(&self) -> u64 {
        self.seen.elapsed().as_millis() as u64
    }
    pub(super) fn mutation_epoch(&self) -> String {
        self.epoch.to_string()
    }
    pub(super) fn executing(
        &self,
        store: &crate::storage::Store,
        actor: &str,
        run: Option<&str>,
    ) -> bool {
        run == Some(self.epoch.to_string().as_str()) && self.valid(store, actor)
    }
    pub(super) fn valid(&self, store: &crate::storage::Store, actor: &str) -> bool {
        self.seen.elapsed() < PRESENCE
            && store
                .resolve_remote_run(&self.principal, actor, &self.epoch.to_string(), false)
                .is_ok()
    }
}
struct PresenceGuard {
    broker: Broker,
    connection: Uuid,
}
impl Drop for PresenceGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.broker.shared.state.lock() {
            let disconnected: Vec<_> = state
                .remote_presence
                .iter()
                .filter(|(_, presence)| presence.connection == self.connection)
                .map(|(actor, presence)| (actor.clone(), presence.mutation_epoch()))
                .collect();
            for (actor, epoch) in disconnected {
                if state
                    .store
                    .observe_remote_disconnect(&actor, &epoch)
                    .is_ok()
                {
                    state.remote_presence.remove(&actor);
                } else if let Some(presence) = state.remote_presence.get_mut(&actor) {
                    presence.seen = Instant::now() - PRESENCE;
                }
            }
            self.broker.shared.changed.notify_all();
        }
    }
}

/// Sensitive frames deliberately have no Debug implementation or durable retry representation.
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
    ActorHeartbeat {
        connection_epoch: Uuid,
        actor_id: Uuid,
        mutation_epoch: Uuid,
    },
    ActorHeartbeatResult {
        connection_epoch: Uuid,
        actor_id: Uuid,
        mutation_epoch: Uuid,
    },
    ActorAnnounce {
        connection_epoch: Uuid,
        workspace_id: Uuid,
        space_id: Uuid,
        checkout_id: Uuid,
        native_session: Uuid,
        native_run: Uuid,
        program: String,
        name: String,
    },
    ActorAnnounced {
        connection_epoch: Uuid,
        actor: Agent,
        mutation_epoch: Uuid,
        expires_at: u64,
    },
    Operation {
        connection_epoch: Uuid,
        frame_id: Uuid,
        actor_id: Uuid,
        mutation_epoch: Uuid,
        operation: Operation,
    },
    OperationResult {
        connection_epoch: Uuid,
        frame_id: Uuid,
        mutation_epoch: Uuid,
        result: serde_json::Value,
    },
    Reconcile {
        connection_epoch: Uuid,
        frame_id: Uuid,
        actor_id: Uuid,
        mutation_epoch: Uuid,
        operation: Operation,
    },
    Reconciled {
        connection_epoch: Uuid,
        frame_id: Uuid,
        mutation_epoch: Uuid,
        result: serde_json::Value,
    },
    Snapshot {
        connection_epoch: Uuid,
        frame_id: Uuid,
        space_id: Uuid,
        after: Option<u64>,
        expected_sequence: Option<u64>,
        limit: Option<u32>,
    },
    SnapshotResult {
        connection_epoch: Uuid,
        frame_id: Uuid,
        result: serde_json::Value,
    },
    CursorAck {
        connection_epoch: Uuid,
        frame_id: Uuid,
        space_id: Uuid,
        sequence: u64,
    },
    CursorAckResult {
        connection_epoch: Uuid,
        frame_id: Uuid,
        result: serde_json::Value,
    },
    Events {
        connection_epoch: Uuid,
        frame_id: Uuid,
        space_id: Uuid,
        after: Option<u64>,
        limit: Option<u32>,
    },
    EventsResult {
        connection_epoch: Uuid,
        frame_id: Uuid,
        result: serde_json::Value,
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
            .join(format!("r-{}.sock", Uuid::new_v4().simple()))
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
            deactivate(&task_broker, nonce);
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
        deactivate(&self.broker, self.nonce);
        let _ = self.shutdown.send(true);
        self.listener.abort();
        if let Ok(descriptor) = read_descriptor(&self.descriptor) {
            if descriptor.nonce == self.nonce {
                #[cfg(target_os = "macos")]
                let _ = std::fs::remove_file(&descriptor.endpoint);
                let _ = std::fs::remove_file(&self.descriptor);
            }
        }
    }
}

// Shutdown fencing must not depend on a storage sweep succeeding (for example on a full disk).
fn deactivate(broker: &Broker, owner: Uuid) {
    if let Ok(mut state) = broker.shared.state.lock() {
        if state.remote_owner != Some(owner) {
            return;
        }
        state.remote_active = false;
        state.remote_owner = None;
        let actors: Vec<_> = state
            .remote_presence
            .iter()
            .map(|(actor, presence)| (actor.clone(), presence.mutation_epoch()))
            .collect();
        for (actor, epoch) in actors {
            if state
                .store
                .observe_remote_disconnect(&actor, &epoch)
                .is_ok()
            {
                state.remote_presence.remove(&actor);
            }
        }
        // Failed uncertainty writes remain fenced and are retried before the next activation.
        let _ = state.store.invalidate_remote_sessions();
        broker.shared.changed.notify_all();
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
            &AuthenticationFrame::Error { error: safe },
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
    let _presence_guard = PresenceGuard {
        broker: broker.clone(),
        connection: epoch,
    };
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
            AuthenticationFrame::ActorHeartbeat {
                connection_epoch,
                actor_id,
                mutation_epoch,
            } => {
                ensure!(
                    connection_epoch == epoch,
                    invalid_input("Connection epoch does not match")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before native presence"))?;
                let mut state = active(broker, owner)?;
                state.store.resolve_remote_run(
                    principal,
                    &actor_id.to_string(),
                    &mutation_epoch.to_string(),
                    false,
                )?;
                ensure!(
                    state.remote_presence.contains_key(&actor_id.to_string())
                        || state.remote_presence.len() < 1000,
                    crate::capacity_exceeded("Remote native presence capacity reached")
                );
                state.remote_presence.insert(
                    actor_id.to_string(),
                    ActorPresence {
                        principal: RemotePrincipal {
                            device: principal.device.clone(),
                            generation: principal.generation,
                        },
                        epoch: mutation_epoch,
                        connection: epoch,
                        seen: Instant::now(),
                    },
                );
                broker.shared.changed.notify_all();
                AuthenticationFrame::ActorHeartbeatResult {
                    connection_epoch: epoch,
                    actor_id,
                    mutation_epoch,
                }
            }
            AuthenticationFrame::ActorAnnounce {
                connection_epoch,
                workspace_id,
                space_id,
                checkout_id,
                native_session,
                native_run,
                program,
                name,
            } => {
                ensure!(
                    connection_epoch == epoch,
                    invalid_input("Connection epoch does not match")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before using this channel"))?;
                let state = active(broker, owner)?;
                let workspace = RemoteWorkspace {
                    id: workspace_id.to_string(),
                    device: principal.device.clone(),
                    space: space_id.to_string(),
                    checkout: checkout_id.to_string(),
                };
                let registered = state
                    .store
                    .register_remote_actor(
                        principal,
                        &workspace,
                        native_session,
                        native_run,
                        &program,
                        &name,
                    )
                    .map_err(crate::classify_storage)?;
                AuthenticationFrame::ActorAnnounced {
                    connection_epoch: epoch,
                    actor: registered.actor,
                    mutation_epoch: Uuid::parse_str(&registered.epoch)?,
                    expires_at: registered.expires_at,
                }
            }
            AuthenticationFrame::Operation {
                connection_epoch,
                frame_id,
                actor_id,
                mutation_epoch,
                operation,
            } => {
                ensure!(
                    connection_epoch == epoch && !frame_id.is_nil(),
                    invalid_input("Invalid connection or correlation epoch")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before using this channel"))?;
                ensure!(
                    !matches!(
                        operation,
                        Operation::AgentRegister { .. }
                            | Operation::AgentReady
                            | Operation::AgentWait
                    ),
                    domain(
                        "feature_unavailable",
                        "Guarded remote readiness is not available on this channel",
                        false,
                        None
                    )
                );
                let write = !matches!(
                    operation,
                    Operation::AgentList
                        | Operation::AgentInbox { .. }
                        | Operation::TaskList { .. }
                        | Operation::TaskGet { .. }
                        | Operation::TaskHistory { .. }
                        | Operation::ThreadGet { .. }
                        | Operation::MessageSearch { .. }
                        | Operation::FileReservations { .. }
                );
                let state = active(broker, owner)?;
                let run = mutation_epoch.to_string();
                let actor = state.store.resolve_remote_run(
                    principal,
                    &actor_id.to_string(),
                    &run,
                    write,
                )?;
                if matches!(operation, Operation::TaskClaim { .. }) {
                    ensure!(
                        state
                            .remote_presence
                            .get(&actor.id)
                            .is_some_and(|presence| presence.connection == epoch
                                && presence.epoch == mutation_epoch
                                && presence.valid(&state.store, &actor.id)),
                        crate::invalid_state(
                            "Native presence expired; reconnect before claiming work"
                        )
                    );
                }
                let result = state.store.execute(&actor, &run, &operation)?;
                broker.shared.changed.notify_all();
                AuthenticationFrame::OperationResult {
                    connection_epoch: epoch,
                    frame_id,
                    mutation_epoch,
                    result,
                }
            }
            AuthenticationFrame::Reconcile {
                connection_epoch,
                frame_id,
                actor_id,
                mutation_epoch,
                operation,
            } => {
                ensure!(
                    connection_epoch == epoch && !frame_id.is_nil(),
                    invalid_input("Invalid reconciliation connection or correlation")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::scope_denied("Authenticate before reconciliation"))?;
                let state = active(broker, owner)?;
                let result = state.store.reconcile_remote_request(
                    principal,
                    &actor_id.to_string(),
                    &mutation_epoch.to_string(),
                    &operation,
                )?;
                AuthenticationFrame::Reconciled {
                    connection_epoch: epoch,
                    frame_id,
                    mutation_epoch,
                    result,
                }
            }
            AuthenticationFrame::Snapshot {
                connection_epoch,
                frame_id,
                space_id,
                after,
                expected_sequence,
                limit,
            } => {
                ensure!(
                    connection_epoch == epoch && !frame_id.is_nil(),
                    invalid_input("Invalid snapshot correlation")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before snapshot"))?;
                let state = active(broker, owner)?;
                let result = state.store.remote_snapshot(
                    principal,
                    &space_id.to_string(),
                    after,
                    expected_sequence,
                    limit,
                )?;
                AuthenticationFrame::SnapshotResult {
                    connection_epoch: epoch,
                    frame_id,
                    result,
                }
            }
            AuthenticationFrame::CursorAck {
                connection_epoch,
                frame_id,
                space_id,
                sequence,
            } => {
                ensure!(
                    connection_epoch == epoch && !frame_id.is_nil(),
                    invalid_input("Invalid cursor confirmation correlation")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before confirming cursor"))?;
                let state = active(broker, owner)?;
                let result = state.store.acknowledge_remote_cursor(
                    principal,
                    &space_id.to_string(),
                    sequence,
                )?;
                AuthenticationFrame::CursorAckResult {
                    connection_epoch: epoch,
                    frame_id,
                    result,
                }
            }
            AuthenticationFrame::Events {
                connection_epoch,
                frame_id,
                space_id,
                after,
                limit,
            } => {
                ensure!(
                    connection_epoch == epoch && !frame_id.is_nil(),
                    invalid_input("Invalid connection or correlation epoch")
                );
                let principal = principal
                    .as_ref()
                    .ok_or_else(|| crate::unauthorized("Authenticate before using this channel"))?;
                let state = active(broker, owner)?;
                let result =
                    state
                        .store
                        .remote_events(principal, &space_id.to_string(), after, limit)?;
                AuthenticationFrame::EventsResult {
                    connection_epoch: epoch,
                    frame_id,
                    result,
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
    fn controller_shutdown_fences_calls_even_when_uncertainty_cannot_be_written() {
        use diesel::{connection::SimpleConnection, Connection};
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("bus.sqlite");
        let descriptor = directory.path().join("controller.json");
        let server = RunningBroker::start(&database).unwrap();
        let controller = RunningController::start(&server, &descriptor).unwrap();
        let owner = controller.nonce;
        let (actor_id, task_id, domain) = {
            let mut state = server.broker.store().unwrap();
            let request = || Uuid::new_v4().to_string();
            let space = state
                .store
                .execute_controller(
                    "/fixture",
                    &ControllerOperation::SpaceCreate {
                        name: "Shutdown failure".into(),
                        request_id: request(),
                    },
                )
                .unwrap()["space_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let issued = state
                .store
                .execute_controller(
                    "/fixture",
                    &ControllerOperation::InvitationCreate {
                        space_ids: vec![space.clone()],
                        ttl_seconds: None,
                        request_id: request(),
                    },
                )
                .unwrap();
            let enrolled = state
                .store
                .enroll_remote(issued["invitation"].as_str().unwrap(), "Participant")
                .unwrap();
            let principal = state
                .store
                .authenticate_remote(enrolled["credential"].as_str().unwrap())
                .unwrap();
            let workspace = state
                .store
                .map_remote_workspace(
                    &principal,
                    Uuid::parse_str(&space).unwrap(),
                    Uuid::new_v4(),
                    "Participant",
                    None,
                )
                .unwrap();
            let actor = state
                .store
                .register_remote_actor(
                    &principal,
                    &workspace,
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    "codex",
                    "shutdown-worker",
                )
                .unwrap();
            let domain = format!("space:{space}");
            let assigned = state
                .store
                .execute(
                    &crate::storage::Store::operator(&domain),
                    crate::storage::OPERATOR_EPOCH,
                    &Operation::TaskAssign {
                        to: actor.actor.name.clone(),
                        description: "Retain unknown work".into(),
                        acceptance: "Fail closed on shutdown".into(),
                        reviewer: None,
                        dependencies: vec![],
                        start_deadline: None,
                        execution_timeout_seconds: None,
                        review_timeout_seconds: None,
                        request_id: request(),
                    },
                )
                .unwrap();
            let task_id = assigned["id"].as_str().unwrap().to_owned();
            state
                .store
                .execute(
                    &actor.actor,
                    &actor.epoch,
                    &Operation::TaskStart {
                        task_id: task_id.clone(),
                        revision: 1,
                        expected_version: None,
                        request_id: request(),
                    },
                )
                .unwrap();
            state.remote_presence.insert(
                actor.actor.id.clone(),
                ActorPresence {
                    principal,
                    epoch: Uuid::parse_str(&actor.epoch).unwrap(),
                    connection: Uuid::new_v4(),
                    seen: Instant::now(),
                },
            );
            (actor.actor.id, task_id, domain)
        };
        let mut fixture = diesel::SqliteConnection::establish(database.to_str().unwrap()).unwrap();
        fixture.batch_execute("CREATE TRIGGER reject_loss BEFORE UPDATE OF certainty ON attempts BEGIN SELECT RAISE(FAIL,'fixture write failure'); END;").unwrap();
        drop(controller);
        {
            let state = server.broker.shared.state.lock().unwrap();
            assert!(!state.remote_active);
            assert!(state.remote_owner.is_none());
            assert!(state.remote_presence.contains_key(&actor_id));
        }
        assert!(!descriptor.exists());
        assert!(active(&server.broker, owner).is_err());
        fixture.batch_execute("DROP TRIGGER reject_loss;").unwrap();
        let state = server.broker.store().unwrap();
        assert!(state.remote_presence.is_empty());
        let task = state.store.operator_task(&domain, &task_id).unwrap();
        assert_eq!(task.state, "running");
        assert_eq!(task.attempts[0].certainty, "unknown");
        assert!(task.attempts[0].finished_at.is_none());
    }

    #[test]
    fn discovery_rejects_unbounded_and_foreign_descriptors() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controller.json");
        let descriptor = Descriptor {
            endpoint: {
                #[cfg(target_os = "macos")]
                {
                    directory
                        .path()
                        .join("controller.sock")
                        .to_string_lossy()
                        .into_owned()
                }
                #[cfg(windows)]
                {
                    format!(r"\\.\pipe\warp-agent-controller-{}", Uuid::new_v4())
                }
            },
            pid: std::process::id(),
            protocol_major: 2,
            coordinator_id: Uuid::new_v4(),
            nonce: Uuid::new_v4(),
        };
        publish(&path, &descriptor).unwrap();
        assert!(read_descriptor(&path).is_ok());
        assert!(publish(&path, &descriptor).is_err());
        std::fs::write(&path, vec![b' '; 4097]).unwrap();
        assert!(read_descriptor(&path).is_err());
        std::fs::remove_file(&path).unwrap();
        #[cfg(target_os = "macos")]
        {
            let target = directory.path().join("target.json");
            publish(&target, &descriptor).unwrap();
            std::os::unix::fs::symlink(&target, &path).unwrap();
            assert!(read_descriptor(&path).is_err());
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(read_descriptor(&target).is_err());
        }
    }

    #[test]
    fn gateway_rejects_invalid_frames_without_echoing_payloads() {
        let directory = tempfile::tempdir().unwrap();
        let server = RunningBroker::start(&directory.path().join("bus.sqlite")).unwrap();
        let descriptor = directory.path().join("controller.json");
        let _controller = RunningController::start(&server, &descriptor).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            for bytes in [
                0u32.to_be_bytes().to_vec(),
                (crate::MAX_FRAME as u32 + 1).to_be_bytes().to_vec(),
                [1u32.to_be_bytes().as_slice(), b"{"].concat(),
            ] {
                let (mut client, relay) = tokio::io::duplex(128);
                let path = descriptor.clone();
                let task = tokio::spawn(async move {
                    let (input, output) = tokio::io::split(relay);
                    gateway(&path, input, output).await
                });
                client.write_all(&bytes).await.unwrap();
                let AuthenticationFrame::Error { error } =
                    receive(&mut client, Instant::now() + DEADLINE)
                        .await
                        .unwrap()
                else {
                    panic!("Expected bounded protocol rejection")
                };
                assert_eq!(error.message, "Controller session is unavailable");
                let _ = task.await.unwrap();
            }
        });
    }

    #[test]
    fn gateway_enrollment_revoke_and_disable_use_real_local_ipc() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("bus.sqlite");
        let descriptor = directory.path().join("controller.json");
        let server = RunningBroker::start(&database).unwrap();
        let controller = RunningController::start(&server, &descriptor).unwrap();
        assert!(RunningController::start(&server, &descriptor).is_err());
        let discovered = read_descriptor(&descriptor).unwrap();
        let coordinator = discovered.coordinator_id;
        let previous_owner = discovered.nonce;
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
            let checkout = Uuid::new_v4();
            let workspace = server
                .broker
                .store()
                .unwrap()
                .store
                .map_remote_workspace(
                    &RemotePrincipal {
                        device: device_id.to_string(),
                        generation: 1,
                    },
                    Uuid::parse_str(&space).unwrap(),
                    checkout,
                    "Owned gateway checkout",
                    None,
                )
                .unwrap();
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
            let mut actors = Vec::new();
            for name in ["remote-issuer", "remote-worker"] {
                send(
                    &mut client,
                    &AuthenticationFrame::ActorAnnounce {
                        connection_epoch,
                        workspace_id: Uuid::parse_str(&workspace.id).unwrap(),
                        space_id: Uuid::parse_str(&space).unwrap(),
                        checkout_id: checkout,
                        native_session: Uuid::new_v4(),
                        native_run: Uuid::new_v4(),
                        program: "codex".into(),
                        name: name.into(),
                    },
                    Instant::now() + DEADLINE,
                )
                .await
                .unwrap();
                let AuthenticationFrame::ActorAnnounced {
                    actor,
                    mutation_epoch,
                    ..
                } = receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
                else {
                    panic!("Expected assigned remote actor")
                };
                assert_eq!(actor.project, format!("space:{space}"));
                actors.push((actor, mutation_epoch));
            }
            for (actor, mutation_epoch) in &actors {
                send(&mut client, &AuthenticationFrame::ActorHeartbeat {
                    connection_epoch, actor_id: Uuid::parse_str(&actor.id).unwrap(),
                    mutation_epoch: *mutation_epoch,
                }, Instant::now() + DEADLINE).await.unwrap();
                let AuthenticationFrame::ActorHeartbeatResult { actor_id, mutation_epoch: returned, .. } =
                    receive(&mut client, Instant::now() + DEADLINE).await.unwrap()
                    else { panic!("Expected native presence receipt") };
                assert_eq!(actor_id.to_string(), actor.id);
                assert_eq!(returned, *mutation_epoch);
            }
            {
                let mut state = server.broker.store().unwrap();
                assert_eq!(state.remote_presence.len(), 2);
                assert!(state.remote_presence[&actors[0].0.id].valid(&state.store, &actors[0].0.id));
                state.remote_presence.get_mut(&actors[0].0.id).unwrap().seen = Instant::now() - PRESENCE;
                assert!(!state.remote_presence[&actors[0].0.id].valid(&state.store, &actors[0].0.id));
            }
            send(&mut client, &AuthenticationFrame::ActorHeartbeat {
                connection_epoch, actor_id: Uuid::parse_str(&actors[0].0.id).unwrap(),
                mutation_epoch: actors[0].1,
            }, Instant::now() + DEADLINE).await.unwrap();
            let AuthenticationFrame::ActorHeartbeatResult { .. } =
                receive(&mut client, Instant::now() + DEADLINE).await.unwrap()
                else { panic!("Expected refreshed receipt-time presence") };
            let pool = server.broker.operator(&format!("space:{space}"), &Operation::TaskCreatePool {
                description: "Native remote claim".into(), acceptance: "Current receipt-time presence".into(),
                eligible: vec![actors[0].0.name.clone()], reviewer: None, dependencies: vec![],
                start_deadline: None, execution_timeout_seconds: None, review_timeout_seconds: None,
                request_id: Uuid::new_v4().to_string(),
            }).unwrap();
            send(&mut client, &AuthenticationFrame::Operation {
                connection_epoch, frame_id: Uuid::new_v4(),
                actor_id: Uuid::parse_str(&actors[0].0.id).unwrap(), mutation_epoch: actors[0].1,
                operation: Operation::TaskClaim { task_id: pool["id"].as_str().unwrap().into(),
                    expected_version: None, request_id: Uuid::new_v4().to_string() },
            }, Instant::now() + DEADLINE).await.unwrap();
            let AuthenticationFrame::OperationResult { result, .. } =
                receive(&mut client, Instant::now() + DEADLINE).await.unwrap()
                else { panic!("Expected presence-authorized native pool claim") };
            assert_eq!(result["assignee"], actors[0].0.id);
            let request_id = Uuid::new_v4().to_string();
            let operation = || AuthenticationFrame::Operation {
                connection_epoch,
                frame_id: Uuid::new_v4(),
                actor_id: Uuid::parse_str(&actors[0].0.id).unwrap(),
                mutation_epoch: actors[0].1,
                operation: Operation::TaskAssign {
                    to: actors[1].0.name.clone(),
                    description: "Exercise the original coordinator engine".into(),
                    acceptance: "Duplicate transport frames create one durable task".into(),
                    reviewer: None,
                    request_id: request_id.clone(),
                    dependencies: vec![],
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                },
            };
            let reconcile = || {
                let AuthenticationFrame::Operation {
                    actor_id,
                    mutation_epoch,
                    operation,
                    ..
                } = operation()
                else {
                    unreachable!()
                };
                AuthenticationFrame::Reconcile {
                    connection_epoch,
                    frame_id: Uuid::new_v4(),
                    actor_id,
                    mutation_epoch,
                    operation,
                }
            };
            send(&mut client, &reconcile(), Instant::now() + DEADLINE)
                .await
                .unwrap();
            let AuthenticationFrame::Reconciled { result, .. } =
                receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
            else {
                panic!("Expected original receipt lookup")
            };
            assert_eq!(result["status"], "not_committed");
            let mut original = None;
            for _ in 0..2 {
                send(&mut client, &operation(), Instant::now() + DEADLINE)
                    .await
                    .unwrap();
                let AuthenticationFrame::OperationResult {
                    result,
                    mutation_epoch,
                    ..
                } = receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
                else {
                    panic!("Expected task operation result")
                };
                assert_eq!(mutation_epoch, actors[0].1);
                if let Some(prior) = &original {
                    assert_eq!(&result, prior);
                } else {
                    original = Some(result);
                }
            }
            send(&mut client, &reconcile(), Instant::now() + DEADLINE)
                .await
                .unwrap();
            let AuthenticationFrame::Reconciled { result, .. } =
                receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
            else {
                panic!("Expected committed original receipt")
            };
            assert_eq!(result["status"], "committed");
            assert_eq!(result["result"], original.unwrap());
            send(
                &mut client,
                &AuthenticationFrame::Events {
                    connection_epoch,
                    frame_id: Uuid::new_v4(),
                    space_id: Uuid::parse_str(&space).unwrap(),
                    after: None,
                    limit: Some(2),
                },
                Instant::now() + DEADLINE,
            )
            .await
            .unwrap();
            let AuthenticationFrame::EventsResult { result, .. } =
                receive(&mut client, Instant::now() + DEADLINE)
                    .await
                    .unwrap()
            else {
                panic!("Expected scoped event page")
            };
            assert_eq!(result["events"].as_array().unwrap().len(), 2);
            assert!(result["cursor"].as_u64().is_some());
            let confirmed_sequence = result["cursor"].as_u64().unwrap();
            send(&mut client, &AuthenticationFrame::Snapshot {
                connection_epoch, frame_id: Uuid::new_v4(), space_id: Uuid::parse_str(&space).unwrap(),
                after: None, expected_sequence: None, limit: Some(2),
            }, Instant::now() + DEADLINE).await.unwrap();
            let AuthenticationFrame::SnapshotResult { result, .. } =
                receive(&mut client, Instant::now() + DEADLINE).await.unwrap()
                else { panic!("Expected authorized scoped snapshot") };
            assert_eq!(result["records"].as_array().unwrap().len(), 2);
            assert!(result["high_water"].as_u64().unwrap() >= confirmed_sequence);
            send(&mut client, &AuthenticationFrame::CursorAck {
                connection_epoch, frame_id: Uuid::new_v4(), space_id: Uuid::parse_str(&space).unwrap(),
                sequence: confirmed_sequence,
            }, Instant::now() + DEADLINE).await.unwrap();
            let AuthenticationFrame::CursorAckResult { result, .. } =
                receive(&mut client, Instant::now() + DEADLINE).await.unwrap()
                else { panic!("Expected confirmed scoped cursor") };
            assert_eq!(result["sequence"], confirmed_sequence);

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
            send(&mut client, &operation(), Instant::now() + DEADLINE)
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
            // Session cleanup removes only ephemeral online leases, not durable tasks.
            for _ in 0..20 {
                if server.broker.store().unwrap().remote_presence.is_empty() { break; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            let state = server.broker.store().unwrap();
            assert!(state.remote_presence.is_empty());
            assert_eq!(state.store.operator_task(&format!("space:{space}"), pool["id"].as_str().unwrap()).unwrap().state, "queued");

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
        assert!(active(&server.broker, previous_owner).is_err());
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
