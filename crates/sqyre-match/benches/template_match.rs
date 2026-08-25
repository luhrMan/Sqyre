//! Synthetic template-match benches (direct correlator vs FFT path).
//!
//! Run: `cargo bench -p sqyre-match` or `make bench`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sqyre_match::{match_template, ImageBuf, MatchMethod};
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

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = bench_match
}
criterion_main!(benches);
