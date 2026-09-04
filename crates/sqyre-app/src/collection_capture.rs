//! Capture a collection's linked search area and write the static preview PNG.

use sqyre_capture::shared_capturer;
use sqyre_domain::{CoordinateRef, Macro, PROGRAM_DELIMITER};
use sqyre_persist::{ProgramCatalog, ProgramCollection};
use sqyre_ports::ScreenCapturer;
use std::path::{Path, PathBuf};

/// Resolve paths/coords on the UI thread; capture runs on a worker.
pub fn collection_capture_job(
    catalog: &ProgramCatalog,
    program: &str,
    collection: &ProgramCollection,
) -> Result<(PathBuf, i32, i32, i32, i32), String> {
    let (left, top, right, bottom) = resolve_collection_capture_rect(catalog, program, collection)?;
    let path = catalog.collection_image_path(program, &collection.name);
    Ok((path, left, top, right, bottom))
}

/// Blocking capture+save used by tests and worker threads.
pub fn capture_search_area_to_png(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    path: &Path,
) -> Result<(), String> {
    let capturer = shared_capturer().map_err(|e| format!("collection capture: {e}"))?;
    let mut wrap = sqyre_capture::SharedRunCapturer(capturer);
    capture_rect_and_save_png_with(&mut wrap, left, top, right, bottom, path)
}

/// Capture using an injected [`ScreenCapturer`] (tests use `SolidCapturer`).
#[cfg(test)]
pub fn capture_and_save_collection_image_with(
    capturer: &mut dyn ScreenCapturer,
    catalog: &ProgramCatalog,
    program: &str,
    collection: &ProgramCollection,
) -> Result<(), String> {
    let (path, left, top, right, bottom) = collection_capture_job(catalog, program, collection)?;
    capture_rect_and_save_png_with(capturer, left, top, right, bottom, &path)
}

/// Resolve the collection's linked search area to screen coordinates.
pub fn resolve_collection_capture_rect(
    catalog: &ProgramCatalog,
    program: &str,
    collection: &ProgramCollection,
) -> Result<(i32, i32, i32, i32), String> {
    if collection.search_area.is_empty() {
        return Err("collection has no search area".into());
    }
    let sa_ref = CoordinateRef(format!(
        "{program}{PROGRAM_DELIMITER}{}",
        collection.search_area
    ));
    // Data-editor capture uses literal coords only (no macro variable scope).
    let empty = Macro::new("", 0, vec![]);
    catalog
        .resolve_search_area(&sa_ref, &empty)
        .map_err(|e| format!("collection capture: {e}"))
}

/// Capture a search-area rectangle and write PNG to `path`.
pub fn capture_rect_and_save_png_with(
    capturer: &mut dyn ScreenCapturer,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    path: &Path,
) -> Result<(), String> {
    let (img, _) = capturer
        .capture_search_area(left, top, right, bottom)
        .map_err(|e| format!("collection capture: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create collections dir: {e}"))?;
    }
    save_png(&img, path)
}

fn save_png(img: &image::RgbaImage, path: &Path) -> Result<(), String> {
    img.save(path)
        .map_err(|e| format!("save collection image {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use sqyre_capture::SolidCapturer;
    use sqyre_domain::ScalarValue;
    use sqyre_persist::ProgramSearchArea;
    use sqyre_ports::DesktopRect;

    fn catalog_with_sa(root: PathBuf) -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_images_root(Some(root));
        cat.create_program("Demo").unwrap();
        cat.upsert_search_area(
            "Demo",
            ProgramSearchArea {
                name: "Box".into(),
                monitor: 1,
                left_x: ScalarValue::Int(10),
                top_y: ScalarValue::Int(20),
                right_x: ScalarValue::Int(110),
                bottom_y: ScalarValue::Int(80),
            },
        )
        .unwrap();
        cat
    }

    #[test]
    fn capture_writes_png_sized_to_search_area() {
        let tmp = tempfile::tempdir().unwrap();
        let cat = catalog_with_sa(tmp.path().to_path_buf());
        let col = ProgramCollection {
            name: "Bag".into(),
            search_area: "Box".into(),
            rows: 2,
            cols: 3,
        };
        let mut capturer = SolidCapturer {
            color: Rgba([1, 2, 3, 255]),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 200,
                h: 200,
            },
        };
        capture_and_save_collection_image_with(&mut capturer, &cat, "Demo", &col).unwrap();
        let path = cat.collection_image_path("Demo", "Bag");
        assert!(path.is_file(), "expected {}", path.display());
        let img = image::open(&path).unwrap().into_rgba8();
        assert_eq!((img.width(), img.height()), (100, 60));
        assert_eq!(*img.get_pixel(0, 0), Rgba([1, 2, 3, 255]));
    }

    #[test]
    fn capture_requires_search_area() {
        let tmp = tempfile::tempdir().unwrap();
        let cat = catalog_with_sa(tmp.path().to_path_buf());
        let col = ProgramCollection {
            name: "Bag".into(),
            search_area: String::new(),
            rows: 1,
            cols: 1,
        };
        let mut capturer = SolidCapturer::default();
        let err =
            capture_and_save_collection_image_with(&mut capturer, &cat, "Demo", &col).unwrap_err();
        assert!(err.contains("no search area"));
    }
}
