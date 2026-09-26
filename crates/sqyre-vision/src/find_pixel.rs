use rayon::prelude::*;
use sqyre_match::{ImageBuf, Point, PointClusterer};

/// Half-open run of consecutive matching pixels in one row: `[start, end)`.
type Run = (usize, usize);

/// Find all pixels matching `#rrggbb` within `tolerance` (row-major order).
pub fn find_pixels(img: &ImageBuf, hex: &str, tolerance: i32) -> Vec<Point> {
    let Some(scan) = Scan::new(img, hex, tolerance) else {
        return Vec::new();
    };
    let rows = scan.runs_by_row(img.height);
    let mut out = Vec::with_capacity(rows.iter().flatten().map(|(s, e)| e - s).sum());
    out.extend(expand_rows(&rows));
    out
}

/// [`find_pixels`] followed by clustering, without materializing every matching
/// pixel first.
///
/// A large flat-colored region matches on the order of `width * height` pixels
/// but collapses to a handful of clusters. Rows are scanned into runs, and
/// within a run the scan jumps past pixels that clustering is guaranteed to
/// reject, so neither the point list nor the per-point tests are paid in full.
///
/// Equivalent to `cluster_points(&find_pixels(..), distance)`.
pub fn find_pixels_clustered(
    img: &ImageBuf,
    hex: &str,
    tolerance: i32,
    close_matches_distance: i32,
) -> Vec<Point> {
    let Some(scan) = Scan::new(img, hex, tolerance) else {
        return Vec::new();
    };
    let rows = scan.runs_by_row(img.height);
    let mut clusterer = PointClusterer::new(close_matches_distance);
    let skip = clusterer.distance();
    let mut out = Vec::new();
    for (y, runs) in rows.iter().enumerate() {
        for &(start, end) in runs {
            let mut x = start;
            while x < end {
                let point = Point {
                    x: x as i32,
                    y: y as i32,
                };
                if clusterer.add_if_far(point) {
                    out.push(point);
                    // Everything up to `skip` further along this row is within
                    // the cluster just opened (same y, so |dy| = 0), and would
                    // be rejected point by point.
                    x += skip as usize + 1;
                } else {
                    x += 1;
                }
            }
        }
    }
    out
}

/// Row-major points for `rows`, matching the order [`find_pixels`] returns.
fn expand_rows(rows: &[Vec<Run>]) -> impl Iterator<Item = Point> + '_ {
    rows.iter().enumerate().flat_map(|(y, runs)| {
        runs.iter().flat_map(move |&(start, end)| {
            (start..end).map(move |x| Point {
                x: x as i32,
                y: y as i32,
            })
        })
    })
}

/// Pixel predicate bound to one image + target color.
struct Scan<'a> {
    data: &'a [u8],
    width: usize,
    target: (i32, i32, i32),
    tol: i32,
}

impl<'a> Scan<'a> {
    /// `None` when the image is not 3-channel RGB.
    fn new(img: &'a ImageBuf, hex: &str, tolerance: i32) -> Option<Self> {
        if img.channels != 3 {
            return None;
        }
        Some(Self {
            data: img.data.as_slice(),
            width: img.width,
            target: parse_hex(hex).unwrap_or((0, 0, 0)),
            tol: tolerance.clamp(0, 255),
        })
    }

    fn matches(&self, y: usize, x: usize) -> bool {
        let o = (y * self.width + x) * 3;
        let (tr, tg, tb) = self.target;
        (self.data[o] as i32 - tr).abs() <= self.tol
            && (self.data[o + 1] as i32 - tg).abs() <= self.tol
            && (self.data[o + 2] as i32 - tb).abs() <= self.tol
    }

    /// Matching spans per row. Runs keep a solid region to a couple of entries
    /// per row instead of one `Point` per pixel.
    fn runs_by_row(&self, height: usize) -> Vec<Vec<Run>> {
        (0..height)
            .into_par_iter()
            .map(|y| {
                let mut runs: Vec<Run> = Vec::new();
                let mut start: Option<usize> = None;
                for x in 0..self.width {
                    if self.matches(y, x) {
                        start.get_or_insert(x);
                    } else if let Some(s) = start.take() {
                        runs.push((s, x));
                    }
                }
                if let Some(s) = start {
                    runs.push((s, self.width));
                }
                runs
            })
            .collect()
    }
}

