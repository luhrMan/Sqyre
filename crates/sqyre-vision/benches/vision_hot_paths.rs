//! Find-pixel scan and OCR preprocess benches (no Tesseract).
//!
//! Run: `cargo bench -p sqyre-vision` or `make bench`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sqyre_match::ImageBuf;
use sqyre_vision::{find_pixels, preprocess_for_ocr, OcrPreprocessOptions};
use std::time::Duration;

fn noisy_rgb(w: usize, h: usize, seed: u64) -> ImageBuf {
    let mut seed = seed;
    let mut img = ImageBuf::new(w, h, 3, 0);
    for px in img.data.iter_mut() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *px = (seed % 200) as u8;
    }
    // One exact hit near the end so a full scan is required.
    let o = img.pixel_offset(w - 3, h - 2);
    img.data[o] = 0xcc;
    img.data[o + 1] = 0x33;
    img.data[o + 2] = 0x99;
    img
}

fn bench_vision(c: &mut Criterion) {
    let img = noisy_rgb(640, 480, 9);
    c.bench_function("find_pixels_640x480_exact", |b| {
        b.iter(|| find_pixels(black_box(&img), black_box("#cc3399"), black_box(0)));
    });

    let ocr_src = noisy_rgb(320, 80, 11);
    let opts = OcrPreprocessOptions {
        grayscale: true,
        blur: true,
        blur_amount: 1,
        threshold: true,
        min_threshold: 0.0,
        threshold_otsu: true,
        threshold_invert: false,
        resize: false,
        resize_scale: 1.0,
    };
    c.bench_function("ocr_preprocess_320x80_gray_blur_otsu", |b| {
        b.iter(|| preprocess_for_ocr(black_box(&ocr_src), black_box(opts)).unwrap());
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = bench_vision
}
criterion_main!(benches);
