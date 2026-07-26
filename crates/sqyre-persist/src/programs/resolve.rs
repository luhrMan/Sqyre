//! Coordinate / entity resolution against the program catalog.

use super::{
    bucket_scale, cell_rect, collection_from, parse_resolution_key, point_from, remap_coord,
    search_area_from, split_target, ProgramCatalog, ProgramCollection, ProgramData, ProgramPoint,
    ProgramSearchArea,
};
use sqyre_domain::{resolve_scalar_int, CoordinateRef, Macro, PROGRAM_DELIMITER};
use sqyre_ports::PortError;
use std::path::PathBuf;

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
        self.remap_xy(x, y, src_key, data)
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
        let lx = resolve_scalar_int(&sa.left_x, &macro_.variables).map_err(PortError::invalid)?;
        let ty = resolve_scalar_int(&sa.top_y, &macro_.variables).map_err(PortError::invalid)?;
        let rx = resolve_scalar_int(&sa.right_x, &macro_.variables).map_err(PortError::invalid)?;
        let by = resolve_scalar_int(&sa.bottom_y, &macro_.variables).map_err(PortError::invalid)?;
        let (lx, ty) = self.remap_xy(lx, ty, src_key, data)?;
        let (rx, by) = self.remap_xy(rx, by, src_key, data)?;
        Ok((lx, ty, rx, by))
    }

    pub(super) fn remap_xy(
        &self,
        x: i32,
        y: i32,
        src_key: &str,
        data: &ProgramData,
    ) -> std::result::Result<(i32, i32), PortError> {
        let rt_key = self.resolution_key();
        if rt_key.is_empty() {
            return Ok((x, y));
        }
        let (src_w, src_h) = parse_resolution_key(src_key)?;
        let (rt_w, rt_h) = parse_resolution_key(rt_key)?;
        let src_scale = bucket_scale(data, src_key);
        let rt_scale = self.runtime_scale();
        Ok((
            remap_coord(x, src_w, rt_w, src_scale, rt_scale),
            remap_coord(y, src_h, rt_h, src_scale, rt_scale),
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
