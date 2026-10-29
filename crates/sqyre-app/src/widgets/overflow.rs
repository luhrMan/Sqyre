//! Button rows whose entries always paint — overflow is the caller's scroll.

use eframe::egui;

/// Unwrapped width of `text` in `style`.
pub fn text_width(ui: &egui::Ui, text: &str, style: egui::TextStyle) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, style)
        .size()
        .x
}

/// One entry in an [`always_bar`] row.
#[derive(Clone, Copy, Debug)]
pub enum OverflowItem<'a> {
    /// Non-interactive caption (for example "Basic" / "Advanced").
    Caption(&'a str),
    /// Visual divider between button sections.
    Separator,
    /// Clickable section or tab. Paints `icon` only — `label` stays the
    /// tooltip and the AccessKit name.
    Choice {
        label: &'a str,
        icon: &'a str,
        selected: bool,
    },
}

/// Paint every item into the current horizontal row — no collapse, no menu.
/// Callers that need off-pane content reachable wrap the row in
/// [`super::show_overflow`].
///
/// Returns the index into `items` of a choice clicked this frame. Captions and
/// separators stay in the slice so indexes match the caller's targets.
pub fn always_bar(ui: &mut egui::Ui, items: &[OverflowItem<'_>]) -> Option<usize> {
    paint_items(ui, items)
}

/// Icon-only tab face: the `selectable_label` selection chrome around a glyph.
/// `label` is the AccessKit name and hover text.
fn paint_icon_choice(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    selected: bool,
) -> egui::Response {
    let side = ui
        .text_style_height(&egui::TextStyle::Button)
        .max(crate::widgets::ICON_BTN_SIDE);
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(side), egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect(
        rect.expand(visuals.expansion),
        visuals.corner_radius,
        visuals.bg_fill,
        visuals.fg_stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::TextStyle::Button.resolve(ui.style()),
        visuals.fg_stroke.color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label)
    });
    response.on_hover_text(label)
}

fn paint_items(ui: &mut egui::Ui, items: &[OverflowItem<'_>]) -> Option<usize> {
    let mut picked = None;
    for (i, item) in items.iter().enumerate() {
        match item {
            OverflowItem::Caption(text) => {
                if i > 0 {
                    ui.add_space(crate::theme::element_gap(ui));
                }
                ui.label(egui::RichText::new(*text).weak().small());
            }
            OverflowItem::Separator => {
                crate::widgets::section_separator(ui);
            }
            OverflowItem::Choice {
                label,
                icon,
                selected,
            } => {
                if paint_icon_choice(ui, icon, label, *selected).clicked() {
                    picked = Some(i);
                }
            }
        }
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::kittest::Queryable;

    fn section_items() -> [OverflowItem<'static>; 7] {
        [
            OverflowItem::Choice {
                label: "Programs",
                icon: egui_phosphor::regular::APP_WINDOW,
                selected: true,
            },
            OverflowItem::Separator,
            OverflowItem::Choice {
                label: "Items",
                icon: egui_phosphor::regular::IMAGES,
                selected: false,
            },
            OverflowItem::Separator,
            OverflowItem::Caption("Advanced"),
            OverflowItem::Choice {
                label: "Coordinates",
                icon: egui_phosphor::regular::CROSSHAIR,
                selected: false,
            },
            OverflowItem::Choice {
                label: "Tools",
                icon: egui_phosphor::regular::WRENCH,
                selected: false,
            },
        ]
    }

    #[test]
    fn narrow_bar_keeps_every_button() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(80.0, 320.0))
            .build_ui_state(
                |ui, picked: &mut Option<usize>| {
                    ui.horizontal(|ui| {
                        if let Some(i) = always_bar(ui, &section_items()) {
                            *picked = Some(i);
                        }
                    });
                },
                None,
            );
        harness.run_steps(2);
        for label in ["Programs", "Items", "Coordinates", "Tools"] {
            assert!(
                harness.query_by_label(label).is_some(),
                "{label} stays a visible button at any width"
            );
        }
        assert!(
            harness.query_by_role(Role::ComboBox).is_none(),
            "buttons never collapse into a menu"
        );
    }

    #[test]
    fn click_reports_the_item_index() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 80.0))
            .build_ui_state(
                |ui, picked: &mut Option<usize>| {
                    ui.horizontal(|ui| {
                        if let Some(i) = always_bar(ui, &section_items()) {
                            *picked = Some(i);
                        }
                    });
                },
                None,
            );
        harness.run();
        harness.get_by_label("Tools").click();
        harness.run();
        assert_eq!(*harness.state(), Some(6));
    }
}
