//! Shared test doubles for executor unit/integration tests.

use crate::backends::CoordinateResolver;
use sqyre_domain::{CoordinateRef, Macro};

/// Fixed point + search-area resolver; optional collection grid.
#[derive(Debug, Clone, Copy)]
pub struct FixedResolver {
    pub point: (i32, i32),
    pub area: (i32, i32, i32, i32),
    pub grid: Option<(i32, i32)>,
}

impl FixedResolver {
    pub const fn point_area(point: (i32, i32), area: (i32, i32, i32, i32)) -> Self {
        Self {
            point,
            area,
            grid: None,
        }
    }

    pub const fn with_grid(rows: i32, cols: i32) -> Self {
        Self {
            point: (0, 0),
            area: (0, 0, 100, 100),
            grid: Some((rows, cols)),
        }
    }
}

/// Default used by most search tests: point (0,0), area (100,200)-(110,210).
pub const SEARCH_FIXED_AREA: FixedResolver = FixedResolver::point_area((0, 0), (100, 200, 110, 210));

impl CoordinateResolver for FixedResolver {
    fn resolve_point(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<(i32, i32), String> {
        if self.grid.is_some() {
            let (r1, c1, _, _) = r.cell_range().ok_or("expected cell")?;
            return Ok((c1 * 10, r1 * 10));
        }
        Ok(self.point)
    }

    fn resolve_search_area(
        &self,
        _r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), String> {
        Ok(self.area)
    }

    fn collection_grid(&self, _program: &str, _collection: &str) -> Result<(i32, i32), String> {
        self.grid
            .ok_or_else(|| "collection grid lookup not configured".into())
    }
}
