use crate::image::ImageBuf;
use thiserror::Error;

/// Correlation result map: size `(W−w+1)×(H−h+1)`, row-major `f32`.
#[derive(Clone, Debug)]
pub struct MatchMap {
    pub width: usize,
    pub height: usize,
    pub scores: Vec<f32>,
}

#[derive(Debug, Error)]
pub enum MatchError {
    #[error("search and template channel counts differ ({search} vs {template})")]
    ChannelMismatch { search: usize, template: usize },
    #[error("template ({tw}x{th}) larger than search ({sw}x{sh})")]
    TemplateTooLarge {
        sw: usize,
        sh: usize,
        tw: usize,
        th: usize,
    },
    #[error("mask length {got} does not match template area {want}")]
    MaskSize { got: usize, want: usize },
    #[error("blur failed: {0}")]
    Blur(String),
    #[error("empty image")]
    Empty,
}

/// Naive OpenCV `TM_CCOEFF_NORMED` (method 5) with optional binary CV_8U mask.
///
/// Per-channel means over masked pixels; numerator/denominator sum across channels.
/// Empty mask (`None` or all-nonzero equivalent path) uses every pixel.
pub fn match_ccoeff_normed(
    search: &ImageBuf,
    template: &ImageBuf,
    mask: Option<&[u8]>,
) -> Result<MatchMap, MatchError> {
    if search.width == 0 || search.height == 0 || template.width == 0 || template.height == 0 {
        return Err(MatchError::Empty);
    }
    if search.channels != template.channels {
        return Err(MatchError::ChannelMismatch {
            search: search.channels,
            template: template.channels,
        });
    }
    if template.width > search.width || template.height > search.height {
        return Err(MatchError::TemplateTooLarge {
            sw: search.width,
            sh: search.height,
            tw: template.width,
            th: template.height,
        });
    }

    let mask_bits = prep_mask(template.width, template.height, mask)?;
    let ch = search.channels;
    let tw = template.width;
    let th = template.height;
    let out_w = search.width - tw + 1;
    let out_h = search.height - th + 1;
    let mut scores = vec![0.0_f32; out_w * out_h];

    // Template means (per channel) and fixed denom_t components.
    let (t_mean, sum_w) = window_means(template, &mask_bits, 0, 0, tw, th, ch);
    if sum_w <= 0.0 {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores,
        });
    }

    let mut t_prime_sq = 0.0_f64;
    let mut t_prime = vec![0.0_f64; tw * th * ch];
    for y in 0..th {
        for x in 0..tw {
            if !mask_bits[y * tw + x] {
                continue;
            }
            let ti = template.pixel_offset(x, y);
            let pi = (y * tw + x) * ch;
            for c in 0..ch {
                let tp = template.data[ti + c] as f64 - t_mean[c];
                t_prime[pi + c] = tp;
                t_prime_sq += tp * tp;
            }
        }
    }

    for oy in 0..out_h {
        for ox in 0..out_w {
            let (i_mean, _) = window_means(search, &mask_bits, ox, oy, tw, th, ch);
            let mut numer = 0.0_f64;
            let mut i_prime_sq = 0.0_f64;
            for y in 0..th {
                for x in 0..tw {
                    if !mask_bits[y * tw + x] {
                        continue;
                    }
                    let si = search.pixel_offset(ox + x, oy + y);
                    let pi = (y * tw + x) * ch;
                    for c in 0..ch {
                        let ip = search.data[si + c] as f64 - i_mean[c];
                        numer += t_prime[pi + c] * ip;
                        i_prime_sq += ip * ip;
                    }
                }
            }
            let denom = (t_prime_sq * i_prime_sq).sqrt();
            // OpenCV TM_CCOEFF_NORMED: near-zero variance → 0 (not 1).
            let score = if denom > f64::EPSILON {
                (numer / denom) as f32
            } else {
                0.0
            };
            scores[oy * out_w + ox] = score;
        }
    }

    Ok(MatchMap {
        width: out_w,
        height: out_h,
        scores,
    })
}

fn prep_mask(tw: usize, th: usize, mask: Option<&[u8]>) -> Result<Vec<bool>, MatchError> {
    let area = tw * th;
    match mask {
        None => Ok(vec![true; area]),
        Some(m) if m.is_empty() => Ok(vec![true; area]),
        Some(m) if m.len() != area => Err(MatchError::MaskSize {
            got: m.len(),
            want: area,
        }),
        Some(m) => Ok(m.iter().map(|&v| v != 0).collect()),
    }
}

