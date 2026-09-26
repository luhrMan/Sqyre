use crate::image::ImageBuf;
use parking_lot::Mutex;
use rayon::prelude::*;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use sqyre_domain::MatchMethod;
use std::collections::HashMap;
use std::sync::Arc;
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

/// True when this thread is already a Rayon pool worker.
///
/// Image Search runs an outer `par_iter` over targets/placements; each job then
/// matches templates. Nested row/channel `par_iter` would oversubscribe the pool,
/// so hot match paths stay serial when this returns true. Callers that are not
/// already on the pool (single-variant / unit tests) still get row parallelism.
#[inline]
fn on_rayon_worker() -> bool {
    rayon::current_thread_index().is_some()
}

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
    let mask_bits = prep_mask(template.width, template.height, mask)?;
    let prepared = prepare_template_from_mask_bits(template, &mask_bits, method);
    run_match(search, template, &prepared, None)
}

/// Template packing (masked/mean-subtracted pixels + sparse SIMD samples), built once
/// per (template, mask, method) and reused across repeated match attempts — see
/// [`prepare_template`].
pub struct PreparedTemplate {
    pack: PackedTemplate,
    sparse: crate::corr_simd::SparseTemplate,
    method: MatchMethod,
    full_mask: bool,
}

impl PreparedTemplate {
    /// Approximate heap bytes retained, for cache accounting.
    pub fn approx_bytes(&self) -> usize {
        let pack_bytes = self.pack.xs.len() * (2 + 2) + self.pack.vals.len() * 8;
        let sparse_bytes = self.sparse.xs.len() * (2 + 2) + self.sparse.vals.len() * 4;
        pack_bytes + sparse_bytes
    }
}

fn prepare_template_from_mask_bits(
    template: &ImageBuf,
    mask_bits: &[bool],
    method: MatchMethod,
) -> PreparedTemplate {
    let ch = template.channels;
    let pack = build_packed_template(template, mask_bits, ch, method);
    let sparse = crate::corr_simd::SparseTemplate::from_packed(&pack.vals, &pack.xs, &pack.ys, ch);
    let full_mask = mask_bits.iter().all(|&b| b);
    PreparedTemplate {
        pack,
        sparse,
        method,
        full_mask,
    }
}

/// Pack `template` (masked/mean-subtracted pixels + sparse SIMD samples) for `method`,
/// so repeated match attempts against the same template + mask can skip re-packing.
pub fn prepare_template(
    template: &ImageBuf,
    mask: Option<&[u8]>,
    method: MatchMethod,
) -> Result<PreparedTemplate, MatchError> {
    let mask_bits = prep_mask(template.width, template.height, mask)?;
    Ok(prepare_template_from_mask_bits(
        template, &mask_bits, method,
    ))
}

/// Like [`match_template`], but reuses a [`PreparedTemplate`] (skips re-packing the
/// template) and optionally a [`SearchPrep`] (skips re-deriving the search frame).
pub fn match_template_with_prepared(
    search: &ImageBuf,
    template: &ImageBuf,
    prepared: &PreparedTemplate,
    search_prep: Option<&SearchPrep>,
) -> Result<MatchMap, MatchError> {
    run_match(search, template, prepared, search_prep)
}

