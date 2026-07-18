//! Shared mutation/resolve helpers for the program catalog.

use super::types::*;
use crate::{PersistError, Result};
use sqyre_domain::PROGRAM_DELIMITER;
use std::collections::BTreeMap;

pub(super) fn ensure_resolution(p: &mut ProgramData, res: &str) {
    p.points.entry(res.to_string()).or_default();
    p.search_areas.entry(res.to_string()).or_default();
}

/// Upsert into a resolution-scoped map (`points` / `search_areas`).
pub(super) fn upsert_resolution_entity<T>(
    catalog: &mut ProgramCatalog,
    program: &str,
    key: String,
    value: T,
    maps: impl FnOnce(&mut ProgramData) -> &mut BTreeMap<String, BTreeMap<String, T>>,
) -> Result<()> {
    let res = catalog.default_resolution_key();
    let p = catalog.program_mut(program)?;
    ensure_resolution(p, &res);
    maps(p).get_mut(&res).unwrap().insert(key, value);
    Ok(())
}

/// Delete from a resolution-scoped map.
pub(super) fn delete_resolution_entity<T>(
    catalog: &mut ProgramCatalog,
    program: &str,
    name: &str,
    kind: &str,
    maps: impl FnOnce(&mut ProgramData) -> &mut BTreeMap<String, BTreeMap<String, T>>,
) -> Result<()> {
    let res = catalog.default_resolution_key();
    let p = catalog.program_mut(program)?;
    let map = maps(p)
        .get_mut(&res)
        .ok_or_else(|| PersistError::Message(format!("no {kind}s for program {program}")))?;
    if map.remove(name).is_none() {
        return Err(PersistError::Message(format!("{kind} {name:?} not found")));
    }
    Ok(())
}

/// Rename inside a resolution-scoped map (ensures resolution bucket exists).
pub(super) fn rename_resolution_entity<T>(
    catalog: &mut ProgramCatalog,
    program: &str,
    old: &str,
    new: &str,
    kind: &str,
    maps: impl FnOnce(&mut ProgramData) -> &mut BTreeMap<String, BTreeMap<String, T>>,
    set_name: impl FnOnce(&mut T, String),
) -> Result<()> {
    let new = new.trim();
    let res = catalog.default_resolution_key();
    let p = catalog.program_mut(program)?;
    ensure_resolution(p, &res);
    let map = maps(p).get_mut(&res).unwrap();
    rename_keyed_map(map, old, new, kind, set_name)
}

/// Upsert into a flat program-level map (`items` / `masks` / `collections`).
pub(super) fn upsert_named_entity<T>(
    catalog: &mut ProgramCatalog,
    program: &str,
    key: String,
    value: T,
    map: impl FnOnce(&mut ProgramData) -> &mut BTreeMap<String, T>,
) -> Result<()> {
    let p = catalog.program_mut(program)?;
    map(p).insert(key, value);
    Ok(())
}

/// Delete from a flat program-level map.
pub(super) fn delete_named_entity<T>(
    catalog: &mut ProgramCatalog,
    program: &str,
    name: &str,
    kind: &str,
    map: impl FnOnce(&mut ProgramData) -> &mut BTreeMap<String, T>,
) -> Result<()> {
    let p = catalog.program_mut(program)?;
    if map(p).remove(name).is_none() {
        return Err(PersistError::Message(format!("{kind} {name:?} not found")));
    }
    Ok(())
}

/// Shared BTreeMap rename: empty-name / conflict / remove / set-name / reinsert.
/// Callers trim `new` and handle side effects (file renames, ref updates) outside.
pub(super) fn rename_keyed_map<T>(
    map: &mut BTreeMap<String, T>,
    old: &str,
    new: &str,
    kind: &str,
    set_name: impl FnOnce(&mut T, String),
) -> Result<()> {
    if new.is_empty() {
        return Err(PersistError::Message(format!(
            "{kind} name cannot be empty"
        )));
    }
    if old != new && map.contains_key(new) {
        return Err(PersistError::Message(format!(
            "{kind} {new:?} already exists"
        )));
    }
    let mut entry = map
        .remove(old)
        .ok_or_else(|| PersistError::Message(format!("{kind} {old:?} not found")))?;
    set_name(&mut entry, new.to_string());
    map.insert(new.to_string(), entry);
    Ok(())
}
pub(super) fn split_target(target: &str) -> Option<(&str, &str)> {
    let (a, b) = target.split_once(PROGRAM_DELIMITER)?;
    if a.is_empty() || b.is_empty() {
        None
    } else {
        Some((a, b))
    }
}

pub(super) fn point_from<'a>(
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

pub(super) fn search_area_from<'a>(
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

pub(super) fn collection_from<'a>(
    cat: &'a ProgramCatalog,
    program: &str,
    name: &str,
) -> std::result::Result<&'a ProgramCollection, String> {
    let p = cat
        .programs
        .get(program)
        .ok_or_else(|| format!("program {program:?} not found"))?;
    p.collections
        .get(name)
        .ok_or_else(|| format!("collection {name:?} not in {program}"))
}

/// Axis-aligned union of selected cells within search-area bounds (1-based inclusive).
#[allow(clippy::too_many_arguments)]
pub(super) fn cell_rect(
    left_x: i32,
    top_y: i32,
    right_x: i32,
    bottom_y: i32,
    rows: i32,
    cols: i32,
    r1: i32,
    c1: i32,
    r2: i32,
    c2: i32,
) -> std::result::Result<(i32, i32, i32, i32), String> {
    if rows < 1 || cols < 1 {
        return Err(format!(
            "collection grid {rows}x{cols}: rows and cols must be >= 1"
        ));
    }
    let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    if r1 < 1 || c1 < 1 || r2 > rows || c2 > cols {
        return Err(format!(
            "cell range {r1},{c1}-{r2},{c2} out of bounds for {rows}x{cols} grid"
        ));
    }
    let width = right_x - left_x;
    let height = bottom_y - top_y;
    if width <= 0 || height <= 0 {
        return Err(format!(
            "invalid search area bounds {left_x},{top_y}-{right_x},{bottom_y}"
        ));
    }
    let cell_left = left_x + (c1 - 1) * width / cols;
    let cell_right = left_x + c2 * width / cols;
    let cell_top = top_y + (r1 - 1) * height / rows;
    let cell_bottom = top_y + r2 * height / rows;
    Ok((cell_left, cell_top, cell_right, cell_bottom))
}
