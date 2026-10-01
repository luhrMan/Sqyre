//! SIMD planar correlation and pixel kernels.
//!
//! Search images are converted to planar `f32` so each template offset updates a
//! contiguous output row (`numer[ox] += t * plane[ox + …]`). Hot accumulate loops
//! use explicit pulp [`Simd`] / [`WithSimd`] saxpy-style kernels (not scalar loops
//! under bare [`Arch::dispatch`]). Row-level work uses Rayon.

use crate::image::ImageBuf;
use pulp::{Arch, Simd, WithSimd};
use rayon::prelude::*;

/// Cached `pulp` ISA dispatch — `Arch::new()` is process-stable; avoid re-detecting
/// on every hot row / accumulate call. Compatible with Phase 0 Arch TLS (QW-1).
#[inline]
pub(crate) fn pulp_arch() -> Arch {
    thread_local! {
        static ARCH: Arch = Arch::new();
    }
    ARCH.with(|a| *a)
}

/// Planar `f32` search image: channel `c` occupies `[c * plane .. (c+1) * plane)`
/// with `plane = width * height`, row-major within each plane.
#[derive(Clone, Debug)]
pub struct PlanarF32 {
    pub data: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub channels: usize,
}

impl PlanarF32 {
    #[inline]
    pub fn plane_len(&self) -> usize {
        self.width * self.height
    }

    /// Interleaved `u8` RGB/gray → planar `f32` (rows in parallel).
    pub fn from_interleaved(img: &ImageBuf) -> Self {
        let w = img.width;
        let h = img.height;
        let ch = img.channels;
        let plane = w * h;
        let mut data = vec![0.0_f32; plane * ch];
        let data_addr = data.as_mut_ptr() as usize;
        let src = img.data.as_slice();
        (0..h).into_par_iter().for_each(|y| {
            let arch = pulp_arch();
            arch.dispatch(|| {
                for x in 0..w {
                    let pi = (y * w + x) * ch;
                    let gi = y * w + x;
                    for c in 0..ch {
                        // SAFETY: each row `y` writes a disjoint `gi` range within
                        // every plane; no two threads share an index.
                        let ptr = data_addr as *mut f32;
                        unsafe {
                            *ptr.add(c * plane + gi) = src[pi + c] as f32;
                        }
                    }
                }
            });
        });
        Self {
            data,
            width: w,
            height: h,
            channels: ch,
        }
    }
}

/// Sparse (or dense-full) template samples for planar correlation.
#[derive(Clone, Debug)]
pub struct SparseTemplate {
    pub xs: Vec<u16>,
    pub ys: Vec<u16>,
    /// Interleaved channel values, length `xs.len() * channels`.
    pub vals: Vec<f32>,
    pub channels: usize,
}

impl SparseTemplate {
    pub fn from_packed(vals_f64: &[f64], xs: &[u16], ys: &[u16], ch: usize) -> Self {
        debug_assert_eq!(xs.len(), ys.len());
        debug_assert_eq!(vals_f64.len(), xs.len() * ch);
        Self {
            xs: xs.to_vec(),
            ys: ys.to_vec(),
            vals: vals_f64.iter().map(|&v| v as f32).collect(),
            channels: ch,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.xs.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.xs.is_empty()
    }
}

/// `y[i] += a * x[i]` over equal-length slices (explicit SIMD lanes + scalar tail).
#[inline(always)]
fn saxpy_row<S: Simd>(simd: S, a: f32, x: &[f32], y: &mut [f32]) {
    debug_assert_eq!(x.len(), y.len());
    let (x_head, x_tail) = S::as_simd_f32s(x);
    let (y_head, y_tail) = S::as_mut_simd_f32s(y);
    let av = simd.splat_f32s(a);
    for (xv, yv) in x_head.iter().zip(y_head.iter_mut()) {
        *yv = simd.mul_add_f32s(av, *xv, *yv);
    }
    for (xv, yv) in x_tail.iter().zip(y_tail.iter_mut()) {
        *yv += a * *xv;
    }
}

/// `y[i] += x[i] * x[i]`.
#[inline(always)]
fn accumulate_sq_row<S: Simd>(simd: S, x: &[f32], y: &mut [f32]) {
    debug_assert_eq!(x.len(), y.len());
    let (x_head, x_tail) = S::as_simd_f32s(x);
    let (y_head, y_tail) = S::as_mut_simd_f32s(y);
    for (xv, yv) in x_head.iter().zip(y_head.iter_mut()) {
        *yv = simd.mul_add_f32s(*xv, *xv, *yv);
    }
    for (xv, yv) in x_tail.iter().zip(y_tail.iter_mut()) {
        *yv += *xv * *xv;
    }
}

/// `y[i] += x[i]`.
#[inline(always)]
fn add_assign_row<S: Simd>(simd: S, x: &[f32], y: &mut [f32]) {
    debug_assert_eq!(x.len(), y.len());
    let (x_head, x_tail) = S::as_simd_f32s(x);
    let (y_head, y_tail) = S::as_mut_simd_f32s(y);
    for (xv, yv) in x_head.iter().zip(y_head.iter_mut()) {
        *yv = simd.add_f32s(*xv, *yv);
    }
    for (xv, yv) in x_tail.iter().zip(y_tail.iter_mut()) {
        *yv += *xv;
    }
}

struct AccumulateCorr<'a> {
    planar: &'a PlanarF32,
    tmpl: &'a SparseTemplate,
    oy: usize,
    numer: &'a mut [f32],
}

