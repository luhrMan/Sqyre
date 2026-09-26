//! Synthetic template-match benches (direct correlator vs FFT path).
//!
//! Run: `cargo bench -p sqyre-match` or `make bench`.

use criterion::{criterion_group, criterion_main, Criterion};
use rayon::prelude::*;
use sqyre_match::{
    accumulate_corr_row, match_template, match_template_with_prepared, prepare_search,
    prepare_template, ImageBuf, MatchMethod, PlanarF32, SparseTemplate,
};
use std::hint::black_box;
use std::time::Duration;

fn xorshift(seed: &mut u64) -> u8 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed % 256) as u8
}

fn random_rgb(w: usize, h: usize, seed: u64) -> ImageBuf {
    let mut seed = seed;
    let mut img = ImageBuf::new(w, h, 3, 0);
    for px in img.data.iter_mut() {
        *px = xorshift(&mut seed);
    }
    img
}

fn dense_rgb_template(tw: usize, th: usize) -> SparseTemplate {
    let n = tw * th;
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    let mut vals = Vec::with_capacity(n * 3);
    for y in 0..th {
        for x in 0..tw {
            xs.push(x as u16);
            ys.push(y as u16);
            let base = (y * tw + x) as f32;
            vals.push(base * 0.01);
            vals.push(base * 0.02 + 1.0);
            vals.push(base * 0.03 + 2.0);
        }
    }
    SparseTemplate {
        xs,
        ys,
        vals,
        channels: 3,
    }
}

fn bench_match(c: &mut Criterion) {
    // Direct path: well under FFT_DIRECT_COST_THRESHOLD.
    let search_small = random_rgb(96, 72, 1);
    let templ_small = random_rgb(12, 10, 2);
    c.bench_function("match_ccoeff_normed_direct_96x72_t12x10", |b| {
        b.iter(|| {
            match_template(
                black_box(&search_small),
                black_box(&templ_small),
                None,
                MatchMethod::CcoeffNormed,
            )
            .unwrap()
        });
    });

    // Unmasked medium search uses DFT cross-correlation.
    let search_med = random_rgb(320, 240, 3);
    let templ_med = random_rgb(32, 24, 4);
    c.bench_function("match_ccoeff_normed_fft_320x240_t32x24", |b| {
        b.iter(|| {
            match_template(
                black_box(&search_med),
                black_box(&templ_med),
                None,
                MatchMethod::CcoeffNormed,
            )
            .unwrap()
        });
    });
}

/// Baseline for nested Rayon (M-1): N template variants share one search frame /
/// [`sqyre_match::SearchPrep`], matched via outer `par_iter`.
fn bench_multi_variant(c: &mut Criterion) {
    const N: usize = 8;
    let search = random_rgb(160, 120, 10);
    let prep = prepare_search(&search);
    let variants: Vec<_> = (0..N)
        .map(|i| {
            let tmpl = random_rgb(16, 12, 100 + i as u64);
            let prepared = prepare_template(&tmpl, None, MatchMethod::CcoeffNormed).unwrap();
            (tmpl, prepared)
        })
        .collect();

    c.bench_function("match_8_variants_shared_prep_160x120_t16x12", |b| {
        b.iter(|| {
            variants.par_iter().for_each(|(tmpl, prepared)| {
                let _ = match_template_with_prepared(
                    black_box(&search),
                    black_box(tmpl),
                    black_box(prepared),
                    Some(black_box(&prep)),
                )
                .unwrap();
            });
        });
    });
}

/// Microbench for L-1 explicit SIMD saxpy (`accumulate_corr_row`).
fn bench_accumulate_corr(c: &mut Criterion) {
    let search = random_rgb(256, 128, 7);
    let planar = PlanarF32::from_interleaved(&search);
    let tmpl = dense_rgb_template(24, 18);
    let out_w = planar.width - 24;
    let mut numer = vec![0.0_f32; out_w];
    c.bench_function("accumulate_corr_row_256w_t24x18", |b| {
        b.iter(|| {
            accumulate_corr_row(
                black_box(&planar),
                black_box(&tmpl),
                black_box(10),
                black_box(&mut numer),
            );
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = bench_match, bench_multi_variant, bench_accumulate_corr
}
criterion_main!(benches);
