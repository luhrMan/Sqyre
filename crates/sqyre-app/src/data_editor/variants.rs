//! Item icon variants, mask images, AutoPic save.

use super::helpers::{copy_image_as_png, form_coord_i32};
use super::{DataEditor, PendingConfirm, VariantPrompt};
use crate::data_editor_preview::{
    fit_panel, fit_thumbnail, variant_display_label, variant_name_from_path,
};
use crate::icon_cache::IconCache;
use crate::icon_variants::{self, AddVariantError};
use eframe::egui;
use sqyre_executor::DesktopRect;
use sqyre_persist::{auto_pic_path, ProgramCatalog};
use sqyre_vision::invalidate_search_masks_under;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

impl DataEditor {
    pub(crate) fn paint_item_variants_ui(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        catalog: &ProgramCatalog,
        target: &str,
        item: &str,
    ) {
        let paths = catalog.variant_paths(target);
        let names = icon_variants::variant_names(
            catalog,
            self.selected_program.as_deref().unwrap_or(""),
            item,
        );
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Icon variants").strong());
            if ui
                .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
                .on_hover_text("Refresh")
                .clicked()
            {
                for path in &paths {
                    icons.invalidate_path(path);
                }
            }
            if ui.button("Add Icon Variant").clicked() {
                self.pick_and_add_variant(catalog, icons);
            }
        });
        if paths.is_empty() {
            let fallback = icons.for_target_or_fallback(ui.ctx(), catalog, target);
            let [tw, th] = fallback.size();
            let size = fit_panel(tw as f32, th as f32);
            ui.add(egui::Image::new((fallback.id(), size)));
            ui.weak("No icon variants on disk.");
            return;
        }
        let can_delete = names.len() > 1;
        ui.horizontal_wrapped(|ui| {
            for path in &paths {
                let variant = variant_name_from_path(path, item);
                ui.vertical(|ui| {
                    ui.set_max_width(112.0);
                    match icons.for_path(ui.ctx(), path) {
                        Some(tex) => {
                            let [tw, th] = tex.size();
                            let size = fit_thumbnail(tw as f32, th as f32);
                            ui.add(egui::Image::new((tex.id(), size)));
                        }
                        None => {
                            ui.weak("Missing");
                        }
                    }
                    ui.small(variant_display_label(&variant));
                    let deny = !can_delete || variant == "Original";
                    if ui
                        .add_enabled(!deny, egui::Button::new("Delete").small())
                        .clicked()
                    {
                        self.confirm = Some(PendingConfirm::DeleteVariant {
                            variant: variant.clone(),
                        });
                    }
                });
                ui.add_space(8.0);
            }
        });
    }

    pub(crate) fn pick_and_add_variant(&mut self, catalog: &ProgramCatalog, icons: &mut IconCache) {
        let Some(path) = crate::file_dialogs::pick_png() else {
            return;
        };
        let (Some(prog), Some(item)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        else {
            self.set_err("Select an item first.");
            return;
        };
        let existing = icon_variants::variant_names(catalog, &prog, &item);
        if existing.is_empty() {
            self.add_icon_variant(catalog, icons, "Original", &path);
        } else {
            self.variant_name_draft.clear();
            self.variant_prompt = Some(VariantPrompt::Name { source: path });
        }
    }

    pub(crate) fn add_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        name: &str,
        source: &std::path::Path,
    ) {
        let (Some(prog), Some(item)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        else {
            self.set_err("Select an item first.");
            return;
        };
        match icon_variants::add_variant(catalog, &prog, &item, name, source) {
            Ok(added) => {
                let path = icon_variants::variant_path(catalog, &prog, &item, &added);
                icons.invalidate_path(&path);
                self.set_ok(format!("Added variant “{added}”."));
            }
            Err(AddVariantError::Exists(e)) => {
                self.confirm = Some(PendingConfirm::OverwriteVariant {
                    variant: e.variant_name,
                    source: source.to_path_buf(),
                });
            }
            Err(AddVariantError::Other(err)) => self.set_err(err),
        }
    }

    pub(crate) fn overwrite_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        variant: &str,
        source: &std::path::Path,
    ) {
        let (Some(prog), Some(item)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        else {
            return;
        };
        match icon_variants::overwrite_variant(catalog, &prog, &item, variant, source) {
            Ok(()) => {
                let path = icon_variants::variant_path(catalog, &prog, &item, variant);
                icons.invalidate_path(&path);
                self.set_ok(format!("Overwrote variant “{variant}”."));
            }
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn delete_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        variant: &str,
    ) {
        let (Some(prog), Some(item)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        else {
            return;
        };
        let names = icon_variants::variant_names(catalog, &prog, &item);
        if names.len() <= 1 {
            self.set_err("Cannot delete the last icon variant.");
            return;
        }
        if variant == "Original" {
            self.set_err("The 'Original' variant cannot be deleted.");
            return;
        }
        match icon_variants::delete_variant(catalog, &prog, &item, variant) {
            Ok(()) => {
                let path = icon_variants::variant_path(catalog, &prog, &item, variant);
                icons.invalidate_path(&path);
                self.set_ok(format!("Deleted variant “{variant}”."));
            }
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn upload_mask_image(&mut self, catalog: &ProgramCatalog, icons: &mut IconCache) {
        let (Some(prog), Some(mask)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        else {
            self.set_err("Select a mask first.");
            return;
        };
        let Some(src) = crate::file_dialogs::pick_image() else {
            return;
        };
        let dest = catalog.mask_image_path(&prog, &mask);
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.set_err(format!("create mask dir: {e}"));
                return;
            }
        }
        match copy_image_as_png(&src, &dest) {
            Ok(()) => {
                icons.invalidate_path(&dest);
                invalidate_search_masks_under(&dest);
                self.set_ok("Uploaded mask image.");
            }
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn remove_mask_image(&mut self, catalog: &ProgramCatalog, icons: &mut IconCache) {
        let (Some(prog), Some(mask)) =
            (self.selected_program.clone(), self.selected_entity.clone())
        else {
            return;
        };
        let path = catalog.mask_image_path(&prog, &mask);
        match std::fs::remove_file(&path) {
            Ok(()) => {
                icons.invalidate_path(&path);
                invalidate_search_masks_under(&path);
                self.set_ok("Removed mask image.");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                icons.invalidate_path(&path);
                invalidate_search_masks_under(&path);
                self.set_ok("Removed mask image.");
            }
            Err(e) => self.set_err(format!("remove mask image: {e}")),
        }
    }

    pub(crate) fn save_autopix(&mut self) {
        if self.autopix_pending.is_some() {
            self.set_ok("AutoPic: capturing…");
            return;
        }
        let name = self.form_name.trim().to_string();
        if name.is_empty() {
            self.set_err("AutoPic: select a search area first.");
            return;
        }
        let lx = form_coord_i32(&self.form_left);
        let ty = form_coord_i32(&self.form_top);
        let rx = form_coord_i32(&self.form_right);
        let by = form_coord_i32(&self.form_bottom);
        let (lx, rx) = if lx <= rx { (lx, rx) } else { (rx, lx) };
        let (ty, by) = if ty <= by { (ty, by) } else { (by, ty) };
        if rx - lx <= 0 || by - ty <= 0 {
            self.set_err("AutoPic: invalid search area dimensions.");
            return;
        }

        let capturer = match sqyre_capture::shared_capturer() {
            Ok(c) => c,
            Err(e) => {
                self.set_err(format!("AutoPic: {e}"));
                return;
            }
        };
        let right = rx;
        let bottom = by;
        let (tx, result_rx) = mpsc::channel();
        let area_name = name.clone();
        thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let img = capturer
                    .capture_rect_ref(DesktopRect {
                        x: lx,
                        y: ty,
                        w: right - lx,
                        h: bottom - ty,
                    })
                    .map_err(|e| format!("AutoPic: {e} (area: {area_name})"))?;
                let dir = auto_pic_path();
                std::fs::create_dir_all(&dir).map_err(|e| format!("AutoPic: create dir: {e}"))?;
                let stamp = {
                    use web_time::{SystemTime, UNIX_EPOCH};
                    let dur = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default();
                    // Timestamp without chrono: YYYYMMDD_HHMMSS UTC.
                    let secs = dur.as_secs() as i64;
                    let days = secs.div_euclid(86_400);
                    let day_secs = secs.rem_euclid(86_400) as u32;
                    let hh = day_secs / 3600;
                    let mm = (day_secs % 3600) / 60;
                    let ss = day_secs % 60;
                    // Civil date from Unix days (algorithm from civil_from_days / Howard Hinnant).
                    let z = days + 719_468;
                    let era = z.div_euclid(146_097);
                    let doe = (z - era * 146_097) as u32;
                    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
                    let y = yoe as i64 + era * 400;
                    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
                    let mp = (5 * doy + 2) / 153;
                    let d = doy - (153 * mp + 2) / 5 + 1;
                    let m = if mp < 10 { mp + 3 } else { mp - 9 };
                    let y = if m <= 2 { y + 1 } else { y };
                    format!("{y:04}{m:02}{d:02}_{hh:02}{mm:02}{ss:02}")
                };
                let filename = format!("{stamp}_{area_name}.png");
                let full = dir.join(&filename);
                img.save(&full)
                    .map_err(|e| format!("AutoPic: save {}: {e}", full.display()))?;
                Ok(format!("AutoPic: saved {}", full.display()))
            })();
            let _ = tx.send(result);
        });
        self.autopix_pending = Some(result_rx);
        self.set_ok("AutoPic: capturing…");
    }

    pub(crate) fn poll_autopix(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.autopix_pending.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(msg)) => {
                self.autopix_pending = None;
                self.set_ok(msg);
            }
            Ok(Err(e)) => {
                self.autopix_pending = None;
                self.set_err(e);
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(TryRecvError::Disconnected) => {
                self.autopix_pending = None;
                self.set_err("AutoPic: capture failed");
            }
        }
    }
}