/// Per-channel mean over masked template-window pixels placed at `(ox, oy)` in `img`.
fn window_means(
    img: &ImageBuf,
    mask_bits: &[bool],
    ox: usize,
    oy: usize,
    tw: usize,
    th: usize,
    ch: usize,
) -> (Vec<f64>, f64) {
    let mut sums = vec![0.0_f64; ch];
    let mut count = 0.0_f64;
    for y in 0..th {
        for x in 0..tw {
            if !mask_bits[y * tw + x] {
                continue;
            }
            count += 1.0;
            let i = img.pixel_offset(ox + x, oy + y);
            for c in 0..ch {
                sums[c] += img.data[i + c] as f64;
            }
        }
    }
    if count <= 0.0 {
        return (sums, 0.0);
    }
    for c in 0..ch {
        sums[c] /= count;
    }
    (sums, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blur::{blur_image, search_blur_kernel};
    use crate::peaks::{find_peaks, DEFAULT_CLOSE_MATCHES_DISTANCE};

    /// Patterned icon: CCOEFF needs spatial/channel variance (flat patches are degenerate).
    fn patterned(w: usize, h: usize) -> ImageBuf {
        let mut img = ImageBuf::new(w, h, 3, 0);
        for y in 0..h {
            for x in 0..w {
                let i = img.pixel_offset(x, y);
                img.data[i] = ((x * 37 + y * 17) % 200 + 40) as u8;
                img.data[i + 1] = ((x * 13 + y * 41) % 180 + 50) as u8;
                img.data[i + 2] = ((x * 29 + y * 7) % 160 + 60) as u8;
            }
        }
        img
    }

    fn gray(w: usize, h: usize, v: u8) -> ImageBuf {
        ImageBuf::new(w, h, 3, v)
    }

    #[test]
    fn finds_stamped_template_top_left() {
        let tmpl = patterned(8, 8);
        let mut search = gray(40, 40, 30);
        search.stamp(&tmpl, 12, 7);

        let map = match_ccoeff_normed(&search, &tmpl, None).unwrap();
        let matches = find_peaks(&map, 0.95, DEFAULT_CLOSE_MATCHES_DISTANCE);
        assert!(
            matches.iter().any(|p| p.x == 12 && p.y == 7),
            "expected peak at (12,7), got {matches:?}"
        );
        let idx = 7 * map.width + 12;
        assert!(
            map.scores[idx] >= 0.99,
            "perfect stamp should score ~1, got {}",
            map.scores[idx]
        );
    }

    #[test]
    fn masked_match_ignores_outside_circle() {
        let tmpl = patterned(9, 9);
        let mut masked_tmpl = tmpl.clone();
        for y in 0..9 {
            for x in 0..9 {
                if (x as i32 - 4).pow(2) + (y as i32 - 4).pow(2) > 9 {
                    let i = masked_tmpl.pixel_offset(x, y);
                    masked_tmpl.data[i..i + 3].copy_from_slice(&[255, 0, 255]);
                }
            }
        }
        let mut mask = vec![0_u8; 81];
        for y in 0..9 {
            for x in 0..9 {
                if (x as i32 - 4).pow(2) + (y as i32 - 4).pow(2) <= 9 {
                    mask[y * 9 + x] = 255;
                }
            }
        }

        let mut search = gray(30, 30, 20);
        // Stamp only the masked pixels' pattern into the search image.
        for y in 0..9 {
            for x in 0..9 {
                if mask[y * 9 + x] == 0 {
                    continue;
                }
                let si = search.pixel_offset(5 + x, 5 + y);
                let ti = tmpl.pixel_offset(x, y);
                search.data[si..si + 3].copy_from_slice(&tmpl.data[ti..ti + 3]);
            }
        }

        let map = match_ccoeff_normed(&search, &masked_tmpl, Some(&mask)).unwrap();
        let matches = find_peaks(&map, 0.9, DEFAULT_CLOSE_MATCHES_DISTANCE);
        assert!(
            matches.iter().any(|p| p.x == 5 && p.y == 5),
            "masked peak at (5,5), got {matches:?}; score={}",
            map.scores[5 * map.width + 5]
        );
    }

    #[test]
    fn blur_roundtrip_still_finds_stamp() {
        let tmpl = patterned(10, 10);
        let mut search = gray(50, 50, 40);
        search.stamp(&tmpl, 15, 18);
        let k = search_blur_kernel(5);
        let search_b = blur_image(&search, k).unwrap();
        let tmpl_b = blur_image(&tmpl, k).unwrap();
        assert_eq!(search_b.channels, 3);
        assert_eq!(search_b.data.len(), 50 * 50 * 3);
        let map = match_ccoeff_normed(&search_b, &tmpl_b, None).unwrap();
        let (bx, by, best) = map
            .scores
            .iter()
            .enumerate()
            .map(|(i, &s)| (i % map.width, i / map.width, s))
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .unwrap();
        assert!(
            best >= 0.7,
            "expected strong peak after blur, best={best} at ({bx},{by})"
        );
        assert!(
            (bx as i32 - 15).abs() <= 2 && (by as i32 - 18).abs() <= 2,
            "blurred peak near (15,18), best={best} at ({bx},{by})"
        );
    }
}
