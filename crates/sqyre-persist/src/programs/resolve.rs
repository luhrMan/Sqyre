//! Coordinate / entity resolution against the program catalog.

use super::{
    bucket_scale, cell_rect, collection_from, point_from, search_area_from, split_target,
    ProgramCatalog, ProgramCollection, ProgramData, ProgramPoint, ProgramSearchArea,
};
use sqyre_domain::{resolve_scalar_int, CoordinateRef, Macro, ScalarValue, PROGRAM_DELIMITER};
use sqyre_ports::PortError;
use std::path::PathBuf;

/// Runtime screen coords (e.g. `${foundX}`) must not be remapped by catalog DPI/resolution.
fn scalar_has_runtime_ref(v: &ScalarValue) -> bool {
    matches!(v, ScalarValue::String(s) if s.contains("${"))
}

impl ProgramCatalog {
    pub fn lookup_point(
        &self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> std::result::Result<&ProgramPoint, PortError> {
        Ok(self.lookup_point_sourced(r, resolution_key)?.0)
    }

    /// Point plus the resolution bucket key that supplied it (for remapping).
    pub(super) fn lookup_point_sourced<'a>(
        &'a self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> std::result::Result<(&'a ProgramPoint, &'a str, &'a ProgramData), PortError> {
        if r.is_collection() {
            return Err(PortError::invalid(format!(
                "point lookup does not accept collection ref {r:?}"
            )));
        }
        let name = r.name();
        if name.is_empty() {
            return Err(PortError::invalid("empty point reference"));
        }
        if let Some(prog) = r.program() {
            let (pt, src) = point_from(self, prog, name, resolution_key)?;
            let data = self
                .programs
                .get(prog)
                .ok_or_else(|| PortError::not_found(format!("program {prog:?} not found")))?;
            return Ok((pt, src, data));
        }
        for prog in self.programs.keys() {
            if let Ok((pt, src)) = point_from(self, prog, name, resolution_key) {
                let data = self.programs.get(prog).expect("program exists");
                return Ok((pt, src, data));
            }
        }
        Err(PortError::not_found(format!("point {name:?} not found")))
    }

    pub fn lookup_search_area(
        &self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> std::result::Result<&ProgramSearchArea, PortError> {
        Ok(self.lookup_search_area_sourced(r, resolution_key)?.0)
    }

    pub(super) fn lookup_search_area_sourced<'a>(
        &'a self,
        r: &CoordinateRef,
        resolution_key: &str,
    ) -> std::result::Result<(&'a ProgramSearchArea, &'a str, &'a ProgramData), PortError> {
        if r.is_collection() {
            return Err(PortError::invalid(format!(
                "search area lookup does not accept collection ref {r:?}"
            )));
        }
        let name = r.name();
        if name.is_empty() {
            return Err(PortError::invalid("empty search area reference"));
        }
        if let Some(prog) = r.program() {
            let (sa, src) = search_area_from(self, prog, name, resolution_key)?;
            let data = self
                .programs
                .get(prog)
                .ok_or_else(|| PortError::not_found(format!("program {prog:?} not found")))?;
            return Ok((sa, src, data));
        }
        for prog in self.programs.keys() {
            if let Ok((sa, src)) = search_area_from(self, prog, name, resolution_key) {
                let data = self.programs.get(prog).expect("program exists");
                return Ok((sa, src, data));
            }
        }
        Err(PortError::not_found(format!(
            "search area {name:?} not found"
        )))
    }

    pub fn lookup_collection(
        &self,
        r: &CoordinateRef,
    ) -> std::result::Result<&ProgramCollection, PortError> {
        let name = r.name();
        if name.is_empty() {
            return Err(PortError::invalid("empty collection reference"));
        }
        if let Some(prog) = r.program() {
            return collection_from(self, prog, name);
        }
        for prog in self.programs.keys() {
            if let Ok(c) = collection_from(self, prog, name) {
                return Ok(c);
            }
        }
        Err(PortError::not_found(format!(
            "collection {name:?} not found"
        )))
    }

