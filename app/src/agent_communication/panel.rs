//! Native collaboration projection; explicit sample mode retains the accepted fixtures.
use crate::appearance::Appearance;
use crate::ui_components::blended_colors;
#[path = "panel_controls.rs"]
mod controls;
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use warp_agent_bus::{
    companion::TaskCommand,
    ssh_remote::{HostClient, SshProfile},
};
use warp_agent_bus::{transport::PanelQuery, Agent, Event, Message, Reservation, Task};
use warpui::r#async::Timer;
use warpui::{
    accessibility::{AccessibilityContent, ActionAccessibilityContent, WarpA11yRole},
    elements::{
        ClippedScrollStateHandle, ClippedScrollable, Container, DispatchEventResult, Element,
        EventHandler, Fill, Flex, MouseStateHandle, Padding, ParentElement, ScrollbarWidth,
        Shrinkable, Wrap,
    },
    fonts::Weight,
    ui_components::{
        button::ButtonVariant,
        components::{UiComponent, UiComponentStyles},
    },
    units::IntoPixels,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

/// Shared panel spacing so every row, section and button run keeps one rhythm.
const GAP_TIGHT: f32 = 4.;
const GAP_ROW: f32 = 8.;
const GAP_SECTION: f32 = 12.;

// Text roles: the 14px semibold primary title outranks semibold section headings;
// 12px secondary text carries status and guidance.
fn panel_title(appearance: &Appearance, text: &'static str) -> Box<dyn Element> {
    let theme = appearance.theme();
    appearance
        .ui_builder()
        .span(text)
        .with_style(UiComponentStyles {
            font_size: Some(14.),
            font_weight: Some(Weight::Semibold),
            font_color: Some(theme.active_ui_text_color().into()),
            ..Default::default()
        })
        .with_soft_wrap()
        .build()
        .finish()
}

fn heading(appearance: &Appearance, text: impl Into<Cow<'static, str>>) -> Box<dyn Element> {
    appearance
        .ui_builder()
        .span(text)
        .with_style(UiComponentStyles {
            font_weight: Some(Weight::Semibold),
            ..Default::default()
        })
        .with_soft_wrap()
        .build()
        .finish()
}

fn note(appearance: &Appearance, text: impl Into<Cow<'static, str>>) -> Box<dyn Element> {
    let theme = appearance.theme();
    appearance
        .ui_builder()
        .span(text)
        .with_style(UiComponentStyles {
            font_size: Some(12.),
            font_color: Some(blended_colors::text_sub(theme, theme.background())),
            ..Default::default()
        })
        .with_soft_wrap()
        .build()
        .finish()
}

fn detail(appearance: &Appearance, text: impl Into<Cow<'static, str>>) -> Box<dyn Element> {
    appearance
        .ui_builder()
        .span(text)
        .with_soft_wrap()
        .with_selectable(true)
        .build()
        .finish()
}

/// Lays related buttons out in wrapping runs instead of one full-width row each.
fn button_row(buttons: Vec<Box<dyn Element>>) -> Option<Box<dyn Element>> {
    (!buttons.is_empty()).then(|| {
        Wrap::row()
            .with_spacing(GAP_SECTION)
            .with_run_spacing(GAP_TIGHT)
            .with_children(buttons)
            .finish()
    })
}

#[derive(Deserialize)]
struct Fixture {
    state: String,
    guidance: String,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
struct Section {
    title: String,
    rows: Vec<String>,
}

#[derive(Deserialize)]
struct PanelAgent {
    agent: Agent,
    #[serde(default)]
    run: Option<String>,
    online: bool,
    activity: Option<warp_agent_bus::readiness::Activity>,
    draft: Option<String>,
    blocked: bool,
    paused: bool,
    ready: bool,
    readiness_source: Option<String>,
    #[serde(default)]
    device: Option<String>,
    #[serde(default)]
    workspace: Option<String>,
    #[serde(default)]
    last_observed_ms: Option<u64>,
    #[serde(default)]
    observation_source: Option<String>,
    #[serde(default)]
    delivery_phase: Option<String>,
    #[serde(default)]
    delivery_retained: Option<bool>,
}
#[derive(Deserialize)]
struct PanelTask {
    id: String,
    state: String,
    revision: u32,
    version: u64,
    assignee: String,
}
#[derive(Deserialize)]
struct TaskRuntime {
    online: bool,
    interrupted: bool,
    #[serde(default)]
    delivery_phase: Option<String>,
    #[serde(default)]
    delivery_retained: Option<bool>,
}
#[derive(Clone, Deserialize)]
struct WorkspacePreview {
    id: String,
    space_id: String,
    root: String,
    repository_id: Option<String>,
    model: String,
    branch: Option<String>,
    base_commit: Option<String>,
}
#[derive(Deserialize)]
struct SpacePreview {
    id: Option<String>,
    name: String,
    private: bool,
    members: Vec<String>,
    workspaces: Vec<WorkspacePreview>,
}
#[derive(Clone, Deserialize)]
struct PurgePreview {
    tasks: u64,
    messages: u64,
    sequence: u64,
}
#[derive(Deserialize)]
struct HistoryPage {
    capacity: serde_json::Value,
    preview: PurgePreview,
    records: Vec<serde_json::Value>,
    cursor: Option<u64>,
}

#[derive(Deserialize)]
struct Snapshot {
    project: String,
    agents: Vec<PanelAgent>,
    agent_cursor: Option<String>,
    tasks: Vec<PanelTask>,
    task_cursor: Option<u64>,
    task: Option<Task>,
    task_runtime: Option<TaskRuntime>,
    events: Vec<Event>,
    event_cursor: Option<u64>,
    admission: String,
    spaces: Vec<SpacePreview>,
    space_cursor: Option<String>,
    reservations: Vec<Reservation>,
    reservation_cursor: Option<u64>,
    messages: Vec<Message>,
    message_cursor: Option<u64>,
    history: Option<HistoryPage>,
}

pub(crate) struct CollaborationPanel {
    fixtures: Vec<Fixture>,
    selected: usize,
    next: MouseStateHandle,
    scroll: ClippedScrollStateHandle,
    preview: bool,
    visible: bool,
    snapshot: Option<Snapshot>,
    query: PanelQuery,
    events: Vec<Event>,
    context: Option<(String, Option<String>)>,
    in_flight: bool,
    status: String,
    task_buttons: HashMap<String, MouseStateHandle>,
    page_buttons: [MouseStateHandle; 8],
    generation: u64,
    connected: bool,
    form: Option<controls::Form>,
    control_buttons: [MouseStateHandle; 9],
    focus_buttons: HashMap<String, MouseStateHandle>,
    show_spaces: bool,
    workspace_preview: Option<WorkspacePreview>,
    workspace_buttons: HashMap<String, MouseStateHandle>,
    scope_buttons: [MouseStateHandle; 5],
    agent_task_buttons: HashMap<String, MouseStateHandle>,
    show_messages: bool,
    thread_buttons: HashMap<String, MouseStateHandle>,
    message_page_buttons: [MouseStateHandle; 3],
    evidence_buttons: HashMap<String, MouseStateHandle>,
    history_buttons: [MouseStateHandle; 4],
    remote: Option<SshProfile>,
    remote_client: Arc<tokio::sync::Mutex<Option<HostClient>>>,
    remote_failed: bool,
    last_received: Option<Instant>,
    remote_buttons: [MouseStateHandle; 3],
    remote_connection: Option<SshConnection>,
}

#[derive(Clone)]
enum SshConnection {
    Multiplexed {
        socket: std::path::PathBuf,
        wsl: Option<String>,
    },
    Native {
        arguments: Vec<String>,
        session: String,
    },
}

impl SshConnection {
    fn scope_key(&self) -> String {
        match self {
            Self::Multiplexed { socket, wsl } => format!("master:{socket:?}:{wsl:?}"),
            Self::Native { session, .. } => format!("native:{session}"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    NextFixture,
    ReconnectSsh,
    RemoteSetup,
    UseLocal,
    PreviousFixture,
    Scroll(f32),
    Exit,
    SelectTask(String),
    Back,
    NextTasks,
    FirstTasks,
    NextAgents,
    FirstAgents,
    OpenControl(controls::Kind),
    ConfirmControl,
    CancelControl,
    FocusAgent(String),
    Spaces,
    PreviewWorkspace(String),
    ConfirmWorkspace,
    FirstSpaces,
    History,
    FirstHistory,
    NextHistory,
    CopyHistory,
    OpenEvidence(String),
    OpenThread(String),
    FirstMessages,
    NextMessages,
    FilterTaskState,
    FilterTaskAssignee(Option<String>),
    ToggleArchived,
    FirstReservations,
    NextReservations,
    NextSpaces,
}

impl CollaborationPanel {
    pub(crate) fn new(ctx: &mut ViewContext<Self>) -> Self {
        let preview = std::env::var_os("WARP_COLLABORATION_PREVIEW").is_some();
        if !preview {
            Self::schedule(ctx);
        }
        Self {
            fixtures: serde_json::from_str(include_str!(
                "../../../specs/agent-communication-v2/panel-fixtures.json"
            ))
            .expect("Validated collaboration fixtures"),
            selected: 0,
            next: Default::default(),
            scroll: Default::default(),
            preview,
            visible: false,
            snapshot: None,
            query: Default::default(),
            events: Vec::new(),
            context: None,
            in_flight: false,
            status: "Loading collaboration…".into(),
            task_buttons: Default::default(),
            page_buttons: Default::default(),
            generation: 0,
            connected: false,
            form: None,
            control_buttons: Default::default(),
            focus_buttons: Default::default(),
            show_spaces: false,
            workspace_preview: None,
            workspace_buttons: Default::default(),
            scope_buttons: Default::default(),
            agent_task_buttons: Default::default(),
            show_messages: false,
            thread_buttons: Default::default(),
            message_page_buttons: Default::default(),
            evidence_buttons: Default::default(),
            history_buttons: Default::default(),
            remote: None,
            remote_client: Default::default(),
            remote_failed: false,
            last_received: None,
            remote_buttons: Default::default(),
            remote_connection: None,
        }
    }

    fn selected_ssh(&self, ctx: &ViewContext<Self>) -> Option<(SshProfile, SshConnection)> {
        let active = crate::workspace::ActiveSession::as_ref(ctx);
        let session = active.session(ctx.window_id())?;
        // Nested or opaque SSH commands cannot adopt a desktop-side transport.
        let arguments = session.ssh_arguments()?;
        let connection = match session.ssh_control_socket() {
            Some(socket) => SshConnection::Multiplexed {
                socket: socket.to_owned(),
                wsl: session.wsl_distro_name().map(str::to_owned),
            },
            None if session.wsl_distro_name().is_some() => return None,
            None => SshConnection::Native {
                arguments: arguments.to_vec(),
                session: format!("{:?}", session.id()),
            },
        };
        let root = active.current_directory(ctx.window_id())?;
        let (path, shell) = warp_agent_bus::installation::companion_path(
            session.home_dir()?,
            session.host_info().os_category.as_deref()?,
        )
        .ok()?;
        let profile = SshProfile {
            target: session.hostname().into(),
            config_file: None,
            remote_root: root.into(),
            companion_path: path,
            remote_shell: shell,
        };
        profile.validate().ok()?;
        Some((profile, connection))
    }

    fn current_context(&self, ctx: &ViewContext<Self>) -> Option<(String, Option<String>)> {
        #[cfg(debug_assertions)]
        if let Some(profile) = self
            .remote
            .as_ref()
            .filter(|profile| profile.target == "native-companion-checkpoint")
        {
            return Some((
                format!(
                    "ssh:{}:{}:{}",
                    profile.target, profile.remote_root, profile.companion_path
                ),
                None,
            ));
        }
        if let Some((profile, connection)) = self.selected_ssh(ctx) {
            return Some((
                format!(
                    "ssh:{}:{}:{}:{}",
                    profile.target,
                    profile.remote_root,
                    profile.companion_path,
                    connection.scope_key()
                ),
                None,
            ));
        }
        let active = crate::workspace::ActiveSession::as_ref(ctx);
        if active.remote_pending(ctx.window_id())
            || active.session(ctx.window_id()).is_some_and(|session| {
                session.is_legacy_ssh_session()
                    || session.ssh_arguments().is_some()
                    || matches!(
                        session.session_type(),
                        crate::terminal::model::session::SessionType::WarpifiedRemote { .. }
                    )
            })
        {
            return None;
        }
        let root = active.path_if_local(ctx.window_id())?.to_str()?.to_owned();
        let terminal = active
            .terminal_view_id(ctx.window_id())
            .and_then(|id| super::terminal_for_view(id));
        Some((root, terminal))
    }

    fn schedule(ctx: &mut ViewContext<Self>) {
        ctx.spawn(Timer::after(Duration::from_millis(250)), |panel, _, ctx| {
            panel.refresh(ctx);
            Self::schedule(ctx);
        });
    }

    /// Closing or switching tools fences pending replies and stops new projection reads.
    pub(crate) fn set_visible(&mut self, visible: bool, ctx: &mut ViewContext<Self>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        self.generation += 1;
        self.connected = false;
        if visible && !self.preview {
            self.refresh(ctx);
        }
        ctx.notify();
    }

    fn refresh(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.visible {
            return;
        }
        #[cfg(debug_assertions)]
        let checkpoint = self
            .remote
            .as_ref()
            .is_some_and(|profile| profile.target == "native-companion-checkpoint");
        #[cfg(not(debug_assertions))]
        let checkpoint = false;
        if !checkpoint {
            let selected = self.selected_ssh(ctx);
            self.remote = selected.as_ref().map(|(profile, _)| profile.clone());
            self.remote_connection = selected.map(|(_, connection)| connection);
        }
        let enabled = super::AgentCommunication::as_ref(ctx).preferences.enabled;
        let context = enabled.then(|| self.current_context(ctx)).flatten();
        if context != self.context {
            self.generation += 1;
            self.context = context.clone();
            self.remote_client = Default::default();
            self.remote_failed = false;
            self.snapshot = None;
            self.last_received = None;
            self.connected = false;
            self.form = None;
            self.show_spaces = false;
            self.show_messages = false;
            self.workspace_preview = None;
            self.events.clear();
            self.query = Default::default();
            self.scroll = Default::default();
            ctx.notify();
        }
        let Some((directory, terminal)) = context.clone() else {
            let status = if enabled {
                "Select a terminal project. For remote collaboration, use ssh user@host, then cd into the project. SSH setup explains installation and shell integration."
            } else {
                "Communication is off. Enable it in Settings > Features > Agent communication."
            };
            if self.status != status {
                self.status = status.into();
                ctx.notify();
            }
            return;
        };
        if self.in_flight {
            return;
        }
        if self.remote.is_some() && self.remote_failed {
            return;
        }
        let broker = super::BROKER.get().cloned();
        if self.remote.is_none() && broker.is_none() {
            self.status = "Coordinator unavailable. Restart Warpai to reconnect.".into();
            ctx.notify();
            return;
        }
        let remote = self.remote.clone();
        let connection = self.remote_connection.clone();
        let client = self.remote_client.clone();
        self.in_flight = true;
        let mut query = self.query.clone();
        let generation = self.generation;
        query.terminal = terminal;
        ctx.spawn(async move {
            let value = if let Some(profile) = remote {
                let mut client = client.lock().await;
                if client.is_none() {
                    *client = Some(match connection {
                        Some(SshConnection::Multiplexed { socket, wsl }) => HostClient::connect_session(&profile, &socket, wsl.as_deref()).await,
                        Some(SshConnection::Native { arguments, .. }) => HostClient::connect_arguments(&profile, &arguments).await,
                        None => HostClient::connect(&profile).await,
                    }.map_err(anyhow::Error::new)?);
                }
                query.project.clear();
                query.terminal = None;
                query.spaces = false;
                Self::remote_command(client.as_mut().unwrap(), &TaskCommand::Panel(query), generation).await?
            } else {
                query.project = warp_agent_bus::project_root(std::path::Path::new(&directory))?;
                broker.unwrap().operator_panel(&query)?
            };
            serde_json::from_value::<Snapshot>(value).map_err(anyhow::Error::from)
        }, move |panel, result: anyhow::Result<Snapshot>, ctx| {
            panel.in_flight = false;
            if !panel.visible || panel.generation != generation || panel.context != context || panel.current_context(ctx) != context
                || !super::AgentCommunication::as_ref(ctx).preferences.enabled {
                return;
            }
            match result {
                Ok(mut snapshot) => {
                    if panel.query.scope.as_deref() != Some(snapshot.project.as_str()) {
                        panel.events.clear();
                        panel.query = Default::default();
                        panel.query.spaces = panel.show_spaces;
                    }
                    panel.query.scope = Some(snapshot.project.clone());
                    for event in snapshot.events.drain(..) {
                        if panel.events.last().is_none_or(|last| last.sequence < event.sequence) {
                            panel.events.push(event);
                        }
                    }
                    if panel.events.len() > 200 {
                        panel.events.drain(..panel.events.len() - 200);
                    }
                    panel.query.event_after = snapshot.event_cursor.or(panel.query.event_after);
                    panel.query.wait = true;
                    panel.task_buttons.retain(|id, _| snapshot.tasks.iter().any(|task| &task.id == id));
                    for task in &snapshot.tasks {
                        panel.task_buttons.entry(task.id.clone()).or_default();
                    }
                    panel.evidence_buttons.retain(|id, _| snapshot.task.as_ref().is_some_and(|task| task.evidence_records.iter().any(|evidence| &evidence.id == id)));
                    if let Some(task) = &snapshot.task { for evidence in &task.evidence_records { panel.evidence_buttons.entry(evidence.id.clone()).or_default(); } }
                    panel.thread_buttons.retain(|id, _| snapshot.messages.iter().any(|message| message.thread_id.as_ref().unwrap_or(&message.id) == id));
                    for message in &snapshot.messages { panel.thread_buttons.entry(message.thread_id.as_ref().unwrap_or(&message.id).clone()).or_default(); }
                    panel.agent_task_buttons.retain(|id, _| snapshot.agents.iter().any(|row| &row.agent.id == id));
                    for row in &snapshot.agents { panel.agent_task_buttons.entry(row.agent.id.clone()).or_default(); }
                    panel.focus_buttons.retain(|id, _| snapshot.agents.iter().any(|row| &row.agent.id == id));
                    for row in &snapshot.agents { panel.focus_buttons.entry(row.agent.id.clone()).or_default(); }
                    panel.workspace_buttons.retain(|id, _| snapshot.spaces.iter().flat_map(|space| &space.workspaces).any(|workspace| &workspace.id == id));
                    for workspace in snapshot.spaces.iter().flat_map(|space| &space.workspaces) { panel.workspace_buttons.entry(workspace.id.clone()).or_default(); }
                    panel.connected = true;
                    panel.last_received = Some(Instant::now());
                    panel.status = if panel.remote.is_some() { "Connected to the remote project. Agent presence is observed remotely.".into() } else { match snapshot.admission.as_str() {
                        "revoked" => "Participation revoked. Existing effects may still be running; review the mapping and open a fresh shared pane.",
                        "directory_mismatch" => "This pane changed checkout. Its shared native connection is unavailable in this directory; open a fresh pane for the reviewed workspace.",
                        _ => "Connected to the local coordinator. Execution and presence are separate.",
                    }.into() };
                    panel.snapshot = Some(snapshot);
                }
                Err(error) => {
                    panel.connected = false;
                    let code = error.downcast_ref::<warp_agent_bus::DomainError>()
                        .map(|error| error.code.as_str()).unwrap_or("coordinator_unavailable");
                    if panel.remote.is_some() {
                        panel.remote_failed = error.downcast_ref::<warp_agent_bus::DomainError>().is_none();
                        panel.status = if panel.remote_failed {
                            match error.downcast_ref::<warp_agent_bus::ssh_remote::ConnectionError>() {
                                Some(warp_agent_bus::ssh_remote::ConnectionError::CompanionUnavailable) => "Warpai Companion is missing or cannot run. Use SSH setup to install the package for this remote system, then reconnect.".into(),
                                Some(warp_agent_bus::ssh_remote::ConnectionError::SshAuthenticationUnavailable) => "The companion connection requires system OpenSSH authentication. Unlock your SSH key agent, then reconnect. Warpai does not store SSH passwords.".into(),
                                Some(warp_agent_bus::ssh_remote::ConnectionError::IncompatibleVersion) => "Warpai Companion is incompatible. Use SSH setup to update it, then reconnect.".into(),
                                _ => format!("SSH project unavailable ({code}). Last received state is stale. Check the terminal connection, then reconnect."),
                            }
                        } else {
                            format!("Could not update the remote view ({code}). Last state is stale; change the view or refresh.")
                        };
                    } else {
                        panel.status = format!("Could not update collaboration ({code}). Last received state may be stale; refresh or restart Warpai.");
                    }
                }
            }
            ctx.notify();
        });
    }

    async fn remote_command(
        client: &mut HostClient,
        command: &TaskCommand,
        generation: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let envelope = client
            .project_tasks(command, generation)
            .await
            .map_err(|_| anyhow::anyhow!("ssh_connection_lost"))?;
        if let Some(error) = envelope.get("error") {
            return Err(
                serde_json::from_value::<warp_agent_bus::DomainError>(error.clone())?.into(),
            );
        }
        envelope
            .get("value")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("ssh_invalid_response"))
    }

    fn reconnect_remote(&mut self, ctx: &mut ViewContext<Self>) {
        self.generation += 1;
        self.remote_client = Default::default();
        self.remote_failed = false;
        self.connected = false;
        self.query.wait = false;
        self.status = "Connecting to the SSH project…".into();
        self.refresh(ctx);
        ctx.notify();
    }

    fn live_fixture(&self) -> Fixture {
        let mut fixture = Fixture {
            state: "live".into(),
            guidance: self.status.clone(),
            sections: vec![],
        };
        if let Some(profile) = &self.remote {
            fixture.sections.push(Section {
                title: "SSH project".into(),
                rows: vec![
                    format!("{} · {}", profile.target, profile.remote_root),
                    self.last_received
                        .map(|at| {
                            format!(
                                "Last received update {}s ago{}",
                                at.elapsed().as_secs(),
                                if self.connected { "" } else { " · stale" }
                            )
                        })
                        .unwrap_or_else(|| "No remote update received.".into()),
                ],
            });
        }
        let Some(snapshot) = &self.snapshot else {
            return fixture;
        };
        if self.remote.is_none() {
            fixture.sections.push(Section {
                title: "Scope".into(),
                rows: vec![
                    format!(
                        "{} · {}",
                        if self.remote.is_some() {
                            "Remote project"
                        } else if cfg!(target_os = "windows") {
                            "This Windows PC"
                        } else {
                            "This Mac"
                        },
                        snapshot.project
                    ),
                    "Only explicitly participating agents in this scope can collaborate.".into(),
                    format!(
                        "Task filters · state {} · assignee {} · archived {}",
                        self.query.task_state.as_deref().unwrap_or("any"),
                        self.query.task_assignee.as_deref().unwrap_or("any"),
                        self.query.include_archived
                    ),
                ],
            });
        }
        if self.query.history {
            fixture.state = "history and capacity".into();
            if let Some(history) = &snapshot.history {
                let capacity = &history.capacity;
                fixture.sections.push(Section {
                    title: "Storage budget".into(),
                    rows: vec![
                        format!(
                            "Database bytes used {} · soft limit {} · hard limit {}",
                            capacity["database"]["used_bytes"],
                            capacity["database"]["soft_limit"],
                            capacity["database"]["hard_limit"]
                        ),
                        format!(
                            "Tasks active {} · total {} · active limit {}",
                            capacity["tasks"]["used"],
                            capacity["tasks"]["total"],
                            capacity["tasks"]["limit"]
                        ),
                        format!(
                            "Messages {} · pending {} · per-agent pending limit {}",
                            capacity["messages"]["used"],
                            capacity["pending_messages"]["used"],
                            capacity["pending_messages"]["per_agent_limit"]
                        ),
                    ],
                });
                fixture.sections.push(Section { title: "Purge preview".into(), rows: vec![
                    format!("Eligible archived tasks {} · acknowledged messages {} · reviewed sequence {}", history.preview.tasks, history.preview.messages, history.preview.sequence),
                    "Unknown execution, unread work, prerequisites and retained reply roots are protected. Cleanup does not stop processes or file writes.".into(),
                ] });
                fixture.sections.push(Section {
                    title: "Ordered export page · up to 50 records".into(),
                    rows: history
                        .records
                        .iter()
                        .map(|record| {
                            format!(
                                "Sequence {} · {} · {}",
                                record["sequence"], record["type"], record["data"]["id"]
                            )
                        })
                        .collect(),
                });
            }
        } else if self.show_messages {
            fixture.state = if self.query.selected_thread.is_some() {
                "thread history"
            } else {
                "message search"
            }
            .into();
            fixture.sections.push(Section {
                title: format!("Messages · query {} · thread {}", self.query.message_query.as_deref().unwrap_or("not set"), self.query.selected_thread.as_deref().unwrap_or("search results")),
                rows: if snapshot.messages.is_empty() { vec!["No messages on this page. Search another literal phrase or return to the first page. Original messages remain immutable; corrections are new replies.".into()] }
                    else { snapshot.messages.iter().flat_map(|message| [
                        format!("{} · {} · {} → {} · subject {} · thread {} · reply {} · task {} · {}", message.sequence, message.id, message.from, message.to, message.subject.as_deref().unwrap_or("none"), message.thread_id.as_deref().unwrap_or(&message.id), message.reply_to.as_deref().unwrap_or("root"), message.task_id.as_deref().unwrap_or("unlinked"), if message.acknowledged { "acknowledged" } else { "pending acknowledgement" }),
                        message.body.clone(),
                    ]).collect() },
            });
        } else if self.show_spaces {
            fixture.state = "space preview".into();
            fixture.sections.push(Section { title: "New shared sessions only".into(), rows: vec![
                "Existing private tasks and panes keep their original scope. Review participants and mapped checkouts, then explicitly open a new shared pane. Start a configured native agent there to participate.".into(),
                "Saved layouts and restored sessions begin private until explicitly joined again. Leaving shared participation revokes coordination access; it does not stop a CLI or file writes.".into(),
            ] });
            if let Some(workspace) = &self.workspace_preview {
                fixture.sections.push(Section { title: "Reviewed workspace admission".into(), rows: vec![
                    format!("Space {} · workspace {} · {}", workspace.space_id, workspace.id, workspace.root),
                    format!("Repository {} · model {} · branch {} · base {}", workspace.repository_id.as_deref().unwrap_or("not linked"), workspace.model, workspace.branch.as_deref().unwrap_or("unspecified"), workspace.base_commit.as_deref().unwrap_or("unspecified")),
                    "Confirmation opens one new local tab in exactly this reviewed mapping. A changed mapping fails closed.".into(),
                ] });
            }
            for space in &snapshot.spaces {
                fixture.sections.push(Section {
                    title: format!(
                        "{} · {}",
                        space.name,
                        space.id.as_deref().unwrap_or("private")
                    ),
                    rows: vec![if space.private {
                        "Ordinary new panes remain isolated by canonical project root.".into()
                    } else {
                        format!(
                            "Participants: {}",
                            if space.members.is_empty() {
                                "none yet".into()
                            } else {
                                space.members.join(", ")
                            }
                        )
                    }]
                    .into_iter()
                    .chain(space.workspaces.iter().map(|workspace| {
                        format!(
                            "{} · repository {} · {}",
                            workspace.root,
                            workspace.repository_id.as_deref().unwrap_or("not linked"),
                            workspace.model
                        )
                    }))
                    .collect(),
                });
            }
            return fixture;
        }
        if let Some(task) = &snapshot.task {
            fixture.state = "task detail".into();
            fixture.sections.extend([
                Section {
                    title: format!(
                        "Task {} · {} · revision {} · version {}",
                        task.id, task.state, task.revision, task.version
                    ),
                    rows: {
                        let mut rows = vec![
                        format!(
                            "Issuer: {} · assignee: {} · reviewer: {}",
                            task.issuer, task.assignee, task.reviewer
                        ),
                        task.wait_reason.clone().unwrap_or_else(|| {
                            "No recorded dependency or delivery blocker.".into()
                        }),
                        snapshot
                            .task_runtime
                            .as_ref()
                            .map(|runtime| {
                                format!(
                                    "Receiver {} · execution {}",
                                    if runtime.online { "online" } else { "offline" },
                                    if runtime.interrupted {
                                        "interrupted; outcome unknown"
                                    } else {
                                        "see recorded attempts"
                                    }
                                )
                            })
                            .unwrap_or_default(),
                        ];
                        if let Some(runtime) = &snapshot.task_runtime {
                            if let Some(phase) = &runtime.delivery_phase {
                                rows.push(format!("Native prompt {} · receiver acknowledgement and TaskStart remain separate{}",
                                    phase, if runtime.delivery_retained == Some(false) { " · delivery history could not be retained" } else { "" }));
                            }
                        }
                        rows
                    },
                },
                Section {
                    title: "Description".into(),
                    rows: vec![task.description.clone()],
                },
                Section {
                    title: "Acceptance".into(),
                    rows: vec![task.acceptance.clone()],
                },
                Section {
                    title: "Attempts and dependencies".into(),
                    rows: task
                        .attempts
                        .iter()
                        .map(|attempt| {
                            format!(
                                "{} · {} · owner {} · outcome {}",
                                attempt.id,
                                attempt.certainty,
                                attempt.owner,
                                attempt.outcome.as_deref().unwrap_or("unknown")
                            )
                        })
                        .chain(
                            task.dependencies
                                .iter()
                                .map(|id| format!("Prerequisite {id}")),
                        )
                        .collect(),
                },
                Section {
                    title: "Scheduling and deadlines".into(),
                    rows: {
                        let deadline = |value: Option<u64>| value.and_then(|value| i64::try_from(value).ok())
                            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
                            .map(|time| time.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "not set".into());
                        vec![
                            format!("Start by {} · execution deadline {} · review deadline {}", deadline(task.start_deadline), deadline(task.execution_deadline), deadline(task.review_deadline)),
                            format!("Execution timeout {} · review timeout {} · review overdue {}", task.execution_timeout_seconds.map(|seconds| format!("{seconds}s")).unwrap_or_else(|| "not set".into()), task.review_timeout_seconds.map(|seconds| format!("{seconds}s")).unwrap_or_else(|| "not set".into()), task.review_overdue),
                            format!("Eligible pool participants: {}", if task.eligible.is_empty() { "assigned task".into() } else { task.eligible.join(", ") }),
                            "Expiry changes coordination state; it does not stop a process or assert that file writes ended.".into(),
                            if task.history_truncated { "Earlier attempts or review feedback are outside this bounded detail. Use history retrieval for older records.".into() } else { "All retained attempt and feedback records fit this detail.".into() },
                        ]
                    },
                },
                Section {
                    title: "Result and review".into(),
                    rows: task
                        .result
                        .iter()
                        .cloned()
                        .chain(task.feedback.iter().cloned())
                        .collect(),
                },
                Section {
                    title: "Evidence".into(),
                    rows: task
                        .evidence
                        .iter()
                        .cloned()
                        .chain(task.evidence_records.iter().map(|evidence| {
                            format!(
                                "{} · {} · {}{}",
                                evidence.kind,
                                evidence
                                    .path
                                    .as_deref()
                                    .or(evidence.summary.as_deref())
                                    .unwrap_or("metadata reference"),
                                if evidence.verified {
                                    "locally verified"
                                } else {
                                    "agent-reported"
                                },
                                [
                                    evidence.commit.as_ref().map(|value| format!("commit {value}")),
                                    evidence.hash.as_ref().map(|value| format!("hash {value}")),
                                    evidence.repository.as_ref().map(|value| format!("repository {value}")),
                                    evidence.branch.as_ref().map(|value| format!("branch {value}")),
                                    evidence.base.as_ref().map(|value| format!("base {value}")),
                                    evidence.head.as_ref().map(|value| format!("head {value}")),
                                    evidence.command.as_ref().map(|value| format!("command {value}")),
                                    evidence.outcome.as_ref().map(|value| format!("outcome {value}")),
                                    evidence.exit_code.map(|value| format!("exit {value}")),
                                    evidence.summary.as_ref().map(|value| format!("summary {value}")),
                                    evidence.device.as_ref().map(|value| format!("device {value}; remote metadata, content not fetched")),
                                ].into_iter().flatten().map(|value| format!(" · {value}")).collect::<String>()
                            )
                        }))
                        .collect(),
                },
            ]);
        } else {
            fixture.sections.push(Section {
                title: "Agents".into(),
                rows: snapshot
                    .agents
                    .iter()
                    .map(|row| {
                        let activity = row.activity.map(|activity| match activity {
                            warp_agent_bus::readiness::Activity::Starting => "starting",
                            warp_agent_bus::readiness::Activity::Idle => "idle",
                            warp_agent_bus::readiness::Activity::Working => "working",
                            warp_agent_bus::readiness::Activity::WaitingApproval => "waiting for approval",
                            warp_agent_bus::readiness::Activity::WaitingInput => "waiting for input",
                            warp_agent_bus::readiness::Activity::Cancelled => "cancelled",
                            warp_agent_bus::readiness::Activity::Error => "error",
                        }).unwrap_or("unknown activity");
                        if self.remote.is_some() {
                            return format!("{} · {} · {} · {}\nRun {} · last observation {} · {}{}",
                                row.agent.name, row.agent.program,
                                if !self.connected { "stale; current state unknown" } else if row.online { "online" } else { "offline" },
                                activity, row.run.as_deref().unwrap_or("not observed"),
                                row.last_observed_ms.map(|age| format!("{}s ago", age / 1000)).unwrap_or_else(|| "not observed".into()),
                                if row.ready { "ready" } else { "readiness not reported" },
                                if row.blocked { " · waiting for approval" } else if row.paused { " · paused" } else { "" });
                        }
                        format!(
                            "{} · {} · {} · {} · draft {} · {} · readiness {} ({})\n{} · checkout {} · last observation {}{} · run {}",
                            row.agent.name,
                            row.agent.program,
                            if self.remote.is_some() && !self.connected { if row.online { "last observed online; current state unknown" } else { "last observed offline; current state unknown" } } else if row.online { "online" } else { "offline" },
                            activity,
                            row.draft.as_deref().unwrap_or("unknown"),
                            if row.blocked {
                                "waiting for approval"
                            } else if row.paused {
                                "manual pause"
                            } else {
                                "no permission blocker recorded"
                            },
                            if row.ready {
                                "reported idle"
                            } else {
                                "not ready"
                            },
                            row.readiness_source.as_deref().unwrap_or("unavailable"),
                            row.device.as_ref().filter(|device| device.as_str() != "local")
                                .map(|device| format!("Remote device {device}"))
                                .unwrap_or_else(|| "This device".into()),
                            row.workspace.as_deref().unwrap_or("unavailable"),
                            row.last_observed_ms.map(|age| format!("{} · {}s ago",
                                row.observation_source.as_deref().unwrap_or("observation"), age / 1000))
                                .unwrap_or_else(|| "unavailable".into()),
                            row.delivery_phase.as_ref().map(|phase| format!("\nLast native prompt {} · acknowledgement remains separate{}",
                                phase, if row.delivery_retained == Some(false) { " · history record unavailable" } else { "" })).unwrap_or_default(),
                            row.run.as_deref().unwrap_or("not observed"),
                        )
                    })
                    .collect(),
            });
            if snapshot.agents.is_empty() {
                fixture.sections.push(Section { title: "No participating agents".into(), rows: vec![if self.remote.is_some() { "Start managed Agents in SSH terminals of this remote project using the manually installed companion." } else { "Configure installed agents in communication settings, then start them in a new terminal pane." }.into()] });
            }
        }
        fixture.sections.push(Section {
            title: "File reservations · current checkout · up to 50 records".into(),
            rows: std::iter::once("Reservations coordinate participants; they do not lock files or prove that writes stopped. Agent renewal requires its owning run. Human maintenance in Spaces and workspaces pins the original owner and expiry; explicit release never completes an attempt.".into())
                .chain(snapshot.reservations.iter().map(|lease| {
                    let expiry = i64::try_from(lease.expires_at).ok()
                        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
                        .map(|time| time.format("%Y-%m-%d %H:%M:%S UTC").to_string()).unwrap_or_else(|| "unavailable".into());
                    format!("{} · {} · {} · owner {} · expires {} · {}{} · task {} · attempt {}", lease.id, lease.path, lease.mode, lease.owner, expiry,
                        if lease.expired { "expired" } else { "active" }, if lease.abandoned { "; abandoned owner, execution effects unknown" } else { "" },
                        lease.task_id.as_deref().unwrap_or("unlinked"), lease.attempt_id.as_deref().unwrap_or("unlinked"))
                })).collect(),
        });
        fixture.sections.push(Section {
            title: "Activity · latest 200 received events".into(),
            rows: self
                .events
                .iter()
                .filter(|event| {
                    self.query
                        .selected_task
                        .as_ref()
                        .is_none_or(|id| event.resource.as_ref() == Some(id))
                })
                .map(|event| {
                    format!(
                        "{} · {} · {}{}",
                        event.sequence,
                        event.kind,
                        event.actor,
                        if event.imported {
                            " · imported historical event"
                        } else {
                            ""
                        }
                    )
                })
                .collect(),
        });
        fixture
    }
}

impl Entity for CollaborationPanel {
    type Event = ();
}
impl TypedActionView for CollaborationPanel {
    type Action = Action;
    fn handle_action(&mut self, action: &Action, ctx: &mut ViewContext<Self>) {
        if !self.preview {
            match action {
                Action::History => {
                    self.query.task_state = None;
                    self.query.task_assignee = None;
                    self.query.include_archived = true;
                    self.query.history = true;
                    self.query.history_after = None;
                    self.query.selected_task = None;
                    self.query.spaces = false;
                    self.show_spaces = false;
                    self.show_messages = false;
                    self.query.message_query = None;
                    self.query.selected_thread = None;
                }
                Action::FirstHistory => self.query.history_after = None,
                Action::NextHistory => {
                    self.query.history_after = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.history.as_ref())
                        .and_then(|history| history.cursor)
                }
                Action::CopyHistory => {
                    if self.connected
                        && self.current_context(ctx) == self.context
                        && super::AgentCommunication::as_ref(ctx).preferences.enabled
                    {
                        if let Some(snapshot) = &self.snapshot {
                            if let Some(history) = &snapshot.history {
                                let page = serde_json::json!({"project": snapshot.project, "after": self.query.history_after, "cursor": history.cursor, "records": history.records});
                                if let Ok(text) = serde_json::to_string_pretty(&page) {
                                    ctx.clipboard().write(
                                        warpui::clipboard::ClipboardContent::plain_text(text),
                                    );
                                }
                            }
                        }
                    }
                    return;
                }
                Action::RemoteSetup => {
                    let active = crate::workspace::ActiveSession::as_ref(ctx);
                    let anchor = self.remote.as_ref().and_then(|_| active.session(ctx.window_id())).and_then(|session| {
                        match session.host_info().os_category.as_deref()? {
                            "Windows" => Some("windows"),
                            "MacOS" | "Darwin" => Some("macos"),
                            "Linux" => Some("linux"),
                            _ => None,
                        }
                    }).unwrap_or("install-on-the-remote-machine");
                    ctx.open_url(&format!("https://github.com/OthinusG/warpai/blob/main/docs/REMOTE-INSTALLATION.md#{anchor}"));
                    return;
                }
                Action::ReconnectSsh => {
                    if self.remote.is_some() {
                        self.reconnect_remote(ctx);
                    }
                    return;
                }
                Action::UseLocal => {
                    if self.form.is_some() {
                        return;
                    }
                    self.remote = None;
                    self.reconnect_remote(ctx);
                    return;
                }
                Action::Spaces => {
                    if self.remote.is_some() {
                        return;
                    }
                    self.query.history = false;
                    self.show_spaces = true;
                    self.show_messages = false;
                    self.query.message_query = None;
                    self.query.selected_thread = None;
                    self.query.message_after = None;
                    self.query.spaces = true;
                    self.query.selected_task = None;
                }
                Action::PreviewWorkspace(id) => {
                    self.workspace_preview = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| {
                            snapshot
                                .spaces
                                .iter()
                                .flat_map(|space| &space.workspaces)
                                .find(|workspace| &workspace.id == id)
                        })
                        .cloned();
                    self.scroll = Default::default();
                    ctx.notify();
                    return;
                }
                Action::ConfirmWorkspace => {
                    if !self.connected || self.current_context(ctx) != self.context {
                        return;
                    }
                    if let Some(workspace) = &self.workspace_preview {
                        // Opening a tab updates this panel's visibility and context.
                        // Dispatch after this view has returned to the app's view map.
                        ctx.dispatch_typed_action_deferred(
                            crate::workspace::WorkspaceAction::OpenCollaborationWorkspace {
                                workspace_id: workspace.id.clone(),
                                space_id: workspace.space_id.clone(),
                                root: workspace.root.clone(),
                            },
                        );
                    }
                    return;
                }
                Action::OpenEvidence(id) => {
                    if self.remote.is_some() {
                        return;
                    }
                    if !self.connected || self.current_context(ctx) != self.context {
                        return;
                    }
                    let Some(snapshot) = &self.snapshot else {
                        return;
                    };
                    let Some(task) = snapshot.task.as_ref().filter(|task| {
                        self.query.selected_task.as_ref() == Some(&task.id)
                            && task.evidence_records.iter().any(|evidence| {
                                &evidence.id == id
                                    && evidence.kind == "file"
                                    && evidence.device.is_none()
                            })
                    }) else {
                        return;
                    };
                    let Some(broker) = super::BROKER.get().cloned() else {
                        return;
                    };
                    let project = snapshot.project.clone();
                    let task_id = task.id.clone();
                    let context = self.context.clone();
                    let evidence_id = id.clone();
                    ctx.spawn(async move { broker.local_evidence_file(&project, &evidence_id) }, move |panel, result, ctx| {
                        if panel.current_context(ctx) != context || panel.context != context || panel.query.selected_task.as_ref() != Some(&task_id) || !super::AgentCommunication::as_ref(ctx).preferences.enabled { return; }
                        match result {
                            Ok(full_path) => ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::OpenFileInNewTab { full_path, line_and_column: None }),
                            Err(_) => ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::CollaborationEvidenceUnavailable),
                        }
                    });
                    return;
                }
                Action::OpenThread(id) => {
                    self.query.history = false;
                    self.show_messages = true;
                    self.show_spaces = false;
                    self.query.selected_task = None;
                    self.query.selected_thread = Some(id.clone());
                    self.query.message_after = None;
                }
                Action::FirstMessages => {
                    self.query.message_after = None;
                }
                Action::NextMessages => {
                    self.query.message_after = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.message_cursor);
                }
                Action::FilterTaskState => {
                    let states = [
                        None,
                        Some("queued"),
                        Some("blocked"),
                        Some("running"),
                        Some("cancel_requested"),
                        Some("submitted"),
                        Some("accepted"),
                        Some("failed"),
                        Some("expired"),
                        Some("cancelled"),
                    ];
                    let index = states
                        .iter()
                        .position(|state| *state == self.query.task_state.as_deref())
                        .unwrap_or(0);
                    self.query.task_state = states[(index + 1) % states.len()].map(str::to_owned);
                    self.query.task_after = None;
                    self.query.selected_task = None;
                }
                Action::FilterTaskAssignee(id) => {
                    self.query.task_assignee = id.clone();
                    self.query.task_after = None;
                    self.query.selected_task = None;
                }
                Action::ToggleArchived => {
                    self.query.include_archived = !self.query.include_archived;
                    self.query.task_after = None;
                    self.query.selected_task = None;
                }
                Action::FirstReservations => {
                    self.query.reservation_after = None;
                }
                Action::NextReservations => {
                    self.query.reservation_after = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.reservation_cursor);
                }
                Action::FirstSpaces => {
                    self.query.space_after = None;
                    self.workspace_preview = None;
                }
                Action::NextSpaces => {
                    self.query.space_after = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.space_cursor.clone());
                    self.workspace_preview = None;
                }
                Action::OpenControl(kind) => {
                    self.open_control(*kind, ctx);
                    return;
                }
                Action::ConfirmControl => {
                    self.confirm_control(ctx);
                    return;
                }
                Action::CancelControl => {
                    self.cancel_control(ctx);
                    return;
                }
                Action::FocusAgent(id) => {
                    if self.remote.is_some() {
                        return;
                    }
                    if self.current_context(ctx) != self.context {
                        return;
                    }
                    let Some(row) = self.snapshot.as_ref().and_then(|snapshot| {
                        snapshot
                            .agents
                            .iter()
                            .find(|row| &row.agent.id == id && row.online)
                    }) else {
                        return;
                    };
                    let target = super::VIEWS
                        .get()
                        .and_then(|views| views.lock().ok())
                        .and_then(|views| {
                            views
                                .iter()
                                .find(|(_, (terminal, _))| terminal == &row.agent.terminal)
                                .and_then(|(id, (_, view))| view.upgrade(ctx).map(|_| *id))
                        });
                    if let Some(terminal_view_id) = target {
                        ctx.dispatch_typed_action_deferred(
                            crate::workspace::WorkspaceAction::FocusTerminalViewInWorkspace {
                                terminal_view_id,
                            },
                        );
                    }
                    return;
                }
                Action::SelectTask(id) => self.query.selected_task = Some(id.clone()),
                Action::Back => {
                    self.query.history = false;
                    self.query.history_after = None;
                    self.query.selected_task = None;
                    self.show_spaces = false;
                    self.show_messages = false;
                    self.query.message_query = None;
                    self.query.selected_thread = None;
                    self.query.message_after = None;
                    self.query.spaces = false;
                    self.workspace_preview = None;
                }
                Action::NextTasks => {
                    self.query.task_after = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.task_cursor)
                }
                Action::FirstTasks => self.query.task_after = None,
                Action::NextAgents => {
                    self.query.agent_after = self
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.agent_cursor.clone())
                }
                Action::FirstAgents => self.query.agent_after = None,
                Action::NextFixture | Action::PreviousFixture => {}
                Action::Scroll(delta) => {
                    self.scroll.scroll_by((*delta).into_pixels());
                    ctx.notify();
                    return;
                }
                Action::Exit => {
                    ctx.dispatch_typed_action_deferred(
                        crate::workspace::WorkspaceAction::FocusLeftPanel,
                    );
                    return;
                }
            }
            self.generation += 1;
            self.query.wait = false;
            self.scroll = Default::default();
            self.refresh(ctx);
            ctx.notify();
            return;
        }
        match action {
            Action::NextFixture => {
                self.selected = (self.selected + 1) % self.fixtures.len();
                self.scroll = Default::default();
                ctx.notify();
            }
            Action::PreviousFixture => {
                self.selected = (self.selected + self.fixtures.len() - 1) % self.fixtures.len();
                self.scroll = Default::default();
                ctx.notify();
            }
            Action::Scroll(delta) => {
                self.scroll.scroll_by((*delta).into_pixels());
                ctx.notify();
            }
            Action::Exit => ctx
                .dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::FocusLeftPanel),
            _ => {}
        }
    }
    fn action_accessibility_contents(
        &mut self,
        _: &Action,
        ctx: &mut ViewContext<Self>,
    ) -> ActionAccessibilityContent {
        self.accessibility_contents(ctx).into()
    }
}
impl View for CollaborationPanel {
    fn ui_name() -> &'static str {
        "AgentCollaboration"
    }
    fn accessibility_contents(&self, _: &AppContext) -> Option<AccessibilityContent> {
        if !self.preview {
            return Some(AccessibilityContent::new(
                format!("Agent collaboration. {}", self.status),
                "Enter refreshes. Page Up and Page Down scroll. Escape returns to terminal. Tasks and pagination use native buttons.",
                WarpA11yRole::ScrollareaRole,
            ));
        }
        let fixture = &self.fixtures[self.selected];
        Some(AccessibilityContent::new(
            format!("Agent collaboration, sample data. {}. {}", fixture.state, fixture.guidance),
            "Left and Right or Enter change preview state. Page Up and Page Down scroll. Escape returns to terminal.",
            WarpA11yRole::ScrollareaRole,
        ))
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let theme = appearance.theme();
        let live = self.live_fixture();
        let fixture = if self.preview {
            &self.fixtures[self.selected]
        } else {
            &live
        };
        let hide_navigation = self
            .form
            .as_ref()
            .is_some_and(|form| matches!(form.kind, controls::Kind::Send));
        let mut header = Flex::column().with_spacing(GAP_ROW);
        header.add_child(panel_title(appearance, "Agent collaboration"));
        let state = if self.preview {
            format!("Design preview — sample data · {}", fixture.state)
        } else {
            format!(
                "{} collaboration · {}",
                if self.remote.is_some() {
                    "SSH"
                } else {
                    "Local"
                },
                fixture.state
            )
        };
        header.add_child(note(
            appearance,
            if fixture.guidance.is_empty() {
                state
            } else {
                format!("{state} — {}", fixture.guidance)
            },
        ));
        let mut connection_controls = Wrap::row()
            .with_spacing(GAP_SECTION)
            .with_run_spacing(GAP_TIGHT);
        if !self.preview {
            for (index, label, action) in [
                (0, "SSH setup", Action::RemoteSetup),
                (1, "Reconnect", Action::ReconnectSsh),
            ] {
                if index > 0 && self.remote.is_none() {
                    continue;
                }
                // The SSH entry point is the panel's primary action; the rest stay secondary text.
                let variant = if index == 0 {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Text
                };
                let button = builder
                    .button(variant, self.remote_buttons[index].clone())
                    .with_text_label(label.into());
                let button = if (index != 1 && self.form.is_some())
                    || !super::AgentCommunication::as_ref(app).preferences.enabled
                {
                    button.disabled()
                } else {
                    button
                };
                connection_controls.add_child(
                    button
                        .build()
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                        .finish(),
                );
            }
        }
        connection_controls.add_child(
            builder
                .button(ButtonVariant::Text, self.next.clone())
                .with_text_label(
                    if self.preview {
                        "Next preview state"
                    } else {
                        "Refresh"
                    }
                    .to_owned(),
                )
                .build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::NextFixture))
                .finish(),
        );
        header.add_child(connection_controls.finish());
        let mut body = Flex::column().with_spacing(GAP_SECTION);
        let render_section = |section: &Section| {
            let mut column = Flex::column().with_spacing(GAP_TIGHT);
            column.add_child(heading(appearance, section.title.clone()));
            for row in &section.rows {
                column.add_child(
                    Container::new(detail(appearance, row.clone()))
                        .with_padding_left(GAP_ROW)
                        .finish(),
                );
            }
            column.finish()
        };
        let leading_status = !self.preview && self.remote.is_some() && self.form.is_none();
        if leading_status {
            for section in fixture.sections.iter().filter(|section| {
                matches!(
                    section.title.as_str(),
                    "SSH project" | "Agents" | "No participating agents"
                )
            }) {
                body.add_child(render_section(section));
            }
        }
        if !self.preview && self.form.is_some() {
            body.add_child(self.render_controls(app));
        }
        if !self.preview {
            if let Some(snapshot) = &self.snapshot {
                let mut navigation = Wrap::row()
                    .with_spacing(GAP_SECTION)
                    .with_run_spacing(GAP_TIGHT);
                if self.remote.is_none() && !hide_navigation {
                    navigation.add_child(
                        builder
                            .button(ButtonVariant::Text, self.scope_buttons[0].clone())
                            .with_text_label("Spaces and workspaces".into())
                            .build()
                            .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Spaces))
                            .finish(),
                    );
                }
                if !hide_navigation {
                    navigation.add_child(
                        builder
                            .button(ButtonVariant::Text, self.history_buttons[0].clone())
                            .with_text_label("History and storage".into())
                            .build()
                            .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::History))
                            .finish(),
                    );
                }
                if self.query.selected_task.is_some()
                    || self.show_spaces
                    || self.show_messages
                    || self.query.history
                {
                    if !hide_navigation {
                        navigation.add_child(
                            builder
                                .button(ButtonVariant::Text, self.page_buttons[0].clone())
                                .with_text_label("Back to agents and tasks".into())
                                .build()
                                .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Back))
                                .finish(),
                        );
                    }
                } else {
                    body.add_child(heading(appearance, "Tasks"));
                    if snapshot.tasks.is_empty() {
                        body.add_child(note(appearance, "No tasks on this page."));
                    }
                    for task in &snapshot.tasks {
                        let id = task.id.clone();
                        body.add_child(
                            builder
                                .button(ButtonVariant::Text, self.task_buttons[&task.id].clone())
                                .with_text_label(format!(
                                    "Open task {}",
                                    task.id.chars().take(8).collect::<String>()
                                ))
                                .build()
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(Action::SelectTask(id.clone()))
                                })
                                .finish(),
                        );
                        body.add_child(
                            builder
                                .span(format!(
                                    "{} · {} · rev {} · v{} · {}",
                                    task.id, task.state, task.revision, task.version, task.assignee
                                ))
                                .with_soft_wrap()
                                .build()
                                .finish(),
                        );
                    }
                    let mut buttons = Vec::new();
                    for (label, action, index) in [
                        ("First task page", Some(Action::FirstTasks), 0),
                        (
                            "Next task page",
                            snapshot
                                .task_cursor
                                .filter(|_| snapshot.tasks.len() == 50)
                                .map(|_| Action::NextTasks),
                            1,
                        ),
                        ("First agent page", Some(Action::FirstAgents), 2),
                        (
                            "Next agent page",
                            snapshot.agent_cursor.as_ref().map(|_| Action::NextAgents),
                            3,
                        ),
                    ] {
                        if let Some(action) = action {
                            buttons.push(
                                builder
                                    .button(ButtonVariant::Text, self.page_buttons[index].clone())
                                    .with_text_label(label.into())
                                    .build()
                                    .on_click(move |ctx, _, _| {
                                        ctx.dispatch_typed_action(action.clone())
                                    })
                                    .finish(),
                            );
                        }
                    }
                    if let Some(buttons) = button_row(buttons) {
                        body.add_child(buttons);
                    }
                }
                if !hide_navigation {
                    header.add_child(navigation.finish());
                }
            }
        }
        for section in &fixture.sections {
            if leading_status
                && matches!(
                    section.title.as_str(),
                    "SSH project" | "Agents" | "No participating agents"
                )
            {
                continue;
            }
            body.add_child(render_section(section));
        }
        if !self.preview
            && !self.show_spaces
            && !self.show_messages
            && !self.query.history
            && self.form.is_none()
        {
            let mut buttons = Vec::new();
            for (label, state, action) in [
                (
                    format!(
                        "Task state: {} (next)",
                        self.query.task_state.as_deref().unwrap_or("any")
                    ),
                    self.scope_buttons[4].clone(),
                    Action::FilterTaskState,
                ),
                (
                    format!(
                        "Archived: {} (toggle)",
                        if self.query.include_archived {
                            "included"
                        } else {
                            "hidden"
                        }
                    ),
                    self.page_buttons[6].clone(),
                    Action::ToggleArchived,
                ),
                (
                    "All assignees".into(),
                    self.page_buttons[7].clone(),
                    Action::FilterTaskAssignee(None),
                ),
            ] {
                buttons.push(
                    builder
                        .button(ButtonVariant::Text, state)
                        .with_text_label(label)
                        .build()
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                        .finish(),
                );
            }
            if let Some(buttons) = button_row(buttons) {
                body.add_child(buttons);
            }
            if let Some(snapshot) = &self.snapshot {
                for row in &snapshot.agents {
                    let id = row.agent.id.clone();
                    body.add_child(
                        builder
                            .button(ButtonVariant::Text, self.agent_task_buttons[&id].clone())
                            .with_text_label(format!("Tasks assigned to {}", row.agent.name))
                            .build()
                            .on_click(move |ctx, _, _| {
                                ctx.dispatch_typed_action(Action::FilterTaskAssignee(Some(
                                    id.clone(),
                                )))
                            })
                            .finish(),
                    );
                }
            }
        }
        if !self.preview {
            if let Some(task) = self
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.task.as_ref())
                .filter(|task| self.query.selected_task.as_ref() == Some(&task.id))
            {
                for evidence in task.evidence_records.iter().filter(|evidence| {
                    self.remote.is_none() && evidence.kind == "file" && evidence.device.is_none()
                }) {
                    let id = evidence.id.clone();
                    body.add_child(
                        builder
                            .button(ButtonVariant::Text, self.evidence_buttons[&id].clone())
                            .with_text_label(format!(
                                "Open local file evidence {}",
                                id.chars().take(8).collect::<String>()
                            ))
                            .build()
                            .on_click(move |ctx, _, _| {
                                ctx.dispatch_typed_action(Action::OpenEvidence(id.clone()))
                            })
                            .finish(),
                    );
                }
            }
        }
        if !self.preview && self.query.history && self.form.is_none() {
            body.add_child(self.render_controls(app));
            let mut buttons = Vec::new();
            for (index, label, action) in [
                (1, "First export page", Some(Action::FirstHistory)),
                (
                    2,
                    "Next export page",
                    self.snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.history.as_ref())
                        .and_then(|history| history.cursor)
                        .map(|_| Action::NextHistory),
                ),
                (
                    3,
                    "Copy this export page as JSON",
                    Some(Action::CopyHistory),
                ),
            ] {
                let button = builder
                    .button(ButtonVariant::Text, self.history_buttons[index].clone())
                    .with_text_label(label.into());
                if let Some(action) = action.filter(|_| self.connected) {
                    buttons.push(
                        button
                            .build()
                            .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                            .finish(),
                    );
                } else {
                    buttons.push(button.disabled().build().finish());
                }
            }
            if let Some(buttons) = button_row(buttons) {
                body.add_child(buttons);
            }
        }
        if !self.preview && self.show_messages {
            if let Some(snapshot) = &self.snapshot {
                let mut buttons = Vec::new();
                for (index, label, action) in [
                    (0, "First message page", Some(Action::FirstMessages)),
                    (
                        1,
                        "Next message page",
                        snapshot.message_cursor.map(|_| Action::NextMessages),
                    ),
                ] {
                    if let Some(action) = action {
                        buttons.push(
                            builder
                                .button(
                                    ButtonVariant::Text,
                                    self.message_page_buttons[index].clone(),
                                )
                                .with_text_label(label.into())
                                .build()
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(action.clone())
                                })
                                .finish(),
                        );
                    }
                }
                if let Some(buttons) = button_row(buttons) {
                    body.add_child(buttons);
                }
                let mut seen = std::collections::HashSet::new();
                for message in &snapshot.messages {
                    let id = message.thread_id.as_ref().unwrap_or(&message.id).clone();
                    if !seen.insert(id.clone()) {
                        continue;
                    }
                    body.add_child(
                        builder
                            .button(ButtonVariant::Text, self.thread_buttons[&id].clone())
                            .with_text_label(format!(
                                "Open thread {}",
                                id.chars().take(8).collect::<String>()
                            ))
                            .build()
                            .on_click(move |ctx, _, _| {
                                ctx.dispatch_typed_action(Action::OpenThread(id.clone()))
                            })
                            .finish(),
                    );
                }
            }
        }
        if !self.preview && self.show_spaces {
            if let Some(snapshot) = &self.snapshot {
                let mut buttons = Vec::new();
                for (index, label, action) in [
                    (1, "First space page", Some(Action::FirstSpaces)),
                    (
                        2,
                        "Next space page",
                        snapshot.space_cursor.as_ref().map(|_| Action::NextSpaces),
                    ),
                    (
                        3,
                        "Confirm new shared pane",
                        self.workspace_preview
                            .as_ref()
                            .map(|_| Action::ConfirmWorkspace),
                    ),
                ] {
                    if let Some(action) = action {
                        buttons.push(
                            builder
                                .button(ButtonVariant::Text, self.scope_buttons[index].clone())
                                .with_text_label(label.into())
                                .build()
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(action.clone())
                                })
                                .finish(),
                        );
                    }
                }
                if let Some(buttons) = button_row(buttons) {
                    body.add_child(buttons);
                }
                for workspace in snapshot.spaces.iter().flat_map(|space| &space.workspaces) {
                    let id = workspace.id.clone();
                    body.add_child(
                        builder
                            .button(ButtonVariant::Text, self.workspace_buttons[&id].clone())
                            .with_text_label(format!(
                                "Review {}",
                                id.chars().take(8).collect::<String>()
                            ))
                            .build()
                            .on_click(move |ctx, _, _| {
                                ctx.dispatch_typed_action(Action::PreviewWorkspace(id.clone()))
                            })
                            .finish(),
                    );
                }
            }
        }
        if !self.preview {
            if let Some(snapshot) = &self.snapshot {
                let mut buttons = Vec::new();
                for (index, label, action) in [
                    (4, "First reservation page", Some(Action::FirstReservations)),
                    (
                        5,
                        "Next reservation page",
                        (snapshot.reservations.len() == 50).then_some(Action::NextReservations),
                    ),
                ] {
                    if let Some(action) = action {
                        buttons.push(
                            builder
                                .button(ButtonVariant::Text, self.page_buttons[index].clone())
                                .with_text_label(label.into())
                                .build()
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(action.clone())
                                })
                                .finish(),
                        );
                    }
                }
                if let Some(buttons) = button_row(buttons) {
                    body.add_child(buttons);
                }
            }
        }
        if !self.preview && self.form.is_none() && self.snapshot.is_some() {
            if let Some(snapshot) = &self.snapshot {
                for row in &snapshot.agents {
                    if self.remote.is_none()
                        && row.online
                        && super::VIEWS
                            .get()
                            .and_then(|views| views.lock().ok())
                            .is_some_and(|views| {
                                views
                                    .values()
                                    .any(|(terminal, _)| terminal == &row.agent.terminal)
                            })
                    {
                        let id = row.agent.id.clone();
                        body.add_child(
                            builder
                                .button(ButtonVariant::Text, self.focus_buttons[&id].clone())
                                .with_text_label(format!(
                                    "Focus {}",
                                    row.agent.name.chars().take(12).collect::<String>()
                                ))
                                .build()
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(Action::FocusAgent(id.clone()))
                                })
                                .finish(),
                        );
                    }
                }
            }
            body.add_child(self.render_controls(app));
        }
        let scroll = ClippedScrollable::vertical(
            self.scroll.clone(),
            body.finish(),
            ScrollbarWidth::Auto,
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .with_overlayed_scrollbar()
        .finish();
        EventHandler::new(
            Container::new(
                Flex::column()
                    .with_spacing(GAP_SECTION)
                    .with_child(header.finish())
                    .with_child(Shrinkable::new(1.0, scroll).finish())
                    .finish(),
            )
            .with_padding(Padding::uniform(GAP_SECTION))
            .finish(),
        )
        .on_keydown(|ctx, _, key| {
            if key.ctrl || key.alt || key.cmd || key.meta || key.shift {
                return DispatchEventResult::PropagateToParent;
            }
            let action = match key.key.as_str() {
                "right" | "enter" => Action::NextFixture,
                "left" => Action::PreviousFixture,
                "pageup" => Action::Scroll(-360.),
                "pagedown" => Action::Scroll(360.),
                "escape" => Action::Exit,
                _ => return DispatchEventResult::PropagateToParent,
            };
            ctx.dispatch_typed_action(action);
            DispatchEventResult::StopPropagation
        })
        .finish()
    }
}

