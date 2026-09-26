use crate::image::Point;
use crate::template::MatchMap;
use rayon::prelude::*;
use sqyre_domain::MatchMethod;
use std::collections::HashMap;

/// Default distance for spatially deduplicating nearby match peaks.
pub const DEFAULT_CLOSE_MATCHES_DISTANCE: i32 = 10;

/// Accept scores `>= threshold`, reject NaN/Inf, then spatial-dedup.
/// Extract match peaks from a template-match result map (higher-is-better).
pub fn find_peaks(map: &MatchMap, threshold: f32, close_matches_distance: i32) -> Vec<Point> {
    find_peaks_polarity(map, threshold, close_matches_distance, true)
}

/// Method-aware peak extraction: `SQDIFF*` use `score <= threshold`.
pub fn find_peaks_for_method(
    map: &MatchMap,
    threshold: f32,
    close_matches_distance: i32,
    method: MatchMethod,
) -> Vec<Point> {
    find_peaks_polarity(
        map,
        threshold,
        close_matches_distance,
        method.higher_is_better(),
    )
}

fn find_peaks_polarity(
    map: &MatchMap,
    threshold: f32,
    close_matches_distance: i32,
    higher_is_better: bool,
) -> Vec<Point> {
    if map.width == 0 || map.height == 0 {
        return Vec::new();
    }
    let w = map.width;
    let scores = &map.scores;
    // Parallel per-row scan; keep row order so clustering stays stable.
    // Stream straight into the clusterer (no intermediate flat Vec), and skip
    // along a row once a cluster opens — same idea as find_pixels_clustered.
    let row_hits: Vec<Vec<Point>> = (0..map.height)
        .into_par_iter()
        .map(|y| {
            let row = y * w;
            let mut hits = Vec::new();
            for x in 0..w {
                let confidence = scores[row + x];
                if !confidence.is_finite() {
                    continue;
                }
                let ok = if higher_is_better {
                    confidence >= threshold
                } else {
                    confidence <= threshold
                };
                if ok {
                    hits.push(Point {
                        x: x as i32,
                        y: y as i32,
                    });
                }
            }
            hits
        })
        .collect();
    let mut clusterer = PointClusterer::new(close_matches_distance);
    let skip = clusterer.distance();
    let mut out = Vec::new();
    for hits in row_hits {
        let mut i = 0;
        while i < hits.len() {
            let point = hits[i];
            if clusterer.add_if_far(point) {
                out.push(point);
                // Same row ⇒ |dy| = 0; later x within `skip` are in-cluster.
                let limit_x = point.x + skip;
                i += 1;
                while i < hits.len() && hits[i].x <= limit_x {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Keep the first point of each spatial cluster (scan order), dropping neighbors
/// within `close_matches_distance` (Chebyshev).
pub fn cluster_points(points: &[Point], close_matches_distance: i32) -> Vec<Point> {
    cluster_points_from(points.iter().copied(), close_matches_distance)
}

/// [`cluster_points`] over an iterator, so callers that can generate points
/// lazily never materialize the full pre-cluster list.
///
/// Yields the same result as `cluster_points` for the same point sequence:
/// clustering is greedy in iteration order, so the order must match.
pub fn cluster_points_from(
    points: impl IntoIterator<Item = Point>,
    close_matches_distance: i32,
) -> Vec<Point> {
    let mut dedup = PointClusterer::new(close_matches_distance);
    let mut out = Vec::new();
    for p in points {
        if dedup.add_if_far(p) {
            out.push(p);
        }
    }
    out
}

/// Greedy spatial dedup: the first point of each cluster wins, and any later
/// point within `distance` (Chebyshev) of a kept point is dropped.
///
/// Exposed so callers that generate points in scan order can skip work when a
/// point is kept — every point within `distance` of it is then known to be
/// rejected without testing it (see [`Self::distance`]).
pub struct PointClusterer {
    distance: i32,
    buckets: HashMap<(i32, i32), Vec<Point>>,
}

impl PointClusterer {
    pub fn new(distance: i32) -> Self {
        Self {
            distance: distance.max(0),
            buckets: HashMap::new(),
        }
    }

    /// Clamped cluster radius: points within this of a kept point are dropped.
    pub fn distance(&self) -> i32 {
        self.distance
    }

    /// Record `point` and return whether it starts a new cluster.
    pub fn add_if_far(&mut self, point: Point) -> bool {
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
        let mut d = PointClusterer::new(5);
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

    #[test]
    fn sqdiff_peaks_use_upper_bound() {
        let mut scores = vec![1.0_f32; 100];
        scores[5 * 10 + 7] = 0.05;
        let map = MatchMap {
            width: 10,
            height: 10,
            scores,
        };
        let matches = find_peaks_for_method(&map, 0.1, 10, MatchMethod::SqdiffNormed);
        assert_eq!(matches, vec![Point { x: 7, y: 5 }]);
    }

    #[test]
    fn dense_peaks_match_scan_then_cluster() {
        // Low threshold ⇒ nearly every cell is a hit; skip-while-clustering must
        // match materialize-then-cluster_points.
        let mut scores = vec![0.5_f32; 40 * 30];
        scores[2 * 40 + 3] = 0.99;
        scores[27 * 40 + 37] = 0.99;
        let map = MatchMap {
            width: 40,
            height: 30,
            scores,
        };
        for distance in [0, 1, 5, 12, 50] {
            let mut all = Vec::new();
            for y in 0..30 {
                for x in 0..40 {
                    let c = map.scores[y * 40 + x];
                    if c.is_finite() && c >= 0.4 {
                        all.push(Point {
                            x: x as i32,
                            y: y as i32,
                        });
                    }
                }
            }
            let want = cluster_points(&all, distance);
            let got = find_peaks(&map, 0.4, distance);
            assert_eq!(got, want, "distance={distance}");
        }
    }
}
