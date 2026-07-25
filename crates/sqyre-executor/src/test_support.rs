//! Shared test doubles for executor unit/integration tests.

use crate::backends::CoordinateResolver;
use sqyre_domain::{CoordinateRef, Macro};
use std::collections::HashMap;

/// Collection grid metadata for [`FixedResolver`] atlas tests.
#[derive(Debug, Clone, Copy)]
pub struct FixedCollection {
    pub rows: i32,
    pub cols: i32,
    pub bounds: (i32, i32, i32, i32),
}

/// One atlas member passed to [`FixedResolver::with_atlas`].
#[derive(Debug, Clone)]
pub struct AtlasMemberSpec {
    pub name: String,
    pub collection: FixedCollection,
}

/// Fixed point + search-area resolver; optional collection grid / atlas.
#[derive(Debug, Clone)]
pub struct FixedResolver {
    pub point: (i32, i32),
    pub area: (i32, i32, i32, i32),
    pub grid: Option<(i32, i32)>,
    /// Collection name → grid; `None` for const/simple resolvers.
    pub collections: Option<HashMap<String, FixedCollection>>,
    pub atlas_members: Option<Vec<String>>,
}

impl FixedResolver {
    pub const fn point_area(point: (i32, i32), area: (i32, i32, i32, i32)) -> Self {
        Self {
            point,
            area,
            grid: None,
            collections: None,
            atlas_members: None,
        }
    }

    #[allow(dead_code)]
    pub const fn with_grid(rows: i32, cols: i32) -> Self {
        Self {
            point: (0, 0),
            area: (0, 0, 100, 100),
            grid: Some((rows, cols)),
            collections: None,
            atlas_members: None,
        }
    }

    pub fn with_atlas(collections: Vec<AtlasMemberSpec>, members: Vec<String>) -> Self {
        let mut map = HashMap::new();
        for spec in collections {
            map.insert(spec.name, spec.collection);
        }
        Self {
            point: (0, 0),
            area: (0, 0, 100, 100),
            grid: None,
            collections: Some(map),
            atlas_members: Some(members),
        }
    }
}

/// Default used by most search tests: point (0,0), area (100,200)-(110,210).
pub const SEARCH_FIXED_AREA: FixedResolver =
    FixedResolver::point_area((0, 0), (100, 200, 110, 210));

impl CoordinateResolver for FixedResolver {
    fn resolve_point(&self, r: &CoordinateRef, _macro_: &Macro) -> Result<(i32, i32), String> {
        if let Some((r1, c1, _, _)) = r.cell_range() {
            if let Some(cols) = &self.collections {
                if let Some(FixedCollection {
                    rows,
                    cols: cols_n,
                    bounds: (lx, ty, rx, by),
                }) = cols.get(r.name())
                {
                    let width = rx - lx;
                    let height = by - ty;
                    let cx = lx + (c1 - 1) * width / cols_n + width / (cols_n * 2);
                    let cy = ty + (r1 - 1) * height / rows + height / (rows * 2);
                    return Ok((cx, cy));
                }
            }
            if self.grid.is_some() {
                return Ok((c1 * 10, r1 * 10));
            }
        }
        Ok(self.point)
    }

    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), String> {
        if r.cell_range().is_some() {
            if let Some(cols) = &self.collections {
                if let Some(FixedCollection { bounds, .. }) = cols.get(r.name()) {
                    return Ok(*bounds);
                }
            }
        }
        Ok(self.area)
    }

    fn collection_grid(&self, _program: &str, collection: &str) -> Result<(i32, i32), String> {
        if let Some(cols) = &self.collections {
            if let Some(FixedCollection { rows, cols, .. }) = cols.get(collection) {
                return Ok((*rows, *cols));
            }
        }
        self.grid
            .ok_or_else(|| "collection grid lookup not configured".into())
    }

    fn atlas_members(&self, _program: &str, _atlas: &str) -> Result<Vec<String>, String> {
        self.atlas_members
            .clone()
            .ok_or_else(|| "atlas lookup not configured".into())
    }
}
