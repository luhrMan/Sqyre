use rayon::prelude::*;
use sqyre_match::{ImageBuf, Point};

/// Find first pixel matching `#rrggbb` within `tolerance`.
pub fn find_pixel(img: &ImageBuf, hex: &str, tolerance: i32) -> Option<Point> {
    find_pixels(img, hex, tolerance).into_iter().next()
}

/// Find all pixels matching `#rrggbb` within `tolerance` (row-major order).
pub fn find_pixels(img: &ImageBuf, hex: &str, tolerance: i32) -> Vec<Point> {
    if img.channels != 3 {
        return Vec::new();
    }
    let (tr, tg, tb) = parse_hex(hex).unwrap_or((0, 0, 0));
    let tol = tolerance.clamp(0, 255);
    let w = img.width;
    let data = img.data.as_slice();
    // Parallel per-row scan; flatten in row order.
    let row_hits: Vec<Vec<Point>> = (0..img.height)
        .into_par_iter()
        .map(|y| {
            let mut hits = Vec::new();
            for x in 0..w {
                let o = (y * w + x) * 3;
                let r = data[o] as i32;
                let g = data[o + 1] as i32;
                let b = data[o + 2] as i32;
                if (r - tr).abs() <= tol && (g - tg).abs() <= tol && (b - tb).abs() <= tol {
                    hits.push(Point {
                        x: x as i32,
                        y: y as i32,
                    });
                }
            }
            hits
        })
        .collect();
    let mut out = Vec::new();
    for row in row_hits {
        out.extend(row);
    }
    out
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
        let p = find_pixel(&img, "#ff0000", 0).unwrap();
        assert_eq!((p.x, p.y), (2, 1));
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
}
