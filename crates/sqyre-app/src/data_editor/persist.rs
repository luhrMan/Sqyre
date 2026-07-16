//! Create / update / delete / persist catalog entities.

use super::helpers::{new_overlay_button_id, parse_i32, parse_scalar, unique_name};
use super::{DataEditor, EditorTab, PendingConfirm};
use crate::collection_capture::capture_and_save_collection_image;
use crate::icon_cache::IconCache;
use crate::overlay_icons;
use crate::preview_tooltip::PreviewTooltipCache;
use sqyre_domain::{Macro, ProgramEntityKind, ScalarValue};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_persist::{
    Database, OverlayButtonConfig, ProgramCatalog, ProgramCollection, ProgramItem, ProgramMask,
    ProgramPoint, ProgramSearchArea, UserSettings, DEFAULT_OVERLAY_BUTTON_SIZE,
};
use sqyre_validate::validate_entity_name;

impl DataEditor {
    pub(crate) fn on_new(
        &mut self,
        db: &mut Database,
        macros: &mut [Macro],
        catalog: &mut ProgramCatalog,
        icons: &mut IconCache,
        screen_click: &ScreenClickBridge,
        settings: &mut UserSettings,
    ) {
        self.clear_status();
        self.save_after_record = false;
        let created = match self.tab {
            EditorTab::Programs => {
                let base = if self.form_name.trim().is_empty() {
                    "New Program".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| catalog.get(n).is_some());
                match catalog.create_program(&name) {
                    Ok(()) => {
                        self.selected_program = Some(name.clone());
                        self.form_name = name;
                        self.form_process_path.clear();
                        self.form_window_title.clear();
                        Ok("Created program.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Items => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Item".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    catalog.get(&prog).and_then(|p| p.items.get(n)).is_some()
                });
                let item = ProgramItem {
                    name: name.clone(),
                    mask: String::new(),
                    stack_max: 0,
                    grid_cols: 1,
                    grid_rows: 1,
                    tags: Vec::new(),
                };
                match catalog.upsert_item(&prog, item) {
                    Ok(()) => {
                        self.selected_entity = Some(name.clone());
                        self.load_form(catalog, settings);
                        Ok("Created item.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Points => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Point".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(&prog)
                        .and_then(|p| p.points.get(res))
                        .and_then(|m| m.get(n))
                        .is_some()
                });
                let pt = ProgramPoint {
                    name: name.clone(),
                    x: ScalarValue::Int(0),
                    y: ScalarValue::Int(0),
                };
                match catalog.upsert_point(&prog, pt) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog, settings);
                        Ok("Created point.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::SearchAreas => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Search Area".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(&prog)
                        .and_then(|p| p.search_areas.get(res))
                        .and_then(|m| m.get(n))
                        .is_some()
                });
                let sa = ProgramSearchArea {
                    name: name.clone(),
                    left_x: ScalarValue::Int(0),
                    top_y: ScalarValue::Int(0),
                    right_x: ScalarValue::Int(100),
                    bottom_y: ScalarValue::Int(100),
                };
                match catalog.upsert_search_area(&prog, sa) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog, settings);
                        Ok("Created search area.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Masks => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Mask".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    catalog.get(&prog).and_then(|p| p.masks.get(n)).is_some()
                });
                let mask = ProgramMask {
                    name: name.clone(),
                    ..Default::default()
                };
                match catalog.upsert_mask(&prog, mask) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog, settings);
                        Ok("Created mask.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Collections => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let base = if self.form_name.trim().is_empty() {
                    "New Collection".to_string()
                } else {
                    self.form_name.trim().to_string()
                };
                let name = unique_name(&base, |n| {
                    catalog
                        .get(&prog)
                        .and_then(|p| p.collections.get(n))
                        .is_some()
                });
                let default_sa = catalog
                    .get(&prog)
                    .and_then(|p| {
                        let res = catalog.resolution_key();
                        p.search_areas
                            .get(res)
                            .or_else(|| p.search_areas.values().next())
                            .and_then(|m| m.keys().next().cloned())
                    })
                    .unwrap_or_default();
                let search_area = if !self.form_search_area.trim().is_empty() {
                    self.form_search_area.trim().to_string()
                } else {
                    default_sa
                };
                if search_area.is_empty() {
                    self.set_err("Create a search area before capturing a collection image.");
                    return;
                }
                let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
                let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
                let col = ProgramCollection {
                    name: name.clone(),
                    search_area,
                    rows,
                    cols,
                };
                match catalog.upsert_collection(&prog, col.clone()) {
                    Ok(()) => match capture_and_save_collection_image(catalog, &prog, &col) {
                        Ok(()) => {
                            let path = catalog.collection_image_path(&prog, &name);
                            icons.invalidate_path(&path);
                            self.selected_entity = Some(name);
                            self.load_form(catalog, settings);
                            Ok("Created collection.")
                        }
                        Err(e) => {
                            let _ = catalog.delete_collection(&prog, &name);
                            Err(e)
                        }
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::AutoPic => {
                self.set_err("Use Save on the AutoPic tab to capture a search area.");
                return;
            }
            EditorTab::Overlay => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let n = settings
                    .overlay_buttons
                    .iter()
                    .filter(|b| b.program == prog)
                    .count();
                let mut btn = OverlayButtonConfig::new(new_overlay_button_id(), &prog);
                btn.icon = overlay_icons::DEFAULT_ICON_ID.into();
                btn.x = 48.0 + (n as f32) * 60.0;
                btn.y = 48.0;
                btn.size = DEFAULT_OVERLAY_BUTTON_SIZE;
                if let Some(first) = macros.first() {
                    btn.macro_name = first.name.clone();
                    btn.label = first.name.clone();
                } else {
                    btn.label = format!("Button {}", n + 1);
                }
                let id = btn.id.clone();
                settings.overlay_buttons.push(btn);
                self.selected_entity = Some(id);
                self.load_form(catalog, settings);
                self.persist_overlay_settings(settings);
                self.set_ok("Created overlay button.");
                return;
            }
        };
        match created {
            Ok(msg) => {
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    match self.tab {
                        EditorTab::Points if !screen_click.is_armed() => {
                            self.save_after_record = true;
                            screen_click.arm_point();
                            self.set_ok(format!(
                                "{msg} Recording… left-click to capture X/Y."
                            ));
                        }
                        EditorTab::SearchAreas if !screen_click.is_armed() => {
                            self.save_after_record = true;
                            screen_click.arm_search_area();
                            self.set_ok(format!(
                                "{msg} Recording… click two corners."
                            ));
                        }
                        _ => self.set_ok(msg),
                    }
                }
            }
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn on_update(
        &mut self,
        db: &mut Database,
        macros: &mut [Macro],
        catalog: &mut ProgramCatalog,
        previews: &mut PreviewTooltipCache,
        settings: &mut UserSettings,
    ) {
        // Check overwrite for renames onto existing keys
        if let Some((kind, name)) = self.would_overwrite(catalog) {
            self.confirm = Some(PendingConfirm::Overwrite { kind, name });
            return;
        }
        self.apply_update(db, macros, catalog, false, previews, settings);
    }

