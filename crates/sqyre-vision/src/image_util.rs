use image::RgbaImage;
use pulp::Arch;
use rayon::prelude::*;
use sqyre_domain::PlaneFlip;
use sqyre_match::{map_rgb_to_gray_u8, ImageBuf};
use sqyre_ports::{PortError, RgbCapture};

/// Cached `pulp` ISA dispatch — avoid `Arch::new()` on every gray→RGB expand.
#[inline]
fn pulp_arch() -> Arch {
    thread_local! {
        static ARCH: Arch = Arch::new();
    }
    ARCH.with(|a| *a)
}

/// Wrap a packed RGB capture as an `ImageBuf` (reuses the pixel buffer).
pub fn rgb_capture_to_image_buf(capture: RgbCapture) -> ImageBuf {
    ImageBuf::from_raw(
        capture.width as usize,
        capture.height as usize,
        3,
        capture.data,
    )
}

/// Convert RGBA capture to 3-channel RGB `ImageBuf` (R, G, B byte order).
pub fn rgba_to_rgb_buf(img: &RgbaImage) -> ImageBuf {
    rgb_capture_to_image_buf(RgbCapture::from_rgba(img))
}

/// Convert RGB `ImageBuf` to grayscale (Rec.601 luma).
pub fn rgb_to_grayscale(img: &ImageBuf) -> ImageBuf {
    if img.channels == 1 {
        return ImageBuf::from_raw(img.width, img.height, 1, img.data.clone());
    }
    let n = img.width * img.height;
    let mut data = vec![0u8; n];
    if img.channels == 3 {
        let row_bytes = img.width * 3;
        data.par_chunks_mut(img.width)
            .zip(img.data.par_chunks(row_bytes))
            .for_each(|(dst_row, src_row)| {
                map_rgb_to_gray_u8(src_row, dst_row);
            });
    } else {
        for (i, chunk) in img.data.chunks_exact(img.channels).enumerate() {
            let r = chunk[0] as f32;
            let g = chunk.get(1).copied().unwrap_or(0) as f32;
            let b = chunk.get(2).copied().unwrap_or(0) as f32;
            data[i] = (0.299 * r + 0.587 * g + 0.114 * b).round() as u8;
        }
    }
    ImageBuf::from_raw(img.width, img.height, 1, data)
}

/// Expand grayscale to RGB by duplicating the channel.
pub fn gray_to_rgb(img: &ImageBuf) -> ImageBuf {
    if img.channels == 3 {
        return img.clone();
    }
    let mut data = vec![0u8; img.width * img.height * 3];
    let arch = pulp_arch();
    arch.dispatch(|| {
        for (i, &v) in img.data.iter().enumerate() {
            let o = i * 3;
            data[o] = v;
            data[o + 1] = v;
            data[o + 2] = v;
        }
    });
    ImageBuf::from_raw(img.width, img.height, 3, data)
}

pub fn load_rgb_image(path: &std::path::Path) -> Result<ImageBuf, PortError> {
    let bytes = sqyre_ports::read_file_capped(path, sqyre_ports::MAX_IMAGE_FILE_BYTES)
        .map_err(|e| PortError::Message(format!("load {}: {e}", path.display())))?;
    let img = sqyre_ports::decode_image_bounded(&bytes)
        .map_err(|e| PortError::Message(format!("load {}: {e}", path.display())))?
        .to_rgb8();
    let w = img.width() as usize;
    let h = img.height() as usize;
    Ok(ImageBuf::from_raw(w, h, 3, img.into_raw()))
}

/// Flatten RGB/gray mask image to CV_8U-style single-channel bytes.
pub fn mask_as_u8(mask: &ImageBuf) -> Vec<u8> {
    if mask.channels == 1 {
        return mask.data.clone();
    }
    let mut out = Vec::with_capacity(mask.width * mask.height);
    for i in 0..mask.width * mask.height {
        let o = i * mask.channels;
        // Luma-ish: any non-zero channel counts as white.
        let v = if mask.data[o] | mask.data[o + 1] | mask.data.get(o + 2).copied().unwrap_or(0) > 0
        {
            255
        } else {
            0
        };
        out.push(v);
    }
    out
}

/// Nearest-neighbor resize of any channel count to `tw` × `th`.
pub fn resize_nearest(img: &ImageBuf, tw: usize, th: usize) -> ImageBuf {
    if img.width == tw && img.height == th {
        return img.clone();
    }
    let ch = img.channels;
    let mut data = vec![0u8; tw * th * ch];
    let data_addr = data.as_mut_ptr() as usize;
    let src = img.data.as_slice();
    let sw = img.width;
    let sh = img.height;
    (0..th).into_par_iter().for_each(|y| {
        let sy = y * sh / th;
        for x in 0..tw {
            let sx = x * sw / tw;
            let s = (sy * sw + sx) * ch;
            let d = (y * tw + x) * ch;
            let dst = data_addr as *mut u8;
            // SAFETY: `sx < sw`, `sy < sh` keep `s + ch <= src.len()`; `d + ch <= data.len()`, `data`
            // outlives the blocking par loop, and each row `y` writes a disjoint output range.
            unsafe {
                std::ptr::copy_nonoverlapping(src.as_ptr().add(s), dst.add(d), ch);
            }
        }
    });
    ImageBuf::from_raw(tw, th, ch, data)
}