    pub fn resolve_point(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> std::result::Result<(i32, i32), PortError> {
        if r.is_collection() {
            let (lx, ty, rx, by) = self.resolve_search_area(r, macro_)?;
            return Ok(((lx + rx) / 2, (ty + by) / 2));
        }
        let key = self.resolution_key().to_string();
        let (pt, src_key, data) = self.lookup_point_sourced(r, &key)?;
        let x = resolve_scalar_int(&pt.x, &macro_.variables)
            .map_err(|e| PortError::invalid(format!("point X: {e}")))?;
        let y = resolve_scalar_int(&pt.y, &macro_.variables)
            .map_err(|e| PortError::invalid(format!("point Y: {e}")))?;
        // Image Search Reference (`${foundX}`/`${foundY}`) and similar points already
        // hold live screen pixels — remapping by stored bucket scale warps them on DPI≠100%.
        if scalar_has_runtime_ref(&pt.x) || scalar_has_runtime_ref(&pt.y) {
            return Ok((x, y));
        }
        let (x, y) = self.remap_monitor_relative(x, y, src_key, data)?;
        let abs = self.apply_monitor_origin(pt.monitor, x, y)?;
        Ok(abs)
    }

    pub fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> std::result::Result<(i32, i32, i32, i32), PortError> {
        if let Some((r1, c1, r2, c2)) = r.cell_range() {
            return self.resolve_collection_cells(r, macro_, r1, c1, r2, c2);
        }
        let key = self.resolution_key().to_string();
        let (sa, src_key, data) = self.lookup_search_area_sourced(r, &key)?;
        let lx = resolve_scalar_int(&sa.left_x, &macro_.variables)
            .map_err(|e| PortError::invalid(format!("search area left_x: {e}")))?;
        let ty = resolve_scalar_int(&sa.top_y, &macro_.variables)
            .map_err(|e| PortError::invalid(format!("search area top_y: {e}")))?;
        let rx = resolve_scalar_int(&sa.right_x, &macro_.variables)
            .map_err(|e| PortError::invalid(format!("search area right_x: {e}")))?;
        let by = resolve_scalar_int(&sa.bottom_y, &macro_.variables)
            .map_err(|e| PortError::invalid(format!("search area bottom_y: {e}")))?;
        if [&sa.left_x, &sa.top_y, &sa.right_x, &sa.bottom_y]
            .into_iter()
            .any(scalar_has_runtime_ref)
        {
            return Ok((lx, ty, rx, by));
        }
        let (lx, ty) = self.remap_monitor_relative(lx, ty, src_key, data)?;
        let (rx, by) = self.remap_monitor_relative(rx, by, src_key, data)?;
        let (lx, ty) = self.apply_monitor_origin(sa.monitor, lx, ty)?;
        let (rx, by) = self.apply_monitor_origin(sa.monitor, rx, by)?;
        Ok((lx, ty, rx, by))
    }

    /// Map a 1-based slot to its live origin. Missing slot → error (no silent clamp).
    fn monitor_origin(&self, monitor: u32) -> std::result::Result<(i32, i32), PortError> {
        let slot = monitor.max(1) as usize;
        let fallback = [(0, 0, 1920, 1080)];
        let rects = if self.monitor_rects.is_empty() {
            fallback.as_slice()
        } else {
            self.monitor_rects.as_slice()
        };
        let Some(&(ox, oy, _, _)) = rects.get(slot - 1) else {
            return Err(PortError::invalid(format!(
                "monitor slot {slot} not available ({} live monitor{})",
                rects.len(),
                if rects.len() == 1 { "" } else { "s" }
            )));
        };
        Ok((ox, oy))
    }

    fn apply_monitor_origin(
        &self,
        monitor: u32,
        x: i32,
        y: i32,
    ) -> std::result::Result<(i32, i32), PortError> {
        let (ox, oy) = self.monitor_origin(monitor)?;
        Ok((ox + x, oy + y))
    }

    /// Remap monitor-relative pixels: DPI scale only (not primary WxH).
    /// WxH remapping was for virtual-desktop absolutes and warps per-monitor relatives
    /// when the runtime resolution key is the leftmost monitor's size.
    fn remap_monitor_relative(
        &self,
        x: i32,
        y: i32,
        src_key: &str,
        data: &ProgramData,
    ) -> std::result::Result<(i32, i32), PortError> {
        let src_scale = bucket_scale(data, src_key);
        let rt_scale = self.runtime_scale();
        let src_scale = if src_scale > 0.0 { src_scale } else { 1.0 };
        let rt_scale = if rt_scale > 0.0 { rt_scale } else { 1.0 };
        if (src_scale - rt_scale).abs() < f32::EPSILON {
            return Ok((x, y));
        }
        let factor = rt_scale as f64 / src_scale as f64;
        Ok((
            (x as f64 * factor).round() as i32,
            (y as f64 * factor).round() as i32,
        ))
    }

    fn resolve_collection_cells(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
        r1: i32,
        c1: i32,
        r2: i32,
        c2: i32,
    ) -> std::result::Result<(i32, i32, i32, i32), PortError> {
        let col = self.lookup_collection(r)?;
        if col.search_area.is_empty() {
            return Err(PortError::invalid(format!(
                "collection {:?} has no search area",
                col.name
            )));
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
        let path = self.mask_image_path(program, &item.mask);
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    pub fn item_meta(&self, target: &str) -> Option<sqyre_ports::ItemMeta> {
        let (program, item) = split_target(target)?;
        let item = self.programs.get(program)?.items.get(item)?;
        Some(sqyre_ports::ItemMeta {
            name: item.name.clone(),
            stack_max: item.stack_max,
            cols: item.grid_cols,
            rows: item.grid_rows,
        })
    }
}
