use crate::image::ImageBuf;
use rayon::prelude::*;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use thiserror::Error;

/// OpenCV `cv::TemplateMatchModes` (methods 0–5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatchMethod {
    Sqdiff = 0,
    SqdiffNormed = 1,
    Ccorr = 2,
    CcorrNormed = 3,
    Ccoeff = 4,
    #[default]
    CcoeffNormed = 5,
}

impl MatchMethod {
    /// `false` for `SQDIFF` / `SQDIFF_NORMED` (lower score is better).
    #[inline]
    pub fn higher_is_better(self) -> bool {
        !matches!(self, Self::Sqdiff | Self::SqdiffNormed)
    }

    #[inline]
    pub fn is_normed(self) -> bool {
        matches!(
            self,
            Self::SqdiffNormed | Self::CcorrNormed | Self::CcoeffNormed
        )
    }

    #[inline]
    fn is_ccoeff_family(self) -> bool {
        matches!(self, Self::Ccoeff | Self::CcoeffNormed)
    }

    pub fn all() -> [Self; 6] {
        [
            Self::Sqdiff,
            Self::SqdiffNormed,
            Self::Ccorr,
            Self::CcorrNormed,
            Self::Ccoeff,
            Self::CcoeffNormed,
        ]
    }
}

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

/// OpenCV `matchTemplate` with optional binary CV_8U mask.
///
/// Large unmasked searches use DFT cross-correlation (OpenCV `crossCorr` path).
/// Small / masked searches use a packed direct correlator with integral images.
pub fn match_template(
    search: &ImageBuf,
    template: &ImageBuf,
    mask: Option<&[u8]>,
    method: MatchMethod,
) -> Result<MatchMap, MatchError> {
    match_template_with_integrals(search, template, mask, method, None)
}

/// Like [`match_template`], but reuses precomputed search-image integrals
/// (built once per capture and shared across template variants).
pub fn match_template_with_integrals(
    search: &ImageBuf,
    template: &ImageBuf,
    mask: Option<&[u8]>,
    method: MatchMethod,
    integrals: Option<&SearchIntegrals>,
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

    let pack = build_packed_template(template, &mask_bits, ch, method);
    if pack.n <= 0.0 {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores: vec![0.0; out_w * out_h],
        });
    }

    // OpenCV: constant / empty-energy templates for some normed methods → all ones.
    if pack.t_energy <= f64::EPSILON
        && matches!(method, MatchMethod::CcoeffNormed | MatchMethod::SqdiffNormed)
    {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores: vec![1.0; out_w * out_h],
        });
    }

    let full_mask = mask_bits.iter().all(|&b| b);
    let direct_cost = (out_w as u64)
        .saturating_mul(out_h as u64)
        .saturating_mul(tw as u64)
        .saturating_mul(th as u64)
        .saturating_mul(ch as u64);

    if full_mask && direct_cost > FFT_DIRECT_COST_THRESHOLD {
        match_fft(
            search, &pack, tw, th, out_w, out_h, ch, method, integrals,
        )
    } else {
        match_direct(
            search, &pack, tw, th, out_w, out_h, ch, full_mask, method, integrals,
        )
    }
}

/// Packed masked template pixels for correlation.
///
/// `vals` are mean-subtracted for `CCOEFF*`, raw otherwise. `t_energy` is Σvals²
/// (primed energy for CCOEFF*, ΣT² for SQDIFF/CCORR).
struct PackedTemplate {
    xs: Vec<u16>,
    ys: Vec<u16>,
    vals: Vec<f64>,
    ch: usize,
    n: f64,
    t_energy: f64,
}

impl PackedTemplate {
    #[inline]
    fn len(&self) -> usize {
        self.xs.len()
    }

    #[inline]
    fn vals_at(&self, i: usize) -> &[f64] {
        let base = i * self.ch;
        &self.vals[base..base + self.ch]
    }
}

