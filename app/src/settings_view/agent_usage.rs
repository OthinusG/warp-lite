//! Native account configuration for opt-in CLI and API usage readings.
use crate::{
    agent_usage::{agent_icon, providers::Provider, AgentUsage},
    appearance::Appearance,
    editor::{EditorView, SingleLineEditorOptions, TextOptions},
    settings_view::{
        features_page::FeaturesPageView,
        settings_page::{render_sub_header, SettingsWidget},
    },
    view_components::{Dropdown, DropdownItem},
};
use std::{cell::RefCell, collections::HashMap};
use warpui::{
    elements::{ChildView, Flex, MouseStateHandle, ParentElement, Shrinkable},
    ui_components::{
        button::ButtonVariant,
        components::{UiComponent, UiComponentStyles},
    },
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Debug, Clone)]
pub(crate) enum Action {
    Provider(Provider),
    Add,
    Login,
    Refresh,
    Toggle(String),
    Remove(String),
}
pub(crate) struct UsageSettingsView {
    provider: Provider,
    selector: ViewHandle<Dropdown<Action>>,
    label: ViewHandle<EditorView>,
    credential: ViewHandle<EditorView>,
    context: ViewHandle<EditorView>,
    buttons: [MouseStateHandle; 3],
    rows: RefCell<HashMap<String, [MouseStateHandle; 2]>>,
}
impl UsageSettingsView {
    pub(crate) fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.observe(&AgentUsage::handle(ctx), |_, _, ctx| ctx.notify());
        let selector = ctx.add_typed_action_view(Dropdown::new);
        selector.update(ctx, |selector, ctx| {
            selector.set_items(
                Provider::ALL
                    .iter()
                    .map(|p| {
                        DropdownItem::new(p.name(), Action::Provider(*p))
                            .with_icon(agent_icon(p.program()))
                    })
                    .collect(),
                ctx,
            );
            selector.set_selected_by_index(0, ctx);
        });
        let mut editor = |placeholder: &'static str, password: bool, limit| {
            ctx.add_typed_action_view(|ctx| {
                let mut editor = EditorView::single_line(
                    SingleLineEditorOptions {
                        text: TextOptions {
                            font_family_override: Some(Appearance::as_ref(ctx).ui_font_family()),
                            ..Default::default()
                        },
                        is_password: password,
                        max_buffer_len: Some(limit),
                        ..Default::default()
                    },
                    ctx,
                );
                editor.set_placeholder_text(placeholder, ctx);
                editor
            })
        };
        Self {
            provider: Provider::Codex,
            selector,
            label: editor("Account label", false, 80),
            credential: editor("Access token / API key / cookie", true, 16 * 1024),
            context: editor("Account / project / workspace ID (optional)", false, 200),
            buttons: Default::default(),
            rows: Default::default(),
        }
    }
}
impl Entity for UsageSettingsView {
    type Event = ();
}
impl TypedActionView for UsageSettingsView {
    type Action = Action;
    fn handle_action(&mut self, action: &Action, ctx: &mut ViewContext<Self>) {
        match action {
            Action::Provider(provider) => {
                if self.provider != *provider {
                    self.credential.update(ctx, |editor, ctx| {
                        editor.clear_buffer_and_reset_undo_stack(ctx)
                    });
                    self.context.update(ctx, |editor, ctx| {
                        editor.clear_buffer_and_reset_undo_stack(ctx)
                    });
                }
                self.provider = *provider;
                self.context.update(ctx, |editor, ctx| {
                    editor.set_placeholder_text(
                        if *provider == Provider::OpenCodeZen {
                            "Workspace ID (required for Zen balance)"
                        } else {
                            "Account / project / workspace ID (optional)"
                        },
                        ctx,
                    )
                });
                ctx.notify();
            }
            Action::Add => {
                let label = self.label.as_ref(ctx).buffer_text(ctx).trim().to_owned();
                let context = self.context.as_ref(ctx).buffer_text(ctx).trim().to_owned();
                let credential = self
                    .credential
                    .as_ref(ctx)
                    .buffer_text(ctx)
                    .trim()
                    .to_owned();
                let provider = self.provider;
                let added = AgentUsage::handle(ctx).update(ctx, |model, ctx| {
                    model.add(provider, label, context, credential, ctx)
                });
                if added {
                    self.credential.update(ctx, |editor, ctx| {
                        editor.clear_buffer_and_reset_undo_stack(ctx)
                    });
                }
            }
            Action::Login => {
                if let Some(command) = self.provider.login() {
                    use crate::tab_configs::tab_config::{
                        TabConfig, TabConfigPaneNode, TabConfigPaneType,
                    };
                    ctx.dispatch_typed_action_deferred(
                        crate::workspace::WorkspaceAction::SelectTabConfig(TabConfig {
                            name: format!("{} sign in", self.provider.name()),
                            title: None,
                            color: None,
                            panes: vec![TabConfigPaneNode {
                                id: "main".into(),
                                pane_type: Some(TabConfigPaneType::Terminal),
                                split: None,
                                children: None,
                                is_focused: Some(true),
                                directory: None,
                                commands: Some(vec![command.into()]),
                                shell: None,
                            }],
                            params: HashMap::new(),
                            source_path: None,
                        }),
                    );
                }
            }
            Action::Refresh => {
                AgentUsage::handle(ctx).update(ctx, |model, ctx| model.refresh(ctx));
            }
            Action::Toggle(id) => {
                AgentUsage::handle(ctx).update(ctx, |model, ctx| model.toggle(id, ctx));
            }
            Action::Remove(id) => {
                AgentUsage::handle(ctx).update(ctx, |model, ctx| model.remove(id, ctx));
            }
        }
    }
}
impl View for UsageSettingsView {
    fn ui_name() -> &'static str {
        "AgentUsageSettings"
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let model = AgentUsage::as_ref(app);
        let mut body = Flex::column().with_spacing(8.);
        body.add_child(render_sub_header(appearance, warpui::localization::text("Data usage"), None));
        body.add_child(ChildView::new(&self.selector).finish());
        body.add_child(ChildView::new(&self.label).finish());
        if self.provider != Provider::Antigravity {
            body.add_child(ChildView::new(&self.credential).finish());
        }
        if matches!(
            self.provider,
            Provider::Codex | Provider::Gemini | Provider::OpenCodeZen | Provider::Grok
        ) {
            body.add_child(ChildView::new(&self.context).finish());
        }
        body.add_child(
            builder
                .paragraph(self.provider.credential_hint().to_owned())
                .build()
                .finish(),
        );
        let mut buttons = Flex::row().with_spacing(8.);
        for (index, label, action) in [
            (0, "Connect", Action::Add),
            (1, "Sign in", Action::Login),
            (2, "Refresh", Action::Refresh),
        ] {
            if index == 1 && self.provider.login().is_none() {
                continue;
            }
            let button = builder
                .button(ButtonVariant::Secondary, self.buttons[index].clone())
                .with_text_label(label.to_owned());
            let button = if model.checking {
                button.disabled()
            } else {
                button
            };
            buttons.add_child(
                button
                    .build()
                    .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                    .finish(),
            );
        }
        body.add_child(buttons.finish());
        if !model.status.is_empty() {
            body.add_child(builder.paragraph(model.status.clone()).build().finish());
        }
        for account in &model.accounts {
            let states = self
                .rows
                .borrow_mut()
                .entry(account.id.clone())
                .or_default()
                .clone();
            let label = format!(
                "{} · {}{}",
                account.provider.name(),
                account.label,
                if account.cli_login {
                    " · current CLI account on this computer"
                } else {
                    ""
                }
            );
            let mut row = Flex::row()
                .with_spacing(8.)
                .with_child(
                    warpui::elements::ConstrainedBox::new(
                        agent_icon(account.provider.program())
                            .to_warpui_icon(appearance.theme().foreground())
                            .finish(),
                    )
                    .with_width(16.)
                    .with_height(16.)
                    .finish(),
                )
                .with_child(
                    Shrinkable::new(1., builder.span(label).with_soft_wrap().build().finish())
                        .finish(),
                );
            for (index, label, action) in [
                (
                    0,
                    if account.visible { "Hide" } else { "Show" },
                    Action::Toggle(account.id.clone()),
                ),
                (1, "Remove", Action::Remove(account.id.clone())),
            ] {
                row.add_child(
                    builder
                        .button(ButtonVariant::Secondary, states[index].clone())
                        .with_text_label(label.to_owned())
                        .build()
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                        .finish(),
                );
            }
            body.add_child(row.finish());
            if let Some(Err(error)) = model.readings.get(&account.id) {
                body.add_child(builder.span(*error).with_soft_wrap().build().finish());
            }
        }
        body.add_child(builder.paragraph(warpui::localization::text("Other agents: usage unsupported. Qoder CN has no verified background usage interface.").to_owned())
            .with_style(UiComponentStyles { font_size: Some(12.), ..Default::default() }).build().finish());
        body.finish()
    }
}
pub(crate) struct UsageWidget(pub(crate) ViewHandle<UsageSettingsView>);
impl SettingsWidget for UsageWidget {
    type View = FeaturesPageView;
    fn search_terms(&self) -> &str {
        "data usage accounts provider quota api key codex claude gemini cursor opencode antigravity"
    }
    fn render(&self, _: &FeaturesPageView, _: &Appearance, _: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.0).finish()
    }
}
