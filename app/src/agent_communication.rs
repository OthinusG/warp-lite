//! Local coordination for independently authenticated third-party CLI agents.
pub(crate) mod setup;
use crate::terminal::{
    cli_agent_sessions::{
        CLIAgentSessionStatus, CLIAgentSessionsModel, CLIAgentSessionsModelEvent,
    },
    CLIAgent, TerminalView,
};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use warp_agent_bus::{
    project_root,
    transport::{Broker, RunningBroker, CAPABILITY, ENDPOINT, TERMINAL},
};
use warpui::r#async::Timer;
use warpui::{Entity, EntityId, ModelContext, SingletonEntity, ViewHandle, WeakViewHandle};

static BROKER: OnceLock<Broker> = OnceLock::new();
static VIEWS: OnceLock<Mutex<HashMap<EntityId, (String, WeakViewHandle<TerminalView>)>>> =
    OnceLock::new();
pub(crate) struct AgentCommunication {
    _server: Option<RunningBroker>,
    pub(crate) preferences: setup::Preferences,
    pub(crate) available: Vec<setup::Available>,
    pub(crate) busy: bool,
    pub(crate) status: String,
    preferences_path: PathBuf,
    pending: Option<std::sync::mpsc::Receiver<(setup::Preferences, Vec<setup::Available>, String)>>,
    notified: HashMap<String, String>,
}
impl Entity for AgentCommunication {
    type Event = ();
}
impl SingletonEntity for AgentCommunication {}
impl AgentCommunication {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        let directory =
            warp_core::paths::secure_state_dir().unwrap_or_else(warp_core::paths::state_dir);
        let server = std::fs::create_dir_all(&directory)
            .ok()
            .and_then(|_| RunningBroker::start(&directory.join("agent-communication.sqlite")).ok());
        if let Some(server) = &server {
            server
                .broker
                .set_programs(Some(std::collections::HashSet::new()));
            let _ = BROKER.set(server.broker.clone());
        }
        VIEWS.get_or_init(|| Mutex::new(HashMap::new()));
        ctx.subscribe_to_model(&CLIAgentSessionsModel::handle(ctx), |model, event, ctx| {
            let Some(broker) = BROKER.get() else {
                return;
            };
            let Some(views) = VIEWS.get() else {
                return;
            };
            let Ok(views) = views.lock() else {
                return;
            };
            let Some((terminal, view)) = views.get(&event.terminal_view_id()) else {
                return;
            };
            match event {
                CLIAgentSessionsModelEvent::Started { agent, .. } => {
                    // Managed custom CLIs can participate through their verified native MCP client.
                    let program = if *agent == CLIAgent::Unknown {
                        "custom"
                    } else {
                        agent.command_prefix()
                    };
                    if !model.preferences.programs().contains(program) {
                        return;
                    }
                    if let Some(view) = view.upgrade(ctx) {
                        let terminal_model = view.as_ref(ctx).model.lock();
                        if let Some(directory) =
                            terminal_model.active_block_metadata().current_working_directory()
                        {
                            if let Ok(project) = project_root(Path::new(directory)) {
                                let command = terminal_model.block_list().active_block().command_to_string();
                                let command = command.trim();
                                let view = view.as_ref(ctx);
                                let unshadowed = shlex::split(command)
                                    .and_then(|words| words.first().cloned())
                                    .is_some_and(|program| view.active_block_session_id()
                                        .and_then(|session| view.sessions_model().as_ref(ctx).get(session))
                                        .is_some_and(|shell| shell.alias_value(&program).is_none()
                                            && shell.abbreviation_value(&program).is_none()
                                            && !shell.function_names().any(|name| name == program.as_str())));
                                let launch_options = model.preferences.selected.values()
                                    .find(|entry| entry.active && entry.program == program)
                                    .map(|entry| &entry.launch_options);
                                let initial_prompt = unshadowed && agent.accepts_peer_prompt(command) && (launch_options.is_some_and(|options| agent.starts_at_empty_prompt(command, options))
                                    || (*agent == CLIAgent::Unknown
                                        && model.preferences.selected.get(command)
                                            .is_some_and(|entry| entry.active)));
                                if broker.activate(terminal, program, &project, initial_prompt).is_err() {
                                    log::warn!("Could not activate local agent communication");
                                }
                            }
                        }
                    }
                }
                CLIAgentSessionsModelEvent::Ended { .. } => broker.end(terminal),
                CLIAgentSessionsModelEvent::StatusChanged { status, .. } => {
                    if matches!(status, CLIAgentSessionStatus::Blocked { .. }) {
                        broker.readiness(terminal, false);
                        return;
                    }
                    // Opaque OSC notifications also announce approvals; they cannot prove idle.
                    if view.upgrade(ctx).is_some_and(|view| {
                        CLIAgentSessionsModel::as_ref(ctx)
                            .session(view.id())
                            .is_some_and(|session| {
                                crate::terminal::cli_agent_sessions::listener::agent_supports_rich_status(&session.agent)
                            })
                    }) {
                        broker.readiness(terminal, matches!(status, CLIAgentSessionStatus::Success));
                    }
                }
                _ => {}
            }
        });
        if server.is_none() {
            log::warn!("Local agent communication is unavailable");
        }
        Self::schedule(ctx);
        let preferences_path = directory.join("agent-communication-settings.json");
        let preferences = match std::fs::read(&preferences_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => setup::Preferences::default(),
        };
        let mut model = Self {
            _server: server,
            preferences,
            available: vec![],
            busy: false,
            status: String::new(),
            preferences_path,
            pending: None,
            notified: HashMap::new(),
        };
        model.configure(None, None, ctx);
        model
    }
    fn policy(&self) {
        if let Some(broker) = BROKER.get() {
            let programs = self.preferences.programs();
            broker.set_programs(Some(programs));
        }
    }
    pub(crate) fn configure(
        &mut self,
        enabled: Option<bool>,
        command: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) {
        if self.busy {
            return;
        }
        let mut preferences = self.preferences.clone();
        if let Some(enabled) = enabled {
            preferences.enabled = enabled;
            // Revocation precedes disk IO and cannot be undone by a failed cleanup.
            if !enabled {
                self.preferences.enabled = false;
                self.policy();
            }
        }
        for (name, entry) in &mut preferences.selected {
            if !preferences.enabled || command.as_ref() == Some(name) {
                entry.active = false;
            }
        }
        self.preferences = preferences.clone();
        self.policy();
        self.busy = true;
        self.status = "Updating native MCP configuration…".into();
        let path = self.preferences_path.clone();
        let mut commands: Vec<(String, String)> = enum_iterator::all::<CLIAgent>()
            .flat_map(|agent| {
                agent
                    .command_prefixes()
                    .iter()
                    .map(move |command| (agent.command_prefix().to_owned(), (*command).to_owned()))
            })
            .collect();
        // Custom toolbar patterns can supply literal executable prefixes, without shell evaluation.
        for (pattern, vendor) in crate::settings::AISettings::as_ref(ctx)
            .cli_agent_footer_enabled_commands
            .iter()
        {
            let command = pattern
                .trim_start_matches('^')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('$');
            if !command.is_empty()
                && command
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            {
                let program =
                    serde_json::from_value::<CLIAgent>(serde_json::Value::String(vendor.clone()))
                        .ok()
                        .filter(|agent| *agent != CLIAgent::Unknown)
                        .map(|agent| agent.command_prefix().to_owned())
                        .unwrap_or_else(|| "custom".into());
                if !commands.iter().any(|(_, known)| known == command) {
                    commands.push((program, command.to_owned()));
                }
            }
        }
        let launcher_directory = self._server.as_ref().map(|server| server.launcher_directory());
        let search_paths: Vec<PathBuf> = VIEWS
            .get()
            .and_then(|views| views.lock().ok())
            .map(|views| {
                views
                    .values()
                    .filter_map(|(_, view)| view.upgrade(ctx))
                    .flat_map(|view| {
                        let view = view.as_ref(ctx);
                        let Some(session) = view.active_block_session_id() else {
                            return vec![];
                        };
                        if !view.session_is_local(session, ctx) {
                            return vec![];
                        }
                        let sessions = view.sessions_model().as_ref(ctx);
                        let shell = sessions.get(session);
                        sessions
                            .env_var_for_session(session, "PATH")
                            .into_iter()
                            .chain(shell.as_ref().and_then(|session| session.path().as_deref()))
                            .flat_map(|path| std::env::split_paths(std::ffi::OsStr::new(path)))
                            .filter(|path| launcher_directory.as_ref() != Some(path))
                            .collect::<Vec<_>>()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let (sender, receiver) = std::sync::mpsc::channel();
        commands.sort_by_key(|(_, command)| !preferences.selected.contains_key(command));
        self.pending = Some(receiver);
        std::thread::spawn(move || {
            let mut errors = Vec::new();
            let mut available = if preferences.enabled {
                match setup::companion() {
                    Ok(bridge) => setup::discover(commands, &bridge, search_paths),
                    Err(_) => {
                        errors.push("Bundled communication bridge is missing".to_owned());
                        vec![]
                    }
                }
            } else {
                vec![]
            };
            if setup::save_preferences(&path, &preferences).is_err() {
                preferences.enabled = false;
                let _ = sender.send((
                    preferences,
                    available,
                    "Could not save communication preferences; communication remains disabled"
                        .into(),
                ));
                return;
            }
            let was_selected = command
                .as_ref()
                .is_some_and(|name| preferences.selected.contains_key(name));
            let remove: Vec<String> = if !preferences.enabled {
                preferences.selected.keys().cloned().collect()
            } else {
                preferences
                    .selected
                    .iter()
                    .filter(|(name, entry)| !entry.active || command.as_ref() == Some(*name))
                    .map(|(name, _)| name.clone())
                    .collect()
            };
            for name in remove {
                let entry = &preferences.selected[&name];
                if setup::configure(entry, false, true).is_ok() {
                    preferences.selected.remove(&name);
                } else {
                    errors.push(format!("{name}: cleanup failed; configuration was preserved. Retry by unchecking or disabling communication."));
                }
            }
            if preferences.enabled {
                if let Some(name) = &command {
                    if !was_selected
                        && !errors
                            .iter()
                            .any(|error| error.starts_with(&format!("{name}:")))
                    {
                        if let Some(entry) = available
                            .iter()
                            .find(|row| &row.command == name)
                            .and_then(|row| row.installed.clone())
                        {
                            if setup::configure(&entry, false, false).is_ok() {
                                let mut pending_entry = entry.clone();
                                pending_entry.active = false;
                                preferences.selected.insert(name.clone(), pending_entry);
                                // Persist cleanup intent before invoking a vendor command or replacing its file.
                                if setup::save_preferences(&path, &preferences).is_ok()
                                    && setup::configure(&entry, true, false).is_ok()
                                {
                                    preferences.selected.insert(name.clone(), entry);
                                } else {
                                    errors.push(format!(
                                        "{name}: setup failed; cleanup remains pending"
                                    ));
                                }
                            } else {
                                errors.push(format!(
                                    "{name}: setup failed; existing configuration was preserved"
                                ));
                            }
                        }
                    }
                }
            }
            for row in &mut available {
                if let (Some(selected), Some(discovered)) = (
                    preferences.selected.get_mut(&row.command), row.installed.as_ref(),
                ) {
                    selected.launch_options = discovered.launch_options.clone();
                }
                if preferences.selected.contains_key(&row.command) {
                    row.status = if preferences.selected[&row.command].active {
                        if matches!(preferences.selected[&row.command].adapter, setup::Adapter::Vibe(_)) {
                            "Configured — open a new terminal pane, then start Vibe to load its native MCP environment"
                        } else {
                            "Configured — restart this agent to load the bridge"
                        }
                    } else {
                        "Disabled — cleanup pending; uncheck or retry cleanup"
                    }
                    .into();
                }
            }
            if setup::save_preferences(&path, &preferences).is_err() {
                preferences.enabled = false;
                errors.push(
                    "Could not save communication preferences; communication remains disabled"
                        .into(),
                );
            }
            let status = if errors.is_empty() {
                if !preferences.enabled {
                    "Communication is off.".into()
                } else if preferences.selected.is_empty() {
                    "Select installed agents to configure communication.".into()
                } else {
                    "Restart running agents to reload MCP configuration. Agents in the same project communicate automatically.".into()
                }
            } else {
                errors.join("\n")
            };
            let _ = sender.send((preferences, available, status));
        });
        ctx.notify();
    }
    fn schedule(ctx: &mut ModelContext<Self>) {
        ctx.spawn(Timer::after(Duration::from_millis(250)), |model, _, ctx| {
            if let Some(result) = model
                .pending
                .as_ref()
                .and_then(|receiver| receiver.try_recv().ok())
            {
                model.preferences = result.0;
                model.available = result.1;
                model.status = result.2;
                model.busy = false;
                model.pending = None;
                model.policy();
                ctx.notify();
            }
            if let (Some(broker), Some(views)) = (BROKER.get(), VIEWS.get()) {
                // ponytail: scan registered terminals; event-driven delivery if many panes make this measurable.
                let bindings: Vec<_> = views
                    .lock()
                    .map(|mut views| {
                        views.retain(|_, (terminal, view)| {
                            let alive = view.upgrade(ctx).is_some();
                            if !alive {
                                broker.end(terminal);
                            }
                            alive
                        });
                        views.values().cloned().collect()
                    })
                    .unwrap_or_default();
                let pending = broker.pending_work();
                model.notified.retain(|terminal, message| {
                    pending.iter().any(|work| &work.terminal == terminal && &work.message_id == message)
                });
                for work in pending {
                    if model.notified.get(&work.terminal) == Some(&work.message_id) {
                        continue;
                    }
                    if let Some((_, view)) = bindings.iter().find(|(terminal, _)| terminal == &work.terminal) {
                        if let Some(view) = view.upgrade(ctx) {
                            view.update(ctx, |_, ctx| {
                                ctx.emit(crate::terminal::view::Event::ShowToast {
                                    message: "Peer work received. It will start when this agent is ready; drafts and permission requests are preserved.".into(),
                                    flavor: crate::view_components::ToastFlavor::Default,
                                });
                            });
                            model.notified.insert(work.terminal, work.message_id);
                        }
                    }
                }
                for wake in broker.wakeups() {
                    if let Some((_, view)) = bindings
                        .iter()
                        .find(|(terminal, _)| terminal == &wake.terminal)
                    {
                        if let Some(view) = view.upgrade(ctx) {
                            let broker = broker.clone();
                            view.update(ctx, |view, ctx| {
                                view.wake_for_peer_work(broker, wake, ctx)
                            });
                        }
                    }
                }
            }
            Self::schedule(ctx);
        });
    }
}
/// Tests and remote terminals without the native singleton receive no capabilities.
pub(crate) fn prepare(
    env: &mut HashMap<OsString, OsString>,
    directory: Option<&Path>,
    ctx: &warpui::AppContext,
) -> Option<String> {
    let broker = BROKER.get()?;
    let terminal = env.get(&OsString::from(TERMINAL))?.to_str()?.to_owned();
    let capability = broker.prepare(&terminal).ok()?;
    env.insert(ENDPOINT.into(), broker.endpoint.clone().into());
    env.insert(CAPABILITY.into(), capability.into());
    let settings = AgentCommunication::as_ref(ctx);
    if settings.preferences.enabled {
        if let Some(installed) = settings
            .preferences
            .selected
            .values()
            .find(|entry| entry.active && matches!(entry.adapter, setup::Adapter::Vibe(_)))
        {
            if setup::vibe_environment(installed, env, directory).is_err() {
                log::warn!("Could not prepare native Vibe communication environment; existing settings were preserved");
            }
        }
    }
    if let Ok(executable) = std::env::current_exe() {
        let companion = executable.with_file_name(if cfg!(windows) {
            "warp-agent.exe"
        } else {
            "warp-agent"
        });
        env.insert("WARP_AGENT_BIN".into(), companion.clone().into_os_string());
        if settings.preferences.enabled {
            let launches: std::collections::BTreeMap<_, _> = settings.preferences.selected.iter()
                .filter(|(_, entry)| entry.active)
                .map(|(command, entry)| (command.clone(), warp_agent_bus::session::NativeLaunch {
                    executable: entry.executable.clone(), program: entry.program.clone(),
                    options: if entry.program == "codex" { entry.launch_options.clone() } else { Default::default() },
                }))
                .collect();
            if let Some(server) = &settings._server {
                let directory = server.launcher_directory();
                if !launches.is_empty() && warp_agent_bus::session::install_launchers(&directory, &companion, &launches).is_ok() {
                        let inherited = env.get(&OsString::from("PATH")).cloned().or_else(|| std::env::var_os("PATH")).unwrap_or_default();
                        let paths = std::iter::once(directory.clone()).chain(std::env::split_paths(&inherited));
                        if let (Ok(path), Ok(launches)) = (std::env::join_paths(paths), serde_json::to_string(&launches)) {
                            env.insert("WARP_AGENT_LAUNCH_PATH".into(), directory.clone().into_os_string());
                            env.insert("PATH".into(), path);
                            env.insert(warp_agent_bus::session::LAUNCHES.into(), launches.into());
                        }
                }
            }
        }
    }
    Some(terminal)
}
pub(crate) fn accepts_peer_prompt(agent: &CLIAgent, command: &str, ctx: &warpui::AppContext) -> bool {
    if !agent.accepts_peer_prompt(command) { return false; }
    if *agent != CLIAgent::Codex { return true; }
    let Some(words) = shlex::split(command) else { return false; };
    AgentCommunication::as_ref(ctx).preferences.selected.values()
        .find(|entry| entry.active && entry.program == "codex")
        .is_some_and(|entry| warp_agent_bus::session::codex_accepts_peer_prompt(&words[1..], &entry.launch_options))
}

pub(crate) fn bind(view: &ViewHandle<TerminalView>, terminal: Option<String>) {
    if let (Some(terminal), Some(views)) = (terminal, VIEWS.get()) {
        if let Ok(mut views) = views.lock() {
            views.insert(view.id(), (terminal, view.downgrade()));
        }
    }
}

fn terminal_for_view(view: EntityId) -> Option<String> {
    VIEWS
        .get()?
        .lock()
        .ok()?
        .get(&view)
        .map(|(terminal, _)| terminal.clone())
}
pub(crate) fn user_input(view: EntityId, bytes: &[u8]) {
    if let (Some(broker), Some(terminal)) = (BROKER.get(), terminal_for_view(view)) {
        // A submit clears a draft. Cancellation keeps automatic work paused until the user resumes.
        let boundary = matches!(bytes, b"\r" | b"\n") || bytes.ends_with(b"\r");
        broker.user_input(&terminal, boundary);
    }
}
pub(crate) fn output(view: EntityId) {
    if let (Some(broker), Some(terminal)) = (BROKER.get(), terminal_for_view(view)) {
        broker.output(&terminal);
    }
}

pub(crate) fn readiness(view: EntityId, ready: bool) {
    if let (Some(broker), Some(terminal)) = (BROKER.get(), terminal_for_view(view)) {
        broker.readiness(&terminal, ready);
    }
}
