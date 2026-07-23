//! Left macro list panel and delete confirmation.

use crate::pickers;
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::Macro;
use sqyre_hotkeys::format_hotkey;

/// Elide `text` to a single line that fits `max_width`, appending `…` only when needed.
fn elide_to_width(ui: &egui::Ui, text: &str, max_width: f32, font_id: egui::FontId) -> String {
    if text.is_empty() {
        return String::new();
    }
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font_id.clone(), egui::Color32::WHITE);
    if full.size().x <= max_width {
        return text.to_owned();
    }

    const ELLIPSIS: char = '…';
    let ellipsis_w = ui
        .painter()
        .layout_no_wrap(ELLIPSIS.to_string(), font_id.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let budget = (max_width - ellipsis_w).max(0.0);
    if budget <= 0.0 {
        return ELLIPSIS.to_string();
    }

    let char_count = text.chars().count();
    let mut lo = 0usize;
    let mut hi = char_count;
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = text.chars().take(mid).collect();
        let w = ui
            .painter()
            .layout_no_wrap(candidate, font_id.clone(), egui::Color32::WHITE)
            .size()
            .x;
        if w <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = text.chars().take(lo).collect();
    out.push(ELLIPSIS);
    out
}

/// Macro name on the first line; hotkey as a weak small hint below when set.
/// Each line is shown in full when it fits `max_text_width`, otherwise elided with `…`.
fn macro_list_item_text(ui: &egui::Ui, m: &Macro, max_text_width: f32) -> egui::WidgetText {
    let style = ui.style();
    let name_font = egui::FontSelection::Default.resolve(style);
    let name = elide_to_width(ui, &m.name, max_text_width, name_font);

    if m.hotkey.is_empty() {
        return name.into();
    }

    let hotkey_font = egui::TextStyle::Small.resolve(style);
    let hotkey = elide_to_width(ui, &format_hotkey(&m.hotkey), max_text_width, hotkey_font);

    let mut job = egui::text::LayoutJob::default();
    egui::RichText::new(name)
        .color(style.visuals.text_color())
        .append_to(
            &mut job,
            style,
            egui::FontSelection::Default,
            egui::Align::LEFT,
        );
    egui::RichText::new(format!("\n{hotkey}"))
        .small()
        .color(style.visuals.weak_text_color())
        .append_to(
            &mut job,
            style,
            egui::FontSelection::Default,
            egui::Align::LEFT,
        );
    job.into()
}

pub fn show(app: &mut SqyreApp, ui: &mut egui::Ui) {
    egui::Panel::left("macro_list")
        .default_size(220.0)
        .show_animated_inside(ui, app.macro_list_open, |ui| {
            ui.heading("Macros");
            if let Some(err) = &app.load_error {
                ui.colored_label(crate::theme::error_fg(), format!("Load error: {err}"));
            } else {
                #[cfg(target_arch = "wasm32")]
                ui.small(format!(
                    "{} (browser — import/export db.yaml)",
                    app.macros.len()
                ));
                #[cfg(not(target_arch = "wasm32"))]
                ui.small(format!(
                    "{} from {}",
                    app.macros.len(),
                    sqyre_persist::db_path().display()
                ));
            }
            if let Some(err) = &app.save_error {
                ui.colored_label(crate::theme::error_fg(), format!("Save error: {err}"));
            }
            ui.horizontal(|ui| {
                // Use ASCII / NotoEmoji glyphs only — fullwidth/math symbols
                // (＋, ⧉) render as tofu in egui's default font stack.
                let new_resp =
                    crate::theme::icon_button_colored(ui, "+", Some(crate::theme::MACRO_START));
                new_resp.clone().on_hover_text("New macro");
                // AccessKit label for interaction tests (default label is just "+").
                new_resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "New macro")
                });
                if new_resp.clicked() {
                    app.create_macro();
                }
                let has_sel = !app.macros.is_empty();
                if ui
                    .add_enabled_ui(has_sel, |ui| crate::theme::icon_button(ui, "📄"))
                    .inner
                    .on_hover_text("Duplicate selected macro")
                    .clicked()
                {
                    app.duplicate_selected_macro();
                }
                if ui
                    .add_enabled_ui(has_sel, |ui| {
                        crate::theme::icon_button_colored(ui, "🗑", Some(crate::theme::MACRO_STOP))
                    })
                    .inner
                    .on_hover_text("Delete selected macro")
                    .clicked()
                {
                    let idx = app.selected_macro.min(app.macros.len() - 1);
                    app.pending_delete_macro = Some(app.macros[idx].name.clone());
                }
            });
            ui.add(
                egui::TextEdit::singleline(&mut app.macro_list_filter)
                    .desired_width(f32::INFINITY)
                    .hint_text("Search macros or tags…"),
            )
            .on_hover_text("Filter by macro name or tag.");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let width = ui.available_width();
                let text_width = (width - ui.spacing().button_padding.x * 2.0).max(0.0);
                let filter = app.macro_list_filter.trim().to_string();
                for (i, m) in app.macros.iter().enumerate() {
                    if !pickers::query_matches_name_or_tags(&filter, &m.name, &m.tags) {
                        continue;
                    }
                    let label = macro_list_item_text(ui, m, text_width);
                    if ui
                        .add(
                            egui::Button::selectable(app.selected_macro == i, label)
                                .wrap_mode(egui::TextWrapMode::Extend)
                                .min_size(egui::vec2(width, 0.0)),
                        )
                        .clicked()
                    {
                        app.selected_macro = i;
                        app.selected_action = None;
                        app.tooltip.cancel();
                    }
                }
            });
        });

    if let Some(name) = app.pending_delete_macro.clone() {
        let mut open = true;
        egui::Window::new("Delete Macro")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Delete macro \"{name}\"?"));
                let mut outcome = crate::widgets::ConfirmCancel::None;
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        outcome = crate::widgets::ConfirmCancel::Cancel;
                    }
                    if ui
                        .button(egui::RichText::new("Delete").color(crate::theme::MACRO_STOP))
                        .clicked()
                    {
                        outcome = crate::widgets::ConfirmCancel::Confirm;
                    }
                });
                if outcome == crate::widgets::ConfirmCancel::None {
                    outcome = crate::widgets::poll_confirm_keys(ui);
                }
                match outcome {
                    crate::widgets::ConfirmCancel::Cancel => {
                        app.pending_delete_macro = None;
                    }
                    crate::widgets::ConfirmCancel::Confirm => {
                        app.pending_delete_macro = None;
                        app.delete_macro_named(&name);
                    }
                    crate::widgets::ConfirmCancel::None => {}
                }
            });
        if !open {
            app.pending_delete_macro = None;
        }
    }
}