impl WithSimd for AccumulateCorr<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let out_w = self.numer.len();
        let search_w = self.planar.width;
        let ch = self.planar.channels;
        let plane = self.planar.plane_len();
        for i in 0..self.tmpl.len() {
            let tx = self.tmpl.xs[i] as usize;
            let ty = self.tmpl.ys[i] as usize;
            let row_y = self.oy + ty;
            let t_base = i * ch;
            for c in 0..ch {
                let tv = self.tmpl.vals[t_base + c];
                let start = c * plane + row_y * search_w + tx;
                let slice = &self.planar.data[start..start + out_w];
                saxpy_row(simd, tv, slice, self.numer);
            }
        }
    }
}

struct AccumulateSumSq<'a> {
    planar: &'a PlanarF32,
    tmpl: &'a SparseTemplate,
    oy: usize,
    sum_sq: &'a mut [f32],
}

impl WithSimd for AccumulateSumSq<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let out_w = self.sum_sq.len();
        let search_w = self.planar.width;
        let ch = self.planar.channels;
        let plane = self.planar.plane_len();
        for i in 0..self.tmpl.len() {
            let tx = self.tmpl.xs[i] as usize;
            let ty = self.tmpl.ys[i] as usize;
            let row_y = self.oy + ty;
            for c in 0..ch {
                let start = c * plane + row_y * search_w + tx;
                let slice = &self.planar.data[start..start + out_w];
                accumulate_sq_row(simd, slice, self.sum_sq);
            }
        }
    }
}

struct AccumulateChannelSums<'a> {
    planar: &'a PlanarF32,
    tmpl: &'a SparseTemplate,
    oy: usize,
    sums: &'a mut [Vec<f32>],
}

impl WithSimd for AccumulateChannelSums<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ch = self.planar.channels;
        let out_w = self.sums[0].len();
        let search_w = self.planar.width;
        let plane = self.planar.plane_len();
        for i in 0..self.tmpl.len() {
            let tx = self.tmpl.xs[i] as usize;
            let ty = self.tmpl.ys[i] as usize;
            let row_y = self.oy + ty;
            for (c, acc) in self.sums.iter_mut().enumerate().take(ch) {
                let start = c * plane + row_y * search_w + tx;
                let slice = &self.planar.data[start..start + out_w];
                add_assign_row(simd, slice, acc);
            }
        }
    }
}

/// `numer[ox] += Σ t·I` over sparse template offsets at output row `oy`.
pub fn accumulate_corr_row(
    planar: &PlanarF32,
    tmpl: &SparseTemplate,
    oy: usize,
    numer: &mut [f32],
) {
    let ch = planar.channels;
    debug_assert_eq!(tmpl.channels, ch);

    numer.fill(0.0);
    pulp_arch().dispatch(AccumulateCorr {
        planar,
        tmpl,
        oy,
        numer,
    });
}

/// `sum_sq[ox] += Σ I²` over sparse template offsets (all channels).
pub fn accumulate_sum_sq_row(
    planar: &PlanarF32,
    tmpl: &SparseTemplate,
    oy: usize,
    sum_sq: &mut [f32],
) {
    sum_sq.fill(0.0);
    pulp_arch().dispatch(AccumulateSumSq {
        planar,
        tmpl,
        oy,
        sum_sq,
    });
}

