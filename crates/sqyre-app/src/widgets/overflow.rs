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
    /// Visual divider between button sections.
    Separator,
    /// Clickable section or tab. Paints glyph + visible `label` (P1-2);
    /// AccessKit name matches `label`.
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
/// Returns the index into `items` of a choice clicked this frame. Separators
/// stay in the slice so indexes match the caller's targets.
pub fn always_bar(ui: &mut egui::Ui, items: &[OverflowItem<'_>]) -> Option<usize> {
    paint_items(ui, items)
}

/// Icon + short text tab face with selectable chrome.
fn paint_labeled_choice(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    selected: bool,
) -> egui::Response {
    let pad_x = crate::theme::SPACE_4;
    let gap = crate::theme::SPACE_4;
    let side = crate::widgets::icon_btn_side(ui);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let text_w = text_width(ui, label, egui::TextStyle::Small);
    let desired = egui::vec2(
        pad_x * 2.0 + side + gap + text_w,
        side.max(ui.spacing().interact_size.y),
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect(
        rect.expand(visuals.expansion),
        visuals.corner_radius,
        visuals.bg_fill,
        visuals.fg_stroke,
        egui::StrokeKind::Inside,
    );
    let icon_center = egui::pos2(rect.left() + pad_x + side * 0.5, rect.center().y);
    ui.painter().text(
        icon_center,
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::TextStyle::Button.resolve(ui.style()),
        visuals.fg_stroke.color,
    );
    ui.painter().text(
        egui::pos2(rect.left() + pad_x + side + gap, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        visuals.fg_stroke.color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label)
    });
    response
}

fn paint_items(ui: &mut egui::Ui, items: &[OverflowItem<'_>]) -> Option<usize> {
    let mut picked = None;
    for (i, item) in items.iter().enumerate() {
        match item {
            OverflowItem::Separator => {
                crate::widgets::section_separator(ui);
            }
            OverflowItem::Choice {
                label,
                icon,
                selected,
            } => {
                if paint_labeled_choice(ui, icon, label, *selected).clicked() {
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

    fn section_items() -> [OverflowItem<'static>; 6] {
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
    fn choice_paints_visible_label_text() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.horizontal(|ui| {
                always_bar(ui, &section_items());
            });
        });
        let painted = |needle: &str| {
            out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => text.galley.text() == needle,
                _ => false,
            })
        };
        for label in ["Programs", "Items", "Coordinates", "Tools"] {
            assert!(painted(label), "{label} must be painted at rest");
        }
        out.drop_without_applying_deltas();
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
        assert_eq!(*harness.state(), Some(5));
    }
}
