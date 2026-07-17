//! Program catalog parsed from `db.yaml` programs section.

mod encode;
mod parse;
mod types;
mod util;

pub use types::{
    ProgramCatalog, ProgramCollection, ProgramData, ProgramItem, ProgramMask, ProgramPoint,
    ProgramSearchArea,
};

use crate::{images_path, PersistError, Result};
use encode::*;
use parse::*;
use serde_yaml::{Mapping, Value};
use sqyre_domain::{resolve_scalar_int, CoordinateRef, Macro, ScalarValue, PROGRAM_DELIMITER};
use std::collections::BTreeMap;
use std::path::PathBuf;
use types::*;
use util::*;

impl ProgramCatalog {
    pub fn from_yaml_value(programs: &Value) -> Result<Self> {
        let mut out = Self::default();
        let mapping = match programs {
            Value::Mapping(m) => m,
            Value::Null => return Ok(out),
            _ => return Err(PersistError::Message("programs must be a mapping".into())),
        };
        for (k, v) in mapping {
            let name = k
                .as_str()
                .ok_or_else(|| PersistError::Message("program key must be string".into()))?
                .to_string();
            out.programs.insert(name.clone(), parse_program(&name, v)?);
        }
        Ok(out)
    }

    pub fn set_images_root(&mut self, path: Option<PathBuf>) {
        self.images_root = path;
    }

    pub fn set_resolution_key(&mut self, key: impl Into<String>) {
        self.resolution_key = key.into();
        self.bump_generation();
    }

    pub fn resolution_key(&self) -> &str {
        if !self.resolution_key.is_empty() {
            return &self.resolution_key;
        }
        for p in self.programs.values() {
            if let Some(k) = p.points.keys().next() {
                return k.as_str();
            }
            if let Some(k) = p.search_areas.keys().next() {
                return k.as_str();
            }
        }
        ""
    }

