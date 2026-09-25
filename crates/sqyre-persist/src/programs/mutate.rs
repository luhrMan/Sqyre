//! Program catalog mutations (CRUD, YAML encode, disk side effects).

use super::{
    delete_named_entity, delete_resolution_entity, encode_program, rename_keyed_map,
    rename_resolution_entity, upsert_named_entity, upsert_resolution_entity, ProgramAtlas,
    ProgramCatalog, ProgramCollection, ProgramData, ProgramItem, ProgramMask, ProgramPoint,
    ProgramSearchArea,
};
use crate::fs_name::{is_safe_fs_entity_name, validate_fs_entity_name};
use crate::{PersistError, Result};
use serde_yaml::{Mapping, Value};
use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_ports::PortError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

impl ProgramCatalog {
    pub fn programs_mut(&mut self) -> &mut BTreeMap<String, ProgramData> {
        self.bump_generation();
        &mut self.programs
    }

    /// Encode typed catalog fields, merging `masks` / `collections` / unknown keys from `previous`.
    pub fn to_yaml_value(&self, previous: &Value) -> Value {
        let empty_root = Mapping::new();
        let prev_map = match previous {
            Value::Mapping(m) => m,
            _ => &empty_root,
        };
        let empty_prog = Mapping::new();
        let mut out = Mapping::new();
        for (name, data) in &self.programs {
            let prev_prog = prev_map
                .get(Value::String(name.clone()))
                .and_then(|v| v.as_mapping())
                .unwrap_or(&empty_prog);
            out.insert(Value::String(name.clone()), encode_program(data, prev_prog));
        }
        Value::Mapping(out)
    }

    pub fn create_program(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        validate_fs_entity_name(name.trim())?;
        let name = name.trim().to_string();
        if self.programs.contains_key(&name) {
            return Err(PersistError::Message(format!(
                "program {name:?} already exists"
            )));
        }
        let res = self.default_resolution_key();
        let scale = self.runtime_scale();
        let mut data = ProgramData {
            name: name.clone(),
            ..Default::default()
        };
        if !res.is_empty() {
            data.points.insert(res.clone(), BTreeMap::new());
            data.search_areas.insert(res.clone(), BTreeMap::new());
            data.coord_scales.insert(res, scale);
        }
        self.programs.insert(name, data);
        self.bump_generation();
        Ok(())
    }

    pub fn rename_program(&mut self, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        validate_fs_entity_name(new)?;
        if old == new {
            return Ok(());
        }
        if self.programs.contains_key(new) {
            return Err(PersistError::Message(format!(
                "program {new:?} already exists"
            )));
        }
        let mut data = self
            .programs
            .remove(old)
            .ok_or_else(|| PersistError::Message(format!("program {old:?} not found")))?;
        data.name = new.to_string();
        self.programs.insert(new.to_string(), data);
        // Move item icons / masks / collections with the program so nested assets stay reachable.
        if is_safe_fs_entity_name(old) {
            self.rename_program_asset_dirs(old, new);
        }
        self.bump_generation();
        Ok(())
    }

    /// Rename `images/{icons|masks|Collections}/{old}` → `{new}` (best-effort).
    fn rename_program_asset_dirs(&self, old: &str, new: &str) {
        for (src, dst) in [
            (self.icons_dir(old), self.icons_dir(new)),
            (self.masks_dir(old), self.masks_dir(new)),
            (self.collections_dir(old), self.collections_dir(new)),
        ] {
            if !src.is_dir() {
                continue;
            }
            if dst.exists() {
                let _ = std::fs::remove_dir_all(&dst);
            }
            let _ = std::fs::rename(&src, &dst);
        }
        let src_icon = self.process_icon_path(old);
        if src_icon.is_file() {
            let dst_icon = self.process_icon_path(new);
            if let Some(parent) = dst_icon.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if dst_icon.exists() {
                let _ = std::fs::remove_file(&dst_icon);
            }
            let _ = std::fs::rename(&src_icon, &dst_icon);
        }
        crate::invalidate_icon_fs_cache_under(&self.icons_dir(old));
        crate::invalidate_icon_fs_cache_under(&self.icons_dir(new));
        crate::invalidate_icon_fs_cache_under(&self.masks_dir(old));
        crate::invalidate_icon_fs_cache_under(&self.masks_dir(new));
    }