/// Nearest-neighbor resize of a 1-channel mask to template size.
pub fn resize_mask(mask: &ImageBuf, tw: usize, th: usize) -> ImageBuf {
    let src = if mask.channels == 1 {
        mask.clone()
    } else {
        ImageBuf::from_raw(mask.width, mask.height, 1, mask_as_u8(mask))
    };
    resize_nearest(&src, tw, th)
}

/// Mirror `img` across the requested axes. Size is unchanged.
pub fn flip_image_buf(img: &ImageBuf, flip: PlaneFlip) -> ImageBuf {
    if flip.is_none() {
        return img.clone();
    }
    let w = img.width;
    let h = img.height;
    let ch = img.channels;
    let mut out = ImageBuf::new(w, h, ch, 0);
    let flip_h = flip.flip_horizontal();
    let flip_v = flip.flip_vertical();
    for y in 0..h {
        let sy = if flip_v { h - 1 - y } else { y };
        for x in 0..w {
            let sx = if flip_h { w - 1 - x } else { x };
            let si = img.pixel_offset(sx, sy);
            let di = out.pixel_offset(x, y);
            out.data[di..di + ch].copy_from_slice(&img.data[si..si + ch]);
        }
    }
    out
}

/// Mirror a single-channel mask (length `w * h`) with the same geometry as [`flip_image_buf`].
pub fn flip_mask_u8(mask: &[u8], w: usize, h: usize, flip: PlaneFlip) -> Vec<u8> {
    debug_assert_eq!(mask.len(), w.saturating_mul(h));
    if flip.is_none() || w == 0 || h == 0 {
        return mask.to_vec();
    }
    let mut out = vec![0u8; w * h];
    let flip_h = flip.flip_horizontal();
    let flip_v = flip.flip_vertical();
    for y in 0..h {
        let sy = if flip_v { h - 1 - y } else { y };
        for x in 0..w {
            let sx = if flip_h { w - 1 - x } else { x };
            out[y * w + x] = mask[sy * w + sx];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_round_trip_dims() {
        let img = RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
        let buf = rgba_to_rgb_buf(&img);
        assert_eq!((buf.width, buf.height, buf.channels), (2, 2, 3));
        assert_eq!(&buf.data[..3], &[1, 2, 3]);
    }

    #[test]
    fn gray_rgb_round_trip() {
        let gray = ImageBuf::from_raw(2, 1, 1, vec![10, 20]);
        let rgb = gray_to_rgb(&gray);
        assert_eq!(rgb.data, vec![10, 10, 10, 20, 20, 20]);
    }

    #[test]
    fn flip_horizontal_swaps_columns() {
        // 2x1 RGB: left=[1,2,3] right=[4,5,6]
        let img = ImageBuf::from_raw(2, 1, 3, vec![1, 2, 3, 4, 5, 6]);
        let flipped = flip_image_buf(&img, PlaneFlip::Horizontal);
        assert_eq!(flipped.data, vec![4, 5, 6, 1, 2, 3]);
    }

    #[test]
    fn flip_vertical_swaps_rows() {
        // 1x2 RGB
        let img = ImageBuf::from_raw(1, 2, 3, vec![1, 2, 3, 4, 5, 6]);
        let flipped = flip_image_buf(&img, PlaneFlip::Vertical);
        assert_eq!(flipped.data, vec![4, 5, 6, 1, 2, 3]);
    }

    #[test]
    fn flip_both_mirrors_both_axes() {
        // 2x2 single-channel: [[1,2],[3,4]]
        let img = ImageBuf::from_raw(2, 2, 1, vec![1, 2, 3, 4]);
        let flipped = flip_image_buf(&img, PlaneFlip::Both);
        assert_eq!(flipped.data, vec![4, 3, 2, 1]);
    }

    #[test]
    fn flip_mask_matches_image_geometry() {
        let mask = vec![1u8, 2, 3, 4];
        assert_eq!(
            flip_mask_u8(&mask, 2, 2, PlaneFlip::Horizontal),
            vec![2, 1, 4, 3]
        );
        assert_eq!(
            flip_mask_u8(&mask, 2, 2, PlaneFlip::Vertical),
            vec![3, 4, 1, 2]
        );
        assert_eq!(flip_mask_u8(&mask, 2, 2, PlaneFlip::Both), vec![4, 3, 2, 1]);
    }
}
