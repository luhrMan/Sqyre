use crate::image::ImageBuf;
use rayon::prelude::*;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
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

/// Switch to FFT when direct correlation would touch this many pixel·channel ops.
const FFT_DIRECT_COST_THRESHOLD: u64 = 4_000_000;

/// OpenCV `TM_CCOEFF_NORMED` (method 5) with optional binary CV_8U mask.
///
/// Large unmasked searches use DFT cross-correlation (OpenCV `crossCorr` path).
/// Small / masked searches use a packed direct correlator with integral images.
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

    let (masked, t_prime_sq, sum_w) = build_t_prime(template, &mask_bits, ch);
    if sum_w <= 0.0 {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores: vec![0.0; out_w * out_h],
        });
    }

    let full_mask = mask_bits.iter().all(|&b| b);
    let direct_cost = (out_w as u64)
        .saturating_mul(out_h as u64)
        .saturating_mul(tw as u64)
        .saturating_mul(th as u64)
        .saturating_mul(ch as u64);

    if full_mask && direct_cost > FFT_DIRECT_COST_THRESHOLD {
        match_fft(search, &masked, tw, th, sum_w, t_prime_sq, out_w, out_h, ch)
    } else {
        match_direct(
            search,
            &masked,
            tw,
            th,
            sum_w,
            t_prime_sq,
            out_w,
            out_h,
            ch,
            full_mask,
        )
    }
}

fn build_t_prime(
    template: &ImageBuf,
    mask_bits: &[bool],
    ch: usize,
) -> (Vec<(usize, Vec<f64>)>, f64, f64) {
    let tw = template.width;
    let th = template.height;
    let mut t_mean = vec![0.0_f64; ch];
    let mut sum_w = 0.0_f64;
    for y in 0..th {
        for x in 0..tw {
            let li = y * tw + x;
            if !mask_bits[li] {
                continue;
            }
            sum_w += 1.0;
            let ti = template.pixel_offset(x, y);
            for c in 0..ch {
                t_mean[c] += template.data[ti + c] as f64;
            }
        }
    }
    if sum_w <= 0.0 {
        return (Vec::new(), 0.0, 0.0);
    }
    for c in 0..ch {
        t_mean[c] /= sum_w;
    }

    let mut masked = Vec::with_capacity((sum_w as usize).max(1));
    let mut t_prime_sq = 0.0_f64;
    for y in 0..th {
        for x in 0..tw {
            let li = y * tw + x;
            if !mask_bits[li] {
                continue;
            }
            let ti = template.pixel_offset(x, y);
            let mut primed = vec![0.0_f64; ch];
            for c in 0..ch {
                let tp = template.data[ti + c] as f64 - t_mean[c];
                primed[c] = tp;
                t_prime_sq += tp * tp;
            }
            masked.push((li, primed));
        }
    }
    (masked, t_prime_sq, sum_w)
}

fn match_direct(
    search: &ImageBuf,
    masked: &[(usize, Vec<f64>)],
    tw: usize,
    th: usize,
    n: f64,
    t_prime_sq: f64,
    out_w: usize,
    out_h: usize,
    ch: usize,
    full_mask: bool,
) -> Result<MatchMap, MatchError> {
    let integrals = if full_mask {
        Some(build_integrals(search))
    } else {
        None
    };
    let search_w = search.width;
    let search_data = &search.data;

    let mut scores = vec![0.0_f32; out_w * out_h];
    scores
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(oy, row)| {
            for ox in 0..out_w {
                let (numer, i_prime_sq) = if let Some(integ) = integrals.as_ref() {
                    score_unmasked(integ, search_data, search_w, ch, ox, oy, tw, th, n, masked)
                } else {
                    score_masked(search_data, search_w, ch, ox, oy, tw, masked, n)
                };
                let denom = (t_prime_sq * i_prime_sq).sqrt();
                row[ox] = if denom > f64::EPSILON {
                    (numer / denom) as f32
                } else {
                    0.0
                };
            }
        });

    Ok(MatchMap {
        width: out_w,
        height: out_h,
        scores,
    })
}

