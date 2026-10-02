//! Authenticated, bounded local IPC. Warp alone creates and activates terminal bindings.
use crate::{
    binding_inactive, capacity_exceeded, coordinator_unavailable, epoch_expired, invalid_input,
    invalid_state, scope_denied, unauthorized, Agent, ControllerOperation, DomainError, Operation,
    Store, Task, MAX_FRAME, MUTATION_EPOCH,
};
use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;
#[cfg(windows)]
#[path = "windows_pipe.rs"]
pub(crate) mod windows_pipe;
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use uuid::Uuid;

#[cfg(any(target_os = "macos", windows))]
#[path = "remote_control.rs"]
pub mod remote_control;
use crate::readiness::{Activity, Draft};

pub const ENDPOINT: &str = "WARP_AGENT_ENDPOINT";
pub const CAPABILITY: &str = "WARP_AGENT_CAPABILITY";
pub const TERMINAL: &str = "WARP_TERMINAL_SESSION_UUID";
pub const PROTOCOL_MAJOR: u16 = 2;
pub const LOCAL_FEATURES: &[&str] = &[
    "task_control", "dependencies", "threads", "reservations", "evidence_refs", "event_resume", "task_history",
];
const WAIT: Duration = Duration::from_secs(20);
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default)]
    pub protocol_major: u16,
    pub terminal: String,
    pub capability: String,
    pub run: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub defer_initial_ready: bool,
    /// Authenticated launch-relay lifecycle, never an MCP tool argument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_activity: Option<Activity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    pub operation: Operation,
}
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<DomainError>,
}
struct Terminal {
    capability: String,
    workspace: Option<crate::storage::WorkspaceBinding>,
    revoked: bool,
    live: Option<Live>,
}
struct Live {
    program: String,
    project: String,
    run: String,
    agent: Option<Agent>,
    waiting: bool,
    ready: Option<Instant>,
    last_output: Instant,
    last_input: Instant,
    observed: Instant,
    draft: Draft,
    rich_draft: bool,
    blocked: bool,
    activity: Activity,
    native_activity: Option<Activity>,
    paused: bool,
    readiness_source: &'static str,
    generation: u64,
    wake: Option<Wake>,
    delivered: HashSet<String>,
    initial_prompt: bool,
    started: Instant,
    expired: bool,
}
/// A run-scoped delivery claim. PTY submission never acknowledges the underlying message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wake {
    pub terminal: String,
    pub run: String,
    pub message_id: String,
    generation: u64,
}
struct State {
    store: Store,
    terminals: HashMap<String, Terminal>,
    programs: Option<HashSet<String>>,
    remote_active: bool,
    remote_owner: Option<Uuid>,
    remote_presence: HashMap<String, remote_control::ActorPresence>,
}
impl State {
    fn expire_remote_presence(&mut self) -> Result<()> {
        let expired: Vec<_> = self.remote_presence.iter()
            .filter(|(actor, presence)| !self.remote_active || !presence.valid(&self.store, actor))
            .map(|(actor, presence)| (actor.clone(), presence.mutation_epoch()))
            .collect();
        for (actor, epoch) in expired {
            self.store.observe_remote_disconnect(&actor, &epoch)?;
            self.remote_presence.remove(&actor);
        }
        Ok(())
    }
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    stopped: AtomicBool,
    connections: AtomicUsize,
    shutdown: tokio::sync::Notify,
}
#[derive(Clone)]
pub struct Broker {
    shared: Arc<Shared>,
    pub endpoint: String,
    runtime: tokio::runtime::Handle,
}
/// Live native MCP bindings in a single project.
#[derive(Clone)]
pub struct Peer {
    pub terminal: String,
    pub run: String,
    pub name: String,
    pub program: String,
}
pub struct RunningBroker {
    pub broker: Broker,
    thread: Option<thread::JoinHandle<()>>,
    directory: std::path::PathBuf,
}
/// Trusted native panel query; cursors are scoped to the selected project.
#[derive(Clone, Default)]
pub struct PanelQuery {
    pub project: String,
    pub scope: Option<String>,
    pub terminal: Option<String>,
    pub agent_after: Option<String>,
    pub task_after: Option<u64>,
    pub selected_task: Option<String>,
    pub event_after: Option<u64>,
    pub wait: bool,
    pub spaces: bool,
    pub space_after: Option<String>,
    pub reservation_after: Option<u64>,
    pub task_state: Option<String>,
    pub task_assignee: Option<String>,
    pub include_archived: bool,
    pub message_query: Option<String>,
    pub selected_thread: Option<String>,
    pub message_after: Option<u64>,
    pub history: bool,
    pub history_after: Option<u64>,
}
impl RunningBroker {
    /// Executable aliases live beside the private broker socket and disappear with the app.
    pub fn launcher_directory(&self) -> std::path::PathBuf {
        self.directory.join("launchers")
    }
    pub fn start(database: &Path) -> Result<Self> {
        let store = Store::open(
            database
                .to_str()
                .ok_or_else(|| anyhow!("Database path must be UTF-8"))?,
        )?;
        #[cfg(unix)]
        let runtime_root = std::path::PathBuf::from("/tmp");
        #[cfg(windows)]
        let runtime_root = std::env::temp_dir();
        // macOS TMPDIR can exceed the Unix socket path limit before adding a filename.
        let directory = runtime_root.join(format!("warp-agent-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(unix)]
        let endpoint = directory.join("bus.sock").to_string_lossy().into_owned();
        #[cfg(windows)]
        let endpoint = format!(r"\\.\pipe\warp-agent-{}", Uuid::new_v4());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let listener = {
            let _entered = runtime.enter();
            #[cfg(unix)]
            let listener = UnixListener::bind(&endpoint)?;
            #[cfg(windows)]
            let listener = windows_pipe::create(&endpoint, true)?;
            listener
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600))?;
        }
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                store,
                terminals: HashMap::new(),
                programs: None,
                remote_active: false,
                remote_owner: None,
                remote_presence: HashMap::new(),
            }),
            changed: Condvar::new(),
            stopped: AtomicBool::new(false),
            connections: AtomicUsize::new(0),
            shutdown: tokio::sync::Notify::new(),
        });
        let broker = Broker {
            shared: shared.clone(),
            endpoint,
            runtime: runtime.handle().clone(),
        };
        let server = broker.clone();
        #[cfg(windows)]
        let endpoint = broker.endpoint.clone();
        let thread = thread::spawn(move || {
            runtime.block_on(async move {
                #[cfg(windows)]
                let mut listener = listener;
                loop {
                    let stream = tokio::select! {
                        _ = shared.shutdown.notified() => break,
                        connection = async {
                            #[cfg(unix)]
                            { listener.accept().await.map(|(stream, _)| stream) }
                            #[cfg(windows)]
                            {
                                listener.connect().await?;
                                // Keep a listening instance alive before handing off the connected pipe.
                                let next = windows_pipe::create(&endpoint, false).map_err(std::io::Error::other)?;
                                Ok::<_, std::io::Error>(std::mem::replace(&mut listener, next))
                            }
                        } => match connection { Ok(stream) => stream, Err(_) => break },
                    };
                    if shared.connections.fetch_add(1, Ordering::AcqRel) >= 32 {
                        shared.connections.fetch_sub(1, Ordering::AcqRel);
                        continue;
                    }
                    let broker = server.clone();
                    tokio::spawn(async move {
                        struct Connection(Arc<Shared>);
                        impl Drop for Connection {
                            fn drop(&mut self) {
                                self.0.connections.fetch_sub(1, Ordering::AcqRel);
                            }
                        }
                        let _connection = Connection(broker.shared.clone());
                        let _ = handle(stream, broker).await;
                    });
                }
            });
            runtime.shutdown_timeout(Duration::from_secs(1));
        });
        Ok(Self {
            broker,
            thread: Some(thread),
            directory,
        })
    }
}
impl Drop for RunningBroker {
    fn drop(&mut self) {
        self.broker.shared.stopped.store(true, Ordering::Release);
        self.broker.shared.changed.notify_all();
        self.broker.shared.shutdown.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
impl Broker {
    /// App settings are authoritative; tests may retain an unrestricted broker.
    pub fn set_programs(&self, programs: Option<HashSet<String>>) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.programs = programs;
            let allowed = state.programs.clone();
            for binding in state.terminals.values_mut() {
                if binding.live.as_ref().is_some_and(|live| {
                    allowed
                        .as_ref()
                        .is_some_and(|programs| !programs.contains(&live.program))
                }) {
                    binding.live = None;
                }
            }
        }
        self.shared.changed.notify_all();
    }
    pub fn peers(&self, terminal: &str) -> Vec<Peer> {
        let Ok(state) = self.shared.state.lock() else {
            return vec![];
        };
        let Some(source) = state.terminals.get(terminal).and_then(|t| t.live.as_ref()) else {
            return vec![];
        };
        if source.agent.as_ref().is_none_or(|actor| state.store.authorize(actor).is_err()) {
            return vec![];
        }
        let mut peers: Vec<_> = state
            .terminals
            .iter()
            .filter_map(|(id, binding)| {
                let live = binding.live.as_ref()?;
                let agent = live.agent.as_ref()?;
                (id != terminal && live.project == source.project && state.store.authorize(agent).is_ok()).then(|| Peer {
                    terminal: id.clone(),
                    run: live.run.clone(),
                    name: agent.name.clone(),
                    program: live.program.clone(),
                })
            })
            .collect();
        peers.sort_by(|a, b| a.name.cmp(&b.name));
        peers
    }

