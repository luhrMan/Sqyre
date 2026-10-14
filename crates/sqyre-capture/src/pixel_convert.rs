//! Offline-testable pixel swizzle / strip kernels (X11 ZPixmap + Portal share).

use image::RgbaImage;
use pulp::Arch;
use rayon::prelude::*;

/// Minimum row count before portal / crop helpers use Rayon (avoid pool spam).
pub const PARALLEL_ROW_GATE: usize = 32;

/// Cached `pulp` ISA dispatch — avoid `Arch::new()` on every swizzle row.
#[inline]
pub(crate) fn pulp_arch() -> Arch {
    thread_local! {
        static ARCH: Arch = Arch::new();
    }
    ARCH.with(|a| *a)
}

/// Interleaved source layout for row→RGBA kernels (Portal + tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbaSrcFormat {
    Rgba,
    Bgra,
    Rgbx,
    Bgrx,
    Rgb,
    Bgr,
}

impl RgbaSrcFormat {
    #[inline]
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb | Self::Bgr => 3,
            Self::Rgba | Self::Bgra | Self::Rgbx | Self::Bgrx => 4,
        }
    }
}

/// Swizzle one tightly-packed source row into RGBA (`dst` length `width * 4`).
#[inline]
pub fn swizzle_row_to_rgba(row: &[u8], width: usize, format: RgbaSrcFormat, dst: &mut [u8]) {
    debug_assert!(dst.len() >= width * 4);
    let bpp = format.bytes_per_pixel();
    let arch = pulp_arch();
    arch.dispatch(|| match format {
        RgbaSrcFormat::Rgba => {
            let n = (width * 4).min(row.len()).min(dst.len());
            dst[..n].copy_from_slice(&row[..n]);
        }
        RgbaSrcFormat::Bgra => {
            for x in 0..width {
                let s = x * 4;
                let d = x * 4;
                if s + 4 > row.len() || d + 4 > dst.len() {
                    break;
                }
                dst[d] = row[s + 2];
                dst[d + 1] = row[s + 1];
                dst[d + 2] = row[s];
                dst[d + 3] = row[s + 3];
            }
        }
        RgbaSrcFormat::Rgbx | RgbaSrcFormat::Rgb => {
            for x in 0..width {
                let s = x * bpp;
                let d = x * 4;
                if s + bpp > row.len() || d + 4 > dst.len() {
                    break;
                }
                dst[d..d + 3].copy_from_slice(&row[s..s + 3]);
                dst[d + 3] = 255;
            }
        }
        RgbaSrcFormat::Bgrx | RgbaSrcFormat::Bgr => {
            for x in 0..width {
                let s = x * bpp;
                let d = x * 4;
                if s + bpp > row.len() || d + 4 > dst.len() {
                    break;
                }
                dst[d] = row[s + 2];
                dst[d + 1] = row[s + 1];
                dst[d + 2] = row[s];
                dst[d + 3] = 255;
            }
        }
    });
}

/// Drop alpha from a tightly packed RGBA row into RGB (`dst` length `width * 3`).
#[inline]
pub fn strip_rgba_row_to_rgb(src: &[u8], width: usize, dst: &mut [u8]) {
    debug_assert!(src.len() >= width * 4);
    debug_assert!(dst.len() >= width * 3);
    let arch = pulp_arch();
    arch.dispatch(|| {
        for (d, s) in dst[..width * 3]
            .chunks_exact_mut(3)
            .zip(src[..width * 4].chunks_exact(4))
        {
            d.copy_from_slice(&s[..3]);
        }
    });
}