    pub fn delete_program(&mut self, name: &str) -> Result<()> {
        if self.programs.remove(name).is_none() {
            return Err(PersistError::Message(format!("program {name:?} not found")));
        }
        // Only touch the filesystem when the name cannot escape the images root.
        if is_safe_fs_entity_name(name) {
            let icons = self.icons_dir(name);
            let masks = self.masks_dir(name);
            let collections = self.collections_dir(name);
            crate::invalidate_icon_fs_cache_under(&icons);
            crate::invalidate_icon_fs_cache_under(&masks);
            let _ = std::fs::remove_dir_all(&icons);
            let _ = std::fs::remove_dir_all(&masks);
            let _ = std::fs::remove_dir_all(collections);
            let _ = std::fs::remove_file(self.process_icon_path(name));
        }
        self.bump_generation();
        Ok(())
    }

    /// Bind a catalog program to a running OS window (`process_path` + `window_title`).
    ///
    /// Clearing the path (empty `process_path`) also removes any saved process icon PNG.
    pub fn set_process_binding(
        &mut self,
        program: &str,
        process_path: impl Into<String>,
        window_title: impl Into<String>,
    ) -> Result<()> {
        let process_path = process_path.into();
        let window_title = window_title.into();
        let clear_icon = process_path.trim().is_empty();
        {
            let p = self.program_mut(program)?;
            p.process_path = process_path;
            p.window_title = window_title;
        }
        if clear_icon {
            self.clear_process_icon(program);
        }
        Ok(())
    }

    /// Remove the persisted Running-program icon for `program` (best-effort).
    pub fn clear_process_icon(&self, program: &str) {
        if !is_safe_fs_entity_name(program) {
            return;
        }
        let path = self.process_icon_path(program);
        let _ = std::fs::remove_file(&path);
    }

    /// Set macro tags used for while-focused hotkey selection.
    pub fn set_program_tags(&mut self, program: &str, tags: Vec<String>) -> Result<()> {
        let p = self.program_mut(program)?;
        p.tags = tags;
        Ok(())
    }

    pub fn upsert_item(&mut self, program: &str, item: ProgramItem) -> Result<()> {
        let key = item.name.clone();
        upsert_named_entity(self, program, key, item, |p| &mut p.items)
    }

    pub fn rename_item(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        validate_fs_entity_name(new)?;
        {
            let p = self.program_mut(program)?;
            rename_keyed_map(&mut p.items, old, new, "item", |item, n| item.name = n)?;
        }
        if old != new && is_safe_fs_entity_name(program) && is_safe_fs_entity_name(old) {
            self.rename_item_icon_files(program, old, new);
        }
        Ok(())
    }

    /// Move `{old}.png` and `{old}~*.png` icon files to the new item name.
    fn rename_item_icon_files(&self, program: &str, old: &str, new: &str) {
        let dir = self.icons_dir(program);
        let prefix = format!("{old}{PROGRAM_DELIMITER}");
        let legacy = format!("{old}.png");
        visit_item_icon_files(&dir, old, |path, name| {
            let dest_name = if name == legacy {
                format!("{new}.png")
            } else {
                format!("{new}{PROGRAM_DELIMITER}{}", &name[prefix.len()..])
            };
            let dest = dir.join(dest_name);
            let _ = std::fs::rename(path, dest);
        });
        crate::invalidate_icon_fs_cache_under(&dir);
    }

    pub fn delete_item(&mut self, program: &str, name: &str) -> Result<()> {
        delete_named_entity(self, program, name, "item", |p| &mut p.items)?;
        if is_safe_fs_entity_name(program) && is_safe_fs_entity_name(name) {
            self.trash_item_icon_files(program, name);
        }
        Ok(())
    }

    /// Move `{name}.png` and `{name}~*.png` into `images/ScreenCap/trash/{program}/`.
    fn trash_item_icon_files(&self, program: &str, name: &str) {
        let dir = self.icons_dir(program);
        let trash = self.screen_cap_trash_dir(program);
        if std::fs::create_dir_all(&trash).is_err() {
            return;
        }
        visit_item_icon_files(&dir, name, |path, file_name| {
            let dest = unique_dest(&trash, file_name);
            let _ = std::fs::rename(path, dest);
        });
        crate::invalidate_icon_fs_cache_under(&dir);
    }

    pub fn upsert_point(&mut self, program: &str, point: ProgramPoint) -> Result<()> {
        let key = point.name.clone();
        upsert_resolution_entity(self, program, key, point, |p| &mut p.points)
    }

    pub fn rename_point(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        rename_resolution_entity(
            self,
            program,
            old,
            new,
            "point",
            |p| &mut p.points,
            |pt, n| pt.name = n,
        )
    }

    pub fn delete_point(&mut self, program: &str, name: &str) -> Result<()> {
        delete_resolution_entity(self, program, name, "point", |p| &mut p.points)
    }

