//! Central-panel toolbars: brand header, run/stop strip, hotkey row, action chrome.

use crate::macro_meta::collect_all_macro_tags;
use crate::theme;
use crate::SqyreApp;
use eframe::egui;
use sqyre_hotkeys::{format_hotkey, HotkeyTrigger};
use std::sync::atomic::Ordering;

/// Compact toolbar control: icon glyph + hover label.
pub fn toolbar_icon(ui: &mut egui::Ui, glyph: &str, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(glyph).size(16.0)),
    )
    .on_hover_text(tip)
}

pub fn brand_header(app: &mut SqyreApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let tex = app.icon_cache.sqyre_fallback(ui.ctx());
        let size = egui::vec2(28.0, 28.0);
        ui.add(
            egui::Image::new((tex.id(), size))
                .fit_to_exact_size(size)
                .maintain_aspect_ratio(true),
        );
        ui.heading("Sqyre");
    });
}

pub fn main_toolbar(app: &mut SqyreApp, ui: &mut egui::Ui) {
    let running = app.run.running.load(Ordering::SeqCst);
    ui.horizontal(|ui| {
        let (list_glyph, list_tip) = if app.macro_list_open {
            ("◁", "Hide macro list")
        } else {
            ("☰", "Show macro list")
        };
        if toolbar_icon(ui, list_glyph, list_tip, true).clicked() {
            app.macro_list_open = !app.macro_list_open;
        }
        ui.separator();
        if toolbar_icon(ui, "▶", "Run", !running && !app.macros.is_empty()).clicked() {
            app.start_macro(ui.ctx());
        }
        if toolbar_icon(ui, "⏹", "Stop", running).clicked() {
            app.request_stop();
        }
        if toolbar_icon(ui, "📁", "Data Editor", true).clicked() {
            app.data_editor.open = true;
        }
        if toolbar_icon(ui, "⚙", "Settings", true).clicked() {
            app.settings_ui.open = true;
        }
        let status = app.run.status.lock().clone();
        if !status.is_empty() {
            ui.label(status);
        }
    });
    ui.small("Esc stops the running macro; Esc+Ctrl+Shift exits (failsafe). Macro hotkeys launch from anywhere.");
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
    let meta = {
        let m = &mut app.macros[idx];
        app.macro_meta
            .show(ui, m, &other_names, &all_tags, meta_enabled)
    };
    if let Some(new_name) = meta.rename_to {
        app.rename_selected_macro(new_name);
    } else if meta.persist {
        app.persist_macro_at(idx);
    }
    // Selection / length may have changed after rename.
    let idx = app.selected_macro.min(app.macros.len().saturating_sub(1));
    app.selected_macro = idx;
    if app.macros.is_empty() {
        return false;
    }

    ui.horizontal(|ui| {
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

        let mut trigger = HotkeyTrigger::parse(&app.macros[idx].hotkey_trigger);
        let mut trigger_changed = false;
        if ui
            .selectable_label(trigger == HotkeyTrigger::Press, "On press")
            .clicked()
        {
            trigger = HotkeyTrigger::Press;
            trigger_changed = true;
        }
        if ui
            .selectable_label(trigger == HotkeyTrigger::Release, "On release")
            .clicked()
        {
            trigger = HotkeyTrigger::Release;
            trigger_changed = true;
        }
        if trigger_changed {
            let chord = app.macros[idx].hotkey.clone();
            app.apply_hotkey_to_selected(chord, Some(trigger));
        }

        if theme::record_icon_button(ui, "Record a global hotkey chord", !running).clicked() {
            app.hotkey_record.open(&app.macro_hotkeys);
        }
        if ui
            .add_enabled(
                !running && !app.macros[idx].hotkey.is_empty(),
                egui::Button::new("Clear"),
            )
            .clicked()
        {
            app.apply_hotkey_to_selected(Vec::new(), None);
        }
    });
    ui.separator();
    true
}

/// Action chrome (add/vars/clipboard/history/expand). Returns expand/collapse force.
pub fn action_toolbar(app: &mut SqyreApp, ui: &mut egui::Ui) -> Option<bool> {
    let running = app.run.running.load(Ordering::SeqCst);
    let mut force_openness: Option<bool> = None;
    ui.horizontal(|ui| {
        let can_copy = app.can_copy_selection();
        let can_paste = app.can_paste_clipboard();
        let can_undo = app.can_undo();
        let can_redo = app.can_redo();
        if toolbar_icon(ui, "+", "Add Action (Ctrl+A)", !running).clicked() {
            app.add_action_picker.open();
        }
        if toolbar_icon(ui, "x", "Variables", true).clicked() {
            app.variables_panel.open = true;
        }
        ui.separator();
        if toolbar_icon(ui, "📄", "Copy (Ctrl+C)", can_copy && !running).clicked() {
            app.copy_selection();
        }
        if toolbar_icon(ui, "✂", "Cut (Ctrl+X)", can_copy && !running).clicked() {
            app.cut_selection();
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
        if toolbar_icon(ui, "⬇⬇", "Expand all branches", true).clicked() {
            force_openness = Some(true);
        }
        if toolbar_icon(ui, "⬆⬆", "Collapse all branches", true).clicked() {
            force_openness = Some(false);
        }
    });
    force_openness
}
