//! Stable section names for Go↔Rust compare and Rust baseline diffs.
//!
//! Keep these names stable — efficiency PRs measure the same keys before/after.

use crate::metrics::{finish, sample_now, SectionResult};
use sqyre_capture::zpixmap_to_rgb;
use sqyre_domain::{
    Action, ActionId, ActionKind, Macro, MatchMethod, MouseButton, PressState, ScalarValue,
};
use sqyre_match::{
    match_template, match_template_with_prepared, prepare_search, prepare_template, ImageBuf,
};
use sqyre_persist::Database;
use sqyre_serialize::{decode_macro_from_yaml, encode_macro_to_yaml};
use sqyre_vision::{find_pixels, preprocess_for_ocr, OcrPreprocessOptions};
use std::hint::black_box;
use std::path::Path;

/// Canonical section identifiers (also used by the Go harness).
pub const ALL_SECTIONS: &[&str] = &[
    "match_direct",
    "match_fft",
    "match_multi_variant",
    "search_prep",
    "find_pixels",
    "ocr_preprocess",
    "persist_yaml_load",
    "persist_yaml_save",
    "macro_codec_encode",
    "macro_codec_decode",
    "zpixmap_swizzle",
];

pub fn run_section(name: &str, iterations: u64, fixture_db: &Path) -> SectionResult {
    match name {
        "match_direct" => match_direct(iterations),
        "match_fft" => match_fft(iterations),
        "match_multi_variant" => match_multi_variant(iterations),
        "search_prep" => search_prep(iterations),
        "find_pixels" => find_pixels_section(iterations),
        "ocr_preprocess" => ocr_preprocess(iterations),
        "persist_yaml_load" => persist_yaml_load(iterations, fixture_db),
        "persist_yaml_save" => persist_yaml_save(iterations, fixture_db),
        "macro_codec_encode" => macro_codec_encode(iterations),
        "macro_codec_decode" => macro_codec_decode(iterations),
        "zpixmap_swizzle" => zpixmap_swizzle(iterations),
        other => SectionResult::error(other, format!("unknown section {other}")),
    }
}

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

fn match_direct(iterations: u64) -> SectionResult {
    let search = random_rgb(96, 72, 1);
    let templ = random_rgb(12, 10, 2);
    let start = sample_now();
    for _ in 0..iterations {
        let map = match_template(
            black_box(&search),
            black_box(&templ),
            None,
            MatchMethod::CcoeffNormed,
        );
        black_box(map.expect("match_direct"));
    }
    finish(
        "match_direct",
        iterations,
        start,
        Some("96x72 search, 12x10 template, CcoeffNormed (direct path)".into()),
    )
}

fn match_fft(iterations: u64) -> SectionResult {
    let search = random_rgb(320, 240, 3);
    let templ = random_rgb(32, 24, 4);
    let start = sample_now();
    for _ in 0..iterations {
        let map = match_template(
            black_box(&search),
            black_box(&templ),
            None,
            MatchMethod::CcoeffNormed,
        );
        black_box(map.expect("match_fft"));
    }
    finish(
        "match_fft",
        iterations,
        start,
        Some("320x240 search, 32x24 template, CcoeffNormed (FFT path)".into()),
    )
}

fn match_multi_variant(iterations: u64) -> SectionResult {
    use rayon::prelude::*;
    const N: usize = 8;
    let search = random_rgb(160, 120, 10);
    let prep = prepare_search(&search);
    let variants: Vec<_> = (0..N)
        .map(|i| {
            let tmpl = random_rgb(16, 12, 100 + i as u64);
            let prepared =
                prepare_template(&tmpl, None, MatchMethod::CcoeffNormed).expect("prepare_template");
            (tmpl, prepared)
        })
        .collect();
    let start = sample_now();
    for _ in 0..iterations {
        let maps: Vec<_> = variants
            .par_iter()
            .map(|(tmpl, prepared)| {
                match_template_with_prepared(
                    black_box(&search),
                    black_box(tmpl),
                    prepared,
                    Some(&prep),
                )
                .expect("multi variant")
            })
            .collect();
        black_box(maps);
    }
    finish(
        "match_multi_variant",
        iterations,
        start,
        Some("8 variants × shared SearchPrep, 160x120 / 16x12".into()),
    )
}

fn search_prep(iterations: u64) -> SectionResult {
    let search = random_rgb(640, 480, 7);
    let start = sample_now();
    for _ in 0..iterations {
        black_box(prepare_search(black_box(&search)));
    }
    finish(
        "search_prep",
        iterations,
        start,
        Some("prepare_search 640x480 RGB".into()),
    )
}

