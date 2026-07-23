//! One-way migration from legacy inline-coordinate `db.yaml` to the current schema.

use crate::{Database, Result};
use serde_yaml::{Mapping, Value};
use sqyre_domain::{VariableDecl, VariableType};
use std::collections::HashMap;

const LEGACY_BUILTIN_VARS: &[&str] = &[
    "cols",
    "rows",
    "foundx",
    "foundy",
    "itemname",
    "imagepixelwidth",
    "imagepixelheight",
    "stackmax",
    "merchant",
    "captchax",
    "captchay",
];

/// Build name → `Program~Entity` lookups from the `programs` section.
#[derive(Debug, Default)]
pub struct LegacyCatalog {
    search_areas: HashMap<String, String>,
    points: HashMap<String, String>,
}

impl LegacyCatalog {
    pub fn from_programs(programs: &Value) -> Self {
        let mut out = Self::default();
        let Some(mapping) = programs.as_mapping() else {
            return out;
        };
        for (prog_key, prog_val) in mapping {
            let Some(program) = prog_key.as_str() else {
                continue;
            };
            let Some(coords) = prog_val.get("coordinates").and_then(|v| v.as_mapping()) else {
                continue;
            };
            for (_res, block) in coords {
                let Some(block) = block.as_mapping() else {
                    continue;
                };
                if let Some(areas) = block.get("searchareas").and_then(|v| v.as_mapping()) {
                    for (area_key, area_val) in areas {
                        let key_name = area_key.as_str().unwrap_or("").to_string();
                        if key_name.is_empty() {
                            continue;
                        }
                        let display = area_val
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&key_name)
                            .to_string();
                        let reference = format!("{program}~{key_name}");
                        out.search_areas.insert(key_name.clone(), reference.clone());
                        if display != key_name {
                            out.search_areas.insert(display, reference);
                        }
                    }
                }
                if let Some(points) = block.get("points").and_then(|v| v.as_mapping()) {
                    for (point_key, point_val) in points {
                        let key_name = point_key.as_str().unwrap_or("").to_string();
                        if key_name.is_empty() {
                            continue;
                        }
                        let display = point_val
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&key_name)
                            .to_string();
                        let reference = format!("{program}~{key_name}");
                        out.points.insert(key_name.clone(), reference.clone());
                        if display != key_name {
                            out.points.insert(display, reference);
                        }
                    }
                }
            }
        }
        out
    }

    fn search_area_ref(&self, name: &str) -> String {
        self.search_areas
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    fn point_ref(&self, name: &str) -> String {
        self.points
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }
}

fn is_legacy_builtin(name: &str) -> bool {
    let key = name.trim().to_ascii_lowercase();
    LEGACY_BUILTIN_VARS.iter().any(|b| *b == key)
}

fn migrate_variables(vars: &Value) -> Option<Value> {
    let inner = vars
        .get("variables")
        .and_then(|v| v.as_mapping())
        .or_else(|| vars.as_mapping())?;
    if inner.is_empty() {
        return None;
    }
    let mut decls = Vec::new();
    for (k, v) in inner {
        let name = k.as_str().unwrap_or("").trim().to_string();
        if name.is_empty() || is_legacy_builtin(&name) {
            continue;
        }
        let (type_, initial) = match v {
            Value::Number(n) => (
                VariableType::Number,
                if let Some(i) = n.as_i64() {
                    i.to_string()
                } else {
                    n.to_string()
                },
            ),
            Value::Bool(b) => (VariableType::Text, b.to_string()),
            Value::String(s) => {
                if s.trim().parse::<i64>().is_ok() || s.trim().parse::<f64>().is_ok() {
                    (VariableType::Number, s.clone())
                } else {
                    (VariableType::Text, s.clone())
                }
            }
            _ => continue,
        };
        if initial.is_empty() {
            continue;
        }
        let decl = VariableDecl {
            name,
            type_,
            initial_value: initial,
            description: String::new(),
        };
        decls.push(serde_yaml::to_value(decl).ok()?);
    }
    if decls.is_empty() {
        return None;
    }
    Some(Value::Sequence(decls))
}

fn migrate_click_button(map: &mut Mapping) {
    let key = Value::String("button".into());
    let Some(Value::Bool(b)) = map.get(&key) else {
        return;
    };
    let label = if *b { "right" } else { "left" };
    map.insert(key, Value::String(label.into()));
}

fn migrate_waittilfound(map: &mut Mapping) {
    let key = Value::String("waittilfound".into());
    let Some(Value::Bool(true)) = map.get(&key) else {
        map.remove(&key);
        return;
    };
    map.remove(&key);
    map.insert(
        Value::String("repeatmode".into()),
        Value::String("waituntilfound".into()),
    );
}

fn migrate_coordinate_field(
    map: &mut Mapping,
    field: &str,
    catalog: &LegacyCatalog,
    lookup: fn(&LegacyCatalog, &str) -> String,
) {
    let key = Value::String(field.into());
    let Some(val) = map.get(&key).cloned() else {
        return;
    };
    let Some(area) = val.as_mapping() else {
        return;
    };
    let name = area
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() {
        map.remove(&key);
        return;
    }
    map.insert(key, Value::String(lookup(catalog, &name)));
}

