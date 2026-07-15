//! Program catalog parsed from `db.yaml` programs section.

use crate::{images_path, PersistError, Result};
use serde_yaml::Value;
use sqyre_domain::{CoordinateRef, Macro, PROGRAM_DELIMITER, ScalarValue};
use sqyre_varref;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ProgramPoint {
    pub name: String,
    pub x: ScalarValue,
    pub y: ScalarValue,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramSearchArea {
    pub name: String,
    pub left_x: ScalarValue,
    pub top_y: ScalarValue,
    pub right_x: ScalarValue,
    pub bottom_y: ScalarValue,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramItem {
    pub name: String,
    pub mask: String,
    pub stack_max: i32,
    pub grid_cols: i32,
    pub grid_rows: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramData {
    pub name: String,
    /// resolution key → points
    pub points: BTreeMap<String, BTreeMap<String, ProgramPoint>>,
    pub search_areas: BTreeMap<String, BTreeMap<String, ProgramSearchArea>>,
    pub items: BTreeMap<String, ProgramItem>,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramCatalog {
    programs: BTreeMap<String, ProgramData>,
    /// Override for tests; empty → `images_path()`.
    images_root: Option<PathBuf>,
    /// Main monitor resolution key (Go `MainMonitorSizeString`). Empty → first key found.
    resolution_key: String,
}

impl ProgramCatalog {
    pub fn from_yaml_value(programs: &Value) -> Result<Self> {
        let mut out = Self::default();
        let mapping = match programs {
            Value::Mapping(m) => m,
            Value::Null => return Ok(out),
            _ => {
                return Err(PersistError::Message(
                    "programs must be a mapping".into(),
                ))
            }
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

    pub fn get(&self, name: &str) -> Option<&ProgramData> {
        self.programs.get(name)
    }

    pub fn program_names(&self) -> impl Iterator<Item = &String> {
        self.programs.keys()
    }

    fn images_root(&self) -> PathBuf {
        self.images_root
            .clone()
            .unwrap_or_else(images_path)
    }

    pub fn icons_dir(&self, program: &str) -> PathBuf {
        self.images_root().join("icons").join(program)
    }

    pub fn masks_dir(&self, program: &str) -> PathBuf {
        self.images_root().join("masks").join(program)
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

    pub fn resolve_point(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> std::result::Result<(i32, i32), String> {
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
        if r.is_collection() {
            return Err("collection cell ranges not implemented yet".into());
        }
        let key = self.resolution_key().to_string();
        let sa = self.lookup_search_area(r, &key)?;
        let lx = resolve_scalar_int(&sa.left_x, macro_)?;
        let ty = resolve_scalar_int(&sa.top_y, macro_)?;
        let rx = resolve_scalar_int(&sa.right_x, macro_)?;
        let by = resolve_scalar_int(&sa.bottom_y, macro_)?;
        Ok((lx, ty, rx, by))
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
        let path = self
            .masks_dir(program)
            .join(format!("{}.png", item.mask));
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    pub fn item_meta(&self, target: &str) -> Option<(String, i32, i32, i32)> {
        let (program, item) = split_target(target)?;
        let item = self.programs.get(program)?.items.get(item)?;
        Some((item.name.clone(), item.stack_max, item.grid_cols, item.grid_rows))
    }
}

fn split_target(target: &str) -> Option<(&str, &str)> {
    let (a, b) = target.split_once(PROGRAM_DELIMITER)?;
    if a.is_empty() || b.is_empty() {
        None
    } else {
        Some((a, b))
    }
}

fn point_from<'a>(
    cat: &'a ProgramCatalog,
    program: &str,
    name: &str,
    resolution_key: &str,
) -> std::result::Result<&'a ProgramPoint, String> {
    let p = cat
        .programs
        .get(program)
        .ok_or_else(|| format!("program {program:?} not found"))?;
    let pts = p
        .points
        .get(resolution_key)
        .or_else(|| p.points.values().next())
        .ok_or_else(|| format!("no points for program {program}"))?;
    pts.get(name)
        .ok_or_else(|| format!("point {name:?} not in {program}"))
}

fn search_area_from<'a>(
    cat: &'a ProgramCatalog,
    program: &str,
    name: &str,
    resolution_key: &str,
) -> std::result::Result<&'a ProgramSearchArea, String> {
    let p = cat
        .programs
        .get(program)
        .ok_or_else(|| format!("program {program:?} not found"))?;
    let areas = p
        .search_areas
        .get(resolution_key)
        .or_else(|| p.search_areas.values().next())
        .ok_or_else(|| format!("no search areas for program {program}"))?;
    areas
        .get(name)
        .ok_or_else(|| format!("search area {name:?} not in {program}"))
}

fn parse_program(name: &str, v: &Value) -> Result<ProgramData> {
    let mut data = ProgramData {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return Ok(data);
    };

    if let Some(Value::Mapping(items)) = map.get(Value::String("items".into())) {
        for (ik, iv) in items {
            let iname = ik.as_str().unwrap_or("").to_string();
            if iname.is_empty() {
                continue;
            }
            data.items.insert(iname.clone(), parse_item(&iname, iv));
        }
    }

    if let Some(Value::Mapping(coords)) = map.get(Value::String("coordinates".into())) {
        for (rk, rv) in coords {
            let res = rk.as_str().unwrap_or("").to_string();
            if res.is_empty() {
                continue;
            }
            let Some(block) = rv.as_mapping() else {
                continue;
            };
            if let Some(Value::Mapping(pts)) = block.get(Value::String("points".into())) {
                let mut m = BTreeMap::new();
                for (pk, pv) in pts {
                    let pname = pk.as_str().unwrap_or("").to_string();
                    if pname.is_empty() {
                        continue;
                    }
                    m.insert(pname.clone(), parse_point(&pname, pv));
                }
                data.points.insert(res.clone(), m);
            }
            if let Some(Value::Mapping(sas)) = block.get(Value::String("searchareas".into())) {
                let mut m = BTreeMap::new();
                for (sk, sv) in sas {
                    let sname = sk.as_str().unwrap_or("").to_string();
                    if sname.is_empty() {
                        continue;
                    }
                    m.insert(sname.clone(), parse_search_area(&sname, sv));
                }
                data.search_areas.insert(res, m);
            }
        }
    }
    Ok(data)
}

fn parse_item(name: &str, v: &Value) -> ProgramItem {
    let mut item = ProgramItem {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return item;
    };
    if let Some(n) = map.get(Value::String("name".into())).and_then(|x| x.as_str()) {
        item.name = n.to_string();
    }
    if let Some(m) = map.get(Value::String("mask".into())).and_then(|x| x.as_str()) {
        item.mask = m.to_string();
    }
    if let Some(n) = yaml_i64(map.get(Value::String("stackmax".into()))) {
        item.stack_max = n as i32;
    }
    if let Some(Value::Sequence(g)) = map.get(Value::String("gridsize".into())) {
        if g.len() >= 2 {
            item.grid_cols = yaml_i64(Some(&g[0])).unwrap_or(0) as i32;
            item.grid_rows = yaml_i64(Some(&g[1])).unwrap_or(0) as i32;
        }
    }
    item
}

fn parse_point(name: &str, v: &Value) -> ProgramPoint {
    let mut pt = ProgramPoint {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return pt;
    };
    if let Some(n) = map.get(Value::String("name".into())).and_then(|x| x.as_str()) {
        pt.name = n.to_string();
    }
    pt.x = scalar_field(map.get(Value::String("x".into())));
    pt.y = scalar_field(map.get(Value::String("y".into())));
    pt
}

fn parse_search_area(name: &str, v: &Value) -> ProgramSearchArea {
    let mut sa = ProgramSearchArea {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return sa;
    };
    if let Some(n) = map.get(Value::String("name".into())).and_then(|x| x.as_str()) {
        sa.name = n.to_string();
    }
    sa.left_x = scalar_field(map.get(Value::String("leftx".into())));
    sa.top_y = scalar_field(map.get(Value::String("topy".into())));
    sa.right_x = scalar_field(map.get(Value::String("rightx".into())));
    sa.bottom_y = scalar_field(map.get(Value::String("bottomy".into())));
    sa
}

fn scalar_field(v: Option<&Value>) -> ScalarValue {
    match v {
        Some(val) => ScalarValue::from_yaml(val),
        None => ScalarValue::Null,
    }
}

fn yaml_i64(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn resolve_scalar_int(v: &ScalarValue, macro_: &Macro) -> std::result::Result<i32, String> {
    match v {
        ScalarValue::Int(i) => Ok(*i as i32),
        ScalarValue::Float(f) => Ok(*f as i32),
        ScalarValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
        ScalarValue::Null => Ok(0),
        ScalarValue::String(s) => {
            let resolved = expand_vars(s, macro_)?;
            resolved
                .trim()
                .parse()
                .map_err(|_| format!("cannot parse int from {resolved:?}"))
        }
    }
}

fn expand_vars(text: &str, macro_: &Macro) -> std::result::Result<String, String> {
    let segs = sqyre_varref::segments(text);
    if segs.is_empty() {
        return Ok(text.to_string());
    }
    let mut out = String::new();
    for seg in segs {
        if !seg.is_ref {
            out.push_str(&seg.text);
            continue;
        }
        let val = macro_
            .variables
            .get(&seg.name)
            .ok_or_else(|| format!("unresolved variable ${{{}}}", seg.name))?;
        out.push_str(&val.as_display());
    }
    Ok(out)
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
}
