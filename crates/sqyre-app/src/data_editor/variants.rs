//! Item icon variants, mask images, ScreenCap save.

use super::helpers::copy_image_as_png;
#[cfg(any(test, feature = "native-runtime"))]
use super::helpers::{parse_i32, unique_name};
use super::{DataEditor, DataEditorCtx, PendingConfirm, VariantPrompt};
use crate::data_editor_preview::{
    fit_panel, fit_thumbnail, pixel_size_text, variant_display_label, variant_name_from_path,
};
use crate::icon_cache::IconCache;
use crate::icon_variants::{self, AddVariantError};
use eframe::egui;
use sqyre_domain::{CoordinateRef, Macro, PROGRAM_DELIMITER};
#[cfg(any(test, feature = "native-runtime"))]
use sqyre_persist::ProgramItem;
use sqyre_persist::{screen_cap_path, ProgramCatalog, UserSettings};
#[cfg(feature = "native-runtime")]
use sqyre_validate::{validate_entity_name, validate_item_grid_fields};
#[cfg(feature = "native-runtime")]
use sqyre_vision::invalidate_search_masks_under;

#[cfg(not(feature = "native-runtime"))]
fn invalidate_search_masks_under(_path: &std::path::Path) {}
#[cfg(feature = "native-runtime")]
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
#[cfg(feature = "native-runtime")]
use std::thread;