    /// Remove every point in `program` (all resolution buckets).
    pub fn clear_points(&mut self, program: &str) -> Result<()> {
        let p = self.program_mut(program)?;
        for bucket in p.points.values_mut() {
            bucket.clear();
        }
        Ok(())
    }

    pub fn upsert_search_area(&mut self, program: &str, area: ProgramSearchArea) -> Result<()> {
        let key = area.name.clone();
        upsert_resolution_entity(self, program, key, area, |p| &mut p.search_areas)
    }

    pub fn rename_search_area(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        rename_resolution_entity(
            self,
            program,
            old,
            new,
            "search area",
            |p| &mut p.search_areas,
            |sa, n| sa.name = n,
        )?;
        // Propagate to collection.search_area references within this program.
        if old != new {
            let p = self.program_mut(program)?;
            for col in p.collections.values_mut() {
                if col.search_area == old {
                    col.search_area = new.to_string();
                }
            }
        }
        Ok(())
    }

    pub fn delete_search_area(&mut self, program: &str, name: &str) -> Result<()> {
        delete_resolution_entity(self, program, name, "search area", |p| &mut p.search_areas)?;
        let p = self.program_mut(program)?;
        for col in p.collections.values_mut() {
            if col.search_area == name {
                col.search_area.clear();
            }
        }
        Ok(())
    }

    pub fn upsert_mask(&mut self, program: &str, mask: ProgramMask) -> Result<()> {
        let key = mask.name.clone();
        upsert_named_entity(self, program, key, mask, |p| &mut p.masks)
    }

    pub fn rename_mask(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        validate_fs_entity_name(new)?;
        let old_path = self.mask_image_path(program, old);
        let new_path = self.mask_image_path(program, new);
        let p = self.program_mut(program)?;
        rename_keyed_map(&mut p.masks, old, new, "mask", |mask, n| mask.name = n)?;
        // Propagate to item.mask references within this program.
        for item in p.items.values_mut() {
            if item.mask == old {
                item.mask = new.to_string();
            }
        }
        if old != new
            && is_safe_fs_entity_name(program)
            && is_safe_fs_entity_name(old)
            && old_path.is_file()
        {
            if let Some(parent) = new_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(&old_path, &new_path);
            crate::invalidate_icon_fs_cache_under(&old_path);
            crate::invalidate_icon_fs_cache_under(&new_path);
        }
        Ok(())
    }

