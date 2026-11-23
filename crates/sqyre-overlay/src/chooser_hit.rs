//! Chooser / context-menu row hit testing (OS-neutral).

use crate::raster::ChooserRow;

/// Index of the row under `(x, y)` in window / client coordinates.
pub fn chooser_row_at(rows: &[ChooserRow], x: i32, y: i32) -> Option<usize> {
    rows.iter().position(|(_, rx, ry, rw, rh)| {
        x >= *rx && y >= *ry && x < *rx + *rw as i32 && y < *ry + *rh as i32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_hit_matches_rect() {
        let rows = vec![("A".into(), 0, 0, 100, 20), ("B".into(), 0, 20, 100, 20)];
        assert_eq!(chooser_row_at(&rows, 10, 5), Some(0));
        assert_eq!(chooser_row_at(&rows, 10, 25), Some(1));
        assert_eq!(chooser_row_at(&rows, 10, 40), None);
        assert_eq!(chooser_row_at(&rows, -1, 5), None);
    }
}
