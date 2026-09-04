//! Shared mutation/resolve helpers for the program catalog.

use super::types::*;
use crate::fs_name::validate_fs_entity_name;
use crate::{PersistError, Result};
use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_ports::PortError;
use std::collections::BTreeMap;

/// 1-based slot + origin for the monitor that contains `(x, y)`.
/// Prefers containment; if none contain the point, uses the nearest monitor center.
/// Empty `rects` → slot 1 at `(0, 0)`.
pub fn monitor_slot_for_point(rects: &[MonitorRect], x: i32, y: i32) -> (u32, i32, i32) {
    if rects.is_empty() {
        return (1, 0, 0);
    }
    if let Some((i, &(ox, oy, _, _))) = rects
        .iter()
        .enumerate()
        .find(|(_, &(ox, oy, w, h))| x >= ox && y >= oy && x < ox + w && y < oy + h)
    {
        return ((i + 1) as u32, ox, oy);
    }
    let (i, &(ox, oy, _, _)) = rects
        .iter()
        .enumerate()
        .min_by_key(|(_, &(ox, oy, w, h))| {
            let cx = ox + w / 2;
            let cy = oy + h / 2;
            (x.abs_diff(cx) as u64).saturating_add(y.abs_diff(cy) as u64)
        })
        .expect("rects non-empty");
    ((i + 1) as u32, ox, oy)
}

/// Convert absolute desktop point → `(monitor, rel_x, rel_y)`.
pub fn absolute_point_to_relative(rects: &[MonitorRect], x: i32, y: i32) -> (u32, i32, i32) {
    let (slot, ox, oy) = monitor_slot_for_point(rects, x, y);
    (slot, x - ox, y - oy)
}

/// Clamp a search-area drag to the monitor of the **press** origin.
///
/// `press_x`/`press_y` must be the first click (before corner normalization). Using the
/// normalized top-left as the anchor mis-assigns the slot when the user presses on
/// monitor B and the sorted top-left lands on monitor A.
/// Returns `(monitor, rel_left, rel_top, rel_right, rel_bottom)`.
pub fn absolute_area_to_relative(
    rects: &[MonitorRect],
    press_x: i32,
    press_y: i32,
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
) -> (u32, i32, i32, i32, i32) {
    let (slot, ox, oy) = monitor_slot_for_point(rects, press_x, press_y);
    let (_, _, w, h) = rects
        .get(slot as usize - 1)
        .copied()
        .unwrap_or((0, 0, 1920, 1080));
    let clamp = |v: i32, lo: i32, hi: i32| v.clamp(lo, hi);
    let max_x = ox + w - 1;
    let max_y = oy + h - 1;
    let ax = clamp(ax, ox, max_x);
    let ay = clamp(ay, oy, max_y);
    let bx = clamp(bx, ox, max_x);
    let by = clamp(by, oy, max_y);
    let (left, top, right, bottom) = sqyre_ports::DesktopRect::normalize_corners(ax, ay, bx, by);
    (slot, left - ox, top - oy, right - ox, bottom - oy)
}

pub(super) fn ensure_resolution(p: &mut ProgramData, res: &str, scale: f32) {
    p.points.entry(res.to_string()).or_default();
    p.search_areas.entry(res.to_string()).or_default();
    p.coord_scales.entry(res.to_string()).or_insert(scale);
}

/// Upsert into a resolution-scoped map (`points` / `search_areas`).
pub(super) fn upsert_resolution_entity<T>(
    catalog: &mut ProgramCatalog,
    program: &str,
    key: String,
    value: T,
    maps: impl FnOnce(&mut ProgramData) -> &mut BTreeMap<String, BTreeMap<String, T>>,
) -> Result<()> {
    validate_fs_entity_name(key.trim())?;
    let res = catalog.default_resolution_key();
    let scale = catalog.runtime_scale();
    let p = catalog.program_mut(program)?;
    ensure_resolution(p, &res, scale);
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
    validate_fs_entity_name(new)?;
    let res = catalog.default_resolution_key();
    let scale = catalog.runtime_scale();
    let p = catalog.program_mut(program)?;
    ensure_resolution(p, &res, scale);
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
    validate_fs_entity_name(key.trim())?;
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
    validate_fs_entity_name(new)?;
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
) -> std::result::Result<(&'a ProgramPoint, &'a str), PortError> {
    let p = cat
        .programs
        .get(program)
        .ok_or_else(|| PortError::not_found(format!("program {program:?} not found")))?;
    if let Some((src_key, pts)) = p.points.get_key_value(resolution_key) {
        if let Some(pt) = pts.get(name) {
            return Ok((pt, src_key.as_str()));
        }
    }
    // Fall back to another bucket; caller remaps by source key dims/scale.
    for (src_key, pts) in &p.points {
        if let Some(pt) = pts.get(name) {
            return Ok((pt, src_key.as_str()));
        }
    }
    Err(PortError::not_found(format!(
        "point {name:?} not in {program}"
    )))
}

pub(super) fn search_area_from<'a>(
    cat: &'a ProgramCatalog,
    program: &str,
    name: &str,
    resolution_key: &str,
) -> std::result::Result<(&'a ProgramSearchArea, &'a str), PortError> {
    let p = cat
        .programs
        .get(program)
        .ok_or_else(|| PortError::not_found(format!("program {program:?} not found")))?;
    if let Some((src_key, areas)) = p.search_areas.get_key_value(resolution_key) {
        if let Some(sa) = areas.get(name) {
            return Ok((sa, src_key.as_str()));
        }
    }
    for (src_key, areas) in &p.search_areas {
        if let Some(sa) = areas.get(name) {
            return Ok((sa, src_key.as_str()));
        }
    }
    Err(PortError::not_found(format!(
        "search area {name:?} not in {program}"
    )))
}

pub(super) fn bucket_scale(program: &ProgramData, res_key: &str) -> f32 {
    program.coord_scales.get(res_key).copied().unwrap_or(1.0)
}

pub(super) fn collection_from<'a>(
    cat: &'a ProgramCatalog,
    program: &str,
    name: &str,
) -> std::result::Result<&'a ProgramCollection, PortError> {
    let p = cat
        .programs
        .get(program)
        .ok_or_else(|| PortError::not_found(format!("program {program:?} not found")))?;
    p.collections
        .get(name)
        .ok_or_else(|| PortError::not_found(format!("collection {name:?} not in {program}")))
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
) -> std::result::Result<(i32, i32, i32, i32), PortError> {
    if rows < 1 || cols < 1 {
        return Err(PortError::invalid(format!(
            "collection grid {rows}x{cols}: rows and cols must be >= 1"
        )));
    }
    let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    if r1 < 1 || c1 < 1 || r2 > rows || c2 > cols {
        return Err(PortError::invalid(format!(
            "cell range {r1},{c1}-{r2},{c2} out of bounds for {rows}x{cols} grid"
        )));
    }
    let width = right_x - left_x;
    let height = bottom_y - top_y;
    if width <= 0 || height <= 0 {
        return Err(PortError::invalid(format!(
            "invalid search area bounds {left_x},{top_y}-{right_x},{bottom_y}"
        )));
    }
    let cell_left = left_x + (c1 - 1) * width / cols;
    let cell_right = left_x + c2 * width / cols;
    let cell_top = top_y + (r1 - 1) * height / rows;
    let cell_bottom = top_y + r2 * height / rows;
    Ok((cell_left, cell_top, cell_right, cell_bottom))
}
