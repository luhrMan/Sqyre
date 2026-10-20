//! Chrome that stays as buttons when it fits and becomes a menu when it does not.

use eframe::egui;

/// Unwrapped width of `text` in `style`.
pub fn text_width(ui: &egui::Ui, text: &str, style: egui::TextStyle) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, style)
        .size()
        .x
}

/// One entry in an [`overflow_bar`].
#[derive(Clone, Copy, Debug)]
pub enum OverflowItem<'a> {
    /// Non-interactive caption (for example "Basic" / "Advanced").
    Caption(&'a str),
    /// Clickable section or tab.
    Choice { label: &'a str, selected: bool },
}

/// Selectable labels when they fit on one line; a combo menu when they do not.
///
/// Returns the index into `items` of a choice clicked this frame. Captions stay
/// in the slice (so indexes match the caller's tabs) but are omitted from the menu.
pub fn overflow_bar(
    ui: &mut egui::Ui,
    id: &str,
    hint: &str,
    items: &[OverflowItem<'_>],
) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let width_id = ui.make_persistent_id(id).with("bar_w");
    let remembered = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(width_id))
        .unwrap_or(0.0);
    let avail = ui.available_width().max(1.0);
    let estimate = items_width(ui, items);
    // `remembered` is the painted row from a pass that overflowed. Once the pane
    // is at least that wide, trust the estimate again so growing the window
    // restores the buttons.
    let too_wide = if remembered > 0.5 && avail + 0.5 < remembered {
        true
    } else {
        estimate > avail
    };
    if too_wide {
        return combo_bar(ui, id, hint, items, avail);
    }
    let inner = ui.horizontal(|ui| paint_items(ui, items));
    let w = inner.response.rect.width();
    ui.ctx().data_mut(|d| d.insert_temp(width_id, w));
    if w > avail + 0.5 {
        ui.ctx().request_discard("overflow bar");
    }
    inner.inner
}

fn items_width(ui: &egui::Ui, items: &[OverflowItem<'_>]) -> f32 {
    let gap = ui.spacing().item_spacing.x;
    let pad = ui.spacing().button_padding.x * 2.0 + 4.0;
    let mut w = 0.0;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            w += gap;
        }
        match item {
            OverflowItem::Caption(text) => {
                if i > 0 {
                    // `add_space` plus the gap the layout inserts before the caption.
                    w += crate::theme::SPACE_12 + gap;
                }
                w += text_width(ui, text, egui::TextStyle::Small);
            }
            OverflowItem::Choice { label, .. } => {
                w += text_width(ui, label, egui::TextStyle::Button) + pad;
            }
        }
    }
    w
}

fn selected_label<'a>(items: &[OverflowItem<'a>]) -> &'a str {
    items
        .iter()
        .find_map(|item| match item {
            OverflowItem::Choice {
                label,
                selected: true,
            } => Some(*label),
            _ => None,
        })
        .unwrap_or_else(|| {
            items
                .iter()
                .find_map(|item| match item {
                    OverflowItem::Choice { label, .. } => Some(*label),
                    _ => None,
                })
                .unwrap_or("…")
        })
}

fn combo_bar(
    ui: &mut egui::Ui,
    id: &str,
    hint: &str,
    items: &[OverflowItem<'_>],
    avail: f32,
) -> Option<usize> {
    let mut picked = None;
    let resp = egui::ComboBox::from_id_salt((id, "menu"))
        .selected_text(selected_label(items))
        .width(avail.clamp(1.0, 320.0))
        .show_ui(ui, |ui| {
            for (i, item) in items.iter().enumerate() {
                if let OverflowItem::Choice { label, selected } = item {
                    if ui.selectable_label(*selected, *label).clicked() {
                        picked = Some(i);
                    }
                }
            }
        });
    resp.response.on_hover_text(hint);
    picked
}

fn paint_items(ui: &mut egui::Ui, items: &[OverflowItem<'_>]) -> Option<usize> {
    let mut picked = None;
    for (i, item) in items.iter().enumerate() {
        match item {
            OverflowItem::Caption(text) => {
                if i > 0 {
                    ui.add_space(crate::theme::SPACE_12);
                }
                ui.label(egui::RichText::new(*text).weak().small());
            }
            OverflowItem::Choice { label, selected } => {
                if ui.selectable_label(*selected, *label).clicked() {
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

    fn section_items() -> [OverflowItem<'static>; 4] {
        [
            OverflowItem::Choice {
                label: "Programs",
                selected: true,
            },
            OverflowItem::Choice {
                label: "Items",
                selected: false,
            },
            OverflowItem::Choice {
                label: "Coordinates",
                selected: false,
            },
            OverflowItem::Choice {
                label: "Tools",
                selected: false,
            },
        ]
    }

    #[test]
    fn narrow_bar_becomes_a_menu() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(140.0, 320.0))
            .build_ui_state(
                |ui, picked: &mut Option<usize>| {
                    if let Some(i) =
                        overflow_bar(ui, "secs", "Data Editor section", &section_items())
                    {
                        *picked = Some(i);
                    }
                },
                None,
            );
        harness.run_steps(3);
        assert!(
            harness.query_by_role(Role::ComboBox).is_some(),
            "narrow section row should collapse to a menu"
        );
        assert!(
            harness.query_by_label("Tools").is_none(),
            "collapsed choices stay inside the menu"
        );
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.get_by_label("Tools").click();
        harness.run();
        assert_eq!(*harness.state(), Some(3));
    }

    #[test]
    fn wide_bar_keeps_selectable_labels() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 80.0))
            .build_ui(|ui| {
                overflow_bar(ui, "secs", "Data Editor section", &section_items());
            });
        harness.run_steps(2);
        assert!(harness.query_by_label("Tools").is_some());
        assert!(
            harness.query_by_role(Role::ComboBox).is_none(),
            "a row that fits stays as buttons"
        );
    }
}