    pub fn delete_mask(&mut self, program: &str, name: &str) -> Result<()> {
        let path = self.mask_image_path(program, name);
        delete_named_entity(self, program, name, "mask", |p| &mut p.masks)?;
        let p = self.program_mut(program)?;
        for item in p.items.values_mut() {
            if item.mask == name {
                item.mask.clear();
            }
        }
        if is_safe_fs_entity_name(program) && is_safe_fs_entity_name(name) {
            crate::invalidate_icon_fs_cache_under(&path);
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    pub fn upsert_collection(
        &mut self,
        program: &str,
        collection: ProgramCollection,
    ) -> Result<()> {
        let key = collection.name.clone();
        upsert_named_entity(self, program, key, collection, |p| &mut p.collections)
    }

    pub fn rename_collection(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        validate_fs_entity_name(new)?;
        let old_path = self.collection_image_path(program, old);
        let new_path = self.collection_image_path(program, new);
        {
            let p = self.program_mut(program)?;
            rename_keyed_map(&mut p.collections, old, new, "collection", |col, n| {
                col.name = n
            })?;
            // Propagate to atlas member lists within this program.
            if old != new {
                for atlas in p.atlases.values_mut() {
                    for member in atlas.collections.iter_mut() {
                        if member == old {
                            *member = new.to_string();
                        }
                    }
                }
            }
        }
        if old != new
            && is_safe_fs_entity_name(program)
            && is_safe_fs_entity_name(old)
            && old_path.is_file()
        {
            if let Some(parent) = new_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(&old_path, &new_path);
        }
        Ok(())
    }

    pub fn delete_collection(&mut self, program: &str, name: &str) -> Result<()> {
        let path = self.collection_image_path(program, name);
        delete_named_entity(self, program, name, "collection", |p| &mut p.collections)?;
        let p = self.program_mut(program)?;
        for atlas in p.atlases.values_mut() {
            atlas.collections.retain(|c| c != name);
        }
        if is_safe_fs_entity_name(program) && is_safe_fs_entity_name(name) {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    pub fn upsert_atlas(&mut self, program: &str, atlas: ProgramAtlas) -> Result<()> {
        let key = atlas.name.clone();
        upsert_named_entity(self, program, key, atlas, |p| &mut p.atlases)
    }

    pub fn rename_atlas(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        validate_fs_entity_name(new)?;
        let p = self.program_mut(program)?;
        rename_keyed_map(&mut p.atlases, old, new, "atlas", |atlas, n| atlas.name = n)
    }

    pub fn delete_atlas(&mut self, program: &str, name: &str) -> Result<()> {
        delete_named_entity(self, program, name, "atlas", |p| &mut p.atlases)
    }

    pub fn lookup_atlas(
        &self,
        program: &str,
        name: &str,
    ) -> std::result::Result<&ProgramAtlas, PortError> {
        let p = self
            .programs
            .get(program)
            .ok_or_else(|| PortError::not_found(format!("program {program:?} not found")))?;
        p.atlases
            .get(name)
            .ok_or_else(|| PortError::not_found(format!("atlas {name:?} not in {program}")))
    }

    pub(crate) fn program_mut(&mut self, name: &str) -> Result<&mut ProgramData> {
        // Presence is checked up front so the generation is only bumped for a
        // lookup that will succeed; `get_mut` then borrows `self.programs` for
        // the returned reference, which rules out calling `&mut self` methods.
        if !self.programs.contains_key(name) {
            return Err(PersistError::Message(format!("program {name:?} not found")));
        }
        self.bump_generation();
        self.programs
            .get_mut(name)
            .ok_or_else(|| PersistError::Message(format!("program {name:?} not found")))
    }

    pub(crate) fn default_resolution_key(&self) -> String {
        let key = self.resolution_key();
        if key.is_empty() {
            "1920x1080".into()
        } else {
            key.to_string()
        }
    }

    /// Union catalogs; on name clashes, prefer `imported` (scalars and nested entities).
    pub fn merge_prefer_imported(&self, imported: &Self) -> Self {
        let mut out = Self {
            images_root: self.images_root.clone(),
            resolution_key: self.resolution_key.clone(),
            runtime_scale: self.runtime_scale,
            monitor_rects: self.monitor_rects.clone(),
            ..Default::default()
        };
        for (name, data) in &self.programs {
            out.programs.insert(name.clone(), data.clone());
        }
        for (name, imported_data) in &imported.programs {
            match out.programs.get_mut(name) {
                Some(live) => merge_program_data_prefer_imported(live, imported_data),
                None => {
                    out.programs.insert(name.clone(), imported_data.clone());
                }
            }
        }
        out.bump_generation();
        out
    }
}

fn visit_item_icon_files(dir: &Path, item: &str, mut visit: impl FnMut(PathBuf, &str)) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let prefix = format!("{item}{PROGRAM_DELIMITER}");
    let legacy = format!("{item}.png");
    for entry in rd.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == legacy || (name.starts_with(&prefix) && name.ends_with(".png")) {
            visit(entry.path(), name);
        }
    }
}

fn unique_dest(dir: &Path, file_name: &str) -> PathBuf {
    let dest = dir.join(file_name);
    if !dest.exists() {
        return dest;
    }
    let stem = dest
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    let ext = dest.extension().and_then(|s| s.to_str()).unwrap_or("png");
    let mut n = 2u32;
    loop {
        let candidate = dir.join(format!("{stem}_{n}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        if n == u32::MAX {
            return candidate;
        }
        n += 1;
    }
}

fn merge_map_prefer_imported<V: Clone>(
    live: &mut BTreeMap<String, V>,
    imported: &BTreeMap<String, V>,
) {
    for (k, v) in imported {
        live.insert(k.clone(), v.clone());
    }
}

fn merge_nested_maps_prefer_imported<V: Clone>(
    live: &mut BTreeMap<String, BTreeMap<String, V>>,
    imported: &BTreeMap<String, BTreeMap<String, V>>,
) {
    for (outer_key, imported_inner) in imported {
        let entry = live.entry(outer_key.clone()).or_default();
        merge_map_prefer_imported(entry, imported_inner);
    }
}

fn merge_program_data_prefer_imported(live: &mut ProgramData, imported: &ProgramData) {
    live.process_path = imported.process_path.clone();
    live.window_title = imported.window_title.clone();
    live.tags = imported.tags.clone();
    merge_nested_maps_prefer_imported(&mut live.points, &imported.points);
    merge_nested_maps_prefer_imported(&mut live.search_areas, &imported.search_areas);
    merge_map_prefer_imported(&mut live.coord_scales, &imported.coord_scales);
    merge_map_prefer_imported(&mut live.items, &imported.items);
    merge_map_prefer_imported(&mut live.masks, &imported.masks);
    merge_map_prefer_imported(&mut live.collections, &imported.collections);
    merge_map_prefer_imported(&mut live.atlases, &imported.atlases);
}