fn find_pixels_section(iterations: u64) -> SectionResult {
    let mut img = random_rgb(640, 480, 9);
    let o = img.pixel_offset(637, 478);
    img.data[o] = 0xcc;
    img.data[o + 1] = 0x33;
    img.data[o + 2] = 0x99;
    let start = sample_now();
    for _ in 0..iterations {
        black_box(find_pixels(
            black_box(&img),
            black_box("#cc3399"),
            black_box(0),
        ));
    }
    finish(
        "find_pixels",
        iterations,
        start,
        Some("640x480 exact RGB scan, one hit".into()),
    )
}

fn ocr_preprocess(iterations: u64) -> SectionResult {
    let src = random_rgb(320, 80, 11);
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
    let start = sample_now();
    for _ in 0..iterations {
        black_box(preprocess_for_ocr(black_box(&src), black_box(opts)).expect("ocr preprocess"));
    }
    finish(
        "ocr_preprocess",
        iterations,
        start,
        Some("320x80 gray+blur+otsu (no Tesseract)".into()),
    )
}

fn persist_yaml_load(iterations: u64, fixture_db: &Path) -> SectionResult {
    if !fixture_db.is_file() {
        return SectionResult::skipped(
            "persist_yaml_load",
            format!("fixture missing: {}", fixture_db.display()),
        );
    }
    let start = sample_now();
    for _ in 0..iterations {
        black_box(Database::load_from_path(black_box(fixture_db)).expect("load db"));
    }
    finish(
        "persist_yaml_load",
        iterations,
        start,
        Some(format!("load {}", fixture_db.display())),
    )
}

fn persist_yaml_save(iterations: u64, fixture_db: &Path) -> SectionResult {
    if !fixture_db.is_file() {
        return SectionResult::skipped(
            "persist_yaml_save",
            format!("fixture missing: {}", fixture_db.display()),
        );
    }
    let db = match Database::load_from_path(fixture_db) {
        Ok(db) => db,
        Err(e) => return SectionResult::error("persist_yaml_save", e.to_string()),
    };
    let out = std::env::temp_dir().join(format!("sqyre-bench-persist-{}.yaml", std::process::id()));
    let start = sample_now();
    for _ in 0..iterations {
        db.save_to_path(black_box(&out)).expect("save db");
    }
    let _ = std::fs::remove_file(&out);
    finish(
        "persist_yaml_save",
        iterations,
        start,
        Some("atomic save of fixture Database".into()),
    )
}

fn sample_macro() -> Macro {
    let mut m = Macro::new("bench", 0, vec![]);
    m.root = sqyre_domain::root_loop(vec![
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(25),
            },
        },
        Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(3),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Click {
                        button: MouseButton::Left,
                        state: PressState::Tap,
                    },
                }],
            },
        },
    ]);
    m
}

fn macro_codec_encode(iterations: u64) -> SectionResult {
    let macro_ = sample_macro();
    let start = sample_now();
    for _ in 0..iterations {
        black_box(encode_macro_to_yaml(black_box(&macro_)).expect("encode"));
    }
    finish(
        "macro_codec_encode",
        iterations,
        start,
        Some("Wait+Loop+Click macro YAML encode".into()),
    )
}

fn macro_codec_decode(iterations: u64) -> SectionResult {
    let yaml = encode_macro_to_yaml(&sample_macro()).expect("encode once");
    let start = sample_now();
    for _ in 0..iterations {
        black_box(decode_macro_from_yaml(black_box(&yaml)).expect("decode"));
    }
    finish(
        "macro_codec_decode",
        iterations,
        start,
        Some("Wait+Loop+Click macro YAML decode".into()),
    )
}

fn zpixmap_swizzle(iterations: u64) -> SectionResult {
    // 1920×1080 BGRA ZPixmap (megapixel capture swizzle).
    let w = 1920u32;
    let h = 1080u32;
    let bpp = 4usize;
    let stride = w as usize * bpp;
    let mut data = vec![0u8; stride * h as usize];
    for (i, px) in data.chunks_exact_mut(4).enumerate() {
        px[0] = (i % 256) as u8;
        px[1] = ((i / 3) % 256) as u8;
        px[2] = ((i / 7) % 256) as u8;
        px[3] = 255;
    }
    let start = sample_now();
    for _ in 0..iterations {
        black_box(zpixmap_to_rgb(black_box(&data), w, h, bpp, stride).expect("swizzle"));
    }
    finish(
        "zpixmap_swizzle",
        iterations,
        start,
        Some("1920x1080 BGRA→RGB (capture swizzle)".into()),
    )
}