fn migrate_action(map: &mut Mapping, catalog: &LegacyCatalog) {
    migrate_coordinate_field(map, "searcharea", catalog, LegacyCatalog::search_area_ref);
    migrate_coordinate_field(map, "point", catalog, LegacyCatalog::point_ref);
    migrate_click_button(map);
    migrate_waittilfound(map);
    map.remove(Value::String("rowsplit".into()));
    map.remove(Value::String("colsplit".into()));

    let sub_key = Value::String("subactions".into());
    if let Some(Value::Sequence(subs)) = map.get_mut(&sub_key) {
        for sub in subs.iter_mut() {
            if let Some(m) = sub.as_mapping_mut() {
                migrate_action(m, catalog);
            }
        }
    }
    let else_key = Value::String("elseactions".into());
    if let Some(Value::Sequence(subs)) = map.get_mut(&else_key) {
        for sub in subs.iter_mut() {
            if let Some(m) = sub.as_mapping_mut() {
                migrate_action(m, catalog);
            }
        }
    }
    // Drop obsolete miss-reuse flag; elseactions replaces it.
    map.remove(Value::String("runbranchonnofind".into()));
}

fn migrate_macro(map: &mut Mapping, catalog: &LegacyCatalog) {
    let vars_key = Value::String("variables".into());
    if let Some(vars) = map.get(&vars_key).cloned() {
        match migrate_variables(&vars) {
            Some(new_vars) => {
                map.insert(vars_key, new_vars);
            }
            None => {
                map.remove(&vars_key);
            }
        }
    }

    let root_key = Value::String("root".into());
    if let Some(Value::Mapping(root)) = map.get_mut(&root_key) {
        migrate_action(root, catalog);
    }
}

/// Migrate legacy `db.yaml` content to the current schema.
pub fn migrate_db_yaml_value(root: &mut Value) -> Result<()> {
    let Some(mapping) = root.as_mapping_mut() else {
        return Ok(());
    };

    let programs = mapping
        .get(Value::String("programs".into()))
        .cloned()
        .unwrap_or(Value::Mapping(Mapping::new()));
    let catalog = LegacyCatalog::from_programs(&programs);

    let macros_key = Value::String("macros".into());
    let Some(Value::Mapping(macros)) = mapping.get_mut(&macros_key) else {
        return Ok(());
    };

    for (_name, macro_val) in macros.iter_mut() {
        if let Some(m) = macro_val.as_mapping_mut() {
            migrate_macro(m, &catalog);
        }
    }

    Ok(())
}

/// Parse, migrate, and validate against the current [`Database`] loader.
pub fn migrate_db_yaml(text: &str) -> Result<String> {
    let mut root: Value = serde_yaml::from_str(text)?;
    migrate_db_yaml_value(&mut root)?;
    let out = serde_yaml::to_string(&root)?;
    Database::from_yaml(&out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_snippets() {
        let text = r#"
macros:
  Demo:
    name: Demo
    globaldelay: 0
    hotkey: []
    variables:
      variables:
        foundx: 1
        delay: 3
    root:
      type: loop
      name: root
      count: 1
      subactions:
        - type: click
          button: false
          state: true
        - type: move
          point:
            name: image search reference
            x: ${foundX}
            y: ${foundY}
          smooth: true
        - type: imagesearch
          name: ""
          targets: [Arena Breakout Infinite~Claim All]
          searcharea:
            name: Mail Area
            leftx: 1
            topy: 2
            rightx: 3
            bottomy: 4
          tolerance: 0.95
          waittilfound: true
          waittilfoundseconds: 5
programs:
  Arena Breakout Infinite:
    coordinates:
      2560x1440:
        searchareas:
          Mail Area:
            name: Mail Area
            leftx: 1
            topy: 2
            rightx: 3
            bottomy: 4
  windows 10:
    coordinates:
      2560x1440:
        points:
          image search reference:
            name: found image search
            x: ${foundX}
            y: ${foundY}
"#;
        let out = migrate_db_yaml(text).expect("migrate");
        let db = Database::from_yaml(&out).expect("load");
        let m = &db.macros["Demo"];
        assert_eq!(m.variable_decls.len(), 1);
        assert_eq!(m.variable_decls[0].name, "delay");
        match &m.root.children()[0].kind {
            sqyre_domain::ActionKind::Click { button, .. } => {
                assert_eq!(*button, sqyre_domain::MouseButton::Left);
            }
            other => panic!("expected click, got {other:?}"),
        }
        match &m.root.children()[1].kind {
            sqyre_domain::ActionKind::Move { point, .. } => {
                assert_eq!(point.as_str(), "windows 10~image search reference");
            }
            other => panic!("expected move, got {other:?}"),
        }
        match &m.root.children()[2].kind {
            sqyre_domain::ActionKind::ImageSearch {
                search_area,
                detection,
                ..
            } => {
                assert_eq!(search_area.as_str(), "Arena Breakout Infinite~Mail Area");
                assert_eq!(
                    detection.wait.repeat_mode,
                    sqyre_domain::RepeatMode::WaitUntilFound
                );
            }
            other => panic!("expected imagesearch, got {other:?}"),
        }
    }
}