/// Per-channel `sums[c][ox] += Σ I` over sparse template offsets.
pub fn accumulate_channel_sums_row(
    planar: &PlanarF32,
    tmpl: &SparseTemplate,
    oy: usize,
    sums: &mut [Vec<f32>],
) {
    let ch = planar.channels;
    debug_assert_eq!(sums.len(), ch);

    for s in sums.iter_mut() {
        s.fill(0.0);
    }
    pulp_arch().dispatch(AccumulateChannelSums {
        planar,
        tmpl,
        oy,
        sums,
    });
}

/// Pointwise `img[i] *= tmpl[i].conj()` (FFT correlator).
pub fn complex_mul_conj(
    img: &mut [rustfft::num_complex::Complex<f32>],
    tmpl: &[rustfft::num_complex::Complex<f32>],
) {
    debug_assert_eq!(img.len(), tmpl.len());
    pulp_arch().dispatch(ComplexMulConj { img, tmpl });
}

struct ComplexMulConj<'a> {
    img: &'a mut [rustfft::num_complex::Complex<f32>],
    tmpl: &'a [rustfft::num_complex::Complex<f32>],
}

impl WithSimd for ComplexMulConj<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        // pulp::c32 == num_complex::Complex<f32> (same crate instance as rustfft).
        let img: &mut [pulp::c32] = self.img;
        let tmpl: &[pulp::c32] = self.tmpl;
        let (img_head, img_tail) = S::as_mut_simd_c32s(img);
        let (tmpl_head, tmpl_tail) = S::as_simd_c32s(tmpl);
        for (a, b) in img_head.iter_mut().zip(tmpl_head.iter()) {
            // a * conj(b)
            *a = simd.mul_c32s(*a, simd.conj_c32s(*b));
        }
        for (a, b) in img_tail.iter_mut().zip(tmpl_tail.iter()) {
            let ar = a.re;
            let ai = a.im;
            a.re = ar * b.re + ai * b.im;
            a.im = ai * b.re - ar * b.im;
        }
    }
}

/// Write `out[y * out_w + x] = img[y * dft_w + x].re * scale` for the top-left
/// `out_w × out_h` window of a DFT buffer (FFT correlator finish).
pub fn extract_scaled_re(
    img: &[rustfft::num_complex::Complex<f32>],
    dft_w: usize,
    out_w: usize,
    out_h: usize,
    scale: f32,
    out: &mut [f32],
) {
    debug_assert_eq!(out.len(), out_w * out_h);
    debug_assert!(dft_w >= out_w);
    pulp_arch().dispatch(ExtractScaledRe {
        img,
        dft_w,
        out_w,
        out_h,
        scale,
        out,
    });
}

struct ExtractScaledRe<'a> {
    img: &'a [rustfft::num_complex::Complex<f32>],
    dft_w: usize,
    out_w: usize,
    out_h: usize,
    scale: f32,
    out: &'a mut [f32],
}

impl WithSimd for ExtractScaledRe<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let scale_v = simd.splat_f32s(self.scale);
        let lanes = S::F32_LANES;
        // AVX-512 = 16 f32 lanes; keep a stack buffer for gather+store.
        debug_assert!(lanes <= 16);
        for y in 0..self.out_h {
            let row = &self.img[y * self.dft_w..y * self.dft_w + self.out_w];
            let dst = &mut self.out[y * self.out_w..(y + 1) * self.out_w];
            let mut x = 0;
            while x + lanes <= self.out_w {
                let mut re = [0.0_f32; 16];
                for lane in 0..lanes {
                    re[lane] = row[x + lane].re;
                }
                let (re_head, _) = S::as_simd_f32s(&re[..lanes]);
                let mut scaled = [0.0_f32; 16];
                {
                    let (sh, _) = S::as_mut_simd_f32s(&mut scaled[..lanes]);
                    sh[0] = simd.mul_f32s(scale_v, re_head[0]);
                }
                dst[x..x + lanes].copy_from_slice(&scaled[..lanes]);
                x += lanes;
            }
            for (d, c) in dst[x..].iter_mut().zip(row[x..].iter()) {
                *d = c.re * self.scale;
            }
        }
    }
}

