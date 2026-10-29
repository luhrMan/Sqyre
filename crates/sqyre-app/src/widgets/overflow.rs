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
    /// Clickable section or tab. Paints `icon` only — `label` stays the
    /// tooltip, the AccessKit name, and the text inside the combo fallback.
    Choice {
        label: &'a str,
        icon: &'a str,
        selected: bool,
    },
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
    let mut w = 0.0;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            w += gap;
        }
        match item {
            OverflowItem::Caption(text) => {
                if i > 0 {
                    // `add_space` plus the gap the layout inserts before the caption.
                    w += crate::theme::element_gap(ui) + gap;
                }
                w += text_width(ui, text, egui::TextStyle::Small);
            }
            OverflowItem::Choice { .. } => {
                w += ui
                    .text_style_height(&egui::TextStyle::Button)
                    .max(crate::widgets::ICON_BTN_SIDE)
                    + 4.0;
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
                ..
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
    let selected = selected_label(items);
    let resp = egui::ComboBox::from_id_salt((id, "menu"))
        .selected_text(selected)
        .truncate()
        .width(avail.clamp(1.0, 320.0))
        .show_ui(ui, |ui| {
            for (i, item) in items.iter().enumerate() {
                if let OverflowItem::Choice {
                    label, selected, ..
                } = item
                {
                    if ui.selectable_label(*selected, *label).clicked() {
                        picked = Some(i);
                    }
                }
            }
        })
        .response;
    if crate::widgets::elide::combo_text_elided(ui, &resp, selected) {
        resp.on_hover_text(format!("{selected}\n\n{hint}"));
    } else {
        resp.on_hover_text(hint);
    }
    picked
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

    fn section_items() -> [OverflowItem<'static>; 4] {
        [
            OverflowItem::Choice {
                label: "Programs",
                icon: egui_phosphor::regular::APP_WINDOW,
                selected: true,
            },
            OverflowItem::Choice {
                label: "Items",
                icon: egui_phosphor::regular::IMAGES,
                selected: false,
            },
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
    fn narrow_bar_becomes_a_menu() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(80.0, 320.0))
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
