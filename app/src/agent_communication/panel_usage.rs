//! A fixed footer with a separate four-row viewport; the main panel never scrolls it away.
use super::*;
use crate::agent_usage::{agent_icon, AgentUsage};
use warpui::elements::{ConstrainedBox, CornerRadius, Percentage, Radius, Rect, Stack};

impl CollaborationPanel {
    pub(super) fn render_usage(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let builder = appearance.ui_builder();
        let usage = AgentUsage::as_ref(app);
        let mut header = Flex::row()
            .with_spacing(GAP_ROW)
            .with_child(Shrinkable::new(1., heading(appearance, warpui::localization::text("Data usage"))).finish());
        for (index, icon, label, action) in [
            (
                0,
                crate::ui_components::icons::Icon::Settings,
                "Manage usage accounts",
                Action::UsageSettings,
            ),
            (
                1,
                crate::ui_components::icons::Icon::RefreshCw04,
                "Refresh usage",
                Action::RefreshUsage,
            ),
        ] {
            let tip = builder.tool_tip(label.to_owned()).build().finish();
            let button = crate::ui_components::buttons::icon_button(
                appearance,
                icon,
                false,
                self.usage_buttons[index].clone(),
            )
            .with_tooltip(move || tip);
            let button = if index == 1 && usage.checking {
                button.disabled()
            } else {
                button
            };
            header.add_child(
                button
                    .build()
                    .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                    .finish(),
            );
        }
        let mut rows = Flex::column();
        let accounts: Vec<_> = usage.accounts.iter().filter(|a| a.visible).collect();
        for account in &accounts {
            let reading = usage.readings.get(&account.id);
            let window = reading
                .and_then(|r| r.as_ref().ok())
                .and_then(|r| r.windows.iter().max_by(|a, b| a.used.total_cmp(&b.used)));
            let status = match reading {
                Some(Ok(r)) => {
                    if let Some(window) = window {
                        {
                            let __warpai_locale_argument_0 = &(window.name);
                            let __warpai_locale_argument_1 = &(window.used);
                            warpui::localization::format_text(
                                "{} · {:.0}% used",
                                &[
                                    ("0", format!("{__warpai_locale_argument_0}").as_str()),
                                    ("1:.0", format!("{__warpai_locale_argument_1:.0}").as_str()),
                                ],
                            )
                        }
                    } else {
                        {
                            let __warpai_locale_argument_0 = &(r.balance.unwrap_or_default());
                            warpui::localization::format_text(
                                "Balance ${:.2}",
                                &[("0:.2", format!("{__warpai_locale_argument_0:.2}").as_str())],
                            )
                        }
                    }
                }
                Some(Err(error)) => (*error).to_owned(),
                None => if usage.checking {
                    "Checking…"
                } else {
                    "Refresh to check usage"
                }
                .into(),
            };
            let mut tooltip = format!("{} · {}\n{status}", account.provider.name(), account.label);
            if let Some(Ok(reading)) = reading {
                for window in &reading.windows {
                    tooltip.push_str(&format!(
                        "\n{}: {:.1}% used{}",
                        window.name,
                        window.used,
                        window
                            .reset
                            .as_ref()
                            .map(|time| warpui::localization::format_text(" · resets {time}", &[("time", format!("{time}").as_str())]))
                            .unwrap_or_default()
                    ));
                }
            }
            if usage.checking && reading.is_some() {
                tooltip.push_str("\nRefreshing; showing the previous reading");
            }
            if reading.is_some_and(Result::is_ok)
                && usage
                    .updated
                    .is_some_and(|t| t.elapsed() > Duration::from_secs(360))
            {
                tooltip.push_str("\nReading is stale; refresh to update");
            }
            let label = builder
                .span(format!("{} · {}", account.provider.name(), account.label))
                .build()
                .finish();
            let row = Flex::row()
                .with_spacing(GAP_ROW)
                .with_child(
                    warpui::elements::ConstrainedBox::new(
                        agent_icon(account.provider.program())
                            .to_warpui_icon(theme.foreground())
                            .finish(),
                    )
                    .with_width(16.)
                    .with_height(16.)
                    .finish(),
                )
                .with_child(Shrinkable::new(1., label).finish());
            let mut content = Flex::column().with_spacing(4.).with_child(row.finish());
            content.add_child(
                builder
                    .span(status)
                    .with_style(UiComponentStyles {
                        font_size: Some(12.),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
            );
            if let Some(window) = window {
                content.add_child(
                    ConstrainedBox::new(
                        Stack::new()
                            .with_child(Rect::new().with_background(theme.outline()).finish())
                            .with_child(
                                Percentage::width(
                                    window.used as f32 / 100.,
                                    Rect::new().with_background(theme.foreground()).finish(),
                                )
                                .finish(),
                            )
                            .finish(),
                    )
                    .with_height(4.)
                    .finish(),
                );
            }
            let state = self
                .usage_rows
                .borrow_mut()
                .entry(account.id.clone())
                .or_default()
                .clone();
            rows.add_child(
                builder.tool_tip_on_element(
                    tooltip,
                    state,
                    ConstrainedBox::new(content.finish())
                        .with_height(64.)
                        .finish(),
                    warpui::elements::ParentAnchor::TopLeft,
                    warpui::elements::ChildAnchor::BottomLeft,
                    pathfinder_geometry::vector::vec2f(0., -4.),
                ),
            );
        }
        if accounts.is_empty() {
            rows.add_child(note(appearance, warpui::localization::text("Connect accounts in Settings")));
        }
        let scroll = ClippedScrollable::vertical(
            self.usage_scroll.clone(),
            rows.finish(),
            ScrollbarWidth::Auto,
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .with_overlayed_scrollbar()
        .finish();
        Container::new(
            Flex::column()
                .with_spacing(GAP_ROW)
                .with_child(header.finish())
                .with_child(
                    ConstrainedBox::new(scroll)
                        .with_height(if accounts.is_empty() {
                            24.
                        } else {
                            accounts.len().min(4) as f32 * 64.
                        })
                        .finish(),
                )
                .finish(),
        )
        .with_padding(Padding::uniform(GAP_ROW))
        .with_border(Border::all(1.).with_border_fill(theme.outline()))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
        .finish()
    }
}
