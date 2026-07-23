//! Offline-testable X11 ZPixmap → RGBA / RGB conversion.

use image::RgbaImage;
use pulp::Arch;
use rayon::prelude::*;

/// Convert X11 ZPixmap bytes (typically BGRA on little-endian) into an [`RgbaImage`].
///
/// `bpp` is bytes per pixel from `bits_per_pixel / 8` (must be ≥ 3).
/// `bytes_per_line` is the XImage row stride (may exceed `width * bpp` due to padding).
/// Pass `0` to treat the buffer as tightly packed (`width * bpp` per row).
pub fn zpixmap_to_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    bpp: usize,
    bytes_per_line: usize,
) -> Result<RgbaImage, String> {
    let mut out = vec![
        0u8;
        (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4)
    ];
    zpixmap_swizzle(data, width, height, bpp, bytes_per_line, true, &mut out)?;
    RgbaImage::from_raw(width, height, out).ok_or_else(|| "RGBA buffer size mismatch".into())
}

/// Convert X11 ZPixmap bytes directly to tightly packed RGB (no alpha).
pub fn zpixmap_to_rgb(
    data: &[u8],
    width: u32,
    height: u32,
    bpp: usize,
    bytes_per_line: usize,
) -> Result<Vec<u8>, String> {
    let mut out = vec![
        0u8;
        (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3)
    ];
    zpixmap_swizzle(data, width, height, bpp, bytes_per_line, false, &mut out)?;
    Ok(out)
}

fn zpixmap_swizzle(
    data: &[u8],
    width: u32,
    height: u32,
    bpp: usize,
    bytes_per_line: usize,
    with_alpha: bool,
    out: &mut [u8],
) -> Result<(), String> {
    if bpp < 3 {
        return Err(format!("unexpected bytes_per_pixel {bpp}"));
    }
    let w = width as usize;
    let h = height as usize;
    let row_stride = if bytes_per_line == 0 {
        w.saturating_mul(bpp)
    } else {
        bytes_per_line
    };
    if row_stride < w.saturating_mul(bpp) {
        return Err(format!(
            "bytes_per_line {row_stride} shorter than width*{bpp}={}",
            w * bpp
        ));
    }
    let need = row_stride.saturating_mul(h);
    if data.len() < need {
        return Err(format!(
            "pixel buffer too short: got {} need {need} (stride {row_stride})",
            data.len()
        ));
    }
    let out_bpp = if with_alpha { 4 } else { 3 };
    let expect = w.saturating_mul(h).saturating_mul(out_bpp);
    if out.len() != expect {
        return Err(format!("output buffer size {} != {expect}", out.len()));
    }

    let out_addr = out.as_mut_ptr() as usize;
    (0..h)
        .into_par_iter()
        .try_for_each(|y| -> Result<(), String> {
            let row = &data[y * row_stride..y * row_stride + w * bpp];
            let arch = Arch::new();
            arch.dispatch(|| {
                for (x, chunk) in row.chunks_exact(bpp).enumerate() {
                    let di = (y * w + x) * out_bpp;
                    // SAFETY: each row `y` writes a disjoint output range.
                    let dst = out_addr as *mut u8;
                    unsafe {
                        *dst.add(di) = chunk[2]; // R
                        *dst.add(di + 1) = chunk[1]; // G
                        *dst.add(di + 2) = chunk[0]; // B
                        if with_alpha {
                            *dst.add(di + 3) = if bpp >= 4 { chunk[3] } else { 255 };
                        }
                    }
                }
            });
            Ok(())
        })?;
    Ok(())
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
        let img = zpixmap_to_rgba(&data, 2, 1, 4, 0).unwrap();
        assert_eq!(*img.get_pixel(0, 0), image::Rgba([0, 0, 255, 255]));
        assert_eq!(*img.get_pixel(1, 0), image::Rgba([255, 0, 0, 128]));
    }

    #[test]
    fn bgr_swizzle_3bpp() {
        let data = [10, 20, 30]; // B G R
        let img = zpixmap_to_rgba(&data, 1, 1, 3, 0).unwrap();
        assert_eq!(*img.get_pixel(0, 0), image::Rgba([30, 20, 10, 255]));
    }

    #[test]
    fn honors_row_stride_padding() {
        // 1×2 image, 4bpp, stride 8 (4 bytes padding per row)
        let mut data = vec![0u8; 16];
        // row0: blue
        data[0] = 255;
        data[1] = 0;
        data[2] = 0;
        data[3] = 255;
        // row1: red
        data[8] = 0;
        data[9] = 0;
        data[10] = 255;
        data[11] = 200;
        let img = zpixmap_to_rgba(&data, 1, 2, 4, 8).unwrap();
        assert_eq!(*img.get_pixel(0, 0), image::Rgba([0, 0, 255, 255]));
        assert_eq!(*img.get_pixel(0, 1), image::Rgba([255, 0, 0, 200]));
    }

    #[test]
    fn rgb_direct() {
        let data = [255, 0, 0, 255]; // BGRA blue
        let rgb = zpixmap_to_rgb(&data, 1, 1, 4, 0).unwrap();
        assert_eq!(rgb, vec![0, 0, 255]);
    }

    #[test]
    fn rejects_short_buffer() {
        assert!(zpixmap_to_rgba(&[1, 2], 1, 1, 4, 0).is_err());
    }
}