fn build_packed_template(
    template: &ImageBuf,
    mask_bits: &[bool],
    ch: usize,
    method: MatchMethod,
) -> PackedTemplate {
    let tw = template.width;
    let th = template.height;
    let mut sum_w = 0.0_f64;
    let mut t_mean = vec![0.0_f64; ch];
    for y in 0..th {
        for x in 0..tw {
            let li = y * tw + x;
            if !mask_bits[li] {
                continue;
            }
            sum_w += 1.0;
            let ti = template.pixel_offset(x, y);
            for (c, m) in t_mean.iter_mut().enumerate() {
                *m += template.data[ti + c] as f64;
            }
        }
    }
    if sum_w <= 0.0 {
        return PackedTemplate {
            xs: Vec::new(),
            ys: Vec::new(),
            vals: Vec::new(),
            ch,
            n: 0.0,
            t_energy: 0.0,
        };
    }

    let mean_subtract = method.is_ccoeff_family();
    if mean_subtract {
        for m in &mut t_mean {
            *m /= sum_w;
        }
    }

    let n = sum_w as usize;
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    let mut vals = Vec::with_capacity(n * ch);
    let mut t_energy = 0.0_f64;
    for y in 0..th {
        for x in 0..tw {
            let li = y * tw + x;
            if !mask_bits[li] {
                continue;
            }
            let ti = template.pixel_offset(x, y);
            xs.push(x as u16);
            ys.push(y as u16);
            for c in 0..ch {
                let v = if mean_subtract {
                    template.data[ti + c] as f64 - t_mean[c]
                } else {
                    template.data[ti + c] as f64
                };
                vals.push(v);
                t_energy += v * v;
            }
        }
    }
    PackedTemplate {
        xs,
        ys,
        vals,
        ch,
        n: sum_w,
        t_energy,
    }
}

#[allow(clippy::too_many_arguments)]
fn match_direct(
    search: &ImageBuf,
    pack: &PackedTemplate,
    tw: usize,
    th: usize,
    out_w: usize,
    out_h: usize,
    ch: usize,
    full_mask: bool,
    method: MatchMethod,
    precomputed: Option<&SearchIntegrals>,
) -> Result<MatchMap, MatchError> {
    let owned;
    let integrals = if full_mask {
        Some(if let Some(integ) = precomputed {
            integ
        } else {
            owned = build_integrals(search);
            &owned
        })
    } else {
        None
    };
    let search_w = search.width;
    let search_data = &search.data;
    let n = pack.n;
    let t_energy = pack.t_energy;

    let mut scores = vec![0.0_f32; out_w * out_h];
    scores
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(oy, row)| {
            for (ox, cell) in row.iter_mut().enumerate() {
                *cell = if let Some(integ) = integrals.as_ref() {
                    score_unmasked(
                        integ,
                        search_data,
                        search_w,
                        ch,
                        ox,
                        oy,
                        tw,
                        th,
                        n,
                        pack,
                        t_energy,
                        method,
                    )
                } else {
                    score_masked(search_data, search_w, ch, ox, oy, pack, n, t_energy, method)
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

/// DFT cross-correlation of packed template vs search, then method-specific finish.
#[allow(clippy::too_many_arguments)]
fn match_fft(
    search: &ImageBuf,
    pack: &PackedTemplate,
    tw: usize,
    th: usize,
    out_w: usize,
    out_h: usize,
    ch: usize,
    method: MatchMethod,
    precomputed: Option<&SearchIntegrals>,
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
            for i in 0..pack.len() {
                let x = pack.xs[i] as usize;
                let y = pack.ys[i] as usize;
                tmpl[y * dft_w + x] = Complex::new(pack.vals_at(i)[c] as f32, 0.0);
            }

            thread_local! {
                static PLANNER: std::cell::RefCell<FftPlanner<f32>> =
                    std::cell::RefCell::new(FftPlanner::new());
            }
            PLANNER.with(|p| {
                let mut planner = p.borrow_mut();
                fft2d_forward(&mut img, dft_w, dft_h, &mut planner);
                fft2d_forward(&mut tmpl, dft_w, dft_h, &mut planner);

                for i in 0..area {
                    img[i] *= tmpl[i].conj();
                }
                fft2d_inverse(&mut img, dft_w, dft_h, &mut planner);
            });

            let mut out = vec![0.0_f32; out_w * out_h];
            for y in 0..out_h {
                for x in 0..out_w {
                    out[y * out_w + x] = img[y * dft_w + x].re * scale;
                }
            }
            out
        })
        .collect();

    let mut corr = vec![0.0_f32; out_w * out_h];
    for ch_num in channel_numers {
        for (i, v) in ch_num.into_iter().enumerate() {
            corr[i] += v;
        }
    }

    if method == MatchMethod::Ccorr {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores: corr,
        });
    }

    let owned;
    let integ = if let Some(integ) = precomputed {
        integ
    } else {
        owned = build_integrals(search);
        &owned
    };
    let stride = integ.width + 1;
    let n = pack.n;
    let t_energy = pack.t_energy;
    let mut scores = vec![0.0_f32; out_w * out_h];
    scores
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(oy, row)| {
            for ox in 0..out_w {
                let mut i_sq = 0.0_f64;
                let mut i_prime_sq = 0.0_f64;
                for c in 0..ch {
                    let s = rect_sum(&integ.sum[c], stride, ox, oy, tw, th);
                    let sq = rect_sum(&integ.sumsq[c], stride, ox, oy, tw, th);
                    i_sq += sq;
                    i_prime_sq += sq - (s * s) / n;
                }
                let numer = corr[oy * out_w + ox] as f64;
                row[ox] = finish_score(method, numer, i_sq, i_prime_sq.max(0.0), t_energy);
            }
        });

    Ok(MatchMap {
        width: out_w,
        height: out_h,
        scores,
    })
}