impl DataEditor {
    pub(crate) fn paint_item_variants_ui(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        catalog: &ProgramCatalog,
        settings: &UserSettings,
        target: &str,
        item: &str,
    ) {
        let paths = crate::demo_icons::merged_variant_paths(catalog, target);
        ui.add_space(8.0);
        ui.separator();
        // Cap to the visible row width — `set_min_width(available_width)` inside a
        // ScrollArea ratchets content wider than the viewport once a scrollbar
        // appears, which clips the trailing buttons.
        let row_w = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(row_w, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(egui::RichText::new("Icon variants").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("Add Icon Variant")
                                .color(crate::theme::MACRO_START),
                        )
                        .clicked()
                    {
                        self.pick_and_add_variant(catalog, icons, settings);
                    }
                    if crate::theme::icon_button(ui, "↻")
                        .on_hover_text("Refresh")
                        .clicked()
                    {
                        icons.invalidate_target(target);
                        for path in &paths {
                            icons.invalidate_path(path);
                        }
                    }
                    ui.label(egui::RichText::new(format!("({})", paths.len())).weak());
                });
            },
        );
        if paths.is_empty() {
            let fallback = icons.for_target_or_fallback(ui.ctx(), catalog, target);
            let [tw, th] = fallback.size();
            let size = fit_panel(tw as f32, th as f32);
            ui.add(egui::Image::new((fallback.id(), size)));
            ui.small(pixel_size_text(tw as i32, th as i32));
            ui.weak("No icon variants on disk.");
            return;
        }
        let on_disk = paths.iter().filter(|p| p.is_file()).count();
        let can_delete = on_disk > 1;
        ui.horizontal_wrapped(|ui| {
            for path in &paths {
                let variant = variant_name_from_path(path, item);
                let is_demo = !path.is_file() && crate::demo_icons::contains(path);
                ui.vertical(|ui| {
                    ui.set_max_width(112.0);
                    match icons.for_path(ui.ctx(), path) {
                        Some(tex) => {
                            let [tw, th] = tex.size();
                            let size = fit_thumbnail(tw as f32, th as f32);
                            ui.add(egui::Image::new((tex.id(), size)));
                            ui.small(pixel_size_text(tw as i32, th as i32));
                        }
                        None => {
                            ui.weak("Missing");
                        }
                    }
                    ui.small(variant_display_label(&variant));
                    if is_demo {
                        ui.weak("demo");
                    }
                    let deny =
                        is_demo || !can_delete || variant.is_empty() || variant == "Original";
                    if ui
                        .add_enabled(
                            !deny,
                            egui::Button::new(
                                egui::RichText::new("Delete").color(crate::theme::MACRO_STOP),
                            )
                            .small(),
                        )
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

    pub(crate) fn pick_and_add_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
    ) {
        let Some(path) = crate::file_dialogs::pick_png(&screen_cap_path()) else {
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
            self.add_icon_variant(catalog, icons, settings, "Original", &path);
        } else {
            self.variant_name_draft.clear();
            self.variant_prompt = Some(VariantPrompt::Name { source: path });
        }
    }

    pub(crate) fn add_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
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
                let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
                icons.invalidate_path(&path);
                icons.invalidate_target(&target);
                self.set_ok(format!("Added variant “{added}”."));
                #[cfg(not(target_arch = "wasm32"))]
                crate::sound::play_add_sound_if(settings.play_ui_sounds, settings.sound_volume);
                #[cfg(target_arch = "wasm32")]
                let _ = settings;
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
                let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
                icons.invalidate_path(&path);
                icons.invalidate_target(&target);
                self.set_ok(format!("Overwrote variant “{variant}”."));
            }
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn delete_icon_variant(
        &mut self,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
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
                let target = format!("{prog}{PROGRAM_DELIMITER}{item}");
                icons.invalidate_path(&path);
                icons.invalidate_target(&target);
                self.set_ok(format!("Deleted variant “{variant}”."));
                #[cfg(not(target_arch = "wasm32"))]
                crate::sound::play_delete_sound_if(settings.play_ui_sounds, settings.sound_volume);
                #[cfg(target_arch = "wasm32")]
                let _ = settings;
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

    /// Seed ScreenCap form buffers from a search-area or collection-cell reference.
    /// Returns `true` when the reference resolved to desktop bounds.
    pub(crate) fn apply_screen_cap_reference(
        &mut self,
        catalog: &ProgramCatalog,
        coord: CoordinateRef,
    ) -> bool {
        let macro_ = Macro::new("", 0, vec![]);
        let (lx, ty, rx, by) = match catalog.resolve_search_area(&coord, &macro_) {
            Ok(bounds) => bounds,
            Err(e) => {
                self.set_err(format!("ScreenCap: {e}"));
                return false;
            }
        };
        let (monitor, lx, ty, rx, by) = sqyre_persist::absolute_area_to_relative(
            catalog.monitor_rects(),
            lx,
            ty,
            lx,
            ty,
            rx,
            by,
        );
        self.form_monitor = monitor;
        self.form_left = lx.to_string();
        self.form_top = ty.to_string();
        self.form_right = rx.to_string();
        self.form_bottom = by.to_string();
        self.form_search_area = coord.0.clone();
        self.form_name = match coord.cell_range() {
            Some((r1, c1, r2, c2)) if r1 == r2 && c1 == c2 => {
                format!("{}_r{}c{}", coord.name(), r1, c1)
            }
            Some((r1, c1, r2, c2)) => {
                format!("{}_r{}c{}-r{}c{}", coord.name(), r1, c1, r2, c2)
            }
            None => coord.name().to_string(),
        };
        if let Some(prog) = coord.program() {
            self.selected_program = Some(prog.to_string());
        }
        self.selected_entity = Some(coord.name().to_string());
        self.coord_preview.reset();
        self.coord_preview_key = None;
        true
    }

    #[cfg(feature = "native-runtime")]
    fn screen_cap_name(&self) -> Result<String, String> {
        let name = self.form_name.trim().to_string();
        if name.is_empty() {
            return Err("ScreenCap: enter a name.".into());
        }
        validate_entity_name(&name).map_err(|e| format!("ScreenCap: {e}"))?;
        Ok(name)
    }

    #[cfg(feature = "native-runtime")]
    fn screen_cap_preview_image(
        &self,
        catalog: &ProgramCatalog,
        previews: &crate::preview_tooltip::PreviewTooltipCache,
    ) -> Result<std::sync::Arc<image::RgbaImage>, String> {
        let (Some(lx), Some(ty), Some(rx), Some(by)) = super::helpers::form_desktop_area(
            catalog,
            self.form_monitor,
            &self.form_left,
            &self.form_top,
            &self.form_right,
            &self.form_bottom,
        ) else {
            return Err("ScreenCap: invalid capture dimensions.".into());
        };
        let (norm_lx, norm_rx) = if lx <= rx { (lx, rx) } else { (rx, lx) };
        let (norm_ty, norm_by) = if ty <= by { (ty, by) } else { (by, ty) };
        if norm_rx - norm_lx <= 0 || norm_by - norm_ty <= 0 {
            return Err("ScreenCap: invalid capture dimensions.".into());
        }
        previews
            .screen_cap_image(lx, ty, rx, by)
            .ok_or_else(|| "ScreenCap: wait for the preview screenshot to finish.".into())
    }

    pub(crate) fn save_screen_cap(
        &mut self,
        catalog: &ProgramCatalog,
        previews: &crate::preview_tooltip::PreviewTooltipCache,
    ) {
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (catalog, previews);
            self.set_err("ScreenCap requires the desktop app.");
            return;
        }
        #[cfg(feature = "native-runtime")]
        {
            if self.screen_cap_pending.is_some() {
                self.set_ok("ScreenCap: saving…");
                return;
            }
            let name = match self.screen_cap_name() {
                Ok(n) => n,
                Err(e) => {
                    self.set_err(e);
                    return;
                }
            };
            let img = match self.screen_cap_preview_image(catalog, previews) {
                Ok(img) => img,
                Err(e) => {
                    self.set_err(e);
                    return;
                }
            };

            let (tx, result_rx) = mpsc::channel();
            let area_name = name.clone();
            thread::spawn(move || {
                let result = (|| -> Result<String, String> {
                    let dir = screen_cap_path();
                    std::fs::create_dir_all(&dir)
                        .map_err(|e| format!("ScreenCap: create dir: {e}"))?;
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
                        .map_err(|e| format!("ScreenCap: save {}: {e}", full.display()))?;
                    Ok(format!("ScreenCap: saved {}", full.display()))
                })();
                let _ = tx.send(result);
            });
            self.screen_cap_pending = Some(result_rx);
            self.set_ok("ScreenCap: saving…");
        }
    }

    pub(crate) fn create_item_from_screen_cap(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        previews: &crate::preview_tooltip::PreviewTooltipCache,
    ) {
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (env, previews);
            self.set_err("ScreenCap requires the desktop app.");
            return;
        }
        #[cfg(feature = "native-runtime")]
        {
            let DataEditorCtx {
                db,
                macros,
                catalog,
                icons,
                settings,
                ..
            } = env;
            let Some(prog) = self.selected_program.clone() else {
                self.set_err("ScreenCap: select a program for the new item.");
                return;
            };
            let requested = match self.screen_cap_name() {
                Ok(n) => n,
                Err(e) => {
                    self.set_err(e);
                    return;
                }
            };
            if let Err(e) =
                validate_item_grid_fields(&self.form_cols, &self.form_rows, &self.form_stack_max)
            {
                self.set_err(format!("ScreenCap: {e}"));
                return;
            }
            let img = match self.screen_cap_preview_image(catalog, previews) {
                Ok(img) => img,
                Err(e) => {
                    self.set_err(e);
                    return;
                }
            };
            let item = ProgramItem {
                name: requested,
                mask: self.form_mask.clone(),
                stack_max: parse_i32(&self.form_stack_max).unwrap_or(0),
                grid_cols: parse_i32(&self.form_cols).unwrap_or(1),
                grid_rows: parse_i32(&self.form_rows).unwrap_or(1),
                tags: self.form_tags.clone(),
            };
            match create_item_with_original(catalog, &prog, item, img.as_ref()) {
                Ok(name) => {
                    let target = format!("{prog}{PROGRAM_DELIMITER}{name}");
                    let path = icon_variants::variant_path(catalog, &prog, &name, "Original");
                    icons.invalidate_path(&path);
                    icons.invalidate_target(&target);
                    if let Err(e) = self.persist(db, macros, catalog) {
                        self.set_err(e);
                    } else {
                        #[cfg(not(target_arch = "wasm32"))]
                        crate::sound::play_add_sound_if(
                            settings.play_ui_sounds,
                            settings.sound_volume,
                        );
                        #[cfg(target_arch = "wasm32")]
                        let _ = settings;
                        self.set_ok(format!(
                            "ScreenCap: created item “{name}” with Original from capture."
                        ));
                        self.form_name = name;
                    }
                }
                Err(e) => self.set_err(format!("ScreenCap: {e}")),
            }
        }
    }

    pub(crate) fn poll_screen_cap(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.screen_cap_pending.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(msg)) => {
                self.screen_cap_pending = None;
                self.set_ok(msg);
            }
            Ok(Err(e)) => {
                self.screen_cap_pending = None;
                self.set_err(e);
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(TryRecvError::Disconnected) => {
                self.screen_cap_pending = None;
                self.set_err("ScreenCap: capture failed");
            }
        }
    }

    pub(crate) fn start_collection_capture(
        &mut self,
        catalog: &ProgramCatalog,
        program: &str,
        collection: &sqyre_persist::ProgramCollection,
        rollback_collection: Option<(String, String)>,
    ) -> Result<(), String> {
        #[cfg(not(feature = "native-runtime"))]
        {
            let _ = (catalog, program, collection, rollback_collection);
            self.set_err("Collection capture requires the desktop app.");
            return Ok(());
        }
        #[cfg(feature = "native-runtime")]
        {
            use crate::collection_capture::{capture_search_area_to_png, collection_capture_job};
            use std::sync::mpsc;
            use std::thread;

            if self.collection_capture_pending.is_some() {
                self.set_ok("Collection: capturing…");
                return Ok(());
            }
            let (path, left, top, right, bottom) =
                collection_capture_job(catalog, program, collection)?;
            let path_for_thread = path.clone();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let result = capture_search_area_to_png(left, top, right, bottom, &path_for_thread);
                let _ = tx.send(result);
            });
            self.collection_capture_pending = Some(super::CollectionCapturePending {
                path,
                rollback_collection,
                rx,
            });
            self.set_ok("Collection: capturing…");
            Ok(())
        }
    }

    pub(crate) fn poll_collection_capture(
        &mut self,
        ctx: &egui::Context,
        catalog: &mut ProgramCatalog,
        icons: &mut crate::icon_cache::IconCache,
    ) {
        use std::sync::mpsc::TryRecvError;

        let Some(pending) = self.collection_capture_pending.take() else {
            return;
        };
        match pending.rx.try_recv() {
            Ok(Ok(())) => {
                icons.invalidate_path(&pending.path);
                self.collection_preview.reset();
                self.set_ok("Collection image saved.");
            }
            Ok(Err(e)) => {
                if let Some((prog, name)) = pending.rollback_collection {
                    let _ = catalog.delete_collection(&prog, &name);
                }
                self.set_err(e);
            }
            Err(TryRecvError::Empty) => {
                self.collection_capture_pending = Some(pending);
                ctx.request_repaint();
            }
            Err(TryRecvError::Disconnected) => {
                if let Some((prog, name)) = pending.rollback_collection {
                    let _ = catalog.delete_collection(&prog, &name);
                }
                self.set_err("Collection: capture failed");
            }
        }
    }
}

