//! Native human intent bound to the original scope/version, using existing editors.
use super::{Action, CollaborationPanel};
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
    Assign,
    Cancel,
    Accept,
    Revise,
    Retry,
    RetryOverride,
    Reassign,
    ReassignOverride,
    ForceCancel,
    Archive,
    CreateSpace,
    MapWorkspace,
    LeaveSpace,
}
impl Kind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Assign => "Assign task",
            Self::Cancel => "Request cancellation",
            Self::Accept => "Accept result",
            Self::Revise => "Request revision",
            Self::Retry => "Retry",
            Self::RetryOverride => "Retry with override",
            Self::Reassign => "Reassign",
            Self::ReassignOverride => "Reassign with override",
            Self::ForceCancel => "Override cancellation",
            Self::Archive => "Archive",
            Self::CreateSpace => "Create space",
            Self::MapWorkspace => "Map checkout",
            Self::LeaveSpace => "Leave participation",
        }
    }
    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Assign => &["Recipient agent name", "Description", "Acceptance criteria"],
            Self::Reassign => &["New recipient agent name", "Reason"],
            Self::ReassignOverride => &["New recipient agent name", "Reason", "Type ALLOW OVERLAP"],
            Self::ForceCancel | Self::RetryOverride => &["Reason", "Type ALLOW OVERLAP"],
            Self::Accept | Self::Revise => &["Review feedback"],
            Self::Archive => &[],
            Self::CreateSpace => &["Space name"],
            Self::MapWorkspace => &[
                "Space UUID",
                "Canonical checkout root",
                "Repository UUID (optional; explicit logical overlap group)",
            ],
            Self::LeaveSpace => &["Agent name to remove"],
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
    kind: Kind,
    project: String,
    task: Option<Task>,
    request_id: String,
    fields: Vec<ViewHandle<EditorView>>,
    submitting: bool,
    error: String,
    submitted_fields: Option<Vec<String>>,
    buttons: [MouseStateHandle; 2],
}

