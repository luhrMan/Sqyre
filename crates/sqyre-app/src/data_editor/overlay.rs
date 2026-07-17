//! Overlay button settings persist + icon picker window.

use super::DataEditor;
use crate::overlay_icons;
use eframe::egui;
use sqyre_persist::UserSettings;

impl DataEditor {
    pub(crate) fn persist_overlay_settings(&mut self, settings: &mut UserSettings) -> bool {
        settings.clamp();
        if let Err(e) = settings.save_default() {
            self.set_err(format!("Failed to save overlay settings: {e}"));
            false
        } else {
            self.clear_status();
            true
        }
    }

    pub(crate) fn apply_overlay_update(&mut self, settings: &mut UserSettings) {
        let Some(id) = self.selected_entity.clone() else {
            self.set_err("Select an overlay button first.");
            return;
        };
        let Some(prog) = self.selected_program.clone() else {
            self.set_err("Select a program first.");
            return;
        };
        if self.form_overlay_macro.trim().is_empty() {
            self.set_err("Pick a macro.");
            return;
        }
        let Some(btn) = settings.overlay_buttons.iter_mut().find(|b| b.id == id) else {
            self.set_err("Overlay button not found.");
            return;
        };
        btn.program = prog;
        btn.label = self.form_name.trim().to_string();
        btn.macro_name = self.form_overlay_macro.trim().to_string();
        btn.icon = self.form_overlay_icon.clone();
        btn.x = self.form_overlay_x;
        btn.y = self.form_overlay_y;
        btn.size = self.form_overlay_size;
        self.apply_overlay_style_to_config(btn);
        if self.persist_overlay_settings(settings) {
            self.set_ok("Saved overlay button.");
        }
    }

    pub(crate) fn draw_overlay_icon_picker(
        &mut self,
        ctx: &egui::Context,
        settings: &mut UserSettings,
    ) {
        let Some(button_id) = self.overlay_icon_picker_for.clone() else {
            return;
        };
        if self.selected_entity.as_deref() != Some(button_id.as_str()) {
            self.overlay_icon_picker_for = None;
            return;
        }
        let current = self.form_overlay_icon.clone();
        let mut open = true;
        let mut close = false;
        egui::Window::new("Choose overlay icon")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([420.0, 480.0])
            .default_pos(egui::pos2(120.0, 80.0))
            .show(ctx, |ui| {
                ui.weak("Phosphor Icons — search by name, then click to select.");
                ui.add_space(4.0);
                if let Some(id) = overlay_icons::show_icon_picker_grid(
                    ui,
                    &current,
                    &mut self.overlay_icon_search,
                ) {
                    self.form_overlay_icon = id.to_string();
                    close = true;
                }
            });
        if !open || close {
            self.overlay_icon_picker_for = None;
        }
        let _ = settings; // form-edited; persist via Update
    }
}
