//! Native static checkpoint. Live controller wiring follows screenshot acceptance.
use crate::appearance::Appearance;
use serde::Deserialize;
use warpui::{
    elements::{ClippedScrollStateHandle, ClippedScrollable, Container, Element, Fill, Flex,
        MouseStateHandle, Padding, ParentElement, ScrollbarWidth},
    ui_components::{button::ButtonVariant, components::UiComponent},
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

pub(crate) struct CollaborationPanel {
    fixtures: Vec<Fixture>,
    selected: usize,
    next: MouseStateHandle,
    scroll: ClippedScrollStateHandle,
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    NextFixture,
}

impl CollaborationPanel {
    pub(crate) fn new(_: &mut ViewContext<Self>) -> Self {
        Self {
            fixtures: serde_json::from_str(include_str!("../../../specs/agent-communication-v2/panel-fixtures.json"))
                .expect("Validated collaboration fixtures"),
            selected: 0,
            next: Default::default(),
            scroll: Default::default(),
        }
    }
}

impl Entity for CollaborationPanel {
    type Event = ();
}
impl TypedActionView for CollaborationPanel {
    type Action = Action;
    fn handle_action(&mut self, action: &Action, ctx: &mut ViewContext<Self>) {
        match action {
            Action::NextFixture => {
                self.selected = (self.selected + 1) % self.fixtures.len();
                self.scroll = Default::default();
                ctx.notify();
            }
        }
    }
}
impl View for CollaborationPanel {
    fn ui_name() -> &'static str {
        "AgentCollaboration"
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let theme = appearance.theme();
        let fixture = &self.fixtures[self.selected];
        let mut body = Flex::column().with_spacing(12.);
        body.add_child(builder.span("Agent collaboration").with_soft_wrap().build().finish());
        body.add_child(builder.span(format!("Design preview — sample data · {}", fixture.state))
            .with_soft_wrap().build().finish());
        body.add_child(builder.span(fixture.guidance.clone()).with_soft_wrap().build().finish());
        body.add_child(builder.button(ButtonVariant::Text, self.next.clone())
            .with_text_label("Next preview state".to_owned()).build()
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::NextFixture)).finish());
        for section in &fixture.sections {
            body.add_child(builder.span(section.title.clone()).with_soft_wrap().build().finish());
            for row in &section.rows {
                body.add_child(Container::new(builder.span(row.clone()).with_soft_wrap()
                    .with_selectable(true).build().finish()).with_padding_left(8.).finish());
            }
        }
        Container::new(ClippedScrollable::vertical(self.scroll.clone(), body.finish(),
            ScrollbarWidth::Auto, theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(), Fill::None).with_overlayed_scrollbar().finish())
            .with_padding(Padding::uniform(12.)).finish()
    }
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
            Box::new(|app, window| {
                warpui::async_assert!(app.root_view::<RootView>(window).is_some_and(|root| {
                    root.read(app, |root, _| root.workspace_view().is_some())
                }))
            }),
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
            Box::new(|app, window| {
                warpui::async_assert!(app.views_of_type::<LeftPanelView>(window).is_some())
            }),
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
            Box::new(|app, window| {
                warpui::async_assert!(app.views_of_type::<CollaborationPanel>(window).is_some())
            }),
        ));
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
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!("../../../specs/agent-communication-v2/panel-fixtures.json")).unwrap();
        for state in ["agents", "tasks", "detail", "empty", "loading", "disconnected", "stale", "capacity", "failed"] {
            let fixture = fixtures.iter().find(|fixture| fixture.state == state).unwrap();
            assert!(!fixture.guidance.is_empty());
        }
        assert!(fixtures.iter().flat_map(|fixture| &fixture.sections)
            .flat_map(|section| &section.rows).any(|row| row.len() > 256));
    }
}
