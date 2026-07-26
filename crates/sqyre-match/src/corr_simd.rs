//! SIMD-friendly planar correlation and pixel kernels.
//!
//! Search images are converted to planar `f32` so each template offset updates a
//! contiguous output row (`numer[ox] += t * plane[ox + …]`). Inner loops run under
//! [`pulp::Arch::dispatch`] for runtime AVX/NEON/etc.; row work uses Rayon.

use crate::image::ImageBuf;
use pulp::Arch;
use rayon::prelude::*;

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
            let arch = Arch::new();
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
}

/// `numer[ox] += Σ t·I` over sparse template offsets at output row `oy`.
pub fn accumulate_corr_row(
    planar: &PlanarF32,
    tmpl: &SparseTemplate,
    oy: usize,
    numer: &mut [f32],
) {
    let out_w = numer.len();
    let search_w = planar.width;
    let ch = planar.channels;
    let plane = planar.plane_len();
    debug_assert_eq!(tmpl.channels, ch);

    numer.fill(0.0);
    let arch = Arch::new();
    arch.dispatch(|| {
        for i in 0..tmpl.len() {
            let tx = tmpl.xs[i] as usize;
            let ty = tmpl.ys[i] as usize;
            let row_y = oy + ty;
            let t_base = i * ch;
            for c in 0..ch {
                let tv = tmpl.vals[t_base + c];
                let start = c * plane + row_y * search_w + tx;
                let slice = &planar.data[start..start + out_w];
                for (n, &s) in numer.iter_mut().zip(slice.iter()) {
                    *n += tv * s;
                }
            }
        }
    });
}

/// `sum_sq[ox] += Σ I²` over sparse template offsets (all channels).
pub fn accumulate_sum_sq_row(
    planar: &PlanarF32,
    tmpl: &SparseTemplate,
    oy: usize,
    sum_sq: &mut [f32],
) {
    let out_w = sum_sq.len();
    let search_w = planar.width;
    let ch = planar.channels;
    let plane = planar.plane_len();

    sum_sq.fill(0.0);
    let arch = Arch::new();
    arch.dispatch(|| {
        for i in 0..tmpl.len() {
            let tx = tmpl.xs[i] as usize;
            let ty = tmpl.ys[i] as usize;
            let row_y = oy + ty;
            for c in 0..ch {
                let start = c * plane + row_y * search_w + tx;
                let slice = &planar.data[start..start + out_w];
                for (acc, &s) in sum_sq.iter_mut().zip(slice.iter()) {
                    *acc += s * s;
                }
            }
        }
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
    let out_w = sums[0].len();
    let search_w = planar.width;
    let plane = planar.plane_len();
    debug_assert_eq!(sums.len(), ch);

    for s in sums.iter_mut() {
        s.fill(0.0);
    }
    let arch = Arch::new();
    arch.dispatch(|| {
        for i in 0..tmpl.len() {
            let tx = tmpl.xs[i] as usize;
            let ty = tmpl.ys[i] as usize;
            let row_y = oy + ty;
            for (c, acc) in sums.iter_mut().enumerate().take(ch) {
                let start = c * plane + row_y * search_w + tx;
                let slice = &planar.data[start..start + out_w];
                for (a, &s) in acc.iter_mut().zip(slice.iter()) {
                    *a += s;
                }
            }
        }
    });
}

/// Pointwise `img[i] *= tmpl[i].conj()` under architecture dispatch (FFT path).
pub fn complex_mul_conj(
    img: &mut [rustfft::num_complex::Complex<f32>],
    tmpl: &[rustfft::num_complex::Complex<f32>],
) {
    debug_assert_eq!(img.len(), tmpl.len());
    let arch = Arch::new();
    arch.dispatch(|| {
        for (a, b) in img.iter_mut().zip(tmpl.iter()) {
            *a *= b.conj();
        }
    });
}

/// Rec.601 RGB→gray under pulp dispatch.
pub fn map_rgb_to_gray_u8(rgb: &[u8], gray: &mut [u8]) {
    debug_assert_eq!(rgb.len(), gray.len() * 3);
    let arch = Arch::new();
    arch.dispatch(|| {
        for (dst, chunk) in gray.iter_mut().zip(rgb.chunks_exact(3)) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            *dst = (0.299 * r + 0.587 * g + 0.114 * b).round() as u8;
        }
    });
}

/// Threshold a gray buffer in place under pulp dispatch.
pub fn threshold_gray_in_place(data: &mut [u8], thresh: u8, invert: bool) {
    let arch = Arch::new();
    arch.dispatch(|| {
        for p in data.iter_mut() {
            let above = *p >= thresh;
            *p = match (above, invert) {
                (true, false) | (false, true) => 255,
                (false, false) | (true, true) => 0,
            };
        }
    });
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
        for (a, b) in simd.iter().zip(naive.iter()) {
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
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
        for (a, b) in simd.iter().zip(naive.iter()) {
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
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
}
