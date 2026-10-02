//! Native collaboration projection; explicit sample mode retains the accepted fixtures.
use crate::appearance::Appearance;
#[path = "panel_controls.rs"]
mod controls;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use warp_agent_bus::{transport::PanelQuery, Agent, Event, Reservation, Task};
use warpui::r#async::Timer;
use warpui::{
    accessibility::{AccessibilityContent, ActionAccessibilityContent, WarpA11yRole},
    elements::{
        ClippedScrollStateHandle, ClippedScrollable, Container, DispatchEventResult, Element,
        EventHandler, Fill, Flex, MouseStateHandle, Padding, ParentElement, ScrollbarWidth,
        Shrinkable,
    },
    ui_components::{button::ButtonVariant, components::UiComponent},
    units::IntoPixels,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

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
    online: bool,
    activity: Option<warp_agent_bus::readiness::Activity>,
    draft: Option<String>,
    blocked: bool,
    paused: bool,
    ready: bool,
    readiness_source: Option<String>,
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
}

pub(crate) struct CollaborationPanel {
    fixtures: Vec<Fixture>,
    selected: usize,
    next: MouseStateHandle,
    scroll: ClippedScrollStateHandle,
    preview: bool,
    snapshot: Option<Snapshot>,
    query: PanelQuery,
    events: Vec<Event>,
    context: Option<(String, Option<String>)>,
    in_flight: bool,
    status: String,
    task_buttons: HashMap<String, MouseStateHandle>,
    page_buttons: [MouseStateHandle; 6],
    generation: u64,
    connected: bool,
    form: Option<controls::Form>,
    control_buttons: [MouseStateHandle; 8],
    focus_buttons: HashMap<String, MouseStateHandle>,
    show_spaces: bool,
    workspace_preview: Option<WorkspacePreview>,
    workspace_buttons: HashMap<String, MouseStateHandle>,
    scope_buttons: [MouseStateHandle; 5],
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    NextFixture,
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
        }
    }

    fn current_context(ctx: &ViewContext<Self>) -> Option<(String, Option<String>)> {
        let active = crate::workspace::ActiveSession::as_ref(ctx);
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

    fn refresh(&mut self, ctx: &mut ViewContext<Self>) {
        let enabled = super::AgentCommunication::as_ref(ctx).preferences.enabled;
        let context = enabled.then(|| Self::current_context(ctx)).flatten();
        if context != self.context {
            self.generation += 1;
            self.context = context.clone();
            self.snapshot = None;
            self.connected = false;
            self.form = None;
            self.show_spaces = false;
            self.workspace_preview = None;
            self.events.clear();
            self.query = Default::default();
            self.scroll = Default::default();
            ctx.notify();
        }
        let Some((directory, terminal)) = context.clone() else {
            let status = if enabled {
                "Select a local terminal project to view collaboration."
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
        let Some(broker) = super::BROKER.get().cloned() else {
            self.status = "Coordinator unavailable. Restart Warpai to reconnect.".into();
            ctx.notify();
            return;
        };
        self.in_flight = true;
        let mut query = self.query.clone();
        let generation = self.generation;
        query.terminal = terminal;
        ctx.spawn(async move {
            let root = warp_agent_bus::project_root(std::path::Path::new(&directory))?;
            query.project = root;
            let value = broker.operator_panel(&query)?;
            serde_json::from_value::<Snapshot>(value).map_err(anyhow::Error::from)
        }, move |panel, result: anyhow::Result<Snapshot>, ctx| {
            panel.in_flight = false;
            if panel.generation != generation || panel.context != context || Self::current_context(ctx) != context
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
                    panel.focus_buttons.retain(|id, _| snapshot.agents.iter().any(|row| &row.agent.id == id));
                    for row in &snapshot.agents { panel.focus_buttons.entry(row.agent.id.clone()).or_default(); }
                    panel.workspace_buttons.retain(|id, _| snapshot.spaces.iter().flat_map(|space| &space.workspaces).any(|workspace| &workspace.id == id));
                    for workspace in snapshot.spaces.iter().flat_map(|space| &space.workspaces) { panel.workspace_buttons.entry(workspace.id.clone()).or_default(); }
                    panel.connected = true;
                    panel.status = match snapshot.admission.as_str() {
                        "revoked" => "Participation revoked. Existing effects may still be running; review the mapping and open a fresh shared pane.",
                        "directory_mismatch" => "This pane changed checkout. Its shared native connection is unavailable in this directory; open a fresh pane for the reviewed workspace.",
                        _ => "Connected to the local coordinator. Execution and presence are separate.",
                    }.into();
                    panel.snapshot = Some(snapshot);
                }
                Err(error) => {
                    panel.connected = false;
                    let code = error.downcast_ref::<warp_agent_bus::DomainError>()
                        .map(|error| error.code.as_str()).unwrap_or("coordinator_unavailable");
                    panel.status = format!("Could not update collaboration ({code}). Last received state may be stale; refresh or restart Warpai.");
                }
            }
            ctx.notify();
        });
    }

    fn live_fixture(&self) -> Fixture {
        let mut fixture = Fixture {
            state: "live".into(),
            guidance: self.status.clone(),
            sections: vec![],
        };
        let Some(snapshot) = &self.snapshot else {
            return fixture;
        };
        fixture.sections.push(Section {
            title: "Scope".into(),
            rows: vec![
                format!(
                    "{} · {}",
                    if cfg!(target_os = "windows") {
                        "This Windows PC"
                    } else {
                        "This Mac"
                    },
                    snapshot.project
                ),
                "Only explicitly participating agents in this scope can collaborate.".into(),
            ],
        });
        if self.show_spaces {
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
                    rows: vec![
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
                    ],
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
                        format!(
                            "{} · {} · {} · {} · draft {} · {} · readiness {} ({})",
                            row.agent.name,
                            row.agent.program,
                            if row.online { "online" } else { "offline" },
                            row.activity
                                .map(|activity| match activity {
                                    warp_agent_bus::readiness::Activity::Starting => "starting",
                                    warp_agent_bus::readiness::Activity::Idle => "idle",
                                    warp_agent_bus::readiness::Activity::Working => "working",
                                    warp_agent_bus::readiness::Activity::WaitingApproval =>
                                        "waiting for approval",
                                    warp_agent_bus::readiness::Activity::WaitingInput =>
                                        "waiting for input",
                                    warp_agent_bus::readiness::Activity::Cancelled => "cancelled",
                                    warp_agent_bus::readiness::Activity::Error => "error",
                                })
                                .unwrap_or("unknown activity"),
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
                            row.readiness_source.as_deref().unwrap_or("unavailable")
                        )
                    })
                    .collect(),
            });
            if snapshot.agents.is_empty() {
                fixture.sections.push(Section { title: "No participating agents".into(), rows: vec!["Configure installed agents in communication settings, then start them in a new terminal pane.".into()] });
            }
        }
        fixture.sections.push(Section {
            title: "File reservations · current checkout · up to 50 records".into(),
            rows: std::iter::once("Reservations coordinate participants; they do not lock files or prove that writes stopped. Renewal and release require the owning agent's valid attempt.".into())
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
                Action::Spaces => {
                    self.show_spaces = true;
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
                    if !self.connected || Self::current_context(ctx) != self.context {
                        return;
                    }
                    if let Some(workspace) = &self.workspace_preview {
                        ctx.dispatch_typed_action(
                            &crate::workspace::WorkspaceAction::OpenCollaborationWorkspace {
                                workspace_id: workspace.id.clone(),
                                space_id: workspace.space_id.clone(),
                                root: workspace.root.clone(),
                            },
                        );
                    }
                    return;
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
                    if Self::current_context(ctx) != self.context {
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
                        ctx.dispatch_typed_action(
                            &crate::workspace::WorkspaceAction::FocusTerminalViewInWorkspace {
                                terminal_view_id,
                            },
                        );
                    }
                    return;
                }
                Action::SelectTask(id) => self.query.selected_task = Some(id.clone()),
                Action::Back => {
                    self.query.selected_task = None;
                    self.show_spaces = false;
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
                    ctx.dispatch_typed_action(&crate::workspace::WorkspaceAction::FocusLeftPanel);
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
            Action::Exit => {
                ctx.dispatch_typed_action(&crate::workspace::WorkspaceAction::FocusLeftPanel)
            }
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
        let mut header = Flex::column().with_spacing(12.);
        header.add_child(
            builder
                .span("Agent collaboration")
                .with_soft_wrap()
                .build()
                .finish(),
        );
        header.add_child(
            builder
                .span(if self.preview {
                    format!("Design preview — sample data · {}", fixture.state)
                } else {
                    format!("Local collaboration · {}", fixture.state)
                })
                .with_soft_wrap()
                .build()
                .finish(),
        );
        header.add_child(
            builder
                .span(fixture.guidance.clone())
                .with_soft_wrap()
                .build()
                .finish(),
        );
        header.add_child(
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
        let mut body = Flex::column().with_spacing(12.);
        if !self.preview && self.form.is_some() {
            body.add_child(self.render_controls(app));
        }
        if !self.preview {
            if let Some(snapshot) = &self.snapshot {
                header.add_child(
                    builder
                        .button(ButtonVariant::Text, self.scope_buttons[0].clone())
                        .with_text_label("Spaces and workspaces".into())
                        .build()
                        .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Spaces))
                        .finish(),
                );
                if self.query.selected_task.is_some() || self.show_spaces {
                    header.add_child(
                        builder
                            .button(ButtonVariant::Text, self.page_buttons[0].clone())
                            .with_text_label("Back to agents and tasks".into())
                            .build()
                            .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Back))
                            .finish(),
                    );
                } else {
                    body.add_child(builder.span("Tasks").with_soft_wrap().build().finish());
                    if snapshot.tasks.is_empty() {
                        body.add_child(
                            builder
                                .span("No tasks on this page.")
                                .with_soft_wrap()
                                .build()
                                .finish(),
                        );
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
                            body.add_child(
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
                }
            }
        }
        for section in &fixture.sections {
            body.add_child(
                builder
                    .span(section.title.clone())
                    .with_soft_wrap()
                    .build()
                    .finish(),
            );
            for row in &section.rows {
                body.add_child(
                    Container::new(
                        builder
                            .span(row.clone())
                            .with_soft_wrap()
                            .with_selectable(true)
                            .build()
                            .finish(),
                    )
                    .with_padding_left(8.)
                    .finish(),
                );
            }
        }
        if !self.preview && self.show_spaces {
            if let Some(snapshot) = &self.snapshot {
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
                        body.add_child(
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
                for (index, label, action) in [
                    (4, "First reservation page", Some(Action::FirstReservations)),
                    (
                        5,
                        "Next reservation page",
                        (snapshot.reservations.len() == 50).then_some(Action::NextReservations),
                    ),
                ] {
                    if let Some(action) = action {
                        body.add_child(
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
            }
        }
        if !self.preview && self.form.is_none() && self.snapshot.is_some() {
            if let Some(snapshot) = &self.snapshot {
                for row in &snapshot.agents {
                    if row.online
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
                    .with_spacing(12.)
                    .with_child(header.finish())
                    .with_child(Shrinkable::new(1.0, scroll).finish())
                    .finish(),
            )
            .with_padding(Padding::uniform(12.))
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
    broker.input_guard(terminal, true, true);
    broker.end(terminal);
    Ok(())
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
                        panel.read(app, |panel, _| panel.selected == 8)
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
                    panel.read(app, |panel, _| panel
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.tasks.len() == 1
                            && snapshot.agents.iter().any(|agent| !agent.online)))
                        && checkpoint_draft(app, window) == "unsent collaboration draft"
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
            TestStep::new("new tab uses reviewed shared scope")
                .add_named_assertion(
                    "pending admission precedes native discovery",
                    move |app, window| {
                        let panel =
                            app.views_of_type::<CollaborationPanel>(window).unwrap()[0].clone();
                        warpui::async_assert!(
                            panel.read(app, |panel, _| panel.connected
                                && panel.snapshot.as_ref().is_some_and(|snapshot| snapshot
                                    .admission
                                    == "shared"
                                    && snapshot.project.starts_with("space:")
                                    && snapshot.agents.is_empty()))
                                && app.read(|ctx| crate::workspace::ActiveSession::as_ref(ctx)
                                    .path_if_local(window)
                                    .and_then(|path| warp_agent_bus::project_root(path).ok())
                                    .as_deref()
                                    == Some(shared_root.as_str()))
                        )
                    },
                )
                .with_take_screenshot("live-workspace-shared-tab.png"),
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
                    *prepared_client.lock().unwrap() = Some(
                        register_capture_participant(
                            "capture-shared-worker",
                            "shared-capture-worker",
                            &root,
                            Some(workspace),
                        )
                        .unwrap(),
                    );
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