enum Command {
    Agent(Operation),
    Controller(ControllerOperation),
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
        fields
            .iter()
            .enumerate()
            .all(|(index, field)| (kind == Kind::MapWorkspace && index == 2
                || !field.trim().is_empty())
                && field.len() <= 8192
                && !field.chars().any(char::is_control)),
        "Complete every field using plain text (up to 8192 bytes)"
    );
    if kind.overrides() {
        anyhow::ensure!(
            fields.last().is_some_and(|field| field == "ALLOW OVERLAP"),
            "Type ALLOW OVERLAP to acknowledge that earlier execution may still be writing"
        );
    }
    if kind == Kind::Assign {
        return Ok(Command::Agent(Operation::TaskAssign {
            to: fields[0].clone(),
            description: fields[1].clone(),
            acceptance: fields[2].clone(),
            reviewer: None,
            request_id,
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
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
        Kind::Assign | Kind::CreateSpace | Kind::MapWorkspace | Kind::LeaveSpace => unreachable!(),
    })
}

impl CollaborationPanel {
    pub(super) fn open_control(&mut self, kind: Kind, ctx: &mut ViewContext<Self>) {
        if !self.connected || self.preview || self.form.as_ref().is_some_and(|form| form.submitting)
        {
            return;
        }
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        if !matches!(
            kind,
            Kind::Assign | Kind::CreateSpace | Kind::MapWorkspace | Kind::LeaveSpace
        ) && snapshot
            .task
            .as_ref()
            .is_none_or(|task| self.query.selected_task.as_ref() != Some(&task.id))
        {
            return;
        }
        let project = snapshot.project.clone();
        let task = snapshot.task.clone();
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
        ctx.dispatch_typed_action(&crate::workspace::WorkspaceAction::FocusLeftPanel);
        ctx.notify();
    }

    pub(super) fn confirm_control(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.connected
            || Self::current_context(ctx) != self.context
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
        let operation = match command(
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
        let Some(broker) = crate::agent_communication::BROKER.get().cloned() else {
            return;
        };
        let project = form.project.clone();
        let request_id = form.request_id.clone();
        form.submitting = true;
        form.submitted_fields = Some(fields);
        form.error.clear();
        ctx.spawn(async move {
            match operation { Command::Agent(operation) => broker.operator(&project, &operation), Command::Controller(operation) => broker.control(&project, &operation) }
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
                    let form = panel.form.as_mut().unwrap();
                    form.submitting = false;
                    let code = error.downcast_ref::<warp_agent_bus::DomainError>().map(|error| error.code.as_str()).unwrap_or("coordinator_unavailable");
                    form.error = format!("Operation rejected ({code}). Original version and request were preserved. Resolve the blocker or close this form, refresh and confirm a new intent.");
                }
            }
            ctx.notify();
        });
        ctx.dispatch_typed_action(&crate::workspace::WorkspaceAction::FocusLeftPanel);
        ctx.notify();
    }

    pub(super) fn render_controls(&self, app: &AppContext) -> Box<dyn Element> {
        let builder = Appearance::as_ref(app).ui_builder();
        let mut body = Flex::column().with_spacing(12.);
        if let Some(form) = &self.form {
            body.add_child(
                builder
                    .span(form.kind.label())
                    .with_soft_wrap()
                    .build()
                    .finish(),
            );
            if let Some(task) = &form.task {
                body.add_child(
                    builder
                        .span(format!(
                            "Task {} · original revision {} · version {}",
                            task.id, task.revision, task.version
                        ))
                        .with_soft_wrap()
                        .build()
                        .finish(),
                );
            }
            if form.kind.overrides() {
                body.add_child(builder.span("Earlier execution may still be writing. This operation changes coordination ownership; it does not stop a process or file writes.").with_soft_wrap().build().finish());
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
                body.add_child(builder.span(*label).with_soft_wrap().build().finish());
                if form.submitting {
                    body.add_child(
                        builder
                            .span(field.as_ref(app).buffer_text(app))
                            .with_soft_wrap()
                            .build()
                            .finish(),
                    );
                } else {
                    body.add_child(ChildView::new(field).finish());
                }
            }
            body.add_child(
                builder
                    .span(form.error.clone())
                    .with_soft_wrap()
                    .build()
                    .finish(),
            );
            for (index, label, action) in [
                (0, "Confirm operation", Action::ConfirmControl),
                (1, "Close form", Action::CancelControl),
            ] {
                let button = builder
                    .button(ButtonVariant::Text, form.buttons[index].clone())
                    .with_text_label(label.into());
                let button = if form.submitting {
                    button.disabled()
                } else {
                    button
                };
                body.add_child(
                    button
                        .build()
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                        .finish(),
                );
            }
        } else {
            let mut kinds = if self.show_spaces {
                vec![Kind::CreateSpace, Kind::MapWorkspace]
            } else {
                vec![Kind::Assign]
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
            body.add_child(
                builder
                    .span("Human controls")
                    .with_soft_wrap()
                    .build()
                    .finish(),
            );
            for (index, kind) in kinds.into_iter().enumerate() {
                let button = builder
                    .button(ButtonVariant::Text, self.control_buttons[index].clone())
                    .with_text_label(kind.label().into());
                let button = if self.connected {
                    button
                } else {
                    button.disabled()
                };
                body.add_child(
                    button
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(Action::OpenControl(kind))
                        })
                        .finish(),
                );
            }
            body.add_child(builder.span("Cancellation requests do not stop the CLI process. Native approvals and terminal drafts remain under your control.").with_soft_wrap().build().finish());
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
        let fields = vec![
            "worker".into(),
            "Edit the fixture".into(),
            "Focused check passes".into(),
        ];
        assert!(
            matches!(command(Kind::Assign, "/project", None, &fields, "original-request".into()).unwrap(), Command::Agent(Operation::TaskAssign { request_id, .. }) if request_id == "original-request")
        );
        assert!(command(Kind::Assign, "/project", None, &fields[..2], "id".into()).is_err());
        assert!(command(
            Kind::ForceCancel,
            "/project",
            None,
            &["Investigated".into(), "yes".into()],
            "id".into()
        )
        .is_err());
        let mut injected = fields;
        injected[0] = "worker\nother".into();
        assert!(command(Kind::Assign, "/project", None, &injected, "id".into()).is_err());
    }
}
