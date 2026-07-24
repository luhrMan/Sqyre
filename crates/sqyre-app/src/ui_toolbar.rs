//! Central-panel toolbars: brand header, run/stop strip, hotkey row, action chrome.

use crate::macro_meta::collect_all_macro_tags;
use crate::theme;
use crate::SqyreApp;
use eframe::egui::{self, Color32, Vec2};
use sqyre_hotkeys::{format_hotkey, HotkeyTrigger};
use sqyre_ui_model::action_pastel_color;
use std::sync::atomic::Ordering;

/// Compact toolbar control: icon glyph + hover label.
pub fn toolbar_icon(ui: &mut egui::Ui, glyph: &str, tip: &str, enabled: bool) -> egui::Response {
    toolbar_icon_colored(ui, glyph, tip, enabled, None)
}

/// Compact toolbar control with an optional fixed glyph color.
fn toolbar_icon_colored(
    ui: &mut egui::Ui,
    glyph: &str,
    tip: &str,
    enabled: bool,
    color: Option<Color32>,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| theme::icon_button_colored(ui, glyph, color))
        .inner
        .on_hover_text(tip)
}

pub fn brand_header(app: &mut SqyreApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let tex = app.icon_cache.sqyre_fallback(ui.ctx());
        let size = egui::vec2(28.0, 28.0);
        let image = egui::Image::new((tex.id(), size))
            .fit_to_exact_size(size)
            .maintain_aspect_ratio(true);
        let button = egui::Button::image_and_text(image, egui::RichText::new("Sqyre").heading())
            .frame_when_inactive(false);

        let (response, _) = egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
            if ui.button("📁  Data Editor").clicked() {
                app.data_editor.open = true;
                ui.close();
            }
            if ui.button("Variables").clicked() {
                app.variables_panel.open = true;
                ui.close();
            }
            if ui.button("⚙  Settings").clicked() {
                app.settings_ui.open = true;
                ui.close();
            }
            ui.separator();
            let list_label = if app.macro_list_open {
                "◁  Hide Macro List"
            } else {
                "☰  Show Macro List"
            };
            if ui.button(list_label).clicked() {
                app.macro_list_open = !app.macro_list_open;
                ui.close();
            }
        });
        response.on_hover_text("App menu");

        #[cfg(not(target_arch = "wasm32"))]
        show_update_banner(app, ui);
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn show_update_banner(app: &mut SqyreApp, ui: &mut egui::Ui) {
    use crate::update::UpdateState;

    if !app.update.show_banner() {
        if let UpdateState::Ready { version } = &app.update.state {
            let version = version.clone();
            ui.horizontal(|ui| {
                ui.colored_label(
                    theme::ok_fg(),
                    format!("v{version} installed — restart to finish"),
                );
                if ui.small_button("Restart").clicked() {
                    crate::update::restart_app(&mut app.instance_lock);
                }
            });
        }
        return;
    }
    let version = app.update.available_version().unwrap_or("?").to_string();
    ui.horizontal(|ui| {
        ui.colored_label(theme::ok_fg(), format!("Update available: v{version}"));
        if ui.small_button("Download & install").clicked() {
            app.update.start_download();
        }
        if ui.small_button("Dismiss").clicked() {
            app.update.dismiss_banner();
        }
    });
}

pub fn main_toolbar(app: &mut SqyreApp, ui: &mut egui::Ui) {
    #[cfg(not(target_arch = "wasm32"))]
    let running = app.run.running.load(Ordering::SeqCst);
    ui.horizontal(|ui| {
        // Half the default gap between toolbar icon buttons.
        ui.spacing_mut().item_spacing.x *= 0.5;
        let (list_glyph, list_tip) = if app.macro_list_open {
            ("◁", "Hide macro list")
        } else {
            ("☰", "Show macro list")
        };
        if toolbar_icon(ui, list_glyph, list_tip, true).clicked() {
            app.macro_list_open = !app.macro_list_open;
        }
        ui.separator();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if toolbar_icon_colored(
                ui,
                "▶",
                "Run",
                !running && !app.macros.is_empty(),
                Some(theme::MACRO_START),
            )
            .clicked()
            {
                app.start_macro(ui.ctx());
            }
            if toolbar_icon_colored(ui, "⏹", "Stop", running, Some(theme::MACRO_STOP)).clicked() {
                app.request_stop();
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if toolbar_icon(ui, "⬇", "Import db.yaml", true).clicked() {
                app.request_db_import();
            }
            if toolbar_icon(ui, "⬆", "Export db.yaml", true).clicked() {
                app.export_db_yaml();
            }
        }
        if toolbar_icon(ui, "📁", "Data Editor", true).clicked() {
            app.data_editor.open = true;
        }

        let status = app.run.status.lock().clone();
        let right_w = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(right_w, ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if toolbar_icon(ui, "⚙", "Settings", true).clicked() {
                    app.settings_ui.open = true;
                }
                if !status.is_empty() {
                    ui.label(status);
                }
            },
        );
    });
    #[cfg(not(target_arch = "wasm32"))]
    ui.small("Esc stops the running macro; Esc+Ctrl+Shift exits (failsafe). Macro hotkeys launch from anywhere.");
    #[cfg(target_arch = "wasm32")]
    ui.small(
        "Browser editor: import/export db.yaml. Run, capture, and global hotkeys are desktop-only.",
    );
    ui.separator();
}