/// Next size >= n of form 2^a * 3^b * 5^c (same idea as OpenCV / PureCV).
fn optimal_dft_size(n: usize) -> usize {
    if n <= 1 {
        return n;
    }
    let mut best = usize::MAX;
    let mut p5 = 1usize;
    while p5 < n.saturating_mul(2) {
        let mut p35 = p5;
        while p35 < n.saturating_mul(2) {
            let mut p = p35;
            while p < n {
                p = p.saturating_mul(2);
            }
            if p >= n && p < best {
                best = p;
            }
            let Some(next) = p35.checked_mul(3) else {
                break;
            };
            p35 = next;
        }
        let Some(next) = p5.checked_mul(5) else {
            break;
        };
        p5 = next;
    }
    best
}

/// DFT cross-correlation of mean-subtracted template vs search (OpenCV `crossCorr` + CCOEFF).
fn match_fft(
    search: &ImageBuf,
    masked: &[(usize, Vec<f64>)],
    tw: usize,
    th: usize,
    n: f64,
    t_prime_sq: f64,
    out_w: usize,
    out_h: usize,
    ch: usize,
) -> Result<MatchMap, MatchError> {
    let dft_w = optimal_dft_size(search.width + tw - 1);
    let dft_h = optimal_dft_size(search.height + th - 1);
    let area = dft_w * dft_h;
    let scale = 1.0_f32 / area as f32;

    let channel_numers: Vec<Vec<f32>> = (0..ch)
        .into_par_iter()
        .map(|c| {
            let mut img = vec![Complex::new(0.0, 0.0); area];
            for y in 0..search.height {
                for x in 0..search.width {
                    let v = search.data[(y * search.width + x) * ch + c] as f32;
                    img[y * dft_w + x] = Complex::new(v, 0.0);
                }
            }
            let mut tmpl = vec![Complex::new(0.0, 0.0); area];
            for &(li, ref primed) in masked {
                let x = li % tw;
                let y = li / tw;
                tmpl[y * dft_w + x] = Complex::new(primed[c] as f32, 0.0);
            }

            let mut planner = FftPlanner::<f32>::new();
            fft2d_forward(&mut img, dft_w, dft_h, &mut planner);
            fft2d_forward(&mut tmpl, dft_w, dft_h, &mut planner);

            for i in 0..area {
                img[i] = img[i] * tmpl[i].conj();
            }
            fft2d_inverse(&mut img, dft_w, dft_h, &mut planner);

            let mut out = vec![0.0_f32; out_w * out_h];
            for y in 0..out_h {
                for x in 0..out_w {
                    out[y * out_w + x] = img[y * dft_w + x].re * scale;
                }
            }
            out
        })
        .collect();

    let mut numer = vec![0.0_f32; out_w * out_h];
    for ch_num in channel_numers {
        for (i, v) in ch_num.into_iter().enumerate() {
            numer[i] += v;
        }
    }

    let integ = build_integrals(search);
    let stride = integ.width + 1;
    let mut scores = vec![0.0_f32; out_w * out_h];
    scores
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(oy, row)| {
            for ox in 0..out_w {
                let mut i_prime_sq = 0.0_f64;
                for c in 0..ch {
                    let s = rect_sum(&integ.sum[c], stride, ox, oy, tw, th);
                    let sq = rect_sum(&integ.sumsq[c], stride, ox, oy, tw, th);
                    i_prime_sq += sq - (s * s) / n;
                }
                i_prime_sq = i_prime_sq.max(0.0);
                let denom = (t_prime_sq * i_prime_sq).sqrt();
                row[ox] = if denom > f64::EPSILON {
                    (numer[oy * out_w + ox] as f64 / denom) as f32
                } else {
                    0.0
                };
            }
        });

    Ok(MatchMap {
        width: out_w,
        height: out_h,
        scores,
    })
}

