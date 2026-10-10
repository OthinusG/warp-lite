//! Native human intent bound to the original scope/version, using existing editors.
use super::{
    button_row, detail, heading, note, Action, CollaborationPanel, GAP_SECTION, GAP_TIGHT,
};
use crate::{
    appearance::Appearance,
    editor::{EditorView, Event, SingleLineEditorOptions, TextOptions},
};
use warp_agent_bus::{ControllerOperation, Operation, Task};
use warpui::{
    elements::{ChildView, Element, Flex, MouseStateHandle, ParentElement},
    ui_components::{button::ButtonVariant, components::UiComponent},
    AppContext, SingletonEntity, ViewContext, ViewHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Send,
    Assign,
    Pool,
    Search,
    Cancel,
    Accept,
    Revise,
    Retry,
    RetryOverride,
    Reassign,
    ReassignOverride,
    ForceCancel,
    Archive,
    ArchiveAged,
    Purge,
    RenewReservation,
    ReleaseReservation,
    CreateSpace,
    MapWorkspace,
    LeaveSpace,
    SelectCoordinator,
    JoinWorktree,
    LeaveWorktree,
}
impl Kind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Send => "Message",
            Self::Assign => "Assign",
            Self::Pool => "Create pool task",
            Self::Search => "Search",
            Self::Cancel => "Request cancellation",
            Self::Accept => "Accept result",
            Self::Revise => "Request revision",
            Self::Retry => "Retry",
            Self::RetryOverride => "Retry with override",
            Self::Reassign => "Reassign",
            Self::ReassignOverride => "Reassign with override",
            Self::ForceCancel => "Override cancellation",
            Self::Archive => "Archive",
            Self::ArchiveAged => "Archive older completed work",
            Self::Purge => "Purge reviewed history",
            Self::RenewReservation => "Renew reservation",
            Self::ReleaseReservation => "Release reservation",
            Self::CreateSpace => "Create space",
            Self::MapWorkspace => "Map checkout",
            Self::LeaveSpace => "Leave participation",
            Self::SelectCoordinator => "Select Coordinator",
            Self::JoinWorktree => "Join worktree team",
            Self::LeaveWorktree => "Leave worktree team",
        }
    }
    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::SelectCoordinator => &[],
            Self::Send => &["Agent", "Message", "Subject (optional)"],
            Self::Assign => &[
                "Agent", "Description", "Acceptance criteria", "Prerequisites (optional)",
                "Start by (e.g. 2026-10-03T09:00:00Z; optional)",
                "Execution timeout in seconds (optional)", "Review timeout in seconds (optional)",
                "Reviewer",
            ],
            Self::Pool => &[
                "Recipient or eligible names (comma-separated for pool)",
                "Description",
                "Acceptance criteria",
                "Prerequisite task IDs (comma-separated; optional)",
                "Start by (e.g. 2026-10-03T09:00:00Z; optional)",
                "Execution timeout in seconds (optional)",
                "Review timeout in seconds (optional)",
                "Reviewer name (optional; defaults to issuer)",
            ],
            Self::Reassign => &["New recipient agent name", "Reason"],
            Self::ReassignOverride => &["New recipient agent name", "Reason", "Type ALLOW OVERLAP"],
            Self::ForceCancel | Self::RetryOverride => &["Reason", "Type ALLOW OVERLAP"],
            Self::Accept | Self::Revise => &["Review feedback"],
            Self::Archive => &[],
            Self::JoinWorktree | Self::LeaveWorktree => &[],
            Self::ArchiveAged => &["Completed work older than days (1–3650)"],
            Self::Purge => &["Type DELETE HISTORY"],
            Self::RenewReservation => &[
                "Reservation UUID from current page",
                "TTL seconds (1–3600)",
                "Reason",
            ],
            Self::ReleaseReservation => &[
                "Reservation UUID from current page",
                "Reason",
                "Type RELEASE RESERVATION",
            ],
            Self::CreateSpace => &["Space name"],
            Self::MapWorkspace => &[
                "Space UUID",
                "Canonical checkout root",
                "Repository UUID (optional; explicit logical overlap group)",
            ],
            Self::LeaveSpace => &["Agent name to remove"],
            Self::Search => &["Literal search text (up to 256 bytes)"],
            _ => &["Reason"],
        }
    }
    fn overrides(self) -> bool {
        matches!(
            self,
            Self::ForceCancel | Self::RetryOverride | Self::ReassignOverride
        )
    }
}

pub(super) struct Form {
    pub(super) kind: Kind,
    pub(super) candidates: Vec<super::WorktreeCandidate>,
    pub(super) selected_candidate: Option<String>,
    candidate_buttons: std::collections::HashMap<String, MouseStateHandle>,
    project: String,
    pub(super) worktree_root: String,
    task: Option<Task>,
    purge_preview: Option<super::PurgePreview>,
    reservations: Vec<warp_agent_bus::Reservation>,
    request_id: String,
    fields: Vec<ViewHandle<EditorView>>,
    selectors: std::collections::HashMap<usize, ViewHandle<super::Dropdown<Action>>>,
    choices: std::collections::HashMap<usize, Vec<(String, String)>>,
    agent_icons: std::collections::HashMap<String, crate::ui_components::icons::Icon>,
    choices_loading: bool,
    pub(super) submitting: bool,
    error: String,
    pub(super) submitted_fields: Option<Vec<String>>,
    buttons: [MouseStateHandle; 2],
}

enum Command {
    Agent(Operation),
    Controller(ControllerOperation),
    Search(String),
    Reservation {
        id: String,
        ttl: Option<u64>,
        reason: String,
    },
}

