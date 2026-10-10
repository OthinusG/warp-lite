//! Native collaboration projection; explicit sample mode retains the accepted fixtures.
use crate::appearance::Appearance;
use crate::ui_components::blended_colors;
use crate::view_components::dropdown::{Dropdown, DropdownItem};
#[path = "panel_controls.rs"]
mod controls;
#[path = "panel_usage.rs"]
mod usage;
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use warp_agent_bus::{
    companion::TaskCommand,
    ssh_files::SshConnection,
    ssh_remote::{HostClient, SshProfile},
};
use warp_agent_bus::{transport::PanelQuery, Agent, Event, Message, Reservation, Task};
use warpui::r#async::Timer;
use warpui::{
    accessibility::{AccessibilityContent, ActionAccessibilityContent, WarpA11yRole},
    elements::{
        Border, ClippedScrollStateHandle, ClippedScrollable, Container, DispatchEventResult, Element,
        EventHandler, Expanded, Fill, Flex, MouseStateHandle, Padding, ParentElement, ScrollbarWidth,
        Shrinkable, Wrap,
    },
    fonts::Weight,
    ui_components::{
        button::ButtonVariant,
        components::{UiComponent, UiComponentStyles},
    },
    units::IntoPixels,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

/// Shared panel spacing so every row, section and button run keeps one rhythm.
const GAP_TIGHT: f32 = 8.;
const GAP_ROW: f32 = 8.;
const GAP_SECTION: f32 = 8.;

// Text roles: semibold function headings;
// 12px secondary text carries status and guidance.
fn heading(appearance: &Appearance, text: impl Into<Cow<'static, str>>) -> Box<dyn Element> {
    let text = text.into();
    use crate::ui_components::icons::Icon;
    let icon = match text.as_ref() {
        "Agents" | "Coordinator" | "Agents Collaboration Mode" => Some(Icon::Users),
        "Worktrees" | "Worktree collaboration" => Some(Icon::GitBranch),
        "History" => Some(Icon::History),
        "Message" | "Messages" | "Conversation" => Some(Icon::MessageText),
        "Data usage" => Some(Icon::Dataflow),
        "Tasks" | "Assign" => Some(Icon::TaskListBlock),
        "Project" | "SSH project" => Some(Icon::Folder),
        _ => None,
    };
    let label = appearance
        .ui_builder()
        .span(text)
        .with_style(UiComponentStyles {
            font_weight: Some(Weight::Semibold),
            ..Default::default()
        })
        .with_soft_wrap()
        .build()
        .finish();
    if let Some(icon) = icon {
        Flex::row().with_spacing(GAP_ROW)
            .with_child(warpui::elements::ConstrainedBox::new(icon.to_warpui_icon(appearance.theme().foreground()).finish()).with_width(16.).with_height(16.).finish())
            .with_child(Shrinkable::new(1., label).finish()).finish()
    } else { label }
}

fn section(appearance: &Appearance, content: Box<dyn Element>) -> Box<dyn Element> {
    Container::new(content)
        .with_padding(Padding::uniform(GAP_SECTION))
        .with_border(Border::bottom(1.).with_border_fill(appearance.theme().outline()))
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
    worktree_branch: Option<String>,
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
    #[serde(default)]
    integration: Option<WorktreeIntegration>,
    id: String,
    #[serde(default)]
    description: String,
    state: String,
    revision: u32,
    version: u64,
    assignee: String,
}

#[derive(Deserialize)]
struct WorktreeIntegration { commit: Option<String> }
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

#[derive(Default, Deserialize)]
struct Snapshot {
    #[serde(default)]
    worktree_mode: Option<bool>,
    #[serde(default)]
    candidates: Vec<WorktreeCandidate>,
    #[serde(default)]
    roles: Vec<WorktreeRole>,
    #[serde(default)]
    worktrees: Vec<WorktreeCheckout>,
    #[serde(default)]
    coordinator_online: bool,
    project: String,
    #[serde(default)]
    worktree_joined: bool,
    #[serde(default)]
    worktree_available: bool,
    #[serde(default)]
    worktree_root: String,
    #[serde(default)]
    worktree_branch: Option<String>,
    agents: Vec<PanelAgent>,
    #[serde(default)]
    participant_names: HashMap<String, String>,
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

#[derive(Clone, Deserialize)]
struct WorktreeCandidate {
    agent: Agent,
    run: String,
    root: String,
    branch: Option<String>,
}
#[derive(Deserialize)]
struct WorktreeRole {
    agent: String,
    role: String,
    root: String,
    run: Option<String>,
}
#[derive(Deserialize)]
struct WorktreeCheckout {
    root: String,
    branch: Option<String>,
}

impl Snapshot {
    fn participant_label(&self, id: &str) -> String {
        if id.is_empty() {
            return "Unassigned".into();
        }
        if id.starts_with("warp:") {
            return "You".into();
        }
        if let Some(row) = self.agents.iter().find(|row| row.agent.id == id) {
            return row.agent.name.clone();
        }
        if let Some(name) = self.participant_names.get(id) {
            return name.clone();
        }
        if uuid::Uuid::parse_str(id).is_ok() {
            "Unavailable agent".into()
        } else {
            id.into()
        }
    }
}

pub(crate) struct CollaborationPanel {
    worktree_mode: bool,
    mode_buttons: [MouseStateHandle; 2],
    coordinator_dropdown: ViewHandle<Dropdown<Action>>,
    expanded_worker: Option<String>,
    team_buttons: [MouseStateHandle; 2],
    checkout_buttons: HashMap<String, MouseStateHandle>,
    fixtures: Vec<Fixture>,
    selected: usize,
    next: MouseStateHandle,
    scroll: ClippedScrollStateHandle,
    usage_scroll: ClippedScrollStateHandle,
    usage_buttons: [MouseStateHandle; 2],
    usage_rows: std::cell::RefCell<HashMap<String, MouseStateHandle>>,
    preview: bool,
    visible: bool,
    snapshot: Option<Snapshot>,
    query: PanelQuery,
    events: Vec<Event>,
    context: Option<(String, Option<String>)>,
    in_flight: bool,
    mode_pending: bool,
    status: String,
    task_buttons: HashMap<String, MouseStateHandle>,
    page_buttons: [MouseStateHandle; 8],
    generation: u64,
    connected: bool,
    form: Option<controls::Form>,
    control_buttons: [MouseStateHandle; 10],
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

#[derive(Clone, Debug)]
pub(crate) enum Action {
    UsageSettings,
    RefreshUsage,
    Mode(bool),
    SelectFormField { request: String, index: usize, value: String },
    ChooseCoordinator { project: String, agent: String, run: String },
    SelectParticipant(String),
    ExpandWorker(String),
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
        ctx.observe(&crate::agent_usage::AgentUsage::handle(ctx), |_, _, ctx| ctx.notify());
        let preview = std::env::var_os("WARP_COLLABORATION_PREVIEW").is_some();
        if !preview {
            Self::schedule(ctx);
        }
        Self {
            worktree_mode: false,
            mode_buttons: Default::default(),
            coordinator_dropdown: ctx.add_typed_action_view(Dropdown::new),
            expanded_worker: None,
            team_buttons: Default::default(),
            checkout_buttons: Default::default(),
            fixtures: serde_json::from_str(include_str!(
                "../../../specs/agent-communication-v2/panel-fixtures.json"
            ))
            .expect("Validated collaboration fixtures"),
            selected: 0,
            next: Default::default(),
            scroll: Default::default(),
            usage_scroll: Default::default(),
            usage_buttons: Default::default(),
            usage_rows: Default::default(),
            preview,
            visible: false,
            snapshot: None,
            query: Default::default(),
            events: Vec::new(),
            context: None,
            in_flight: false,
            mode_pending: false,
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
        crate::remote_server::selected_session::selected_ssh(ctx, ctx.window_id())
    }

    fn communication_enabled(&self, app: &warpui::AppContext) -> bool {
        let model = super::AgentCommunication::as_ref(app);
        #[cfg(windows)]
        if let Some(SshConnection::Wsl { distribution, user }) = &self.remote_connection {
            return model.wsl.preferences.enabled && model.wsl.targets.iter().any(|target|
                &target.distribution == distribution && &target.user == user);
        }
        model.preferences.enabled
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
                session.is_wsl() || session.is_legacy_ssh_session()
                    || session.ssh_arguments().is_some()
                    || matches!(
                        session.session_type(),
                        crate::terminal::model::session::SessionType::WarpifiedRemote { .. }
                    )
            })
        {
            return None;
        }
        // Shell integration may change Windows path spelling without changing the checkout.
        let root = warp_agent_bus::project_root(&active.path_if_local(ctx.window_id())?).ok()?;
        let terminal = active
            .terminal_view_id(ctx.window_id())
            .and_then(|id| super::terminal_for_view(id));
        Some((root, terminal))
    }

    fn set_mode(&mut self, worktree: bool, ctx: &mut ViewContext<Self>) {
        if self.worktree_mode == worktree || self.mode_pending || !self.connected || self.current_context(ctx) != self.context { return; }
        let Some(snapshot) = &self.snapshot else { return; };
        if worktree && !snapshot.worktree_available { return; }
        self.coordinator_dropdown.update(ctx, |dropdown, ctx| dropdown.close(ctx));
        let project = snapshot.project.clone();
        let root = snapshot.worktree_root.clone();
        let remote = self.remote.is_some();
        let client = self.remote_client.clone();
        let broker = super::BROKER.get().cloned();
        let context = self.context.clone();
        self.generation += 1;
        let generation = self.generation;
        self.mode_pending = true;
        ctx.spawn(async move {
            let operation = warp_agent_bus::ControllerOperation::WorktreeMode {
                worktree, request_id:uuid::Uuid::new_v4().to_string(),
            };
            if remote {
                let mut client = client.lock().await;
                let client = client.as_mut().ok_or_else(|| anyhow::anyhow!("ssh_connection_lost"))?;
                Self::remote_command(client, &TaskCommand::Scoped {
                    scope:project, command:Box::new(TaskCommand::Controller(operation)),
                }, generation).await
            } else {
                broker.ok_or_else(|| anyhow::anyhow!("Broker unavailable"))?.control(&root, &operation)
            }
        }, move |panel, result, ctx| {
            if panel.context != context || panel.generation != generation { return; }
            panel.mode_pending = false;
            match result {
                Ok(_) => {
                    panel.worktree_mode = worktree;
                    panel.query = PanelQuery { worktree, ..Default::default() };
                    panel.snapshot = None;
                    panel.events.clear();
                    panel.show_spaces = false;
                    panel.show_messages = false;
                    panel.expanded_worker = None;
                    panel.scroll = Default::default();
                }
                Err(_) => {
                    panel.connected = false;
                    panel.status = "Could not change communication mode. Reconnect and try again.".into();
                }
            }
            panel.refresh(ctx);
            ctx.notify();
        });
        ctx.notify();
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
        let enabled = self.communication_enabled(ctx);
        let context = enabled.then(|| self.current_context(ctx)).flatten();
        if context != self.context {
            self.coordinator_dropdown.update(ctx, |dropdown, ctx| dropdown.close(ctx));
            self.generation += 1;
            self.mode_pending = false;
            if self.context.as_ref().map(|(root, _)| root) != context.as_ref().map(|(root, _)| root) {
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
            }
            self.context = context.clone();
            ctx.notify();
        }
        let Some((directory, terminal)) = context.clone() else {
            let status = if enabled {
                "Select a terminal project. For remote collaboration, use ssh user@host, then cd into the project. SSH setup explains installation and shell integration."
            } else if self.remote_connection.as_ref().is_some_and(SshConnection::is_wsl) {
                "Enable WSL communication and select this distribution in Settings > Features."
            } else {
                "Communication is off. Enable it in Settings > Features > Agent communication."
            };
            if self.status != status {
                self.status = status.into();
                ctx.notify();
            }
            return;
        };
        self.query.worktree = self.worktree_mode;
        if self.in_flight || self.mode_pending {
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
                        Some(connection) => connection.connect(&profile).await,
                        None => HostClient::connect(&profile).await,
                    }.map_err(anyhow::Error::new)?);
                }
                query.project.clear();
                if query.worktree && !client.as_ref().unwrap().capabilities().iter().any(|capability| capability == "worktree_orchestration") {
                    return Err(warp_agent_bus::DomainError { code:"feature_unavailable".into(), message:"Update the remote Companion for Worktree mode".into(), retryable:false, version:None }.into());
                }
                query.terminal = None;
                query.spaces = false;
                Self::remote_command(client.as_mut().unwrap(), &TaskCommand::Panel(query), generation).await?
            } else {
                query.project = warp_agent_bus::project_root(std::path::Path::new(&directory))?;
                broker.unwrap().operator_panel(&query)?
            };
            let mut value = value;
            if let Some(scope) = value.get("collaboration_scope").cloned() {
                value["project"] = scope;
            }
            serde_json::from_value::<Snapshot>(value).map_err(anyhow::Error::from)
        }, move |panel, result: anyhow::Result<Snapshot>, ctx| {
            panel.in_flight = false;
            if !panel.visible || panel.generation != generation || panel.context != context || panel.current_context(ctx) != context
                || !panel.communication_enabled(ctx) {
                return;
            }
            match result {
                Ok(mut snapshot) => {
                    if let Some(mode) = snapshot.worktree_mode {
                        if panel.worktree_mode != mode {
                            panel.worktree_mode = mode;
                            panel.query = PanelQuery { worktree:mode, ..Default::default() };
                            panel.events.clear();
                            panel.expanded_worker = None;
                        }
                    }
                    if panel.query.scope.as_deref() != Some(snapshot.project.as_str()) {
                        panel.events.clear();
                        panel.query = Default::default();
                        panel.query.worktree = panel.worktree_mode;
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
                    panel.checkout_buttons.retain(|root, _| snapshot.worktrees.iter().any(|checkout| &checkout.root == root));
                    for checkout in &snapshot.worktrees { panel.checkout_buttons.entry(checkout.root.clone()).or_default(); }
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
                        _ => "Connected.",
                    }.into() };
                    panel.sync_coordinator_dropdown(&snapshot, ctx);
                    panel.snapshot = Some(snapshot);
                }
                Err(error) => {
                    #[cfg(debug_assertions)]
                    if panel.worktree_mode {
                        if let Some(directory) = std::env::var_os(warpui::integration::ARTIFACTS_DIR_ENV_VAR) {
                            let category = error.downcast_ref::<warp_agent_bus::DomainError>()
                                .and_then(|error| ["Project path is unavailable", "Project must be a directory", "A registered Git worktree is required", "Worktree registry unavailable", "Git checkout unavailable", "Git repository unavailable", "Invalid Git metadata"].iter().position(|message| *message == error.message)).map(|index| index + 1).unwrap_or(99);
                            let _ = std::fs::write(std::path::Path::new(&directory).join("checkpoint-worktree-error.txt"), format!("Native Worktree read error: category={category}"));
                        }
                    }
                    panel.connected = false;
                    let code = error.downcast_ref::<warp_agent_bus::DomainError>()
                        .map(|error| error.code.as_str()).unwrap_or("coordinator_unavailable");
                    if panel.remote.is_some() {
                        panel.remote_failed = error.downcast_ref::<warp_agent_bus::DomainError>().is_none();
                        panel.status = if panel.remote_failed {
                            if panel.remote_connection.as_ref().is_some_and(SshConnection::is_wsl) {
                                "WSL workspace unavailable. Check the selected Linux account and its dedicated WSL Companion, then reconnect.".into()
                            } else { match error.downcast_ref::<warp_agent_bus::ssh_remote::ConnectionError>() {
                                Some(warp_agent_bus::ssh_remote::ConnectionError::CompanionUnavailable) => "Warpai Companion is missing or cannot run. Install the Companion package in the selected environment, then reconnect.".into(),
                                Some(warp_agent_bus::ssh_remote::ConnectionError::SshAuthenticationUnavailable) => "The companion connection requires system OpenSSH authentication. Unlock your SSH key agent, then reconnect. Warpai does not store SSH passwords.".into(),
                                Some(warp_agent_bus::ssh_remote::ConnectionError::IncompatibleVersion) => "Warpai Companion is incompatible. Update Companion in the selected environment, then reconnect.".into(),
                                _ => warpui::localization::format_text("SSH project unavailable ({code}). Last received state is stale. Check the terminal connection, then reconnect.", &[("code", format!("{code}").as_str())]),
                            } }
                        } else {
                            warpui::localization::format_text("Could not update the remote view ({code}). Last state is stale; change the view or refresh.", &[("code", format!("{code}").as_str())])
                        };
                    } else {
                        panel.status = warpui::localization::format_text("Could not update collaboration ({code}). Last received state may be stale; refresh or restart Warpai.", &[("code", format!("{code}").as_str())]);
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
                title: if self.remote_connection.as_ref().is_some_and(SshConnection::is_wsl) { "WSL project" } else { "SSH project" }.into(),
                rows: vec![
                    match &self.remote_connection {
                        #[cfg(windows)]
                        Some(SshConnection::Wsl { distribution, user }) => format!("{distribution} · {user} · {}", profile.remote_root),
                        _ => format!("{} · {}", profile.target, profile.remote_root),
                    },
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
        if self.remote.is_none() && !snapshot.worktree_available {
            fixture.sections.push(Section {
                title: "Project".into(),
                rows: vec![format!(
                    "{} · {}",
                    if self.remote.is_some() {
                        "Remote project"
                    } else if cfg!(target_os = "windows") {
                        "This Windows PC"
                    } else {
                        "This Mac"
                    },
                    if snapshot.worktree_root.is_empty() { &snapshot.project } else { &snapshot.worktree_root }
                )],
            });
        }
        if self.query.history {
            fixture.state = "history and capacity".into();
            if let Some(history) = &snapshot.history {
                fixture.sections.push(Section {
                    title: "History".into(),
                    rows: history.records.iter().filter_map(|record| {
                        let data = &record["data"];
                        match record["type"].as_str()? {
                            "task" => Some(format!("{} · {}", data["description"].as_str().unwrap_or("Task"),
                                data["state"].as_str().unwrap_or("unknown"))),
                            "message" => Some(format!("{} → {}\n{}", snapshot.participant_label(data["from"].as_str().unwrap_or("")),
                                snapshot.participant_label(data["to"].as_str().unwrap_or("")), data["body"].as_str().unwrap_or(""))),
                            _ => None,
                        }
                    }).collect(),
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
                title: if self.query.selected_thread.is_some() { "Conversation".into() } else { "Messages".into() },
                rows: if snapshot.messages.is_empty() { vec!["No messages on this page. Search another literal phrase or return to the first page. Original messages remain immutable; corrections are new replies.".into()] }
                    else { snapshot.messages.iter().flat_map(|message| [
                        format!("{} → {} · {} · {}", message.from, message.to, message.subject.as_deref().unwrap_or("Message"), if message.acknowledged { "read" } else { "unread" }),
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
                    {
                        let __warpai_locale_argument_0 = &(workspace.space_id);
                        let __warpai_locale_argument_1 = &(workspace.id);
                        let __warpai_locale_argument_2 = &(workspace.root);
                        warpui::localization::format_text(
                            "Space {} · workspace {} · {}",
                            &[
                                ("0", format!("{__warpai_locale_argument_0}").as_str()),
                                ("1", format!("{__warpai_locale_argument_1}").as_str()),
                                ("2", format!("{__warpai_locale_argument_2}").as_str()),
                            ],
                        )
                    },
                    {
                        let __warpai_locale_argument_0 =
                            &(workspace.repository_id.as_deref().unwrap_or("not linked"));
                        let __warpai_locale_argument_1 = &(workspace.model);
                        let __warpai_locale_argument_2 = &(workspace.branch.as_deref().unwrap_or("unspecified"));
                        let __warpai_locale_argument_3 =
                            &(workspace.base_commit.as_deref().unwrap_or("unspecified"));
                        warpui::localization::format_text(
                            "Repository {} · model {} · branch {} · base {}",
                            &[
                                ("0", format!("{__warpai_locale_argument_0}").as_str()),
                                ("1", format!("{__warpai_locale_argument_1}").as_str()),
                                ("2", format!("{__warpai_locale_argument_2}").as_str()),
                                ("3", format!("{__warpai_locale_argument_3}").as_str()),
                            ],
                        )
                    },
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
                        "Task · {}",
                        task.state
                    ),
                    rows: {
                        let rows = vec![
                        format!(
                            "Issuer: {} · assignee: {} · reviewer: {}",
                            snapshot.participant_label(&task.issuer), snapshot.participant_label(&task.assignee), snapshot.participant_label(&task.reviewer)
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
                                "{} · owner {} · outcome {}",
                                attempt.certainty,
                                snapshot.participant_label(&attempt.owner),
                                attempt.outcome.as_deref().unwrap_or("unknown")
                            )
                        })
                        .chain(
                            task.dependencies
                                .iter()
                                .map(|id| warpui::localization::format_text("Prerequisite {id}", &[("id", format!("{id}").as_str())])),
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
                            {
                                let __warpai_locale_argument_0 = &(deadline(task.start_deadline));
                                let __warpai_locale_argument_1 = &(deadline(task.execution_deadline));
                                let __warpai_locale_argument_2 = &(deadline(task.review_deadline));
                                warpui::localization::format_text(
                                    "Start by {} · execution deadline {} · review deadline {}",
                                    &[
                                        ("0", format!("{__warpai_locale_argument_0}").as_str()),
                                        ("1", format!("{__warpai_locale_argument_1}").as_str()),
                                        ("2", format!("{__warpai_locale_argument_2}").as_str()),
                                    ],
                                )
                            },
                            {
                                let __warpai_locale_argument_0 = &(task
                                    .execution_timeout_seconds
                                    .map(|seconds| format!("{seconds}s"))
                                    .unwrap_or_else(|| "not set".into()));
                                let __warpai_locale_argument_1 = &(task
                                    .review_timeout_seconds
                                    .map(|seconds| format!("{seconds}s"))
                                    .unwrap_or_else(|| "not set".into()));
                                let __warpai_locale_argument_2 = &(task.review_overdue);
                                warpui::localization::format_text(
                                    "Execution timeout {} · review timeout {} · review overdue {}",
                                    &[
                                        ("0", format!("{__warpai_locale_argument_0}").as_str()),
                                        ("1", format!("{__warpai_locale_argument_1}").as_str()),
                                        ("2", format!("{__warpai_locale_argument_2}").as_str()),
                                    ],
                                )
                            },
                            {
                                let __warpai_locale_argument_0 = &(if task.eligible.is_empty() {
                                    "assigned task".into()
                                } else {
                                    task.eligible.join(", ")
                                });
                                warpui::localization::format_text(
                                    "Eligible pool participants: {}",
                                    &[("0", format!("{__warpai_locale_argument_0}").as_str())],
                                )
                            },
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
                                    evidence.commit.as_ref().map(|value| warpui::localization::format_text("commit {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.hash.as_ref().map(|value| warpui::localization::format_text("hash {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.repository.as_ref().map(|value| warpui::localization::format_text("repository {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.branch.as_ref().map(|value| warpui::localization::format_text("branch {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.base.as_ref().map(|value| warpui::localization::format_text("base {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.head.as_ref().map(|value| warpui::localization::format_text("head {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.command.as_ref().map(|value| warpui::localization::format_text("command {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.outcome.as_ref().map(|value| warpui::localization::format_text("outcome {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.exit_code.map(|value| warpui::localization::format_text("exit {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.summary.as_ref().map(|value| warpui::localization::format_text("summary {value}", &[("value", format!("{value}").as_str())])),
                                    evidence.device.as_ref().map(|value| warpui::localization::format_text("device {value}; remote metadata, content not fetched", &[("value", format!("{value}").as_str())])),
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
                    .filter(|row| row.online)
                    .map(|row| {
                        let activity = row
                            .activity
                            .map(|activity| match activity {
                                warp_agent_bus::readiness::Activity::Starting => warpui::localization::text("starting"),
                                warp_agent_bus::readiness::Activity::Idle => warpui::localization::text("idle"),
                                warp_agent_bus::readiness::Activity::Working => warpui::localization::text("working"),
                                warp_agent_bus::readiness::Activity::WaitingApproval => {
                                    warpui::localization::text("waiting for approval")
                                }
                                warp_agent_bus::readiness::Activity::WaitingInput => {
                                    warpui::localization::text("waiting for input")
                                }
                                warp_agent_bus::readiness::Activity::Cancelled => warpui::localization::text("cancelled"),
                                warp_agent_bus::readiness::Activity::Error => warpui::localization::text("error"),
                            })
                            .unwrap_or(warpui::localization::text("unknown activity"));
                        format!("{} · {}{}", row.agent.name,
                            if self.remote.is_some() && !self.connected { warpui::localization::text("disconnected") } else { activity },
                            if row.blocked { warpui::localization::text(" · approval required") } else if row.paused { warpui::localization::text(" · paused") } else { "" })
                    })
                    .collect(),
            });
            if snapshot.agents.is_empty() {
                fixture.sections.push(Section { title: "No participating agents".into(), rows: vec![if self.remote.is_some() { "Start an Agent in this remote project." } else { "Start an Agent in this project." }.into()] });
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
        match action {
            Action::UsageSettings => {
                ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::ShowSettingsPageWithSearch {
                    search_query: "data usage".into(), section: Some(crate::settings_view::SettingsSection::Features),
                });
                return;
            }
            Action::RefreshUsage => {
                crate::agent_usage::AgentUsage::handle(ctx).update(ctx, |model, ctx| model.refresh(ctx));
                return;
            }
            _ => (),
        }
        if !self.preview {
            match action {
                Action::SelectFormField { request, index, value } => {
                    self.select_form_field(request, *index, value, ctx);
                    return;
                }
                Action::Mode(worktree) => {
                    if self.form.is_some() { return; }
                    self.set_mode(*worktree, ctx);
                    return;
                }
                Action::SelectParticipant(id) => {
                    if let Some(form) = self.form.as_mut().filter(|form| !form.submitting && form.submitted_fields.is_none()) {
                        if form.candidates.iter().any(|candidate| candidate.agent.id == *id) {
                            form.selected_candidate = Some(id.clone());
                        }
                    }
                    ctx.notify();
                    return;
                }
                Action::ChooseCoordinator { project, agent, run } => {
                    if self.form.is_some() || !self.worktree_mode || !self.connected
                        || self.snapshot.as_ref().is_none_or(|snapshot| snapshot.project != *project
                            || !snapshot.candidates.iter().any(|candidate| candidate.agent.id == *agent && candidate.run == *run)) {
                        if let Some(snapshot) = &self.snapshot { self.sync_coordinator_dropdown(snapshot, ctx); }
                        return;
                    }
                    self.open_control(controls::Kind::SelectCoordinator, ctx);
                    if let Some(form) = &mut self.form { form.selected_candidate = Some(agent.clone()); }
                    self.confirm_control(ctx);
                    return;
                }
                Action::ExpandWorker(root) => {
                    self.expanded_worker = if self.expanded_worker.as_ref() == Some(root) { None } else { Some(root.clone()) };
                    ctx.notify();
                    return;
                }
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
                        && self.communication_enabled(ctx)
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
                    #[cfg(windows)]
                    if self.remote_connection.as_ref().is_some_and(SshConnection::is_wsl) {
                        ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::ShowSettingsPageWithSearch {
                            search_query:"wsl".into(), section:Some(crate::settings_view::SettingsSection::Features),
                        });
                        return;
                    }
                    let active = crate::workspace::ActiveSession::as_ref(ctx);
                    let anchor = self
                        .remote
                        .as_ref()
                        .and_then(|_| active.session(ctx.window_id()))
                        .and_then(
                            |session| match session.host_info().os_category.as_deref()? {
                                "Windows" => Some("windows"),
                                "MacOS" | "Darwin" => Some("macos"),
                                "Linux" => Some("linux"),
                                _ => None,
                            },
                        )
                        .unwrap_or("install-on-the-remote-machine");
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
                        if panel.current_context(ctx) != context || panel.context != context || panel.query.selected_task.as_ref() != Some(&task_id) || !panel.communication_enabled(ctx) { return; }
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
                Action::NextFixture | Action::PreviousFixture | Action::UsageSettings | Action::RefreshUsage => {}
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
                {
                    let __warpai_locale_argument_0 = &(self.status);
                    warpui::localization::format_text(
                        "Agent collaboration. {}",
                        &[("0", format!("{__warpai_locale_argument_0}").as_str())],
                    )
                },
                warpui::localization::text("Enter refreshes. Page Up and Page Down scroll. Escape returns to terminal. Tasks and pagination use native buttons."),
                WarpA11yRole::ScrollareaRole,
            ));
        }
        let fixture = &self.fixtures[self.selected];
        Some(AccessibilityContent::new(
            {
                let __warpai_locale_argument_0 = &(fixture.state);
                let __warpai_locale_argument_1 = &(fixture.guidance);
                warpui::localization::format_text(
                    "Agent collaboration, sample data. {}. {}",
                    &[
                        ("0", format!("{__warpai_locale_argument_0}").as_str()),
                        ("1", format!("{__warpai_locale_argument_1}").as_str()),
                    ],
                )
            },
            warpui::localization::text("Left and Right or Enter change preview state. Page Up and Page Down scroll. Escape returns to terminal."),
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
        header.add_child(heading(appearance, warpui::localization::text("Agents Collaboration Mode")));
        if !self.preview {
            let mut modes = Flex::row().with_spacing(GAP_ROW);
            for (index, (label, worktree)) in [("Project", false), ("Worktree", true)].into_iter().enumerate() {
                let button = builder.button(if self.worktree_mode == worktree { ButtonVariant::Accent } else { ButtonVariant::Secondary }, self.mode_buttons[index].clone())
                    .with_centered_text_label(label.into());
                let button = if self.form.is_some() || self.mode_pending || !self.connected
                    || (worktree && !self.snapshot.as_ref().is_some_and(|snapshot| snapshot.worktree_available)) {
                    button.disabled()
                } else { button };
                modes.add_child(Expanded::new(1., button.build().on_click(move |ctx, _, _| ctx.dispatch_typed_action(Action::Mode(worktree))).finish()).finish());
            }
            header.add_child(modes.finish());
        }
        if self.preview {
            header.add_child(note(appearance, {
                let __warpai_locale_argument_0 = &(fixture.state);
                warpui::localization::format_text(
                    "Preview · {}",
                    &[("0", format!("{__warpai_locale_argument_0}").as_str())],
                )
            }));
        } else if !self.connected {
            header.add_child(note(appearance, self.status.clone()));
        }
        let mut connection_controls = Wrap::row()
            .with_spacing(GAP_SECTION)
            .with_run_spacing(GAP_TIGHT);
        if !self.preview && self.remote.is_some() {
            for (index, label, action) in [
                (0, if self.remote_connection.as_ref().is_some_and(SshConnection::is_wsl) { "WSL setup" } else { "SSH setup" }, Action::RemoteSetup),
                (1, "Reconnect", Action::ReconnectSsh),
            ] {
                let variant = if index == 0 {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Text
                };
                let button = builder
                    .button(variant, self.remote_buttons[index].clone())
                    .with_text_label(warpui::localization::text(label).into());
                let button = if (index != 1 && self.form.is_some())
                    || !self.communication_enabled(app)
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
        if self.preview {
            connection_controls.add_child(builder.button(ButtonVariant::Secondary, self.next.clone())
                .with_text_label(warpui::localization::text("Next preview").into()).build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::NextFixture)).finish());
        }
        if self.preview || self.remote.is_some() {
            header.add_child(connection_controls.finish());
        }
        let mut body = Flex::column().with_spacing(GAP_SECTION);
        let render_section = |section: &Section| {
            let mut column = Flex::column().with_spacing(GAP_TIGHT);
            column.add_child(heading(appearance, warpui::localization::text(&section.title).to_owned()));
            for (index, row) in section.rows.iter().enumerate() {
                let record = detail(appearance, row.clone());
                column.add_child(if section.title == "History" {
                    self::section(appearance, record)
                } else if section.title == "Agents" {
                    let icon = self.snapshot.as_ref().and_then(|s| s.agents.iter().filter(|a| a.online).nth(index))
                        .map(|a| crate::agent_usage::agent_icon(&a.agent.program))
                        .unwrap_or(crate::ui_components::icons::Icon::Terminal);
                    Flex::row().with_spacing(GAP_ROW).with_child(warpui::elements::ConstrainedBox::new(icon.to_warpui_icon(theme.foreground()).finish()).with_width(16.).with_height(16.).finish())
                        .with_child(Shrinkable::new(1., record).finish()).finish()
                } else { record });
            }
            self::section(appearance, column.finish())
        };
        if self.worktree_mode && !self.preview
            && self.form.as_ref().is_none_or(|form| form.kind == controls::Kind::SelectCoordinator)
            && !self.query.history && !self.show_messages {
            body.add_child(self.render_worktree_team(app));
        }
        let leading_status = !self.worktree_mode && !self.preview && self.remote.is_some() && self.form.is_none();
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
                if self.query.selected_task.is_some()
                    || self.show_spaces
                    || self.show_messages
                    || self.query.history
                {
                    if !hide_navigation {
                        navigation.add_child(
                            builder
                                .button(ButtonVariant::Text, self.page_buttons[0].clone())
                                .with_text_label(warpui::localization::text("Back").into())
                                .build()
                                .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Back))
                                .finish(),
                        );
                    }
                } else {
                    body.add_child(heading(appearance, warpui::localization::text("Tasks")));
                    if snapshot.tasks.is_empty() {
                        body.add_child(note(appearance, warpui::localization::text("No tasks on this page.")));
                    }
                    for task in &snapshot.tasks {
                        let id = task.id.clone();
                        body.add_child(
                            builder
                                .button(ButtonVariant::Text, self.task_buttons[&task.id].clone())
                                .with_custom_label(
                                    builder
                                        .span(if task.description.is_empty() {
                                            "Open task".to_owned()
                                        } else {
                                            task.description.clone()
                                        })
                                        .with_soft_wrap()
                                        .build()
                                        .finish(),
                                )
                                .build()
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(Action::SelectTask(id.clone()))
                                })
                                .finish(),
                        );
                        body.add_child(
                            builder
                                .span(format!(
                                    "{} · {}",
                                    task.state,
                                    snapshot.participant_label(&task.assignee)
                                ))
                                .with_soft_wrap()
                                .build()
                                .finish(),
                        );
                        if self.worktree_mode && task.state == "accepted" {
                            body.add_child(note(appearance, match &task.integration {
                                Some(integration) => match &integration.commit {
                                    Some(commit) => {
                                        let __warpai_locale_argument_0 = &(commit.chars().take(8).collect::<String>());
                                        warpui::localization::format_text(
                                            "Integration commit recorded · {}",
                                            &[("0", format!("{__warpai_locale_argument_0}").as_str())],
                                        )
                                    },
                                    None => "Selected for integration".into(),
                                },
                                None => "Reviewed · awaiting Coordinator integration choice".into(),
                            }));
                        }
                    }
                    let mut buttons = Vec::new();
                    for (label, action, index) in [
                        (
                            "First task page",
                            self.query.task_after.map(|_| Action::FirstTasks),
                            0,
                        ),
                        (
                            "Next task page",
                            snapshot
                                .task_cursor
                                .filter(|_| snapshot.tasks.len() == 50)
                                .map(|_| Action::NextTasks),
                            1,
                        ),
                        (
                            "First agent page",
                            self.query.agent_after.as_ref().map(|_| Action::FirstAgents),
                            2,
                        ),
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
                                    .with_text_label(warpui::localization::text(label).into())
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
            if !self.preview && self.query.history && section.title != "History" {
                continue;
            }
            if self.worktree_mode && !self.preview && !self.query.history && !self.show_messages &&
                (matches!(section.title.as_str(), "Worktree collaboration" | "Agents" | "No participating agents")
                    || section.title.starts_with("File reservations") || section.title.starts_with("Activity")) {
                continue;
            }
            if !self.preview
                && !self.query.history
                && !self.show_spaces
                && (section.title.starts_with("File reservations")
                    || section.title.starts_with("Activity")
                    || section.title == "Scheduling and deadlines")
            {
                continue;
            }
            if section.rows.is_empty() {
                continue;
            }
            if leading_status
                && matches!(
                    section.title.as_str(),
                    "SSH project" | "Worktree collaboration" | "Agents" | "No participating agents"
                )
            {
                continue;
            }
            body.add_child(render_section(section));
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
                (1, "First page", Some(Action::FirstHistory)),
                (
                    2,
                    "Next page",
                    self.snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.history.as_ref())
                        .and_then(|history| history.cursor)
                        .map(|_| Action::NextHistory),
                ),
            ] {
                let button = builder
                    .button(ButtonVariant::Text, self.history_buttons[index].clone())
                    .with_text_label(warpui::localization::text(label).into());
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
                                .with_text_label(warpui::localization::text(label).into())
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
                                .with_text_label(warpui::localization::text(label).into())
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
        if !self.preview && self.show_spaces {
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
                                .with_text_label(warpui::localization::text(label).into())
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
                    .with_child(self.render_usage(app))
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
impl CollaborationPanel {
    fn worktree_layout_checkpoint(&mut self, state: &str, ctx: &mut ViewContext<Self>) {
        self.coordinator_dropdown.update(ctx, |dropdown, ctx| dropdown.close(ctx));
        self.preview = false;
        self.worktree_mode = true;
        self.in_flight = true;
        self.context = self.current_context(ctx);
        self.connected = true;
        self.form = None;
        self.show_spaces = false;
        self.show_messages = false;
        self.query = PanelQuery { worktree:true, ..Default::default() };
        self.events.clear();
        self.scroll = Default::default();
        let root = "/sample/repository/integration";
        let mut snapshot = Snapshot { project:"space:sample-team".into(), admission:"private".into(), worktree_root:root.into(),
            worktree_available:true, worktree_joined:true, coordinator_online:state != "empty", ..Default::default() };
        if state != "empty" {
            for index in 0..16 {
                let id = format!("sample-{index}");
                let checkout = if index == 0 { root.into() } else { format!("/sample/repository/worktrees/long-directory-name-with-many-components/worker-{index}") };
                let name = if index == 0 { "coordinator".into() } else { format!("worker-{index}") };
                let branch = if index == 0 { "main".into() } else { format!("feature/long-branch-name-to-exercise-narrow-panel-wrapping-and-overflow-{index}") };
                snapshot.agents.push(serde_json::from_value(serde_json::json!({"agent":{"id":id,"terminal":format!("sample-terminal-{index}"),"name":name,"program":"fixture","project":"space:sample-team"},
                    "run":"sample-run","online":true,"activity":"idle","draft":"empty","blocked":false,"paused":false,"ready":true,"readiness_source":"native"})).unwrap());
                snapshot.candidates.push(WorktreeCandidate { agent:snapshot.agents.last().unwrap().agent.clone(), run:"sample-run".into(), root:checkout.clone(), branch:Some(branch.clone()) });
                self.agent_task_buttons.entry(id.clone()).or_default();
                snapshot.roles.push(WorktreeRole { agent:id, role:if index == 0 { "coordinator" } else { "worker" }.into(), root:checkout.clone(), run:Some("sample-run".into()) });
                self.checkout_buttons.entry(checkout.clone()).or_default();
                snapshot.worktrees.push(WorktreeCheckout { root:checkout, branch:Some(branch) });
            }
            self.expanded_worker = Some(snapshot.worktrees[1].root.clone());
        } else { self.expanded_worker = None; }
        self.sync_coordinator_dropdown(&snapshot, ctx);
        self.snapshot = Some(snapshot);
        if state == "selector" { self.coordinator_dropdown.update(ctx, |dropdown, ctx| dropdown.toggle_expanded(ctx)); }
        ctx.notify();
    }
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
    broker.activate_project(terminal, "codex", root, true)?;
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
    let project = transport::call(&broker.endpoint, &warp_agent_bus::transport::Request {
        operation:Operation::AgentRegister { name:String::new() }, ..request.clone()
    })?["agent"]["project"].as_str().unwrap().to_owned();
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
    let task = broker.operator(&project, &Operation::TaskAssign {
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
        &project,
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
                            agent_program: Some("fixture".into()), working_directory: None,
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
    use ::settings::Setting as _;
    use warpui::integration::{Builder, TestStep, ARTIFACTS_DIR_ENV_VAR};
    let language_code = std::env::var("WARPAI_CAPTURE_LANGUAGE").unwrap_or_else(|_| "en".into());
    let capture_language: warpui::localization::Language =
        serde_json::from_value(serde_json::Value::String(language_code.clone()))?;

    std::fs::create_dir_all(&directory)?;
    let directory = directory
        .canonicalize()?
        .join(format!("capture-{}-{language_code}", std::process::id()));
    std::fs::create_dir(&directory)?;
    let worktree_fixture = directory.join("owned-worktree-checkout");
    std::fs::create_dir(&worktree_fixture)?;
    let initialized = std::process::Command::new(warp_agent_bus::installation::git_executable()?)
        .args(["-c", "core.hooksPath=", "init", "--initial-branch=main"])
        .current_dir(&worktree_fixture).output()?;
    anyhow::ensure!(initialized.status.success(), "Owned capture Git fixture initializes");
    std::fs::write(worktree_fixture.join("fixture.txt"), "Owned native Worktree capture\n")?;
    for arguments in [vec!["add", "fixture.txt"], vec!["commit", "-m", "Seed owned Worktree capture"]] {
        let output = std::process::Command::new(warp_agent_bus::installation::git_executable()?)
            .args(["-c", "core.hooksPath=", "-c", "user.name=Capture Fixture", "-c", "user.email=capture@example.invalid", "-c", "commit.gpgsign=false"])
            .args(arguments).current_dir(&worktree_fixture).output()?;
        anyhow::ensure!(output.status.success(), "Owned capture Git fixture has a base commit");
    }
    let worktree_root = warp_agent_bus::project_root(&worktree_fixture)?;
    let expected_worktree_root = worktree_root.clone();
    let panic_directory = directory.clone();
    let previous_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(location) = info.location() {
            let file = location.file().replace('\\', "/");
            if let Some(source) = ["app/src/", "crates/"]
                .iter()
                .find_map(|prefix| file.find(prefix).map(|offset| &file[offset..]))
            {
                // Persist the source location only, never the panic's runtime payload.
                let _ = std::fs::write(
                    panic_directory.join("checkpoint-panic.txt"),
                    format!("{source}:{}:{}", location.line(), location.column()),
                );
            }
        }
        previous_panic_hook(info);
    }));
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
            use std::io::Write as _;
            utils.set_env("WARP_INTEGRATION", Some("1"));
            utils.set_env("WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS", Some("1"));
            utils.set_env("WARP_COLLABORATION_PREVIEW", Some("1"));
            utils.set_env(
                "WARP_DATA_PROFILE",
                Some(format!("collaboration-capture-{}-{language_code}", std::process::id())),
            );
            assert!(!warp_core::paths::data_dir().exists(), "Capture profile must be fresh");
            crate::user_data_migration::migrate().expect("Create private capture profile");
            let mut preferences = std::fs::OpenOptions::new().write(true).create_new(true)
                .open(crate::settings::user_preferences_toml_file_path())
                .expect("Create owned capture preferences");
            write!(preferences, "[appearance]\nlanguage = {}\n", serde_json::to_string(&capture_language).unwrap())
                .expect("Write owned capture language");
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
        .with_step(TestStep::new("verify persisted interface language").add_named_assertion(
            "interface language loaded from isolated preferences",
            move |app, _| {
                warpui::async_assert!(app.read(|ctx| {
                    *crate::settings::LanguageSettings::as_ref(ctx).language.value() == capture_language
                        && warpui::localization::language() == capture_language
                }))
            },
        ))
        .with_step(TestStep::new("open owned worktree fixture tab").with_action(move |app, window, _| {
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            workspace.update(app, |workspace, ctx| workspace.add_tab_with_pane_layout(
                crate::pane_group::PanesLayout::SingleTerminal(Box::new(crate::pane_group::NewTerminalOptions {
                    initial_directory: Some(std::path::PathBuf::from(&worktree_root)),
                    hide_homepage: true,
                    ..Default::default()
                })),
                std::sync::Arc::new(std::collections::HashMap::new()), None, ctx,
            ));
        }))
        .with_step(TestStep::new("owned worktree fixture becomes active")
            .set_timeout(std::time::Duration::from_secs(45))
            .add_named_assertion("owned checkout is the native directory", move |app, window| {
                warpui::async_assert!(app.read(|ctx| crate::workspace::ActiveSession::as_ref(ctx)
                    .path_if_local(window).and_then(|path| warp_agent_bus::project_root(path).ok())
                    .as_deref() == Some(expected_worktree_root.as_str())))
            }))
        .with_step(TestStep::new("retain only the owned capture terminal").with_action(|app, window, _| {
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            workspace.update(app, |workspace, ctx| workspace.remove_tab_without_undo(0, ctx));
        }).add_named_assertion("original fixture terminal is unambiguous", |app, window| {
            warpui::async_assert!(app.views_of_type::<crate::terminal::TerminalView>(window)
                .is_some_and(|terminals| terminals.len() == 1))
        }))
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
    for (theme_name, theme) in [("light", ThemeKind::Light), ("dark", ThemeKind::Dark)] {
        for width in [320, 600] {
            for state in ["empty", "team", "selector"] {
                let filename = format!("worktree-mode-{theme_name}-{width}-{state}.png");
                filenames.push(filename.clone());
                let theme = theme.clone();
                driver = driver.with_step(TestStep::new(&format!("Worktree layout {theme_name} {width} {state}"))
                    .with_action(move |app, window, _| {
                        app.update(|ctx| {
                            let colors = Settings::theme_for_theme_kind(&theme, ctx);
                            Appearance::handle(ctx).update(ctx, |appearance, ctx| appearance.set_theme(colors, ctx));
                            ctx.set_zoom_factor(1.0);
                            ResizableData::as_ref(ctx).get_all_handles(window).unwrap().left_panel_width.lock().unwrap().set_size(width as f32);
                        });
                        let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        panel.update(app, |panel, ctx| panel.worktree_layout_checkpoint(state, ctx));
                    }).add_named_assertion("Worktree layout keeps terminal draft", |app, window| {
                        warpui::async_assert!(checkpoint_draft(app, window) == "unsent collaboration draft")
                    }).with_take_screenshot(&filename));
            }
        }
    }
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
                    panel.worktree_mode = false;
                    panel.form = None;
                    panel.in_flight = false;
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
        .with_step(TestStep::new("select Worktree mode for an empty native project")
            .with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| panel.handle_action(&Action::Mode(true), ctx));
            })
            .add_named_assertion("project identity exists before Agent registration", |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(panel.read(app, |panel, _| !panel.mode_pending && panel.worktree_mode
                    && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot.project.starts_with("space:")
                        && snapshot.worktree_joined && snapshot.agents.is_empty() && !snapshot.coordinator_online)))
            }).with_take_screenshot("worktree-empty-project.png"))
        .with_step(TestStep::new("selected mode preserves the loaded project").with_action(|app, window, _| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| {
                assert!(panel.worktree_mode && panel.snapshot.is_some());
                panel.handle_action(&Action::Mode(true), ctx);
                assert!(!panel.mode_pending && panel.snapshot.is_some());
            });
        }))
        .with_step(TestStep::new("select Worktree mode with an existing native IPC Agent").with_action(|app, window, _| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            let root = panel.read(app, |panel, _| panel.snapshot.as_ref().unwrap().worktree_root.clone());
            register_capture_participant("ui-coordinator-terminal", "ui-coordinator", &root, None).unwrap();
            panel.update(app, |panel, ctx| panel.handle_action(&Action::Mode(true), ctx));
        }))
        .with_step(TestStep::new("existing Agent is eligible for Coordinator selection").add_named_assertion("candidate observed", |app, window| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            warpui::async_assert!(panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(|snapshot| snapshot.candidates.iter().any(|candidate| candidate.agent.name == "ui-coordinator"))))
        }))
        .with_step(TestStep::new("open native Coordinator selector").with_action(|app, window, _| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| panel.open_control(controls::Kind::SelectCoordinator, ctx));
        }).with_take_screenshot("worktree-coordinator-selector.png"))
        .with_step(TestStep::new("confirm existing Coordinator without restarting").with_action(|app, window, _| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| {
                let id = panel.form.as_ref().unwrap().candidates.iter().find(|candidate| candidate.agent.name == "ui-coordinator").unwrap().agent.id.clone();
                panel.handle_action(&Action::SelectParticipant(id), ctx);
                panel.confirm_control(ctx);
            });
        }))
        .with_step(TestStep::new("native Coordinator role is active").add_named_assertion("Coordinator owns its original run", |app, window| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            warpui::async_assert!(panel.read(app, |panel, _| panel.form.is_none() && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot.coordinator_online))
                && checkpoint_draft(app, window) == "unsent collaboration draft")
        }))
        .with_step(TestStep::new("open Warp native worktree creation").with_action(|app, window, data| {
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            data.insert("worktree-source-tab", workspace.read(app, |workspace, _| workspace.active_tab_index()));
            data.insert("worktree-tab-count", workspace.read(app, |workspace, _| workspace.tabs.len()));
            workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::OpenNewWorktreeModal, ctx));
            let repo = app.read(|ctx| crate::workspace::ActiveSession::as_ref(ctx)
                .current_directory(window).unwrap().to_owned());
            let modal = app.views_of_type::<crate::tab_configs::NewWorktreeModal>(window).unwrap()[0].clone();
            modal.update(app, |modal, ctx| modal.fill_checkpoint(repo, ctx));
        }))
        .with_step(TestStep::new("native worktree repository and branches are ready")
            .set_timeout(std::time::Duration::from_secs(45))
            .add_named_assertion("native main branch is selected", |app, window| {
                let modal = app.views_of_type::<crate::tab_configs::NewWorktreeModal>(window).unwrap()[0].clone();
                warpui::async_assert!(modal.read(app, |modal, ctx| modal.checkpoint_ready(ctx)))
            }).with_take_screenshot("worktree-create-form.png"))
        .with_step(TestStep::new("submit native worktree modal into new tab").with_action(|app, window, _| {
            let modal = app.views_of_type::<crate::tab_configs::NewWorktreeModal>(window).unwrap()[0].clone();
            modal.update(app, |modal, ctx| modal.handle_action(&crate::tab_configs::new_worktree_modal::NewWorktreeModalAction::Open, ctx));
        }))
        .with_step(TestStep::new("new worktree opens a tab without launching an Agent")
            .set_timeout(std::time::Duration::from_secs(45))
            .add_named_assertion("native tab and persistent Coordinator", |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(|snapshot|
                    snapshot.worktree_branch.as_deref() == Some("ui-worker") && snapshot.coordinator_online
                        && snapshot.roles.len() == 1 && snapshot.candidates.len() == 1)))
            }).with_take_screenshot("worktree-created.png"))
        .with_step(TestStep::new("prepare separate native IPC worker").with_action(|app, window, _| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            let root = panel.read(app, |panel, _| panel.snapshot.as_ref().unwrap().worktrees.iter().find(|checkout| checkout.branch.as_deref() == Some("ui-worker")).unwrap().root.clone());
            register_capture_participant("ui-worker-terminal", "ui-worker", &root, None).unwrap();
        }))
        .with_step(TestStep::new("worker selection observes its actual checkout").add_named_assertion("worker candidate observed", |app, window| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            warpui::async_assert!(panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(|snapshot| snapshot.candidates.iter().any(|candidate| candidate.agent.name == "ui-worker"))))
        }))
        .with_step(TestStep::new("worker automatically joins its native checkout")
            .add_named_assertion("automatic Worktree participant", |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(|snapshot|
                    snapshot.roles.iter().any(|role| role.role == "worker"))))
            }).with_take_screenshot("worktree-worker-selector.png"))
        .with_step(TestStep::new("native Worktree team has explicit ownership").add_named_assertion("two roles and preserved terminal draft", |app, window| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            warpui::async_assert!(panel.read(app, |panel, _| panel.form.is_none() && panel.snapshot.as_ref().is_some_and(|snapshot|
                snapshot.roles.len() == 2 && snapshot.roles.iter().any(|role| role.role == "worker")))
                && checkpoint_draft(app, window) == "unsent collaboration draft")
        }).with_take_screenshot("worktree-bound-team.png"))
        .with_step(TestStep::new("switch Coordinator to the Agent in another worktree").with_action(|app, window, _| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| {
                panel.open_control(controls::Kind::SelectCoordinator, ctx);
                let id = panel.form.as_ref().unwrap().candidates.iter()
                    .find(|candidate| candidate.agent.name == "ui-worker").unwrap().agent.id.clone();
                panel.handle_action(&Action::SelectParticipant(id), ctx);
                panel.confirm_control(ctx);
            });
        }))
        .with_step(TestStep::new("Coordinator switch preserves both processes")
            .add_named_assertion("worker becomes Coordinator", |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(panel.read(app, |panel, _| panel.form.is_none() && panel.snapshot.as_ref().is_some_and(|snapshot|
                    snapshot.coordinator_online && snapshot.roles.iter().any(|role| role.role == "coordinator"
                        && snapshot.participant_label(&role.agent) == "ui-worker"))))
            }).with_take_screenshot("worktree-switched-coordinator.png"))
        .with_step(TestStep::new("return to the original project tab").with_action(|app, window, data| {
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            let index = *data.get::<_, usize>("worktree-source-tab").unwrap();
            workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ActivateTab(index), ctx));
        }))
        .with_step(TestStep::new("Coordinator survives tab and checkout changes")
            .add_named_assertion("project Coordinator persists", |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(|snapshot|
                    snapshot.worktree_branch.as_deref() == Some("main") && snapshot.coordinator_online
                        && snapshot.roles.iter().any(|role| role.role == "coordinator"
                            && snapshot.participant_label(&role.agent) == "ui-worker")))
                    && checkpoint_draft(app, window) == "unsent collaboration draft")
            }).with_take_screenshot("worktree-coordinator-after-tab-switch.png"))
        .with_step(TestStep::new("restore Project mode and finish owned IPC team runs").with_action(|app, window, data| {
            super::BROKER.get().unwrap().end("ui-coordinator-terminal");
            super::BROKER.get().unwrap().end("ui-worker-terminal");
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            let index = *data.get::<_, usize>("worktree-source-tab").unwrap();
            let count = *data.get::<_, usize>("worktree-tab-count").unwrap();
            assert_eq!(workspace.read(app, |workspace, _| workspace.tabs.len()), count + 1);
            workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ActivateTab(index), ctx));
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| panel.handle_action(&Action::Mode(false), ctx));
        }))
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
            "offline task history is retained without an offline participant row",
            |app, window| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                warpui::async_assert!(
                    panel.read(app, |panel, _| panel.snapshot.as_ref().is_some_and(
                        |snapshot| snapshot.tasks.len() == 1
                            && snapshot.agents.iter().all(|agent| agent.online)
                            && snapshot.participant_label(&snapshot.tasks[0].assignee) == "capture-worker"
                    )) && checkpoint_draft(app, window) == "unsent collaboration draft"
                )
            },
        ));
    filenames.push("live-empty.png".into());
    filenames.extend(["worktree-coordinator-selector.png", "worktree-create-form.png", "worktree-created.png", "worktree-worker-selector.png", "worktree-bound-team.png"].map(str::to_owned));
    filenames.extend(["worktree-switched-coordinator.png", "worktree-coordinator-after-tab-switch.png"].map(str::to_owned));
    filenames.push("worktree-empty-project.png".into());
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
            TestStep::new("focus live detail").with_action(|app, window, data| {
                let bounds = app.read(|ctx| ctx.window_bounds(&window).unwrap());
                data.insert("live-detail-window-bounds", bounds);
                app.update(|ctx| {
                    ctx.set_and_cache_window_bounds(window, pathfinder_geometry::rect::RectF::new(
                        bounds.origin(), pathfinder_geometry::vector::vec2f(bounds.size().x(), 600.)));
                    ResizableData::as_ref(ctx).get_all_handles(window).unwrap()
                        .left_panel_width.lock().unwrap().set_size(320.);
                });
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
        )
        .with_step(TestStep::new("restore window after live detail scrolling").with_action(|app, window, data| {
            let bounds = *data.get::<_, pathfinder_geometry::rect::RectF>("live-detail-window-bounds").unwrap();
            app.update(|ctx| ctx.set_and_cache_window_bounds(window, bounds));
        }));
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
                                    .any(|space| space.name == "Native reviewed collaboration" && !space.workspaces.is_empty())))
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
                            .find(|space| space.name == "Native reviewed collaboration")
                            .unwrap()
                            .workspaces
                            .iter()
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
            TestStep::new("non-Git project rejects Worktree mode without disconnecting").with_action(|app, window, _| {
                let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                panel.update(app, |panel, ctx| {
                    assert!(panel.connected && !panel.worktree_mode
                        && panel.snapshot.as_ref().is_some_and(|snapshot| !snapshot.worktree_available));
                    panel.handle_action(&Action::Mode(true), ctx);
                    panel.handle_action(&Action::Mode(false), ctx);
                    assert!(panel.connected && !panel.mode_pending && !panel.worktree_mode && panel.snapshot.is_some());
                });
            }),
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
                            .participant_names
                            .iter()
                            .find(|(_, name)| name.as_str() == "capture-worker")
                            .unwrap()
                            .0
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
    driver = driver
        .with_step(
            TestStep::new("connect existing Explorer to owned SSH project").with_action(
                |app, window, data| {
                    let terminal = app.views_of_type::<crate::terminal::TerminalView>(window).unwrap()[0].clone();
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    workspace.update(app, |workspace, ctx| assert!(workspace.focus_terminal_view_locally(terminal.id(), ctx)));
                    let (parent, cwd) = terminal.read(app, |terminal, _| (terminal.active_block_session_id().unwrap(), terminal.pwd().unwrap()));
                    data.insert("owned_parent_session", parent);
                    data.insert("owned_parent_cwd", cwd);
                    app.update(|ctx| {
                        // Shell hooks are simulated on a local PTY; remote generators must not write into it.
                        crate::settings::DebugSettings::handle(ctx).update(ctx, |settings, ctx| {
                            settings.force_disable_in_band_generators.set_value(true, ctx)
                                .expect("Disable generators for the owned SSH hook fixture");
                        });
                    });
                    terminal.update(app, |terminal, _| {
                        use crate::terminal::model::ansi::{Handler, InitShellValue, BootstrappedValue, PreexecValue, PrecmdValue, SSHValue};
                        let config = std::env::var("WARP_TEST_SSH_CONFIG").unwrap();
                        let command = format!("ssh -F \"{config}\" warpai-test");
                        let mut model = terminal.model.lock();
                        model.block_list_mut().active_block_mut().start();
                        for character in command.chars() {
                            model.block_list_mut().input(character);
                        }
                        model.preexec(PreexecValue { command });
                        if let Some(socket) = std::env::var_os("WARP_TEST_SSH_SOCKET") {
                            model.ssh(SSHValue { socket_path: socket.into(), remote_shell: "bash".into() });
                        }
                        // Populate simulated session metadata without bootstrapping the real local PTY.
                        let (wakeups, _wakeups_rx) = async_channel::unbounded();
                        let (events, events_rx) = async_channel::unbounded();
                        let (reads, _reads_rx) = async_broadcast::broadcast(1);
                        let original_listener = std::mem::replace(
                            &mut model.event_proxy,
                            crate::terminal::event_listener::ChannelEventListener::new(wakeups, events, reads),
                        );
                        model.init_shell(InitShellValue {
                            session_id: 987654321_u64.into(),
                            shell: "bash".into(),
                            is_subshell: true,
                            user: "fixture".into(),
                            hostname: "warpai-test".into(),
                            ..Default::default()
                        });
                        model.event_proxy = original_listener;
                        while let Ok(event) = events_rx.try_recv() {
                            if !matches!(event, crate::terminal::event::Event::Handler(
                                crate::terminal::model::terminal_model::HandlerEvent::InitShell { .. }
                            )) {
                                model.event_proxy.send_terminal_event(event);
                            }
                        }
                        model.bootstrapped(BootstrappedValue {
                            shell: "bash".into(),
                            home_dir: Some(std::env::var("WARP_TEST_REMOTE_HOME").unwrap()),
                            os_category: Some(if cfg!(target_os = "windows") { "Windows" } else { "Darwin" }.into()),
                            ..Default::default()
                        });
                        model.precmd(PrecmdValue {
                            session_id: Some(987654321),
                            pwd: Some(std::env::var("WARP_TEST_REMOTE_ROOT").unwrap()),
                            ..Default::default()
                        });
                    });
                },
            ).add_named_assertion("terminal confirms owned SSH cwd", |app, window| {
                warpui::async_assert!(app.update(|ctx| crate::remote_server::selected_session::selected_ssh(ctx, window)
                    .is_some_and(|(profile, _)| profile.target == "warpai-test" && profile.remote_root == std::env::var("WARP_TEST_REMOTE_ROOT").unwrap())))
            }),
        )
        .with_step(
            TestStep::new("open Explorer after confirmed SSH cd").with_action(|app, window, _| {
                    app.update(|ctx| {
                        let sizes = ResizableData::as_ref(ctx).get_all_handles(window).unwrap();
                        sizes.left_panel_width.lock().unwrap().set_size(320.);
                    });
                    let terminal = app.views_of_type::<crate::terminal::TerminalView>(window).unwrap()[0].clone();
                    terminal.update(app, |terminal, ctx| {
                        terminal.input().update(ctx, |input, ctx| input.replace_buffer_content("unsent collaboration draft", ctx));
                    });
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    workspace.update(app, |workspace, ctx| {
                        if !workspace.is_left_panel_open(ctx) {
                            workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx);
                        }
                    });
                    let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                    panel.update(app, |panel, ctx| {
                        panel.handle_action_with_force_open(
                            &LeftPanelAction::ProjectExplorer,
                            false,
                            ctx,
                        )
                    });
                    let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                    let tree = panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)).unwrap();
                    tree.update(app, |tree, ctx| tree.connect_ssh_checkpoint(ctx));
                },
            ),
        )
        .with_step(
            TestStep::new("remote Explorer uses native file rows")
                .add_named_assertion("Code Review toolbar entry is supported", |app, _| {
                    warpui::async_assert!(app.update(|ctx| crate::workspace::header_toolbar_item::HeaderToolbarItemKind::CodeReview.is_available(ctx)))
                })
                .add_named_assertion("owned remote tree populated", |app, window| {
                    let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                    warpui::async_assert!(panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx).is_some_and(|tree| tree.as_ref(ctx).ssh_checkpoint_ready(ctx, window))))
                })
                .with_take_screenshot("live-ssh-file-explorer.png"),
        )
        .with_step(
            TestStep::new("click remote code in existing Explorer").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                    let tree = panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)).unwrap();
                    tree.update(app, |tree, ctx| tree.open_ssh_checkpoint("example.rs", ctx));
                },
            ),
        )
        .with_step(
            TestStep::new("remote code opens in existing app editor")
                .add_named_assertion("editor retains original SSH save source", |app, window| {
                    warpui::async_assert!(app.update(|ctx| crate::remote_server::selected_session::selected_ssh(ctx, window)
                        .is_some_and(|(profile, _)| profile.remote_root == std::env::var("WARP_TEST_REMOTE_ROOT").unwrap())) && app
                        .views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(
                            window
                        )
                        .is_some_and(|views| views.iter().any(|view| view.read(
                            app,
                            |view, app| view.file_path().is_some_and(|path| {
                                warp_files::FileModel::as_ref(app)
                                    .ssh_source(path)
                                    .is_some_and(|source| source.path.ends_with("example.rs") && view.editor().as_ref(app).text(app).as_str().contains("Remote changed"))
                            })
                        ))))
                })
                .with_take_screenshot("live-ssh-code-editor.png"),
        )
        .with_step(TestStep::new("edit remote code in existing editor").with_action(|app, window, _| {
            let views = app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window).unwrap();
            let editor = views.iter().find(|view| view.read(app, |view, app| view.file_path().is_some_and(|path| warp_files::FileModel::as_ref(app).is_ssh_file(path)))).unwrap().clone();
            editor.update(app, |view, ctx| {
                assert!(view.file_loaded(ctx));
                view.editor().update(ctx, |editor, ctx| {
                    let offset = editor.cursor_head_offset(ctx);
                    editor.apply_edits(vec1::vec1![("// Native remote UI save\n".to_string(), offset..offset)], ctx);
                });
            });
        }).add_named_assertion("user edit marks remote buffer dirty", |app, window| {
            warpui::async_assert!(app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window).is_some_and(|views| views.iter().any(|view| view.read(app, |view, app| view.file_path().is_some_and(|path| warp_files::FileModel::as_ref(app).is_ssh_file(path)) && view.has_unsaved_changes(app)))))
        }).with_take_screenshot("live-ssh-code-dirty.png"))
        .with_step(TestStep::new("save remote code through existing FileModel").with_action(|app, window, _| {
            let views = app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window).unwrap();
            let editor = views.iter().find(|view| view.read(app, |view, app| view.file_path().is_some_and(|path| warp_files::FileModel::as_ref(app).is_ssh_file(path)))).unwrap().clone();
            editor.update(app, |view, ctx| view.save_local(ctx).expect("Remote editor save starts"));
        }).add_named_assertion("remote original contains saved editor changes", |app, window| {
            let remote_path = std::path::PathBuf::from(std::env::var_os("WARP_TEST_REMOTE_ROOT").unwrap()).join("example.rs");
            warpui::async_assert!(std::fs::read_to_string(remote_path).is_ok_and(|text| text.contains("// Native remote UI save")) && app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window).is_some_and(|views| views.iter().any(|view| view.read(app, |view, app| view.file_path().is_some_and(|path| warp_files::FileModel::as_ref(app).is_ssh_file(path)) && !view.has_unsaved_changes(app)))))
        }).with_take_screenshot("live-ssh-code-saved.png"))
        .with_step(
            TestStep::new("click remote Markdown in existing Explorer").with_action(
                |app, window, _| {
                    let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                    let tree = panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)).unwrap();
                    tree.update(app, |tree, ctx| tree.open_ssh_checkpoint("preview.md", ctx));
                },
            ),
        )
        .with_step(
            TestStep::new("remote Markdown renders with transferred relative image")
                .add_named_assertion(
                    "in-app preview and remote resource loaded",
                    |app, window| {
                        warpui::async_assert!(app
                            .views_of_type::<crate::notebooks::file::FileNotebookView>(window)
                            .is_some_and(|views| views
                                .iter()
                                .any(|view| view
                                    .read(app, |view, app| view.ssh_checkpoint_ready(app)))))
                    },
                )
                .with_take_screenshot("live-ssh-markdown-preview.png"),
        )
        .with_step(
            TestStep::new("connect existing Review to owned SSH repository").with_action(
                |app, window, _| {
                    let root = app.root_view::<RootView>(window).unwrap();
                    let workspace =
                        root.read(app, |root, _| root.workspace_view().unwrap().clone());
                    workspace.update(app, |workspace, ctx| {
                        if workspace.is_left_panel_open(ctx) {
                            workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx);
                        }
                        if !workspace.active_tab_pane_group().as_ref(ctx).right_panel_open {
                            workspace.handle_action(&WorkspaceAction::ToggleRightPanel, ctx);
                        }
                    });
                },
            ),
        )
        .with_step(
            TestStep::new("remote changes render in existing Review")
                .add_named_assertion("remote Review has one changed file", |app, window| {
                    warpui::async_assert!(app.update(|ctx| crate::workspace::header_toolbar_item::HeaderToolbarItemKind::CodeReview.is_available(ctx)) && app
                        .views_of_type::<crate::code_review::code_review_view::CodeReviewView>(
                            window
                        )
                        .is_some_and(|views| views.iter().any(|view| view.read(
                            app,
                            |view, app| view.diff_state_model().as_ref(app).is_ssh()
                                && view
                                    .loaded_diff_stats()
                                    .is_some_and(|stats| stats.files_changed == 1)
                        ))))
                })
                .with_take_screenshot("live-ssh-file-review.png"),
        )
        .with_step(
            TestStep::new("open remote Review file in existing Code View")
                .with_action(|app, window, _| {
                    let view = app.views_of_type::<crate::code_review::code_review_view::CodeReviewView>(window)
                        .unwrap().into_iter().find(|view| view.read(app, |view, app| view.diff_state_model().as_ref(app).is_ssh() && view.has_file_states())).unwrap();
                    view.update(app, |view, ctx| view.open_ssh_checkpoint(ctx));
                })
                .add_named_assertion("Review file retains SSH editor identity", |app, window| {
                    warpui::async_assert!(app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window)
                        .is_some_and(|views| views.iter().any(|view| view.read(app, |view, app|
                            view.file_path().is_some_and(|path| warp_files::FileModel::as_ref(app).ssh_source(path)
                                .is_some_and(|source| source.path.ends_with("example.rs")))))))
                })
                .with_take_screenshot("live-ssh-review-code-view.png"),
        );
    filenames.extend(
        [
            "live-ssh-file-explorer.png",
            "live-ssh-code-editor.png",
            "live-ssh-code-dirty.png",
            "live-ssh-code-saved.png",
            "live-ssh-markdown-preview.png",
            "live-ssh-file-review.png",
            "live-ssh-review-code-view.png",
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
    driver = driver
        .with_step(
            TestStep::new("exit owned SSH restores terminal-driven local cwd")
                .with_action(|app, window, data| {
                    let parent = *data
                        .get::<_, warp_core::SessionId>("owned_parent_session")
                        .unwrap();
                    let cwd = data.get::<_, String>("owned_parent_cwd").unwrap().clone();
                    let terminal = app
                        .views_of_type::<crate::terminal::TerminalView>(window)
                        .unwrap()[0]
                        .clone();
                    terminal.update(app, |terminal, _| {
                        use crate::terminal::model::ansi::{ExitShellValue, Handler, PrecmdValue};
                        let mut model = terminal.model.lock();
                        model.command_finished(Default::default());
                        model.exit_shell(ExitShellValue {
                            session_id: 987654321_u64.into(),
                        });
                        model.precmd(PrecmdValue {
                            session_id: Some(parent.as_u64()),
                            pwd: Some(cwd),
                            ..Default::default()
                        });
                    });
                })
                .add_named_assertion(
                    "original local terminal selection restored",
                    |app, window| {
                        warpui::async_assert!(app.update(|ctx| {
                            crate::workspace::ActiveSession::as_ref(ctx)
                                .session(window)
                                .is_some_and(|session| session.is_local())
                                && crate::remote_server::selected_session::selected_ssh(ctx, window)
                                    .is_none()
                        }))
                    },
                ),
        )
        .with_step(
            TestStep::new("prepare local SSH banner fixture").with_action(|app, window, _| {
                let terminal = app
                    .views_of_type::<crate::terminal::TerminalView>(window)
                    .unwrap()[0]
                    .clone();
                terminal.update(app, |terminal, ctx| {
                    ctx.focus_self();
                    terminal.input().update(ctx, |input, ctx| {
                        input.replace_buffer_content("unsent collaboration draft", ctx)
                    });
                });
            }),
        );
    for (theme_kind, theme_name) in [
        (ThemeKind::ClaudeWarmLight, "claude-warm"),
        (ThemeKind::Dark, "dark"),
    ] {
        let filename = format!("ssh-powershell-integration-{theme_name}.png");
        filenames.push(filename.clone());
        driver = driver.with_step(
            TestStep::new(&format!(
                "review SSH PowerShell integration entry {theme_name}"
            ))
            .with_action(move |app, window, _| {
                app.update(|ctx| {
                    let theme = Settings::theme_for_theme_kind(&theme_kind, ctx);
                    Appearance::handle(ctx).update(ctx, |appearance, ctx| {
                        appearance.set_theme(theme, ctx);
                    });
                    ctx.set_zoom_factor(1.);
                });
                let terminal = app
                    .views_of_type::<crate::terminal::TerminalView>(window)
                    .unwrap()[0]
                    .clone();
                terminal.update(app, |terminal, ctx| {
                    use crate::terminal::model::ansi::{Handler, PreexecValue};
                    if terminal
                        .model
                        .lock()
                        .block_list()
                        .active_block()
                        .block_banner()
                        .is_none()
                    {
                        // A banner belongs to a running block; no SSH process is executed.
                        let mut model = terminal.model.lock();
                        model.block_list_mut().active_block_mut().start();
                        // A fresh Windows prompt expects its shell's Reset Grid OSC before input.
                        model.on_reset_grid();
                        for character in "ssh user@host".chars() {
                            model.block_list_mut().input(character);
                        }
                        model.block_list_mut().preexec(PreexecValue {
                            command: "ssh user@host".into(),
                        });
                    }
                    terminal.handle_action(
                        &crate::terminal::view::TerminalAction::ShowWarpifySshBanner(
                            "ssh user@host".into(),
                            Some("host".into()),
                        ),
                        ctx,
                    );
                });
            })
            .add_named_assertion(
                "SSH banner belongs to a visible command block",
                |app, window| {
                    let terminal = app
                        .views_of_type::<crate::terminal::TerminalView>(window)
                        .unwrap()[0]
                        .clone();
                    warpui::async_assert!(
                        terminal.read(app, |terminal, _| {
                            let model = terminal.model.lock();
                            let block = model.block_list().active_block();
                            block.command_to_string() == "ssh user@host"
                                && block.block_banner().is_some()
                        }) && checkpoint_draft(app, window) == "unsent collaboration draft"
                    )
                },
            )
            .with_take_screenshot(filename),
        );
    }
    for (theme, label) in [(ThemeKind::Light, "light"), (ThemeKind::Dark, "dark")] {
        for width in [320, 420] {
            for worktree in [false, true] {
                for state in ["empty", "loading", "available", "error", "balance", "overflow"] {
                    let filename = format!("usage-{label}-{width}-{worktree}-{state}.png");
                    filenames.push(filename.clone());
                    let theme = theme.clone();
                    driver = driver.with_step(TestStep::new(&filename).with_action(move |app, window, _| {
                        app.update(|ctx| {
                            let origin = ctx.window_bounds(&window).unwrap().origin();
                            ctx.set_and_cache_window_bounds(window, pathfinder_geometry::rect::RectF::new(origin,
                                pathfinder_geometry::vector::vec2f(1200., 800.)));
                            ctx.set_zoom_factor(1.25);
                            let colors = Settings::theme_for_theme_kind(&theme, ctx);
                            Appearance::handle(ctx).update(ctx, |appearance, ctx| appearance.set_theme(colors, ctx));
                            ResizableData::as_ref(ctx).get_all_handles(window).unwrap().left_panel_width.lock().unwrap().set_size(width as f32);
                            crate::agent_usage::AgentUsage::handle(ctx).update(ctx, |model, ctx| model.capture_fixture(state, ctx));
                        });
                        let root = app.root_view::<RootView>(window).unwrap();
                        let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                        workspace.update(app, |workspace, ctx| {
                            if workspace.active_tab_pane_group().as_ref(ctx).right_panel_open {
                                workspace.handle_action(&WorkspaceAction::ToggleRightPanel, ctx);
                            }
                            if !workspace.is_left_panel_open(ctx) {
                                workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx);
                            }
                        });
                        let tools = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
                        tools.update(app, |tools, ctx| tools.handle_action_with_force_open(&LeftPanelAction::Collaboration, true, ctx));
                        let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        panel.update(app, |panel, ctx| {
                            panel.worktree_layout_checkpoint("team", ctx);
                            panel.worktree_mode = worktree;
                            panel.usage_scroll = Default::default();
                            if state == "overflow" { panel.scroll.scroll_by(800.0.into_pixels()); }
                            ctx.notify();
                        });
                    }).add_named_assertion("usage fixture preserves terminal draft and account count", move |app, window| {
                        let root = app.root_view::<RootView>(window).unwrap();
                        let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                        let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        warpui::async_assert!(workspace.read(app, |workspace, ctx| workspace.is_left_panel_open(ctx))
                            && panel.read(app, |panel, _| panel.visible)
                            && app.read(|ctx| crate::agent_usage::AgentUsage::as_ref(ctx).accounts.len()
                            == if state == "empty" { 0 } else if state == "overflow" { 8 } else { 3 })
                            && checkpoint_draft(app, window) == "unsent collaboration draft")
                    }).with_take_screenshot(filename));
                    if state == "available" {
                        driver = driver.with_step(TestStep::new("focus change retains global usage account state").with_action(|app, window, _| {
                            let root = app.root_view::<RootView>(window).unwrap();
                            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                            workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::FocusLeftPanel, ctx));
                        }).add_named_assertion("focus keeps configured accounts and cached percentages", |app, _| {
                            warpui::async_assert!(app.read(|ctx| {
                                let usage = crate::agent_usage::AgentUsage::as_ref(ctx);
                                usage.accounts.len() == 3 && usage.readings.len() == 3
                                    && usage.readings.get(&uuid::Uuid::from_u128(1).to_string())
                                        .is_some_and(|reading| reading.as_ref().is_ok_and(|reading| reading.windows.first().is_some_and(|window| window.used == 37.)))
                            }))
                        }));
                    }
                    if state == "overflow" {
                        let filename = format!("usage-{label}-{width}-{worktree}-scrolled.png");
                        filenames.push(filename.clone());
                        driver = driver.with_step(TestStep::new(&filename).with_action(|app, window, _| {
                            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                            panel.update(app, |panel, ctx| { panel.usage_scroll.scroll_by(256.0.into_pixels()); ctx.notify(); });
                        }).add_named_assertion("usage scroll advances independently", |app, window| {
                            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                            warpui::async_assert!(panel.read(app, |panel, _| panel.usage_scroll.scroll_start().as_f32() > 0.))
                        }).with_take_screenshot(filename));
                    }
                }
            }
        }
    }
    driver = driver.with_step(TestStep::new("restore empty usage fixture").with_action(|app, _, _| {
        app.update(|ctx| crate::agent_usage::AgentUsage::handle(ctx).update(ctx, |model, ctx| model.capture_fixture("empty", ctx)));
    }));
    driver = driver.with_step(
        TestStep::new("open native communication settings").with_action(|app, window, _| {
            let terminal = app.views_of_type::<crate::terminal::TerminalView>(window).unwrap()[0].clone();
            terminal.update(app, |terminal, ctx| {
                use crate::terminal::model::ansi::Handler;
                terminal.handle_action(
                    &crate::terminal::view::TerminalAction::DismissWarpifyBanner(crate::terminal::view::RememberForWarpification::DoNotRememberSSHHost),
                    ctx,
                );
                let mut model = terminal.model.lock();
                model.block_list_mut().command_finished(Default::default());
                model.block_list_mut().precmd(Default::default());
            });
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
    for (theme, label) in [(ThemeKind::Light, "light"), (ThemeKind::Dark, "dark")] {
        for width in [800, 1200] {
            let filename = format!("settings-usage-{label}-{width}.png");
            filenames.push(filename.clone());
            let theme = theme.clone();
            driver = driver.with_step(TestStep::new(&filename).with_action(move |app, window, _| {
                app.update(|ctx| {
                    let origin = ctx.window_bounds(&window).unwrap().origin();
                    ctx.set_and_cache_window_bounds(window, pathfinder_geometry::rect::RectF::new(origin,
                        pathfinder_geometry::vector::vec2f(width as f32, 800.)));
                    ctx.set_zoom_factor(1.25);
                    let colors = Settings::theme_for_theme_kind(&theme, ctx);
                    Appearance::handle(ctx).update(ctx, |appearance, ctx| appearance.set_theme(colors, ctx));
                    crate::agent_usage::AgentUsage::handle(ctx).update(ctx, |model, ctx| model.capture_fixture("available", ctx));
                });
                let root = app.root_view::<RootView>(window).unwrap();
                let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
                workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ShowSettingsPageWithSearch {
                    search_query: "data usage".into(), section: Some(crate::settings_view::SettingsSection::Features),
                }, ctx));
            }).add_named_assertion("native usage settings exists", |app, window| {
                warpui::async_assert!(app.views_of_type::<crate::settings_view::agent_usage::UsageSettingsView>(window)
                    .is_some_and(|views| !views.is_empty()))
            }).with_take_screenshot(filename));
        }
    }
    driver = driver.with_step(TestStep::new("open native About update settings").with_action(|app, window, _| {
        let root = app.root_view::<RootView>(window).unwrap();
        let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
        workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ShowSettingsPageWithSearch {
            search_query:String::new(), section:Some(crate::settings_view::SettingsSection::About),
        }, ctx));
    }).add_named_assertion("About view is available", |app, window| {
        warpui::async_assert!(app.views_of_type::<crate::settings_view::about_page::AboutPageView>(window)
            .is_some_and(|views| !views.is_empty()))
    }));
    driver = driver.with_step(TestStep::new("About startup switch uses registered native actions").with_action(|app, window, _| {
        let about = app.views_of_type::<crate::settings_view::about_page::AboutPageView>(window).unwrap()[0].clone();
        app.dispatch_typed_action(window, &[about.id()], &crate::settings_view::about_page::AboutAction::ToggleStartup);
    }).add_named_assertion("startup preference is enabled", |app, _| {
        warpui::async_assert!(app.read(|ctx| *crate::release_updates::UpdateSettings::as_ref(ctx).check_on_startup.value()))
    }));
    for (theme, label) in [(ThemeKind::Light, "light"), (ThemeKind::Dark, "dark")] {
        for width in [800, 1200] {
            for state in ["idle", "checking", "current", "available", "unavailable", "error"] {
                let filename = format!("about-updates-{label}-{width}-{state}.png");
                filenames.push(filename.clone());
                let theme = theme.clone();
                driver = driver.with_step(TestStep::new(&filename).with_action(move |app, window, _| {
                    app.update(|ctx| {
                        let origin = ctx.window_bounds(&window).unwrap().origin();
                        ctx.set_and_cache_window_bounds(window, pathfinder_geometry::rect::RectF::new(origin,
                            pathfinder_geometry::vector::vec2f(width as f32, 800.)));
                        ctx.set_zoom_factor(1.25);
                        let colors = Settings::theme_for_theme_kind(&theme, ctx);
                        Appearance::handle(ctx).update(ctx, |appearance, ctx| appearance.set_theme(colors, ctx));
                        crate::release_updates::ReleaseUpdates::handle(ctx).update(ctx, |updates, ctx| {
                            updates.checking = state == "checking";
                            updates.available = (["available", "unavailable"].contains(&state)).then(|| "v1.99.0".into());
                            updates.download_url = (state == "available").then(|| crate::release_updates::installer_url("v1.99.0", std::env::consts::OS, std::env::consts::ARCH)).flatten();
                            updates.status = match state {
                                "checking" => "Checking for updates…",
                                "current" => "Warpai is up to date.",
                                "available" | "unavailable" => "Warpai v1.99.0 is available.",
                                "error" => "Could not check for updates. Try again or open GitHub Releases.",
                                _ => "",
                            }.into();
                            ctx.notify();
                        });
                    });
                }).with_take_screenshot(filename));
            }
        }
    }
    driver = driver.with_step(TestStep::new("restore disabled startup checking").with_action(|app, window, _| {
        let about = app.views_of_type::<crate::settings_view::about_page::AboutPageView>(window).unwrap()[0].clone();
        app.dispatch_typed_action(window, &[about.id()], &crate::settings_view::about_page::AboutAction::ToggleStartup);
    }).add_named_assertion("startup preference is disabled", |app, _| {
        warpui::async_assert!(app.read(|ctx| !*crate::release_updates::UpdateSettings::as_ref(ctx).check_on_startup.value()))
    }));
    #[cfg(windows)]
    if std::env::var_os("WARP_TEST_WSL_ROOT").is_some() {
        filenames.extend(["live-wsl-file-explorer.png", "live-wsl-code-editor.png", "live-wsl-collaboration.png", "live-wsl-markdown-preview.png", "live-wsl-file-review.png", "settings-wsl-communication.png"].map(str::to_owned));
        driver = driver.with_step(TestStep::new("select confirmed owned WSL environment").with_action(|app, window, _| {
            std::env::set_var("WARP_TEST_REMOTE_ROOT", std::env::var("WARP_TEST_WSL_ROOT").unwrap());
            app.update(|ctx| super::AgentCommunication::handle(ctx).update(ctx, |model, ctx| {
                model.wsl.preferences.enabled = true;
                model.wsl.preferences.distribution = Some(std::env::var("WARP_TEST_WSL_DISTRIBUTION").unwrap());
                model.wsl.preferences.user = Some(std::env::var("WARP_TEST_WSL_USER").unwrap());
                ctx.notify();
            }));
            let terminal = app.views_of_type::<crate::terminal::TerminalView>(window).unwrap()[0].clone();
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            workspace.update(app, |workspace, ctx| {
                assert!(workspace.focus_terminal_view_locally(terminal.id(), ctx));
                if !workspace.is_left_panel_open(ctx) { workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx); }
            });
                    terminal.update(app, |terminal, _| {
                        use crate::terminal::model::ansi::{Handler, InitShellValue, BootstrappedValue, PreexecValue, PrecmdValue};
                        let command = format!("wsl --distribution {}", std::env::var("WARP_TEST_WSL_DISTRIBUTION").unwrap());
                        let mut model = terminal.model.lock();
                        model.block_list_mut().active_block_mut().start();
                        // A fresh Windows prompt expects its shell's Reset Grid OSC before input.
                        model.on_reset_grid();
                        for character in command.chars() {
                            model.block_list_mut().input(character);
                        }
                        model.preexec(PreexecValue { command });
                        // Populate simulated session metadata without bootstrapping the real local PTY.
                        let (wakeups, _wakeups_rx) = async_channel::unbounded();
                        let (events, events_rx) = async_channel::unbounded();
                        let (reads, _reads_rx) = async_broadcast::broadcast(1);
                        let original_listener = std::mem::replace(
                            &mut model.event_proxy,
                            crate::terminal::event_listener::ChannelEventListener::new(wakeups, events, reads),
                        );
                        model.init_shell(InitShellValue {
                            session_id: 987654322_u64.into(),
                            shell: "bash".into(),
                            is_subshell: true,
                            user: std::env::var("WARP_TEST_WSL_USER").unwrap(),
                            hostname: "owned-wsl".into(),
                            wsl_name: Some(std::env::var("WARP_TEST_WSL_DISTRIBUTION").unwrap()),
                            ..Default::default()
                        });
                        model.event_proxy = original_listener;
                        while let Ok(event) = events_rx.try_recv() {
                            if !matches!(event, crate::terminal::event::Event::Handler(
                                crate::terminal::model::terminal_model::HandlerEvent::InitShell { .. }
                            )) {
                                model.event_proxy.send_terminal_event(event);
                            }
                        }
                        model.bootstrapped(BootstrappedValue {
                            shell: "bash".into(),
                            home_dir: Some("/home/warpai-test".into()),
                            os_category: Some("Linux".into()),
                            wsl_name: Some(std::env::var("WARP_TEST_WSL_DISTRIBUTION").unwrap()),
                            ..Default::default()
                        });
                        model.precmd(PrecmdValue {
                            session_id: Some(987654322),
                            pwd: Some(std::env::var("WARP_TEST_WSL_ROOT").unwrap()),
                            ..Default::default()
                        });
                    });
        }).add_named_assertion("confirmed WSL selection carries guest user", |app, window| {
            warpui::async_assert!(app.update(|ctx| crate::remote_server::selected_session::selected_ssh(ctx, window)
                .is_some_and(|(profile, connection)| profile.remote_root == std::env::var("WARP_TEST_WSL_ROOT").unwrap()
                    && matches!(connection, SshConnection::Wsl { user, .. } if user == "warpai-test"))))
        })).with_step(TestStep::new("open existing Explorer on WSL").with_action(|app, window, _| {
            let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| panel.handle_action_with_force_open(&LeftPanelAction::ProjectExplorer, false, ctx));
            let tree = panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)).unwrap();
            tree.update(app, |tree, ctx| tree.connect_ssh_checkpoint(ctx));
        }).add_named_assertion("owned WSL tree populated", |app, window| {
            let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
            warpui::async_assert!(panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)
                .is_some_and(|tree| tree.as_ref(ctx).ssh_checkpoint_ready(ctx, window))))
        }).with_take_screenshot("live-wsl-file-explorer.png"))
        .with_step(TestStep::new("open WSL source in native editor").with_action(|app, window, _| {
            let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
            let tree = panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)).unwrap();
            tree.update(app, |tree, ctx| tree.open_ssh_checkpoint("example.rs", ctx));
        }).add_named_assertion("native editor retains WSL save source", |app, window| {
            warpui::async_assert!(app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window)
                .is_some_and(|views| views.iter().any(|view| view.read(app, |view, ctx|
                    view.file_path().is_some_and(|path| warp_files::FileModel::as_ref(ctx).ssh_source(path)
                        .is_some_and(|source| matches!(source.files.connection, SshConnection::Wsl { .. })))))))
        }).with_take_screenshot("live-wsl-code-editor.png"))
        .with_step(TestStep::new("open WSL collaboration using existing tools").with_action(|app, window, _| {
            let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| panel.handle_action_with_force_open(&LeftPanelAction::Collaboration, false, ctx));
        }).add_named_assertion("WSL collaboration uses guest Companion", |app, window| {
            let panel = app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
            warpui::async_assert!(panel.read(app, |panel, _| panel.visible && panel.connected
                && matches!(panel.remote_connection, Some(SshConnection::Wsl { .. }))))
        }).with_take_screenshot("live-wsl-collaboration.png"))
        .with_step(TestStep::new("open WSL Markdown in existing Explorer").with_action(|app, window, _| {
            let panel = app.views_of_type::<LeftPanelView>(window).unwrap()[0].clone();
            panel.update(app, |panel, ctx| panel.handle_action_with_force_open(&LeftPanelAction::ProjectExplorer, false, ctx));
            let tree = panel.read(app, |panel, ctx| panel.active_file_tree_view(ctx)).unwrap();
            tree.update(app, |tree, ctx| tree.open_ssh_checkpoint("preview.md", ctx));
        }).add_named_assertion("WSL Markdown and guest image loaded", |app, window| {
            warpui::async_assert!(app.views_of_type::<crate::notebooks::file::FileNotebookView>(window)
                .is_some_and(|views| views.iter().any(|view| view.read(app, |view, ctx| view.wsl_checkpoint_ready(ctx)))))
        }).with_take_screenshot("live-wsl-markdown-preview.png"))
        .with_step(TestStep::new("open original Review on WSL").with_action(|app, window, _| {
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            workspace.update(app, |workspace, ctx| {
                if workspace.is_left_panel_open(ctx) { workspace.handle_action(&WorkspaceAction::ToggleLeftPanel, ctx); }
                if !workspace.active_tab_pane_group().as_ref(ctx).right_panel_open {
                    workspace.handle_action(&WorkspaceAction::ToggleRightPanel, ctx);
                }
            });
        }).add_named_assertion("original Review retains WSL project authority", |app, window| {
            warpui::async_assert!(app.views_of_type::<crate::code_review::code_review_view::CodeReviewView>(window)
                .is_some_and(|views| views.iter().any(|view| view.read(app, |view, ctx|
                    view.diff_state_model().as_ref(ctx).ssh_files().is_some_and(|files|
                        files.connection.is_wsl() && files.canonical_root == std::env::var("WARP_TEST_WSL_ROOT").unwrap())
                    && view.loaded_diff_stats().is_some_and(|stats| stats.files_changed > 0)))))
        }).with_take_screenshot("live-wsl-file-review.png"))
        .with_step(TestStep::new("show current-account WSL MCP settings").with_action(|app, window, _| {
            app.update(|ctx| super::AgentCommunication::handle(ctx).update(ctx, |model, ctx| {
                model.refresh_wsl_targets(ctx);
                model.maintain_wsl(false, ctx);
            }));
            let root = app.root_view::<RootView>(window).unwrap();
            let workspace = root.read(app, |root, _| root.workspace_view().unwrap().clone());
            workspace.update(app, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ShowSettingsPageWithSearch {
                search_query: "agent communication".into(), section: Some(crate::settings_view::SettingsSection::Features),
            }, ctx));
        }).add_named_assertion("WSL settings detect installed bundled component for logged-in account", |app, _| {
            warpui::async_assert!(app.read(|ctx| {
                let settings = &super::AgentCommunication::as_ref(ctx).wsl;
                !settings.busy && settings.account.as_deref() == Some("warpai-test")
                    && settings.companion_installed == Some(true)
                    && settings.selected_target().is_some_and(|target| target.user == "warpai-test")
            }))
        }).with_take_screenshot("settings-wsl-communication.png"));
    }
    let driver = driver.with_on_finish(move |app, window, data| {
        // Keep the original failing step; missing later screenshots must not mask it.
        if data.contains_key(warpui::integration::RUNTIME_TAG_FAILURE_REASON) {
            if let Some(assertion) = data.get("failed_assertion_name") {
                for name in ["live detail scroll reaches lower controls", "native main branch is selected", "native tab and persistent Coordinator", "Coordinator owns its original run", "worker candidate observed", "automatic Worktree participant", "two roles and preserved terminal draft", "worker becomes Coordinator", "project Coordinator persists", "join request was submitted", "unjoined Worktree projection is loaded", "team projection is active", "private checkout and draft retained", "hidden panel is fenced and draft retained", "SSH banner belongs to a visible command block", "Code Review toolbar entry is supported", "remote Review has one changed file", "owned remote tree populated", "original local terminal selection restored"] {
                    if assertion == name {
                        eprintln!("Native checkpoint failed: {name}");
                        // Windows GUI processes may not retain redirected stderr.
                        let _ = std::fs::write(directory.join("checkpoint-assertion.txt"), name);
                    }
                }
            }
            if data.get("failed_assertion_name").is_some_and(|name| name == "team projection is active" || name == "join request was submitted") {
                for panel in app.views_of_type::<CollaborationPanel>(window).unwrap_or_default() {
                    panel.read(app, |panel, _| {
                        let snapshot = panel.snapshot.as_ref();
                        let query = PanelQuery { project: warp_agent_bus::project_root(&worktree_fixture).unwrap_or_default(), worktree: true, ..Default::default() };
                        let query_error = super::BROKER.get().unwrap().operator_panel(&query).err()
                            .and_then(|error| error.downcast::<warp_agent_bus::DomainError>().ok())
                            .map(|error| ["Project path is unavailable", "Project must be a directory", "A registered Git worktree is required", "Worktree registry unavailable", "Git checkout unavailable", "Git repository unavailable", "Invalid Git metadata"].iter().position(|message| *message == error.message).map(|index| index + 1).unwrap_or(99)).unwrap_or(0);
                        let mut exits = Vec::new();
                        for arguments in [vec!["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-common-dir"], vec!["worktree", "list", "--porcelain", "-z"]] {
                            let status = warp_agent_bus::installation::git_executable().ok().and_then(|git|
                                std::process::Command::new(git).args(arguments).current_dir(&worktree_fixture)
                                    .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().ok());
                            exits.push(status.and_then(|status| status.code()).unwrap_or(-1));
                        }
                        // Only fixture booleans, source-owned categories and exit codes leave the runner.
                        let diagnostic = format!("Native Worktree: connected={}, form={}, joined={}, team={}, main={}, submitting={}, query_error={}, git_rev_parse={}, git_registry={}",
                            panel.connected, panel.form.is_some(), snapshot.is_some_and(|snapshot| snapshot.worktree_joined),
                            snapshot.is_some_and(|snapshot| snapshot.project.starts_with("space:")), snapshot.is_some_and(|snapshot| snapshot.worktree_branch.as_deref() == Some("main")),
                            panel.form.as_ref().is_some_and(|form| form.submitting), query_error, exits[0], exits[1]);
                        let _ = std::fs::write(directory.join("checkpoint-worktree.txt"), diagnostic);
                    });
                }
            }
            for editor in app.views_of_type::<crate::code::local_code_editor::LocalCodeEditorView>(window).unwrap_or_default() {
                editor.update(app, |editor, ctx| {
                    if let Some(path) = editor.file_path().filter(|path| path.parent().and_then(|parent| parent.file_name()).is_some_and(|name| name.to_string_lossy().starts_with("warpai-ssh-"))) {
                        let source = warp_files::FileModel::as_ref(ctx).is_ssh_file(path);
                        let cache = path.is_file();
                        let loaded = editor.file_loaded(ctx);
                        eprintln!("Remote editor diagnostic: source={source}, cache={cache}, loaded={loaded}");
                    }
                });
            }
            return Box::pin(async {});
        }
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
    fn collaboration_participant_labels_hide_internal_identifiers() {
        let id = "94cdc52c-48eb-415c-a1f2-355a6be4fbe9";
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "project": "/project", "agents": [{"agent": {"id": id, "terminal": "terminal", "name": "Reviewer", "program": "codex", "project": "/project"}, "online": true, "blocked": false, "paused": false, "ready": true}],
            "tasks": [], "events": [], "admission": "active", "spaces": [], "reservations": [], "messages": []
        })).unwrap();
        assert_eq!(snapshot.participant_label(id), "Reviewer");
        assert_eq!(snapshot.participant_label("warp:/project"), "You");
        assert_eq!(snapshot.participant_label(""), "Unassigned");
        assert_eq!(
            snapshot.participant_label("3479c354-aac2-4ce9-975b-d7aa525c488d"),
            "Unavailable agent"
        );
        assert_eq!(
            snapshot.participant_label("Legacy reviewer"),
            "Legacy reviewer"
        );
    }
    #[test]
    fn collaboration_offline_participant_names_remain_readable_in_history() {
        let id = "94cdc52c-48eb-415c-a1f2-355a6be4fbe9";
        let mut snapshot = Snapshot::default();
        snapshot.participant_names.insert(id.into(), "Reviewer".into());
        assert!(snapshot.agents.is_empty());
        assert_eq!(snapshot.participant_label(id), "Reviewer");
    }
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