/// Convert raw correlation + window stats into the OpenCV method score.
///
/// `i_sq` is Σ_c Σ I²; `i_prime_sq` is Σ_c (ΣI² − (ΣI)²/n) (CCOEFF window energy).
#[inline]
fn finish_score(
    method: MatchMethod,
    numer: f64,
    i_sq: f64,
    i_prime_sq: f64,
    t_energy: f64,
) -> f32 {
    match method {
        MatchMethod::Ccorr | MatchMethod::Ccoeff => numer as f32,
        MatchMethod::CcorrNormed => {
            let denom = (t_energy * i_sq.max(0.0)).sqrt();
            if denom > f64::EPSILON {
                (numer / denom) as f32
            } else {
                0.0
            }
        }
        MatchMethod::CcoeffNormed => {
            let denom = (t_energy * i_prime_sq.max(0.0)).sqrt();
            if denom > f64::EPSILON {
                (numer / denom) as f32
            } else {
                0.0
            }
        }
        MatchMethod::Sqdiff => (i_sq - 2.0 * numer + t_energy).max(0.0) as f32,
        MatchMethod::SqdiffNormed => {
            let sq = (i_sq - 2.0 * numer + t_energy).max(0.0);
            let denom = (t_energy * i_sq.max(0.0)).sqrt();
            if denom > f64::EPSILON {
                (sq / denom) as f32
            } else {
                1.0
            }
        }
    }
}