#[cfg(debug_assertions)]
fn checkpoint_draft(app: &warpui::App, window: warpui::WindowId) -> String {
    let terminal = app
        .views_of_type::<crate::terminal::TerminalView>(window)
        .unwrap()[0]
        .clone();
    terminal.read(app, |terminal, ctx| {
        terminal
            .input()
            .read(ctx, |input, ctx| input.buffer_text(ctx))
    })
}

#[cfg(debug_assertions)]
fn register_capture_participant(
    terminal: &str,
    name: &str,
    root: &str,
    workspace: Option<&str>,
) -> anyhow::Result<warp_agent_bus::transport::Request> {
    use warp_agent_bus::{
        transport::{self, Request},
        Operation,
    };
    let broker = super::BROKER
        .get()
        .ok_or_else(|| anyhow::anyhow!("No capture broker"))?;
    broker.set_programs(Some(["codex".to_owned()].into_iter().collect()));
    let capability = match workspace {
        Some(workspace) => broker.prepare_in_workspace(terminal, workspace)?,
        None => broker.prepare(terminal)?,
    };
    broker.activate(terminal, "codex", root, true)?;
    let mut request = Request {
        protocol_major: transport::PROTOCOL_MAJOR,
        terminal: terminal.into(),
        capability,
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: Some(root.into()),
        operation: Operation::AgentRegister { name: name.into() },
    };
    request.run = transport::call(&broker.endpoint, &request)?["run"]
        .as_str()
        .map(str::to_owned);
    Ok(request)
}

