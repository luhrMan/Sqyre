//! Pure atlas layout: Collections placed by screen bounds, neighbors by geometry.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// One Collection member of an atlas, with resolved pixel bounds and grid size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasNode {
    pub collection: String,
    /// Inclusive screen rect `(left, top, right, bottom)`.
    pub bounds: (i32, i32, i32, i32),
    pub rows: i32,
    pub cols: i32,
}

/// Catalog-free atlas geometry used by the executor and editor preview.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AtlasLayout {
    nodes: Vec<AtlasNode>,
}

/// Current cell within an [`AtlasLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasPos {
    pub node: usize,
    /// 1-based row.
    pub row: i32,
    /// 1-based column.
    pub col: i32,
}

/// Cardinal navigation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Up,
    Down,
    Left,
    Right,
}

/// Allowed perpendicular overlap slack when judging edge clearance (pixels).
const EDGE_SLACK: i32 = 4;

/// Axis-aligned union of cells in a grid over `bounds` (left, top, right, bottom).
/// Cells are 1-based inclusive `(r1,c1)-(r2,c2)`. `rows`/`cols` are the full grid.
pub fn grid_cell_rect(
    bounds: (i32, i32, i32, i32),
    rows: i32,
    cols: i32,
    r1: i32,
    c1: i32,
    r2: i32,
    c2: i32,
) -> Option<(i32, i32, i32, i32)> {
    if rows < 1 || cols < 1 {
        return None;
    }
    let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    if r1 < 1 || c1 < 1 || r2 > rows || c2 > cols {
        return None;
    }
    let (left_x, top_y, right_x, bottom_y) = bounds;
    let width = right_x - left_x;
    let height = bottom_y - top_y;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some((
        left_x + (c1 - 1) * width / cols,
        top_y + (r1 - 1) * height / rows,
        left_x + c2 * width / cols,
        top_y + r2 * height / rows,
    ))
}

/// One sliding footprint placement inside a collection grid selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPlacement {
    /// Pixel union of the covered cells `(left, top, right, bottom)`.
    pub rect: (i32, i32, i32, i32),
    /// Inclusive 1-based cell range covered by this footprint.
    pub r1: i32,
    pub c1: i32,
    pub r2: i32,
    pub c2: i32,
}

impl GridPlacement {
    /// Every cell `(row, col)` covered by this footprint.
    pub fn cells(self) -> impl Iterator<Item = (i32, i32)> {
        let r1 = self.r1;
        let r2 = self.r2;
        let c1 = self.c1;
        let c2 = self.c2;
        (r1..=r2).flat_map(move |r| (c1..=c2).map(move |c| (r, c)))
    }

    /// True when any covered cell is already claimed.
    pub fn overlaps_occupied(self, occupied: &HashSet<(i32, i32)>) -> bool {
        self.cells().any(|cell| occupied.contains(&cell))
    }

    /// Mark every covered cell as claimed.
    pub fn claim_into(self, occupied: &mut HashSet<(i32, i32)>) {
        occupied.extend(self.cells());
    }
}

/// Slide an `item` rows×cols footprint through a selected cell range.
///
/// Each placement is the pixel union of that many cells. Origins step by one
/// cell. `item` below 1 is treated as 1. Empty when the footprint does not fit
/// in `selection` `(r1, c1, r2, c2)`. `grid` is `(rows, cols)`.
pub fn grid_item_placements(
    bounds: (i32, i32, i32, i32),
    grid: (i32, i32),
    selection: (i32, i32, i32, i32),
    item: (i32, i32),
) -> Vec<GridPlacement> {
    let (grid_rows, grid_cols) = grid;
    let (sel_r1, sel_c1, sel_r2, sel_c2) = selection;
    let (item_rows, item_cols) = item;
    let item_rows = item_rows.max(1);
    let item_cols = item_cols.max(1);
    let (sel_r1, sel_r2) = if sel_r1 <= sel_r2 {
        (sel_r1, sel_r2)
    } else {
        (sel_r2, sel_r1)
    };
    let (sel_c1, sel_c2) = if sel_c1 <= sel_c2 {
        (sel_c1, sel_c2)
    } else {
        (sel_c2, sel_c1)
    };
    let max_r = sel_r2 - item_rows + 1;
    let max_c = sel_c2 - item_cols + 1;
    if max_r < sel_r1 || max_c < sel_c1 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for r in sel_r1..=max_r {
        for c in sel_c1..=max_c {
            let r2 = r + item_rows - 1;
            let c2 = c + item_cols - 1;
            if let Some(rect) = grid_cell_rect(bounds, grid_rows, grid_cols, r, c, r2, c2) {
                out.push(GridPlacement {
                    rect,
                    r1: r,
                    c1: c,
                    r2,
                    c2,
                });
            }
        }
    }
    out
}

