//! Native collaboration projection; explicit sample mode retains the accepted fixtures.
use crate::appearance::Appearance;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use warp_agent_bus::{transport::PanelQuery, Agent, Event, Task};
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
    page_buttons: [MouseStateHandle; 4],
    generation: u64,
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
                    panel.status = "Connected to the local coordinator. Execution and presence are separate.".into();
                    panel.snapshot = Some(snapshot);
                }
                Err(error) => {
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
                                if evidence.device.is_some() {
                                    " · remote metadata; content not fetched"
                                } else {
                                    ""
                                }
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
                Action::SelectTask(id) => self.query.selected_task = Some(id.clone()),
                Action::Back => self.query.selected_task = None,
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
        if !self.preview {
            if let Some(snapshot) = &self.snapshot {
                if self.query.selected_task.is_some() {
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
                            snapshot.task_cursor.map(|_| Action::NextTasks),
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

/// Captures only fixed fixtures in an isolated debug profile; never enables live operations.
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