fn run_match(
    search: &ImageBuf,
    template: &ImageBuf,
    prepared: &PreparedTemplate,
    search_prep: Option<&SearchPrep>,
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

    let ch = search.channels;
    let tw = template.width;
    let th = template.height;
    let out_w = search.width - tw + 1;
    let out_h = search.height - th + 1;
    let pack = &prepared.pack;
    let method = prepared.method;

    if pack.n <= 0.0 {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores: vec![0.0; out_w * out_h],
        });
    }

    // OpenCV: constant / empty-energy templates for some normed methods → all ones.
    if pack.t_energy <= f64::EPSILON
        && matches!(
            method,
            MatchMethod::CcoeffNormed | MatchMethod::SqdiffNormed
        )
    {
        return Ok(MatchMap {
            width: out_w,
            height: out_h,
            scores: vec![1.0; out_w * out_h],
        });
    }

    let full_mask = prepared.full_mask;
    let direct_cost = (out_w as u64)
        .saturating_mul(out_h as u64)
        .saturating_mul(tw as u64)
        .saturating_mul(th as u64)
        .saturating_mul(ch as u64);

    if full_mask && direct_cost > FFT_DIRECT_COST_THRESHOLD {
        match_fft(search, pack, tw, th, out_w, out_h, ch, method, search_prep)
    } else {
        match_direct(
            search,
            pack,
            &prepared.sparse,
            tw,
            th,
            out_w,
            out_h,
            ch,
            full_mask,
            method,
            search_prep,
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
            for (c, mean) in t_mean.iter().enumerate() {
                let v = if mean_subtract {
                    template.data[ti + c] as f64 - mean
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

#[allow(clippy::too_many_arguments)] // match kernel: image geometry, packed template, method, and optional prep
fn match_direct(
    search: &ImageBuf,
    pack: &PackedTemplate,
    tmpl: &crate::corr_simd::SparseTemplate,
    tw: usize,
    th: usize,
    out_w: usize,
    out_h: usize,
    ch: usize,
    full_mask: bool,
    method: MatchMethod,
    search_prep: Option<&SearchPrep>,
) -> Result<MatchMap, MatchError> {
    let owned_planar;
    let planar = if let Some(prep) = search_prep {
        &prep.planar
    } else {
        owned_planar = crate::corr_simd::PlanarF32::from_interleaved(search);
        &owned_planar
    };
    let n = pack.n;
    let t_energy = pack.t_energy;
    let ccoeff = method.is_ccoeff_family();

    let owned;
    let integ = if full_mask {
        Some(if let Some(prep) = search_prep {
            &prep.integrals
        } else {
            owned = build_integrals(search);
            &owned
        })
    } else {
        None
    };

    let mut scores = vec![0.0_f32; out_w * out_h];
    let score_row = |oy: usize, row: &mut [f32]| {
        DIRECT_SCRATCH.with(|cell_scratch| {
            let scratch = &mut *cell_scratch.borrow_mut();
            scratch.reset(out_w, ch);
            let DirectScratch {
                numer,
                sum_sq,
                sums,
            } = scratch;
            crate::corr_simd::accumulate_corr_row(planar, tmpl, oy, numer);

            if let Some(integ) = integ {
                let stride = integ.width + 1;
                for (ox, cell) in row.iter_mut().enumerate() {
                    let mut i_sq = 0.0_f64;
                    let mut i_prime_sq = 0.0_f64;
                    for c in 0..ch {
                        let s = rect_sum(&integ.sum[c], stride, ox, oy, tw, th);
                        let sq = rect_sum(&integ.sumsq[c], stride, ox, oy, tw, th);
                        i_sq += sq;
                        i_prime_sq += sq - (s * s) / n;
                    }
                    *cell = finish_score(
                        method,
                        numer[ox] as f64,
                        i_sq,
                        i_prime_sq.max(0.0),
                        t_energy,
                    );
                }
            } else if ccoeff {
                // Masked CCOEFF: ΣT'=0 ⇒ numer = Σ T'·I. Energy: ΣI² − Σ_c (ΣI_c)²/n.
                crate::corr_simd::accumulate_sum_sq_row(planar, tmpl, oy, sum_sq);
                crate::corr_simd::accumulate_channel_sums_row(planar, tmpl, oy, sums);
                for (ox, cell) in row.iter_mut().enumerate() {
                    let mut i_prime = sum_sq[ox] as f64;
                    for channel_sum in sums.iter().take(ch) {
                        let s = channel_sum[ox] as f64;
                        i_prime -= (s * s) / n;
                    }
                    *cell = match method {
                        MatchMethod::Ccoeff => numer[ox],
                        MatchMethod::CcoeffNormed => {
                            let denom = (t_energy * i_prime.max(0.0)).sqrt();
                            if denom > f64::EPSILON {
                                (numer[ox] as f64 / denom) as f32
                            } else {
                                0.0
                            }
                        }
                        _ => unreachable!("ccoeff branch"),
                    };
                }
            } else {
                crate::corr_simd::accumulate_sum_sq_row(planar, tmpl, oy, sum_sq);
                for (ox, cell) in row.iter_mut().enumerate() {
                    *cell =
                        finish_score(method, numer[ox] as f64, sum_sq[ox] as f64, 0.0, t_energy);
                }
            }
        });
    };

    // Outer Image Search `par_iter` already owns the pool; stay serial there.
    if on_rayon_worker() {
        for (oy, row) in scores.chunks_mut(out_w).enumerate() {
            score_row(oy, row);
        }
    } else {
        scores
            .par_chunks_mut(out_w)
            .enumerate()
            .for_each(|(oy, row)| score_row(oy, row));
    }

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

/// Per-row accumulators for [`match_direct`], reused across rows on each rayon
/// worker instead of allocating `out_w`-sized buffers per output row.
#[derive(Default)]
struct DirectScratch {
    numer: Vec<f32>,
    sum_sq: Vec<f32>,
    /// One row of per-channel sums; only the masked CCOEFF path fills these.
    sums: Vec<Vec<f32>>,
}

impl DirectScratch {
    /// Zeroed buffers for one output row. `accumulate_*_row` adds into them, so
    /// they must start at zero.
    fn reset(&mut self, out_w: usize, ch: usize) {
        zeroed(&mut self.numer, out_w);
        zeroed(&mut self.sum_sq, out_w);
        self.sums.resize_with(ch, Vec::new);
        for s in &mut self.sums {
            zeroed(s, out_w);
        }
    }

    /// Drop capacity so Rayon worker TLS does not retain peak row widths.
    fn shrink(&mut self) {
        self.numer = Vec::new();
        self.sum_sq = Vec::new();
        self.sums = Vec::new();
    }
}

fn zeroed(buf: &mut Vec<f32>, len: usize) {
    buf.clear();
    buf.resize(len, 0.0);
}

thread_local! {
    static DIRECT_SCRATCH: std::cell::RefCell<DirectScratch> =
        std::cell::RefCell::new(DirectScratch::default());
}

/// Per-thread FFT planner plus correlation scratch.
///
/// Both spectrum buffers are sized `dft_w * dft_h`, which for a full-screen
/// search is several megabytes. Reusing them keeps every variant match on a
/// rayon worker from allocating and freeing that much per channel — but the
/// capacity ratchets to the largest DFT seen on that worker for the process
/// lifetime unless [`clear_match_scratch`] runs (after each macro).
struct FftScratch {
    planner: FftPlanner<f32>,
    /// Mutable copy of the search spectrum (consumed by the inverse transform).
    img: Vec<Complex<f32>>,
    /// Zero-padded template spectrum.
    tmpl: Vec<Complex<f32>>,
    /// Column gather buffer for [`fft2d_forward`] / [`fft2d_inverse`].
    col: Vec<Complex<f32>>,
}

impl FftScratch {
    fn new() -> Self {
        Self {
            planner: FftPlanner::new(),
            img: Vec::new(),
            tmpl: Vec::new(),
            col: Vec::new(),
        }
    }

    /// Release peak DFT / planner capacity held in this worker's TLS.
    fn shrink(&mut self) {
        self.img = Vec::new();
        self.tmpl = Vec::new();
        self.col = Vec::new();
        // Drop cached FFT plans (can be ~1 MiB+ of tables per worker).
        self.planner = FftPlanner::new();
    }
}

thread_local! {
    static FFT_SCRATCH: std::cell::RefCell<FftScratch> =
        std::cell::RefCell::new(FftScratch::new());
}

fn shrink_local_match_scratch() {
    DIRECT_SCRATCH.with(|c| c.borrow_mut().shrink());
    FFT_SCRATCH.with(|c| c.borrow_mut().shrink());
}

/// Release per-Rayon-worker match scratch (FFT spectra + direct-row buffers).
///
/// Call after a macro finishes (wired through [`sqyre_vision::clear_search_cache`])
/// so full-screen DFT buffers do not keep RSS ratcheted across runs. Safe to call
/// at any time; the next match reallocates as needed.
pub fn clear_match_scratch() {
    shrink_local_match_scratch();
    // Idle pool workers never hit the calling thread's TLS; broadcast reaches them.
    rayon::broadcast(|_| shrink_local_match_scratch());
}

/// Forward-FFT each channel of `search`, zero-padded to `dft_w`×`dft_h`.
///
/// When `parallel` is false, channels are transformed on the calling thread so
/// single-flight builders can run safely while other rayon workers wait on a
/// gate (nested `par_iter` would otherwise deadlock the pool), and so outer
/// Image Search `par_iter` jobs do not nest another channel-level pool.
fn forward_fft_search(search: &ImageBuf, dft_w: usize, dft_h: usize, parallel: bool) -> SearchFft {
    let ch = search.channels;
    let area = dft_w * dft_h;
    let build_channel = |c: usize| {
        let mut img = vec![Complex::new(0.0, 0.0); area];
        for y in 0..search.height {
            for x in 0..search.width {
                let v = search.data[(y * search.width + x) * ch + c] as f32;
                img[y * dft_w + x] = Complex::new(v, 0.0);
            }
        }
        FFT_SCRATCH.with(|s| {
            let scratch = &mut *s.borrow_mut();
            fft2d_forward(
                &mut img,
                dft_w,
                dft_h,
                &mut scratch.planner,
                &mut scratch.col,
            );
        });
        img
    };
    if parallel {
        (0..ch).into_par_iter().map(build_channel).collect()
    } else {
        (0..ch).map(build_channel).collect()
    }
}

/// DFT cross-correlation of packed template vs search, then method-specific finish.
#[allow(clippy::too_many_arguments)] // match kernel: image geometry, packed template, method, and optional prep
fn match_fft(
    search: &ImageBuf,
    pack: &PackedTemplate,
    tw: usize,
    th: usize,
    out_w: usize,
    out_h: usize,
    ch: usize,
    method: MatchMethod,
    search_prep: Option<&SearchPrep>,
) -> Result<MatchMap, MatchError> {
    let dft_w = optimal_dft_size(search.width + tw - 1);
    let dft_h = optimal_dft_size(search.height + th - 1);
    let area = dft_w * dft_h;
    let scale = 1.0_f32 / area as f32;

    // Nested under Image Search's outer `par_iter`: keep channel / row work serial.
    let parallel_inner = !on_rayon_worker();

    let owned_search_fft;
    let search_fft: &[Vec<Complex<f32>>] = if let Some(prep) = search_prep {
        owned_search_fft = prep.fft_for_size(search, dft_w, dft_h);
        owned_search_fft.as_ref()
    } else {
        owned_search_fft = Arc::new(forward_fft_search(search, dft_w, dft_h, parallel_inner));
        owned_search_fft.as_ref()
    };

    let channel_fft = |c: usize| {
        FFT_SCRATCH.with(|s| {
            let FftScratch {
                planner,
                img,
                tmpl,
                col,
            } = &mut *s.borrow_mut();

            // Overwritten wholesale, so no need to clear first.
            img.clear();
            img.extend_from_slice(&search_fft[c]);
            // Only sparse positions are written below; the rest must be zero.
            tmpl.clear();
            tmpl.resize(area, Complex::new(0.0, 0.0));
            for i in 0..pack.len() {
                let x = pack.xs[i] as usize;
                let y = pack.ys[i] as usize;
                tmpl[y * dft_w + x] = Complex::new(pack.vals_at(i)[c] as f32, 0.0);
            }

            fft2d_forward(tmpl, dft_w, dft_h, planner, col);
            crate::corr_simd::complex_mul_conj(img, tmpl);
            fft2d_inverse(img, dft_w, dft_h, planner, col);

            let mut out = vec![0.0_f32; out_w * out_h];
            let arch = crate::corr_simd::pulp_arch();
            arch.dispatch(|| {
                for y in 0..out_h {
                    for x in 0..out_w {
                        out[y * out_w + x] = img[y * dft_w + x].re * scale;
                    }
                }
            });
            out
        })
    };

    let channel_numers: Vec<Vec<f32>> = if parallel_inner {
        (0..ch).into_par_iter().map(channel_fft).collect()
    } else {
        (0..ch).map(channel_fft).collect()
    };

    let mut corr = vec![0.0_f32; out_w * out_h];
    if parallel_inner {
        for ch_num in channel_numers {
            corr.par_iter_mut()
                .zip(ch_num.into_par_iter())
                .for_each(|(dst, v)| *dst += v);
        }
    } else {
        for ch_num in channel_numers {
            for (dst, v) in corr.iter_mut().zip(ch_num) {
                *dst += v;
            }
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
    let integ = if let Some(prep) = search_prep {
        &prep.integrals
    } else {
        owned = build_integrals(search);
        &owned
    };
    let stride = integ.width + 1;
    let n = pack.n;
    let t_energy = pack.t_energy;
    let mut scores = vec![0.0_f32; out_w * out_h];
    let finish_row = |oy: usize, row: &mut [f32]| {
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
    };
    if parallel_inner {
        scores
            .par_chunks_mut(out_w)
            .enumerate()
            .for_each(|(oy, row)| finish_row(oy, row));
    } else {
        for (oy, row) in scores.chunks_mut(out_w).enumerate() {
            finish_row(oy, row);
        }
    }

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
fn finish_score(method: MatchMethod, numer: f64, i_sq: f64, i_prime_sq: f64, t_energy: f64) -> f32 {
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
    col: &mut Vec<Complex<f32>>,
) {
    let fft_row = planner.plan_fft_forward(width);
    for row in buf.chunks_exact_mut(width) {
        fft_row.process(row);
    }
    let fft_col = planner.plan_fft_forward(height);
    col.clear();
    col.resize(height, Complex::default());
    for x in 0..width {
        for y in 0..height {
            col[y] = buf[y * width + x];
        }
        fft_col.process(col);
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
    col: &mut Vec<Complex<f32>>,
) {
    let ifft_row = planner.plan_fft_inverse(width);
    for row in buf.chunks_exact_mut(width) {
        ifft_row.process(row);
    }
    let ifft_col = planner.plan_fft_inverse(height);
    col.clear();
    col.resize(height, Complex::default());
    for x in 0..width {
        for y in 0..height {
            col[y] = buf[y * width + x];
        }
        ifft_col.process(col);
        for y in 0..height {
            buf[y * width + x] = col[y];
        }
    }
}

/// Precomputed integral images for a search frame (shared across template variants).
struct SearchIntegrals {
    width: usize,
    sum: Vec<Vec<f64>>,
    sumsq: Vec<Vec<f64>>,
}

/// Per-capture search-frame prep, built once and shared across every template variant
/// matched against the same buffer: integral images (for CCOEFF*/normed finish) and
/// planar `f32` (for the direct SIMD correlator). FFT cross-correlations of the search
/// frame are cached lazily per padded DFT size, since different template sizes need
/// different padding.
pub struct SearchPrep {
    integrals: SearchIntegrals,
    planar: crate::corr_simd::PlanarF32,
    fft_cache: Mutex<HashMap<(usize, usize), Arc<SearchFft>>>,
    /// Single-flight gates so parallel variant matches do not stampede the same DFT.
    fft_inflight: FftInflightGates,
}

/// Forward-FFT'd search image, one plane per channel.
type SearchFft = Vec<Vec<Complex<f32>>>;
type FftInflightGates = Mutex<HashMap<(usize, usize), Arc<Mutex<()>>>>;

impl SearchPrep {
    fn fft_for_size(&self, search: &ImageBuf, dft_w: usize, dft_h: usize) -> Arc<SearchFft> {
        if let Some(hit) = self.fft_cache.lock().get(&(dft_w, dft_h)) {
            return Arc::clone(hit);
        }
        let gate = {
            let mut gates = self.fft_inflight.lock();
            Arc::clone(
                gates
                    .entry((dft_w, dft_h))
                    .or_insert_with(|| Arc::new(Mutex::new(()))),
            )
        };
        let _busy = gate.lock();
        if let Some(hit) = self.fft_cache.lock().get(&(dft_w, dft_h)) {
            self.fft_inflight.lock().remove(&(dft_w, dft_h));
            return Arc::clone(hit);
        }
        // Serial channel FFT: callers may be rayon workers waiting on this gate.
        let built = Arc::new(forward_fft_search(search, dft_w, dft_h, false));
        self.fft_cache
            .lock()
            .insert((dft_w, dft_h), Arc::clone(&built));
        self.fft_inflight.lock().remove(&(dft_w, dft_h));
        built
    }

    /// Precompute the search-frame FFT used for an unmasked template of size `tw`×`th`
    /// when the match would take the DFT path. Call from the main thread before
    /// parallel variant matching.
    pub fn warm_fft_for_template(&self, search: &ImageBuf, tw: usize, th: usize) {
        if tw == 0 || th == 0 || search.width < tw || search.height < th {
            return;
        }
        let out_w = search.width - tw + 1;
        let out_h = search.height - th + 1;
        let ch = search.channels.max(1);
        let direct_cost = (out_w as u64)
            .saturating_mul(out_h as u64)
            .saturating_mul(tw as u64)
            .saturating_mul(th as u64)
            .saturating_mul(ch as u64);
        if direct_cost <= FFT_DIRECT_COST_THRESHOLD {
            return;
        }
        let dft_w = optimal_dft_size(search.width + tw - 1);
        let dft_h = optimal_dft_size(search.height + th - 1);
        let _ = self.fft_for_size(search, dft_w, dft_h);
    }
}

/// Build the shared search-frame prep once per blurred capture, for reuse across
/// every template variant matched against it.
pub fn prepare_search(img: &ImageBuf) -> SearchPrep {
    SearchPrep {
        integrals: build_integrals(img),
        planar: crate::corr_simd::PlanarF32::from_interleaved(img),
        fft_cache: Mutex::new(HashMap::new()),
        fft_inflight: Mutex::new(HashMap::new()),
    }
}

fn build_integrals(img: &ImageBuf) -> SearchIntegrals {
    let w = img.width;
    let h = img.height;
    let ch = img.channels;
    let stride = w + 1;
    // Independent per channel — parallelize across channels.
    let mut planes: Vec<(Vec<f64>, Vec<f64>)> = (0..ch)
        .into_par_iter()
        .map(|c| {
            let mut sum = vec![0.0_f64; stride * (h + 1)];
            let mut sumsq = vec![0.0_f64; stride * (h + 1)];
            for y in 0..h {
                for x in 0..w {
                    let v = img.data[img.pixel_offset(x, y) + c] as f64;
                    let above = sum[y * stride + (x + 1)];
                    let left = sum[(y + 1) * stride + x];
                    let corner = sum[y * stride + x];
                    sum[(y + 1) * stride + (x + 1)] = above + left - corner + v;

                    let above_sq = sumsq[y * stride + (x + 1)];
                    let left_sq = sumsq[(y + 1) * stride + x];
                    let corner_sq = sumsq[y * stride + x];
                    sumsq[(y + 1) * stride + (x + 1)] = above_sq + left_sq - corner_sq + v * v;
                }
            }
            (sum, sumsq)
        })
        .collect();
    let mut sum = Vec::with_capacity(ch);
    let mut sumsq = Vec::with_capacity(ch);
    for (s, sq) in planes.drain(..) {
        sum.push(s);
        sumsq.push(sq);
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

        let map = match_template(
            &search,
            &masked_tmpl,
            Some(&mask),
            MatchMethod::CcoeffNormed,
        )
        .unwrap();
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
        let sparse =
            crate::corr_simd::SparseTemplate::from_packed(&pack.vals, &pack.xs, &pack.ys, 3);
        let direct = match_direct(
            &search,
            &pack,
            &sparse,
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

    #[test]
    fn clear_match_scratch_does_not_break_subsequent_fft() {
        // Force the FFT path (large search relative to template).
        let tmpl = patterned(16, 16);
        let mut search = gray(128, 128, 30);
        search.stamp(&tmpl, 40, 40);
        let map = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        assert!(!map.scores.is_empty());
        clear_match_scratch();
        let map2 = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        let peaks = find_peaks_for_method(
            &map2,
            0.9,
            DEFAULT_CLOSE_MATCHES_DISTANCE,
            MatchMethod::CcoeffNormed,
        );
        assert!(
            !peaks.is_empty(),
            "expected a hit after clearing TLS scratch"
        );
    }

    /// Post-run contract: clear is idempotent and Rayon workers stay usable.
    #[test]
    fn clear_match_scratch_twice_then_parallel_fft_ok() {
        let tmpl = patterned(16, 16);
        let mut search = gray(96, 96, 25);
        search.stamp(&tmpl, 20, 20);
        let _ = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        clear_match_scratch();
        clear_match_scratch();
        let maps: Vec<_> = (0..4)
            .into_par_iter()
            .map(|_| match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap())
            .collect();
        assert!(maps.iter().all(|m| !m.scores.is_empty()));
        clear_match_scratch();
        let again = match_template(&search, &tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        assert!(!again.scores.is_empty());
    }

    /// Nested under `par_iter`, match must stay serial on rows but scores must
    /// match the off-pool (row-parallel) path.
    #[test]
    fn nested_rayon_match_scores_match_off_pool() {
        let tmpl = patterned(12, 10);
        let mut search = gray(64, 48, 20);
        search.stamp(&tmpl, 10, 8);
        let prep = prepare_search(&search);
        let prepared = prepare_template(&tmpl, None, MatchMethod::CcoeffNormed).unwrap();
        let off_pool =
            match_template_with_prepared(&search, &tmpl, &prepared, Some(&prep)).unwrap();

        let on_pool: Vec<_> = (0..4)
            .into_par_iter()
            .map(|_| {
                assert!(
                    rayon::current_thread_index().is_some(),
                    "expected Rayon worker"
                );
                match_template_with_prepared(&search, &tmpl, &prepared, Some(&prep)).unwrap()
            })
            .collect();

        for map in &on_pool {
            assert_eq!(map.width, off_pool.width);
            assert_eq!(map.height, off_pool.height);
            assert_eq!(map.scores.len(), off_pool.scores.len());
            for (a, b) in map.scores.iter().zip(off_pool.scores.iter()) {
                assert!(
                    (a - b).abs() < 1e-5,
                    "score drift nested vs off-pool: {a} vs {b}"
                );
            }
        }
    }
}
