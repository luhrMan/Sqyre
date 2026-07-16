use sqyre_match::{ImageBuf, Point};

/// Find first pixel matching `#rrggbb` within `tolerance`.
pub fn find_pixel(img: &ImageBuf, hex: &str, tolerance: i32) -> Option<Point> {
    if img.channels != 3 {
        return None;
    }
    let (tr, tg, tb) = parse_hex(hex).unwrap_or((0, 0, 0));
    let tol = tolerance.clamp(0, 255);
    for y in 0..img.height {
        for x in 0..img.width {
            let o = img.pixel_offset(x, y);
            let r = img.data[o] as i32;
            let g = img.data[o + 1] as i32;
            let b = img.data[o + 2] as i32;
            if (r - tr).abs() <= tol && (g - tg).abs() <= tol && (b - tb).abs() <= tol {
                return Some(Point {
                    x: x as i32,
                    y: y as i32,
                });
            }
        }
    }
    None
}

fn parse_hex(s: &str) -> Option<(i32, i32, i32)> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 {
        return None;
    }
    let r = i32::from_str_radix(&t[0..2], 16).ok()?;
    let g = i32::from_str_radix(&t[2..4], 16).ok()?;
    let b = i32::from_str_radix(&t[4..6], 16).ok()?;
    Some((r, g, b))
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
}