    pub fn run(&self, terminal: &str) -> Option<String> {
        self.shared
            .state
            .lock()
            .ok()?
            .terminals
            .get(terminal)?
            .live
            .as_ref()
            .filter(|live| live.agent.is_some())
            .map(|live| live.run.clone())
    }

    pub fn prepare(&self, terminal: &str) -> Result<String> {
        self.prepare_binding(terminal, None)
    }

    /// Only the trusted app selects a mapped workspace for a fresh shared pane.
    pub fn prepare_in_workspace(&self, terminal: &str, workspace_id: &str) -> Result<String> {
        let workspace = self.store()?.store.workspace_binding(workspace_id)?;
        self.prepare_binding(terminal, Some(workspace))
    }

    /// Admit exactly the scope reviewed by the user; remapping cannot change that intent.
    pub fn prepare_bound_workspace(&self, terminal: &str, binding: &crate::WorkspaceBinding) -> Result<String> {
        self.prepare_binding(terminal, Some(binding.clone()))
    }

    pub fn validate_workspace(&self, binding: &crate::WorkspaceBinding) -> Result<()> {
        self.store()?.store.authorize_workspace(binding)
    }

    fn prepare_binding(&self, terminal: &str, workspace: Option<crate::storage::WorkspaceBinding>) -> Result<String> {
        let capability = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| anyhow!("Broker unavailable"))?;
        ensure!(
            state.terminals.len() < 1000,
            capacity_exceeded("Terminal capacity reached")
        );
        ensure!(!state.terminals.contains_key(terminal), invalid_state("Terminal is already prepared; open a fresh pane"));
        if let Some(workspace) = &workspace { state.store.authorize_workspace(workspace)?; }
        state.terminals.insert(
            terminal.into(),
            Terminal {
                capability: capability.clone(),
                workspace,
                revoked: false,
                live: None,
            },
        );
        Ok(capability)
    }
    pub fn activate(
        &self,
        terminal: &str,
        program: &str,
        project: &str,
        initial_prompt: bool,
    ) -> Result<()> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| anyhow!("Broker unavailable"))?;
        ensure!(
            state
                .programs
                .as_ref()
                .is_none_or(|programs| programs.contains(program)),
            scope_denied("Agent communication is disabled for this program")
        );
        let binding = state.terminals.get(terminal).ok_or_else(|| unauthorized("Terminal is not bound"))?;
        ensure!(!binding.revoked, scope_denied("Terminal participation was revoked"));
        let project = if let Some(workspace) = &binding.workspace {
            state.store.authorize_workspace(workspace)?;
            ensure!(crate::project_root(Path::new(project))? == workspace.root,
                scope_denied("Native directory does not match the selected workspace"));
            workspace.domain()
        } else { project.to_owned() };
        let binding = state.terminals.get_mut(terminal).unwrap();
        binding.live = Some(Live {
            program: program.into(),
            project,
            run: Uuid::new_v4().to_string(),
            agent: None,
            waiting: false,
            ready: None,
            last_output: Instant::now(),
            last_input: Instant::now(),
            observed: Instant::now(),
            draft: Draft::default(),
            rich_draft: false,
            blocked: false,
            activity: Activity::Starting,
            native_activity: None,
            paused: false,
            readiness_source: "startup",
            generation: 0,
            wake: None,
            delivered: HashSet::new(),
            initial_prompt,
            started: Instant::now(),
            expired: false,
        });
        self.shared.changed.notify_all();
        Ok(())
    }
    pub fn end(&self, terminal: &str) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(binding) = state.terminals.get_mut(terminal) {
                binding.live = None;
            }
        }
        self.shared.changed.notify_all();
    }
    /// Fences every pending request of this terminal's current run without ending the session.
    #[doc(hidden)]
    pub fn expire_epoch(&self, terminal: &str) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state
                .terminals
                .get_mut(terminal)
                .and_then(|binding| binding.live.as_mut())
            {
                live.expired = true;
            }
        }
        self.shared.changed.notify_all();
    }
    /// Called by native lifecycle completion/busy events; model readiness covers clients without hooks.
    pub fn readiness(&self, terminal: &str, ready: bool) {
        self.activity(terminal, if ready { Activity::Idle } else { Activity::Working });
    }
    pub fn activity(&self, terminal: &str, activity: Activity) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state
                .terminals
                .get_mut(terminal)
                .and_then(|binding| binding.live.as_mut())
            {
                live.generation += 1;
                live.activity = activity;
                live.readiness_source = "native"; live.observed = Instant::now();
                live.ready = activity.is_idle().then(Instant::now);
                if !activity.is_idle() {
                    live.initial_prompt = false;
                    if let Some(wake) = live.wake.take() {
                        live.delivered.remove(&wake.message_id);
                    }
                }
            }
        }
    }
    /// User input always wins over automatic submission, including an Enter timer already scheduled.
    pub fn user_input(&self, terminal: &str, submitted_or_cancelled: bool) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state
                .terminals
                .get_mut(terminal)
                .and_then(|binding| binding.live.as_mut())
            {
                if submitted_or_cancelled {
                    live.draft.clear();
                    live.activity = Activity::Working;
                    live.paused = false;
                    live.readiness_source = "user_submit"; live.observed = Instant::now();
                    live.ready = None;
                } else {
                    live.draft.invalidate();
                }
                live.initial_prompt = false;
                live.generation += 1;
                if let Some(wake) = live.wake.take() {
                    live.delivered.remove(&wake.message_id);
                }
            }
        }
    }
    /// Track actual native edits while retaining the lifecycle's idle evidence.
    pub fn input_bytes(&self, terminal: &str, bytes: &[u8]) {
        if bytes.is_empty() { return; }
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state.terminals.get_mut(terminal).and_then(|binding| binding.live.as_mut()) {
                if let Some(activity) = live.draft.input(bytes) {
                    live.activity = activity;
                    live.paused = activity == Activity::Cancelled;
                    live.readiness_source = "user_input"; live.observed = Instant::now();
                    live.ready = None;
                }
                live.initial_prompt = false;
                live.generation += 1;
                live.last_input = Instant::now(); live.observed = live.last_input;
                if let Some(wake) = live.wake.take() { live.delivered.remove(&wake.message_id); }
            }
        }
    }
    /// App-owned rich drafts/attachments and permission overlays affect delivery, not idle evidence.
    pub fn input_guard(&self, terminal: &str, rich_draft: bool, blocked: bool) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state.terminals.get_mut(terminal).and_then(|binding| binding.live.as_mut()) {
                if live.rich_draft != rich_draft || live.blocked != blocked {
                    live.rich_draft = rich_draft;
                    live.blocked = blocked;
                    live.generation += 1;
                    if let Some(wake) = live.wake.take() { live.delivered.remove(&wake.message_id); }
                    live.last_input = Instant::now(); live.observed = live.last_input;
                }
            }
        }
    }
    pub fn output(&self, terminal: &str) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state
                .terminals
                .get_mut(terminal)
                .and_then(|binding| binding.live.as_mut())
            {
                live.last_output = Instant::now(); live.observed = live.last_output;
            }
        }
    }
    pub fn wakeups(&self) -> Vec<Wake> {
        let Ok(state) = self.shared.state.lock() else {
            return Vec::new();
        };
        // Pool notices are broadcast for visibility, but only one eligible terminal is woken.
        // A submitted notice stays outstanding until claim; do not paste it into a second model.
        let dispatched_pools = dispatched_pools(&state);
        let mut candidates: Vec<_> = state
            .terminals
            .iter()
            .filter_map(|(terminal, binding)| {
                let live = binding.live.as_ref()?;
                if live.waiting
                    || live.paused
                    || live.expired || live.started.elapsed() >= MUTATION_EPOCH
                    || !live.draft.is_empty() || live.rich_draft || live.blocked
                    || live.wake.is_some()
                    || live.ready?.elapsed() < Duration::from_millis(750)
                    || live.last_output.elapsed() < Duration::from_millis(500)
                    || live.last_input.elapsed() < Duration::from_millis(500)
                    || live.native_activity.is_some_and(|activity| !activity.is_idle())
                {
                    return None;
                }
                let actor = live.agent.as_ref()?;
                let message = state.store.next_work(actor, &live.run).ok()??;
                if live.delivered.contains(&message.id) {
                    return None;
                }
                let pool = (message.kind == "available").then_some(message.task_id.clone()).flatten();
                if pool.as_ref().is_some_and(|task| dispatched_pools.contains(task)) {
                    return None;
                }
                Some((live.project.clone(), message.sequence, pool, Wake {
                    terminal: terminal.clone(),
                    run: live.run.clone(),
                    message_id: message.id,
                    generation: live.generation,
                }))
            })
            .collect();
        candidates.sort_by(|left, right| (&left.0, left.1, &left.3.terminal)
            .cmp(&(&right.0, right.1, &right.3.terminal)));
        let mut pools = HashSet::new();
        candidates.into_iter().filter_map(|(_, _, pool, wake)| {
            if pool.is_some_and(|task| !pools.insert(task)) { None } else { Some(wake) }
        }).collect()
    }
    /// Surface queued work even when the receiver is busy or has not announced readiness.
    pub fn pending_work(&self) -> Vec<Wake> {
        let Ok(state) = self.shared.state.lock() else {
            return vec![];
        };
        state
            .terminals
            .iter()
            .filter_map(|(terminal, binding)| {
                let live = binding.live.as_ref()?;
                let message = state
                    .store
                    .first_pending(live.agent.as_ref()?, false)
                    .ok()??;
                Some(Wake {
                    terminal: terminal.clone(),
                    run: live.run.clone(),
                    message_id: message.id,
                    generation: live.generation,
                })
            })
            .collect()
    }
    pub fn claim_wake(&self, wake: &Wake) -> bool {
        let Ok(mut state) = self.shared.state.lock() else {
            return false;
        };
        let Some(live) = state
            .terminals
            .get(&wake.terminal)
            .and_then(|binding| binding.live.as_ref())
        else {
            return false;
        };
        if live.run != wake.run
            || live.paused
            || live.expired || live.started.elapsed() >= MUTATION_EPOCH
            || live.generation != wake.generation
            || !live.draft.is_empty() || live.rich_draft || live.blocked
            || live.ready.is_none()
            || live.native_activity.is_some_and(|activity| !activity.is_idle())
            || live.waiting
            || live.wake.is_some()
            || live.delivered.contains(&wake.message_id)
        {
            return false;
        }
        let Some(actor) = live.agent.as_ref() else {
            return false;
        };
        let Ok(Some(message)) = state.store.next_work(actor, &live.run) else {
            return false;
        };
        if message.id != wake.message_id
            || (message.kind == "available" && message.task_id.as_ref()
                .is_some_and(|task| dispatched_pools(&state).contains(task)))
        {
            return false;
        }
        let live = state
            .terminals
            .get_mut(&wake.terminal)
            .unwrap()
            .live
            .as_mut()
            .unwrap();
        live.delivered.insert(wake.message_id.clone());
        live.wake = Some(wake.clone());
        true
    }
    pub fn wake_valid(&self, wake: &Wake) -> bool {
        self.shared.state.lock().ok().is_some_and(|state| {
            state
                .terminals
                .get(&wake.terminal)
                .and_then(|binding| binding.live.as_ref())
                .is_some_and(|live| {
                    live.agent.as_ref().is_some_and(|actor| state.store.authorize(actor).is_ok()) && live.run == wake.run && !live.paused && !live.expired && live.started.elapsed() < MUTATION_EPOCH && live.draft.is_empty() && !live.rich_draft && !live.blocked && live.wake.as_ref() == Some(wake)
                })
        })
    }
    pub fn finish_wake(&self, wake: &Wake, submitted: bool) {
        if let Ok(mut state) = self.shared.state.lock() {
            if let Some(live) = state
                .terminals
                .get_mut(&wake.terminal)
                .and_then(|binding| binding.live.as_mut())
            {
                if live.wake.as_ref() == Some(wake) {
                    live.wake = None;
                    if !submitted {
                        live.delivered.remove(&wake.message_id);
                        live.ready = Some(Instant::now());
                        live.activity = Activity::Idle;
                        live.readiness_source = "retry"; live.observed = Instant::now();
                        live.generation += 1;
                    }
                    else { live.ready = None; live.activity = Activity::Working; live.readiness_source = "peer_submit"; live.observed = Instant::now(); }
                }
            }
        }
    }
    fn execute(&self, request: &Request) -> Result<Value> {
        ensure!(
            !self.shared.stopped.load(Ordering::Acquire),
            coordinator_unavailable("Broker stopped")
        );
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| coordinator_unavailable("Broker unavailable"))?;
        authenticate(
            &state,
            request,
            matches!(request.operation, Operation::AgentRegister { .. }),
        )?;
        if let Some(directory) = &request.directory {
            ensure!(
                Path::new(directory).is_absolute(),
                invalid_input("Native workspace must be absolute")
            );
            let project = crate::project_root(Path::new(directory))?;
            let workspace = state.terminals.get(&request.terminal).unwrap().workspace.clone();
            if let Some(workspace) = &workspace {
                ensure!(project == workspace.root, scope_denied("Native directory does not match the selected workspace"));
            }
            let live = state
                .terminals
                .get_mut(&request.terminal)
                .unwrap()
                .live
                .as_mut()
                .unwrap();
            if workspace.is_some() {
                // The captured shared domain is immutable, including before first discovery.
            } else if live.agent.is_some() {
                ensure!(
                    live.project == project,
                    invalid_state("Native workspace changed; start a fresh managed CLI session")
                );
            } else {
                live.project = project;
            }
        }
        {
            let live = state
                .terminals
                .get_mut(&request.terminal)
                .unwrap()
                .live
                .as_mut()
                .unwrap();
            if request.defer_initial_ready && live.agent.is_none() {
                live.initial_prompt = false;
                live.ready = None;
            }
        }
        let live = state
            .terminals
            .get(&request.terminal)
            .unwrap()
            .live
            .as_ref()
            .unwrap();
        let run = live.run.clone();
        if let Some(activity) = request.native_activity {
            let live = state.terminals.get_mut(&request.terminal).unwrap().live.as_mut().unwrap();
            live.native_activity = Some(activity);
            live.activity = activity;
            live.readiness_source = "native"; live.observed = Instant::now();
            live.ready = activity.is_idle().then(Instant::now);
            live.generation += 1;
            live.initial_prompt = false;
            if let Some(wake) = live.wake.take() { live.delivered.remove(&wake.message_id); }
            self.shared.changed.notify_all();
            return Ok(json!({"activity": live.activity, "ready": live.ready.is_some()}));
        }
        let live = state.terminals.get(&request.terminal).unwrap().live.as_ref().unwrap();
        if let Operation::AgentRegister { name } = &request.operation {
            let automatic = name.is_empty();
            let name = if automatic {
                format!(
                    "{}-{}",
                    live.program,
                    &Uuid::new_v4().simple().to_string()[..8]
                )
            } else {
                name.clone()
            };
            if let Some(agent) = &live.agent {
                if automatic || agent.name == name {
                    return Ok(registration_result(agent, &run));
                }
                ensure!(
                    !state.store.has_work(agent)?,
                    invalid_state("This identity already has work; preserve it instead of rebinding")
                );
            }
            let program = live.program.clone();
            let project = live.project.clone();
            ensure!(
                !state
                    .terminals
                    .iter()
                    .any(|(terminal, binding)| terminal != &request.terminal
                        && binding
                            .live
                            .as_ref()
                            .and_then(|live| live.agent.as_ref())
                            .is_some_and(|agent| agent.project == project && agent.name == name)),
                invalid_state("Agent name is owned by another live terminal")
            );
            // An offline name is reclaimed explicitly, preserving its pending work across new panes.
            let agent = match &state.terminals.get(&request.terminal).unwrap().workspace {
                Some(workspace) => state.store.register_in_workspace(&request.terminal, &program, workspace, &name)?,
                None => state.store.register(&request.terminal, &program, &project, &name)?,
            };
            state.store.recover(&agent, &run)?;
            let live = state
                .terminals
                .get_mut(&request.terminal)
                .unwrap()
                .live
                .as_mut()
                .unwrap();
            live.agent = Some(agent.clone());
            // Native discovery precedes the first model turn on bare interactive launches.
            // Any input or lifecycle event since activation cancels this one-time lease.
            if live.initial_prompt && live.generation == 0 {
                live.ready = Some(Instant::now());
                live.activity = Activity::Idle;
                live.readiness_source = "startup"; live.observed = Instant::now();
            }
            live.initial_prompt = false;
            self.shared.changed.notify_all();
            let mut result = registration_result(&agent, &run);
            result["capacity"] = state.store.capacity(&project)?;
            return Ok(result);
        }
        let actor = live
            .agent
            .clone()
            .ok_or_else(|| invalid_state("Register before using communication tools"))?;
        let recipients: Vec<&str> = match &request.operation {
            Operation::AgentSend { to, .. } => vec![to.as_str()],
            Operation::TaskAssign { to, reviewer, .. } => {
                let mut recipients = vec![to.as_str()];
                if let Some(reviewer) = reviewer {
                    recipients.push(reviewer.as_str());
                }
                recipients
            }
            _ => vec![],
        };
        for recipient in recipients {
            if recipient == actor.id || recipient == actor.name {
                continue;
            }
            let live_match = state
                .terminals
                .values()
                .filter_map(|t| t.live.as_ref())
                .any(|live| {
                    live.agent.as_ref().is_some_and(|agent| {
                        state.store.authorize(agent).is_ok() && agent.project == actor.project
                            && (agent.id == recipient || agent.name == recipient)
                    })
                });
            if live_match {
                continue;
            }
            // Offline teammates remain addressable while their program stays enabled.
            let known = state.store.agent(&actor.project, recipient).ok();
            let allowed = known.as_ref().is_some_and(|agent| {
                state.store.authorize(agent).is_ok() && state
                    .programs
                    .as_ref()
                    .is_none_or(|programs| programs.contains(&agent.program))
            });
            ensure!(
                allowed,
                scope_denied("Recipient is not a known enabled agent in this project")
            );
        }
        if matches!(request.operation, Operation::AgentReady) {
            let executing = state.store.execution_in_run(&actor.id, &run)?;
            let live = state
                .terminals
                .get_mut(&request.terminal)
                .unwrap()
                .live
                .as_mut()
                .unwrap();
            if executing {
                live.ready = None;
                return Ok(json!({"ready":false, "activity":live.activity,
                    "instruction":"Resolve the task you started: submit its result, report failure or confirm requested cancellation before announcing readiness."}));
            }
            if live.native_activity.is_some_and(|activity| !activity.is_idle()) || live.blocked {
                return Ok(json!({"ready": false, "activity": live.activity, "instruction": "Finish this turn now. Native session completion will establish readiness after work and approval dialogs end."}));
            }
            live.ready = Some(Instant::now());
            live.activity = Activity::Idle;
            live.readiness_source = "model"; live.observed = Instant::now();
            live.generation += 1;
            if let Some(wake) = live.wake.take() {
                live.delivered.remove(&wake.message_id);
            }
            return Ok(
                json!({"ready": true, "instruction": "Finish this turn now. Warp will submit a new inbox notification when peer work arrives."}),
            );
        }
        if matches!(request.operation, Operation::AgentWait) {
            let deadline = Instant::now() + WAIT;
            loop {
                ensure!(
                    !self.shared.stopped.load(Ordering::Acquire),
                    coordinator_unavailable("Broker stopped")
                );
                authenticate(&state, request, false)?;
                if let Some(message) = state.store.next_work(&actor, &run)? {
                    state
                        .terminals
                        .get_mut(&request.terminal)
                        .unwrap()
                        .live
                        .as_mut()
                        .unwrap()
                        .waiting = false;
                    return Ok(json!({"message": message, "idle": false}));
                }
                if Instant::now() >= deadline {
                    state
                        .terminals
                        .get_mut(&request.terminal)
                        .unwrap()
                        .live
                        .as_mut()
                        .unwrap()
                        .waiting = false;
                    return Ok(
                        json!({"message": null, "idle": true, "instruction": "Continue waiting until the user stops collaboration."}),
                    );
                }
                state
                    .terminals
                    .get_mut(&request.terminal)
                    .unwrap()
                    .live
                    .as_mut()
                    .unwrap()
                    .waiting = true;
                let (next, _) = self
                    .shared
                    .changed
                    .wait_timeout(state, deadline.saturating_duration_since(Instant::now()))
                    .map_err(|_| coordinator_unavailable("Broker unavailable"))?;
                state = next;
            }
        }
        let replayed = match &request.operation {
            Operation::TaskStart { request_id, .. }
            | Operation::TaskSubmit { request_id, .. }
            | Operation::TaskFinishCancel { request_id, .. }
            | Operation::TaskFail { request_id, .. } => state.store.request_seen(&actor.id, request_id)?,
            _ => false,
        };
        let mut result = state.store.execute(&actor, &run, &request.operation)?;
        // A replay returns its historical response; it must not rewrite current lifecycle state.
        let current_transition = match &request.operation {
            Operation::TaskStart { task_id, .. }
            | Operation::TaskSubmit { task_id, .. }
            | Operation::TaskFinishCancel { task_id, .. }
            | Operation::TaskFail { task_id, .. } => {
                let current = state.store.task(&actor, task_id)?;
                !replayed && result["version"].as_u64() == Some(current.version)
                    && result["state"].as_str() == Some(current.state.as_str())
            }
            _ => false,
        };
        if current_transition && matches!(request.operation, Operation::TaskStart { .. }) {
            let live = state.terminals.get_mut(&request.terminal).unwrap().live.as_mut().unwrap();
            live.activity = Activity::Working;
            live.readiness_source = "task_start"; live.observed = Instant::now();
            live.ready = None;
            live.generation += 1;
            if let Some(wake) = live.wake.take() { live.delivered.remove(&wake.message_id); }
        }
        if current_transition && matches!(request.operation, Operation::TaskSubmit { .. } | Operation::TaskFinishCancel { .. } | Operation::TaskFail { .. }) {
            let live = state.terminals.get_mut(&request.terminal).unwrap().live.as_mut().unwrap();
            live.activity = Activity::Idle;
            live.readiness_source = "task_finished"; live.observed = Instant::now();
            live.ready = Some(Instant::now());
            result["instruction"] = json!("Task execution has finished. Before ending your turn, call warp_agent_ready as your final tool action; native completion also restores readiness. Keep polling any other work you are coordinating.");
        }
        if matches!(request.operation, Operation::AgentList) {
            result.as_array_mut().unwrap().retain(|agent| {
                if agent["id"].as_str() == Some(actor.id.as_str()) {
                    return false;
                }
                state
                    .terminals
                    .values()
                    .filter_map(|t| t.live.as_ref())
                    .any(|live| {
                        live.agent
                            .as_ref()
                            .is_some_and(|a| Some(a.id.as_str()) == agent["id"].as_str())
                    })
            });
            for agent in result.as_array_mut().unwrap() {
                let online = state
                    .terminals
                    .values()
                    .filter_map(|t| t.live.as_ref())
                    .find(|live| {
                        live.agent
                            .as_ref()
                            .is_some_and(|a| Some(a.id.as_str()) == agent["id"].as_str())
                    });
                agent["online"] = json!(online.is_some());
                agent["waiting"] = json!(online.is_some_and(|live| live.waiting));
                if let Some(live) = online {
                    agent["ready"] = json!(live.ready.is_some());
                    agent["activity"] = json!(live.activity);
                    agent["native_activity"] = json!(live.native_activity);
                    agent["readiness_source"] = json!(live.readiness_source);
                    agent["paused"] = json!(live.paused);
                    agent["has_draft"] = json!(!live.draft.is_empty() || live.rich_draft);
                    agent["draft_state"] = json!(if live.rich_draft { "present" } else { live.draft.state() });
                    let mut blockers = Vec::new();
                    if live.ready.is_none() { blockers.push("lifecycle_not_idle"); }
                    if live.native_activity.is_some_and(|activity| !activity.is_idle()) { blockers.push("native_not_idle"); }
                    if live.paused { blockers.push("cancelled"); }
                    if !live.draft.is_empty() || live.rich_draft { blockers.push("draft"); }
                    if live.blocked { blockers.push("permission_or_question"); }
                    if live.waiting { blockers.push("cooperative_wait"); }
                    if live.wake.is_some() { blockers.push("dispatching"); }
                    if live.ready.is_some_and(|ready| ready.elapsed() < Duration::from_millis(750)) || live.last_output.elapsed() < Duration::from_millis(500) || live.last_input.elapsed() < Duration::from_millis(500) { blockers.push("settling"); }
                    if live.expired || live.started.elapsed() >= MUTATION_EPOCH { blockers.push("run_expired"); }
                    agent["can_auto_submit"] = json!(blockers.is_empty());
                    agent["delivery_blockers"] = json!(blockers);
                }
                let mut pending = 0;
                if let Some(peer) = online.and_then(|live| live.agent.as_ref()) {
                    pending = state.store.inbox_count(&peer.id)? as usize;
                }
                agent["pending_count"] = json!(pending);
                let mut tasks = state
                    .store
                    .task_states(&actor, agent["id"].as_str().unwrap())?;
                for task in &mut tasks {
                    let stored = state.store.task(&actor, task["id"].as_str().unwrap())?;
                    task["interrupted"] = json!(task_runtime(&state, &stored).1);
                }
                agent["can_start_task"] = json!(agent["can_auto_submit"] == true && !state.store.running_task(agent["id"].as_str().unwrap())?);
                agent["tasks"] = json!(tasks);
            }
        }
        if let Operation::TaskGet { task_id } = &request.operation {
            let task = state.store.task(&actor, task_id)?;
            let (online, interrupted) = task_runtime(&state, &task);
            result["assignee_online"] = json!(online);
            result["interrupted"] = json!(interrupted);
        }
        if matches!(
            request.operation,
            Operation::AgentSend { .. } | Operation::TaskAssign { .. }
        ) {
            result["delivery"] = json!("queued");
            result["instruction"] = json!("Work is queued, not completed or acknowledged. The receiver is notified and will process it once ready. By default, monitor every task you delegated: poll warp_agent_list for peer task states, use warp_task_get for results, and warp_agent_wait between polls. Keep coordinating until all delegated tasks are reviewed or the user stops. For ordinary instructions, wait for the recipient's reply before reporting completion.");
        }
        self.shared.changed.notify_all();
        Ok(result)
    }
    fn store(&self) -> Result<std::sync::MutexGuard<'_, State>> {
        let mut state = self.shared.state.lock()
            .map_err(|_| coordinator_unavailable("Broker unavailable"))?;
        state.expire_remote_presence()?;
        Ok(state)
    }
    /// Trusted local panel mutations under the deterministic operator principal.
    pub fn operator(&self, project: &str, operation: &Operation) -> Result<Value> {
        let state = self.store()?;
        let result = state
            .store
            .execute(&Store::operator(project), crate::storage::OPERATOR_EPOCH, operation);
        self.shared.changed.notify_all();
        result
    }
    /// Trusted explicit file-view intent; remote references never resolve on this device.
    pub fn local_evidence_file(&self, project: &str, evidence_id: &str) -> Result<std::path::PathBuf> {
        self.store()?.store.local_evidence_file(project, evidence_id)
    }

    /// Space, workspace, evidence, archive and history control operations.
    pub fn control(&self, project: &str, operation: &ControllerOperation) -> Result<Value> {
        let mut state = self.store()?;
        let result = state.store.execute_controller(project, operation)?;
        // Removing an admission is permanent for this capability, even if metadata is later restored.
        let revoked: Vec<_> = state.terminals.iter().filter_map(|(terminal, binding)| {
            let invalid_workspace = binding.workspace.as_ref().is_some_and(|workspace| state.store.authorize_workspace(workspace).is_err());
            let invalid_actor = binding.live.as_ref().and_then(|live| live.agent.as_ref()).is_some_and(|actor| state.store.authorize(actor).is_err());
            (invalid_workspace || invalid_actor).then(|| terminal.clone())
        }).collect();
        for terminal in revoked {
            let binding = state.terminals.get_mut(&terminal).unwrap();
            binding.revoked = true;
            binding.live = None;
        }
        self.shared.changed.notify_all();
        Ok(result)
    }
    /// Panel reads span the whole project, unlike caller-scoped agent tools.
    pub fn operator_agents(&self, project: &str) -> Result<Value> {
        let state = self.store()?;
        Ok(json!(state.store.agents(project)?))
    }
    pub fn operator_tasks(
        &self,
        project: &str,
        task_state: Option<&str>,
        assignee: Option<&str>,
        cursor: Option<u64>,
        limit: Option<u32>,
        include_archived: bool,
    ) -> Result<Value> {
        let state = self.store()?;
        state.store.operator_tasks(
            project,
            task_state,
            assignee,
            cursor,
            limit,
            include_archived,
        )
    }
    pub fn operator_task(&self, project: &str, task_id: &str) -> Result<Value> {
        let state = self.store()?;
        Ok(json!(state.store.operator_task(project, task_id)?))
    }
    pub fn operator_events(&self, project: &str, after: Option<u64>, limit: Option<u32>) -> Result<Value> {
        let state = self.store()?;
        state.store.events(project, after, limit)
    }

    /// Bounded read projection and resumable events, off the UI thread. A timeout also
    /// refreshes volatile readiness, which deliberately does not enter durable history.
    pub fn operator_panel(&self, query: &PanelQuery) -> Result<Value> {
        let mut state = self.store()?;
        let binding = query.terminal.as_ref().and_then(|terminal| state.terminals.get(terminal));
        let admission = match binding {
            Some(binding) if binding.revoked => "revoked",
            Some(binding) if binding.workspace.as_ref().is_some_and(|workspace| workspace.root != query.project) => "directory_mismatch",
            Some(binding) if binding.workspace.as_ref().is_some_and(|workspace| state.store.authorize_workspace(workspace).is_err()) => "revoked",
            Some(binding) if binding.workspace.is_some() => "shared",
            _ => "private",
        };
        let project = binding.and_then(|binding| binding.workspace.as_ref())
            .filter(|workspace| workspace.root == query.project)
            .map(crate::WorkspaceBinding::domain)
            .or_else(|| query.terminal.as_ref()
            .and_then(|terminal| state.terminals.get(terminal))
            .filter(|binding| !binding.revoked)
            .and_then(|binding| binding.live.as_ref())
            .and_then(|live| live.agent.as_ref())
            .filter(|agent| state.store.physical_root(agent).ok().as_deref() == Some(query.project.as_str()))
            .map(|agent| agent.project.clone()))
            .unwrap_or_else(|| query.project.clone());
        let same_scope = query.scope.as_deref() == Some(project.as_str());
        let after = same_scope.then_some(query.event_after).flatten();
        let mut events = state.store.events(&project, after, Some(50))?;
        if query.wait && same_scope && events["events"].as_array().is_some_and(Vec::is_empty) {
            let (next, _) = self.shared.changed.wait_timeout(state, Duration::from_secs(1))
                .map_err(|_| coordinator_unavailable("Broker unavailable"))?;
            state = next;
            events = state.store.events(&project, after, Some(50))?;
        }
        ensure!(!self.shared.stopped.load(Ordering::SeqCst), coordinator_unavailable("Broker unavailable"));
        let agents = state.store.agents(&project)?;
        let remaining: Vec<_> = agents.into_iter()
            .filter(|agent| !same_scope || query.agent_after.as_ref().is_none_or(|after| &agent.name > after))
            .collect();
        let agent_cursor = (remaining.len() > 50).then(|| remaining[49].name.clone());
        let agents: Vec<_> = remaining.into_iter().take(50).map(|agent| {
            let live = state.terminals.values().filter(|binding| !binding.revoked)
                .filter_map(|binding| binding.live.as_ref())
                .find(|live| !live.expired && live.started.elapsed() < MUTATION_EPOCH
                    && live.agent.as_ref().is_some_and(|actor| actor.id == agent.id));
            let remote_online = state.remote_active && state.remote_presence.get(&agent.id)
                .is_some_and(|presence| presence.valid(&state.store, &agent.id));
            let device = agent.terminal.strip_prefix("remote:")
                .and_then(|qualified| qualified.split_once(':')).map(|(device, _)| device.to_owned())
                .unwrap_or_else(|| "local".into());
            let workspace = state.store.physical_root(&agent).ok();
            let observed = live.map(|live| live.observed.elapsed().as_millis() as u64)
                .or_else(|| remote_online.then(|| state.remote_presence[&agent.id].age_ms()));
            json!({"agent": agent, "online": live.is_some() || remote_online,
                "device": device, "workspace": workspace, "last_observed_ms": observed,
                "observation_source": if live.is_some() { Some("local observation") } else if remote_online { Some("presence receipt") } else { None },
                "activity": live.map(|live| live.activity),
                "draft": live.map(|live| if live.rich_draft { "present" } else { live.draft.state() }),
                "blocked": live.is_some_and(|live| live.blocked),
                "paused": live.is_some_and(|live| live.paused),
                "ready": live.is_some_and(|live| live.ready.is_some()),
                "readiness_source": live.map(|live| live.readiness_source)})
        }).collect();
        let tasks = state.store.operator_tasks(&project, query.task_state.as_deref(), query.task_assignee.as_deref(), same_scope.then_some(query.task_after).flatten(), Some(50), query.include_archived)?;
        let task = query.selected_task.as_ref().filter(|_| same_scope).map(|id| state.store.operator_task(&project, id))
            .transpose()?;
        let runtime = task.as_ref().map(|task| {
            let (online, interrupted) = task_runtime(&state, task);
            json!({"online": online, "interrupted": interrupted})
        });
        let messages = if let Some(thread) = query.selected_thread.as_ref().filter(|_| same_scope) {
            state.store.execute(&Store::operator(&project), crate::storage::OPERATOR_EPOCH, &Operation::ThreadGet {
                thread_id: thread.clone(), cursor: query.message_after, limit: Some(50),
            })?
        } else if let Some(query_text) = &query.message_query {
            state.store.execute(&Store::operator(&project), crate::storage::OPERATOR_EPOCH, &Operation::MessageSearch {
                query: query_text.clone(), task_id: None, thread_id: None,
                cursor: same_scope.then_some(query.message_after).flatten(), limit: Some(50),
            })?
        } else { json!({"messages": [], "cursor": null}) };
        let reservations = state.store.operator_reservations(&project, &query.project, None,
            same_scope.then_some(query.reservation_after).flatten(), Some(50), true)?;
        let spaces = if query.spaces {
            state.store.execute_controller(&project, &ControllerOperation::SpaceList { cursor: query.space_after.clone(), limit: Some(50) })?
        } else { json!({"spaces": [], "cursor": null}) };
        let history = if query.history {
            let page = state.store.execute_controller(&project, &ControllerOperation::HistoryExport {
                after: same_scope.then_some(query.history_after).flatten(), limit: Some(50),
            })?;
            let preview = state.store.execute_controller(&project, &ControllerOperation::PurgePreview)?;
            Some(json!({"capacity": state.store.capacity(&project)?, "preview": preview,
                "records": page["records"], "cursor": page["cursor"]}))
        } else { None };
        Ok(json!({"project": project, "admission": admission, "agents": agents, "agent_cursor": agent_cursor,
            "tasks": tasks["tasks"], "task_cursor": tasks["cursor"],
            "task": task, "task_runtime": runtime, "events": events["events"],
            "event_cursor": events["cursor"], "history": history, "spaces": spaces["spaces"], "space_cursor": spaces["cursor"], "reservations": reservations["reservations"], "reservation_cursor": reservations["cursor"], "messages": messages["messages"], "message_cursor": messages["cursor"]}))
    }
}
fn registration_result(agent: &Agent, run: &str) -> Value {
    json!({"agent": agent, "run": run, "protocol_major": PROTOCOL_MAJOR,
        "protocol_minor": 0, "features": LOCAL_FEATURES})
}
fn dispatched_pools(state: &State) -> HashSet<String> {
    state.terminals.values()
        .filter_map(|binding| binding.live.as_ref())
        .filter(|live| !live.expired && live.started.elapsed() < MUTATION_EPOCH)
        .filter_map(|live| {
            let message = state.store.first_pending(live.agent.as_ref()?, false).ok()??;
            (message.kind == "available" && live.delivered.contains(&message.id))
                .then_some(message.task_id).flatten()
        }).collect()
}
fn task_runtime(state: &State, task: &Task) -> (bool, bool) {
    let live = state
        .terminals
        .values()
        .filter(|binding| !binding.revoked)
        .filter_map(|binding| binding.live.as_ref())
        .filter(|live| !live.expired && live.started.elapsed() < MUTATION_EPOCH)
        .find(|live| {
            live.agent
                .as_ref()
                .is_some_and(|agent| agent.id == task.assignee)
        });
    let remote = state.remote_presence.get(&task.assignee)
        .filter(|presence| state.remote_active && presence.valid(&state.store, &task.assignee));
    (
        live.is_some() || remote.is_some(),
        matches!(task.state.as_str(), "running" | "cancel_requested")
            && !live.is_some_and(|live| task.executing_run.as_deref() == Some(live.run.as_str()))
            && !remote.is_some_and(|presence| presence.executing(&state.store,
                &task.assignee, task.executing_run.as_deref())),
    )
}
fn authenticate<'a>(state: &'a State, request: &Request, registration: bool) -> Result<&'a Live> {
    ensure!(request.protocol_major == PROTOCOL_MAJOR,
        crate::domain("protocol_incompatible", "Install the matching Warpai application and communication companion", false, None));
    let binding = state
        .terminals
        .get(&request.terminal)
        .ok_or_else(|| unauthorized("Invalid terminal binding"))?;
    ensure!(!binding.revoked, scope_denied("Terminal participation was revoked"));
    if let Some(workspace) = &binding.workspace { state.store.authorize_workspace(workspace)?; }
    ensure!(
        binding.capability == request.capability,
        unauthorized("Invalid terminal capability")
    );
    let live = binding
        .live
        .as_ref()
        .ok_or_else(|| binding_inactive("No managed agent owns this terminal"))?;
    if !registration || request.run.is_some() {
        ensure!(
            request.run.as_deref() == Some(&live.run),
            unauthorized("Expired agent run; register again")
        );
    }
    if let Some(actor) = &live.agent { state.store.authorize(actor)?; }
    // Reads stay available; a fencing epoch rejects every stale mutation before it can commit.
    if request.operation.request_id().is_some() {
        ensure!(
            !live.expired && live.started.elapsed() < MUTATION_EPOCH,
            epoch_expired("Terminal mutation epoch expired; start a fresh managed CLI session")
        );
    }
    Ok(live)
}
pub(crate) async fn receive<T: serde::de::DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
    deadline: Instant,
) -> Result<T> {
    tokio::time::timeout_at(deadline.into(), async {
        let mut header = [0; 4];
        stream
            .read_exact(&mut header)
            .await
            .map_err(|_| anyhow!("Local IPC header read failed"))?;
        let length = u32::from_be_bytes(header) as usize;
        ensure!(
            length > 0 && length <= MAX_FRAME,
            "Invalid local IPC frame size"
        );
        let mut bytes = vec![0; length];
        stream
            .read_exact(&mut bytes)
            .await
            .map_err(|_| anyhow!("Local IPC frame read failed"))?;
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("Invalid local IPC payload"))
    })
    .await
    .map_err(|_| anyhow!("Local IPC read deadline exceeded"))?
}
pub(crate) async fn send<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &T,
    deadline: Instant,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(
        bytes.len() <= MAX_FRAME,
        "Local IPC response exceeds capacity; acknowledge old messages"
    );
    tokio::time::timeout_at(deadline.into(), async {
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(&bytes).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| anyhow!("Local IPC write deadline exceeded"))?
    .map_err(|_| anyhow!("Local IPC write failed"))
}
async fn handle(mut stream: impl AsyncRead + AsyncWrite + Unpin, broker: Broker) -> Result<()> {
    let request: Request = receive(&mut stream, Instant::now() + Duration::from_secs(5)).await?;
    let result = tokio::task::spawn_blocking(move || broker.execute(&request))
        .await
        .map_err(|_| anyhow!("Broker unavailable"))?;
    let response = match result {
        Ok(result) => Response {
            result: Some(result),
            error: None,
        },
        Err(error) => Response {
            result: None,
            error: Some(DomainError::from_error(error)),
        },
    };
    send(
        &mut stream,
        &response,
        Instant::now() + Duration::from_secs(5),
    )
    .await?;
    // Windows pipe close may discard queued data; wait for the client to confirm frame receipt.
    let mut acknowledged = [0];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut acknowledged)).await??;
    ensure!(acknowledged == [1], "Invalid frame acknowledgement");
    Ok(())
}
#[cfg(unix)]
pub(crate) async fn connect(endpoint: &str) -> std::io::Result<UnixStream> {
    UnixStream::connect(endpoint).await
}
#[cfg(windows)]
pub(crate) async fn connect(
    endpoint: &str,
) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    loop {
        match ClientOptions::new().open(endpoint) {
            // ERROR_PIPE_BUSY: an accepted pipe is being replaced by the next listening instance.
            Err(error) if error.raw_os_error() == Some(231) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            result => return result,
        }
    }
}
pub fn call(endpoint: &str, request: &Request) -> Result<Value> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut stream = tokio::time::timeout(Duration::from_secs(5), connect(endpoint))
                .await
                .map_err(|_| anyhow!("Local IPC connect deadline exceeded"))??;
            send(
                &mut stream,
                request,
                Instant::now() + Duration::from_secs(5),
            )
            .await?;
            let response: Response =
                receive(&mut stream, Instant::now() + WAIT + Duration::from_secs(5)).await?;
            tokio::time::timeout(Duration::from_secs(5), async {
                stream.write_all(&[1]).await?;
                stream.flush().await
            })
            .await
            .map_err(|_| anyhow!("Local IPC acknowledgement deadline exceeded"))??;
            if let Some(error) = response.error {
                return Err(anyhow::Error::from(error));
            }
            response
                .result
                .ok_or_else(|| anyhow!("Missing broker result"))
        })
}
