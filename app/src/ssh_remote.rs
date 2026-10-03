//! Native connection/project preview; live IO waits for the screenshot gate.
use crate::appearance::Appearance;
use serde::Deserialize;
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
pub(crate) struct Fixture {
    pub(crate) state: String,
    guidance: String,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
struct Section {
    title: String,
    rows: Vec<String>,
}

pub(crate) fn fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!(
        "../../specs/agent-communication-v2/connection-fixtures.json"
    ))
    .expect("Validated connection fixtures")
}

pub(crate) struct RemoteProjectsView {
    pub(crate) selected: usize,
    pub(crate) scroll: ClippedScrollStateHandle,
    fixtures: Vec<Fixture>,
    next: MouseStateHandle,
}
#[derive(Clone)]
pub(crate) enum Action {
    Next,
    Scroll(f32),
    Exit,
}

impl RemoteProjectsView {
    pub(crate) fn new(_: &mut ViewContext<Self>) -> Self {
        Self {
            selected: 0,
            scroll: Default::default(),
            fixtures: fixtures(),
            next: Default::default(),
        }
    }
}
impl Entity for RemoteProjectsView {
    type Event = ();
}
impl TypedActionView for RemoteProjectsView {
    type Action = Action;
    fn handle_action(&mut self, action: &Action, ctx: &mut ViewContext<Self>) {
        match action {
            Action::Next => {
                self.selected = (self.selected + 1) % self.fixtures.len();
                self.scroll = Default::default();
            }
            Action::Scroll(delta) => self.scroll.scroll_by((*delta).into_pixels()),
            Action::Exit => {
                ctx.dispatch_typed_action(&crate::workspace::WorkspaceAction::FocusLeftPanel);
                return;
            }
        }
        ctx.notify();
    }
    fn action_accessibility_contents(
        &mut self,
        _: &Action,
        ctx: &mut ViewContext<Self>,
    ) -> ActionAccessibilityContent {
        self.accessibility_contents(ctx).into()
    }
}
impl View for RemoteProjectsView {
    fn ui_name() -> &'static str {
        "RemoteProjects"
    }
    fn accessibility_contents(&self, _: &AppContext) -> Option<AccessibilityContent> {
        let fixture = &self.fixtures[self.selected];
        Some(AccessibilityContent::new(format!("Connections and projects, sample data. {}. {}", fixture.state, fixture.guidance),
            "Enter changes preview state. Page Up and Page Down scroll. Escape returns to terminal.", WarpA11yRole::ScrollareaRole))
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let builder = appearance.ui_builder();
        let theme = appearance.theme();
        let fixture = &self.fixtures[self.selected];
        let mut header = Flex::column().with_spacing(12.);
        for text in [
            "Connections / Projects".to_owned(),
            format!("Design preview — sample data · {}", fixture.state),
            fixture.guidance.clone(),
        ] {
            header.add_child(builder.span(text).with_soft_wrap().build().finish());
        }
        header.add_child(
            builder
                .button(ButtonVariant::Text, self.next.clone())
                .with_text_label("Next preview state".into())
                .build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(Action::Next))
                .finish(),
        );
        let mut body = Flex::column().with_spacing(12.);
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
                    .with_child(Shrinkable::new(1., scroll).finish())
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
                "enter" | "right" => Action::Next,
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
