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
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use uuid::Uuid;
use crate::readiness::{Activity, Draft};

pub const ENDPOINT: &str = "WARP_AGENT_ENDPOINT";
pub const CAPABILITY: &str = "WARP_AGENT_CAPABILITY";
pub const TERMINAL: &str = "WARP_TERMINAL_SESSION_UUID";
const WAIT: Duration = Duration::from_secs(20);
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
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
            let listener = ServerOptions::new()
                .first_pipe_instance(true)
                .create(&endpoint)?;
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
            }),
            changed: Condvar::new(),
            stopped: AtomicBool::new(false),
            connections: AtomicUsize::new(0),
            shutdown: tokio::sync::Notify::new(),
        });
        let broker = Broker {
            shared: shared.clone(),
            endpoint,
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
                                let next = ServerOptions::new().create(&endpoint)?;
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
        if source.agent.is_none() {
            return vec![];
        }
        let mut peers: Vec<_> = state
            .terminals
            .iter()
            .filter_map(|(id, binding)| {
                let live = binding.live.as_ref()?;
                let agent = live.agent.as_ref()?;
                (id != terminal && live.project == source.project).then(|| Peer {
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
        state.terminals.insert(
            terminal.into(),
            Terminal {
                capability: capability.clone(),
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
        let binding = state
            .terminals
            .get_mut(terminal)
            .ok_or_else(|| unauthorized("Terminal is not bound"))?;
        binding.live = Some(Live {
            program: program.into(),
            project: project.into(),
            run: Uuid::new_v4().to_string(),
            agent: None,
            waiting: false,
            ready: None,
            last_output: Instant::now(),
            last_input: Instant::now(),
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
                live.readiness_source = "native";
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
                    live.readiness_source = "user_submit";
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
                    live.readiness_source = "user_input";
                    live.ready = None;
                }
                live.initial_prompt = false;
                live.generation += 1;
                live.last_input = Instant::now();
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
                    live.last_input = Instant::now();
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
                live.last_output = Instant::now();
            }
        }
    }
    pub fn wakeups(&self) -> Vec<Wake> {
        let Ok(state) = self.shared.state.lock() else {
            return Vec::new();
        };
        state
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
                Some(Wake {
                    terminal: terminal.clone(),
                    run: live.run.clone(),
                    message_id: message.id,
                    generation: live.generation,
                })
            })
            .collect()
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
                    .pending(live.agent.as_ref()?)
                    .ok()?
                    .into_iter()
                    .next()?;
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
        let Ok(pending) = state.store.pending(actor) else {
            return false;
        };
        if !pending.iter().any(|message| message.id == wake.message_id) {
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
                    live.run == wake.run && !live.paused && !live.expired && live.started.elapsed() < MUTATION_EPOCH && live.draft.is_empty() && !live.rich_draft && !live.blocked && live.wake.as_ref() == Some(wake)
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
                        live.readiness_source = "retry";
                        live.generation += 1;
                    }
                    else { live.ready = None; live.activity = Activity::Working; live.readiness_source = "peer_submit"; }
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
            let live = state
                .terminals
                .get_mut(&request.terminal)
                .unwrap()
                .live
                .as_mut()
                .unwrap();
            if live.agent.is_some() {
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
            live.readiness_source = "native";
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
                    return Ok(json!({"agent": agent, "run": run}));
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
            let agent = state
                .store
                .register(&request.terminal, &program, &project, &name)?;
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
                live.readiness_source = "startup";
            }
            live.initial_prompt = false;
            self.shared.changed.notify_all();
            return Ok(
                json!({"agent": agent, "run": run, "capacity": state.store.capacity(&project)?}),
            );
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
                        agent.project == actor.project
                            && (agent.id == recipient || agent.name == recipient)
                    })
                });
            if live_match {
                continue;
            }
            // Offline teammates remain addressable while their program stays enabled.
            let known = state.store.agent(&actor.project, recipient).ok();
            let allowed = known.as_ref().is_some_and(|agent| {
                state
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
            let live = state
                .terminals
                .get_mut(&request.terminal)
                .unwrap()
                .live
                .as_mut()
                .unwrap();
            if live.native_activity.is_some_and(|activity| !activity.is_idle()) || live.blocked {
                return Ok(json!({"ready": false, "activity": live.activity, "instruction": "Finish this turn now. Native session completion will establish readiness after work and approval dialogs end."}));
            }
            live.ready = Some(Instant::now());
            live.activity = Activity::Idle;
            live.readiness_source = "model";
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
        let mut result = state.store.execute(&actor, &run, &request.operation)?;
        if matches!(request.operation, Operation::TaskStart { .. }) {
            let live = state.terminals.get_mut(&request.terminal).unwrap().live.as_mut().unwrap();
            live.activity = Activity::Working;
            live.readiness_source = "task_start";
            live.ready = None;
            live.generation += 1;
            if let Some(wake) = live.wake.take() { live.delivered.remove(&wake.message_id); }
        }
        if matches!(request.operation, Operation::TaskSubmit { .. } | Operation::TaskFinishCancel { .. } | Operation::TaskFail { .. }) {
            let live = state.terminals.get_mut(&request.terminal).unwrap().live.as_mut().unwrap();
            live.activity = Activity::Idle;
            live.readiness_source = "task_finished";
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
                    pending = state.store.pending(peer)?.len();
                }
                agent["pending_count"] = json!(pending);
                let mut tasks = state
                    .store
                    .task_states(&actor, agent["id"].as_str().unwrap())?;
                for task in &mut tasks {
                    let stored = state.store.task(&actor, task["id"].as_str().unwrap())?;
                    task["interrupted"] = json!(task_runtime(&state, &stored).1);
                }
                agent["can_start_task"] = json!(agent["can_auto_submit"] == true && !tasks.iter().any(|task| matches!(task["state"].as_str(), Some("running" | "cancel_requested"))));
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
        self.shared
            .state
            .lock()
            .map_err(|_| coordinator_unavailable("Broker unavailable"))
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
    /// Space, workspace, evidence, archive and history control operations.
    pub fn control(&self, project: &str, operation: &ControllerOperation) -> Result<Value> {
        let state = self.store()?;
        state.store.execute_controller(project, operation)
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
}
fn task_runtime(state: &State, task: &Task) -> (bool, bool) {
    let live = state
        .terminals
        .values()
        .filter_map(|binding| binding.live.as_ref())
        .find(|live| {
            live.agent
                .as_ref()
                .is_some_and(|agent| agent.id == task.assignee)
        });
    (
        live.is_some(),
        matches!(task.state.as_str(), "running" | "cancel_requested")
            && !live.is_some_and(|live| task.executing_run.as_deref() == Some(live.run.as_str())),
    )
}
fn authenticate<'a>(state: &'a State, request: &Request, registration: bool) -> Result<&'a Live> {
    let binding = state
        .terminals
        .get(&request.terminal)
        .ok_or_else(|| unauthorized("Invalid terminal binding"))?;
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