fn fft2d_forward(
    buf: &mut [Complex<f32>],
    width: usize,
    height: usize,
    planner: &mut FftPlanner<f32>,
) {
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

fn fft2d_inverse(
    buf: &mut [Complex<f32>],
    width: usize,
    height: usize,
    planner: &mut FftPlanner<f32>,
) {
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

/// Precomputed integral images for a search frame (shared across template variants).
pub struct SearchIntegrals {
    width: usize,
    sum: Vec<Vec<f64>>,
    sumsq: Vec<Vec<f64>>,
}

/// Build integral images once per blurred search capture.
pub fn prepare_search_integrals(img: &ImageBuf) -> SearchIntegrals {
    build_integrals(img)
}

fn build_integrals(img: &ImageBuf) -> SearchIntegrals {
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
    SearchIntegrals {
        width: w,
        sum,
        sumsq,
    }
}

#[inline]
fn rect_sum(integ: &[f64], stride: usize, x: usize, y: usize, tw: usize, th: usize) -> f64 {
    let x2 = x + tw;
    let y2 = y + th;
    integ[y2 * stride + x2] - integ[y * stride + x2] - integ[y2 * stride + x]
        + integ[y * stride + x]
}

#[allow(clippy::too_many_arguments)]
fn score_unmasked(
    integ: &SearchIntegrals,
    search_data: &[u8],
    search_w: usize,
    ch: usize,
    ox: usize,
    oy: usize,
    tw: usize,
    th: usize,
    n: f64,
    pack: &PackedTemplate,
    t_energy: f64,
    method: MatchMethod,
) -> f32 {
    let stride = integ.width + 1;
    let mut i_sq = 0.0_f64;
    let mut i_prime_sq = 0.0_f64;
    for c in 0..ch {
        let s = rect_sum(&integ.sum[c], stride, ox, oy, tw, th);
        let sq = rect_sum(&integ.sumsq[c], stride, ox, oy, tw, th);
        i_sq += sq;
        i_prime_sq += sq - (s * s) / n;
    }

    let mut numer = 0.0_f64;
    for i in 0..pack.len() {
        let x = pack.xs[i] as usize;
        let y = pack.ys[i] as usize;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        let vals = pack.vals_at(i);
        for c in 0..ch {
            numer += vals[c] * search_data[si + c] as f64;
        }
    }

    // CCOEFF: Σ(T'·I) == CCOEFF when ΣT'=0. No extra mean correction needed.
    finish_score(method, numer, i_sq, i_prime_sq.max(0.0), t_energy)
}

#[allow(clippy::too_many_arguments)]
fn score_masked(
    search_data: &[u8],
    search_w: usize,
    ch: usize,
    ox: usize,
    oy: usize,
    pack: &PackedTemplate,
    n: f64,
    t_energy: f64,
    method: MatchMethod,
) -> f32 {
    if method.is_ccoeff_family() {
        return score_masked_ccoeff(search_data, search_w, ch, ox, oy, pack, n, t_energy, method);
    }

    let mut numer = 0.0_f64;
    let mut i_sq = 0.0_f64;
    for i in 0..pack.len() {
        let x = pack.xs[i] as usize;
        let y = pack.ys[i] as usize;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        let vals = pack.vals_at(i);
        for c in 0..ch {
            let iv = search_data[si + c] as f64;
            numer += vals[c] * iv;
            i_sq += iv * iv;
        }
    }
    finish_score(method, numer, i_sq, 0.0, t_energy)
}

#[allow(clippy::too_many_arguments)]
fn score_masked_ccoeff(
    search_data: &[u8],
    search_w: usize,
    ch: usize,
    ox: usize,
    oy: usize,
    pack: &PackedTemplate,
    n: f64,
    t_energy: f64,
    method: MatchMethod,
) -> f32 {
    let mut i_sum_buf = [0.0_f64; 4];
    if ch > 4 {
        return score_masked_ccoeff_heap(search_data, search_w, ch, ox, oy, pack, n, t_energy, method);
    }
    let i_sum = &mut i_sum_buf[..ch];
    for i in 0..pack.len() {
        let x = pack.xs[i] as usize;
        let y = pack.ys[i] as usize;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        for c in 0..ch {
            i_sum[c] += search_data[si + c] as f64;
        }
    }
    for s in i_sum.iter_mut() {
        *s /= n;
    }

    let mut numer = 0.0_f64;
    let mut i_prime_sq = 0.0_f64;
    for i in 0..pack.len() {
        let x = pack.xs[i] as usize;
        let y = pack.ys[i] as usize;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        let vals = pack.vals_at(i);
        for c in 0..ch {
            let ip = search_data[si + c] as f64 - i_sum[c];
            numer += vals[c] * ip;
            i_prime_sq += ip * ip;
        }
    }

    match method {
        MatchMethod::Ccoeff => numer as f32,
        MatchMethod::CcoeffNormed => {
            let denom = (t_energy * i_prime_sq).sqrt();
            if denom > f64::EPSILON {
                (numer / denom) as f32
            } else {
                0.0
            }
        }
        _ => unreachable!("score_masked_ccoeff only for CCOEFF family"),
    }
}

#[allow(clippy::too_many_arguments)]
fn score_masked_ccoeff_heap(
    search_data: &[u8],
    search_w: usize,
    ch: usize,
    ox: usize,
    oy: usize,
    pack: &PackedTemplate,
    n: f64,
    t_energy: f64,
    method: MatchMethod,
) -> f32 {
    let mut i_sum = vec![0.0_f64; ch];
    for i in 0..pack.len() {
        let x = pack.xs[i] as usize;
        let y = pack.ys[i] as usize;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        for c in 0..ch {
            i_sum[c] += search_data[si + c] as f64;
        }
    }
    for s in i_sum.iter_mut() {
        *s /= n;
    }
    let mut numer = 0.0_f64;
    let mut i_prime_sq = 0.0_f64;
    for i in 0..pack.len() {
        let x = pack.xs[i] as usize;
        let y = pack.ys[i] as usize;
        let si = ((oy + y) * search_w + (ox + x)) * ch;
        let vals = pack.vals_at(i);
        for c in 0..ch {
            let ip = search_data[si + c] as f64 - i_sum[c];
            numer += vals[c] * ip;
            i_prime_sq += ip * ip;
        }
    }
    match method {
        MatchMethod::Ccoeff => numer as f32,
        MatchMethod::CcoeffNormed => {
            let denom = (t_energy * i_prime_sq).sqrt();
            if denom > f64::EPSILON {
                (numer / denom) as f32
            } else {
                0.0
            }
        }
        _ => unreachable!("score_masked_ccoeff_heap only for CCOEFF family"),
    }
}

fn prep_mask(tw: usize, th: usize, mask: Option<&[u8]>) -> Result<Vec<bool>, MatchError> {
    let area = tw * th;
    match mask {
        None => Ok(vec![true; area]),
        Some([]) => Ok(vec![true; area]),
        Some(m) if m.len() != area => Err(MatchError::MaskSize {
            got: m.len(),
            want: area,
        }),
        Some(m) => {
            if m.iter().all(|&v| v != 0) {
                Ok(vec![true; area])
            } else {
                Ok(m.iter().map(|&v| v != 0).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blur::{blur_image, search_blur_kernel};
    use crate::peaks::{find_peaks_for_method, DEFAULT_CLOSE_MATCHES_DISTANCE};
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

        let map = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        let matches = find_peaks_for_method(
            &map,
            0.95,
            DEFAULT_CLOSE_MATCHES_DISTANCE,
            MatchMethod::CcoeffNormed,
        );
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
    fn sqdiff_normed_finds_stamp_as_minimum() {
        let tmpl = patterned(8, 8);
        let mut search = gray(40, 40, 30);
        search.stamp(&tmpl, 12, 7);
        let map = match_template(&search, &tmpl, None, MatchMethod::SqdiffNormed).unwrap();
        let idx = 7 * map.width + 12;
        assert!(
            map.scores[idx] <= 0.05,
            "perfect stamp SQDIFF_NORMED ~0, got {}",
            map.scores[idx]
        );
        let matches = find_peaks_for_method(
            &map,
            0.1,
            DEFAULT_CLOSE_MATCHES_DISTANCE,
            MatchMethod::SqdiffNormed,
        );
        assert!(
            matches.iter().any(|p| p.x == 12 && p.y == 7),
            "expected min peak at (12,7), got {matches:?}"
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

        let map =
            match_template(&search, &masked_tmpl, Some(&mask), MatchMethod::CcoeffNormed).unwrap();
        let matches = find_peaks_for_method(
            &map,
            0.9,
            DEFAULT_CLOSE_MATCHES_DISTANCE,
            MatchMethod::CcoeffNormed,
        );
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
        let map = match_template(&search_b, &tmpl_b, None, MatchMethod::CcoeffNormed).unwrap();
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
        let map = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        let elapsed = t0.elapsed();
        let budget = perf_budget_secs(2.0);
        assert!(
            elapsed.as_secs_f64() < budget,
            "640x480 match took {elapsed:?} (budget {budget}s)"
        );
        let matches = find_peaks_for_method(
            &map,
            0.95,
            DEFAULT_CLOSE_MATCHES_DISTANCE,
            MatchMethod::CcoeffNormed,
        );
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
        let map = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        let elapsed = t0.elapsed();
        let budget = perf_budget_secs(5.0);
        assert!(
            elapsed.as_secs_f64() < budget,
            "1100x700 / 120x150 took {elapsed:?} (budget {budget}s) — FFT path broken?"
        );
        let matches = find_peaks_for_method(
            &map,
            0.95,
            DEFAULT_CLOSE_MATCHES_DISTANCE,
            MatchMethod::CcoeffNormed,
        );
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
        let pack = build_packed_template(&tmpl, &mask_bits, 3, MatchMethod::CcoeffNormed);
        let direct = match_direct(
            &search,
            &pack,
            24,
            24,
            97,
            77,
            3,
            true,
            MatchMethod::CcoeffNormed,
            None,
        )
        .unwrap();
        let fft = match_fft(
            &search,
            &pack,
            24,
            24,
            97,
            77,
            3,
            MatchMethod::CcoeffNormed,
            None,
        )
        .unwrap();
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

    #[test]
    fn mask_size_mismatch_errors() {
        let search = gray(20, 20, 10);
        let tmpl = patterned(5, 5);
        let bad_mask = vec![255u8; 3];
        let err =
            match_template(&search, &tmpl, Some(&bad_mask), MatchMethod::CcoeffNormed).unwrap_err();
        assert!(matches!(err, MatchError::MaskSize { .. }), "got {err:?}");
    }

    #[test]
    fn empty_image_errors() {
        let empty = ImageBuf::from_raw(0, 0, 3, vec![]);
        let tmpl = patterned(2, 2);
        let err = match_template(&empty, &tmpl, None, MatchMethod::CcoeffNormed).unwrap_err();
        assert!(matches!(err, MatchError::Empty), "got {err:?}");
    }

    #[test]
    fn template_too_large_errors() {
        let search = gray(4, 4, 10);
        let tmpl = patterned(8, 8);
        let err = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap_err();
        assert!(
            matches!(err, MatchError::TemplateTooLarge { .. }),
            "got {err:?}"
        );
    }
}