#[cfg(any(test, feature = "native-runtime"))]
fn create_item_with_original(
    catalog: &mut ProgramCatalog,
    program: &str,
    mut item: ProgramItem,
    img: &image::RgbaImage,
) -> Result<String, String> {
    let name = unique_name(&item.name, |n| {
        catalog.get(program).and_then(|p| p.items.get(n)).is_some()
    });
    item.name = name.clone();
    catalog
        .upsert_item(program, item)
        .map_err(|e| e.to_string())?;
    if let Err(e) = icon_variants::add_variant_image(catalog, program, &name, img) {
        return Err(match catalog.delete_item(program, &name) {
            Ok(()) => e.to_string(),
            Err(del) => format!("{e}; also failed to remove item: {del}"),
        });
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::ProgramCatalog;
    use tempfile::tempdir;

    fn catalog_with_icons(root: &std::path::Path) -> ProgramCatalog {
        let mut c = ProgramCatalog::default();
        c.set_images_root(Some(root.to_path_buf()));
        c.create_program("Game").unwrap();
        c
    }

    fn sample_item(name: &str) -> ProgramItem {
        ProgramItem {
            name: name.into(),
            mask: String::new(),
            stack_max: 0,
            grid_cols: 1,
            grid_rows: 1,
            tags: Vec::new(),
        }
    }

    #[test]
    fn create_item_with_original_uses_name_and_writes_png() {
        let dir = tempdir().unwrap();
        let mut cat = catalog_with_icons(dir.path());
        let img = image::RgbaImage::from_pixel(3, 2, image::Rgba([1, 2, 3, 255]));
        let name =
            create_item_with_original(&mut cat, "Game", sample_item("Potion"), &img).unwrap();
        assert_eq!(name, "Potion");
        let item = cat.get("Game").unwrap().items.get("Potion").unwrap();
        assert_eq!(item.grid_cols, 1);
        assert_eq!(item.grid_rows, 1);
        let path = icon_variants::variant_path(&cat, "Game", "Potion", "Original");
        assert!(path.is_file());
        assert!(icon_variants::validate_png_file(&path).is_ok());
    }

    #[test]
    fn create_item_with_original_applies_item_fields() {
        let dir = tempdir().unwrap();
        let mut cat = catalog_with_icons(dir.path());
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([4, 5, 6, 255]));
        let item = ProgramItem {
            name: "Elixir".into(),
            mask: "round".into(),
            stack_max: 9,
            grid_cols: 2,
            grid_rows: 3,
            tags: vec!["potion".into(), "craft".into()],
        };
        let name = create_item_with_original(&mut cat, "Game", item, &img).unwrap();
        assert_eq!(name, "Elixir");
        let saved = cat.get("Game").unwrap().items.get("Elixir").unwrap();
        assert_eq!(saved.mask, "round");
        assert_eq!(saved.stack_max, 9);
        assert_eq!(saved.grid_cols, 2);
        assert_eq!(saved.grid_rows, 3);
        assert_eq!(saved.tags, ["potion", "craft"]);
    }

    #[test]
    fn create_item_with_original_uniques_existing_name() {
        let dir = tempdir().unwrap();
        let mut cat = catalog_with_icons(dir.path());
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([9, 9, 9, 255]));
        create_item_with_original(&mut cat, "Game", sample_item("Potion"), &img).unwrap();
        let name =
            create_item_with_original(&mut cat, "Game", sample_item("Potion"), &img).unwrap();
        assert_eq!(name, "Potion 2");
        assert!(cat.get("Game").unwrap().items.contains_key("Potion 2"));
        assert!(icon_variants::variant_path(&cat, "Game", "Potion 2", "Original").is_file());
    }
}
