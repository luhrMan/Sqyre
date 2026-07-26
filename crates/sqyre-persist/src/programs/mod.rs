//! Program catalog parsed from `db.yaml` programs section.

mod encode;
mod mutate;
mod resolve;
mod parse;
mod seed_general;
mod types;
mod util;

pub use seed_general::{
    ensure_general_program, MonitorRect, GENERAL_PROGRAM, IMAGE_SEARCH_REFERENCE,
};
pub use types::{
    ProgramAtlas, ProgramCatalog, ProgramCollection, ProgramData, ProgramItem, ProgramMask,
    ProgramPoint, ProgramSearchArea,
};

use crate::fs_name::{confined_join_or_invalid, is_safe_fs_entity_name, validate_fs_entity_name};
use crate::{images_path, PersistError, Result};
use encode::*;
use parse::*;
use serde_yaml::{Mapping, Value};
use sqyre_domain::{resolve_scalar_int, CoordinateRef, Macro, PROGRAM_DELIMITER};
use sqyre_ports::PortError;
use std::collections::BTreeMap;
use std::path::PathBuf;
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
            if let Some(k) = p.coord_scales.keys().next() {
                return k.as_str();
            }
        }
        ""
    }

    pub fn set_runtime_scale(&mut self, scale: f32) {
        self.runtime_scale = if scale > 0.0 { scale } else { 1.0 };
        self.bump_generation();
    }

    pub fn runtime_scale(&self) -> f32 {
        if self.runtime_scale > 0.0 {
            self.runtime_scale
        } else {
            1.0
        }
    }

    /// Monotonic counter bumped when programs/entities change (or resolution key).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// After a YAML/DB round-trip that rebuilt this catalog at generation 0, continue
    /// the prior counter so UI caches keyed on [`Self::generation`] invalidate.
    pub fn continue_generation_after_reload(&mut self, previous: u64) {
        self.generation = previous.wrapping_add(1);
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
        confined_join_or_invalid(&self.images_root().join("icons"), program)
    }

    pub fn masks_dir(&self, program: &str) -> PathBuf {
        confined_join_or_invalid(&self.images_root().join("masks"), program)
    }

    pub fn collections_dir(&self, program: &str) -> PathBuf {
        confined_join_or_invalid(&self.images_root().join("Collections"), program)
    }

    pub fn collection_image_path(&self, program: &str, collection: &str) -> PathBuf {
        let dir = self.collections_dir(program);
        if is_safe_fs_entity_name(collection) {
            dir.join(format!("{collection}.png"))
        } else {
            dir.join("__invalid__.png")
        }
    }

    pub fn mask_image_path(&self, program: &str, mask: &str) -> PathBuf {
        let dir = self.masks_dir(program);
        if is_safe_fs_entity_name(mask) {
            dir.join(format!("{mask}.png"))
        } else {
            dir.join("__invalid__.png")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::ScalarValue;

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
    fn atlas_roundtrip_and_collection_cascade() {
        let yaml = r#"
Game:
  name: Game
  items: {}
  coordinates:
    1920x1080:
      points: {}
      searchareas:
        BagArea:
          name: BagArea
          leftx: 0
          topy: 0
          rightx: 100
          bottomy: 100
        EquipArea:
          name: EquipArea
          leftx: 120
          topy: 0
          rightx: 220
          bottomy: 100
  masks: {}
  collections:
    Bag:
      name: Bag
      searcharea: BagArea
      rows: 2
      cols: 2
    Equip:
      name: Equip
      searcharea: EquipArea
      rows: 2
      cols: 2
  atlases:
    Inventory:
      name: Inventory
      collections:
        - Bag
        - Equip
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        let atlas = cat.lookup_atlas("Game", "Inventory").unwrap();
        assert_eq!(atlas.collections, vec!["Bag", "Equip"]);

        let encoded = cat.to_yaml_value(&Value::Mapping(Mapping::new()));
        let reparsed = ProgramCatalog::from_yaml_value(&encoded).unwrap();
        let atlas = reparsed.lookup_atlas("Game", "Inventory").unwrap();
        assert_eq!(atlas.name, "Inventory");
        assert_eq!(atlas.collections, vec!["Bag", "Equip"]);

        cat.rename_collection("Game", "Bag", "Satchel").unwrap();
        assert_eq!(
            cat.lookup_atlas("Game", "Inventory").unwrap().collections,
            vec!["Satchel", "Equip"]
        );
        assert_eq!(
            cat.get("Game").unwrap().collections["Satchel"].search_area,
            "BagArea"
        );

        cat.rename_search_area("Game", "BagArea", "PackArea")
            .unwrap();
        assert_eq!(
            cat.get("Game").unwrap().collections["Satchel"].search_area,
            "PackArea"
        );

        cat.delete_collection("Game", "Equip").unwrap();
        assert_eq!(
            cat.lookup_atlas("Game", "Inventory").unwrap().collections,
            vec!["Satchel"]
        );
    }

    #[test]
    fn resolve_remaps_by_resolution_ratio() {
        let yaml = r#"
Game:
  name: Game
  coordinates:
    1920x1080:
      scale: 1.0
      points:
        Spot:
          name: Spot
          x: 192
          y: 108
      searchareas:
        Box:
          name: Box
          leftx: 0
          topy: 0
          rightx: 192
          bottomy: 108
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        cat.set_resolution_key("2560x1440");
        cat.set_runtime_scale(1.0);
        let m = Macro::new("t", 0, vec![]);
        let (x, y) = cat
            .resolve_point(&CoordinateRef("Game~Spot".into()), &m)
            .unwrap();
        // 192 * 2560/1920 = 256, 108 * 1440/1080 = 144
        assert_eq!((x, y), (256, 144));
        let sa = cat
            .resolve_search_area(&CoordinateRef("Game~Box".into()), &m)
            .unwrap();
        assert_eq!(sa, (0, 0, 256, 144));
    }

    #[test]
    fn resolve_remaps_by_dpi_scale() {
        let yaml = r#"
Game:
  name: Game
  coordinates:
    1920x1080:
      scale: 1.0
      points:
        Spot:
          name: Spot
          x: 100
          y: 200
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        cat.set_resolution_key("1920x1080");
        cat.set_runtime_scale(1.5);
        let m = Macro::new("t", 0, vec![]);
        let (x, y) = cat
            .resolve_point(&CoordinateRef("Game~Spot".into()), &m)
            .unwrap();
        assert_eq!((x, y), (150, 300));
    }

    #[test]
    fn resolve_remaps_resolution_and_scale() {
        let yaml = r#"
Game:
  name: Game
  coordinates:
    1920x1080:
      scale: 1.0
      points:
        Spot:
          name: Spot
          x: 100
          y: 50
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        cat.set_resolution_key("2560x1440");
        cat.set_runtime_scale(1.5);
        let m = Macro::new("t", 0, vec![]);
        let (x, y) = cat
            .resolve_point(&CoordinateRef("Game~Spot".into()), &m)
            .unwrap();
        // 100 * (2560/1920) * 1.5 = 200, 50 * (1440/1080) * 1.5 = 100
        assert_eq!((x, y), (200, 100));
    }

    #[test]
    fn resolve_same_bucket_identity() {
        let yaml = r#"
Game:
  name: Game
  coordinates:
    1920x1080:
      scale: 1.0
      points:
        Spot:
          name: Spot
          x: 100
          y: 200
"#;
        let v: Value = serde_yaml::from_str(yaml).unwrap();
        let mut cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        cat.set_resolution_key("1920x1080");
        cat.set_runtime_scale(1.0);
        let m = Macro::new("t", 0, vec![]);
        let (x, y) = cat
            .resolve_point(&CoordinateRef("Game~Spot".into()), &m)
            .unwrap();
        assert_eq!((x, y), (100, 200));
    }

    #[test]
    fn upsert_stamps_bucket_scale() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.set_runtime_scale(1.25);
        cat.create_program("Demo").unwrap();
        cat.upsert_point(
            "Demo",
            ProgramPoint {
                name: "A".into(),
                x: ScalarValue::Int(1),
                y: ScalarValue::Int(2),
            },
        )
        .unwrap();
        let scale = cat.get("Demo").unwrap().coord_scales["1920x1080"];
        assert!((scale - 1.25).abs() < f32::EPSILON);
        let encoded = cat.to_yaml_value(&Value::Null);
        let reparsed = ProgramCatalog::from_yaml_value(&encoded).unwrap();
        let scale2 = reparsed.get("Demo").unwrap().coord_scales["1920x1080"];
        assert!((scale2 - 1.25).abs() < f32::EPSILON);
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

    #[test]
    fn continue_generation_after_yaml_reload() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("Demo").unwrap();
        let gen_after_mutate = cat.generation();
        assert!(gen_after_mutate > 0);

        // Simulate DataEditor::persist replacing the catalog from YAML.
        let yaml = cat.to_yaml_value(&Value::Null);
        let mut reloaded = ProgramCatalog::from_yaml_value(&yaml).unwrap();
        assert_eq!(reloaded.generation(), 0);
        reloaded.continue_generation_after_reload(gen_after_mutate);
        assert_eq!(reloaded.generation(), gen_after_mutate.wrapping_add(1));
    }
}
