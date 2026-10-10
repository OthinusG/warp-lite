//! Local-only setup controls for independently installed CLI agents.
use crate::agent_communication::AgentCommunication;
use crate::{
    appearance::Appearance,
    settings_view::{
        features_page::FeaturesPageView,
        settings_page::{
            render_body_item, render_body_item_label_with_icon, build_toggle_element,
            render_sub_header, LocalOnlyIconState, SettingsWidget, ToggleState,
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
    #[cfg(windows)]
    WslEnable,
    #[cfg(windows)]
    WslSelect(Option<crate::agent_communication::wsl_settings::Target>),
    #[cfg(windows)]
    WslAgent(String),
    #[cfg(windows)]
    WslRefresh,
    #[cfg(windows)]
    WslRemoveAll,
    #[cfg(windows)]
    WslCompanion,
}
pub struct CommunicationSettingsView {
    switch: SwitchStateHandle,
    refresh: MouseStateHandle,
    uninstall_all: MouseStateHandle,
    checkboxes: RefCell<HashMap<String, MouseStateHandle>>,
    #[cfg(windows)]
    wsl_switch: SwitchStateHandle,
    #[cfg(windows)]
    wsl_distribution: ViewHandle<crate::view_components::Dropdown<Action>>,
    #[cfg(windows)]
    wsl_maintenance: [MouseStateHandle; 2],
    #[cfg(windows)]
    wsl_companion: MouseStateHandle,
}
impl CommunicationSettingsView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        #[cfg(windows)]
        let wsl_distribution = ctx.add_typed_action_view(|ctx| {
            let mut dropdown = crate::view_components::Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(260.);
            dropdown
        });
        ctx.observe(&AgentCommunication::handle(ctx), |_view, _, ctx| {
            #[cfg(windows)]
            _view.update_wsl_dropdown(ctx);
            ctx.notify();
        });
        #[cfg(windows)]
        {
            AgentCommunication::handle(ctx).update(ctx, |model, ctx| {
                if model.wsl.preferences.enabled { model.configure_wsl(None, None, ctx); }
            });
        }
        let view = Self {
            switch: Default::default(),
            refresh: Default::default(),
            uninstall_all: Default::default(),
            checkboxes: Default::default(),
            #[cfg(windows)]
            wsl_switch: Default::default(),
            #[cfg(windows)]
            wsl_distribution,
            #[cfg(windows)]
            wsl_maintenance: Default::default(),
            #[cfg(windows)]
            wsl_companion: Default::default(),
        };
        #[cfg(windows)]
        view.update_wsl_dropdown(ctx);
        view
    }

    #[cfg(windows)]
    fn update_wsl_dropdown(&self, ctx: &mut ViewContext<Self>) {
        use crate::view_components::DropdownItem;
        let settings = &AgentCommunication::as_ref(ctx).wsl;
        let selected = settings.selected_target();
        let mut items = vec![DropdownItem::new(warpui::localization::text("Select logged-in WSL account"), Action::WslSelect(None))];
        let busy = settings.busy;
        let mut index = 0;
        for target in &settings.targets {
            if selected == Some(target) { index = items.len(); }
            items.push(DropdownItem::new(target.label(), Action::WslSelect(Some(target.clone()))));
        }
        self.wsl_distribution.update(ctx, |dropdown, ctx| {
            dropdown.set_items(items, ctx);
            dropdown.set_selected_by_index(index, ctx);
            if busy { dropdown.set_disabled(ctx); } else { dropdown.set_enabled(ctx); }
        });
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
            #[cfg(windows)]
            Action::WslEnable => model.configure_wsl(Some(!model.wsl.preferences.enabled), None, ctx),
            #[cfg(windows)]
            Action::WslSelect(target) => {
                if let Some(target) = target { model.configure_wsl(None, Some(target.clone()), ctx); }
            }
            #[cfg(windows)]
            Action::WslAgent(command) => model.select_wsl_agent(command.clone(), ctx),
            #[cfg(windows)]
            Action::WslRefresh => model.maintain_wsl(false, ctx),
            #[cfg(windows)]
            Action::WslRemoveAll => model.maintain_wsl(true, ctx),
            #[cfg(windows)]
            Action::WslCompanion => model.install_wsl_companion(ctx),
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
            warpui::localization::text("Agent communication").into(),
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
            body.add_child(render_sub_header(appearance, warpui::localization::text("Agents"), None));
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
                        .paragraph(warpui::localization::text("No installed managed CLI agents were found.").to_owned())
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
                body.add_child(build_toggle_element(
                    render_body_item_label_with_icon::<Action>(
                        row.command, crate::agent_usage::agent_icon(&row.program), None, None,
                        LocalOnlyIconState::Hidden,
                        if model.busy || row.installed.is_none() { ToggleState::Disabled } else { ToggleState::Enabled },
                        appearance,
                    ),
                    checkbox
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(Action::Select(command.clone()))
                        })
                        .finish(),
                    appearance, Some(warpui::localization::text(&row.status).to_owned()),
                ));
            }
        }
        body.add_child(secondary_text(appearance, warpui::localization::text(&model.status).to_owned(), None));
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
                .with_text_label(warpui::localization::text("Rescan agents").to_owned());
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
                .with_text_label(warpui::localization::text("Remove Warpai MCP from all agents").to_owned());
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
        #[cfg(windows)]
        {
            let wsl_switch = builder.switch(self.wsl_switch.clone()).check(model.wsl.preferences.enabled);
            let wsl_switch = if model.wsl.busy { wsl_switch.disable() } else { wsl_switch };
            body.add_child(render_body_item::<Action>(
                warpui::localization::text("WSL communication").into(), None, LocalOnlyIconState::Hidden, ToggleState::Enabled, appearance,
                wsl_switch.build().on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::WslEnable)).finish(),
                Some("Log into WSL in a Warpai terminal before configuring communication.".into()),
            ));
            if model.wsl.preferences.enabled {
                body.add_child(render_body_item::<Action>(
                    warpui::localization::text("WSL account").into(), None, LocalOnlyIconState::Hidden, ToggleState::Enabled, appearance,
                    ChildView::new(&self.wsl_distribution).finish(), None,
                ));
                if model.wsl.targets.is_empty() {
                    body.add_child(secondary_text(appearance, "No logged-in WSL 2 accounts. Open a WSL terminal in Warpai first.".into(), None));
                }
                if model.wsl.selected_target().is_some() {
                    let checkbox = builder.checkbox(self.wsl_companion.clone(), None)
                        .check(model.wsl.companion_installed == Some(true));
                    let checkbox = if model.wsl.busy || model.wsl.companion_installed.is_none() { checkbox.disabled() } else { checkbox };
                    body.add_child(build_toggle_element(
                        render_body_item_label_with_icon::<Action>(warpui::localization::text("Install WSL Companion").into(), crate::agent_usage::agent_icon("custom"), None, None,
                            LocalOnlyIconState::Hidden, if model.wsl.busy { ToggleState::Disabled } else { ToggleState::Enabled }, appearance),
                        checkbox.build().on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::WslCompanion)).finish(),
                        appearance, Some("Bundled with Windows Warpai. Applies only to the selected WSL account; unchecking removes its installation.".into()),
                    ));
                }
                for (command, _) in &model.wsl.agents {
                    body.add_child(render_body_item_label_with_icon::<Action>(
                        command.clone(), crate::agent_usage::agent_icon(command), None, None,
                        LocalOnlyIconState::Hidden, ToggleState::Enabled, appearance,
                    ));
                }
                for row in &model.wsl.available {
                    let state = self.checkboxes.borrow_mut().entry(format!("wsl:{}", row.command)).or_default().clone();
                    let checkbox = builder.checkbox(state, None)
                        .check(model.wsl.guest_preferences.selected.get(&row.command).is_some_and(|entry| entry.active));
                    let checkbox = if model.wsl.busy || row.installed.is_none() { checkbox.disabled() } else { checkbox };
                    let command = row.command.clone();
                    body.add_child(build_toggle_element(
                        render_body_item_label_with_icon::<Action>(row.command.clone(), crate::agent_usage::agent_icon(&row.program), None, None,
                            LocalOnlyIconState::Hidden, if model.wsl.busy || row.installed.is_none() { ToggleState::Disabled } else { ToggleState::Enabled }, appearance),
                        checkbox.build().on_click(move |ctx, _, _| ctx.dispatch_typed_action(Action::WslAgent(command.clone()))).finish(),
                        appearance, Some(warpui::localization::text(&row.status).to_owned()),
                    ));
                }
                if !model.wsl.status.is_empty() {
                    body.add_child(secondary_text(appearance, warpui::localization::text(&model.wsl.status).to_owned(), None));
                }
                if model.wsl.selected_target().is_some() {
                    for (index, label, action) in [(0, "Rescan agents", Action::WslRefresh), (1, "Remove Warpai MCP from all agents", Action::WslRemoveAll)] {
                        let button = builder.button(ButtonVariant::Secondary, self.wsl_maintenance[index].clone())
                            .with_text_label(warpui::localization::text(label).into());
                        let button = if model.wsl.busy { button.disabled() } else { button };
                        body.add_child(Container::new(button.build().on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone())).finish())
                            .with_margin_top(8.).finish());
                    }
                    body.add_child(secondary_text(appearance, warpui::localization::text("Applies only to this WSL account.").into(), None));
                }
            }
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
        if cfg!(windows) { "agent communication collaboration mcp bridge wsl distribution" }
        else { "agent communication collaboration mcp bridge" }
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