impl AtlasLayout {
    pub fn new(nodes: Vec<AtlasNode>) -> Self {
        Self { nodes }
    }

    pub fn nodes(&self) -> &[AtlasNode] {
        &self.nodes
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Index of the node named `collection`, if present.
    pub fn find_collection(&self, collection: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.collection == collection)
    }

    /// Pixel rect of a single cell (1-based). Returns `None` when out of range.
    pub fn cell_rect(&self, pos: AtlasPos) -> Option<(i32, i32, i32, i32)> {
        let node = self.nodes.get(pos.node)?;
        grid_cell_rect(
            node.bounds,
            node.rows,
            node.cols,
            pos.row,
            pos.col,
            pos.row,
            pos.col,
        )
    }

    /// Center of a cell, or `None` when out of range.
    pub fn cell_center(&self, pos: AtlasPos) -> Option<(i32, i32)> {
        let (lx, ty, rx, by) = self.cell_rect(pos)?;
        Some(((lx + rx) / 2, (ty + by) / 2))
    }

    /// Move one step in `dir`. Stays in-grid when possible; otherwise hops to a
    /// geometrically adjacent member. When no neighbor exists, wraps or clamps
    /// within the current collection per `wrap`.
    pub fn step(&self, pos: AtlasPos, dir: NavDir, wrap: bool) -> AtlasPos {
        let Some(node) = self.nodes.get(pos.node) else {
            return pos;
        };
        let (next_row, next_col) = match dir {
            NavDir::Up => (pos.row - 1, pos.col),
            NavDir::Down => (pos.row + 1, pos.col),
            NavDir::Left => (pos.row, pos.col - 1),
            NavDir::Right => (pos.row, pos.col + 1),
        };
        if next_row >= 1 && next_row <= node.rows && next_col >= 1 && next_col <= node.cols {
            return AtlasPos {
                node: pos.node,
                row: next_row,
                col: next_col,
            };
        }
        if let Some(hop) = self.find_neighbor(pos, dir) {
            return hop;
        }
        let (row, col) = match dir {
            NavDir::Up => {
                if wrap {
                    (node.rows, pos.col)
                } else {
                    (1, pos.col)
                }
            }
            NavDir::Down => {
                if wrap {
                    (1, pos.col)
                } else {
                    (node.rows, pos.col)
                }
            }
            NavDir::Left => {
                if wrap {
                    (pos.row, node.cols)
                } else {
                    (pos.row, 1)
                }
            }
            NavDir::Right => {
                if wrap {
                    (pos.row, 1)
                } else {
                    (pos.row, node.cols)
                }
            }
        };
        AtlasPos {
            node: pos.node,
            row: row.clamp(1, node.rows.max(1)),
            col: col.clamp(1, node.cols.max(1)),
        }
    }