/// Rec.601 RGB→gray under pulp dispatch.
pub fn map_rgb_to_gray_u8(rgb: &[u8], gray: &mut [u8]) {
    debug_assert_eq!(rgb.len(), gray.len() * 3);
    pulp_arch().dispatch(MapRgbToGray { rgb, gray });
}

struct MapRgbToGray<'a> {
    rgb: &'a [u8],
    gray: &'a mut [u8],
}

impl WithSimd for MapRgbToGray<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let wr = simd.splat_f32s(0.299);
        let wg = simd.splat_f32s(0.587);
        let wb = simd.splat_f32s(0.114);
        let lanes = S::F32_LANES;
        debug_assert!(lanes <= 16);
        let n = self.gray.len();
        let mut i = 0;
        while i + lanes <= n {
            let mut r = [0.0_f32; 16];
            let mut g = [0.0_f32; 16];
            let mut b = [0.0_f32; 16];
            for lane in 0..lanes {
                let o = (i + lane) * 3;
                r[lane] = self.rgb[o] as f32;
                g[lane] = self.rgb[o + 1] as f32;
                b[lane] = self.rgb[o + 2] as f32;
            }
            let (rh, _) = S::as_simd_f32s(&r[..lanes]);
            let (gh, _) = S::as_simd_f32s(&g[..lanes]);
            let (bh, _) = S::as_simd_f32s(&b[..lanes]);
            let y = simd.mul_add_f32s(
                wr,
                rh[0],
                simd.mul_add_f32s(wg, gh[0], simd.mul_f32s(wb, bh[0])),
            );
            let mut yf = [0.0_f32; 16];
            {
                let (yh, _) = S::as_mut_simd_f32s(&mut yf[..lanes]);
                yh[0] = y;
            }
            for lane in 0..lanes {
                self.gray[i + lane] = yf[lane].round() as u8;
            }
            i += lanes;
        }
        for (dst, chunk) in self.gray[i..]
            .iter_mut()
            .zip(self.rgb[i * 3..].chunks_exact(3))
        {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            *dst = (0.299 * r + 0.587 * g + 0.114 * b).round() as u8;
        }
    }
}

/// Threshold a gray buffer in place under pulp dispatch.
pub fn threshold_gray_in_place(data: &mut [u8], thresh: u8, invert: bool) {
    pulp_arch().dispatch(ThresholdGray {
        data,
        thresh,
        invert,
    });
}

struct ThresholdGray<'a> {
    data: &'a mut [u8],
    thresh: u8,
    invert: bool,
}

