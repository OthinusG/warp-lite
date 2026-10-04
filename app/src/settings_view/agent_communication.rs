//! Local-only setup controls for independently installed CLI agents.
use crate::agent_communication::AgentCommunication;
use crate::{
    appearance::Appearance,
    settings_view::{
        features_page::FeaturesPageView,
        settings_page::{render_body_item, LocalOnlyIconState, SettingsWidget, ToggleState},
    },
};
use std::{cell::RefCell, collections::HashMap};
use warpui::elements::{ChildView, Container, Flex, MouseStateHandle, ParentElement};
use warpui::ui_components::{
    button::ButtonVariant, components::UiComponent, switch::SwitchStateHandle,
};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Debug, Clone)]
pub enum Action {
    Enable,
    Select(String),
    Refresh,
}
pub struct CommunicationSettingsView {
    switch: SwitchStateHandle,
    refresh: MouseStateHandle,
    checkboxes: RefCell<HashMap<String, MouseStateHandle>>,
}
impl CommunicationSettingsView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.observe(&AgentCommunication::handle(ctx), |_, _, ctx| ctx.notify());
        Self {
            switch: Default::default(),
            refresh: Default::default(),
            checkboxes: Default::default(),
        }
    }
}
impl Entity for CommunicationSettingsView {
    type Event = ();
}
impl TypedActionView for CommunicationSettingsView {
    type Action = Action;
    fn handle_action(&mut self, action: &Action, ctx: &mut ViewContext<Self>) {
        AgentCommunication::handle(ctx).update(ctx, |model, ctx| match action {
            Action::Enable => model.configure(Some(!model.preferences.enabled), None, ctx),
            Action::Select(command) => model.configure(None, Some(command.clone()), ctx),
            Action::Refresh => model.configure(None, None, ctx),
        });
    }
}
impl View for CommunicationSettingsView {
    fn ui_name() -> &'static str {
        "AgentCommunicationSettings"
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let model = AgentCommunication::as_ref(app);
        let switch = builder
            .switch(self.switch.clone())
            .check(model.preferences.enabled);
        let switch = if model.busy { switch.disable() } else { switch };
        let mut body = Flex::column();
        body.add_child(render_body_item::<Action>(
            "Agent communication".into(),
            None,
            LocalOnlyIconState::Hidden,
            if model.busy {
                ToggleState::Disabled
            } else {
                ToggleState::Enabled
            },
            appearance,
            switch
                .build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Enable))
                .finish(),
            Some("Configure local MCP access for selected CLI agents. Communication stays within each project.".into()),
        ));
        if model.preferences.enabled || !model.preferences.selected.is_empty() {
            let mut rows = model.available.clone();
            // Keep removed executables visible so owned configuration can still be cleaned up.
            for (command, installed) in &model.preferences.selected {
                if !rows.iter().any(|row| &row.command == command) {
                    rows.push(crate::agent_communication::setup::Available {
                        command: command.clone(),
                        program: installed.program.clone(),
                        installed: Some(installed.clone()),
                        status: "Disabled — select to retry configuration cleanup".into(),
                    });
                }
            }
            if rows.is_empty() && !model.busy {
                body.add_child(
                    builder
                        .paragraph("No installed managed CLI agents were found.".to_owned())
                        .build()
                        .finish(),
                );
            }
            for row in rows {
                let state = self
                    .checkboxes
                    .borrow_mut()
                    .entry(row.command.clone())
                    .or_default()
                    .clone();
                let checkbox = builder.checkbox(state, None).check(
                    model
                        .preferences
                        .selected
                        .get(&row.command)
                        .is_some_and(|entry| entry.active),
                );
                let checkbox = if model.busy || row.installed.is_none() {
                    checkbox.disabled()
                } else {
                    checkbox
                };
                let command = row.command.clone();
                body.add_child(render_body_item::<Action>(
                    row.command,
                    None,
                    LocalOnlyIconState::Hidden,
                    if model.busy || row.installed.is_none() {
                        ToggleState::Disabled
                    } else {
                        ToggleState::Enabled
                    },
                    appearance,
                    checkbox
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(Action::Select(command.clone()))
                        })
                        .finish(),
                    Some(row.status),
                ));
            }
        }
        body.add_child(builder.paragraph(model.status.clone()).build().finish());
        if model.preferences.legacy_cleanup_pending() {
            body.add_child(builder.paragraph(
                "Legacy device access is disabled. Unlock secure storage and restart Warpai to retry credential cleanup.".to_owned(),
            ).build().finish());
        }
        let refresh = builder
            .button(ButtonVariant::Secondary, self.refresh.clone())
            .with_text_label("Refresh agents / retry cleanup".to_owned());
        let refresh = if model.busy {
            refresh.disabled()
        } else {
            refresh
        };
        body.add_child(
            Container::new(
                refresh
                    .build()
                    .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Refresh))
                    .finish(),
            )
            .with_margin_top(12.)
            .finish(),
        );
        Container::new(body.finish())
            .with_margin_bottom(16.)
            .finish()
    }
}
pub struct CommunicationWidget(pub ViewHandle<CommunicationSettingsView>);
impl SettingsWidget for CommunicationWidget {
    type View = FeaturesPageView;
    fn search_terms(&self) -> &str {
        "agent communication collaboration mcp bridge"
    }
    fn render(&self, _: &FeaturesPageView, _: &Appearance, _: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.0).finish()
    }
}
