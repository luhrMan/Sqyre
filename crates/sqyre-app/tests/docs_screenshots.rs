//! In-memory egui screenshots for README assets under `docs/images/`.
//!
//! Regenerate:
//!   SQYRE_UPDATE_SCREENSHOTS=1 ./scripts/generate-docs-media.sh
//! or: make docs-media

use egui::os::OperatingSystem;
use egui_kittest::{Harness, SnapshotOptions};
use image::{ImageFormat, RgbaImage};
use sqyre_app::{theme, SettingsUi, SqyreApp};
use std::io::Cursor;
use std::path::{Path, PathBuf};

const MAIN_SIZE: [f32; 2] = [1000.0, 500.0];
const PICKER_SIZE: [f32; 2] = [1100.0, 520.0];
const MIN_PNG_BYTES: usize = 5_000;

fn docs_images_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/images")
        .canonicalize()
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/images")
        })
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

fn sync_update_env() {
    if matches!(
        std::env::var("SQYRE_UPDATE_SCREENSHOTS").ok().as_deref(),
        Some("1") | Some("true")
    ) {
        std::env::set_var("UPDATE_SNAPSHOTS", "force");
    }
}

fn snapshot_opts(dir: &Path) -> SnapshotOptions {
    SnapshotOptions::new()
        .output_path(dir)
        .threshold(0.8)
}

fn build_harness(size: [f32; 2], mut setup: impl FnMut(&mut SqyreApp)) -> Harness<'static, SqyreApp> {
    let mut app = SqyreApp::for_docs();
    setup(&mut app);
    let settings = app.docs_settings().clone();
    Harness::builder()
        .with_size(size)
        .with_os(OperatingSystem::Nix)
        .with_options(snapshot_opts(&docs_images_dir()))
        .wgpu()
        .build_eframe(move |cc| {
            SettingsUi::install_fonts(&cc.egui_ctx);
            SettingsUi::apply_appearance(&cc.egui_ctx, &settings);
            theme::apply(&cc.egui_ctx);
            app
        })
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
    sync_update_env();
    let mut harness = build_harness(MAIN_SIZE, |app| {
        app.expand_all_branches_for_docs();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("main-window.png"), &img);
}

#[test]
fn docs_add_action_picker() {
    sync_update_env();
    let mut harness = build_harness(PICKER_SIZE, |app| {
        app.open_add_action_picker();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("add-action-picker.png"), &img);
}

#[test]
fn docs_data_editor() {
    sync_update_env();
    let mut harness = build_harness(MAIN_SIZE, |app| {
        app.open_data_editor();
    });
    let img = render_png(&mut harness);
    write_or_compare_png(&docs_images_dir().join("data-editor.png"), &img);
}