/// Deterministic native IPC participants only: no vendor executable or model call.
#[cfg(debug_assertions)]
fn seed_live_checkpoint(root: &str) -> anyhow::Result<()> {
    use warp_agent_bus::{transport, Operation};
    let broker = super::BROKER
        .get()
        .ok_or_else(|| anyhow::anyhow!("No capture broker"))?;
    let terminal = "capture-native-worker";
    let mut request = register_capture_participant(terminal, "capture-worker", root, None)?;
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../specs/agent-communication-v2/panel-fixtures.json"
    ))?;
    let description = fixtures[2]
        .sections
        .iter()
        .find(|section| section.title == "Description")
        .and_then(|section| section.rows.first())
        .cloned()
        .unwrap_or_else(|| "Verify the native capture fixture".into());
    let task = broker.operator(root, &Operation::TaskAssign {
        to: "capture-worker".into(), description,
        acceptance: "The deterministic native IPC check passes and the unsent terminal draft remains intact.".into(),
        reviewer: None, request_id: uuid::Uuid::new_v4().to_string(), dependencies: vec![],
        start_deadline: None, execution_timeout_seconds: None, review_timeout_seconds: None,
    })?;
    request.operation = Operation::TaskStart {
        task_id: task["id"].as_str().unwrap().into(),
        revision: 1,
        expected_version: None,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    transport::call(&broker.endpoint, &request)?;
    request.operation = Operation::FileReserve {
        paths: vec!["native-capture-fixture.txt".into()],
        mode: "exclusive".into(),
        task_id: Some(task["id"].as_str().unwrap().into()),
        attempt_id: None,
        ttl_seconds: Some(600),
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    transport::call(&broker.endpoint, &request)?;
    let thread = broker.operator(
        root,
        &Operation::AgentSend {
            to: "capture-worker".into(),
            body: "Native thread checkpoint literal _% text".into(),
            subject: Some("Native checkpoint".into()),
            thread_id: None,
            reply_to: None,
            task_id: Some(task["id"].as_str().unwrap().into()),
            request_id: uuid::Uuid::new_v4().to_string(),
        },
    )?;
    request.operation = Operation::AgentSend {
        to: "capture-worker".into(),
        body: "Follow-up native checkpoint".into(),
        subject: Some("Native correction".into()),
        thread_id: None,
        reply_to: Some(thread["id"].as_str().unwrap().into()),
        task_id: None,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    transport::call(&broker.endpoint, &request)?;
    broker.input_guard(terminal, true, true);
    broker.end(terminal);
    Ok(())
}

/// The native panel checkpoint uses real private companion/MCP processes.
#[cfg(debug_assertions)]
impl CollaborationPanel {
    fn connect_remote_checkpoint(&mut self, launch: bool, ctx: &mut ViewContext<Self>) {
        let companion = std::path::PathBuf::from(
            std::env::var_os("WARP_TEST_COMPANION").expect("Explicit CI companion"),
        );
        let root = std::env::temp_dir().join(format!("warpai-panel-remote-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap().to_str().unwrap().to_owned();
        self.remote = Some(SshProfile {
            target: "native-companion-checkpoint".into(),
            config_file: None,
            remote_root: root.clone(),
            companion_path: companion.to_str().unwrap().into(),
            remote_shell: if cfg!(windows) {
                warp_agent_bus::ssh_remote::RemoteShell::PowerShell
            } else {
                warp_agent_bus::ssh_remote::RemoteShell::Posix
            },
        });
        self.generation += 1;
        self.connected = false;
        // Suppress polling until the real checkpoint transport is installed.
        self.remote_failed = true;
        self.remote_client = Default::default();
        self.refresh(ctx);
        let generation = self.generation;
        ctx.spawn(
            async move {
                let mut client = HostClient::connect_companion(&companion, &root)
                    .await
                    .expect("Private native companion");
                if launch {
                    let fixture = std::fs::read_dir(companion.parent().unwrap().join("deps"))
                        .unwrap()
                        .map(|entry| entry.unwrap().path())
                        .find(|path| {
                            path.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .starts_with("managed_agent-")
                                && if cfg!(windows) {
                                    path.extension().is_some_and(|ext| ext == "exe")
                                } else {
                                    path.extension().is_none()
                                }
                        })
                        .expect("Compiled native Agent fixture");
                    client
                        .terminal_launch(remote_server::proto::TerminalLaunch {
                            fence: client.fence().cloned(),
                            session_id: uuid::Uuid::new_v4().to_string(),
                            executable: fixture.to_str().unwrap().into(),
                            arguments: [
                                "--exact",
                                "managed_agent_child",
                                "--ignored",
                                "--nocapture",
                            ]
                            .map(str::to_owned)
                            .to_vec(),
                            columns: 120,
                            rows: 24,
                            agent_program: Some("fixture".into()),
                        })
                        .await
                        .expect("Real managed native Agent");
                }
                client
            },
            move |panel, client, ctx| {
                assert_eq!(
                    panel.generation, generation,
                    "Checkpoint attachment selection changed"
                );
                panel.remote_client = Arc::new(tokio::sync::Mutex::new(Some(client)));
                panel.remote_failed = false;
                panel.query.wait = false;
                panel.refresh(ctx);
            },
        );
        ctx.notify();
    }
}

/// Exercises fixed fixtures and deterministic local operations in an isolated debug profile.
#[cfg(debug_assertions)]
pub(crate) fn capture_checkpoint(directory: std::path::PathBuf) -> anyhow::Result<()> {
    use crate::{
        root_view::RootView,
        settings::Settings,
        terminal::resizable_data::ResizableData,
        themes::theme::ThemeKind,
        workspace::{
            view::left_panel::{LeftPanelAction, LeftPanelView},
            WorkspaceAction,
        },
    };
    #[cfg(target_os = "macos")]
    use ::settings::Setting as _;
    use warpui::integration::{Builder, TestStep, ARTIFACTS_DIR_ENV_VAR};

    std::fs::create_dir_all(&directory)?;
    let directory = directory
        .canonicalize()?
        .join(format!("capture-{}", std::process::id()));
    std::fs::create_dir(&directory)?;
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../specs/agent-communication-v2/panel-fixtures.json"
    ))?;
    // Cover startup stalls too; the driver's watchdog starts only after initialization.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(300));
        eprintln!("Native collaboration capture exceeded its 300-second deadline");
        std::process::exit(1);
    });
    let output = directory.clone();
    let mut driver = Builder::new(std::env::temp_dir())
        .use_tmp_filesystem_for_test_root_directory()
        .with_setup(move |utils| {
            utils.set_env("WARP_INTEGRATION", Some("1"));
            utils.set_env("WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS", Some("1"));
            utils.set_env("WARP_COLLABORATION_PREVIEW", Some("1"));
            utils.set_env(
                "WARP_DATA_PROFILE",
                Some(format!("collaboration-capture-{}", std::process::id())),
            );
            utils.set_env(ARTIFACTS_DIR_ENV_VAR, Some(&output));
            // Worker subprocesses must take their normal worker entrypoint.
            utils.set_env::<_, &str>("WARP_COLLABORATION_CAPTURE", None);
        })
        .with_step(TestStep::new("wait for workspace").add_named_assertion(
            "workspace exists",
            |app, window| {
                warpui::async_assert!(app.root_view::<RootView>(window).is_some_and(|root| {
                    root.read(app, |root, _| root.workspace_view().is_some())
                }))
            },
        ))
        .with_step(
            TestStep::new("open tools panel").with_action(|app, window, _| {
                app.update(|ctx| {
                    let origin = ctx.window_bounds(&window).unwrap().origin();
                    ctx.set_and_cache_window_bounds(
                        window,
                        pathfinder_geometry::rect::RectF::new(
                            origin,
                            pathfinder_geometry::vector::vec2f(1200., 800.),
                        ),
                    );
                });
                let root = app.root_view::<RootView>(window).unwrap();
                let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                workspace.update(app, |workspace, ctx| {
                    if !workspace.is_left_panel_open(ctx) {
                        workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx);
                    }
                });
            }),
        )
        .with_step(TestStep::new("wait for tools panel").add_named_assertion(
            "tools panel exists",
            |app, window| {
                warpui::async_assert!(app.views_of_type::<LeftPanelView>(window).is_some())
            },
        ))
        .with_step(
            TestStep::new("select collaboration").with_action(|app, window, _| {
                let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.handle_action_with_force_open(
                        &LeftPanelAction::Collaboration,
                        false,
                        ctx,
                    );
                });
            }),
        )
        .with_step(TestStep::new("wait for fixture panel").add_named_assertion(
            "fixture panel exists",
            |app, window| {
                warpui::async_assert!(app.views_of_type::<CollaborationPanel>(window).is_some())
            },
        ))
        .with_step(
            TestStep::new("seed unsent draft")
                .with_typed_characters(&["unsent collaboration draft"])
                .add_named_assertion("draft entered", |app, window| {
                    warpui::async_assert!(
                        checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        );
    let mut filenames = Vec::new();
    for (theme_name, theme) in [("light", ThemeKind::Light), ("dark", ThemeKind::Dark)] {
        for width in [320, 600] {
            for zoom in [1.0, 1.25] {
                for (selected, fixture) in fixtures.iter().enumerate() {
                    let filename = format!("{theme_name}-{width}-{zoom}-{}.png", fixture.state);
                    filenames.push(filename.clone());
                    let theme = theme.clone();
                    driver = driver.with_step(
                        TestStep::new(&filename)
                            .with_action(move |app, window, _| {
                                app.update(|ctx| {
                                    let origin = ctx.window_bounds(&window).unwrap().origin();
                                    ctx.set_and_cache_window_bounds(
                                        window,
                                        pathfinder_geometry::rect::RectF::new(
                                            origin,
                                            pathfinder_geometry::vector::vec2f(1200., 800.),
                                        ),
                                    );
                                    let colors = Settings::theme_for_theme_kind(&theme, ctx);
                                    Appearance::handle(ctx).update(ctx, |appearance, ctx| {
                                        appearance.set_theme(colors, ctx);
                                    });
                                    ctx.set_zoom_factor(zoom);
                                    let sizes =
                                        ResizableData::as_ref(ctx).get_all_handles(window).unwrap();
                                    sizes
                                        .left_panel_width
                                        .lock()
                                        .unwrap()
                                        .set_size(width as f32);
                                });
                                let panel =
                                    app.views_of_type::<CollaborationPanel>(window).unwrap()[0]
                                        .clone();
                                panel.update(app, |panel, ctx| {
                                    panel.selected = selected;
                                    panel.scroll = Default::default();
                                    ctx.notify();
                                });
                            })
                            .with_take_screenshot(filename),
                    );
                }
            }
        }
    }
    driver = driver
        .with_step(
            TestStep::new("focus tools from keyboard action")
                .with_action(|app, window, _| {
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace =
                        root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    workspace.update(app, |workspace, ctx| {
                        workspace.handle_action(&WorkspaceAction::FocusRightPanel, ctx)
                    });
                })
                .add_named_assertion("collaboration focused", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        app.update(|ctx| panel.is_self_or_child_focused(ctx))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        )
        .with_step(
            TestStep::new("right wraps preview state")
                .with_keystrokes(&["right"])
                .add_named_assertion("state advanced without changing draft", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        app.update(|ctx| panel.is_self_or_child_focused(ctx))
                            && panel.read(app, |panel, _| panel.selected == 0)
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        )
        .with_step(
            TestStep::new("left wraps preview state")
                .with_keystrokes(&["left"])
                .add_named_assertion("previous state wraps", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.selected + 1 == panel.fixtures.len())
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        )
        .with_step(
            TestStep::new("right returns to first preview")
                .with_keystrokes(&["right"])
                .add_named_assertion("first state restored", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel.selected == 0))
                }),
        )
        .with_step(
            TestStep::new("enter advances preview state")
                .with_keystrokes(&["enter"])
                .add_named_assertion("enter belongs to panel", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.selected == 1)
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        )
        .with_step(
            TestStep::new("select overflowing detail").with_action(|app, window, _| {
                app.update(|ctx| {
                    let sizes = ResizableData::as_ref(ctx).get_all_handles(window).unwrap();
                    sizes.left_panel_width.lock().unwrap().set_size(320.);
                });
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.selected = 2;
                    panel.scroll = Default::default();
                    ctx.notify();
                });
            }),
        )
        .with_step(
            TestStep::new("page down scrolls detail")
                .with_keystrokes(&["pagedown", "pagedown", "pagedown"])
                .add_named_assertion("detail scrolled", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.scroll.scroll_start().as_f32() > 0.)
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("detail-scrolled.png"),
        )
        .with_step(
            TestStep::new("page up restores detail")
                .with_keystrokes(&["pageup", "pageup", "pageup"])
                .add_named_assertion("detail at top", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.scroll.scroll_start().as_f32() == 0.)
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("detail-restored.png"),
        )
        .with_step(
            TestStep::new("escape restores terminal focus")
                .with_keystrokes(&["escape"])
                .add_named_assertion("terminal focused with original draft", |app, window| {
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace =
                        root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    warpui::async_assert!(
                        workspace.update(app, |workspace, ctx| workspace
                            .active_tab_pane_group()
                            .is_self_or_child_focused(ctx))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        );
    filenames.extend([
        "detail-scrolled.png".to_owned(),
        "detail-restored.png".to_owned(),
    ]);
    driver = driver
        .with_step(
            TestStep::new("wait for local live context").add_named_assertion(
                "context ready",
                |app, window| {
                    warpui::async_assert!(app.read(|ctx| !super::AgentCommunication::as_ref(ctx)
                        .busy
                        && crate::workspace::ActiveSession::as_ref(ctx)
                            .path_if_local(window)
                            .is_some()))
                },
            ),
        )
        .with_step(
            TestStep::new("enable isolated live projection").with_action(|app, window, _| {
                app.update(|ctx| {
                    super::AgentCommunication::handle(ctx).update(ctx, |model, ctx| {
                        model.preferences.enabled = true;
                        ctx.notify();
                    });
                });
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.preview = false;
                    panel.refresh(ctx);
                    CollaborationPanel::schedule(ctx);
                    ctx.notify();
                });
            }),
        )
        .with_step(
            TestStep::new("live empty projection")
                .add_named_assertion("empty snapshot with draft", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(
                            |snapshot| snapshot.tasks.is_empty() && snapshot.agents.is_empty()
                        )) && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-empty.png"),
        )
        .with_step(
            TestStep::new("seed deterministic live IPC work").with_action(|app, window, _| {
                let root = app.read(|ctx| {
                    warp_agent_bus::project_root(
                        crate::workspace::ActiveSession::as_ref(ctx)
                            .path_if_local(window)
                            .unwrap(),
                    )
                    .unwrap()
                });
                seed_live_checkpoint(&root).unwrap();
            }),
        )
        .with_step(TestStep::new("live tasks projection").add_named_assertion(
            "native task and offline identity",
            |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(
                    panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(
                        |snapshot| snapshot.tasks.len() == 1
                            && snapshot.agents.iter().any(|agent| !agent.online
                                && agent.device.as_deref() == Some("local")
                                && agent.workspace.as_deref() == Some(snapshot.project.as_str())
                                && agent.last_observed_ms.is_none())
                    )) && checkpoint_draft(app, window) == "unsent collaboration draft"
                )
            },
        ));
    filenames.push("live-empty.png".into());
    for detail in [false, true] {
        if detail {
            driver = driver
                .with_step(
                    TestStep::new("select live task").with_action(|app, window, _| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        panel.update(app, |panel, ctx| {
                            let id = panel.snapshot.as_ref().unwrap().tasks[0].id.clone();
                            panel.handle_action(&Action::SelectTask(id), ctx);
                        });
                    }),
                )
                .with_step(
                    TestStep::new("live original attempt detail").add_named_assertion(
                        "uncertain execution remains running",
                        |app, window| {
                            let panel =
                                app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                            warpui::async_assert!(panel.read(app, |panel, _| panel
                                .snapshot
                                .as_ref()
                                .is_some_and(|snapshot| snapshot
                                    .task
                                    .as_ref()
                                    .is_some_and(|task| task.state == "running")
                                    && snapshot.task_runtime.as_ref().is_some_and(
                                        |runtime| !runtime.online && runtime.interrupted
                                    ))))
                        },
                    ),
                );
        }
        for (theme_name, theme) in [("light", ThemeKind::Light), ("dark", ThemeKind::Dark)] {
            for width in [320, 600] {
                for zoom in [1.0, 1.25] {
                    let theme = theme.clone();
                    let filename = format!(
                        "live-{}-{theme_name}-{width}-{zoom}.png",
                        if detail { "detail" } else { "tasks" }
                    );
                    filenames.push(filename.clone());
                    driver = driver.with_step(
                        TestStep::new(&filename)
                            .with_action(move |app, window, _| {
                                app.update(|ctx| {
                                    let colors = Settings::theme_for_theme_kind(&theme, ctx);
                                    Appearance::handle(ctx).update(ctx, |appearance, ctx| {
                                        appearance.set_theme(colors, ctx)
                                    });
                                    ctx.set_zoom_factor(zoom);
                                    ResizableData::as_ref(ctx)
                                        .get_all_handles(window)
                                        .unwrap()
                                        .left_panel_width
                                        .lock()
                                        .unwrap()
                                        .set_size(width as f32);
                                });
                                let panel =
                                    app.views_of_type::<CollaborationPanel>(window).unwrap()[0]
                                        .clone();
                                panel.update(app, |panel, ctx| {
                                    panel.scroll = Default::default();
                                    ctx.notify();
                                });
                            })
                            .with_take_screenshot(filename),
                    );
                }
            }
        }
    }
    driver = driver
        .with_step(
            TestStep::new("focus live detail").with_action(|app, window, _| {
                let root = app.root_view::<RootView>(window).unwrap();
                let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                workspace.update(app, |workspace, ctx| {
                    workspace.handle_action(&WorkspaceAction::FocusRightPanel, ctx)
                });
            }),
        )
        .with_step(
            TestStep::new("live detail scrolling")
                .with_keystrokes(&["pagedown", "pagedown", "pagedown"])
                .add_named_assertion(
                    "live detail scroll reaches lower controls",
                    |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        warpui::async_assert!(
                            panel.read(app, |panel, _| panel.scroll.scroll_start().as_f32() > 0.)
                                && checkpoint_draft(app, window) == "unsent collaboration draft"
                        )
                    },
                )
                .with_take_screenshot("live-detail-end.png"),
        )
        .with_step(
            TestStep::new("live escape restores original draft")
                .with_keystrokes(&["escape"])
                .add_named_assertion("draft and terminal focus preserved", |app, window| {
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace =
                        root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    warpui::async_assert!(
                        workspace.update(app, |workspace, ctx| workspace
                            .active_tab_pane_group()
                            .is_self_or_child_focused(ctx))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                }),
        );
    filenames.push("live-detail-end.png".into());
    driver = driver
        .with_step(
            TestStep::new("reject unconfirmed execution override")
                .with_action(|app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.open_control(controls::Kind::ForceCancel, ctx);
                        panel.fill_control_checkpoint(
                            &["Investigated the disconnected fixture", "yes"],
                            ctx,
                        );
                        panel.confirm_control(ctx);
                    });
                })
                .add_named_assertion("typed confirmation required", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.control_checkpoint_rejected()
                            && panel
                                .snapshot
                                .as_ref()
                                .and_then(|snapshot| snapshot.task.as_ref())
                                .is_some_and(|task| task.state == "running"))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-override-confirmation.png"),
        )
        .with_step(
            TestStep::new("confirm native operator override").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.fill_control_checkpoint(
                        &["Investigated the disconnected fixture", "ALLOW OVERLAP"],
                        ctx,
                    );
                    panel.confirm_control(ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("operator cancellation preserves unknown effects")
                .add_named_assertion("cancelled without claiming stopped", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.form.is_none()
                            && panel
                                .snapshot
                                .as_ref()
                                .and_then(|snapshot| snapshot.task.as_ref())
                                .is_some_and(|task| task.state == "cancelled"
                                    && task
                                        .attempts
                                        .iter()
                                        .any(|attempt| attempt.certainty == "unknown"
                                            && attempt.finished_at.is_none())))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-cancelled.png"),
        )
        .with_step(
            TestStep::new("normal retry cannot overlap unknown execution").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.open_control(controls::Kind::Retry, ctx);
                        panel.fill_control_checkpoint(&["Retry the deterministic fixture"], ctx);
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("retry blocked by execution uncertainty")
                .add_named_assertion("unknown execution rejected", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .control_checkpoint_rejected()
                        && panel
                            .snapshot
                            .as_ref()
                            .and_then(|snapshot| snapshot.task.as_ref())
                            .is_some_and(|task| task.state == "cancelled")))
                })
                .with_take_screenshot("live-retry-blocked.png"),
        )
        .with_step(
            TestStep::new("explicit retry override").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.cancel_control(ctx);
                    panel.open_control(controls::Kind::RetryOverride, ctx);
                    panel.fill_control_checkpoint(
                        &["Reviewed previous unknown execution", "ALLOW OVERLAP"],
                        ctx,
                    );
                    panel.confirm_control(ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("native retry retains earlier attempt")
                .add_named_assertion(
                    "new revision with original unknown attempt",
                    |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        warpui::async_assert!(
                            panel.read(app, |panel, _| panel.form.is_none()
                                && panel
                                    .snapshot
                                    .as_ref()
                                    .and_then(|snapshot| snapshot.task.as_ref())
                                    .is_some_and(|task| task.revision == 2
                                        && matches!(task.state.as_str(), "queued" | "blocked")
                                        && task
                                            .attempts
                                            .iter()
                                            .any(|attempt| attempt.certainty == "unknown")))
                                && checkpoint_draft(app, window) == "unsent collaboration draft"
                        )
                    },
                )
                .with_take_screenshot("live-retried.png"),
        );
    filenames.extend(
        [
            "live-override-confirmation.png",
            "live-cancelled.png",
            "live-retry-blocked.png",
            "live-retried.png",
        ]
        .map(str::to_owned),
    );
    let shared_fixture = directory.join("owned-shared-checkout");
    std::fs::create_dir(&shared_fixture)?;
    let shared_root = warp_agent_bus::project_root(&shared_fixture)?;
    let mapping_root = shared_root.clone();
    let original_view = std::sync::Arc::new(std::sync::Mutex::new(None));
    let saved_view = original_view.clone();
    let shared_client = std::sync::Arc::new(std::sync::Mutex::new(None));
    let prepared_client = shared_client.clone();
    let shared_terminal_view = std::sync::Arc::new(std::sync::Mutex::new(None));
    let file_source_view = shared_terminal_view.clone();
    driver = driver
        .with_step(
            TestStep::new("create native shared space").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.handle_action(&Action::Back, ctx);
                    panel.handle_action(&Action::Spaces, ctx);
                    panel.open_control(controls::Kind::CreateSpace, ctx);
                    panel.fill_control_checkpoint(&["Native reviewed collaboration"], ctx);
                    panel.confirm_control(ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("native space created")
                .add_named_assertion("space visible with original draft", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.form.is_none()
                            && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                .spaces
                                .iter()
                                .any(|space| space.name == "Native reviewed collaboration")))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-space-created.png"),
        )
        .with_step(
            TestStep::new("map owned checkout from native form").with_action(
                move |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let space = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .spaces
                            .iter()
                            .find(|space| space.name == "Native reviewed collaboration")
                            .unwrap()
                            .id
                            .clone()
                            .unwrap();
                        panel.open_control(controls::Kind::MapWorkspace, ctx);
                        panel.fill_control_checkpoint(&[&space, &mapping_root, ""], ctx);
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("mapped checkout visible")
                .add_named_assertion("private scope retained", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.form.is_none()
                            && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                .admission
                                == "private"
                                && snapshot
                                    .spaces
                                    .iter()
                                    .any(|space| !space.workspaces.is_empty())))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-workspace-mapped.png"),
        )
        .with_step(
            TestStep::new("review exact native admission")
                .with_action(|app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let id = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .spaces
                            .iter()
                            .flat_map(|space| &space.workspaces)
                            .next()
                            .unwrap()
                            .id
                            .clone();
                        panel.handle_action(&Action::PreviewWorkspace(id), ctx);
                    });
                })
                .with_take_screenshot("live-workspace-reviewed.png"),
        )
        .with_step(
            TestStep::new("confirm new shared tab").with_action(move |app, window, _| {
                *saved_view.lock().unwrap() = app.read(|ctx| {
                    crate::workspace::ActiveSession::as_ref(ctx).terminal_view_id(window)
                });
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.handle_action(&Action::ConfirmWorkspace, ctx)
                });
            }),
        )
        .with_step(
            TestStep::new("new shared tab keeps reviewed native root")
                // A cold Windows shell can outlast the default ten-second assertion budget.
                .set_timeout(std::time::Duration::from_secs(45))
                .add_named_assertion("exact native project", move |app, window| {
                    warpui::async_assert!(app.read(|ctx| crate::workspace::ActiveSession::as_ref(
                        ctx
                    )
                    .path_if_local(window)
                    .and_then(|path| warp_agent_bus::project_root(path).ok())
                    .as_deref()
                        == Some(shared_root.as_str())))
                })
                .with_take_screenshot("live-workspace-shared-tab.png"),
        )
        .with_step(
            TestStep::new("new shared tab reconnects task panel").add_named_assertion(
                "current projection connected",
                |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel.connected))
                },
            ),
        )
        .with_step(
            TestStep::new("new tab uses reviewed shared scope").add_named_assertion(
                "pending admission precedes native discovery",
                |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.admission == "shared"
                            && snapshot.project.starts_with("space:")
                            && snapshot.agents.is_empty())))
                },
            ),
        )
        .with_step(
            TestStep::new("register deterministic shared participant").with_action(
                move |app, window, _| {
                    let root = app.read(|ctx| {
                        warp_agent_bus::project_root(
                            crate::workspace::ActiveSession::as_ref(ctx)
                                .path_if_local(window)
                                .unwrap(),
                        )
                        .unwrap()
                    });
                    let broker = super::BROKER.get().unwrap();
                    let spaces = broker
                        .control(
                            &root,
                            &warp_agent_bus::ControllerOperation::SpaceList {
                                cursor: None,
                                limit: None,
                            },
                        )
                        .unwrap();
                    let workspace = spaces["spaces"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .flat_map(|space| space["workspaces"].as_array().unwrap())
                        .find(|workspace| workspace["root"] == root)
                        .unwrap()["id"]
                        .as_str()
                        .unwrap();
                    let mut participant = register_capture_participant(
                        "capture-shared-worker",
                        "shared-capture-worker",
                        &root,
                        Some(workspace),
                    )
                    .unwrap();
                    let domain = format!(
                        "space:{}",
                        spaces["spaces"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|space| space["name"] == "Native reviewed collaboration")
                            .unwrap()["id"]
                            .as_str()
                            .unwrap()
                    );
                    let task = broker
                        .operator(
                            &domain,
                            &warp_agent_bus::Operation::TaskAssign {
                                to: "shared-capture-worker".into(),
                                description: "Inspect the owned evidence source".into(),
                                acceptance: "Open its original producing file".into(),
                                reviewer: None,
                                dependencies: vec![],
                                start_deadline: None,
                                execution_timeout_seconds: None,
                                review_timeout_seconds: None,
                                request_id: uuid::Uuid::new_v4().to_string(),
                            },
                        )
                        .unwrap();
                    let task_id = task["id"].as_str().unwrap().to_owned();
                    participant.operation = warp_agent_bus::Operation::TaskStart {
                        task_id: task_id.clone(),
                        revision: 1,
                        expected_version: None,
                        request_id: uuid::Uuid::new_v4().to_string(),
                    };
                    warp_agent_bus::transport::call(&broker.endpoint, &participant).unwrap();
                    std::fs::write(
                        std::path::Path::new(&root).join("capture-evidence.rs"),
                        "// Owned native evidence fixture.\npub fn capture_value() -> u32 { 42 }\n",
                    )
                    .unwrap();
                    participant.operation = warp_agent_bus::Operation::EvidenceAdd {
                        task_id,
                        kind: "file".into(),
                        attempt_id: None,
                        path: Some("capture-evidence.rs".into()),
                        hash: None,
                        commit: None,
                        repository: None,
                        branch: None,
                        base: None,
                        head: None,
                        command: None,
                        outcome: None,
                        exit_code: None,
                        summary: Some("Owned native source reference".into()),
                        request_id: uuid::Uuid::new_v4().to_string(),
                    };
                    warp_agent_bus::transport::call(&broker.endpoint, &participant).unwrap();
                    *prepared_client.lock().unwrap() = Some(participant);
                },
            ),
        )
        .with_step(
            TestStep::new("shared participant is visible").add_named_assertion(
                "online mapped participant",
                |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot
                            .agents
                            .iter()
                            .any(|row| row.online && row.agent.name == "shared-capture-worker"))))
                },
            ),
        )
        .with_step(
            TestStep::new("select shared file evidence detail").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    let id = panel.snapshot.as_ref().unwrap().tasks[0].id.clone();
                    panel.handle_action(&Action::SelectTask(id), ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("shared evidence retains producing checkout")
                .add_named_assertion("local file descriptor is reported", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.task.as_ref())
                        .is_some_and(|task| task.evidence_records.len() == 1
                            && task.evidence_records[0].path.as_deref()
                                == Some("capture-evidence.rs")
                            && !task.evidence_records[0].verified)))
                })
                .with_take_screenshot("live-evidence-shared-detail.png"),
        )
        .with_step(
            TestStep::new("open original local evidence in native viewer").with_action(
                move |app, window, _| {
                    *file_source_view.lock().unwrap() = app.read(|ctx| {
                        crate::workspace::ActiveSession::as_ref(ctx).terminal_view_id(window)
                    });
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let id = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .task
                            .as_ref()
                            .unwrap()
                            .evidence_records[0]
                            .id
                            .clone();
                        panel.handle_action(&Action::OpenEvidence(id), ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native evidence source is opened")
                .with_action(|app, window, _| {
                    // README captures use the default warm palette and macOS vertical tabs.
                    app.update(|ctx| {
                        #[cfg(target_os = "macos")]
                        {
                            crate::workspace::tab_settings::TabSettings::handle(ctx).update(
                                ctx,
                                |settings, ctx| {
                                    settings
                                        .use_vertical_tabs
                                        .set_value(true, ctx)
                                        .expect("Enable vertical tabs for macOS README captures");
                                },
                            );
                            let origin = ctx.window_bounds(&window).unwrap().origin();
                            ctx.set_and_cache_window_bounds(
                                window,
                                pathfinder_geometry::rect::RectF::new(
                                    origin,
                                    pathfinder_geometry::vector::vec2f(1440., 1000.),
                                ),
                            );
                        }
                        #[cfg(not(target_os = "macos"))]
                        let _ = window;
                        let theme =
                            Settings::theme_for_theme_kind(&ThemeKind::ClaudeWarmLight, ctx);
                        Appearance::handle(ctx).update(ctx, |appearance, ctx| {
                            appearance.set_theme(theme, ctx);
                        });
                    });
                })
                .add_named_assertion("existing code viewer has the owned file", |app, window| {
                    let views = app
                        .views_of_type::<crate::code::view::CodeView>(window)
                        .unwrap_or_default();
                    warpui::async_assert!(views.iter().any(|view| view.read(app, |view, ctx| view
                        .local_path(ctx)
                        .is_some_and(|path| path
                            .file_name()
                            .is_some_and(|name| name == "capture-evidence.rs")))))
                })
                .add_named_assertion(
                    "macOS documentation uses visible vertical tabs",
                    |app, window| {
                        warpui::async_assert!(
                            !cfg!(target_os = "macos")
                                || app
                                    .root_view::<RootView>(window)
                                    .is_some_and(|root| root.read(app, |root, ctx| root
                                        .workspace_view()
                                        .is_some_and(|workspace| workspace
                                            .as_ref(ctx)
                                            .snapshot(window, false, ctx)
                                            .vertical_tabs_panel_open)))
                        )
                    },
                )
                .with_take_screenshot("live-evidence-file-open.png"),
        )
        .with_step(
            TestStep::new("restore originating shared pane after file view").with_action(
                move |app, window, _| {
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace =
                        root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    let terminal_view_id = shared_terminal_view.lock().unwrap().unwrap();
                    workspace.update(app, |workspace, ctx| {
                        workspace.handle_action(
                            &WorkspaceAction::FocusTerminalViewInWorkspace { terminal_view_id },
                            ctx,
                        )
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("shared pane is restored before departure").add_named_assertion(
                "shared admission is current",
                |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel.connected
                        && panel
                            .snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.admission == "shared"
                                && snapshot
                                    .agents
                                    .iter()
                                    .any(|row| row.agent.name == "shared-capture-worker"))))
                },
            ),
        )
        .with_step(
            TestStep::new("remove shared participation through native form").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.handle_action(&Action::Spaces, ctx);
                        panel.open_control(controls::Kind::LeaveSpace, ctx);
                        panel.fill_control_checkpoint(&["shared-capture-worker"], ctx);
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("shared departure fences original capability")
                .add_named_assertion(
                    "revoked participant cannot read or rejoin",
                    move |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        let closed = panel.read(app, |panel, _| panel.form.is_none());
                        let revoked = if closed {
                            let mut request =
                                shared_client.lock().unwrap().as_ref().unwrap().clone();
                            request.operation = warp_agent_bus::Operation::AgentList;
                            warp_agent_bus::transport::call(
                                &super::BROKER.get().unwrap().endpoint,
                                &request,
                            )
                            .is_err()
                        } else {
                            false
                        };
                        warpui::async_assert!(closed && revoked)
                    },
                )
                .with_take_screenshot("live-workspace-departure.png"),
        )
        .with_step(
            TestStep::new("return to original private pane").with_action(move |app, window, _| {
                let root = app.root_view::<RootView>(window).unwrap();
                let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                let terminal_view_id = original_view.lock().unwrap().unwrap();
                workspace.update(app, |workspace, ctx| {
                    workspace.handle_action(
                        &WorkspaceAction::FocusTerminalViewInWorkspace { terminal_view_id },
                        ctx,
                    )
                });
            }),
        )
        .with_step(
            TestStep::new("private work and draft survive shared tab")
                .add_named_assertion("original private task and draft retained", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.connected
                            && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                .admission
                                == "private"
                                && snapshot.tasks.len() == 1
                                && snapshot.tasks[0].revision == 2))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-workspace-private-retained.png"),
        );
    filenames.extend(
        [
            "live-space-created.png",
            "live-workspace-mapped.png",
            "live-workspace-reviewed.png",
            "live-workspace-shared-tab.png",
            "live-workspace-private-retained.png",
            "live-workspace-departure.png",
            "live-evidence-shared-detail.png",
            "live-evidence-file-open.png",
        ]
        .map(str::to_owned),
    );
    driver = driver
        .with_step(
            TestStep::new("create native pool with deadline policy").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.open_control(controls::Kind::Pool, ctx);
                        panel.fill_control_checkpoint(
                            &[
                                "capture-worker",
                                "Run the deterministic pool fixture",
                                "Record the outcome",
                                "",
                                "2030-01-01T09:00:00Z",
                                "60",
                                "120",
                                "",
                            ],
                            ctx,
                        );
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native pool created without invented execution").add_named_assertion(
                "unclaimed pool and draft retained",
                |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.form.is_none()
                            && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                .tasks
                                .len()
                                == 2
                                && snapshot.tasks.iter().any(|task| task.assignee.is_empty())))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                },
            ),
        )
        .with_step(
            TestStep::new("open native pool scheduling detail").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    let id = panel
                        .snapshot
                        .as_ref()
                        .unwrap()
                        .tasks
                        .iter()
                        .find(|task| task.assignee.is_empty())
                        .unwrap()
                        .id
                        .clone();
                    panel.handle_action(&Action::SelectTask(id), ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("pool detail retains eligible participants and timeouts")
                .add_named_assertion("explicit policy is persisted", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.task.as_ref())
                        .is_some_and(|task| task.eligible.len() == 1
                            && task.attempts.is_empty()
                            && task.start_deadline.is_some()
                            && task.execution_timeout_seconds == Some(60)
                            && task.review_timeout_seconds == Some(120))))
                })
                .with_take_screenshot("live-pool-policy.png"),
        );
    filenames.push("live-pool-policy.png".into());
    driver = driver
        .with_step(
            TestStep::new("filter native queued tasks by participant").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let worker = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .agents
                            .iter()
                            .find(|row| row.agent.name == "capture-worker")
                            .unwrap()
                            .agent
                            .id
                            .clone();
                        panel.handle_action(&Action::FilterTaskState, ctx);
                        panel.handle_action(&Action::FilterTaskAssignee(Some(worker)), ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native participant filter excludes unassigned pool")
                .add_named_assertion("one explicitly assigned queued task", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .query
                        .task_state
                        .as_deref()
                        == Some("queued")
                        && panel
                            .snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.tasks.len() == 1
                                && snapshot.tasks[0].assignee
                                    == *panel.query.task_assignee.as_ref().unwrap())))
                }),
        )
        .with_step(
            TestStep::new("switch native task state and archive filter").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.handle_action(&Action::FilterTaskState, ctx);
                        panel.handle_action(&Action::ToggleArchived, ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native filtered empty state retains draft")
                .add_named_assertion("blocked filter is empty and explicit", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.query.task_state.as_deref()
                            == Some("blocked")
                            && panel.query.include_archived
                            && panel
                                .snapshot
                                .as_ref()
                                .is_some_and(|snapshot| snapshot.tasks.is_empty()))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-task-filtered.png"),
        );
    filenames.push("live-task-filtered.png".into());
    driver = driver
        .with_step(
            TestStep::new("search native thread with literal wildcard text").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.open_control(controls::Kind::Search, ctx);
                        panel.fill_control_checkpoint(&["literal _%"], ctx);
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native literal search returns only matching root")
                .add_named_assertion("escaped substring with retained draft", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.show_messages
                            && panel.form.is_none()
                            && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                .messages
                                .len()
                                == 1
                                && snapshot.messages[0].body
                                    == "Native thread checkpoint literal _% text"))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-message-search.png"),
        )
        .with_step(
            TestStep::new("open native immutable thread history").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    let message = &panel.snapshot.as_ref().unwrap().messages[0];
                    let thread = message.thread_id.as_ref().unwrap_or(&message.id).clone();
                    panel.handle_action(&Action::OpenThread(thread), ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("native thread retains original and correction")
                .add_named_assertion("two ordered immutable messages", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(
                            |snapshot| snapshot.messages.len() == 2
                                && snapshot.messages[0].body
                                    == "Native thread checkpoint literal _% text"
                                && snapshot.messages[1].body == "Follow-up native checkpoint"
                                && snapshot.messages[1].reply_to.as_deref()
                                    == Some(snapshot.messages[0].id.as_str())
                        )) && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-thread-history.png"),
        );
    filenames.extend(["live-message-search.png", "live-thread-history.png"].map(str::to_owned));
    driver = driver
        .with_step(
            TestStep::new("open native history and capacity").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| panel.handle_action(&Action::History, ctx));
            }),
        )
        .with_step(
            TestStep::new("history page preserves protected work")
                .add_named_assertion(
                    "bounded ordered export and storage budget",
                    |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        warpui::async_assert!(
                            panel.read(app, |panel, _| panel.query.history
                                && panel
                                    .snapshot
                                    .as_ref()
                                    .and_then(|snapshot| snapshot.history.as_ref())
                                    .is_some_and(|history| !history.records.is_empty()
                                        && history.records.len() <= 50
                                        && history.capacity["database"]["hard_limit"]
                                            .as_u64()
                                            .is_some()
                                        && history
                                            .records
                                            .windows(2)
                                            .all(|rows| rows[0]["sequence"].as_u64()
                                                < rows[1]["sequence"].as_u64())))
                                && checkpoint_draft(app, window) == "unsent collaboration draft"
                        )
                    },
                )
                .with_take_screenshot("live-history-capacity.png"),
        );
    // Clipboard acceptance runs only on isolated cloud runners, never against a daily local clipboard.
    if std::env::var_os("GITHUB_ACTIONS").is_some_and(|value| value == "true") {
        let expected = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured = expected.clone();
        driver = driver.with_step(
            TestStep::new("copy the explicit scoped native history page")
                .with_action(move |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        if let Some(snapshot) = &panel.snapshot {
                            if let Some(history) = &snapshot.history {
                                *captured.lock().unwrap() = Some(serde_json::json!({
                                    "project": snapshot.project, "after": panel.query.history_after,
                                    "cursor": history.cursor, "records": history.records,
                                }));
                            }
                        }
                        panel.handle_action(&Action::CopyHistory, ctx);
                    });
                })
                .add_named_assertion(
                    "export preserves original scope ordering and relationships",
                    move |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        let copied = panel.update(app, |_, ctx| ctx.clipboard().read());
                        let parsed =
                            serde_json::from_str::<serde_json::Value>(&copied.plain_text).ok();
                        let original = expected.lock().unwrap().clone();
                        warpui::async_assert!(
                            original.is_some()
                                && parsed == original
                                && copied.paths.is_none()
                                && copied.html.is_none()
                                && copied.images.is_none()
                                && checkpoint_draft(app, window) == "unsent collaboration draft"
                        )
                    },
                ),
        );
    }
    driver = driver
        .with_step(
            TestStep::new("require explicit native history deletion text")
                .with_action(|app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.open_control(controls::Kind::Purge, ctx);
                        panel.fill_control_checkpoint(&["yes"], ctx);
                        panel.confirm_control(ctx);
                        assert!(panel.control_checkpoint_rejected());
                    });
                })
                .with_take_screenshot("live-history-purge-confirmation.png"),
        )
        .with_step(
            TestStep::new("change history after the reviewed purge preview").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let project = panel.snapshot.as_ref().unwrap().project.clone();
                        super::BROKER
                            .get()
                            .unwrap()
                            .operator(
                                &project,
                                &warp_agent_bus::Operation::AgentSend {
                                    to: "capture-worker".into(),
                                    body: "Preserve work arriving after purge preview".into(),
                                    subject: None,
                                    thread_id: None,
                                    reply_to: None,
                                    task_id: None,
                                    request_id: uuid::Uuid::new_v4().to_string(),
                                },
                            )
                            .unwrap();
                        panel.fill_control_checkpoint(&["DELETE HISTORY"], ctx);
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native purge rejects changed history")
                .add_named_assertion("original preview is fenced", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.control_checkpoint_rejected())
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-history-purge-stale.png"),
        )
        .with_step(
            TestStep::new("archive only older completed history").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.cancel_control(ctx);
                    panel.open_control(controls::Kind::ArchiveAged, ctx);
                    panel.fill_control_checkpoint(&["1"], ctx);
                    panel.confirm_control(ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("native aged archive retains recent and uncertain work")
                .add_named_assertion("archive response and draft preserved", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.form.is_none()
                            && panel.query.history
                            && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                .tasks
                                .len()
                                == 2
                                && snapshot.history.as_ref().is_some_and(|history| history
                                    .capacity["tasks"]["used"]
                                    == 2
                                    && history.records.iter().any(|record| record["type"]
                                        == "task"
                                        && record["data"]["archived"] == false
                                        && record["data"]["attempts"]
                                            .as_array()
                                            .is_some_and(|attempts| attempts.iter().any(
                                                |attempt| attempt["certainty"] == "unknown"
                                            ))))))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-history-archive-aged.png"),
        );
    filenames.extend(
        [
            "live-history-capacity.png",
            "live-history-purge-confirmation.png",
            "live-history-purge-stale.png",
            "live-history-archive-aged.png",
        ]
        .map(str::to_owned),
    );
    driver = driver
        .with_step(
            TestStep::new("prepare native reservation maintenance fixture").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let root = warp_agent_bus::project_root(
                            crate::workspace::ActiveSession::as_ref(ctx)
                                .path_if_local(ctx.window_id())
                                .unwrap(),
                        )
                        .unwrap();
                        let mut request = register_capture_participant(
                            "capture-reservation-worker",
                            "capture-reservations",
                            &root,
                            None,
                        )
                        .unwrap();
                        request.operation = warp_agent_bus::Operation::FileReserve {
                            paths: vec!["native-renew-fixture.txt".into()],
                            mode: "exclusive".into(),
                            task_id: None,
                            attempt_id: None,
                            ttl_seconds: Some(600),
                            request_id: uuid::Uuid::new_v4().to_string(),
                        };
                        warp_agent_bus::transport::call(
                            &super::BROKER.get().unwrap().endpoint,
                            &request,
                        )
                        .unwrap();
                        panel.handle_action(&Action::Spaces, ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("native reservation fixture appears in original checkout")
                .add_named_assertion("scoped reservation metadata", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot
                            .reservations
                            .iter()
                            .any(
                                |lease| lease.path == "native-renew-fixture.txt" && !lease.expired
                            ))))
                }),
        )
        .with_step(
            TestStep::new("renew reviewed native reservation").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    let id = panel
                        .snapshot
                        .as_ref()
                        .unwrap()
                        .reservations
                        .iter()
                        .find(|lease| lease.path == "native-renew-fixture.txt")
                        .unwrap()
                        .id
                        .clone();
                    panel.open_control(controls::Kind::RenewReservation, ctx);
                    panel.fill_control_checkpoint(
                        &[&id, "120", "Reviewed active advisory lease"],
                        ctx,
                    );
                    panel.confirm_control(ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("native renewal commits without touching execution")
                .add_named_assertion("renewed event and retained draft", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.form.is_none()
                            && panel
                                .events
                                .iter()
                                .any(|event| event.kind == "reservation_renewed"))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-reservation-renewed.png"),
        )
        .with_step(
            TestStep::new("native release requires explicit reservation confirmation")
                .with_action(|app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let id = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .reservations
                            .iter()
                            .find(|lease| lease.path == "native-renew-fixture.txt")
                            .unwrap()
                            .id
                            .clone();
                        panel.open_control(controls::Kind::ReleaseReservation, ctx);
                        panel.fill_control_checkpoint(
                            &[&id, "Release advisory coordination", "yes"],
                            ctx,
                        );
                        panel.confirm_control(ctx);
                        assert!(panel.control_checkpoint_rejected());
                    });
                })
                .with_take_screenshot("live-reservation-release-confirmation.png"),
        )
        .with_step(
            TestStep::new("release reviewed advisory reservation").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    let id = panel
                        .snapshot
                        .as_ref()
                        .unwrap()
                        .reservations
                        .iter()
                        .find(|lease| lease.path == "native-renew-fixture.txt")
                        .unwrap()
                        .id
                        .clone();
                    panel.fill_control_checkpoint(
                        &[&id, "Release advisory coordination", "RELEASE RESERVATION"],
                        ctx,
                    );
                    panel.confirm_control(ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("native release removes only the selected coordination record")
                .add_named_assertion(
                    "original uncertain reservation and draft retained",
                    |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        warpui::async_assert!(
                            panel.read(app, |panel, _| panel.form.is_none()
                                && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                    .reservations
                                    .iter()
                                    .all(|lease| lease.path != "native-renew-fixture.txt")
                                    && snapshot
                                        .reservations
                                        .iter()
                                        .any(|lease| lease.path == "native-capture-fixture.txt")))
                                && checkpoint_draft(app, window) == "unsent collaboration draft"
                        )
                    },
                )
                .with_take_screenshot("live-reservation-released.png"),
        );
    filenames.extend(
        [
            "live-reservation-renewed.png",
            "live-reservation-release-confirmation.png",
            "live-reservation-released.png",
        ]
        .map(str::to_owned),
    );
    driver = driver
        .with_step(
            TestStep::new("review terminal-driven SSH guidance")
                .with_take_screenshot("live-ssh-selection.png"),
        )
        .with_step(
            TestStep::new("attach real native companion to existing panel").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.cancel_control(ctx);
                        panel.connect_remote_checkpoint(true, ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("receive real remote Agent projection")
                // Native Agent startup has a 30-second bound; allow projection polling too.
                .set_timeout(std::time::Duration::from_secs(45))
                // SDK discovery registers first; address the fixture only after its final rename.
                .add_named_assertion("native run and isolated scope", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| panel.connected
                            && panel.remote.is_some()
                            && panel.snapshot.as_ref().is_some_and(
                                |snapshot| uuid::Uuid::parse_str(&snapshot.project).is_ok()
                                    && snapshot.agents.iter().any(|row| row.online
                                        && row.agent.program == "fixture"
                                        && row.agent.name
                                            == format!("fixture-{}", row.agent.terminal)
                                        && row
                                            .run
                                            .as_ref()
                                            .is_some_and(|run| uuid::Uuid::parse_str(run).is_ok()))
                            ))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-ssh-agent.png"),
        )
        .with_step(
            TestStep::new("send explicit human message through remote panel").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let name = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .agents
                            .iter()
                            .find(|row| row.online)
                            .unwrap()
                            .agent
                            .name
                            .clone();
                        panel.open_control(controls::Kind::Send, ctx);
                        panel.fill_control_checkpoint(
                            &[&name, "Native remote panel message", "Panel checkpoint"],
                            ctx,
                        );
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("remote Store confirms the human message")
                .add_named_assertion("real remote message committed", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel.connected
                        && panel.form.is_none()
                        && panel
                            .events
                            .iter()
                            .any(|event| event.kind == "message_queued")))
                })
                .with_take_screenshot("live-ssh-message.png"),
        )
        .with_step(
            TestStep::new("assign a task through the remote native panel").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let name = panel
                            .snapshot
                            .as_ref()
                            .unwrap()
                            .agents
                            .iter()
                            .find(|row| row.online)
                            .unwrap()
                            .agent
                            .name
                            .clone();
                        panel.open_control(controls::Kind::Assign, ctx);
                        panel.fill_control_checkpoint(
                            &[
                                &name,
                                "Remote panel task",
                                "Native projection contains this task",
                                "",
                                "",
                                "",
                                "",
                                "",
                            ],
                            ctx,
                        );
                        panel.confirm_control(ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("receive remote task list")
                .add_named_assertion("remote Store task visible", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel.connected
                        && panel.form.is_none()
                        && panel
                            .snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.tasks.len() == 1)))
                })
                .with_take_screenshot("live-ssh-task.png"),
        )
        .with_step(
            TestStep::new("open remote task in existing detail view").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let id = panel.snapshot.as_ref().unwrap().tasks[0].id.clone();
                        panel.handle_action(&Action::SelectTask(id), ctx);
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("receive actual remote task details")
                .add_named_assertion("selected remote task detail matches", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.task.as_ref())
                        .is_some_and(|task| task.description == "Remote panel task"
                            && panel.query.selected_task.as_ref() == Some(&task.id))))
                })
                .with_take_screenshot("live-ssh-task-detail.png"),
        )
        .with_step(
            TestStep::new("return to remote Agents before drafting a message").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| panel.handle_action(&Action::Back, ctx));
                },
            ),
        )
        .with_step(
            TestStep::new("retain remote form on disconnect")
                .with_action(|app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        let name = panel.snapshot.as_ref().unwrap().agents[0]
                            .agent
                            .name
                            .clone();
                        panel.open_control(controls::Kind::Send, ctx);
                        panel.fill_control_checkpoint(
                            &[&name, "Unsent remote message", "Retain this draft"],
                            ctx,
                        );
                        panel.generation += 1;
                        panel.connected = false;
                        panel.remote_failed = true;
                        panel.status =
                            "Disconnected. Last remote state is stale; reconnect before writing."
                                .into();
                        let client = panel.remote_client.clone();
                        ctx.spawn(
                            async move {
                                if let Some(client) = client.lock().await.as_mut() {
                                    client.disconnect();
                                }
                            },
                            |_, _, _| {},
                        );
                        panel.confirm_control(ctx);
                        assert!(panel.control_checkpoint_draft("Unsent remote message", ctx));
                        ctx.notify();
                    });
                })
                .with_take_screenshot("live-ssh-disconnected-draft.png"),
        )
        .with_step(
            TestStep::new("reconnect private native project without replaying the draft")
                .with_action(|app, window, _| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.connect_remote_checkpoint(false, ctx)
                    });
                }),
        )
        .with_step(
            TestStep::new("reconnect preserves original remote intent")
                .add_named_assertion("remote draft and terminal draft retained", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, app| panel.connected
                            && panel.control_checkpoint_draft("Unsent remote message", app))
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                })
                .with_take_screenshot("live-ssh-reconnected-draft.png"),
        )
        .with_step(
            TestStep::new("return to original local project").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    panel.cancel_control(ctx);
                    panel.handle_action(&Action::UseLocal, ctx);
                });
            }),
        )
        .with_step(
            TestStep::new("local projection works after remote selection")
                .add_named_assertion("local authority restored", |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, _| panel.connected
                        && panel.remote.is_none()
                        && panel
                            .snapshot
                            .as_ref()
                            .is_some_and(
                                |snapshot| uuid::Uuid::parse_str(&snapshot.project).is_err()
                            )))
                })
                .with_take_screenshot("live-ssh-return-local.png"),
        );
    filenames.extend(
        [
            "live-ssh-selection.png",
            "live-ssh-agent.png",
            "live-ssh-message.png",
            "live-ssh-task.png",
            "live-ssh-task-detail.png",
            "live-ssh-disconnected-draft.png",
            "live-ssh-reconnected-draft.png",
            "live-ssh-return-local.png",
        ]
        .map(str::to_owned),
    );
    driver = driver.with_step(
        TestStep::new("close on-demand tasks without changing the terminal draft")
            .with_action(|app, window, _| {
                let root = app.root_view::<RootView>(window).unwrap();
                let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                workspace.update(app, |workspace, ctx| {
                    if workspace.is_left_panel_open(ctx) {
                        workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx);
                    }
                });
            })
            .add_named_assertion(
                "hidden panel is fenced and draft retained",
                |app, window| {
                    let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                    warpui::async_assert!(
                        panel.read(app, |panel, _| !panel.visible && !panel.connected)
                            && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                },
            ),
    );
    driver = driver.with_step(
        TestStep::new("open native communication settings").with_action(|app, window, _| {
            // Fixed visual data stays in the isolated debug profile and never configures a CLI.
            app.update(|ctx| {
                super::AgentCommunication::handle(ctx).update(ctx, |model, ctx| {
                    model.available = vec![
                        super::setup::Available {
                            command: "codex".into(),
                            program: "codex".into(),
                            installed: None,
                            status: "Not installed — install the CLI to enable project-local MCP access.".into(),
                        },
                        super::setup::Available {
                            command: "claude".into(),
                            program: "claude".into(),
                            installed: None,
                            status: "Unavailable — refresh agents after updating the executable search path.".into(),
                        },
                    ];
                    model.status = "Communication is enabled. No CLI agents selected.".into();
                    ctx.notify();
                });
            });
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            workspace.update(app, |workspace, ctx| {
                workspace.handle_action(
                    &WorkspaceAction::ShowSettingsPageWithSearch {
                        search_query: "agent communication".into(),
                        section: Some(crate::settings_view::SettingsSection::Features),
                    },
                    ctx,
                );
            });
        }),
    );
    for (theme_kind, theme_name) in [
        (ThemeKind::Light, "light"),
        (ThemeKind::Dark, "dark"),
        (ThemeKind::ClaudeWarmLight, "claude-warm"),
    ] {
        for zoom in [1., 1.25] {
            let filename = format!("settings-communication-{theme_name}-{zoom}.png");
            filenames.push(filename.clone());
            let theme_kind = theme_kind.clone();
            driver = driver.with_step(
                TestStep::new(&format!("native communication settings {theme_name} at {zoom}"))
                    .with_action(move |app, window, _| {
                        app.update(|ctx| {
                            let theme = Settings::theme_for_theme_kind(&theme_kind, ctx);
                            Appearance::handle(ctx).update(ctx, |appearance, ctx| {
                                appearance.set_theme(theme, ctx);
                            });
                            ctx.set_zoom_factor(zoom);
                        });
                    })
                    .add_named_assertion("communication settings is visible", |app, window| {
                        warpui::async_assert!(app
                            .views_of_type::<crate::settings_view::agent_communication::CommunicationSettingsView>(window)
                            .is_some_and(|views| !views.is_empty()))
                    })
                    .with_take_screenshot(filename),
            );
        }
    }
    let driver = driver.with_on_finish(move |_, _, _| {
        let directory = directory.clone();
        let filenames = filenames.clone();
        Box::pin(async move {
            let runs = directory.join("collaboration-static");
            let run = std::fs::read_dir(runs)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .max()
                .unwrap();
            for filename in filenames {
                assert!(
                    std::fs::metadata(run.join(&filename)).is_ok_and(|metadata| metadata.len() > 0),
                    "Missing native capture: {filename}"
                );
            }
        })
    });
    // Do not invoke the legacy HOME override; the isolated data profile owns preferences.
    crate::run_integration_test(driver.build("collaboration-static", false))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collaboration_fixtures_cover_required_states_and_long_content() {
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../../../specs/agent-communication-v2/panel-fixtures.json"
        ))
        .unwrap();
        for state in [
            "agents",
            "tasks",
            "detail",
            "empty",
            "loading",
            "disconnected",
            "stale",
            "capacity",
            "failed",
        ] {
            let fixture = fixtures
                .iter()
                .find(|fixture| fixture.state == state)
                .unwrap();
            assert!(!fixture.guidance.is_empty());
        }
        assert!(fixtures
            .iter()
            .flat_map(|fixture| &fixture.sections)
            .flat_map(|section| &section.rows)
            .any(|row| row.len() > 256));
    }
}
