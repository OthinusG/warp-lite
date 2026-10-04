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
    ConnectSsh,
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
}
impl Kind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ConnectSsh => "Connect SSH project",
            Self::Send => "Send message",
            Self::Assign => "Assign task",
            Self::Pool => "Create pool task",
            Self::Search => "Search messages",
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
        }
    }
    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::ConnectSsh => &[
                "System SSH alias",
                "Absolute remote project root",
                "Absolute companion path",
            ],
            Self::Send => &["Recipient agent name", "Message", "Subject (optional)"],
            Self::Assign | Self::Pool => &[
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
    project: String,
    task: Option<Task>,
    purge_preview: Option<super::PurgePreview>,
    reservations: Vec<warp_agent_bus::Reservation>,
    request_id: String,
    fields: Vec<ViewHandle<EditorView>>,
    submitting: bool,
    error: String,
    submitted_fields: Option<Vec<String>>,
    buttons: [MouseStateHandle; 2],
}

enum Command {
    Connect(warp_agent_bus::ssh_remote::SshProfile),
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
    if kind == Kind::ConnectSsh {
        let profile = warp_agent_bus::ssh_remote::SshProfile {
            target: fields[0].trim().into(),
            remote_root: fields[1].trim().into(),
            companion_path: fields[2].trim().into(),
            config_file: None,
            remote_shell: if fields[1].trim().starts_with('/') {
                warp_agent_bus::ssh_remote::RemoteShell::Posix
            } else {
                warp_agent_bus::ssh_remote::RemoteShell::PowerShell
            },
        };
        profile.validate().map_err(|_| {
            anyhow::anyhow!(
                "Use a system SSH alias and absolute paths for the remote operating system"
            )
        })?;
        return Ok(Command::Connect(profile));
    }
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
        Kind::ConnectSsh
        | Kind::Send
        | Kind::RenewReservation
        | Kind::ReleaseReservation
        | Kind::ArchiveAged
        | Kind::Purge
        | Kind::Assign
        | Kind::Pool
        | Kind::Search
        | Kind::CreateSpace
        | Kind::MapWorkspace
        | Kind::LeaveSpace => {
            unreachable!()
        }
    })
}

impl CollaborationPanel {
    pub(super) fn open_control(&mut self, kind: Kind, ctx: &mut ViewContext<Self>) {
        if (kind != Kind::ConnectSsh && !self.connected) || self.preview || self.form.is_some() {
            return;
        }
        let snapshot = self.snapshot.as_ref();
        if kind != Kind::ConnectSsh && snapshot.is_none() {
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
            Kind::ConnectSsh
                | Kind::Send
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
        let task = if matches!(kind, Kind::Send | Kind::ConnectSsh) {
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
            project,
            task,
            purge_preview,
            reservations,
            fields,
            request_id: uuid::Uuid::new_v4().to_string(),
            submitting: false,
            error: String::new(),
            submitted_fields: None,
            buttons: Default::default(),
        });
        self.scroll = Default::default();
        ctx.notify();
    }

