//! OCR preprocess goldens (no Tesseract): stable dims + FNV-1a of the output buffer.
//!
//! Refresh expected files with `SQYRE_UPDATE_VISION_GOLDENS=1 cargo test -p sqyre-vision --test preprocess_golden`.

use sqyre_match::ImageBuf;
use sqyre_vision::{preprocess_for_ocr, OcrPreprocessOptions};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/preprocess")
}

/// Deterministic 16×8 RGB gradient (not a photograph).
fn source_rgb() -> ImageBuf {
    let w = 16usize;
    let h = 8usize;
    let mut data = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            data.push((x * 17) as u8);
            data.push((y * 31) as u8);
            data.push(((x + y) * 13) as u8);
        }
    }
    ImageBuf::from_raw(w, h, 3, data)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn expected_path(name: &str) -> PathBuf {
    fixtures_dir().join(format!("{name}.expected.txt"))
}

fn assert_variant(name: &str, opts: OcrPreprocessOptions) {
    let (out, _scale) = preprocess_for_ocr(&source_rgb(), opts).expect(name);
    let digest = fnv1a64(&out.data);
    let body = format!(
        "{} {} {} {digest:016x}\n",
        out.width, out.height, out.channels
    );
    let path = expected_path(name);
    if std::env::var_os("SQYRE_UPDATE_VISION_GOLDENS").is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, &body).unwrap();
        eprintln!("updated {}", path.display());
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing golden {}: {e} (set SQYRE_UPDATE_VISION_GOLDENS=1 to write)",
            path.display()
        );
    });
    assert_eq!(
        body, expected,
        "preprocess golden mismatch for {name} (set SQYRE_UPDATE_VISION_GOLDENS=1 to refresh)"
    );
}

fn grayscale_only() -> OcrPreprocessOptions {
    OcrPreprocessOptions {
        grayscale: true,
        blur: false,
        blur_amount: 0,
        threshold: false,
        min_threshold: 0.0,
        threshold_otsu: false,
        threshold_invert: false,
        resize: false,
        resize_scale: 1.0,
    }
}

#[test]
fn preprocess_grayscale_golden() {
    assert_variant("grayscale", grayscale_only());
}

#[test]
fn preprocess_otsu_golden() {
    let mut opts = grayscale_only();
    opts.threshold = true;
    opts.threshold_otsu = true;
    assert_variant("grayscale_otsu", opts);
}

#[test]
fn preprocess_threshold_invert_golden() {
    let mut opts = grayscale_only();
    opts.threshold = true;
    opts.min_threshold = 128.0;
    opts.threshold_invert = true;
    assert_variant("threshold_invert", opts);
}

#[test]
fn preprocess_blur_threshold_changes_pixels() {
    let gray = preprocess_for_ocr(&source_rgb(), grayscale_only())
        .unwrap()
        .0;
    let mut opts = grayscale_only();
    opts.blur = true;
    opts.blur_amount = 1;
    opts.threshold = true;
    opts.min_threshold = 64.0;
    let blurred = preprocess_for_ocr(&source_rgb(), opts).unwrap().0;
    assert_eq!(
        (blurred.width, blurred.height, blurred.channels),
        (16, 8, 1)
    );
    assert_ne!(
        blurred.data, gray.data,
        "blur+threshold should differ from grayscale-only"
    );
}

#[test]
fn preprocess_resize_golden() {
    let mut opts = grayscale_only();
    opts.resize = true;
    opts.resize_scale = 2.0;
    assert_variant("grayscale_resize2", opts);
}
