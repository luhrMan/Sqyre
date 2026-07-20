use crate::image::Point;
use crate::template::MatchMap;
use std::collections::HashMap;

/// Default distance for spatially deduplicating nearby match peaks.
pub const DEFAULT_CLOSE_MATCHES_DISTANCE: i32 = 10;

/// Accept scores `>= threshold`, reject NaN/Inf, then spatial-dedup.
/// Extract match peaks from a template-match result map.
pub fn find_peaks(map: &MatchMap, threshold: f32, close_matches_distance: i32) -> Vec<Point> {
    if map.width == 0 || map.height == 0 {
        return Vec::new();
    }
    let mut matches = Vec::new();
    for y in 0..map.height {
        let row = y * map.width;
        for x in 0..map.width {
            let confidence = map.scores[row + x];
            if confidence < threshold || !confidence.is_finite() {
                continue;
            }
            matches.push(Point {
                x: x as i32,
                y: y as i32,
            });
        }
    }
    cluster_points(&matches, close_matches_distance)
}

/// Keep the first point of each spatial cluster (scan order), dropping neighbors
/// within `close_matches_distance` (Chebyshev).
pub fn cluster_points(points: &[Point], close_matches_distance: i32) -> Vec<Point> {
    let mut dedup = MatchPointDedup::new(close_matches_distance);
    let mut out = Vec::new();
    for &p in points {
        if dedup.add_if_far(p) {
            out.push(p);
        }
    }
    out
}

struct MatchPointDedup {
    distance: i32,
    buckets: HashMap<(i32, i32), Vec<Point>>,
}

impl MatchPointDedup {
    fn new(distance: i32) -> Self {
        Self {
            distance: distance.max(0),
            buckets: HashMap::new(),
        }
    }

    fn add_if_far(&mut self, point: Point) -> bool {
        if self.distance <= 0 {
            self.buckets.entry((0, 0)).or_default().push(point);
            return true;
        }
        let cell = self.distance + 1;
        let bx = point.x / cell;
        let by = point.y / cell;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(existing) = self.buckets.get(&(bx + dx, by + dy)) {
                    for e in existing {
                        if (e.x - point.x).abs() <= self.distance
                            && (e.y - point.y).abs() <= self.distance
                        {
                            return false;
                        }
                    }
                }
            }
        }
        self.buckets.entry((bx, by)).or_default().push(point);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_rejects_nearby() {
        let mut d = MatchPointDedup::new(5);
        assert!(d.add_if_far(Point { x: 10, y: 10 }));
        assert!(!d.add_if_far(Point { x: 12, y: 12 }));
        assert!(d.add_if_far(Point { x: 20, y: 20 }));
    }

    #[test]
    fn finds_peak() {
        let mut scores = vec![0.0_f32; 100];
        scores[5 * 10 + 7] = 0.95;
        let map = MatchMap {
            width: 10,
            height: 10,
            scores,
        };
        let matches = find_peaks(&map, 0.9, 10);
        assert_eq!(matches, vec![Point { x: 7, y: 5 }]);
    }
}