    /// Monotonic counter bumped when programs/entities change (or resolution key).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn get(&self, name: &str) -> Option<&ProgramData> {
        self.programs.get(name)
    }

    pub fn program_names(&self) -> impl Iterator<Item = &String> {
        self.programs.keys()
    }

    fn images_root(&self) -> PathBuf {
        self.images_root.clone().unwrap_or_else(images_path)
    }

    pub fn icons_dir(&self, program: &str) -> PathBuf {
        self.images_root().join("icons").join(program)
    }

    pub fn masks_dir(&self, program: &str) -> PathBuf {
        self.images_root().join("masks").join(program)
    }

    pub fn collections_dir(&self, program: &str) -> PathBuf {
        self.images_root().join("Collections").join(program)
    }

    pub fn collection_image_path(&self, program: &str, collection: &str) -> PathBuf {
        self.collections_dir(program)
            .join(format!("{collection}.png"))
    }

    pub fn mask_image_path(&self, program: &str, mask: &str) -> PathBuf {
        self.masks_dir(program).join(format!("{mask}.png"))
    }

    pub fn lookup_point(
        &self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> std::result::Result<&ProgramPoint, String> {
        if r.is_collection() {
            return Err(format!("point lookup does not accept collection ref {r:?}"));
        }
        let name = r.name();
        if name.is_empty() {
            return Err("empty point reference".into());
        }
        if let Some(prog) = r.program() {
            return point_from(self, prog, name, resolution_key);
        }
        for prog in self.programs.keys() {
            if let Ok(pt) = point_from(self, prog, name, resolution_key) {
                return Ok(pt);
            }
        }
        Err(format!("point {name:?} not found"))
    }

    pub fn lookup_search_area(
        &self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> std::result::Result<&ProgramSearchArea, String> {
        if r.is_collection() {
            return Err(format!(
                "search area lookup does not accept collection ref {r:?}"
            ));
        }
        let name = r.name();
        if name.is_empty() {
            return Err("empty search area reference".into());
        }
        if let Some(prog) = r.program() {
            return search_area_from(self, prog, name, resolution_key);
        }
        for prog in self.programs.keys() {
            if let Ok(sa) = search_area_from(self, prog, name, resolution_key) {
                return Ok(sa);
            }
        }
        Err(format!("search area {name:?} not found"))
    }

    pub fn lookup_collection(
        &self,
        r: &CoordinateRef,
    ) -> std::result::Result<&ProgramCollection, String> {
        let name = r.name();
        if name.is_empty() {
            return Err("empty collection reference".into());
        }
        if let Some(prog) = r.program() {
            return collection_from(self, prog, name);
        }
        for prog in self.programs.keys() {
            if let Ok(c) = collection_from(self, prog, name) {
                return Ok(c);
            }
        }
        Err(format!("collection {name:?} not found"))
    }

    pub fn resolve_point(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> std::result::Result<(i32, i32), String> {
        if r.is_collection() {
            let (lx, ty, rx, by) = self.resolve_search_area(r, macro_)?;
            return Ok(((lx + rx) / 2, (ty + by) / 2));
        }
        let key = self.resolution_key().to_string();
        let pt = self.lookup_point(r, &key)?;
        let x = resolve_scalar_int(&pt.x, macro_).map_err(|e| format!("point X: {e}"))?;
        let y = resolve_scalar_int(&pt.y, macro_).map_err(|e| format!("point Y: {e}"))?;
        Ok((x, y))
    }

    pub fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> std::result::Result<(i32, i32, i32, i32), String> {
        if let Some((r1, c1, r2, c2)) = r.cell_range() {
            return self.resolve_collection_cells(r, macro_, r1, c1, r2, c2);
        }
        let key = self.resolution_key().to_string();
        let sa = self.lookup_search_area(r, &key)?;
        let lx = resolve_scalar_int(&sa.left_x, macro_)?;
        let ty = resolve_scalar_int(&sa.top_y, macro_)?;
        let rx = resolve_scalar_int(&sa.right_x, macro_)?;
        let by = resolve_scalar_int(&sa.bottom_y, macro_)?;
        Ok((lx, ty, rx, by))
    }

    fn resolve_collection_cells(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
        r1: i32,
        c1: i32,
        r2: i32,
        c2: i32,
    ) -> std::result::Result<(i32, i32, i32, i32), String> {
        let col = self.lookup_collection(r)?;
        if col.search_area.is_empty() {
            return Err(format!("collection {:?} has no search area", col.name));
        }
        let sa_ref = match r.program() {
            Some(prog) => CoordinateRef(format!("{prog}{PROGRAM_DELIMITER}{}", col.search_area)),
            None => CoordinateRef(col.search_area.clone()),
        };
        let (left_x, top_y, right_x, bottom_y) = self.resolve_search_area(&sa_ref, macro_)?;
        cell_rect(
            left_x, top_y, right_x, bottom_y, col.rows, col.cols, r1, c1, r2, c2,
        )
    }

    /// `program~item` → icon PNG paths (variants + legacy).
    pub fn variant_paths(&self, target: &str) -> Vec<PathBuf> {
        let Some((program, item)) = split_target(target) else {
            return Vec::new();
        };
        let dir = self.icons_dir(program);
        let mut paths = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let prefix = format!("{item}{PROGRAM_DELIMITER}");
            let legacy = format!("{item}.png");
            for entry in rd.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name == legacy || (name.starts_with(&prefix) && name.ends_with(".png")) {
                    paths.push(entry.path());
                }
            }
        }
        paths.sort();
        paths
    }

    pub fn mask_path(&self, target: &str) -> Option<PathBuf> {
        let (program, item) = split_target(target)?;
        let item = self.programs.get(program)?.items.get(item)?;
        if item.mask.is_empty() {
            return None;
        }
        let path = self.masks_dir(program).join(format!("{}.png", item.mask));
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    pub fn item_meta(&self, target: &str) -> Option<(String, i32, i32, i32)> {
        let (program, item) = split_target(target)?;
        let item = self.programs.get(program)?.items.get(item)?;
        Some((
            item.name.clone(),
            item.stack_max,
            item.grid_cols,
            item.grid_rows,
        ))
    }

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
        if name.trim().is_empty() {
            return Err(PersistError::Message("program name cannot be empty".into()));
        }
        if self.programs.contains_key(&name) {
            return Err(PersistError::Message(format!(
                "program {name:?} already exists"
            )));
        }
        let res = self.default_resolution_key();
        let mut data = ProgramData {
            name: name.clone(),
            ..Default::default()
        };
        if !res.is_empty() {
            data.points.insert(res.clone(), BTreeMap::new());
            data.search_areas.insert(res, BTreeMap::new());
        }
        self.programs.insert(name, data);
        self.bump_generation();
        Ok(())
    }

    pub fn rename_program(&mut self, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        if new.is_empty() {
            return Err(PersistError::Message("program name cannot be empty".into()));
        }
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
        self.bump_generation();
        Ok(())
    }

    pub fn delete_program(&mut self, name: &str) -> Result<()> {
        if self.programs.remove(name).is_none() {
            return Err(PersistError::Message(format!("program {name:?} not found")));
        }
        let icons = self.icons_dir(name);
        let masks = self.masks_dir(name);
        let collections = self.collections_dir(name);
        let _ = std::fs::remove_dir_all(icons);
        let _ = std::fs::remove_dir_all(masks);
        let _ = std::fs::remove_dir_all(collections);
        self.bump_generation();
        Ok(())
    }

    /// Bind a catalog program to a running OS window (`process_path` + `window_title`).
    pub fn set_process_binding(
        &mut self,
        program: &str,
        process_path: impl Into<String>,
        window_title: impl Into<String>,
    ) -> Result<()> {
        let p = self.program_mut(program)?;
        p.process_path = process_path.into();
        p.window_title = window_title.into();
        Ok(())
    }

    pub fn upsert_item(&mut self, program: &str, item: ProgramItem) -> Result<()> {
        let p = self.program_mut(program)?;
        let key = item.name.clone();
        p.items.insert(key, item);
        Ok(())
    }

    pub fn rename_item(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        {
            let p = self.program_mut(program)?;
            rename_keyed_map(&mut p.items, old, new, "item", |item, n| item.name = n)?;
        }
        if old != new {
            self.rename_item_icon_files(program, old, new);
        }
        Ok(())
    }

    /// Move `{old}.png` and `{old}~*.png` icon files to the new item name.
    fn rename_item_icon_files(&self, program: &str, old: &str, new: &str) {
        let dir = self.icons_dir(program);
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return;
        };
        let prefix = format!("{old}{PROGRAM_DELIMITER}");
        let legacy = format!("{old}.png");
        for entry in rd.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let dest_name = if name.as_ref() == legacy {
                format!("{new}.png")
            } else if name.starts_with(&prefix) && name.ends_with(".png") {
                format!("{new}{PROGRAM_DELIMITER}{}", &name[prefix.len()..])
            } else {
                continue;
            };
            let dest = dir.join(dest_name);
            let _ = std::fs::rename(entry.path(), dest);
        }
    }

    pub fn delete_item(&mut self, program: &str, name: &str) -> Result<()> {
        let p = self.program_mut(program)?;
        if p.items.remove(name).is_none() {
            return Err(PersistError::Message(format!("item {name:?} not found")));
        }
        Ok(())
    }

    pub fn upsert_point(&mut self, program: &str, point: ProgramPoint) -> Result<()> {
        let res = self.default_resolution_key();
        let p = self.program_mut(program)?;
        ensure_resolution(p, &res);
        let key = point.name.clone();
        p.points.get_mut(&res).unwrap().insert(key, point);
        Ok(())
    }

    pub fn rename_point(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        let res = self.default_resolution_key();
        let p = self.program_mut(program)?;
        ensure_resolution(p, &res);
        let pts = p.points.get_mut(&res).unwrap();
        rename_keyed_map(pts, old, new, "point", |pt, n| pt.name = n)
    }

    pub fn delete_point(&mut self, program: &str, name: &str) -> Result<()> {
        let res = self.default_resolution_key();
        let p = self.program_mut(program)?;
        let pts = p
            .points
            .get_mut(&res)
            .ok_or_else(|| PersistError::Message(format!("no points for program {program}")))?;
        if pts.remove(name).is_none() {
            return Err(PersistError::Message(format!("point {name:?} not found")));
        }
        Ok(())
    }

    pub fn upsert_search_area(&mut self, program: &str, area: ProgramSearchArea) -> Result<()> {
        let res = self.default_resolution_key();
        let p = self.program_mut(program)?;
        ensure_resolution(p, &res);
        let key = area.name.clone();
        p.search_areas.get_mut(&res).unwrap().insert(key, area);
        Ok(())
    }

    pub fn rename_search_area(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        let res = self.default_resolution_key();
        let p = self.program_mut(program)?;
        ensure_resolution(p, &res);
        let areas = p.search_areas.get_mut(&res).unwrap();
        rename_keyed_map(areas, old, new, "search area", |sa, n| sa.name = n)
    }

    pub fn delete_search_area(&mut self, program: &str, name: &str) -> Result<()> {
        let res = self.default_resolution_key();
        let p = self.program_mut(program)?;
        let areas = p.search_areas.get_mut(&res).ok_or_else(|| {
            PersistError::Message(format!("no search areas for program {program}"))
        })?;
        if areas.remove(name).is_none() {
            return Err(PersistError::Message(format!(
                "search area {name:?} not found"
            )));
        }
        Ok(())
    }

    pub fn upsert_mask(&mut self, program: &str, mask: ProgramMask) -> Result<()> {
        let p = self.program_mut(program)?;
        let key = mask.name.clone();
        p.masks.insert(key, mask);
        Ok(())
    }

    pub fn rename_mask(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
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
        if old != new && old_path.is_file() {
            if let Some(parent) = new_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(&old_path, &new_path);
        }
        Ok(())
    }

    pub fn delete_mask(&mut self, program: &str, name: &str) -> Result<()> {
        let path = self.mask_image_path(program, name);
        let p = self.program_mut(program)?;
        if p.masks.remove(name).is_none() {
            return Err(PersistError::Message(format!("mask {name:?} not found")));
        }
        for item in p.items.values_mut() {
            if item.mask == name {
                item.mask.clear();
            }
        }
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    pub fn upsert_collection(
        &mut self,
        program: &str,
        collection: ProgramCollection,
    ) -> Result<()> {
        let p = self.program_mut(program)?;
        let key = collection.name.clone();
        p.collections.insert(key, collection);
        Ok(())
    }

    pub fn rename_collection(&mut self, program: &str, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        let old_path = self.collection_image_path(program, old);
        let new_path = self.collection_image_path(program, new);
        {
            let p = self.program_mut(program)?;
            rename_keyed_map(&mut p.collections, old, new, "collection", |col, n| {
                col.name = n
            })?;
        }
        if old != new && old_path.is_file() {
            if let Some(parent) = new_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(&old_path, &new_path);
        }
        Ok(())
    }

    pub fn delete_collection(&mut self, program: &str, name: &str) -> Result<()> {
        let path = self.collection_image_path(program, name);
        let p = self.program_mut(program)?;
        if p.collections.remove(name).is_none() {
            return Err(PersistError::Message(format!(
                "collection {name:?} not found"
            )));
        }
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    fn program_mut(&mut self, name: &str) -> Result<&mut ProgramData> {
        if !self.programs.contains_key(name) {
            return Err(PersistError::Message(format!("program {name:?} not found")));
        }
        self.bump_generation();
        Ok(self.programs.get_mut(name).expect("program exists"))
    }

    fn default_resolution_key(&self) -> String {
        let key = self.resolution_key();
        if key.is_empty() {
            "1920x1080".into()
        } else {
            key.to_string()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_points_and_resolves() {
        let yaml = r#"
Schedule 1:
  name: Schedule 1
  items:
    Collect:
      name: Collect
      mask: ""
      stackmax: 0
      gridsize: [1, 2]
  coordinates:
    2560x1440:
      points:
        Spot:
          name: Spot
          x: 100
          y: 200
      searchareas:
        Box:
          name: Box
          leftx: 10
          topy: 20
          rightx: 30
          bottomy: 40
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        let m = Macro::new("t", 0, vec![]);
        let (x, y) = cat
            .resolve_point(&CoordinateRef("Schedule 1~Spot".into()), &m)
            .unwrap();
        assert_eq!((x, y), (100, 200));
        let sa = cat
            .resolve_search_area(&CoordinateRef("Schedule 1~Box".into()), &m)
            .unwrap();
        assert_eq!(sa, (10, 20, 30, 40));
    }

    #[test]
    fn resolves_point_arithmetic_expressions() {
        let yaml = r#"
general:
  name: general
  coordinates:
    2560x1440:
      points:
        Main Monitor Screen Top Middle:
          name: Main Monitor Screen Top Middle
          x: "2560+(1920/2)"
          y: "0+(10)"
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        let m = Macro::new("t", 0, vec![]);
        let (x, y) = cat
            .resolve_point(
                &CoordinateRef("general~Main Monitor Screen Top Middle".into()),
                &m,
            )
            .unwrap();
        assert_eq!((x, y), (3520, 10));
    }

    #[test]
    fn roundtrip_preserves_masks_and_tags() {
        let yaml = r#"
Game:
  name: Game
  items:
    Potion:
      name: Potion
      mask: circle
      stackmax: 5
      gridsize: [2, 3]
      tags: [consumable, healing]
  coordinates:
    1920x1080:
      points:
        Spawn:
          name: Spawn
          x: 1
          y: 2
      searchareas: {}
  masks:
    circle:
      name: circle
      shape: circle
      centerx: "50"
      centery: "50"
      base: ""
      height: ""
      radius: "10"
      inverse: true
  collections:
    Bag:
      name: Bag
      searcharea: Box
      rows: 2
      cols: 3
"#;
        let previous: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&previous).unwrap();
        assert_eq!(
            cat.get("Game").unwrap().items["Potion"].tags,
            vec!["consumable", "healing"]
        );

        cat.upsert_point(
            "Game",
            ProgramPoint {
                name: "Spawn".into(),
                x: ScalarValue::Int(10),
                y: ScalarValue::Int(20),
            },
        )
        .unwrap();

        let encoded = cat.to_yaml_value(&previous);
        let prog = encoded
            .as_mapping()
            .unwrap()
            .get(Value::String("Game".into()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert!(prog.contains_key(Value::String("masks".into())));
        assert!(prog.contains_key(Value::String("collections".into())));
        let masks = prog.get(Value::String("masks".into())).unwrap();
        assert!(masks
            .as_mapping()
            .unwrap()
            .contains_key(Value::String("circle".into())));

        let reparsed = ProgramCatalog::from_yaml_value(&encoded).unwrap();
        let item = &reparsed.get("Game").unwrap().items["Potion"];
        assert_eq!(item.tags, vec!["consumable", "healing"]);
        assert_eq!(item.mask, "circle");
        let pt = &reparsed.get("Game").unwrap().points["1920x1080"]["Spawn"];
        assert_eq!(pt.x, ScalarValue::Int(10));
        assert_eq!(pt.y, ScalarValue::Int(20));
        let mask = &reparsed.get("Game").unwrap().masks["circle"];
        assert_eq!(mask.shape, sqyre_domain::MaskShape::Circle);
        assert!(mask.inverse);
        assert_eq!(mask.radius, "10");
        let col = &reparsed.get("Game").unwrap().collections["Bag"];
        assert_eq!(col.search_area, "Box");
        assert_eq!((col.rows, col.cols), (2, 3));
    }

    #[test]
    fn resolve_collection_cell_range() {
        let yaml = r#"
Demo:
  name: Demo
  items: {}
  coordinates:
    1920x1080:
      points: {}
      searchareas:
        inv:
          name: inv
          leftx: 0
          topy: 0
          rightx: 100
          bottomy: 100
  masks: {}
  collections:
    grid:
      name: grid
      searcharea: inv
      rows: 2
      cols: 2
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        let m = Macro::new("t", 0, vec![]);
        let rect = cat
            .resolve_search_area(&CoordinateRef("Demo~grid@1,1-1,1".into()), &m)
            .unwrap();
        assert_eq!(rect, (0, 0, 50, 50));
        let center = cat
            .resolve_point(&CoordinateRef("Demo~grid@1,1-1,1".into()), &m)
            .unwrap();
        assert_eq!(center, (25, 25));
    }

    #[test]
    fn create_rename_delete_program() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("2560x1440");
        cat.create_program("Alpha").unwrap();
        assert!(cat.get("Alpha").is_some());
        cat.rename_program("Alpha", "Beta").unwrap();
        assert!(cat.get("Alpha").is_none());
        assert_eq!(cat.get("Beta").unwrap().name, "Beta");
        cat.delete_program("Beta").unwrap();
        assert!(cat.get("Beta").is_none());
    }

    #[test]
    fn process_binding_roundtrip() {
        let yaml = r#"
Demo:
  name: Demo
  processpath: /opt/demo/bin/DemoGame
  windowtitle: Demo Game
  items: {}
  coordinates: {}
  masks: {}
  collections: {}
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        let p = cat.get("Demo").unwrap();
        assert_eq!(p.process_path, "/opt/demo/bin/DemoGame");
        assert_eq!(p.window_title, "Demo Game");
        cat.set_process_binding("Demo", "/usr/bin/other", "Other")
            .unwrap();
        let encoded = cat.to_yaml_value(&Value::Null);
        let cat2 = ProgramCatalog::from_yaml_value(&encoded).unwrap();
        let p2 = cat2.get("Demo").unwrap();
        assert_eq!(p2.process_path, "/usr/bin/other");
        assert_eq!(p2.window_title, "Other");
    }

    #[test]
    fn database_set_programs_from_catalog_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        crate::with_sqyre_dir_override(dir.path().to_path_buf(), || {
            crate::initialize_directories().unwrap();

            let mut cat = ProgramCatalog::default();
            cat.set_resolution_key("1920x1080");
            cat.create_program("Demo").unwrap();
            cat.upsert_item(
                "Demo",
                ProgramItem {
                    name: "Gem".into(),
                    mask: String::new(),
                    stack_max: 3,
                    grid_cols: 2,
                    grid_rows: 2,
                    tags: vec!["loot".into()],
                },
            )
            .unwrap();
            cat.upsert_point(
                "Demo",
                ProgramPoint {
                    name: "A".into(),
                    x: ScalarValue::Int(5),
                    y: ScalarValue::Int(6),
                },
            )
            .unwrap();
            cat.upsert_search_area(
                "Demo",
                ProgramSearchArea {
                    name: "Zone".into(),
                    left_x: ScalarValue::Int(0),
                    top_y: ScalarValue::Int(0),
                    right_x: ScalarValue::Int(50),
                    bottom_y: ScalarValue::Int(50),
                },
            )
            .unwrap();

            let mut db = crate::Database::default();
            db.set_programs_from_catalog(&cat);
            db.save_default().unwrap();

            let loaded = crate::Database::load_default().unwrap();
            let cat2 = loaded.program_catalog().unwrap();
            assert!(cat2.get("Demo").is_some());
            assert_eq!(cat2.get("Demo").unwrap().items["Gem"].tags, vec!["loot"]);
            assert_eq!(
                cat2.get("Demo").unwrap().points["1920x1080"]["A"].x,
                ScalarValue::Int(5)
            );
        });
    }
}
