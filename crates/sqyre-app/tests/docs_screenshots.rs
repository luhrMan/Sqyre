//! In-memory egui screenshots for README assets under `docs/images/`.
//!
//! Regenerate:
//!   SQYRE_UPDATE_SCREENSHOTS=1 ./scripts/generate-docs-media.sh
//! or: make docs-media

mod common;

use common::build_docs_harness;
use egui_kittest::Harness;
use image::{ImageFormat, RgbaImage};
use sqyre_app::SqyreApp;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const MAIN_SIZE: [f32; 2] = [1120.0, 560.0];
const PICKER_SIZE: [f32; 2] = [1100.0, 520.0];
const EDITOR_SIZE: [f32; 2] = [1120.0, 620.0];
const SETTINGS_SIZE: [f32; 2] = [760.0, 700.0];
const PALETTE_SIZE: [f32; 2] = [1000.0, 560.0];
const MIN_PNG_BYTES: usize = 5_000;

fn docs_images_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/images")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/images"))
}

fn update_screenshots() -> bool {
    matches!(
        std::env::var("SQYRE_UPDATE_SCREENSHOTS").ok().as_deref(),
        Some("1") | Some("true")
    ) || matches!(
        std::env::var("UPDATE_SNAPSHOTS").ok().as_deref(),
        Some("true") | Some("force") | Some("1")
    )
}

fn write_or_compare_png(path: &Path, img: &RgbaImage) {
    let mut encoded = Vec::new();
    img.write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
        .expect("encode png");
    assert!(
        encoded.len() >= MIN_PNG_BYTES,
        "screenshot {} too small ({} bytes)",
        path.display(),
        encoded.len()
    );
    if update_screenshots() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, &encoded).unwrap_or_else(|e| {
            panic!("write {}: {e}", path.display());
        });
        return;
    }
    let existing = std::fs::read(path).unwrap_or_else(|e| {
        panic!(
            "missing golden {} (run make docs-media): {e}",
            path.display()
        );
    });
    if existing != encoded {
        if existing.len() > MIN_PNG_BYTES {
            let old = image::load_from_memory(&existing)
                .expect("decode golden")
                .into_rgba8();
            if old.dimensions() == img.dimensions() {
                let mut diff = 0u64;
                let mut sum = 0u64;
                for (a, b) in old.pixels().zip(img.pixels()) {
                    for i in 0..3 {
                        let d = (a.0[i] as i16 - b.0[i] as i16).unsigned_abs() as u64;
                        diff += d;
                        sum += 255;
                    }
                }
                let score = diff as f64 / sum as f64;
                if score < 0.02 {
                    return;
                }
                panic!(
                    "screenshot drift: {} (diff score {score:.4}); regenerate with make docs-media",
                    path.display()
                );
            }
        }
        panic!(
            "screenshot drift: {}; regenerate with make docs-media",
            path.display()
        );
    }
}

fn render_png(harness: &mut Harness<'_, SqyreApp>) -> RgbaImage {
    let _ = harness.run_ok();
    harness.run_steps(2);
    harness.render().expect("wgpu render")
}

#[test]
fn docs_main_window() {
    let mut harness = build_docs_harness(MAIN_SIZE, |app| {
        app.expand_all_branches_for_docs();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("main-window.png"), &img);
}

#[test]
fn docs_add_action_picker() {
    let mut harness = build_docs_harness(PICKER_SIZE, |app| {
        app.open_add_action_picker();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("add-action-picker.png"), &img);
}

#[test]
fn docs_data_editor() {
    let mut harness = build_docs_harness(EDITOR_SIZE, |app| {
        app.open_data_editor();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("data-editor.png"), &img);
}

#[test]
fn docs_settings() {
    let mut harness = build_docs_harness(SETTINGS_SIZE, |app| {
        app.open_settings_appearance_for_docs();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("settings.png"), &img);
}

#[test]
fn docs_command_palette() {
    let mut harness = build_docs_harness(PALETTE_SIZE, |app| {
        app.open_command_palette_for_docs();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("command-palette.png"), &img);
}
