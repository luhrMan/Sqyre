//! Offline-testable X11 ZPixmap → RGBA conversion.

use image::RgbaImage;

/// Convert X11 ZPixmap bytes (typically BGRA on little-endian) into an [`RgbaImage`].
///
/// `bpp` is bytes per pixel from `bits_per_pixel / 8` (must be ≥ 3).
pub fn zpixmap_to_rgba(data: &[u8], width: u32, height: u32, bpp: usize) -> Result<RgbaImage, String> {
    if bpp < 3 {
        return Err(format!("unexpected bytes_per_pixel {bpp}"));
    }
    let pixels = (width as usize).saturating_mul(height as usize);
    let need = pixels.saturating_mul(bpp);
    if data.len() < need {
        return Err(format!(
            "pixel buffer too short: got {} need {need}",
            data.len()
        ));
    }
    let mut out = Vec::with_capacity(pixels * 4);
    if bpp >= 4 {
        for chunk in data[..need].chunks_exact(bpp) {
            out.push(chunk[2]); // R
            out.push(chunk[1]); // G
            out.push(chunk[0]); // B
            out.push(chunk[3]); // A
        }
    } else {
        for chunk in data[..need].chunks_exact(bpp) {
            out.push(chunk[2]);
            out.push(chunk[1]);
            out.push(chunk[0]);
            out.push(255);
        }
    }
    RgbaImage::from_raw(width, height, out).ok_or_else(|| "RGBA buffer size mismatch".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_swizzle_4bpp() {
        // Two pixels: blue, then red (BGRA)
        let data = [
            255, 0, 0, 255, // B G R A → blue
            0, 0, 255, 128, // B G R A → red, a=128
        ];
        let img = zpixmap_to_rgba(&data, 2, 1, 4).unwrap();
        assert_eq!(*img.get_pixel(0, 0), image::Rgba([0, 0, 255, 255]));
        assert_eq!(*img.get_pixel(1, 0), image::Rgba([255, 0, 0, 128]));
    }

    #[test]
    fn bgr_swizzle_3bpp() {
        let data = [10, 20, 30]; // B G R
        let img = zpixmap_to_rgba(&data, 1, 1, 3).unwrap();
        assert_eq!(*img.get_pixel(0, 0), image::Rgba([30, 20, 10, 255]));
    }

    #[test]
    fn rejects_short_buffer() {
        assert!(zpixmap_to_rgba(&[1, 2], 1, 1, 4).is_err());
    }
}
