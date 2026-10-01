//! Megapixel capture swizzle Criterion benches (X11 ZPixmap path; Portal shares kernels).
//!
//! Run: `cargo bench -p sqyre-capture --bench pixel_swizzle` or via `make bench`.

use criterion::{criterion_group, criterion_main, Criterion};
use sqyre_capture::{zpixmap_to_rgb, zpixmap_to_rgba};
use std::hint::black_box;
use std::time::Duration;

fn megapixel_bgra(w: usize, h: usize) -> Vec<u8> {
    let mut data = vec![0u8; w * h * 4];
    for (i, px) in data.chunks_exact_mut(4).enumerate() {
        px[0] = (i % 251) as u8;
        px[1] = ((i * 3) % 251) as u8;
        px[2] = ((i * 7) % 251) as u8;
        px[3] = 255;
    }
    data
}

fn bench_swizzle(c: &mut Criterion) {
    let w = 1920u32;
    let h = 1080u32;
    let data = megapixel_bgra(w as usize, h as usize);

    c.bench_function("zpixmap_to_rgb_1920x1080_bgra", |b| {
        b.iter(|| {
            black_box(zpixmap_to_rgb(black_box(&data), w, h, 4, 0).expect("swizzle"));
        });
    });

    c.bench_function("zpixmap_to_rgba_1920x1080_bgra", |b| {
        b.iter(|| {
            black_box(zpixmap_to_rgba(black_box(&data), w, h, 4, 0).expect("swizzle"));
        });
    });

    // Smaller ROI still exercises Rayon gate / serial path boundary.
    let small = megapixel_bgra(320, 24);
    c.bench_function("zpixmap_to_rgb_320x24_bgra", |b| {
        b.iter(|| {
            black_box(zpixmap_to_rgb(black_box(&small), 320, 24, 4, 0).expect("swizzle"));
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_secs(2))
        .sample_size(20);
    targets = bench_swizzle
}
criterion_main!(benches);
