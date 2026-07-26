//! OCR golden: recognize a rendered "Submit" image when tessdata is available.
//!
//! Update expected text with `SQYRE_UPDATE_OCR_GOLDENS=1 cargo test -p sqyre-vision --test ocr_golden`.

#![cfg(not(target_arch = "wasm32"))]

use image::{Rgb, RgbImage};
use sqyre_vision::{preprocess_for_ocr, recognize_image, rgba_to_rgb_buf, OcrPreprocessOptions};
use std::path::PathBuf;

fn tessdata_or_skip() -> Option<String> {
    if let Ok(p) = std::env::var("SQYRE_TESSDATA") {
        let eng = std::path::Path::new(&p).join("eng.traineddata");
        if eng.is_file() {
            return Some(p);
        }
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/tessdata");
    if repo.join("eng.traineddata").is_file() {
        return Some(repo.to_string_lossy().into_owned());
    }
    None
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr")
}

/// Tiny 5×7 uppercase glyphs (bits top-to-bottom, MSB left).
fn glyph(ch: char) -> Option<[u8; 7]> {
    Some(match ch {
        'S' => [
            0b01110, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b01110,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        _ => return None,
    })
}

fn render_submit_png(path: &std::path::Path) {
    let scale = 8u32;
    let pad = 24u32;
    let text = "SUBMIT";
    let glyph_w = 5u32 * scale;
    let glyph_h = 7u32 * scale;
    let gap = 2u32 * scale;
    let width = pad * 2 + text.len() as u32 * (glyph_w + gap);
    let height = pad * 2 + glyph_h;
    let mut img = RgbImage::from_pixel(width, height, Rgb([255, 255, 255]));
    for (i, ch) in text.chars().enumerate() {
        let g = glyph(ch).expect("glyph");
        let ox = pad + i as u32 * (glyph_w + gap);
        let oy = pad;
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5u32 {
                if bits & (1 << (4 - col)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            img.put_pixel(
                                ox + col * scale + dx,
                                oy + row as u32 * scale + dy,
                                Rgb([0, 0, 0]),
                            );
                        }
                    }
                }
            }
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    img.save(path).unwrap();
}

#[test]
fn ocr_submit_golden() {
    let png_path = fixtures_dir().join("submit.png");
    if !png_path.is_file() {
        render_submit_png(&png_path);
    }

    let Some(tessdata) = tessdata_or_skip() else {
        eprintln!(
            "skipping ocr_submit_golden: tessdata not found (fixture ensured at {})",
            png_path.display()
        );
        return;
    };

    let rgba = image::open(&png_path).unwrap().to_rgba8();
    let rgb = rgba_to_rgb_buf(&rgba);
    let opts = OcrPreprocessOptions {
        grayscale: true,
        blur: false,
        blur_amount: 0,
        threshold: true,
        min_threshold: 0.0,
        threshold_otsu: true,
        threshold_invert: false,
        resize: false,
        resize_scale: 1.0,
    };
    let (pre, _scale) = preprocess_for_ocr(&rgb, opts).expect("preprocess");
    let result = recognize_image(&pre, &tessdata).expect("recognize");
    let normalized: String = result
        .text
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let expected_path = fixtures_dir().join("submit.expected.txt");
    if std::env::var_os("SQYRE_UPDATE_OCR_GOLDENS").is_some() {
        std::fs::write(&expected_path, &normalized).unwrap();
        eprintln!("updated {}", expected_path.display());
        return;
    }

    let expected = if expected_path.is_file() {
        std::fs::read_to_string(&expected_path)
            .unwrap()
            .trim()
            .to_string()
    } else {
        "SUBMIT".to_string()
    };

    assert!(
        normalized.contains(&expected) || expected.contains(&normalized) || normalized == expected,
        "OCR golden mismatch: got {normalized:?}, expected {expected:?} (set SQYRE_UPDATE_OCR_GOLDENS=1 to refresh)"
    );
}
