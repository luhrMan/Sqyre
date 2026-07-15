use image::RgbaImage;
use sqyre_match::ImageBuf;

/// Convert RGBA capture to 3-channel RGB `ImageBuf` (OpenCV ImageToMatRGB order).
pub fn rgba_to_rgb_buf(img: &RgbaImage) -> ImageBuf {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mut data = Vec::with_capacity(w * h * 3);
    for p in img.pixels() {
        data.push(p.0[0]);
        data.push(p.0[1]);
        data.push(p.0[2]);
    }
    ImageBuf::from_raw(w, h, 3, data)
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
        let v = if mask.data[o] | mask.data[o + 1] | mask.data.get(o + 2).copied().unwrap_or(0) > 0 {
            255
        } else {
            0
        };
        out.push(v);
    }
    out
}

/// Nearest-neighbor resize of a 1-channel mask to template size.
pub fn resize_mask(mask: &ImageBuf, tw: usize, th: usize) -> ImageBuf {
    let src = if mask.channels == 1 {
        mask.clone()
    } else {
        ImageBuf::from_raw(mask.width, mask.height, 1, mask_as_u8(mask))
    };
    if src.width == tw && src.height == th {
        return src;
    }
    let mut data = vec![0u8; tw * th];
    for y in 0..th {
        let sy = y * src.height / th;
        for x in 0..tw {
            let sx = x * src.width / tw;
            data[y * tw + x] = src.data[sy * src.width + sx];
        }
    }
    ImageBuf::from_raw(tw, th, 1, data)
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
}
