use image::RgbaImage;
use pulp::Arch;
use rayon::prelude::*;
use sqyre_match::{map_rgb_to_gray_u8, ImageBuf};

/// Convert RGBA capture to 3-channel RGB `ImageBuf` (R, G, B byte order).
pub fn rgba_to_rgb_buf(img: &RgbaImage) -> ImageBuf {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let src = img.as_raw();
    let mut data = vec![0u8; w * h * 3];
    let data_addr = data.as_mut_ptr() as usize;
    (0..h).into_par_iter().for_each(|y| {
        let arch = Arch::new();
        arch.dispatch(|| {
            for x in 0..w {
                let si = (y * w + x) * 4;
                let di = (y * w + x) * 3;
                // SAFETY: each row writes a disjoint range of `data`.
                let dst = data_addr as *mut u8;
                unsafe {
                    *dst.add(di) = src[si];
                    *dst.add(di + 1) = src[si + 1];
                    *dst.add(di + 2) = src[si + 2];
                }
            }
        });
    });
    ImageBuf::from_raw(w, h, 3, data)
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
    let arch = Arch::new();
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

pub fn load_rgb_image(path: &std::path::Path) -> Result<ImageBuf, String> {
    let img = image::open(path)
        .map_err(|e| format!("load {}: {e}", path.display()))?
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
            // SAFETY: each output row `y` is disjoint.
            let dst = data_addr as *mut u8;
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
}
