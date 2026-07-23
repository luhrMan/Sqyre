//! Port of OpenCV `modules/imgproc/test/test_templmatchmask.cpp` (binary-mask subset).
//!
//! Compares [`match_template`] against naive OpenCV formulas at a fixed ROI point,
//! and checks all-ones binary mask ≈ no mask.

use sqyre_match::{match_template, ImageBuf, MatchMethod};

const IMG_W: usize = 160;
const IMG_H: usize = 100;
const TEMPL_W: usize = 21;
const TEMPL_H: usize = 13;
const TEST_X: usize = 8;
const TEST_Y: usize = 9;

fn random_image(w: usize, h: usize, ch: usize, seed: &mut u64, max_val: u8) -> ImageBuf {
    let mut img = ImageBuf::new(w, h, ch, 0);
    for y in 0..h {
        for x in 0..w {
            let o = img.pixel_offset(x, y);
            for c in 0..ch {
                *seed ^= *seed << 13;
                *seed ^= *seed >> 7;
                *seed ^= *seed << 17;
                img.data[o + c] = (*seed % (max_val as u64 + 1)) as u8;
            }
        }
    }
    img
}

fn random_binary_mask(w: usize, h: usize, seed: &mut u64) -> Vec<u8> {
    let mut mask = vec![0u8; w * h];
    for v in &mut mask {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        // Mix of zeros and nonzeros (OpenCV CV_8U binary interpretation).
        *v = if (*seed).is_multiple_of(5) {
            0
        } else {
            ((*seed % 254) + 1) as u8
        };
    }
    // Ensure at least some active pixels.
    if mask.iter().all(|&m| m == 0) {
        mask[0] = 255;
    }
    mask
}

/// Naive OpenCV masked formula at `(TEST_X, TEST_Y)`.
fn naive_at(img: &ImageBuf, templ: &ImageBuf, mask: &[u8], method: MatchMethod) -> f64 {
    let ch = img.channels;
    let mut sum_m = 0.0_f64;
    let mut sum_tm = vec![0.0_f64; ch];
    let mut sum_im = vec![0.0_f64; ch];
    for ty in 0..TEMPL_H {
        for tx in 0..TEMPL_W {
            let mi = ty * TEMPL_W + tx;
            if mask[mi] == 0 {
                continue;
            }
            sum_m += 1.0;
            let ti = templ.pixel_offset(tx, ty);
            let ii = img.pixel_offset(TEST_X + tx, TEST_Y + ty);
            for c in 0..ch {
                sum_tm[c] += templ.data[ti + c] as f64;
                sum_im[c] += img.data[ii + c] as f64;
            }
        }
    }
    if sum_m <= 0.0 {
        return 0.0;
    }

    let inv_m = 1.0 / sum_m;
    let mut val = 0.0_f64;
    let mut templ_energy = 0.0_f64;
    let mut img_energy = 0.0_f64;

    match method {
        MatchMethod::Sqdiff | MatchMethod::SqdiffNormed => {
            for ty in 0..TEMPL_H {
                for tx in 0..TEMPL_W {
                    if mask[ty * TEMPL_W + tx] == 0 {
                        continue;
                    }
                    let ti = templ.pixel_offset(tx, ty);
                    let ii = img.pixel_offset(TEST_X + tx, TEST_Y + ty);
                    for c in 0..ch {
                        let d = img.data[ii + c] as f64 - templ.data[ti + c] as f64;
                        val += d * d;
                        templ_energy += (templ.data[ti + c] as f64).powi(2);
                        img_energy += (img.data[ii + c] as f64).powi(2);
                    }
                }
            }
            if method == MatchMethod::SqdiffNormed {
                let norm = (templ_energy * img_energy).sqrt();
                if norm > f64::EPSILON {
                    val /= norm;
                } else {
                    val = 1.0;
                }
            }
        }
        MatchMethod::Ccorr | MatchMethod::CcorrNormed => {
            for ty in 0..TEMPL_H {
                for tx in 0..TEMPL_W {
                    if mask[ty * TEMPL_W + tx] == 0 {
                        continue;
                    }
                    let ti = templ.pixel_offset(tx, ty);
                    let ii = img.pixel_offset(TEST_X + tx, TEST_Y + ty);
                    for c in 0..ch {
                        let tv = templ.data[ti + c] as f64;
                        let iv = img.data[ii + c] as f64;
                        val += tv * iv;
                        templ_energy += tv * tv;
                        img_energy += iv * iv;
                    }
                }
            }
            if method == MatchMethod::CcorrNormed {
                let norm = (templ_energy * img_energy).sqrt();
                if norm > f64::EPSILON {
                    val /= norm;
                } else {
                    val = 0.0;
                }
            }
        }
        MatchMethod::Ccoeff | MatchMethod::CcoeffNormed => {
            let t_mean: Vec<f64> = sum_tm.iter().map(|s| s * inv_m).collect();
            let i_mean: Vec<f64> = sum_im.iter().map(|s| s * inv_m).collect();
            for ty in 0..TEMPL_H {
                for tx in 0..TEMPL_W {
                    if mask[ty * TEMPL_W + tx] == 0 {
                        continue;
                    }
                    let ti = templ.pixel_offset(tx, ty);
                    let ii = img.pixel_offset(TEST_X + tx, TEST_Y + ty);
                    for c in 0..ch {
                        let tp = templ.data[ti + c] as f64 - t_mean[c];
                        let ip = img.data[ii + c] as f64 - i_mean[c];
                        val += tp * ip;
                        templ_energy += tp * tp;
                        img_energy += ip * ip;
                    }
                }
            }
            if method == MatchMethod::CcoeffNormed {
                let norm = (templ_energy * img_energy).sqrt();
                if norm > f64::EPSILON {
                    val /= norm;
                } else {
                    val = 0.0;
                }
            }
        }
    }
    val
}