fn command(
    kind: Kind,
    project: &str,
    task: Option<&Task>,
    fields: &[String],
    request_id: String,
) -> anyhow::Result<Command> {
    anyhow::ensure!(fields.len() == kind.labels().len(), "Invalid form");
    anyhow::ensure!(
        fields.iter().enumerate().all(|(index, field)| ((matches!(
            kind,
            Kind::MapWorkspace | Kind::Send
        ) && index == 2)
            || (matches!(kind, Kind::Assign | Kind::Pool) && index >= 3)
            || !field.trim().is_empty())
            && field.len() <= 8192
            && !field.chars().any(char::is_control)),
        "Complete every field using plain text (up to 8192 bytes)"
    );
    if kind == Kind::Send {
        return Ok(Command::Agent(Operation::AgentSend {
            to: fields[0].trim().into(),
            body: fields[1].clone(),
            subject: (!fields[2].trim().is_empty()).then(|| fields[2].clone()),
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id,
        }));
    }
    if kind.overrides() {
        anyhow::ensure!(
            fields.last().is_some_and(|field| field == "ALLOW OVERLAP"),
            "Type ALLOW OVERLAP to acknowledge that earlier execution may still be writing"
        );
    }
    if matches!(kind, Kind::RenewReservation | Kind::ReleaseReservation) {
        uuid::Uuid::parse_str(fields[0].trim())
            .map_err(|_| anyhow::anyhow!("Enter a reservation UUID from the current page"))?;
        let (ttl, reason) = if kind == Kind::RenewReservation {
            let seconds: u64 = fields[1]
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("TTL must be whole seconds between 1 and 3600"))?;
            anyhow::ensure!(
                (1..=3600).contains(&seconds),
                "TTL must be between 1 and 3600 seconds"
            );
            (Some(seconds), fields[2].clone())
        } else {
            anyhow::ensure!(
                fields[2] == "RELEASE RESERVATION",
                "Type RELEASE RESERVATION; this does not stop writes"
            );
            (None, fields[1].clone())
        };
        return Ok(Command::Reservation {
            id: fields[0].trim().into(),
            ttl,
            reason,
        });
    }
    if kind == Kind::ArchiveAged {
        let days: u64 = fields[0]
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Enter whole days between 1 and 3650"))?;
        anyhow::ensure!(
            (1..=3650).contains(&days),
            "Enter whole days between 1 and 3650"
        );
        return Ok(Command::Controller(ControllerOperation::ArchiveAged {
            older_than_days: Some(days),
            request_id,
        }));
    }
    if kind == Kind::Purge {
        anyhow::ensure!(
            fields[0] == "DELETE HISTORY",
            "Type DELETE HISTORY to confirm irreversible deletion"
        );
        return Ok(Command::Controller(ControllerOperation::HistoryPurge {
            expected_sequence: None,
            archived_tasks: true,
            acknowledged_messages: true,
            request_id,
        }));
    }
    if kind == Kind::Search {
        anyhow::ensure!(fields[0].len() <= 256, "Search text must fit 256 bytes");
        return Ok(Command::Search(fields[0].trim().to_owned()));
    }
    if matches!(kind, Kind::Assign | Kind::Pool) {
        let names = |value: &str| {
            value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let seconds = |value: &str| -> anyhow::Result<Option<u64>> {
            if value.trim().is_empty() {
                return Ok(None);
            }
            let seconds: u64 = value
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("Timeout must be a whole number of seconds"))?;
            anyhow::ensure!(seconds > 0, "Timeout must be positive");
            Ok(Some(seconds))
        };
        let dependencies = names(&fields[3]);
        let start_deadline = (!fields[4].trim().is_empty()).then(|| fields[4].trim().to_owned());
        let execution_timeout_seconds = seconds(&fields[5])?;
        let review_timeout_seconds = seconds(&fields[6])?;
        let reviewer = (!fields[7].trim().is_empty()).then(|| fields[7].trim().to_owned());
        return Ok(Command::Agent(if kind == Kind::Pool {
            Operation::TaskCreatePool {
                eligible: names(&fields[0]),
                description: fields[1].clone(),
                acceptance: fields[2].clone(),
                reviewer,
                request_id,
                dependencies,
                start_deadline,
                execution_timeout_seconds,
                review_timeout_seconds,
            }
        } else {
            Operation::TaskAssign {
                to: fields[0].trim().to_owned(),
                description: fields[1].clone(),
                acceptance: fields[2].clone(),
                reviewer,
                request_id,
                dependencies,
                start_deadline,
                execution_timeout_seconds,
                review_timeout_seconds,
            }
        }));
    }
    match kind {
        Kind::CreateSpace => {
            return Ok(Command::Controller(ControllerOperation::SpaceCreate {
                name: fields[0].clone(),
                request_id,
            }))
        }
        Kind::MapWorkspace => {
            return Ok(Command::Controller(ControllerOperation::WorkspaceMap {
                space_id: fields[0].clone(),
                root: fields[1].clone(),
                repository_id: (!fields[2].trim().is_empty()).then(|| fields[2].clone()),
                model: "independent_worktrees".into(),
                branch: None,
                base_commit: None,
                request_id,
            }))
        }
        Kind::LeaveSpace => {
            let space_id = project.strip_prefix("space:").ok_or_else(|| {
                anyhow::anyhow!("Open a shared pane before leaving participation")
            })?;
            return Ok(Command::Controller(ControllerOperation::SpaceLeave {
                space_id: space_id.into(),
                agent: fields[0].clone(),
                request_id,
            }));
        }
        _ => {}
    }
    let task = task.ok_or_else(|| anyhow::anyhow!("Select a task first"))?;
    let task_id = task.id.clone();
    let expected_version = Some(task.version);
    Ok(match kind {
        Kind::Cancel => Command::Agent(Operation::TaskCancel {
            task_id,
            reason: fields[0].clone(),
            expected_version,
            request_id,
        }),
        Kind::Accept | Kind::Revise => Command::Agent(Operation::TaskReview {
            task_id,
            revision: task.revision,
            accepted: kind == Kind::Accept,
            feedback: fields[0].clone(),
            expected_version,
            request_id,
        }),
        Kind::Retry | Kind::RetryOverride => Command::Agent(Operation::TaskRetry {
            task_id,
            reason: fields[0].clone(),
            start_deadline: None,
            clear_start_deadline: false,
            override_uncertain: kind.overrides(),
            expected_version,
            request_id,
        }),
        Kind::Reassign | Kind::ReassignOverride => Command::Agent(Operation::TaskReassign {
            task_id,
            assignee: fields[0].clone(),
            reason: fields[1].clone(),
            reviewer: None,
            start_deadline: None,
            clear_start_deadline: false,
            override_uncertain: kind.overrides(),
            expected_version,
            request_id,
        }),
        Kind::ForceCancel => Command::Controller(ControllerOperation::TaskForceCancel {
            task_id,
            reason: fields[0].clone(),
            expected_version,
            request_id,
        }),
        Kind::Archive => Command::Controller(ControllerOperation::TaskArchive {
            task_id,
            request_id,
        }),
        Kind::Send
        | Kind::RenewReservation
        | Kind::ReleaseReservation
        | Kind::ArchiveAged
        | Kind::Purge
        | Kind::Assign
        | Kind::Pool
        | Kind::Search
        | Kind::CreateSpace
        | Kind::MapWorkspace
        | Kind::LeaveSpace
        | Kind::SelectCoordinator
        | Kind::JoinWorktree
        | Kind::LeaveWorktree => {
            unreachable!()
        }
    })
}

fn toggle_prerequisite(text: &str, value: &str) -> String {
    if value.is_empty() { return String::new(); }
    let mut selected: Vec<_> = text.split(',').filter(|id| !id.is_empty()).map(str::to_owned).collect();
    if selected.iter().any(|id| id == value) { selected.retain(|id| id != value); }
    else { selected.push(value.to_owned()); }
    selected.join(",")
}

