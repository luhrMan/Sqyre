//! OCR capture preprocessing (Go `vision.PreprocessOptions` / `preprocessCaptureMat`).

use sqyre_match::{blur_image, search_blur_kernel, ImageBuf};

/// One intermediate image from OCR preprocessing (chronological).
#[derive(Debug, Clone)]
pub struct OcrPreprocessStep {
    pub label: String,
    pub image: ImageBuf,
}

/// Options mirroring Go `PreprocessOptions` for OCR.
#[derive(Debug, Clone, Copy)]
pub struct OcrPreprocessOptions {
    pub grayscale: bool,
    pub blur: bool,
    pub blur_amount: i32,
    pub threshold: bool,
    pub min_threshold: f32,
    pub threshold_otsu: bool,
    pub threshold_invert: bool,
    pub resize: bool,
    pub resize_scale: f64,
}

impl Default for OcrPreprocessOptions {
    fn default() -> Self {
        Self {
            grayscale: true,
            blur: true,
            blur_amount: 1,
            threshold: false,
            min_threshold: 0.0,
            threshold_otsu: false,
            threshold_invert: false,
            resize: false,
            resize_scale: 1.0,
        }
    }
}

impl OcrPreprocessOptions {
    /// Build from OCR action fields (Go `ocrCapture` options construction).
    pub fn from_action_fields(
        grayscale: bool,
        blur: i32,
        min_threshold: i32,
        resize: f64,
        threshold_otsu: bool,
        threshold_invert: bool,
    ) -> Self {
        Self {
            grayscale,
            blur: blur > 0,
            blur_amount: blur,
            threshold: min_threshold > 0 || threshold_otsu,
            min_threshold: min_threshold as f32,
            threshold_otsu,
            threshold_invert,
            resize: (resize - 1.0).abs() > f64::EPSILON,
            resize_scale: resize,
        }
    }
}

/// Preprocess an RGB (or gray) capture for OCR. Returns the processed buffer and the
/// effective resize scale applied (1.0 when resize is off / invalid).
pub fn preprocess_for_ocr(
    img: &ImageBuf,
    opts: OcrPreprocessOptions,
) -> Result<(ImageBuf, f64), String> {
    let (out, scale, _) = preprocess_for_ocr_with_steps(img, opts, false)?;
    Ok((out, scale))
}

/// Like [`preprocess_for_ocr`], optionally collecting each applied step for UI logs.
pub fn preprocess_for_ocr_with_steps(
    img: &ImageBuf,
    opts: OcrPreprocessOptions,
    collect_steps: bool,
) -> Result<(ImageBuf, f64, Vec<OcrPreprocessStep>), String> {
    if img.width == 0 || img.height == 0 {
        return Err("empty OCR image".into());
    }
    let mut steps = Vec::new();
    let mut step_n = 0u32;

    let mut cur = if opts.grayscale {
        let g = to_grayscale(img);
        if collect_steps {
            step_n += 1;
            steps.push(OcrPreprocessStep {
                label: format!("{step_n}. Grayscale"),
                image: g.clone(),
            });
        }
        g
    } else if img.channels == 1 || img.channels == 3 {
        img.clone()
    } else {
        return Err(format!("unsupported OCR channels {}", img.channels));
    };

    if opts.blur && opts.blur_amount > 0 {
        let k = search_blur_kernel(opts.blur_amount);
        cur = blur_image(&cur, k).map_err(|e| format!("OCR blur: {e}"))?;
        if collect_steps {
            step_n += 1;
            steps.push(OcrPreprocessStep {
                label: format!("{step_n}. Blur (amount={})", opts.blur_amount),
                image: cur.clone(),
            });
        }
    }

    if opts.threshold {
        if cur.channels != 1 {
            cur = to_grayscale(&cur);
        }
        let thresh = if opts.threshold_otsu {
            otsu_threshold(&cur.data)
        } else {
            opts.min_threshold.clamp(0.0, 255.0) as u8
        };
        apply_threshold(&mut cur, thresh, opts.threshold_invert);
        morph_open_2x2(&mut cur);
        if collect_steps {
            step_n += 1;
            let kind = if opts.threshold_otsu {
                format!("Threshold Otsu→{thresh}")
            } else {
                format!("Threshold ≥{thresh}")
            };
            let label = if opts.threshold_invert {
                format!("{step_n}. {kind} (invert)")
            } else {
                format!("{step_n}. {kind}")
            };
            steps.push(OcrPreprocessStep {
                label,
                image: cur.clone(),
            });
        }
    }

    let mut scale = 1.0_f64;
    if opts.resize && opts.resize_scale > 0.0 && (opts.resize_scale - 1.0).abs() > f64::EPSILON {
        scale = opts.resize_scale;
        cur = resize_image(&cur, scale)?;
        if collect_steps {
            step_n += 1;
            steps.push(OcrPreprocessStep {
                label: format!("{step_n}. Resize ×{scale:.2}"),
                image: cur.clone(),
            });
        }
    }

    if cur.width == 0 || cur.height == 0 {
        return Err("preprocessing produced empty image".into());
    }
    if collect_steps {
        step_n += 1;
        steps.push(OcrPreprocessStep {
            label: format!("{step_n}. Ready for OCR"),
            image: cur.clone(),
        });
    }
    Ok((cur, scale, steps))
}

