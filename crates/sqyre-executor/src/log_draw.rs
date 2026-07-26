//! RGB drawing helpers for executor debug log overlays.

use sqyre_match::ImageBuf;

/// Draw axis-aligned rectangles on a 3-channel RGB buffer (clips to bounds).
pub fn draw_rect_rgb(img: &mut ImageBuf, x0: i32, y0: i32, x1: i32, y1: i32, rgb: [u8; 3]) {
    if img.channels != 3 {
        return;
    }
    let w = img.width as i32;
    let h = img.height as i32;
    let left = x0.min(x1).clamp(0, w - 1);
    let right = x0.max(x1).clamp(0, w - 1);
    let top = y0.min(y1).clamp(0, h - 1);
    let bottom = y0.max(y1).clamp(0, h - 1);
    for x in left..=right {
        put_rgb(img, x, top, rgb);
        put_rgb(img, x, bottom, rgb);
    }
    for y in top..=bottom {
        put_rgb(img, left, y, rgb);
        put_rgb(img, right, y, rgb);
    }
}

fn put_rgb(img: &mut ImageBuf, x: i32, y: i32, rgb: [u8; 3]) {
    if x < 0 || y < 0 || x as usize >= img.width || y as usize >= img.height {
        return;
    }
    let i = img.pixel_offset(x as usize, y as usize);
    img.data[i] = rgb[0];
    img.data[i + 1] = rgb[1];
    img.data[i + 2] = rgb[2];
}

/// Crop a padded region around a template match and draw the match box (for logs).
pub fn crop_match_preview(
    search: &ImageBuf,
    x: i32,
    y: i32,
    tw: i32,
    th: i32,
    pad: i32,
) -> Option<ImageBuf> {
    if search.channels != 3 || tw <= 0 || th <= 0 {
        return None;
    }
    let w = search.width as i32;
    let h = search.height as i32;
    let x0 = (x - pad).max(0);
    let y0 = (y - pad).max(0);
    let x1 = (x + tw + pad).min(w);
    let y1 = (y + th + pad).min(h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let cw = (x1 - x0) as usize;
    let ch = (y1 - y0) as usize;
    let mut out = ImageBuf::new(cw, ch, 3, 0);
    for py in 0..ch {
        for px in 0..cw {
            let si = search.pixel_offset((x0 as usize) + px, (y0 as usize) + py);
            let di = out.pixel_offset(px, py);
            out.data[di..di + 3].copy_from_slice(&search.data[si..si + 3]);
        }
    }
    draw_rect_rgb(
        &mut out,
        x - x0,
        y - y0,
        x - x0 + tw - 1,
        y - y0 + th - 1,
        [255, 40, 40],
    );
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_match_preview_draws_box() {
        let search = ImageBuf::new(40, 30, 3, 80);
        let crop = crop_match_preview(&search, 10, 8, 6, 4, 4).unwrap();
        assert_eq!(crop.width, 14);
        assert_eq!(crop.height, 12);
    }
}
