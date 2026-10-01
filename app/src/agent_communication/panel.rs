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