fn to_grayscale(img: &ImageBuf) -> ImageBuf {
    if img.channels == 1 {
        return img.clone();
    }
    let mut data = Vec::with_capacity(img.width * img.height);
    for i in 0..img.width * img.height {
        let o = i * img.channels;
        let r = img.data[o] as u32;
        let g = img.data[o + 1] as u32;
        let b = img.data[o + 2] as u32;
        // OpenCV BGR→Gray on RGB buffer: approximate Rec.601
        data.push(((299 * r + 587 * g + 114 * b) / 1000) as u8);
    }
    ImageBuf::from_raw(img.width, img.height, 1, data)
}

fn otsu_threshold(hist_src: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &p in hist_src {
        hist[p as usize] += 1;
    }
    let total = hist_src.len() as f64;
    if total == 0.0 {
        return 0;
    }
    let mut sum_all = 0.0_f64;
    for (i, &c) in hist.iter().enumerate() {
        sum_all += i as f64 * c as f64;
    }
    let mut sum_b = 0.0_f64;
    let mut w_b = 0.0_f64;
    let mut max_var = -1.0_f64;
    let mut threshold = 0u8;
    for t in 0..256 {
        w_b += hist[t] as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let m_b = sum_b / w_b;
        let m_f = (sum_all - sum_b) / w_f;
        let var = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if var > max_var {
            max_var = var;
            threshold = t as u8;
        }
    }
    threshold
}

fn apply_threshold(img: &mut ImageBuf, thresh: u8, invert: bool) {
    debug_assert_eq!(img.channels, 1);
    for p in &mut img.data {
        let above = *p >= thresh;
        *p = match (above, invert) {
            (true, false) | (false, true) => 255,
            (false, false) | (true, true) => 0,
        };
    }
}

/// 2×2 rectangular morphological open (erode then dilate), matching Go after threshold.
fn morph_open_2x2(img: &mut ImageBuf) {
    debug_assert_eq!(img.channels, 1);
    let eroded = erode_2x2(img);
    *img = dilate_2x2(&eroded);
}

fn erode_2x2(img: &ImageBuf) -> ImageBuf {
    let w = img.width;
    let h = img.height;
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut min_v = 255u8;
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = x + dx;
                    let sy = y + dy;
                    if sx < w && sy < h {
                        min_v = min_v.min(img.data[sy * w + sx]);
                    }
                }
            }
            out[y * w + x] = min_v;
        }
    }
    ImageBuf::from_raw(w, h, 1, out)
}