fn fft2d_forward(buf: &mut [Complex<f32>], width: usize, height: usize, planner: &mut FftPlanner<f32>) {
    let fft_row = planner.plan_fft_forward(width);
    for row in buf.chunks_exact_mut(width) {
        fft_row.process(row);
    }
    let fft_col = planner.plan_fft_forward(height);
    let mut col = vec![Complex::default(); height];
    for x in 0..width {
        for y in 0..height {
            col[y] = buf[y * width + x];
        }
        fft_col.process(&mut col);
        for y in 0..height {
            buf[y * width + x] = col[y];
        }
    }
}

fn fft2d_inverse(buf: &mut [Complex<f32>], width: usize, height: usize, planner: &mut FftPlanner<f32>) {
    let ifft_row = planner.plan_fft_inverse(width);
    for row in buf.chunks_exact_mut(width) {
        ifft_row.process(row);
    }
    let ifft_col = planner.plan_fft_inverse(height);
    let mut col = vec![Complex::default(); height];
    for x in 0..width {
        for y in 0..height {
            col[y] = buf[y * width + x];
        }
        ifft_col.process(&mut col);
        for y in 0..height {
            buf[y * width + x] = col[y];
        }
    }
}

struct Integrals {
    width: usize,
    sum: Vec<Vec<f64>>,
    sumsq: Vec<Vec<f64>>,
}

fn build_integrals(img: &ImageBuf) -> Integrals {
    let w = img.width;
    let h = img.height;
    let ch = img.channels;
    let stride = w + 1;
    let mut sum = vec![vec![0.0_f64; stride * (h + 1)]; ch];
    let mut sumsq = vec![vec![0.0_f64; stride * (h + 1)]; ch];
    for y in 0..h {
        for x in 0..w {
            let pi = img.pixel_offset(x, y);
            for c in 0..ch {
                let v = img.data[pi + c] as f64;
                let above = sum[c][y * stride + (x + 1)];
                let left = sum[c][(y + 1) * stride + x];
                let corner = sum[c][y * stride + x];
                sum[c][(y + 1) * stride + (x + 1)] = above + left - corner + v;

                let above_sq = sumsq[c][y * stride + (x + 1)];
                let left_sq = sumsq[c][(y + 1) * stride + x];
                let corner_sq = sumsq[c][y * stride + x];
                sumsq[c][(y + 1) * stride + (x + 1)] = above_sq + left_sq - corner_sq + v * v;
            }
        }
    }
    Integrals { width: w, sum, sumsq }
}

#[inline]
fn rect_sum(integ: &[f64], stride: usize, x: usize, y: usize, tw: usize, th: usize) -> f64 {
    let x2 = x + tw;
    let y2 = y + th;
    integ[y2 * stride + x2] - integ[y * stride + x2] - integ[y2 * stride + x] + integ[y * stride + x]
}

fn score_unmasked(
    integ: &Integrals,
    search_data: &[u8],
    search_w: usize,
    ch: usize,
    ox: usize,
    oy: usize,
    tw: usize,
    th: usize,
    n: f64,
    masked: &[(usize, Vec<f64>)],
) -> (f64, f64) {
    let stride = integ.width + 1;
    let mut i_prime_sq = 0.0_f64;
    for c in 0..ch {
        let s = rect_sum(&integ.sum[c], stride, ox, oy, tw, th);
        let sq = rect_sum(&integ.sumsq[c], stride, ox, oy, tw, th);
        i_prime_sq += sq - (s * s) / n;
    }
    let mut numer = 0.0_f64;
    for &(li, ref primed) in masked {
        let x = li % tw;
        let y = li / tw;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        for c in 0..ch {
            numer += primed[c] * search_data[si + c] as f64;
        }
    }
    (numer, i_prime_sq.max(0.0))
}