impl WithSimd for ThresholdGray<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        // Exact truth table; tight loop under WithSimd for ISA-targeted auto-vec.
        let thresh = self.thresh;
        let invert = self.invert;
        for p in self.data.iter_mut() {
            let above = *p >= thresh;
            *p = match (above, invert) {
                (true, false) | (false, true) => 255,
                (false, false) | (true, true) => 0,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageBuf;

    fn naive_corr_row(
        planar: &PlanarF32,
        tmpl: &SparseTemplate,
        oy: usize,
        out_w: usize,
    ) -> Vec<f32> {
        let mut numer = vec![0.0_f32; out_w];
        let search_w = planar.width;
        let ch = planar.channels;
        let plane = planar.plane_len();
        for i in 0..tmpl.len() {
            let tx = tmpl.xs[i] as usize;
            let ty = tmpl.ys[i] as usize;
            let row_y = oy + ty;
            let t_base = i * ch;
            for c in 0..ch {
                let tv = tmpl.vals[t_base + c];
                let start = c * plane + row_y * search_w + tx;
                for (ox, n) in numer.iter_mut().enumerate() {
                    *n += tv * planar.data[start + ox];
                }
            }
        }
        numer
    }

    fn naive_sum_sq_row(
        planar: &PlanarF32,
        tmpl: &SparseTemplate,
        oy: usize,
        out_w: usize,
    ) -> Vec<f32> {
        let mut sum_sq = vec![0.0_f32; out_w];
        let search_w = planar.width;
        let ch = planar.channels;
        let plane = planar.plane_len();
        for i in 0..tmpl.len() {
            let tx = tmpl.xs[i] as usize;
            let ty = tmpl.ys[i] as usize;
            let row_y = oy + ty;
            for c in 0..ch {
                let start = c * plane + row_y * search_w + tx;
                for (ox, acc) in sum_sq.iter_mut().enumerate() {
                    let s = planar.data[start + ox];
                    *acc += s * s;
                }
            }
        }
        sum_sq
    }

    fn naive_channel_sums_row(
        planar: &PlanarF32,
        tmpl: &SparseTemplate,
        oy: usize,
        out_w: usize,
    ) -> Vec<Vec<f32>> {
        let ch = planar.channels;
        let mut sums = vec![vec![0.0_f32; out_w]; ch];
        let search_w = planar.width;
        let plane = planar.plane_len();
        for i in 0..tmpl.len() {
            let tx = tmpl.xs[i] as usize;
            let ty = tmpl.ys[i] as usize;
            let row_y = oy + ty;
            for (c, acc) in sums.iter_mut().enumerate() {
                let start = c * plane + row_y * search_w + tx;
                for (ox, a) in acc.iter_mut().enumerate() {
                    *a += planar.data[start + ox];
                }
            }
        }
        sums
    }

    fn assert_f32_close(got: &[f32], expect: &[f32], tol: f32) {
        assert_eq!(got.len(), expect.len());
        for (i, (a, b)) in got.iter().zip(expect.iter()).enumerate() {
            assert!((a - b).abs() < tol, "index {i}: {a} vs {b} (tol {tol})");
        }
    }

    #[test]
    fn accumulate_corr_row_matches_naive() {
        let mut img = ImageBuf::new(32, 24, 3, 0);
        for (i, b) in img.data.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        let planar = PlanarF32::from_interleaved(&img);
        let tmpl = SparseTemplate {
            xs: vec![0, 2, 5],
            ys: vec![0, 1, 2],
            vals: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
            channels: 3,
        };
        let out_w = planar.width - 5;
        let mut simd = vec![0.0_f32; out_w];
        accumulate_corr_row(&planar, &tmpl, 3, &mut simd);
        let naive = naive_corr_row(&planar, &tmpl, 3, out_w);
        assert_f32_close(&simd, &naive, 1e-3);
    }

    #[test]
    fn accumulate_sum_sq_row_matches_naive() {
        let mut img = ImageBuf::new(28, 20, 1, 0);
        for (i, b) in img.data.iter_mut().enumerate() {
            *b = ((i * 17) % 200) as u8;
        }
        let planar = PlanarF32::from_interleaved(&img);
        let tmpl = SparseTemplate {
            xs: vec![0, 1, 3],
            ys: vec![0, 0, 1],
            vals: vec![1.0, 1.0, 1.0],
            channels: 1,
        };
        let out_w = planar.width - 3;
        let mut simd = vec![0.0_f32; out_w];
        accumulate_sum_sq_row(&planar, &tmpl, 2, &mut simd);
        let naive = naive_sum_sq_row(&planar, &tmpl, 2, out_w);
        assert_f32_close(&simd, &naive, 1e-3);
    }

    #[test]
    fn accumulate_channel_sums_row_matches_naive() {
        let mut img = ImageBuf::new(40, 16, 3, 0);
        for (i, b) in img.data.iter_mut().enumerate() {
            *b = ((i * 13) % 240) as u8;
        }
        let planar = PlanarF32::from_interleaved(&img);
        let tmpl = SparseTemplate {
            xs: vec![0, 1, 4],
            ys: vec![0, 2, 1],
            vals: vec![1.0; 9],
            channels: 3,
        };
        let out_w = planar.width - 4;
        let mut sums = vec![vec![0.0_f32; out_w]; 3];
        accumulate_channel_sums_row(&planar, &tmpl, 1, &mut sums);
        let naive = naive_channel_sums_row(&planar, &tmpl, 1, out_w);
        for (got, expect) in sums.iter().zip(naive.iter()) {
            assert_f32_close(got, expect, 1e-3);
        }
    }

    /// Wide rows force multi-lane SIMD heads; odd widths exercise scalar tails.
    #[test]
    fn accumulate_corr_parity_wide_and_odd_widths() {
        for &w in &[17usize, 63, 64, 65, 128, 257] {
            let h = 48usize;
            let mut img = ImageBuf::new(w, h, 3, 0);
            for (i, b) in img.data.iter_mut().enumerate() {
                *b = ((i * 31) % 255) as u8;
            }
            let planar = PlanarF32::from_interleaved(&img);
            let tmpl = SparseTemplate {
                xs: vec![0, 3, 7, 11],
                ys: vec![0, 1, 2, 3],
                vals: (1..=12).map(|v| v as f32 * 0.5).collect(),
                channels: 3,
            };
            let out_w = w - 11;
            let mut simd = vec![0.0_f32; out_w];
            accumulate_corr_row(&planar, &tmpl, 5, &mut simd);
            let naive = naive_corr_row(&planar, &tmpl, 5, out_w);
            // FMA vs separate mul+add can drift slightly; keep tight but realistic.
            assert_f32_close(&simd, &naive, 1e-2);
        }
    }

    #[test]
    fn rgb_to_gray_and_threshold_smoke() {
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 10, 10, 10];
        let mut gray = [0u8; 4];
        map_rgb_to_gray_u8(&rgb, &mut gray);
        assert!(gray[0] > 50);
        threshold_gray_in_place(&mut gray, 128, false);
        assert!(gray.iter().all(|&p| p == 0 || p == 255));
    }

    #[test]
    fn rgb_to_gray_matches_scalar_rec601() {
        let mut rgb = Vec::with_capacity(64 * 3);
        for i in 0..64 {
            rgb.push((i * 3) as u8);
            rgb.push((i * 5) as u8);
            rgb.push((i * 7) as u8);
        }
        let mut expect = vec![0u8; 64];
        for (dst, chunk) in expect.iter_mut().zip(rgb.chunks_exact(3)) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            *dst = (0.299 * r + 0.587 * g + 0.114 * b).round() as u8;
        }
        let mut got = vec![0u8; 64];
        map_rgb_to_gray_u8(&rgb, &mut got);
        assert_eq!(got, expect);
    }

    #[test]
    fn threshold_matches_truth_table() {
        let mut data = (0u8..=255).collect::<Vec<_>>();
        let mut expect = data.clone();
        for p in &mut expect {
            *p = if *p >= 128 { 255 } else { 0 };
        }
        threshold_gray_in_place(&mut data, 128, false);
        assert_eq!(data, expect);

        let mut data = (0u8..=255).collect::<Vec<_>>();
        let mut expect = data.clone();
        for p in &mut expect {
            *p = if *p >= 64 { 0 } else { 255 };
        }
        threshold_gray_in_place(&mut data, 64, true);
        assert_eq!(data, expect);
    }

    #[test]
    fn complex_mul_conj_matches_scalar() {
        use rustfft::num_complex::Complex;
        let mut img: Vec<Complex<f32>> = (0..257)
            .map(|i| Complex::new(i as f32 * 0.1, (i as f32 * 0.07) - 3.0))
            .collect();
        let tmpl: Vec<Complex<f32>> = (0..257)
            .map(|i| Complex::new((i as f32 * 0.03) + 1.0, i as f32 * -0.05))
            .collect();
        let mut expect = img.clone();
        for (a, b) in expect.iter_mut().zip(tmpl.iter()) {
            *a *= b.conj();
        }
        complex_mul_conj(&mut img, &tmpl);
        for (i, (a, b)) in img.iter().zip(expect.iter()).enumerate() {
            assert!(
                (a.re - b.re).abs() < 1e-4 && (a.im - b.im).abs() < 1e-4,
                "index {i}: {a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn extract_scaled_re_matches_scalar() {
        use rustfft::num_complex::Complex;
        let dft_w = 40;
        let dft_h = 24;
        let out_w = 33;
        let out_h = 17;
        let scale = 1.0 / (dft_w * dft_h) as f32;
        let img: Vec<Complex<f32>> = (0..dft_w * dft_h)
            .map(|i| Complex::new((i % 97) as f32 * 0.25, (i % 53) as f32 * -0.1))
            .collect();
        let mut expect = vec![0.0_f32; out_w * out_h];
        for y in 0..out_h {
            for x in 0..out_w {
                expect[y * out_w + x] = img[y * dft_w + x].re * scale;
            }
        }
        let mut got = vec![0.0_f32; out_w * out_h];
        extract_scaled_re(&img, dft_w, out_w, out_h, scale, &mut got);
        assert_f32_close(&got, &expect, 1e-5);
    }
}