fn dilate_2x2(img: &ImageBuf) -> ImageBuf {
    let w = img.width;
    let h = img.height;
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut max_v = 0u8;
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = x as isize - dx as isize;
                    let sy = y as isize - dy as isize;
                    if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                        max_v = max_v.max(img.data[sy as usize * w + sx as usize]);
                    }
                }
            }
            out[y * w + x] = max_v;
        }
    }
    ImageBuf::from_raw(w, h, 1, out)
}

fn resize_image(img: &ImageBuf, scale: f64) -> Result<ImageBuf, String> {
    if scale <= 0.0 {
        return Err("invalid resize scale".into());
    }
    let nw = ((img.width as f64) * scale).round().max(1.0) as usize;
    let nh = ((img.height as f64) * scale).round().max(1.0) as usize;
    let ch = img.channels;
    let mut data = vec![0u8; nw * nh * ch];
    // Cubic-ish when upscaling, nearest when down — nearest is fine for OCR prep parity tests.
    for y in 0..nh {
        let sy = ((y as f64) / scale).floor() as usize;
        let sy = sy.min(img.height - 1);
        for x in 0..nw {
            let sx = ((x as f64) / scale).floor() as usize;
            let sx = sx.min(img.width - 1);
            let si = img.pixel_offset(sx, sy);
            let di = (y * nw + x) * ch;
            data[di..di + ch].copy_from_slice(&img.data[si..si + ch]);
        }
    }
    Ok(ImageBuf::from_raw(nw, nh, ch, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grayscale_and_threshold() {
        let img = ImageBuf::from_raw(2, 2, 3, vec![0, 0, 0, 255, 255, 255, 0, 0, 0, 255, 255, 255]);
        let opts = OcrPreprocessOptions {
            grayscale: true,
            blur: false,
            blur_amount: 0,
            threshold: true,
            min_threshold: 128.0,
            threshold_otsu: false,
            threshold_invert: false,
            resize: false,
            resize_scale: 1.0,
        };
        let (out, scale) = preprocess_for_ocr(&img, opts).unwrap();
        assert_eq!(scale, 1.0);
        assert_eq!(out.channels, 1);
        assert!(out.data.iter().any(|&p| p == 0) || out.data.iter().any(|&p| p == 255));
    }

    #[test]
    fn resize_doubles_dims() {
        let img = ImageBuf::new(4, 4, 1, 128);
        let opts = OcrPreprocessOptions {
            grayscale: false,
            blur: false,
            blur_amount: 0,
            threshold: false,
            min_threshold: 0.0,
            threshold_otsu: false,
            threshold_invert: false,
            resize: true,
            resize_scale: 2.0,
        };
        let (out, scale) = preprocess_for_ocr(&img, opts).unwrap();
        assert_eq!(scale, 2.0);
        assert_eq!((out.width, out.height), (8, 8));
    }

    #[test]
    fn with_steps_records_enabled_stages() {
        let img = ImageBuf::from_raw(2, 2, 3, vec![0, 0, 0, 255, 255, 255, 0, 0, 0, 255, 255, 255]);
        let opts = OcrPreprocessOptions {
            grayscale: true,
            blur: false,
            blur_amount: 0,
            threshold: true,
            min_threshold: 128.0,
            threshold_otsu: false,
            threshold_invert: false,
            resize: false,
            resize_scale: 1.0,
        };
        let (_, _, steps) = preprocess_for_ocr_with_steps(&img, opts, true).unwrap();
        assert!(steps.len() >= 3);
        assert!(steps[0].label.contains("Grayscale"));
        assert!(steps.iter().any(|s| s.label.contains("Threshold")));
        assert!(steps.last().unwrap().label.contains("Ready for OCR"));
        let (_, _, none) = preprocess_for_ocr_with_steps(&img, opts, false).unwrap();
        assert!(none.is_empty());
    }
}
