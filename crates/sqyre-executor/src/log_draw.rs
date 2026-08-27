//! RGB drawing helpers for executor debug log overlays.

use sqyre_match::ImageBuf;
use sqyre_ports::LogImage;

/// Cap long edge when storing log images (keeps UI memory bounded).
const LOG_IMAGE_MAX_EDGE: usize = 640;

/// Convert a match buffer to RGBA log pixels (downscaled if needed).
pub(crate) fn image_buf_to_log_image(label: String, image: &ImageBuf) -> Option<LogImage> {
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let scaled = downscale_for_log(image);
    let rgba = image_buf_to_rgba(&scaled);
    LogImage::from_rgba(label, scaled.width as u32, scaled.height as u32, rgba)
}

fn downscale_for_log(img: &ImageBuf) -> ImageBuf {
    let long = img.width.max(img.height);
    if long <= LOG_IMAGE_MAX_EDGE {
        return img.clone();
    }
    let scale = LOG_IMAGE_MAX_EDGE as f64 / long as f64;
    let nw = ((img.width as f64) * scale).round().max(1.0) as usize;
    let nh = ((img.height as f64) * scale).round().max(1.0) as usize;
    nearest_resize(img, nw, nh)
}

fn nearest_resize(img: &ImageBuf, nw: usize, nh: usize) -> ImageBuf {
    let ch = img.channels;
    let mut data = vec![0u8; nw * nh * ch];
    for y in 0..nh {
        let sy = (y * img.height / nh).min(img.height - 1);
        for x in 0..nw {
            let sx = (x * img.width / nw).min(img.width - 1);
            let si = img.pixel_offset(sx, sy);
            let di = (y * nw + x) * ch;
            data[di..di + ch].copy_from_slice(&img.data[si..si + ch]);
        }
    }
    ImageBuf::from_raw(nw, nh, ch, data)
}

fn image_buf_to_rgba(img: &ImageBuf) -> Vec<u8> {
    let n = img.width * img.height;
    let mut out = Vec::with_capacity(n * 4);
    match img.channels {
        1 => {
            for &v in &img.data {
                out.extend_from_slice(&[v, v, v, 255]);
            }
        }
        3 => {
            for i in 0..n {
                let o = i * 3;
                out.extend_from_slice(&[img.data[o], img.data[o + 1], img.data[o + 2], 255]);
            }
        }
        _ => {
            for _ in 0..n {
                out.extend_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    out
}

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