/// Macro name/tags editor and global hotkey controls for the selected macro.
/// Returns `false` if macros became empty (caller should stop drawing the editor).
pub fn show_meta_and_hotkey(app: &mut SqyreApp, ui: &mut egui::Ui) -> bool {
    let running = app.run.running.load(Ordering::SeqCst);
    let idx = app.selected_macro.min(app.macros.len() - 1);
    app.selected_macro = idx;
    let meta_enabled = !running;
    app.macro_meta.sync_selection(idx, &app.macros[idx]);
    let other_names: Vec<String> = app.macros.iter().map(|m| m.name.clone()).collect();
    let all_tags = collect_all_macro_tags(&app.macros);
    let meta = ui
        .horizontal(|ui| {
            let row = {
                let m = &mut app.macros[idx];
                app.macro_meta
                    .paint_name_row(ui, m, &other_names, meta_enabled)
            };
            ui.separator();
            paint_hotkey_controls(app, ui, idx, running);
            row
        })
        .inner;
    if let Some(new_name) = meta.rename_to {
        app.rename_selected_macro(new_name);
    }
    let persist_tags = {
        let m = &mut app.macros[idx];
        app.macro_meta
            .paint_tags_row(ui, m, &all_tags, meta_enabled)
    };
    if persist_tags {
        app.persist_macro_at(idx);
    }
    {
        let m = &mut app.macros[idx];
        let delay_out = app.macro_meta.paint_delay_popup(ui, m, meta_enabled);
        if delay_out.persist {
            app.persist_macro_at(idx);
        }
    }
    // Selection / length may have changed after rename.
    let idx = app.selected_macro.min(app.macros.len().saturating_sub(1));
    app.selected_macro = idx;
    if app.macros.is_empty() {
        return false;
    }

    ui.separator();
    true
}

fn paint_hotkey_controls(app: &mut SqyreApp, ui: &mut egui::Ui, idx: usize, running: bool) {
    ui.label("Hotkey:");
    let hk_label = {
        let m = &app.macros[idx];
        if m.hotkey.is_empty() {
            "—".to_string()
        } else {
            format_hotkey(&m.hotkey)
        }
    };
    ui.monospace(&hk_label);
    if theme::record_icon_button(ui, "Record a global hotkey chord", !running).clicked() {
        app.hotkey_record.open(&app.macro_hotkeys);
    }
    if toolbar_icon(
        ui,
        egui_phosphor::regular::ERASER,
        crate::action_tooltip::help::META_HOTKEY_CLEAR,
        !running && !app.macros[idx].hotkey.is_empty(),
    )
    .clicked()
    {
        app.apply_hotkey_to_selected(Vec::new(), None);
    }

    let mut trigger = HotkeyTrigger::parse(&app.macros[idx].hotkey_trigger);
    let mut trigger_changed = false;
    if ui
        .selectable_label(trigger == HotkeyTrigger::Press, "On press")
        .on_hover_text(crate::action_tooltip::help::META_HOTKEY_PRESS)
        .clicked()
    {
        trigger = HotkeyTrigger::Press;
        trigger_changed = true;
    }
    if ui
        .selectable_label(trigger == HotkeyTrigger::Release, "On release")
        .on_hover_text(crate::action_tooltip::help::META_HOTKEY_RELEASE)
        .clicked()
    {
        trigger = HotkeyTrigger::Release;
        trigger_changed = true;
    }
    if trigger_changed {
        let chord = app.macros[idx].hotkey.clone();
        app.apply_hotkey_to_selected(chord, Some(trigger));
    }
}

/// Action chrome (add/vars/clipboard/history/expand). Returns expand/collapse force.
pub fn action_toolbar(app: &mut SqyreApp, ui: &mut egui::Ui) -> Option<bool> {
    let running = app.run.running.load(Ordering::SeqCst);
    let mut force_openness: Option<bool> = None;
    ui.horizontal(|ui| {
        // Half the default gap between toolbar icon buttons.
        ui.spacing_mut().item_spacing.x *= 0.5;
        let can_copy = app.can_copy_selection();
        let can_paste = app.can_paste_clipboard();
        let can_undo = app.can_undo();
        let can_redo = app.can_redo();
        if toolbar_icon_colored(
            ui,
            "+",
            "Add Action (Ctrl+A)",
            !running,
            Some(theme::MACRO_START),
        )
        .clicked()
        {
            app.add_action_picker.open();
        }
        // Light-theme variables pastel reads better as a glyph on dark chrome.
        let vars_color = theme::rgba(action_pastel_color("setvariable", false));
        if toolbar_icon_colored(ui, "x", "Variables", true, Some(vars_color)).clicked() {
            app.variables_panel.open = true;
        }
        ui.separator();
        if toolbar_icon(ui, "📄", "Copy (Ctrl+C)", can_copy && !running).clicked() {
            app.copy_selection(ui.ctx());
        }
        if toolbar_icon(ui, "✂", "Cut (Ctrl+X)", can_copy && !running).clicked() {
            app.cut_selection(ui.ctx());
        }
        if toolbar_icon(ui, "📋", "Paste (Ctrl+V)", can_paste && !running).clicked() {
            app.paste_clipboard();
        }
        if toolbar_icon(ui, "↺", "Undo (Ctrl+Z)", can_undo && !running).clicked() {
            app.undo_tree();
        }
        if toolbar_icon(ui, "↻", "Redo (Ctrl+Y)", can_redo && !running).clicked() {
            app.redo_tree();
        }
        if toolbar_icon(
            ui,
            egui_phosphor::regular::TREE_VIEW,
            "Expand all branches",
            true,
        )
        .clicked()
        {
            force_openness = Some(true);
        }
        if toolbar_icon(
            ui,
            egui_phosphor::regular::SQUARE_SPLIT_VERTICAL,
            "Collapse all branches",
            true,
        )
        .clicked()
        {
            force_openness = Some(false);
        }
    });
    ui.add_space(4.0);
    force_openness
}