    pub(super) fn cancel_control(&mut self, ctx: &mut ViewContext<Self>) {
        if self.form.as_ref().is_some_and(|form| form.submitting) {
            return;
        }
        self.form = None;
        ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::FocusLeftPanel);
        ctx.notify();
    }

    pub(super) fn confirm_control(&mut self, ctx: &mut ViewContext<Self>) {
        let connecting = self
            .form
            .as_ref()
            .is_some_and(|form| form.kind == Kind::ConnectSsh);
        if (!connecting && !self.connected)
            || (!connecting && self.current_context(ctx) != self.context)
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
        if !connecting
            && self
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
        let mut operation = match command(
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
        };
        if let Command::Connect(profile) = operation {
            self.remote = Some(profile);
            self.form = None;
            self.context = None;
            self.reconnect_remote(ctx);
            ctx.dispatch_typed_action_deferred(crate::workspace::WorkspaceAction::FocusLeftPanel);
            return;
        }
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
                        format!("Operation rejected ({code}). Original version and request were preserved. Resolve the blocker or close this form, refresh and confirm a new intent.")
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
                body.add_child(builder.span("Earlier execution may still be writing. This operation changes coordination ownership; it does not stop a process or file writes.").with_soft_wrap().build().finish());
            }
            if matches!(form.kind, Kind::RenewReservation | Kind::ReleaseReservation) {
                body.add_child(builder.span("Use an ID from the original reservation page. Renewal requires a confirmed active owner; release removes advisory coordination only and does not stop execution or file writes.").with_soft_wrap().build().finish());
                let id = form.fields[0].as_ref(app).buffer_text(app);
                if let Some(reservation) = form
                    .reservations
                    .iter()
                    .find(|reservation| reservation.id == id.trim())
                {
                    body.add_child(builder.span(format!("Original checkout {} · path {} · owner {} · expiry {} · attempt {}", reservation.workspace, reservation.path, reservation.owner, reservation.expires_at, reservation.attempt_id.as_deref().unwrap_or("unlinked"))).with_soft_wrap().build().finish());
                }
            }
            if form.kind == Kind::Purge {
                if let Some(preview) = &form.purge_preview {
                    body.add_child(builder.span(format!("Delete up to {} archived tasks and {} acknowledged messages from original preview sequence {}. This is irreversible. History changes require a new preview and intent.", preview.tasks, preview.messages, preview.sequence)).with_soft_wrap().build().finish());
                }
            }
            if form.kind == Kind::Archive {
                body.add_child(
                    builder
                        .span(
                            "Archive this terminal task while retaining its evidence and history.",
                        )
                        .with_soft_wrap()
                        .build()
                        .finish(),
                );
            }
            if form.kind == Kind::MapWorkspace {
                body.add_child(builder.span("This mapping applies only to new explicitly joined panes. Changing a checkout's space revokes its earlier shared admissions. Existing private work remains private; matching Git remotes never joins projects.").with_soft_wrap().build().finish());
            }
            if form.kind == Kind::LeaveSpace {
                body.add_child(builder.span("Revoke the selected agent's shared coordination access and wake eligibility. Its task attempts remain in this space and may still be executing; this does not stop the CLI process.").with_soft_wrap().build().finish());
            }
            for (label, field) in form.kind.labels().iter().zip(&form.fields) {
                let value = if form.submitting {
                    detail(appearance, field.as_ref(app).buffer_text(app))
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
            }
            if !form.error.is_empty() {
                body.add_child(detail(appearance, form.error.clone()));
            }
            let mut buttons = Vec::new();
            for (index, label, action) in [
                (0, "Confirm operation", Action::ConfirmControl),
                (1, "Close form", Action::CancelControl),
            ] {
                let button = builder
                    .button(ButtonVariant::Text, form.buttons[index].clone())
                    .with_text_label(label.into());
                let button = if form.submitting
                    || (index == 0 && form.kind != Kind::ConnectSsh && !self.connected)
                {
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
                vec![Kind::ArchiveAged, Kind::Purge]
            } else if self.show_spaces {
                vec![
                    Kind::CreateSpace,
                    Kind::MapWorkspace,
                    Kind::RenewReservation,
                    Kind::ReleaseReservation,
                ]
            } else {
                vec![Kind::Assign, Kind::Pool, Kind::Search, Kind::Send]
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
            body.add_child(heading(appearance, "Human controls"));
            let mut buttons = Vec::new();
            for (index, kind) in kinds.into_iter().enumerate() {
                let button = builder
                    .button(ButtonVariant::Text, self.control_buttons[index].clone())
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
            if let Some(buttons) = button_row(buttons) {
                body.add_child(buttons);
            }
            body.add_child(note(appearance, "Cancellation requests do not stop the CLI process. Native approvals and terminal drafts remain under your control."));
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
    fn native_intent_requires_explicit_fields_and_typed_override() {
        let ssh = vec![
            "research-node".into(),
            "/srv/project".into(),
            "/opt/warpai/warpai-companion".into(),
        ];
        assert!(
            matches!(command(Kind::ConnectSsh, "", None, &ssh, "selection".into()).unwrap(), Command::Connect(profile) if profile.target == "research-node")
        );
        let mut invalid = ssh.clone();
        invalid[0] = "-oProxyCommand=unexpected".into();
        assert!(command(Kind::ConnectSsh, "", None, &invalid, "selection".into()).is_err());
        invalid = ssh;
        invalid[1] = "relative/root".into();
        assert!(command(Kind::ConnectSsh, "", None, &invalid, "selection".into()).is_err());
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