/// Copy a `w × h` window at (`x`, `y`) out of a tightly packed RGBA frame `frame_w` wide.
///
/// The window must lie inside the frame.
pub fn crop_packed_rgba(
    src: &[u8],
    frame_w: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> Vec<u8> {
    crop_rows(src, frame_w, x, y, w, h, 4, |s, d| d.copy_from_slice(s))
}

/// Like [`crop_packed_rgba`], dropping alpha (tightly packed RGB out).
pub fn crop_packed_rgba_to_rgb(
    src: &[u8],
    frame_w: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> Vec<u8> {
    crop_rows(src, frame_w, x, y, w, h, 3, |s, d| {
        strip_rgba_row_to_rgb(s, w, d)
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "private helper shared by the two crop kernels"
)]
fn crop_rows(
    src: &[u8],
    frame_w: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    out_bpp: usize,
    copy_row: impl Fn(&[u8], &mut [u8]) + Sync,
) -> Vec<u8> {
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut out = vec![0u8; w * h * out_bpp];
    let row = |(r, dst): (usize, &mut [u8])| {
        let s = ((y + r) * frame_w + x) * 4;
        copy_row(&src[s..s + w * 4], dst);
    };
    if h >= PARALLEL_ROW_GATE {
        out.par_chunks_mut(w * out_bpp).enumerate().for_each(row);
    } else {
        out.chunks_mut(w * out_bpp).enumerate().for_each(row);
    }
    out
}

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
            let arch = pulp_arch();
            arch.dispatch(|| {
                for (x, chunk) in row.chunks_exact(bpp).enumerate() {
                    let di = (y * w + x) * out_bpp;
                    let dst = out_addr as *mut u8;
                    // SAFETY: `out.len() == w * h * out_bpp` (checked above) bounds `di..di + out_bpp`;
                    // `out` outlives the blocking par loop and each row `y` writes a disjoint range.
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

    #[test]
    fn zero_stride_means_tightly_packed() {
        let data = [0, 0, 255, 255]; // BGRA red
        let packed = zpixmap_to_rgba(&data, 1, 1, 4, 0).unwrap();
        let explicit = zpixmap_to_rgba(&data, 1, 1, 4, 4).unwrap();
        assert_eq!(packed.get_pixel(0, 0), explicit.get_pixel(0, 0));
        assert_eq!(*packed.get_pixel(0, 0), image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn odd_width_3bpp() {
        let data = [
            1, 2, 3, // BGR
            4, 5, 6, 7, 8, 9, // two more pixels
        ];
        let img = zpixmap_to_rgba(&data, 3, 1, 3, 0).unwrap();
        assert_eq!(*img.get_pixel(0, 0), image::Rgba([3, 2, 1, 255]));
        assert_eq!(*img.get_pixel(2, 0), image::Rgba([9, 8, 7, 255]));
    }

    #[test]
    fn rejects_stride_shorter_than_row() {
        let data = vec![0u8; 16];
        let err = zpixmap_to_rgba(&data, 2, 1, 4, 4).unwrap_err();
        assert!(
            err.contains("shorter than"),
            "expected stride error, got {err}"
        );
    }

    #[test]
    fn shared_row_kernels_bgrx_bgra_rgbx() {
        let mut out = [0u8; 8];
        swizzle_row_to_rgba(
            &[0, 1, 2, 0, 10, 11, 12, 0],
            2,
            RgbaSrcFormat::Bgrx,
            &mut out,
        );
        assert_eq!(out, [2, 1, 0, 255, 12, 11, 10, 255]);

        swizzle_row_to_rgba(
            &[0, 1, 2, 9, 10, 11, 12, 8],
            2,
            RgbaSrcFormat::Bgra,
            &mut out,
        );
        assert_eq!(out, [2, 1, 0, 9, 12, 11, 10, 8]);

        swizzle_row_to_rgba(&[1, 2, 3, 0, 4, 5, 6, 0], 2, RgbaSrcFormat::Rgbx, &mut out);
        assert_eq!(out, [1, 2, 3, 255, 4, 5, 6, 255]);
    }

    /// `frame_w × frame_h` RGBA frame; pixel (x, y) = [x, y, 9, 200].
    fn coord_frame(frame_w: usize, frame_h: usize) -> Vec<u8> {
        (0..frame_h)
            .flat_map(|y| (0..frame_w).flat_map(move |x| [x as u8, y as u8, 9, 200]))
            .collect()
    }

    #[test]
    fn crop_packed_rgba_copies_the_window() {
        let frame = coord_frame(5, 4);
        let out = crop_packed_rgba(&frame, 5, 1, 2, 3, 2);
        assert_eq!(out.len(), 3 * 2 * 4);
        assert_eq!(&out[..4], &[1, 2, 9, 200]);
        assert_eq!(&out[out.len() - 4..], &[3, 3, 9, 200]);
    }

    #[test]
    fn crop_packed_rgba_to_rgb_drops_alpha() {
        let frame = coord_frame(4, 3);
        let out = crop_packed_rgba_to_rgb(&frame, 4, 2, 1, 2, 2);
        assert_eq!(out, vec![2, 1, 9, 3, 1, 9, 2, 2, 9, 3, 2, 9]);
    }

    #[test]
    fn crop_parallel_matches_serial_rows() {
        let (fw, fh) = (40, PARALLEL_ROW_GATE + 8);
        let frame = coord_frame(fw, fh);
        let h = PARALLEL_ROW_GATE + 2;
        let out = crop_packed_rgba_to_rgb(&frame, fw, 3, 4, 10, h);
        for r in 0..h {
            let px = &out[r * 10 * 3..r * 10 * 3 + 3];
            assert_eq!(px, &[3, (4 + r) as u8, 9], "row {r}");
        }
    }

    #[test]
    fn crop_empty_window_is_empty() {
        let frame = coord_frame(2, 2);
        assert!(crop_packed_rgba(&frame, 2, 0, 0, 0, 2).is_empty());
        assert!(crop_packed_rgba_to_rgb(&frame, 2, 0, 0, 2, 0).is_empty());
    }

    #[test]
    fn strip_rgba_row_drops_alpha() {
        let src = [1u8, 2, 3, 255, 4, 5, 6, 128];
        let mut dst = [0u8; 6];
        strip_rgba_row_to_rgb(&src, 2, &mut dst);
        assert_eq!(dst, [1, 2, 3, 4, 5, 6]);
    }
}