impl CollaborationPanel {
    pub(super) fn select_form_field(&mut self, request: &str, index: usize, value: &str, ctx: &mut ViewContext<Self>) {
        let Some(form) = self.form.as_ref().filter(|form| form.request_id == request && !form.submitting && form.submitted_fields.is_none()) else { return; };
        let Some(choices) = form.choices.get(&index) else { return; };
        if !choices.iter().any(|(_, choice)| choice == value) { return; }
        let value = if index == 3 && matches!(form.kind, Kind::Assign | Kind::Pool) && !value.is_empty() {
            let text = form.fields[index].as_ref(ctx).buffer_text(ctx);
            toggle_prerequisite(&text, value)
        } else { value.to_owned() };
        form.fields[index].update(ctx, |editor, ctx| editor.set_buffer_text(&value, ctx));
        ctx.notify();
    }

    pub(super) fn sync_coordinator_dropdown(&self, snapshot: &super::Snapshot, ctx: &mut ViewContext<Self>) {
        let selected = snapshot.roles.iter().find(|role| role.role == "coordinator" && role.run.is_some()).map(|role| &role.agent);
        self.coordinator_dropdown.update(ctx, |dropdown, ctx| {
            if snapshot.candidates.is_empty() || !self.connected || self.form.is_some() { dropdown.set_disabled(ctx); }
            else { dropdown.set_enabled(ctx); }
            if self.form.is_none() {
                let index = selected.and_then(|id| snapshot.candidates.iter().position(|candidate| &candidate.agent.id == id)).map_or(0, |index| index + 1);
                dropdown.set_selected_by_index(index, ctx);
            }
        });
        if self.snapshot.as_ref().is_some_and(|previous| previous.project == snapshot.project
            && previous.candidates.iter().map(|candidate| (&candidate.agent.id, &candidate.run, &candidate.root, &candidate.agent.name, &candidate.branch))
                .eq(snapshot.candidates.iter().map(|candidate| (&candidate.agent.id, &candidate.run, &candidate.root, &candidate.agent.name, &candidate.branch)))
            && previous.roles.iter().find(|role| role.role == "coordinator" && role.run.is_some()).map(|role| &role.agent) == selected) {
            return;
        }
        self.coordinator_dropdown.update(ctx, |dropdown, ctx| {
            let mut items = vec![super::DropdownItem::new(warpui::localization::text("Select Coordinator"), Action::ChooseCoordinator {
                project: snapshot.project.clone(), agent: String::new(), run: String::new(),
            })];
            let mut selected_index = 0;
            for candidate in &snapshot.candidates {
                if selected == Some(&candidate.agent.id) { selected_index = items.len(); }
                items.push(super::DropdownItem::new(format!("{} · {} · {}", candidate.agent.name,
                    candidate.branch.as_deref().unwrap_or("detached"), candidate.agent.id.chars().take(8).collect::<String>()),
                    Action::ChooseCoordinator { project: snapshot.project.clone(), agent: candidate.agent.id.clone(), run: candidate.run.clone() })
                    .with_icon(crate::agent_usage::agent_icon(&candidate.agent.program))
                    .with_tooltip(format!("{}\n{}\n{}\n{}", candidate.agent.name,
                        candidate.branch.as_deref().unwrap_or("detached"), candidate.agent.id, candidate.root)));
            }
            dropdown.set_items(items, ctx);
            dropdown.set_selected_by_index(selected_index, ctx);
        });
    }

    pub(super) fn render_worktree_team(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let mut body = Flex::column().with_spacing(GAP_SECTION);
        let Some(snapshot) = &self.snapshot else { return body.finish(); };
        if !snapshot.worktree_available {
            body.add_child(note(appearance, warpui::localization::text("Open a Git repository to use Worktree mode.")));
            return body.finish();
        }
        let coordinator = snapshot.roles.iter().find(|role| role.role == "coordinator" && role.run.is_some());
        body.add_child(heading(appearance, warpui::localization::text("Coordinator")));
        body.add_child(ChildView::new(&self.coordinator_dropdown).finish());
        if self.remote.is_none() {
            body.add_child(builder.button(ButtonVariant::Secondary, self.team_buttons[1].clone())
                .with_text_label(warpui::localization::text("New worktree").into()).build().on_click(|ctx, _, _|
                    ctx.dispatch_typed_action(crate::workspace::WorkspaceAction::OpenNewWorktreeModal)).finish());
        }
        if snapshot.candidates.is_empty() {
            body.add_child(note(appearance, warpui::localization::text("Start an Agent in this project.")));
        }
        body.add_child(heading(appearance, warpui::localization::text("Worktrees")));
        for checkout in &snapshot.worktrees {
            let branch = checkout.branch.as_deref().unwrap_or("detached HEAD");
            let mut summary: String = branch.chars().take(48).collect();
            if branch.chars().count() > 48 { summary.push('…'); }
            let root = checkout.root.clone();
            let expanded = self.expanded_worker.as_ref() == Some(&root);
            if let Some(state) = self.checkout_buttons.get(&root) {
                body.add_child(builder.button(ButtonVariant::Secondary, state.clone())
                    .with_text_label(format!("{} {summary}", if expanded { "▾" } else { "▸" }))
                    .build().on_click(move |ctx, _, _|
                        ctx.dispatch_typed_action(Action::ExpandWorker(root.clone()))).finish());
            }
            let agents: Vec<_> = snapshot.candidates.iter().filter(|candidate| candidate.root == checkout.root).collect();
            if agents.is_empty() { body.add_child(note(appearance, warpui::localization::text("No online Agents"))); }
            for candidate in agents {
                let selected = coordinator.is_some_and(|role| role.agent == candidate.agent.id);
                body.add_child(Flex::row().with_spacing(super::GAP_ROW)
                    .with_child(warpui::elements::ConstrainedBox::new(crate::agent_usage::agent_icon(&candidate.agent.program).to_warpui_icon(appearance.theme().foreground()).finish()).with_width(16.).with_height(16.).finish())
                    .with_child(super::Shrinkable::new(1., detail(appearance, format!("{}{}", candidate.agent.name,
                        if selected { " · Coordinator" } else { "" }))).finish()).finish());
                if expanded {
                    for task in snapshot.tasks.iter().filter(|task| task.assignee == candidate.agent.id) {
                        body.add_child(note(appearance, format!("{} · {}", task.state, task.description)));
                    }
                }
            }
            if expanded {
                body.add_child(detail(appearance, format!("{branch}\n{}", checkout.root)));
                if self.remote.is_none() && snapshot.worktrees.first().is_some_and(|main| main.root != checkout.root) {
                    let repository = std::path::PathBuf::from(&snapshot.worktree_root);
                    let root = std::path::PathBuf::from(&checkout.root);
                    body.add_child(builder.button(ButtonVariant::Secondary, self.team_buttons[0].clone())
                        .with_text_label(warpui::localization::text("Remove worktree").into()).build().on_click(move |ctx, _, _|
                            ctx.dispatch_typed_action(crate::workspace::WorkspaceAction::RemoveLocalWorktree {
                                repository: repository.clone(), checkout: root.clone(),
                            })).finish());
                }
            }
        }
        body.finish()
    }

