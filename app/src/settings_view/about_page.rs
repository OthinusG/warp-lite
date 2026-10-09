use super::{
    settings_page::{
        MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle,
        SettingsWidget,
    },
    SettingsSection,
};
use crate::release_updates::{ReleaseUpdates, UpdateSettings, RELEASES_URL};
use crate::{appearance::Appearance, channel::ChannelState, workspace::WorkspaceAction};
use settings::{Setting as _, ToggleableSetting as _};
use warpui::{
    assets::asset_cache::AssetSource,
    elements::{
        Align, CacheOption, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Hoverable,
        Image, MainAxisAlignment, MouseStateHandle, ParentElement, Shrinkable, Wrap,
    },
    ui_components::{
        button::ButtonVariant,
        components::UiComponent,
        switch::SwitchStateHandle,
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Clone, Debug)]
pub enum AboutAction {
    Update,
    Download,
    ToggleStartup,
    OpenReleases,
}

pub struct AboutPageView {
    page: PageType<Self>,
}

impl AboutPageView {
    pub fn new(ctx: &mut ViewContext<AboutPageView>) -> Self {
        ctx.observe(&ReleaseUpdates::handle(ctx), |_, _, ctx| ctx.notify());
        ctx.observe(&UpdateSettings::handle(ctx), |_, _, ctx| ctx.notify());
        AboutPageView {
            page: PageType::new_monolith(AboutPageWidget::default(), None, false),
        }
    }
}

impl Entity for AboutPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for AboutPageView {
    type Action = AboutAction;
    fn handle_action(&mut self, action: &AboutAction, ctx: &mut ViewContext<Self>) {
        match action {
            AboutAction::OpenReleases => ctx.open_url(RELEASES_URL),
            AboutAction::Update => {
                ReleaseUpdates::handle(ctx).update(ctx, |updates, ctx| updates.check(ctx));
            }
            AboutAction::Download => {
                if let Some(url) = &ReleaseUpdates::as_ref(ctx).download_url {
                    ctx.open_url(url);
                }
            }
            AboutAction::ToggleStartup => {
                UpdateSettings::handle(ctx).update(ctx, |settings, ctx| {
                    crate::report_if_error!(settings.check_on_startup.toggle_and_save_value(ctx));
                })
            }
        }
    }
}

impl View for AboutPageView {
    fn ui_name() -> &'static str {
        "AboutPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Default)]
struct AboutPageWidget {
    copy_version_button_mouse_state: MouseStateHandle,
    update_button: MouseStateHandle,
    download_button: MouseStateHandle,
    startup_switch: SwitchStateHandle,
    startup_row: MouseStateHandle,
    releases_button: MouseStateHandle,
}

impl SettingsWidget for AboutPageWidget {
    type View = AboutPageView;

    fn search_terms(&self) -> &str {
        "about warpai warp version update check startup releases"
    }

    fn render(
        &self,
        _view: &AboutPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();
        let updates = ReleaseUpdates::as_ref(app);
        let update = ui_builder
            .button(ButtonVariant::Accent, self.update_button.clone())
            .with_text_label("Update".into());
        let update = if updates.checking {
            update.disabled()
        } else {
            update
        };
        let startup = Hoverable::new(self.startup_row.clone(), |_| {
            Flex::row()
                .with_spacing(8.)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(Shrinkable::new(1., ui_builder
                    .span("Check for updates on startup")
                    .with_soft_wrap()
                    .build().finish()).finish())
                .with_child(ui_builder
                    .switch(self.startup_switch.clone())
                    .check(*UpdateSettings::as_ref(app).check_on_startup.value())
                    .build().finish())
                .finish()
        })
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(AboutAction::ToggleStartup))
            .finish();
        let mut update_controls = Flex::column()
            .with_spacing(8.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);
        update_controls.add_child(
            update
                .build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(AboutAction::Update))
                .finish(),
        );
        if updates.download_url.is_some() && !updates.checking {
            update_controls.add_child(
                ui_builder
                    .button(ButtonVariant::Accent, self.download_button.clone())
                    .with_text_label("Download".into())
                    .build()
                    .on_click(|ctx, _, _| ctx.dispatch_typed_action(AboutAction::Download))
                    .finish(),
            );
        }
        update_controls.add_child(startup);
        if !updates.status.is_empty() {
            update_controls.add_child(
                ui_builder
                    .span(updates.status.clone())
                    .with_soft_wrap()
                    .build()
                    .finish(),
            );
            if updates.download_url.is_none()
                && !updates.checking
                && (updates.available.is_some() || updates.status.starts_with("Could not"))
            {
                update_controls.add_child(
                    ui_builder
                        .button(ButtonVariant::Text, self.releases_button.clone())
                        .with_text_label("GitHub Releases".into())
                        .build()
                        .on_click(|ctx, _, _| ctx.dispatch_typed_action(AboutAction::OpenReleases))
                        .finish(),
                );
            }
        }

        let version = ChannelState::app_version().unwrap_or("v#.##.###");

        let version_text = ui_builder
            .span(version.to_string())
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let copy_version_icon = appearance
            .ui_builder()
            .copy_button(16., self.copy_version_button_mouse_state.clone())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(version));
            })
            .finish();

        let version_row = Wrap::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_children([
                version_text,
                Container::new(copy_version_icon)
                    .with_margin_top(16.)
                    .with_padding_left(6.)
                    .finish(),
            ]);

        Align::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    ConstrainedBox::new(
                        Image::new(
                            AssetSource::Bundled {
                                path: "branding/warpai.png",
                            },
                            CacheOption::BySize,
                        )
                        .finish(),
                    )
                    .with_max_height(100.)
                    .with_max_width(350.)
                    .finish(),
                )
                .with_child(
                    ui_builder
                        .span("Warpai")
                        .build()
                        .with_margin_top(16.)
                        .finish(),
                )
                .with_child(version_row.finish())
                .with_child(
                    Container::new(update_controls.finish())
                        .with_margin_top(16.)
                        .finish(),
                )
                .with_child(
                    ui_builder
                        .span("Warpai by OthinusG · Based on Warp")
                        .with_soft_wrap()
                        .build()
                        .with_margin_top(16.)
                        .finish(),
                )
                .finish(),
        )
        .finish()
    }
}

impl SettingsPageMeta for AboutPageView {
    fn section() -> SettingsSection {
        SettingsSection::About
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<AboutPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AboutPageView>) -> Self {
        SettingsPageViewHandle::About(view_handle)
    }
}