    /// Derived neighbor for each cardinal direction from `from`, if any.
    pub fn neighbor_links(&self, from: usize) -> Vec<(NavDir, usize)> {
        let dirs = [NavDir::Up, NavDir::Down, NavDir::Left, NavDir::Right];
        let Some(node) = self.nodes.get(from) else {
            return Vec::new();
        };
        // Probe from the center cell so perpendicular alignment is stable.
        let probe = AtlasPos {
            node: from,
            row: ((node.rows + 1) / 2).max(1),
            col: ((node.cols + 1) / 2).max(1),
        };
        dirs.into_iter()
            .filter_map(|dir| {
                let hop = self.find_neighbor(probe, dir)?;
                Some((dir, hop.node))
            })
            .collect()
    }

    fn find_neighbor(&self, pos: AtlasPos, dir: NavDir) -> Option<AtlasPos> {
        let cur = self.nodes.get(pos.node)?;
        let (cl, ct, cr, cb) = cur.bounds;
        let cx = (cl + cr) / 2;
        let cy = (ct + cb) / 2;
        let exit_center = self.cell_center(pos).unwrap_or((cx, cy));

        let mut best: Option<(i32, i32, usize)> = None; // (gap, perp_dist, idx)
        for (i, cand) in self.nodes.iter().enumerate() {
            if i == pos.node {
                continue;
            }
            let (nl, nt, nr, nb) = cand.bounds;
            let nx = (nl + nr) / 2;
            let ny = (nt + nb) / 2;

            let (beyond, gap, perp_overlap, perp_dist) = match dir {
                NavDir::Right => {
                    let beyond = nx > cx;
                    let gap = nl - cr;
                    let overlap = spans_overlap(ct, cb, nt, nb);
                    let perp = (ny - cy).abs();
                    (beyond, gap, overlap, perp)
                }
                NavDir::Left => {
                    let beyond = nx < cx;
                    let gap = cl - nr;
                    let overlap = spans_overlap(ct, cb, nt, nb);
                    let perp = (ny - cy).abs();
                    (beyond, gap, overlap, perp)
                }
                NavDir::Down => {
                    let beyond = ny > cy;
                    let gap = nt - cb;
                    let overlap = spans_overlap(cl, cr, nl, nr);
                    let perp = (nx - cx).abs();
                    (beyond, gap, overlap, perp)
                }
                NavDir::Up => {
                    let beyond = ny < cy;
                    let gap = ct - nb;
                    let overlap = spans_overlap(cl, cr, nl, nr);
                    let perp = (nx - cx).abs();
                    (beyond, gap, overlap, perp)
                }
            };
            if !beyond || !perp_overlap {
                continue;
            }
            // Near edge must clear the current far edge (allow small overlap slack).
            if gap < -EDGE_SLACK {
                continue;
            }
            let rank = (gap.max(0), perp_dist, i);
            match best {
                None => best = Some(rank),
                Some(prev) if rank < prev => best = Some(rank),
                _ => {}
            }
        }

        let (_, _, idx) = best?;
        let entry = self.entry_cell(idx, dir, exit_center)?;
        Some(entry)
    }

    fn entry_cell(
        &self,
        node_idx: usize,
        dir: NavDir,
        exit_center: (i32, i32),
    ) -> Option<AtlasPos> {
        let node = self.nodes.get(node_idx)?;
        if node.rows < 1 || node.cols < 1 {
            return None;
        }
        let (edge_row, edge_col_fixed) = match dir {
            // Entering from the left (we moved right) → left edge.
            NavDir::Right => (None, Some(1)),
            NavDir::Left => (None, Some(node.cols)),
            NavDir::Down => (Some(1), None),
            NavDir::Up => (Some(node.rows), None),
        };

        let mut best: Option<(i32, AtlasPos)> = None;
        match (edge_row, edge_col_fixed) {
            (None, Some(col)) => {
                for row in 1..=node.rows {
                    let p = AtlasPos {
                        node: node_idx,
                        row,
                        col,
                    };
                    let Some((cx, cy)) = self.cell_center(p) else {
                        continue;
                    };
                    let dist = (cy - exit_center.1).abs() + (cx - exit_center.0).abs() / 4;
                    if best.map(|(d, _)| dist < d).unwrap_or(true) {
                        best = Some((dist, p));
                    }
                }
            }
            (Some(row), None) => {
                for col in 1..=node.cols {
                    let p = AtlasPos {
                        node: node_idx,
                        row,
                        col,
                    };
                    let Some((cx, cy)) = self.cell_center(p) else {
                        continue;
                    };
                    let dist = (cx - exit_center.0).abs() + (cy - exit_center.1).abs() / 4;
                    if best.map(|(d, _)| dist < d).unwrap_or(true) {
                        best = Some((dist, p));
                    }
                }
            }
            _ => return None,
        }
        best.map(|(_, p)| p)
    }
}