fn parse_hex(s: &str) -> Option<(i32, i32, i32)> {
    let [r, g, b, _] = sqyre_domain::parse_hex_color(s)?;
    Some((r as i32, g as i32, b as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_red_pixel() {
        let mut img = ImageBuf::new(4, 4, 3, 0);
        let o = img.pixel_offset(2, 1);
        img.data[o] = 255;
        img.data[o + 1] = 0;
        img.data[o + 2] = 0;
        let pts = find_pixels(&img, "#ff0000", 0);
        assert_eq!(pts, vec![Point { x: 2, y: 1 }]);
    }

    #[test]
    fn find_pixels_collects_all() {
        let mut img = ImageBuf::new(4, 2, 3, 0);
        for &(x, y) in &[(0, 0), (3, 0), (1, 1)] {
            let o = img.pixel_offset(x, y);
            img.data[o] = 255;
            img.data[o + 1] = 0;
            img.data[o + 2] = 0;
        }
        let pts = find_pixels(&img, "#ff0000", 0);
        assert_eq!(pts.len(), 3);
        assert_eq!((pts[0].x, pts[0].y), (0, 0));
        assert_eq!((pts[1].x, pts[1].y), (3, 0));
        assert_eq!((pts[2].x, pts[2].y), (1, 1));
    }

    #[test]
    fn clustered_matches_scan_then_cluster() {
        // Solid block plus a far-away speck: the block is exactly the case that
        // used to materialize one Point per pixel before clustering.
        let mut img = ImageBuf::new(40, 30, 3, 0);
        for y in 4..20 {
            for x in 3..28 {
                let o = img.pixel_offset(x, y);
                img.data[o] = 255;
                img.data[o + 1] = 0;
                img.data[o + 2] = 0;
            }
        }
        let o = img.pixel_offset(37, 27);
        img.data[o] = 255;
        img.data[o + 1] = 0;
        img.data[o + 2] = 0;

        for distance in [0, 1, 5, 12, 50] {
            let want = sqyre_match::cluster_points(&find_pixels(&img, "#ff0000", 0), distance);
            let got = find_pixels_clustered(&img, "#ff0000", 0, distance);
            assert_eq!(got, want, "distance={distance}");
        }
    }

    #[test]
    fn find_pixels_row_major_order() {
        let mut img = ImageBuf::new(16, 16, 3, 0);
        for &(x, y) in &[(9, 2), (1, 2), (15, 0), (4, 11)] {
            let o = img.pixel_offset(x, y);
            img.data[o] = 255;
            img.data[o + 1] = 0;
            img.data[o + 2] = 0;
        }
        let pts = find_pixels(&img, "#ff0000", 0);
        assert_eq!(
            pts,
            vec![
                Point { x: 15, y: 0 },
                Point { x: 1, y: 2 },
                Point { x: 9, y: 2 },
                Point { x: 4, y: 11 },
            ]
        );
    }

    #[test]
    fn runs_span_whole_row_when_image_is_uniform() {
        let img = ImageBuf::new(6, 3, 3, 0);
        // Every pixel matches black, so each row is a single run.
        let pts = find_pixels(&img, "#000000", 0);
        assert_eq!(pts.len(), 18);
        assert_eq!((pts[0].x, pts[0].y), (0, 0));
        assert_eq!((pts[6].x, pts[6].y), (0, 1));
        assert_eq!((pts[17].x, pts[17].y), (5, 2));
    }

    #[test]
    fn find_pixels_respects_tolerance() {
        let mut img = ImageBuf::new(8, 8, 3, 10);
        let o = img.pixel_offset(4, 5);
        img.data[o] = 200;
        img.data[o + 1] = 10;
        img.data[o + 2] = 10;
        assert!(find_pixels(&img, "#c80000", 0).is_empty());
        let pts = find_pixels(&img, "#c80000", 15);
        assert_eq!(pts, vec![Point { x: 4, y: 5 }]);
    }
}
