//! Local-only setup controls for independently installed CLI agents.
use crate::agent_communication::AgentCommunication;
use crate::{
    appearance::Appearance,
    settings_view::{
        features_page::FeaturesPageView,
        settings_page::{
            render_body_item, render_sub_header, LocalOnlyIconState, SettingsWidget, ToggleState,
        },
    },
    ui_components::blended_colors,
};
use std::{cell::RefCell, collections::HashMap};
use warpui::elements::{ChildView, Container, Flex, MouseStateHandle, ParentElement};
use warpui::ui_components::{
    button::ButtonVariant,
    components::{UiComponent, UiComponentStyles},
    switch::SwitchStateHandle,
};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Debug, Clone)]
pub enum Action {
    Enable,
    Select(String),
    Refresh,
    UninstallAll,
}
pub struct CommunicationSettingsView {
    switch: SwitchStateHandle,
    refresh: MouseStateHandle,
    uninstall_all: MouseStateHandle,
    checkboxes: RefCell<HashMap<String, MouseStateHandle>>,
}
impl CommunicationSettingsView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.observe(&AgentCommunication::handle(ctx), |_, _, ctx| ctx.notify());
        Self {
            switch: Default::default(),
            refresh: Default::default(),
            uninstall_all: Default::default(),
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
            Action::UninstallAll => model.uninstall_all(ctx),
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
            body.add_child(render_sub_header(appearance, "Agents", None));
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
        body.add_child(secondary_text(appearance, model.status.clone(), None));
        if model.preferences.legacy_cleanup_pending() {
            // Cleanup needs user action, so it uses the theme warning color instead of plain text.
            body.add_child(secondary_text(
                appearance,
                "Legacy device access is disabled. Unlock secure storage and restart Warpai to retry credential cleanup.".to_owned(),
                Some(appearance.theme().ui_warning_color()),
            ));
        }
        if model.preferences.enabled {
        let refresh = builder
            .button(ButtonVariant::Secondary, self.refresh.clone())
            .with_text_label("Rescan agents".to_owned());
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
        let uninstall_all = builder
            .button(ButtonVariant::Secondary, self.uninstall_all.clone())
            .with_text_label("Remove Warpai MCP from all agents".to_owned());
        let uninstall_all = if model.busy {
            uninstall_all.disabled()
        } else {
            uninstall_all
        };
        body.add_child(
            Container::new(
                uninstall_all
                    .build()
                    .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::UninstallAll))
                    .finish(),
            )
            .with_margin_top(8.)
            .finish(),
        );
        }
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

/// 12px secondary text matching `render_body_item` descriptions; `color` overrides for warnings.
fn secondary_text(
    appearance: &Appearance,
    text: String,
    color: Option<pathfinder_color::ColorU>,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    appearance
        .ui_builder()
        .span(text)
        .with_style(UiComponentStyles {
            font_size: Some(12.),
            font_color: Some(
                color.unwrap_or_else(|| blended_colors::text_sub(theme, theme.surface_1())),
            ),
            ..Default::default()
        })
        .with_soft_wrap()
        .build()
        .finish()
}
