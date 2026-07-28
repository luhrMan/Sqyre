//! Overlay button settings persist + icon picker window.

use super::DataEditor;
use crate::overlay_icons;
use eframe::egui;
use sqyre_persist::{
    default_overlay_position, ProgramCatalog, UserSettings, DEFAULT_OVERLAY_BUTTON_SIZE,
    DEFAULT_OVERLAY_FALLBACK_SCREEN_H, DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
};

/// Primary-monitor desktop rect `(x, y, w, h)` for centering new overlay buttons.
pub(crate) fn primary_overlay_screen_rect(catalog: &ProgramCatalog) -> (f32, f32, f32, f32) {
    #[cfg(feature = "native-runtime")]
    {
        if let Ok(capturer) = sqyre_capture::shared_capturer() {
            if let Ok(rects) = capturer.monitor_rects_ref() {
                if let Some(r) = rects.into_iter().find(|r| r.w > 1 && r.h > 1) {
                    return (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
                }
            }
        }
    }
    let key = catalog.resolution_key();
    if let Some((w, h)) = key.split_once('x') {
        if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
            if w > 1.0 && h > 1.0 {
                return (0.0, 0.0, w, h);
            }
        }
    }
    (
        0.0,
        0.0,
        DEFAULT_OVERLAY_FALLBACK_SCREEN_W,
        DEFAULT_OVERLAY_FALLBACK_SCREEN_H,
    )
}

/// Centered default `(x, y)` for a new overlay button on the primary monitor.
pub(crate) fn default_overlay_xy(catalog: &ProgramCatalog, index: usize) -> (f32, f32) {
    let (sx, sy, sw, sh) = primary_overlay_screen_rect(catalog);
    default_overlay_position(sx, sy, sw, sh, DEFAULT_OVERLAY_BUTTON_SIZE, index)
}

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
        btn.enabled = self.form_overlay_enabled;
        btn.icon = self.form_overlay_icon.clone();
        btn.point = self.form_overlay_point.trim().to_string();
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
