//! Create / update / delete / persist catalog entities.

use super::helpers::{is_editor_listed_program, new_overlay_button_id, parse_i32, unique_name};
use super::{DataEditor, DataEditorCtx, EditorTab, PendingConfirm};
use crate::icon_cache::IconCache;
use crate::overlay_icons;
use crate::preview_tooltip::PreviewTooltipCache;
use crate::window_types::ProcessIcon;
use sqyre_domain::{Macro, ProgramEntityKind, ScalarValue};
use sqyre_persist::{
    merge_fs_warning, Database, FsWarn, OverlayButtonConfig, ProgramAtlas, ProgramCatalog,
    ProgramCollection, ProgramItem, ProgramMask, ProgramPoint, ProgramSearchArea, UserSettings,
    DEFAULT_OVERLAY_BUTTON_SIZE,
};
use sqyre_validate::validate_entity_name;

fn play_ui_add_sound(settings: &UserSettings) {
    #[cfg(not(target_arch = "wasm32"))]
    crate::sound::play_add_sound_if(settings.play_ui_sounds, settings.sound_volume);
    #[cfg(target_arch = "wasm32")]
    let _ = settings;
}

fn play_ui_delete_sound(settings: &UserSettings) {
    #[cfg(not(target_arch = "wasm32"))]
    crate::sound::play_delete_sound_if(settings.play_ui_sounds, settings.sound_volume);
    #[cfg(target_arch = "wasm32")]
    let _ = settings;
}

/// Status after a successful save/delete when filesystem cascades may have failed.
fn status_with_fs_warn(ok_msg: &str, fs_warn: FsWarn) -> (String, bool) {
    match fs_warn {
        Some(w) => (format!("{ok_msg} Warning: {w}"), true),
        None => (ok_msg.to_string(), false),
    }
}

fn set_program_identity(
    catalog: &mut ProgramCatalog,
    icons: &mut IconCache,
    program: &str,
    process_path: String,
    window_title: String,
    tags: Vec<String>,
) -> Result<(), sqyre_persist::PersistError> {
    let prev_path = catalog
        .get(program)
        .map(|p| p.process_path.clone())
        .unwrap_or_default();
    catalog.set_process_binding(program, process_path.clone(), window_title.clone())?;
    catalog.set_program_tags(program, tags)?;
    persist_running_program_icon(
        catalog,
        icons,
        program,
        &prev_path,
        &process_path,
        &window_title,
    );
    Ok(())
}

/// Save the Running-program OS icon under `images/process/{program}.png`.
///
/// Uses picker/OS-retained RGBA first; otherwise asks the OS again while the
/// window may still be open. Empty binding is cleared by [`set_process_binding`].
/// When the process path changes and no fresh icon is available, drop any stale PNG.
fn persist_running_program_icon(
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    program: &str,
    prev_process_path: &str,
    process_path: &str,
    window_title: &str,
) {
    let path = process_path.trim();
    if path.is_empty() {
        return;
    }
    let path_changed = prev_process_path.trim() != path;
    let icon = icons
        .process_icon_bytes(path)
        .cloned()
        .or_else(|| fetch_process_icon(path, window_title));
    let Some(icon) = icon else {
        if path_changed {
            catalog.clear_process_icon(program);
            icons.invalidate_path(&catalog.process_icon_path(program));
        }
        return;
    };
    if let Err(e) = save_process_icon_png(catalog, program, icon) {
        crate::log::warn(format!("save process icon for {program:?}: {e}"));
        return;
    }
    // Prefer the freshly saved file next time live OS lookup misses.
    icons.invalidate_process(path);
    icons.invalidate_path(&catalog.process_icon_path(program));
}

fn fetch_process_icon(process_path: &str, window_title: &str) -> Option<ProcessIcon> {
    #[cfg(feature = "native-runtime")]
    {
        sqyre_capture::process_icon(process_path, window_title).map(|i| ProcessIcon {
            width: i.width,
            height: i.height,
            rgba: i.rgba,
        })
    }
    #[cfg(not(feature = "native-runtime"))]
    {
        let _ = (process_path, window_title);
        None
    }
}

fn save_process_icon_png(
    catalog: &ProgramCatalog,
    program: &str,
    icon: ProcessIcon,
) -> Result<(), String> {
    let dest = catalog.process_icon_path(program);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create process icons dir: {e}"))?;
    }
    let img = image::RgbaImage::from_raw(icon.width, icon.height, icon.rgba)
        .ok_or_else(|| "process icon rgba does not match width×height".to_string())?;
    img.save(&dest)
        .map_err(|e| format!("save {}: {e}", dest.display()))
}