    pub(crate) fn would_overwrite(&self, catalog: &ProgramCatalog) -> Option<(&'static str, String)> {
        let new = self.form_name.trim();
        match self.tab {
            EditorTab::Programs => {
                let old = self.selected_program.as_deref()?;
                if old != new && catalog.get(new).is_some() {
                    return Some(("Program", new.to_string()));
                }
            }
            EditorTab::Items => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new && catalog.get(prog).and_then(|p| p.items.get(new)).is_some() {
                    return Some(("Item", new.to_string()));
                }
            }
            EditorTab::Points => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                let res = catalog.resolution_key();
                if old != new
                    && catalog
                        .get(prog)
                        .and_then(|p| p.points.get(res))
                        .and_then(|m| m.get(new))
                        .is_some()
                {
                    return Some(("Point", new.to_string()));
                }
            }
            EditorTab::SearchAreas => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                let res = catalog.resolution_key();
                if old != new
                    && catalog
                        .get(prog)
                        .and_then(|p| p.search_areas.get(res))
                        .and_then(|m| m.get(new))
                        .is_some()
                {
                    return Some(("Search area", new.to_string()));
                }
            }
            EditorTab::Masks => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new && catalog.get(prog).and_then(|p| p.masks.get(new)).is_some() {
                    return Some(("Mask", new.to_string()));
                }
            }
            EditorTab::Collections => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new
                    && catalog
                        .get(prog)
                        .and_then(|p| p.collections.get(new))
                        .is_some()
                {
                    return Some(("Collection", new.to_string()));
                }
            }
            EditorTab::AutoPic => {}
            EditorTab::Overlay => {}
        }
        None
    }

    pub(crate) fn apply_update(
        &mut self,
        db: &mut Database,
        macros: &mut [Macro],
        catalog: &mut ProgramCatalog,
        overwrite: bool,
        previews: &mut PreviewTooltipCache,
        settings: &mut UserSettings,
    ) {
        self.clear_status();
        if matches!(self.tab, EditorTab::Overlay) {
            self.apply_overlay_update(settings);
            return;
        }
        let new_name = self.form_name.trim().to_string();
        if validate_entity_name(&new_name).is_err() {
            self.set_err("Invalid name.");
            return;
        }

        let old_entity = self.selected_entity.clone();
        let mut overlay_program_renamed = false;
        let result = match self.tab {
            EditorTab::Programs => {
                if let Some(old) = self.selected_program.clone() {
                    if old == new_name {
                        catalog
                            .set_process_binding(
                                &old,
                                self.form_process_path.clone(),
                                self.form_window_title.clone(),
                            )
                            .map(|_| ())
                    } else {
                        if overwrite {
                            let _ = catalog.delete_program(&new_name);
                        }
                        catalog.rename_program(&old, &new_name).map(|_| {
                            let _ = catalog.set_process_binding(
                                &new_name,
                                self.form_process_path.clone(),
                                self.form_window_title.clone(),
                            );
                            for m in macros.iter_mut() {
                                m.rename_program(&old, &new_name);
                            }
                            for btn in settings.overlay_buttons.iter_mut() {
                                if btn.program == old {
                                    btn.program = new_name.clone();
                                }
                            }
                            overlay_program_renamed = true;
                            self.selected_program = Some(new_name.clone());
                        })
                    }
                } else {
                    catalog.create_program(&new_name).map(|_| {
                        let _ = catalog.set_process_binding(
                            &new_name,
                            self.form_process_path.clone(),
                            self.form_window_title.clone(),
                        );
                        self.selected_program = Some(new_name.clone());
                    })
                }
            }
            EditorTab::Items => self.update_item(catalog, macros, &new_name, overwrite),
            EditorTab::Points => self.update_point(catalog, macros, &new_name, overwrite),
            EditorTab::SearchAreas => self.update_search_area(catalog, macros, &new_name, overwrite),
            EditorTab::Masks => self.update_mask(catalog, &new_name, overwrite),
            EditorTab::Collections => {
                self.update_collection(catalog, macros, &new_name, overwrite)
            }
            EditorTab::AutoPic | EditorTab::Overlay => Ok(()),
        };

        match result {
            Ok(()) => {
                if matches!(self.tab, EditorTab::Points | EditorTab::SearchAreas) {
                    if let Some(old) = old_entity.as_deref() {
                        previews.invalidate_entity(old);
                    }
                    previews.invalidate_entity(&new_name);
                }
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    if overlay_program_renamed {
                        self.persist_overlay_settings(settings);
                    }
                    self.load_form(catalog, settings);
                    self.set_ok("Saved.");
                }
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    pub(crate) fn update_item(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let cols = parse_i32(&self.form_cols).unwrap_or(1);
        let rows = parse_i32(&self.form_rows).unwrap_or(1);
        let stack = parse_i32(&self.form_stack_max).unwrap_or(0);
        let item = ProgramItem {
            name: new_name.to_string(),
            mask: self.form_mask.clone(),
            stack_max: stack,
            grid_cols: cols,
            grid_rows: rows,
            tags: self.form_tags.clone(),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_item(&prog, new_name);
                }
                catalog.rename_item(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Item, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_item(&prog, item)?;
        } else {
            catalog.upsert_item(&prog, item)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    pub(crate) fn update_point(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let pt = ProgramPoint {
            name: new_name.to_string(),
            x: parse_scalar(&self.form_x),
            y: parse_scalar(&self.form_y),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_point(&prog, new_name);
                }
                catalog.rename_point(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Point, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_point(&prog, pt)?;
        } else {
            catalog.upsert_point(&prog, pt)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    pub(crate) fn update_search_area(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let sa = ProgramSearchArea {
            name: new_name.to_string(),
            left_x: parse_scalar(&self.form_left),
            top_y: parse_scalar(&self.form_top),
            right_x: parse_scalar(&self.form_right),
            bottom_y: parse_scalar(&self.form_bottom),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_search_area(&prog, new_name);
                }
                catalog.rename_search_area(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::SearchArea, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_search_area(&prog, sa)?;
        } else {
            catalog.upsert_search_area(&prog, sa)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    pub(crate) fn update_mask(
        &mut self,
        catalog: &mut ProgramCatalog,
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let shape = if self.form_shape == "circle" {
            sqyre_domain::MaskShape::Circle
        } else {
            sqyre_domain::MaskShape::Rectangle
        };
        let mask = ProgramMask {
            name: new_name.to_string(),
            shape,
            center_x: self.form_center_x.trim().to_string(),
            center_y: self.form_center_y.trim().to_string(),
            base: self.form_base.trim().to_string(),
            height: self.form_height.trim().to_string(),
            radius: self.form_radius.trim().to_string(),
            inverse: self.form_inverse,
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_mask(&prog, new_name);
                }
                catalog.rename_mask(&prog, &old, new_name)?;
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_mask(&prog, mask)?;
        } else {
            catalog.upsert_mask(&prog, mask)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    pub(crate) fn update_collection(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        new_name: &str,
        overwrite: bool,
    ) -> Result<(), sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let rows = parse_i32(&self.form_rows).unwrap_or(1).max(1);
        let cols = parse_i32(&self.form_cols).unwrap_or(1).max(1);
        let col = ProgramCollection {
            name: new_name.to_string(),
            search_area: self.form_search_area.trim().to_string(),
            rows,
            cols,
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_collection(&prog, new_name);
                }
                catalog.rename_collection(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Collection, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_collection(&prog, col)?;
        } else {
            catalog.upsert_collection(&prog, col)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(())
    }

    pub(crate) fn on_delete(
        &mut self,
        db: &mut Database,
        macros: &mut [Macro],
        catalog: &mut ProgramCatalog,
        previews: &mut PreviewTooltipCache,
        settings: &mut UserSettings,
    ) {
        self.clear_status();
        if matches!(self.tab, EditorTab::Overlay) {
            let Some(id) = self.selected_entity.clone() else {
                return;
            };
            settings.overlay_buttons.retain(|b| b.id != id);
            if self.overlay_icon_picker_for.as_deref() == Some(id.as_str()) {
                self.overlay_icon_picker_for = None;
            }
            self.selected_entity = None;
            self.reset_overlay_form();
            self.persist_overlay_settings(settings);
            self.set_ok("Deleted overlay button.");
            return;
        }
        let deleted_name = self.selected_entity.clone();
        let result = match self.tab {
            EditorTab::Programs => {
                let Some(name) = self.selected_program.clone() else {
                    return;
                };
                catalog.delete_program(&name).map(|_| {
                    self.selected_program = None;
                    self.form_name.clear();
                })
            }
            EditorTab::Items => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_item(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.reset_item_form();
                })
            }
            EditorTab::Points => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_point(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.form_name.clear();
                })
            }
            EditorTab::SearchAreas => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_search_area(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.form_name.clear();
                })
            }
            EditorTab::Masks => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_mask(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.reset_mask_form();
                })
            }
            EditorTab::Collections => {
                let (Some(prog), Some(name)) = (
                    self.selected_program.clone(),
                    self.selected_entity.clone(),
                ) else {
                    return;
                };
                catalog.delete_collection(&prog, &name).map(|_| {
                    self.selected_entity = None;
                    self.reset_collection_form();
                })
            }
            EditorTab::AutoPic | EditorTab::Overlay => return,
        };
        match result {
            Ok(()) => {
                if matches!(self.tab, EditorTab::Points | EditorTab::SearchAreas) {
                    if let Some(name) = deleted_name.as_deref() {
                        previews.invalidate_entity(name);
                    }
                }
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    self.set_ok("Deleted.");
                }
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    pub(crate) fn persist(
        &mut self,
        db: &mut Database,
        macros: &[Macro],
        catalog: &mut ProgramCatalog,
    ) -> Result<(), String> {
        db.set_programs_from_catalog(catalog);
        db.replace_macros(macros.iter().cloned());
        db.save_default().map_err(|e| e.to_string())?;
        *catalog = db.program_catalog().map_err(|e| e.to_string())?;
        Ok(())
    }
}