fn expect_near(got: f64, want: f64, area: f64) {
    let eps = area * want.abs() * f64::from(f32::EPSILON) + 1e-4;
    assert!(
        (got - want).abs() <= eps.max(1e-3),
        "got {got} want {want} (eps {eps})"
    );
}

#[test]
fn compare_naive_impl_all_methods_rgb() {
    let mut seed = 0xC0FFEE_u64;
    let img = random_image(IMG_W, IMG_H, 3, &mut seed, 9);
    let templ = random_image(TEMPL_W, TEMPL_H, 3, &mut seed, 9);
    let mask = random_binary_mask(TEMPL_W, TEMPL_H, &mut seed);
    let area = (TEMPL_W * TEMPL_H) as f64;

    for method in MatchMethod::all() {
        let map = match_template(&img, &templ, Some(&mask), method).unwrap();
        let got = map.scores[TEST_Y * map.width + TEST_X] as f64;
        let want = naive_at(&img, &templ, &mask, method);
        expect_near(got, want, area);
    }
}

#[test]
fn compare_naive_impl_all_methods_gray() {
    let mut seed = 0xBADC0DE_u64;
    let img = random_image(IMG_W, IMG_H, 1, &mut seed, 9);
    let templ = random_image(TEMPL_W, TEMPL_H, 1, &mut seed, 9);
    let mask = random_binary_mask(TEMPL_W, TEMPL_H, &mut seed);
    let area = (TEMPL_W * TEMPL_H) as f64;

    for method in MatchMethod::all() {
        let map = match_template(&img, &templ, Some(&mask), method).unwrap();
        let got = map.scores[TEST_Y * map.width + TEST_X] as f64;
        let want = naive_at(&img, &templ, &mask, method);
        expect_near(got, want, area);
    }
}

#[test]
fn compare_with_and_without_all_ones_mask() {
    let mut seed = 0x1234_5678_u64;
    let img = random_image(IMG_W, IMG_H, 3, &mut seed, 99);
    let templ = random_image(TEMPL_W, TEMPL_H, 3, &mut seed, 99);
    let ones = vec![255u8; TEMPL_W * TEMPL_H];
    let area = (TEMPL_W * TEMPL_H) as f64;

    for method in MatchMethod::all() {
        let with = match_template(&img, &templ, Some(&ones), method).unwrap();
        let without = match_template(&img, &templ, None, method).unwrap();
        assert_eq!(with.width, without.width);
        assert_eq!(with.height, without.height);
        let mut max_abs = 0.0_f32;
        let mut max_val = 0.0_f32;
        for (&a, &b) in with.scores.iter().zip(without.scores.iter()) {
            max_abs = max_abs.max((a - b).abs());
            max_val = max_val.max(a.abs()).max(b.abs());
        }
        let lim = (max_val as f64) * area * f64::from(f32::EPSILON) + 1e-3;
        assert!(
            (max_abs as f64) < lim.max(1e-3),
            "{method:?}: maxdiff={max_abs} lim={lim} max_val={max_val}"
        );
    }
}

/// OpenCV `Imgproc_MatchTemplateWithMask.bug_26389` — constant inputs must not panic.
#[test]
fn bug_26389_constant_ccoeff() {
    let image = ImageBuf::new(10, 10, 1, 1);
    let templ = ImageBuf::new(10, 7, 1, 1);
    let mask = vec![1u8; 10 * 7];
    for method in [MatchMethod::Ccoeff, MatchMethod::CcoeffNormed] {
        let map = match_template(&image, &templ, Some(&mask), method).unwrap();
        assert_eq!(map.width, 1);
        assert_eq!(map.height, 4);
        assert!(map.scores.iter().all(|s| s.is_finite()));
    }
}