fn score_masked(
    search_data: &[u8],
    search_w: usize,
    ch: usize,
    ox: usize,
    oy: usize,
    tw: usize,
    masked: &[(usize, Vec<f64>)],
    n: f64,
) -> (f64, f64) {
    let mut i_sum = vec![0.0_f64; ch];
    for &(li, _) in masked {
        let x = li % tw;
        let y = li / tw;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        for c in 0..ch {
            i_sum[c] += search_data[si + c] as f64;
        }
    }
    for c in 0..ch {
        i_sum[c] /= n;
    }
    let i_mean = i_sum;

    let mut numer = 0.0_f64;
    let mut i_prime_sq = 0.0_f64;
    for &(li, ref primed) in masked {
        let x = li % tw;
        let y = li / tw;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        for c in 0..ch {
            let ip = search_data[si + c] as f64 - i_mean[c];
            numer += primed[c] * ip;
            i_prime_sq += ip * ip;
        }
    }
    (numer, i_prime_sq)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blur::{blur_image, search_blur_kernel};
    use crate::peaks::{find_peaks, DEFAULT_CLOSE_MATCHES_DISTANCE};
    use std::time::Instant;

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

    /// Wall-clock budgets assume `--release`. Default `cargo test` / CI is debug and much slower.
    fn perf_budget_secs(release_secs: f64) -> f64 {
        if cfg!(debug_assertions) {
            release_secs * 10.0
        } else {
            release_secs
        }
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

    #[test]
    fn large_search_completes_quickly() {
        let tmpl = patterned(32, 32);
        let mut search = gray(640, 480, 25);
        search.stamp(&tmpl, 200, 150);
        let t0 = Instant::now();
        let map = match_ccoeff_normed(&search, &tmpl, None).unwrap();
        let elapsed = t0.elapsed();
        let budget = perf_budget_secs(2.0);
        assert!(
            elapsed.as_secs_f64() < budget,
            "640x480 match took {elapsed:?} (budget {budget}s)"
        );
        let matches = find_peaks(&map, 0.95, DEFAULT_CLOSE_MATCHES_DISTANCE);
        assert!(
            matches
                .iter()
                .any(|p| (p.x - 200).abs() <= 1 && (p.y - 150).abs() <= 1),
            "expected peak near (200,150), got {matches:?}"
        );
    }

    #[test]
    fn huge_template_fft_path_is_fast() {
        let tmpl = patterned(120, 150);
        let mut search = gray(1100, 700, 20);
        search.stamp(&tmpl, 400, 200);
        let t0 = Instant::now();
        let map = match_ccoeff_normed(&search, &tmpl, None).unwrap();
        let elapsed = t0.elapsed();
        let budget = perf_budget_secs(5.0);
        assert!(
            elapsed.as_secs_f64() < budget,
            "1100x700 / 120x150 took {elapsed:?} (budget {budget}s) — FFT path broken?"
        );
        let matches = find_peaks(&map, 0.95, DEFAULT_CLOSE_MATCHES_DISTANCE);
        assert!(
            matches
                .iter()
                .any(|p| (p.x - 400).abs() <= 2 && (p.y - 200).abs() <= 2),
            "expected peak near (400,200), got {matches:?}"
        );
    }

    #[test]
    fn fft_matches_direct_scores() {
        let tmpl = patterned(24, 24);
        let mut search = gray(120, 100, 30);
        search.stamp(&tmpl, 40, 30);
        let mask_bits = vec![true; 24 * 24];
        let (masked, t_prime_sq, sum_w) = build_t_prime(&tmpl, &mask_bits, 3);
        let direct =
            match_direct(&search, &masked, 24, 24, sum_w, t_prime_sq, 97, 77, 3, true).unwrap();
        let fft = match_fft(&search, &masked, 24, 24, sum_w, t_prime_sq, 97, 77, 3).unwrap();
        let di = 30 * direct.width + 40;
        let fi = 30 * fft.width + 40;
        assert!(
            (direct.scores[di] - fft.scores[fi]).abs() < 0.02,
            "direct={} fft={}",
            direct.scores[di],
            fft.scores[fi]
        );
        assert!(direct.scores[di] > 0.99);
    }
}