fn new_entity_name(form_name: &str, default_base: &str, exists: impl Fn(&str) -> bool) -> String {
    let base = match form_name.trim() {
        "" => default_base,
        name => name,
    };
    unique_name(base, exists)
}

impl DataEditor {
    pub(crate) fn on_new(&mut self, env: &mut DataEditorCtx<'_>) {
        let DataEditorCtx {
            db,
            macros,
            catalog,
            screen_click,
            settings,
            ..
        } = env;
        self.clear_status();
        self.save_after_record = false;
        let created = match self.tab {
            EditorTab::Programs => {
                let requested = self.form_name.trim();
                if !requested.is_empty() && !is_editor_listed_program(requested) {
                    Err("That name is reserved for recording.".into())
                } else {
                    let name = new_entity_name(&self.form_name, "New Program", |n| {
                        catalog.get(n).is_some()
                    });
                    match catalog.create_program(&name) {
                        Ok(()) => {
                            self.selected_program = Some(name.clone());
                            self.form_name = name;
                            self.form_process_path.clear();
                            self.form_window_title.clear();
                            self.form_tags.clear();
                            self.tag_draft.clear();
                            Ok("Created program.")
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }
            }
            EditorTab::Items => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                let name = new_entity_name(&self.form_name, "New Item", |n| {
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
                let name = new_entity_name(&self.form_name, "New Point", |n| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(&prog)
                        .and_then(|p| p.points.get(res))
                        .and_then(|m| m.get(n))
                        .is_some()
                });
                let pt = ProgramPoint {
                    name: name.clone(),
                    monitor: 1,
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
                let name = new_entity_name(&self.form_name, "New Search Area", |n| {
                    let res = catalog.resolution_key();
                    catalog
                        .get(&prog)
                        .and_then(|p| p.search_areas.get(res))
                        .and_then(|m| m.get(n))
                        .is_some()
                });
                let sa = ProgramSearchArea {
                    name: name.clone(),
                    monitor: 1,
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
                let name = new_entity_name(&self.form_name, "New Mask", |n| {
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
                let name = new_entity_name(&self.form_name, "New Collection", |n| {
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
                    Ok(()) => match self.start_collection_capture(
                        catalog,
                        &prog,
                        &col,
                        Some((prog.clone(), name.clone())),
                    ) {
                        Ok(()) => {
                            self.selected_entity = Some(name);
                            self.load_form(catalog, settings);
                            Ok("Created collection; capturing image…")
                        }
                        Err(e) => {
                            let _ = catalog.delete_collection(&prog, &name);
                            Err(e)
                        }
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::Atlases => {
                let Some(prog) = self.selected_program.clone() else {
                    self.set_err("Select a program first.");
                    return;
                };
                if self.form_atlas_members.is_empty() {
                    self.set_err("Add at least one Collection to the Atlas.");
                    return;
                }
                let name = new_entity_name(&self.form_name, "New Atlas", |n| {
                    catalog.get(&prog).and_then(|p| p.atlases.get(n)).is_some()
                });
                let atlas = ProgramAtlas {
                    name: name.clone(),
                    collections: self.form_atlas_members.clone(),
                };
                match catalog.upsert_atlas(&prog, atlas) {
                    Ok(()) => {
                        self.selected_entity = Some(name);
                        self.load_form(catalog, settings);
                        Ok("Created atlas.")
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            EditorTab::ScreenCap => {
                self.set_err(format!(
                    "Use Save on the {} tab to write the preview screenshot.",
                    EditorTab::ScreenCap.label()
                ));
                return;
            }
            EditorTab::PixelCheck => {
                self.set_err(format!(
                    "{} is read-only — select an item to probe.",
                    EditorTab::PixelCheck.label()
                ));
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
                let (x, y) = super::overlay::default_overlay_xy(catalog, n);
                btn.x = x;
                btn.y = y;
                btn.size = DEFAULT_OVERLAY_BUTTON_SIZE;
                if let Some(first) = macros.first() {
                    btn.macro_name = first.name.clone();
                }
                let id = btn.id.clone();
                settings.overlay_buttons.push(btn);
                self.selected_entity = Some(id);
                self.load_form(catalog, settings);
                if self.persist_overlay_settings(settings) {
                    self.set_ok("Created overlay button.");
                    play_ui_add_sound(settings);
                }
                return;
            }
        };
        match created {
            Ok(msg) => {
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    play_ui_add_sound(settings);
                    match self.tab {
                        EditorTab::Points if !screen_click.is_armed() => {
                            self.save_after_record = true;
                            screen_click.arm_point();
                            self.set_ok(format!("{msg} Recording… left-click to capture X/Y."));
                        }
                        EditorTab::SearchAreas if !screen_click.is_armed() => {
                            self.save_after_record = true;
                            screen_click.arm_search_area();
                            self.set_ok(format!("{msg} Recording… click two corners."));
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
        env: &mut DataEditorCtx<'_>,
        previews: &mut PreviewTooltipCache,
    ) {
        // Check overwrite for renames onto existing keys
        if let Some((kind, name)) = self.would_overwrite(env.catalog) {
            self.confirm = Some(PendingConfirm::Overwrite { kind, name });
            return;
        }
        self.apply_update(env, false, previews);
    }

    pub(crate) fn would_overwrite(
        &self,
        catalog: &ProgramCatalog,
    ) -> Option<(&'static str, String)> {
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
            EditorTab::Atlases => {
                let prog = self.selected_program.as_deref()?;
                let old = self.selected_entity.as_deref()?;
                if old != new && catalog.get(prog).and_then(|p| p.atlases.get(new)).is_some() {
                    return Some(("Atlas", new.to_string()));
                }
            }
            EditorTab::ScreenCap | EditorTab::PixelCheck => {}
            EditorTab::Overlay => {}
        }
        None
    }

    pub(crate) fn apply_update(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        overwrite: bool,
        previews: &mut PreviewTooltipCache,
    ) {
        let DataEditorCtx {
            db,
            macros,
            catalog,
            settings,
            icons,
            ..
        } = env;
        self.clear_status();
        if matches!(self.tab, EditorTab::Overlay) {
            self.apply_overlay_update(settings);
            return;
        }
        let new_name = self.form_name.trim().to_string();
        if let Err(e) = validate_entity_name(&new_name) {
            self.set_err(format!("Invalid name: {e}"));
            return;
        }

        let old_entity = self.selected_entity.clone();
        let mut overlay_settings_dirty = false;
        let result: Result<FsWarn, sqyre_persist::PersistError> = match self.tab {
            EditorTab::Programs => {
                if let Some(old) = self.selected_program.clone() {
                    if old == new_name {
                        set_program_identity(
                            catalog,
                            icons,
                            &old,
                            self.form_process_path.clone(),
                            self.form_window_title.clone(),
                            self.form_tags.clone(),
                        )
                        .map(|()| None)
                    } else {
                        if overwrite {
                            let _ = catalog.delete_program(&new_name);
                        }
                        catalog.rename_program(&old, &new_name).and_then(|fs_warn| {
                            set_program_identity(
                                catalog,
                                icons,
                                &new_name,
                                self.form_process_path.clone(),
                                self.form_window_title.clone(),
                                self.form_tags.clone(),
                            )?;
                            for m in macros.iter_mut() {
                                m.rename_program(&old, &new_name);
                            }
                            for btn in settings.overlay_buttons.iter_mut() {
                                if btn.program == old {
                                    btn.program = new_name.clone();
                                }
                            }
                            let _ = settings.rename_overlay_point_program(&old, &new_name);
                            overlay_settings_dirty = true;
                            self.selected_program = Some(new_name.clone());
                            Ok(fs_warn)
                        })
                    }
                } else {
                    catalog.create_program(&new_name).and_then(|()| {
                        set_program_identity(
                            catalog,
                            icons,
                            &new_name,
                            self.form_process_path.clone(),
                            self.form_window_title.clone(),
                            self.form_tags.clone(),
                        )?;
                        self.selected_program = Some(new_name.clone());
                        Ok(None)
                    })
                }
            }
            EditorTab::Items => self.update_item(catalog, macros, &new_name, overwrite),
            EditorTab::Points => self.update_point(
                catalog,
                macros,
                settings,
                &mut overlay_settings_dirty,
                &new_name,
                overwrite,
            ),
            EditorTab::SearchAreas => self.update_search_area(
                catalog,
                macros,
                settings,
                &mut overlay_settings_dirty,
                &new_name,
                overwrite,
            ),
            EditorTab::Masks => self.update_mask(catalog, &new_name, overwrite),
            EditorTab::Collections => self.update_collection(catalog, macros, &new_name, overwrite),
            EditorTab::Atlases => self.update_atlas(catalog, macros, &new_name, overwrite),
            EditorTab::ScreenCap | EditorTab::PixelCheck | EditorTab::Overlay => Ok(None),
        };

        match result {
            Ok(fs_warn) => {
                if matches!(self.tab, EditorTab::Points | EditorTab::SearchAreas) {
                    if let Some(old) = old_entity.as_deref() {
                        previews.invalidate_entity(old);
                    }
                    previews.invalidate_entity(&new_name);
                }
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    let overlay_ok =
                        !overlay_settings_dirty || self.persist_overlay_settings(settings);
                    self.load_form(catalog, settings);
                    if overlay_ok {
                        let (msg, is_err) = status_with_fs_warn("Saved.", fs_warn);
                        if is_err {
                            self.set_err(msg);
                        } else {
                            self.set_ok(msg);
                        }
                    }
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
    ) -> Result<FsWarn, sqyre_persist::PersistError> {
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
        let mut fs_warn = None;
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    if let Ok(w) = catalog.delete_item(&prog, new_name) {
                        merge_fs_warning(&mut fs_warn, w);
                    }
                }
                merge_fs_warning(&mut fs_warn, catalog.rename_item(&prog, &old, new_name)?);
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
        Ok(fs_warn)
    }

    pub(crate) fn update_point(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        settings: &mut UserSettings,
        overlay_settings_dirty: &mut bool,
        new_name: &str,
        overwrite: bool,
    ) -> Result<FsWarn, sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let pt = ProgramPoint {
            name: new_name.to_string(),
            monitor: self.form_monitor.max(1),
            x: ScalarValue::parse_edit(&self.form_x),
            y: ScalarValue::parse_edit(&self.form_y),
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
                if settings.rename_overlay_point_entity(&prog, &old, new_name) {
                    *overlay_settings_dirty = true;
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_point(&prog, pt)?;
        } else {
            catalog.upsert_point(&prog, pt)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(None)
    }

    pub(crate) fn update_search_area(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        settings: &mut UserSettings,
        overlay_settings_dirty: &mut bool,
        new_name: &str,
        overwrite: bool,
    ) -> Result<FsWarn, sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        let sa = ProgramSearchArea {
            name: new_name.to_string(),
            monitor: self.form_monitor.max(1),
            left_x: ScalarValue::parse_edit(&self.form_left),
            top_y: ScalarValue::parse_edit(&self.form_top),
            right_x: ScalarValue::parse_edit(&self.form_right),
            bottom_y: ScalarValue::parse_edit(&self.form_bottom),
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
                if settings.rename_overlay_search_area_entity(&prog, &old, new_name) {
                    *overlay_settings_dirty = true;
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_search_area(&prog, sa)?;
        } else {
            catalog.upsert_search_area(&prog, sa)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(None)
    }

    pub(crate) fn update_mask(
        &mut self,
        catalog: &mut ProgramCatalog,
        new_name: &str,
        overwrite: bool,
    ) -> Result<FsWarn, sqyre_persist::PersistError> {
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
        let mut fs_warn = None;
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    if let Ok(w) = catalog.delete_mask(&prog, new_name) {
                        merge_fs_warning(&mut fs_warn, w);
                    }
                }
                merge_fs_warning(&mut fs_warn, catalog.rename_mask(&prog, &old, new_name)?);
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_mask(&prog, mask)?;
        } else {
            catalog.upsert_mask(&prog, mask)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(fs_warn)
    }

    pub(crate) fn update_collection(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        new_name: &str,
        overwrite: bool,
    ) -> Result<FsWarn, sqyre_persist::PersistError> {
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
        let mut fs_warn = None;
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    if let Ok(w) = catalog.delete_collection(&prog, new_name) {
                        merge_fs_warning(&mut fs_warn, w);
                    }
                }
                merge_fs_warning(
                    &mut fs_warn,
                    catalog.rename_collection(&prog, &old, new_name)?,
                );
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
        Ok(fs_warn)
    }

    pub(crate) fn update_atlas(
        &mut self,
        catalog: &mut ProgramCatalog,
        macros: &mut [Macro],
        new_name: &str,
        overwrite: bool,
    ) -> Result<FsWarn, sqyre_persist::PersistError> {
        let prog = self
            .selected_program
            .clone()
            .ok_or_else(|| sqyre_persist::PersistError::Message("no program".into()))?;
        if self.form_atlas_members.is_empty() {
            return Err(sqyre_persist::PersistError::Message(
                "atlas needs at least one collection".into(),
            ));
        }
        let known = catalog
            .get(&prog)
            .map(|p| {
                p.collections
                    .keys()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        for m in &self.form_atlas_members {
            if !known.contains(m) {
                return Err(sqyre_persist::PersistError::Message(format!(
                    "collection {m:?} not in program {prog}"
                )));
            }
        }
        let atlas = ProgramAtlas {
            name: new_name.to_string(),
            collections: self.form_atlas_members.clone(),
        };
        if let Some(old) = self.selected_entity.clone() {
            if old != new_name {
                if overwrite {
                    let _ = catalog.delete_atlas(&prog, new_name);
                }
                catalog.rename_atlas(&prog, &old, new_name)?;
                for m in macros.iter_mut() {
                    m.rename_program_entity(ProgramEntityKind::Atlas, &prog, &old, new_name);
                }
                self.selected_entity = Some(new_name.to_string());
            }
            catalog.upsert_atlas(&prog, atlas)?;
        } else {
            catalog.upsert_atlas(&prog, atlas)?;
            self.selected_entity = Some(new_name.to_string());
        }
        Ok(None)
    }

    pub(crate) fn on_delete(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        previews: &mut PreviewTooltipCache,
    ) {
        let DataEditorCtx {
            db,
            macros,
            catalog,
            settings,
            ..
        } = env;
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
            if self.persist_overlay_settings(settings) {
                self.set_ok("Deleted overlay button.");
                play_ui_delete_sound(settings);
            }
            return;
        }
        let deleted_name = self.selected_entity.clone();
        let result: Result<FsWarn, sqyre_persist::PersistError> = match self.tab {
            EditorTab::Programs => {
                let Some(name) = self.selected_program.clone() else {
                    return;
                };
                catalog.delete_program(&name).inspect(|_| {
                    if settings.remove_overlay_buttons_for_program(&name) {
                        if let Some(id) = self.overlay_icon_picker_for.as_deref() {
                            if !settings.overlay_buttons.iter().any(|b| b.id == id) {
                                self.overlay_icon_picker_for = None;
                            }
                        }
                        let _ = self.persist_overlay_settings(settings);
                    }
                    self.selected_program = None;
                    self.form_name.clear();
                })
            }
            EditorTab::Items => {
                let (Some(prog), Some(name)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                else {
                    return;
                };
                catalog.delete_item(&prog, &name).inspect(|_| {
                    self.selected_entity = None;
                    self.reset_item_form();
                })
            }
            EditorTab::Points => {
                let (Some(prog), Some(name)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                else {
                    return;
                };
                catalog.delete_point(&prog, &name).map(|()| {
                    if settings.clear_overlay_point_refs(&prog, &name) {
                        let _ = self.persist_overlay_settings(settings);
                    }
                    self.selected_entity = None;
                    self.form_name.clear();
                    None
                })
            }
            EditorTab::SearchAreas => {
                let (Some(prog), Some(name)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                else {
                    return;
                };
                catalog.delete_search_area(&prog, &name).map(|()| {
                    if settings.clear_overlay_search_area_refs(&prog, &name) {
                        let _ = self.persist_overlay_settings(settings);
                    }
                    self.selected_entity = None;
                    self.form_name.clear();
                    None
                })
            }
            EditorTab::Masks => {
                let (Some(prog), Some(name)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                else {
                    return;
                };
                catalog.delete_mask(&prog, &name).inspect(|_| {
                    self.selected_entity = None;
                    self.reset_mask_form();
                })
            }
            EditorTab::Collections => {
                let (Some(prog), Some(name)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                else {
                    return;
                };
                catalog.delete_collection(&prog, &name).inspect(|_| {
                    self.selected_entity = None;
                    self.reset_collection_form();
                })
            }
            EditorTab::Atlases => {
                let (Some(prog), Some(name)) =
                    (self.selected_program.clone(), self.selected_entity.clone())
                else {
                    return;
                };
                catalog.delete_atlas(&prog, &name).map(|()| {
                    self.selected_entity = None;
                    self.reset_atlas_form();
                    None
                })
            }
            EditorTab::ScreenCap | EditorTab::PixelCheck | EditorTab::Overlay => return,
        };
        match result {
            Ok(fs_warn) => {
                if matches!(self.tab, EditorTab::Points | EditorTab::SearchAreas) {
                    if let Some(name) = deleted_name.as_deref() {
                        previews.invalidate_entity(name);
                    }
                }
                if let Err(e) = self.persist(db, macros, catalog) {
                    self.set_err(e);
                } else {
                    play_ui_delete_sound(settings);
                    let (msg, is_err) = status_with_fs_warn("Deleted.", fs_warn);
                    if is_err {
                        self.set_err(msg);
                    } else {
                        self.set_ok(msg);
                    }
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
        crate::SqyreApp::persist_database_for_editor(db, macros, catalog)?;
        self.rebuild_list_cache(catalog);
        Ok(())
    }
}