fn spans_overlap(a0: i32, a1: i32, b0: i32, b1: i32) -> bool {
    a0 < b1 && b0 < a1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, bounds: (i32, i32, i32, i32), rows: i32, cols: i32) -> AtlasNode {
        AtlasNode {
            collection: name.into(),
            bounds,
            rows,
            cols,
        }
    }

    #[test]
    fn grid_cell_rect_2x2() {
        assert_eq!(
            grid_cell_rect((0, 0, 100, 100), 2, 2, 1, 1, 1, 1),
            Some((0, 0, 50, 50))
        );
        assert_eq!(
            grid_cell_rect((0, 0, 100, 100), 2, 2, 1, 2, 2, 2),
            Some((50, 0, 100, 100))
        );
        assert_eq!(
            grid_cell_rect((0, 0, 100, 100), 2, 2, 1, 1, 2, 2),
            Some((0, 0, 100, 100))
        );
        assert!(grid_cell_rect((0, 0, 100, 100), 2, 2, 1, 1, 3, 1).is_none());
    }

    #[test]
    fn grid_item_placements_slide_by_cell() {
        let bounds = (0, 0, 40, 40);
        assert_eq!(
            grid_item_placements(bounds, (2, 2), (1, 1, 2, 2), (1, 1)).len(),
            4
        );
        assert_eq!(
            grid_item_placements(bounds, (2, 2), (1, 1, 2, 2), (2, 2)),
            vec![GridPlacement {
                rect: (0, 0, 40, 40),
                r1: 1,
                c1: 1,
                r2: 2,
                c2: 2,
            }]
        );
        assert_eq!(
            grid_item_placements(bounds, (2, 2), (1, 1, 2, 2), (2, 1)),
            vec![
                GridPlacement {
                    rect: (0, 0, 20, 40),
                    r1: 1,
                    c1: 1,
                    r2: 2,
                    c2: 1,
                },
                GridPlacement {
                    rect: (20, 0, 40, 40),
                    r1: 1,
                    c1: 2,
                    r2: 2,
                    c2: 2,
                },
            ]
        );
        assert!(grid_item_placements(bounds, (2, 2), (1, 1, 2, 2), (3, 1)).is_empty());
        assert_eq!(
            grid_item_placements(bounds, (2, 2), (1, 1, 2, 2), (0, 0)).len(),
            4
        );
        let one = grid_item_placements(bounds, (2, 2), (1, 1, 2, 2), (1, 1))[0];
        assert_eq!(one.cells().collect::<Vec<_>>(), vec![(1, 1)]);
        let mut occupied = HashSet::from([(1, 1)]);
        assert!(one.overlaps_occupied(&occupied));
        one.claim_into(&mut occupied);
        assert_eq!(occupied.len(), 1);
    }

    #[test]
    fn in_grid_step() {
        let layout = AtlasLayout::new(vec![node("A", (0, 0, 100, 100), 3, 3)]);
        let start = AtlasPos {
            node: 0,
            row: 2,
            col: 2,
        };
        assert_eq!(
            layout.step(start, NavDir::Up, false),
            AtlasPos {
                node: 0,
                row: 1,
                col: 2
            }
        );
        assert_eq!(
            layout.step(start, NavDir::Right, false),
            AtlasPos {
                node: 0,
                row: 2,
                col: 3
            }
        );
    }

    #[test]
    fn wrap_and_clamp_at_edge_without_neighbor() {
        let layout = AtlasLayout::new(vec![node("A", (0, 0, 100, 100), 2, 2)]);
        let top_left = AtlasPos {
            node: 0,
            row: 1,
            col: 1,
        };
        assert_eq!(
            layout.step(top_left, NavDir::Up, true),
            AtlasPos {
                node: 0,
                row: 2,
                col: 1
            }
        );
        assert_eq!(
            layout.step(top_left, NavDir::Up, false),
            AtlasPos {
                node: 0,
                row: 1,
                col: 1
            }
        );
        assert_eq!(
            layout.step(top_left, NavDir::Left, true),
            AtlasPos {
                node: 0,
                row: 1,
                col: 2
            }
        );
        assert_eq!(
            layout.step(top_left, NavDir::Left, false),
            AtlasPos {
                node: 0,
                row: 1,
                col: 1
            }
        );
    }

    #[test]
    fn cross_edge_equal_rows() {
        // A left of B, same vertical span, both 3x2.
        let layout = AtlasLayout::new(vec![
            node("A", (0, 0, 100, 90), 3, 2),
            node("B", (120, 0, 220, 90), 3, 2),
        ]);
        let from = AtlasPos {
            node: 0,
            row: 2,
            col: 2,
        };
        let hop = layout.step(from, NavDir::Right, false);
        assert_eq!(hop.node, 1);
        assert_eq!(hop.col, 1);
        assert_eq!(hop.row, 2);

        let back = layout.step(hop, NavDir::Left, false);
        assert_eq!(back.node, 0);
        assert_eq!(back.col, 2);
        assert_eq!(back.row, 2);
    }

    #[test]
    fn cross_edge_mismatched_row_counts() {
        // 4-row bag → 3-row equip: entry row chosen by nearest pixel center.
        let layout = AtlasLayout::new(vec![
            node("Bag", (0, 0, 100, 120), 4, 2),
            node("Equip", (120, 0, 200, 120), 3, 2),
        ]);
        // Leaving bag row 1 (center y ≈ 15) should land near equip row 1 (center y ≈ 20).
        let from_r1 = AtlasPos {
            node: 0,
            row: 1,
            col: 2,
        };
        let hop = layout.step(from_r1, NavDir::Right, false);
        assert_eq!(hop.node, 1);
        assert_eq!(hop.col, 1);
        assert_eq!(hop.row, 1);

        // Leaving bag row 4 (center y ≈ 105) → equip row 3 (center y ≈ 100).
        let from_r4 = AtlasPos {
            node: 0,
            row: 4,
            col: 2,
        };
        let hop = layout.step(from_r4, NavDir::Right, false);
        assert_eq!(hop.node, 1);
        assert_eq!(hop.row, 3);
    }

    #[test]
    fn diagonal_member_rejected() {
        // B is to the right but vertically disjoint from A.
        let layout = AtlasLayout::new(vec![
            node("A", (0, 0, 100, 50), 2, 2),
            node("B", (120, 80, 220, 130), 2, 2),
        ]);
        let from = AtlasPos {
            node: 0,
            row: 1,
            col: 2,
        };
        // No neighbor → clamp at right edge.
        let stay = layout.step(from, NavDir::Right, false);
        assert_eq!(
            stay,
            AtlasPos {
                node: 0,
                row: 1,
                col: 2
            }
        );
    }

    #[test]
    fn neighbor_links_report_derived_edges() {
        let layout = AtlasLayout::new(vec![
            node("A", (0, 0, 100, 100), 2, 2),
            node("B", (120, 0, 220, 100), 2, 2),
            node("C", (0, 120, 100, 220), 2, 2),
        ]);
        let links = layout.neighbor_links(0);
        assert!(links.contains(&(NavDir::Right, 1)));
        assert!(links.contains(&(NavDir::Down, 2)));
        assert!(!links.iter().any(|(d, _)| *d == NavDir::Left));
        assert!(!links.iter().any(|(d, _)| *d == NavDir::Up));
    }
}