    pub(super) fn open_control(&mut self, kind: Kind, ctx: &mut ViewContext<Self>) {
        if !self.connected || self.preview || self.form.is_some() {
            return;
        }
        let snapshot = self.snapshot.as_ref();
        if snapshot.is_none() {
            return;
        }
        if self.remote.is_some()
            && matches!(
                kind,
                Kind::CreateSpace
                    | Kind::MapWorkspace
                    | Kind::LeaveSpace
                    | Kind::RenewReservation
                    | Kind::ReleaseReservation
            )
        {
            return;
        }
        if !matches!(
            kind,
            Kind::Send
                | Kind::RenewReservation
                | Kind::ReleaseReservation
                | Kind::ArchiveAged
                | Kind::Purge
                | Kind::Assign
                | Kind::Pool
                | Kind::Search
                | Kind::CreateSpace
                | Kind::MapWorkspace
                | Kind::LeaveSpace
                | Kind::SelectCoordinator
                | Kind::JoinWorktree
                | Kind::LeaveWorktree
        ) && snapshot
            .and_then(|snapshot| snapshot.task.as_ref())
            .is_none_or(|task| self.query.selected_task.as_ref() != Some(&task.id))
        {
            return;
        }
        if kind == Kind::Purge
            && (!self.query.history || snapshot.is_none_or(|snapshot| snapshot.history.is_none()))
        {
            return;
        }
        let purge_preview = snapshot
            .and_then(|snapshot| snapshot.history.as_ref())
            .map(|history| history.preview.clone());
        let reservations = if matches!(kind, Kind::RenewReservation | Kind::ReleaseReservation) {
            snapshot
                .map(|snapshot| snapshot.reservations.clone())
                .unwrap_or_default()
        } else {
            vec![]
        };
        let project = snapshot
            .map(|snapshot| snapshot.project.clone())
            .unwrap_or_default();
        let task = if kind == Kind::Send {
            None
        } else {
            snapshot.and_then(|snapshot| snapshot.task.clone())
        };
        let fields: Vec<_> = kind
            .labels()
            .iter()
            .enumerate()
            .map(|(index, label)| {
                let label = (*label).to_owned();
                let editor = ctx.add_typed_action_view(|ctx| {
                    let mut editor = EditorView::single_line(
                        SingleLineEditorOptions {
                            text: TextOptions {
                                font_family_override: Some(
                                    Appearance::as_ref(ctx).ui_font_family(),
                                ),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ctx,
                    );
                    editor.set_placeholder_text(&label, ctx);
                    editor
                });
                ctx.subscribe_to_view(&editor, move |panel, _, event, ctx| match event {
                    Event::Edited(_) => ctx.notify(),
                    Event::Escape => {
                        panel.cancel_control(ctx);
                    }
                    Event::Navigate(warp_editor::editor::NavigationKey::Tab) => {
                        if let Some(form) = &panel.form {
                            ctx.focus(&form.fields[(index + 1) % form.fields.len()]);
                        }
                    }
                    Event::Navigate(warp_editor::editor::NavigationKey::ShiftTab) => {
                        if let Some(form) = &panel.form {
                            ctx.focus(
                                &form.fields[(index + form.fields.len() - 1) % form.fields.len()],
                            );
                        }
                    }
                    _ => {}
                });
                editor
            })
            .collect();
        if let Some(first) = fields.first() {
            ctx.focus(first);
        }
        self.form = Some(Form {
            kind,
            candidates: snapshot.map(|snapshot| snapshot.candidates.iter().filter(|candidate| match kind {
                Kind::SelectCoordinator => true,
                _ => false,
            }).cloned().collect()).unwrap_or_default(),
            selected_candidate: None,
            candidate_buttons: snapshot.map(|snapshot| snapshot.candidates.iter().map(|candidate| (candidate.agent.id.clone(), MouseStateHandle::default())).collect()).unwrap_or_default(),
            project,
            worktree_root: snapshot.map(|snapshot| snapshot.worktree_root.clone()).unwrap_or_default(),
            task,
            purge_preview,
            reservations,
            fields,
            selectors: Default::default(),
            choices: Default::default(),
            agent_icons: snapshot.map(|s| s.agents.iter().map(|row| &row.agent).chain(s.candidates.iter().map(|candidate| &candidate.agent))
                .map(|agent| (agent.id.clone(), crate::agent_usage::agent_icon(&agent.program))).collect()).unwrap_or_default(),
            choices_loading: false,
            request_id: uuid::Uuid::new_v4().to_string(),
            submitting: false,
            error: String::new(),
            submitted_fields: None,
            buttons: Default::default(),
        });
        self.build_form_selectors(ctx);
        self.load_remaining_choices(ctx);
        if self.worktree_mode { self.coordinator_dropdown.update(ctx, |dropdown, ctx| dropdown.set_disabled(ctx)); }
        self.scroll = Default::default();
        ctx.notify();
    }

    fn build_form_selectors(&mut self, ctx: &mut ViewContext<Self>) {
        let (Some(form), Some(snapshot)) = (&mut self.form, &self.snapshot) else { return; };
        let peers: Vec<_> = if self.worktree_mode {
            snapshot.candidates.iter().map(|candidate| (candidate.agent.id.clone(), format!("{} · {}", candidate.agent.name, candidate.branch.as_deref().unwrap_or("detached")))).collect()
        } else {
            snapshot.agents.iter().filter(|row| row.online).map(|row| (row.agent.id.clone(), row.agent.name.clone())).collect()
        };
        let indexes: &[usize] = match form.kind {
            Kind::Send | Kind::Reassign | Kind::ReassignOverride => &[0],
            Kind::Assign => &[0, 3, 7],
            _ => &[],
        };
        for &index in indexes {
            let mut choices = vec![(if index == 7 { "You (default reviewer)" } else if index == 3 { "No prerequisites" } else { "Select Agent" }.to_owned(), String::new())];
            if index == 3 {
                choices.extend(snapshot.tasks.iter().map(|task| (format!("{} · {}", task.description, task.state), task.id.clone())));
            } else {
                choices.extend(peers.iter().map(|(id, label)| (format!("{label} · {}", id.chars().take(8).collect::<String>()), id.clone())));
            }
            let request = form.request_id.clone();
            let dropdown = ctx.add_typed_action_view(|ctx| {
                let mut dropdown = super::Dropdown::new(ctx);
                dropdown.set_items(choices.iter().map(|(label, value)| {
                    let item = super::DropdownItem::new(label.clone(), Action::SelectFormField { request: request.clone(), index, value: value.clone() });
                    if index != 3 && !value.is_empty() { item.with_icon(form.agent_icons.get(value).copied().unwrap_or(crate::ui_components::icons::Icon::Terminal)) } else { item }
                }).collect(), ctx);
                dropdown.set_selected_by_index(0, ctx);
                dropdown
            });
            form.selectors.insert(index, dropdown);
            form.choices.insert(index, choices);
        }
    }

    fn load_remaining_choices(&mut self, ctx: &mut ViewContext<Self>) {
        let (Some(form), Some(snapshot)) = (&mut self.form, &self.snapshot) else { return; };
        if form.selectors.is_empty() || (snapshot.agent_cursor.is_none() && snapshot.task_cursor.is_none()) { return; }
        form.choices_loading = true;
        let request = form.request_id.clone();
        let response_request = request.clone();
        let project = snapshot.project.clone();
        let mut query = warp_agent_bus::transport::PanelQuery {
            project: self.context.as_ref().map(|context| context.0.clone()).unwrap_or_default(),
            scope: Some(project.clone()), terminal: self.context.as_ref().and_then(|context| context.1.clone()),
            worktree: self.worktree_mode, agent_after: snapshot.agent_cursor.clone(), task_after: snapshot.task_cursor,
            ..Default::default()
        };
        let remote = self.remote.is_some();
        let client = self.remote_client.clone();
        let broker = super::super::BROKER.get().cloned();
        let generation = self.generation;
        ctx.spawn(async move {
            tokio::time::timeout(std::time::Duration::from_secs(30), async move {
                let mut peers = Vec::new();
                let mut tasks = Vec::new();
                let mut icons = Vec::new();
                loop {
                    let previous = (query.agent_after.clone(), query.task_after);
                    let mut value = if remote {
                        query.project.clear(); query.terminal = None;
                        let mut guard = client.lock().await;
                        let client = guard.as_mut().ok_or_else(|| anyhow::anyhow!("SSH connection unavailable"))?;
                        Self::remote_command(client, &warp_agent_bus::companion::TaskCommand::Panel(query.clone()), generation).await?
                    } else {
                        query.project = warp_agent_bus::project_root(std::path::Path::new(&query.project))?;
                        broker.as_ref().ok_or_else(|| anyhow::anyhow!("Broker unavailable"))?.operator_panel(&query)?
                    };
                    if let Some(scope) = value.get("collaboration_scope").cloned() { value["project"] = scope; }
                    let page: super::Snapshot = serde_json::from_value(value)?;
                    anyhow::ensure!(page.project == project, "Selection scope changed");
                    icons.extend(page.agents.iter().filter(|row| row.online).map(|row| (row.agent.id.clone(), crate::agent_usage::agent_icon(&row.agent.program))));
                    peers.extend(page.agents.into_iter().filter(|row| row.online).map(|row| (row.agent.id, row.agent.name)));
                    tasks.extend(page.tasks.into_iter().map(|task| (task.id, format!("{} · {}", task.description, task.state))));
                    let complete = page.agent_cursor.is_none() && page.task_cursor.is_none();
                    query.agent_after = page.agent_cursor.or(query.agent_after);
                    query.task_after = page.task_cursor.or(query.task_after);
                    if complete { break; }
                    anyhow::ensure!(previous != (query.agent_after.clone(), query.task_after), "Selection cursor did not advance");
                }
                Ok::<_, anyhow::Error>((peers, tasks, icons))
            }).await.map_err(|_| anyhow::anyhow!("Selection loading timed out"))?
        }, move |panel, result, ctx| {
            let Some(form) = panel.form.as_mut().filter(|form| form.request_id == response_request) else { return; };
            form.choices_loading = false;
            match result {
                Ok((peers, tasks, icons)) => {
                    form.agent_icons.extend(icons);
                    for (&index, choices) in &mut form.choices {
                        for (value, label) in if index == 3 { &tasks } else { &peers } {
                            if !choices.iter().any(|(_, id)| id == value) { choices.push((label.clone(), value.clone())); }
                        }
                        let request = form.request_id.clone();
                        let selected = form.fields[index].as_ref(ctx).buffer_text(ctx);
                        form.selectors[&index].update(ctx, |dropdown, ctx| {
                            dropdown.set_items(choices.iter().map(|(label, value)| {
                                let item = super::DropdownItem::new(label.clone(), Action::SelectFormField { request: request.clone(), index, value: value.clone() });
                                if index != 3 && !value.is_empty() { item.with_icon(form.agent_icons.get(value).copied().unwrap_or(crate::ui_components::icons::Icon::Terminal)) } else { item }
                            }).collect(), ctx);
                            dropdown.set_selected_by_index(choices.iter().position(|(_, value)| value == &selected).unwrap_or(0), ctx);
                        });
                    }
                }
                Err(_) => {
                    form.choices_loading = true;
                    form.error = "Could not load all choices. Close this form, reconnect or refresh, then retry.".into();
                }
            }
            ctx.notify();
        });
    }

    pub(super) fn cancel_control(&mut self, ctx: &mut ViewContext<Self>) {
        if self.form.as_ref().is_some_and(|form| form.submitting) {
            return;
        }
        self.form = None;
        if let Some(snapshot) = &self.snapshot { self.sync_coordinator_dropdown(snapshot, ctx); }
        ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::FocusLeftPanel);
        ctx.notify();
    }

    pub(super) fn confirm_control(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.connected
            || self.current_context(ctx) != self.context
            || !crate::agent_communication::AgentCommunication::as_ref(ctx)
                .preferences
                .enabled
        {
            return;
        }
        let Some(form) = &mut self.form else {
            return;
        };
        if form.submitting {
            return;
        }
        if form.choices_loading { return; }
        if self
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.project != form.project)
        {
            form.error =
                "Scope changed. Close this form and confirm a new operation in the current scope."
                    .into();
            ctx.notify();
            return;
        }
        let fields = form
            .fields
            .iter()
            .map(|field| field.as_ref(ctx).buffer_text(ctx))
            .collect::<Vec<_>>();
        if form
            .submitted_fields
            .as_ref()
            .is_some_and(|original| original != &fields)
        {
            form.error = "Intent changed. Close this form and confirm a new request; the original outcome must not be replayed with different content.".into();
            ctx.notify();
            return;
        }
        let mut operation = if matches!(form.kind, Kind::SelectCoordinator) {
            let Some(candidate) = form.candidates.iter().find(|candidate| form.selected_candidate.as_ref() == Some(&candidate.agent.id)) else {
                form.error = "Select an active Agent.".into(); ctx.notify(); return;
            };
            let root = candidate.root.clone(); let agent = candidate.agent.id.clone(); let run = candidate.run.clone(); let request_id = form.request_id.clone();
            Command::Controller(ControllerOperation::WorktreeCoordinator { root, agent, run, request_id })
        } else if matches!(form.kind, Kind::JoinWorktree | Kind::LeaveWorktree) {
            let root = form.worktree_root.clone();
            let request_id = form.request_id.clone();
            Command::Controller(if form.kind == Kind::JoinWorktree {
                ControllerOperation::WorktreeJoin { root, request_id }
            } else { ControllerOperation::WorktreeLeave { root, request_id } })
        } else { match command(
            form.kind,
            &form.project,
            form.task.as_ref(),
            &fields,
            form.request_id.clone(),
        ) {
            Ok(operation) => operation,
            Err(error) => {
                form.error = error.to_string();
                ctx.notify();
                return;
            }
        } };
        if let Command::Reservation { id, ttl, reason } = &operation {
            let Some(reservation) = form
                .reservations
                .iter()
                .find(|reservation| reservation.id == *id)
            else {
                form.error = "Reservation was not on the original page. Close, refresh the page and confirm a new intent.".into();
                ctx.notify();
                return;
            };
            operation = Command::Controller(ControllerOperation::ReservationUpdate {
                reservation_id: id.clone(),
                workspace: reservation.workspace.clone(),
                expected_owner: reservation.owner.clone(),
                expected_expires_at: reservation.expires_at,
                ttl_seconds: *ttl,
                reason: reason.clone(),
                request_id: form.request_id.clone(),
            });
        }
        if let Command::Controller(ControllerOperation::HistoryPurge {
            expected_sequence, ..
        }) = &mut operation
        {
            let Some(preview) = &form.purge_preview else {
                return;
            };
            *expected_sequence = Some(preview.sequence);
        }
        if let Command::Search(query) = operation {
            self.query.history = false;
            self.query.message_query = Some(query);
            self.query.selected_thread = None;
            self.query.message_after = None;
            self.query.selected_task = None;
            self.query.wait = false;
            self.show_messages = true;
            self.show_spaces = false;
            self.query.spaces = false;
            self.form = None;
            self.generation += 1;
            self.refresh(ctx);
            ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::FocusLeftPanel);
            ctx.notify();
            return;
        }
        let broker = crate::agent_communication::BROKER.get().cloned();
        if self.remote.is_none() && broker.is_none() {
            return;
        }
        let remote = self.remote.is_some();
        let client = self.remote_client.clone();
        let response_client = client.clone();
        let generation = self.generation;
        let project = form.project.clone();
        let request_id = form.request_id.clone();
        form.submitting = true;
        form.submitted_fields = Some(fields);
        form.error.clear();
        ctx.spawn(async move {
            if remote {
                let command = match operation {
                    Command::Agent(operation) => warp_agent_bus::companion::TaskCommand::Operator(operation),
                    Command::Controller(operation) => warp_agent_bus::companion::TaskCommand::Controller(operation),
                    _ => unreachable!("Form intent resolved before dispatch"),
                };
                let mut client = client.lock().await;
                let client = client.as_mut().ok_or_else(|| anyhow::anyhow!("ssh_connection_lost"))?;
                let command = if client.capabilities().iter().any(|capability| capability == "worktree_collaboration") {
                    warp_agent_bus::companion::TaskCommand::Scoped { scope: project, command: Box::new(command) }
                } else { command };
                Self::remote_command(client, &command, generation).await
            } else {
                let broker = broker.unwrap();
                match operation { Command::Agent(operation) => broker.operator(&project, &operation), Command::Controller(operation) => broker.control(&project, &operation), _ => unreachable!("Form intent resolved before dispatch") }
            }
        }, move |panel, result, ctx| {
            if panel.form.as_ref().is_none_or(|form| form.request_id != request_id) { return; }
            match result {
                Ok(_) => {
                    panel.form = None;
                    panel.generation += 1;
                    panel.query.wait = false;
                    panel.refresh(ctx);
                }
                Err(error) => {
                    if remote && std::sync::Arc::ptr_eq(&panel.remote_client, &response_client) && error.downcast_ref::<warp_agent_bus::DomainError>().is_none() {
                        panel.connected = false;
                        panel.remote_failed = true;
                        panel.status = "SSH connection lost. Last state is stale; reconnect before writing. The original request is retained.".into();
                    }
                    let form = panel.form.as_mut().unwrap();
                    form.submitting = false;
                    let code = error.downcast_ref::<warp_agent_bus::DomainError>().map(|error| error.code.as_str()).unwrap_or("coordinator_unavailable");
                    form.error = if remote && error.downcast_ref::<warp_agent_bus::DomainError>().is_none() {
                        "Outcome unknown. Original content, version and request are retained. Reconnect, then explicitly retry this unchanged intent to reconcile its receipt.".into()
                    } else {
                        warpui::localization::format_text("Operation rejected ({code}). Original version and request were preserved. Resolve the blocker or close this form, refresh and confirm a new intent.", &[("code", format!("{code}").as_str())])
                    };
                }
            }
            ctx.notify();
        });
        ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::FocusLeftPanel);
        ctx.notify();
    }

    pub(super) fn render_controls(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let mut body = Flex::column().with_spacing(GAP_SECTION);
        if let Some(form) = &self.form {
            if form.kind == Kind::SelectCoordinator {
                if !form.error.is_empty() { body.add_child(detail(appearance, form.error.clone())); }
                if form.submitting { body.add_child(note(appearance, warpui::localization::text("Updating Coordinator…"))); }
                else if !form.error.is_empty() {
                    body.add_child(builder.button(ButtonVariant::Secondary, form.buttons[0].clone())
                        .with_text_label(warpui::localization::text("Retry").into()).build().on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::ConfirmControl)).finish());
                    body.add_child(builder.button(ButtonVariant::Text, form.buttons[1].clone())
                        .with_text_label(warpui::localization::text("Dismiss").into()).build().on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::CancelControl)).finish());
                }
                return body.finish();
            }
            body.add_child(heading(appearance, form.kind.label()));
            if let Some(task) = &form.task {
                body.add_child(note(
                    appearance,
                    format!(
                        "Task {} · original revision {} · version {}",
                        task.id, task.revision, task.version
                    ),
                ));
            }
            if form.kind.overrides() {
                body.add_child(builder.span(warpui::localization::text("Earlier execution may still be writing. This operation changes coordination ownership; it does not stop a process or file writes.")).with_soft_wrap().build().finish());
            }
            if matches!(form.kind, Kind::RenewReservation | Kind::ReleaseReservation) {
                body.add_child(builder.span(warpui::localization::text("Use an ID from the original reservation page. Renewal requires a confirmed active owner; release removes advisory coordination only and does not stop execution or file writes.")).with_soft_wrap().build().finish());
                let id = form.fields[0].as_ref(app).buffer_text(app);
                if let Some(reservation) = form
                    .reservations
                    .iter()
                    .find(|reservation| reservation.id == id.trim())
                {
                    body.add_child(note(
                        appearance,
                        format!(
                            "Original checkout {} · path {} · owner {} · expiry {} · attempt {}",
                            reservation.workspace,
                            reservation.path,
                            reservation.owner,
                            reservation.expires_at,
                            reservation.attempt_id.as_deref().unwrap_or("unlinked")
                        ),
                    ));
                }
            }
            if form.kind == Kind::Purge {
                if let Some(preview) = &form.purge_preview {
                    body.add_child(builder.span({
                        let __warpai_locale_argument_0 = &(preview.tasks);
                        let __warpai_locale_argument_1 = &(preview.messages);
                        let __warpai_locale_argument_2 = &(preview.sequence);
                        warpui::localization::format_text("Delete up to {} archived tasks and {} acknowledged messages from original preview sequence {}. This is irreversible. History changes require a new preview and intent.", &[("0", format!("{__warpai_locale_argument_0}").as_str()), ("1", format!("{__warpai_locale_argument_1}").as_str()), ("2", format!("{__warpai_locale_argument_2}").as_str())])
                    }).with_soft_wrap().build().finish());
                }
            }
            if form.kind == Kind::Archive {
                body.add_child(note(
                    appearance,
                    warpui::localization::text("Archive this terminal task while retaining its evidence and history."),
                ));
            }
            if form.kind == Kind::MapWorkspace {
                body.add_child(builder.span(warpui::localization::text("This mapping applies only to new explicitly joined panes. Changing a checkout's space revokes its earlier shared admissions. Existing private work remains private; matching Git remotes never joins projects.")).with_soft_wrap().build().finish());
            }
            if form.kind == Kind::LeaveSpace {
                body.add_child(builder.span(warpui::localization::text("Revoke the selected agent's shared coordination access and wake eligibility. Its task attempts remain in this space and may still be executing; this does not stop the CLI process.")).with_soft_wrap().build().finish());
            }
            if matches!(form.kind, Kind::JoinWorktree | Kind::LeaveWorktree) {
                body.add_child(detail(appearance, form.worktree_root.clone()));
                body.add_child(note(appearance, if form.kind == Kind::JoinWorktree {
                    "Fresh Agent runs in this checkout will share messages and tasks with other explicitly joined worktrees of this repository. Existing private runs and history keep their scope. Start an Agent in a fresh pane after joining."
                } else {
                    "Revoke this checkout's shared communication and queued delivery. Existing processes may still be writing. Start a private Agent in a fresh pane after leaving; task acceptance does not merge branches."
                }));
            }
            if matches!(form.kind, Kind::SelectCoordinator) {
                if form.candidates.is_empty() {
                    body.add_child(note(appearance, warpui::localization::text("No eligible active Agents. Start an Agent in the intended checkout using Agent management, then reopen this selector.")));
                }
                for candidate in &form.candidates {
                    let selected = form.selected_candidate.as_ref() == Some(&candidate.agent.id);
                    let id = candidate.agent.id.clone();
                    let button = builder.button(if selected { ButtonVariant::Accent } else { ButtonVariant::Secondary }, form.candidate_buttons[&id].clone())
                        .with_custom_label(builder.span(format!("{} · {}", candidate.agent.name,
                            candidate.branch.as_deref().unwrap_or("detached"))).with_soft_wrap().build().finish());
                    let button = if form.submitting || form.submitted_fields.is_some() { button.disabled() } else { button };
                    body.add_child(button.build().on_click(move |ctx, _, _| ctx.dispatch_typed_action(Action::SelectParticipant(id.clone()))).finish());
                    if selected { body.add_child(note(appearance, candidate.root.clone())); }
                }

            }
            for (index, (label, field)) in form.kind.labels().iter().zip(&form.fields).enumerate() {
                let value = if form.submitting || form.submitted_fields.is_some() {
                    let text = field.as_ref(app).buffer_text(app);
                    detail(appearance, form.choices.get(&index).map(|choices| text.split(',').filter_map(|id|
                        choices.iter().find(|(_, value)| value == id).map(|(label, _)| label.clone()))
                        .collect::<Vec<_>>().join(", ")).unwrap_or(text))
                } else if let Some(selector) = form.selectors.get(&index) {
                    ChildView::new(selector).finish()
                } else {
                    ChildView::new(field).finish()
                };
                body.add_child(
                    Flex::column()
                        .with_spacing(GAP_TIGHT)
                        .with_child(heading(appearance, *label))
                        .with_child(value)
                        .finish(),
                );
                if index == 3 && matches!(form.kind, Kind::Assign) {
                    let selected = field.as_ref(app).buffer_text(app);
                    if !selected.is_empty() {
                        let names = form.choices.get(&index).into_iter().flatten()
                            .filter(|(_, id)| selected.split(',').any(|selected| selected == id))
                            .map(|(label, _)| label.clone()).collect::<Vec<_>>().join(", ");
                        body.add_child(note(appearance, names));
                    }
                }
            }
            if !form.error.is_empty() {
                body.add_child(detail(appearance, form.error.clone()));
            }
            let mut buttons = Vec::new();
            for (index, label, action) in [
                (0, "Confirm", Action::ConfirmControl),
                (1, "Cancel", Action::CancelControl),
            ] {
                let button = builder
                    .button(if index == 0 { ButtonVariant::Accent } else { ButtonVariant::Secondary }, form.buttons[index].clone())
                    .with_text_label(label.into());
                let missing_recipient = form.selectors.contains_key(&0) && form.fields[0].as_ref(app).buffer_text(app).is_empty();
                let button = if form.submitting || (index == 0 && (!self.connected || form.choices_loading || missing_recipient ||
                    (matches!(form.kind, Kind::SelectCoordinator) && form.selected_candidate.is_none()))) {
                    button.disabled()
                } else {
                    button
                };
                buttons.push(
                    button
                        .build()
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                        .finish(),
                );
            }
            if let Some(buttons) = button_row(buttons) {
                body.add_child(buttons);
            }
        } else {
            let mut kinds = if self.query.history {
                vec![]
            } else if self.show_spaces {
                vec![
                    Kind::CreateSpace,
                    Kind::MapWorkspace,
                    Kind::RenewReservation,
                    Kind::ReleaseReservation,
                ]
            } else if self.worktree_mode && self.snapshot.as_ref().is_none_or(|snapshot| snapshot.roles.is_empty() || !snapshot.worktree_available) {
                vec![]
            } else {
                vec![Kind::Assign, Kind::Send]
            };
            if self.show_spaces
                && self
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.project.starts_with("space:"))
            {
                kinds.push(Kind::LeaveSpace);
            }
            if let Some(task) = self
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.task.as_ref())
                .filter(|task| self.query.selected_task.as_ref() == Some(&task.id))
            {
                match task.state.as_str() {
                    "queued" | "blocked" => kinds.extend([Kind::Cancel, Kind::Reassign]),
                    "running" | "cancel_requested" => {
                        kinds.extend([Kind::Cancel, Kind::ForceCancel])
                    }
                    "submitted" => kinds.extend([Kind::Accept, Kind::Revise, Kind::Cancel]),
                    "failed" | "expired" | "cancelled" => kinds.extend([
                        Kind::Retry,
                        Kind::RetryOverride,
                        Kind::Reassign,
                        Kind::ReassignOverride,
                        Kind::Archive,
                    ]),
                    "accepted" => kinds.push(Kind::Archive),
                    _ => {}
                }
            }
            if self.worktree_mode {
                kinds.retain(|kind| !matches!(kind, Kind::Cancel | Kind::ForceCancel | Kind::Reassign | Kind::ReassignOverride | Kind::Retry | Kind::RetryOverride));
            }

            let mut buttons = Vec::new();
            for (index, kind) in kinds.into_iter().enumerate() {
                let button = builder
                    .button(ButtonVariant::Secondary, self.control_buttons[index].clone())
                    .with_text_label(kind.label().into());
                let button = if self.connected {
                    button
                } else {
                    button.disabled()
                };
                buttons.push(
                    button
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(Action::OpenControl(kind))
                        })
                        .finish(),
                );
            }
            if !self.query.history && !self.show_spaces && self.query.selected_task.is_none() {
                buttons.push(builder.button(ButtonVariant::Secondary, self.history_buttons[0].clone())
                    .with_text_label(warpui::localization::text("History").into()).build()
                    .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::History)).finish());
            }
            if let Some(buttons) = button_row(buttons) {
                body.add_child(buttons);
            }

        }
        body.finish()
    }

    #[cfg(debug_assertions)]
    pub(super) fn fill_control_checkpoint(&mut self, fields: &[&str], ctx: &mut ViewContext<Self>) {
        let form = self.form.as_ref().expect("Capture form open");
        assert_eq!(fields.len(), form.fields.len());
        for (editor, text) in form.fields.iter().zip(fields) {
            editor.update(ctx, |editor, ctx| editor.set_buffer_text(text, ctx));
        }
    }

    #[cfg(debug_assertions)]
    pub(super) fn control_checkpoint_draft(&self, message: &str, app: &AppContext) -> bool {
        self.form.as_ref().is_some_and(|form| {
            !form.submitting && form.fields[1].as_ref(app).buffer_text(app) == message
        })
    }

    #[cfg(debug_assertions)]
    pub(super) fn control_checkpoint_rejected(&self) -> bool {
        self.form
            .as_ref()
            .is_some_and(|form| !form.submitting && !form.error.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prerequisite_selector_toggles_stable_ids_and_clears_explicitly() {
        assert_eq!(toggle_prerequisite("", "task-a"), "task-a");
        assert_eq!(toggle_prerequisite("task-a", "task-b"), "task-a,task-b");
        assert_eq!(toggle_prerequisite("task-a,task-b", "task-a"), "task-b");
        assert_eq!(toggle_prerequisite("task-a,task-b", ""), "");
    }
    #[test]
    fn native_intent_requires_explicit_fields_and_typed_override() {
        let message = vec![
            "worker".into(),
            "Review the remote result".into(),
            String::new(),
        ];
        assert!(
            matches!(command(Kind::Send, "remote-project", None, &message, "original-message".into()).unwrap(), Command::Agent(Operation::AgentSend { request_id, subject: None, .. }) if request_id == "original-message")
        );
        let fields = vec![
            "worker".into(),
            "Edit the fixture".into(),
            "Focused check passes".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        assert!(
            matches!(command(Kind::Assign, "/project", None, &fields, "original-request".into()).unwrap(), Command::Agent(Operation::TaskAssign { request_id, .. }) if request_id == "original-request")
        );
        assert!(command(Kind::Assign, "/project", None, &fields[..2], "id".into()).is_err());
        let mut scheduled = fields.clone();
        scheduled[0] = "worker, helper".into();
        scheduled[3] = "prerequisite-a, prerequisite-b".into();
        scheduled[4] = "2030-01-01T09:00:00Z".into();
        scheduled[5] = "60".into();
        scheduled[6] = "120".into();
        scheduled[7] = "reviewer".into();
        assert!(
            matches!(command(Kind::Pool, "/project", None, &scheduled, "pool-intent".into()).unwrap(),
            Command::Agent(Operation::TaskCreatePool { eligible, dependencies, start_deadline, execution_timeout_seconds: Some(60), review_timeout_seconds: Some(120), reviewer: Some(reviewer), .. })
                if eligible == ["worker", "helper"] && dependencies == ["prerequisite-a", "prerequisite-b"]
                    && start_deadline.as_deref() == Some("2030-01-01T09:00:00Z") && reviewer == "reviewer")
        );
        scheduled[5] = "0".into();
        assert!(
            matches!(command(Kind::Search, "/project", None, &["literal _%".into()], "read".into()).unwrap(), Command::Search(query) if query == "literal _%")
        );
        assert!(command(
            Kind::Search,
            "/project",
            None,
            &["x".repeat(257)],
            "read".into()
        )
        .is_err());
        assert!(command(
            Kind::Pool,
            "/project",
            None,
            &scheduled,
            "bad-timeout".into()
        )
        .is_err());
        assert!(command(
            Kind::ForceCancel,
            "/project",
            None,
            &["Investigated".into(), "yes".into()],
            "id".into()
        )
        .is_err());
        assert!(command(
            Kind::Purge,
            "/project",
            None,
            &["yes".into()],
            "purge".into()
        )
        .is_err());
        assert!(matches!(
            command(
                Kind::Purge,
                "/project",
                None,
                &["DELETE HISTORY".into()],
                "purge".into()
            )
            .unwrap(),
            Command::Controller(ControllerOperation::HistoryPurge {
                expected_sequence: None,
                ..
            })
        ));
        assert!(command(
            Kind::ArchiveAged,
            "/project",
            None,
            &["0".into()],
            "archive".into()
        )
        .is_err());
        assert!(matches!(
            command(
                Kind::ArchiveAged,
                "/project",
                None,
                &["30".into()],
                "archive".into()
            )
            .unwrap(),
            Command::Controller(ControllerOperation::ArchiveAged {
                older_than_days: Some(30),
                ..
            })
        ));
        let reservation_id = uuid::Uuid::new_v4().to_string();
        assert!(command(
            Kind::ReleaseReservation,
            "/project",
            None,
            &[reservation_id.clone(), "Reviewed".into(), "yes".into()],
            "release".into()
        )
        .is_err());
        assert!(matches!(
            command(
                Kind::ReleaseReservation,
                "/project",
                None,
                &[
                    reservation_id.clone(),
                    "Reviewed".into(),
                    "RELEASE RESERVATION".into()
                ],
                "release".into()
            )
            .unwrap(),
            Command::Reservation { ttl: None, .. }
        ));
        assert!(command(
            Kind::RenewReservation,
            "/project",
            None,
            &[reservation_id, "3601".into(), "Reviewed".into()],
            "renew".into()
        )
        .is_err());
        let mut injected = fields;
        injected[0] = "worker\nother".into();
        assert!(command(Kind::Assign, "/project", None, &injected, "id".into()).is_err());
    }
}
