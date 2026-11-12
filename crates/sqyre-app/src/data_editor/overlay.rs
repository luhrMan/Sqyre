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
        if let Some(r) = sqyre_capture::preferred_monitor_rects()
            .into_iter()
            .find(|r| r.w > 1 && r.h > 1)
        {
            return (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
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
            self.set_err(format!(
                "Failed to save overlay settings: {}",
                crate::status_banner::user_error_detail(&e)
            ));
            false
        } else {
            self.clear_status();
            true
        }
    }

    /// Keep the open overlay form in sync when a referenced macro is renamed.
    pub(crate) fn rename_overlay_form_macro(&mut self, old_name: &str, new_name: &str) {
        if self.form_overlay_macro == old_name {
            self.form_overlay_macro = new_name.to_string();
        }
    }

    pub(crate) fn apply_overlay_update(&mut self, settings: &mut UserSettings, ppp: f32) {
        let Some(id) = self.selected_entity.clone() else {
            self.set_err("Select an overlay button first.");
            return;
        };
        let hub = sqyre_persist::is_quick_access_hub_id(&id);
        let Some(prog) = self.selected_program_or_err() else {
            return;
        };
        if !hub && self.form_overlay_macro.trim().is_empty() {
            self.set_err("Pick a macro.");
            return;
        }
        let Some(btn) = settings.overlay_buttons.iter_mut().find(|b| b.id == id) else {
            self.set_err("Overlay button not found.");
            return;
        };
        if hub {
            btn.program = sqyre_domain::GENERAL_PROGRAM.into();
            btn.macro_name = sqyre_persist::QUICK_ACCESS_HUB_ID.into();
            btn.icon = sqyre_persist::QUICK_ACCESS_HUB_ICON.into();
            btn.point.clear();
            btn.enabled = self.form_overlay_enabled;
            btn.x = self.form_overlay_x;
            btn.y = self.form_overlay_y;
            btn.size = self.form_overlay_size;
            btn.visibility_gate = Default::default();
            self.apply_overlay_style_to_config(btn);
            btn.border_width = 0.0;
            btn.border_alpha = 0;
            btn.bg_alpha = 0;
            btn.corner_radius = (btn.size * 0.5).clamp(
                sqyre_persist::MIN_OVERLAY_CORNER_RADIUS,
                sqyre_persist::MAX_OVERLAY_CORNER_RADIUS,
            );
            // Persist edge-docked coords so form X/Y match what the face shows.
            let size_px = (btn.size * ppp.max(0.01)).round().max(1.0);
            let (mon_x, mon_y, mon_w, mon_h) = crate::quick_access::monitor_for_point(btn.x, btn.y);
            let _ =
                sqyre_persist::snap_hub_button_to_monitor(btn, mon_x, mon_y, mon_w, mon_h, size_px);
            self.form_overlay_x = btn.x;
            self.form_overlay_y = btn.y;
        } else {
            btn.program = prog;
            btn.macro_name = self.form_overlay_macro.trim().to_string();
            btn.enabled = self.form_overlay_enabled;
            btn.icon = self.form_overlay_icon.clone();
            btn.point = self.form_overlay_point.trim().to_string();
            btn.x = self.form_overlay_x;
            btn.y = self.form_overlay_y;
            btn.size = self.form_overlay_size;
            btn.visibility_gate = self.overlay_gate_from_form();
            self.apply_overlay_style_to_config(btn);
        }
        if self.persist_overlay_settings(settings) {
            self.set_ok("Saved overlay button.");
        }
    }

    pub(crate) fn draw_overlay_icon_picker(
        &mut self,
        ctx: &egui::Context,
        settings: &mut UserSettings,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) {
        let Some(button_id) = self.overlay_icon_picker_for.clone() else {
            return;
        };
        if sqyre_persist::is_quick_access_hub_id(&button_id) {
            self.overlay_icon_picker_for = None;
            return;
        }
        if self.selected_entity.as_deref() != Some(button_id.as_str()) {
            self.overlay_icon_picker_for = None;
            return;
        }
        let current = self.form_overlay_icon.clone();
        let mut open = true;
        let mut close = false;
        // Bounds only (no Window scroll). `fill_resize_body` pins content to the
        // Resize region so fill widgets cannot ratchet/lock the window size.
        // Salt bump drops huge sizes persisted from earlier ratchets.
        let constrain = crate::widgets::dialog_constrain_rect(ctx).size();
        let picker_id = egui::Id::new(("overlay_icon_picker", "resize_v4"));
        // When viewport-scaling, skip the tighter max override so the one-frame
        // min==max clamp from fit_dialog_popup is not widened/undone.
        let window = crate::widgets::fit_dialog_popup(
            egui::Window::new("Choose overlay icon")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([420.0, 480.0])
                .min_size(crate::widgets::FLOATER_MIN_MODAL)
                .default_pos(egui::pos2(120.0, 80.0)),
            ctx,
            picker_id,
            pending_scale,
        );
        let window = if pending_scale.is_some() {
            window
        } else {
            // Cap below full screen so a layout bug cannot fill the monitor.
            window.max_size(egui::vec2(constrain.x.min(720.0), constrain.y.min(900.0)))
        };
        window.show(ctx, |ui| {
            let body = crate::widgets::fill_resize_body(ui, |ui| {
                crate::action_tooltip::help::label(ui, egui_phosphor::regular::SMILEY, "Icons");
                if let Some(id) = overlay_icons::show_icon_picker_grid(
                    ui,
                    &current,
                    &mut self.overlay_icon_search,
                ) {
                    self.form_overlay_icon = id.to_string();
                    close = true;
                }
            });
            crate::action_tooltip::help::tip(
                body,
                crate::action_tooltip::help::DE_OVERLAY_ICON_PICKER,
            );
            close |= crate::widgets::consume_escape(ui);
        });
        if !open || close {
            self.overlay_icon_picker_for = None;
        }
        let _ = settings; // form-edited; persist via Save
    }
}
