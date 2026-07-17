//! Capture a collection's linked search area and write the static preview PNG.

use sqyre_capture::X11Capturer;
use sqyre_domain::{CoordinateRef, Macro, PROGRAM_DELIMITER};
use sqyre_executor::ScreenCapturer;
use sqyre_persist::{ProgramCatalog, ProgramCollection};
use std::path::Path;

/// Open the platform capturer, capture the collection's search area, and save PNG.
pub fn capture_and_save_collection_image(
    catalog: &ProgramCatalog,
    program: &str,
    collection: &ProgramCollection,
) -> Result<(), String> {
    let mut capturer = X11Capturer::open().map_err(|e| format!("collection capture: {e}"))?;
    capture_and_save_collection_image_with(&mut capturer, catalog, program, collection)
}

/// Capture using an injected [`ScreenCapturer`] (tests use `SolidCapturer`).
pub fn capture_and_save_collection_image_with(
    capturer: &mut dyn ScreenCapturer,
    catalog: &ProgramCatalog,
    program: &str,
    collection: &ProgramCollection,
) -> Result<(), String> {
    if collection.search_area.is_empty() {
        return Err("collection has no search area".into());
    }
    let sa_ref = CoordinateRef(format!(
        "{program}{PROGRAM_DELIMITER}{}",
        collection.search_area
    ));
    // Data-editor capture uses literal coords only (no macro variable scope).
    let empty = Macro::new("", 0, vec![]);
    let (left, top, right, bottom) = catalog
        .resolve_search_area(&sa_ref, &empty)
        .map_err(|e| format!("collection capture: {e}"))?;
    let (img, _) = capturer
        .capture_search_area(left, top, right, bottom)
        .map_err(|e| format!("collection capture: {e}"))?;
    let dir = catalog.collections_dir(program);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create collections dir: {e}"))?;
    let path = catalog.collection_image_path(program, &collection.name);
    save_png(&img, &path)
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
    use sqyre_executor::DesktopRect;
    use sqyre_persist::ProgramSearchArea;
    use std::path::PathBuf;

    fn catalog_with_sa(root: PathBuf) -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_images_root(Some(root));
        cat.create_program("Demo").unwrap();
        cat.upsert_search_area(
            "Demo",
            ProgramSearchArea {
                name: "Box".into(),
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
