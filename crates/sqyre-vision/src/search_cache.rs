//! Blurred-template and resized-mask cache.
//!
//! Entries are keyed by path + mtime (+ blur kernel / size). Invalidation helpers
//! drop prefixes when icons or masks change on disk.

use crate::image_util::{load_rgb_image, mask_as_u8, resize_mask};
use sqyre_match::{blur_image, ImageBuf};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::SystemTime;

struct TemplateEntry {
    blurred: Arc<ImageBuf>,
    mod_time: SystemTime,
    blur_kernel: i32,
}

struct MaskEntry {
    /// CV_8U-style bytes, length `width * height`.
    mask: Arc<Vec<u8>>,
    mod_time: SystemTime,
}

#[derive(Default)]
struct SearchCache {
    templates: HashMap<String, TemplateEntry>,
    image_masks: HashMap<String, MaskEntry>,
}

fn cache() -> &'static RwLock<SearchCache> {
    static CACHE: OnceLock<RwLock<SearchCache>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(SearchCache::default()))
}

fn template_cache_key(path: &Path, blur_kernel: i32) -> String {
    format!("{}\0{blur_kernel}", path.display())
}

fn mask_cache_key(path: &Path, rows: usize, cols: usize) -> String {
    format!("{}\0{rows}\0{cols}", path.display())
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Clears all cached templates and masks (tests).
pub fn reset_search_cache_for_testing() {
    let mut guard = cache().write().expect("search cache lock");
    guard.templates.clear();
    guard.image_masks.clear();
}

/// Drop cached templates whose path starts with `icon_prefix` (item or program icons dir).
pub fn invalidate_search_templates_under(icon_prefix: &Path) {
    let prefix = icon_prefix.to_string_lossy();
    let mut guard = cache().write().expect("search cache lock");
    guard
        .templates
        .retain(|key, _| !key.starts_with(prefix.as_ref()));
}

/// Drop cached masks whose path starts with `mask_prefix`.
pub fn invalidate_search_masks_under(mask_prefix: &Path) {
    let prefix = mask_prefix.to_string_lossy();
    let mut guard = cache().write().expect("search cache lock");
    guard
        .image_masks
        .retain(|key, _| !key.starts_with(prefix.as_ref()));
}

/// Load (or reuse) a blurred template for `icon_path` at `blur_kernel`.
pub fn get_cached_blurred_template(
    icon_path: &Path,
    blur_kernel: i32,
) -> Result<Arc<ImageBuf>, String> {
    let mod_time = file_mtime(icon_path)
        .ok_or_else(|| format!("stat {}: missing", icon_path.display()))?;
    let key = template_cache_key(icon_path, blur_kernel);

    if let Ok(guard) = cache().read() {
        if let Some(entry) = guard.templates.get(&key) {
            if entry.mod_time == mod_time && entry.blur_kernel == blur_kernel {
                return Ok(Arc::clone(&entry.blurred));
            }
        }
    }

    let raw = load_rgb_image(icon_path)?;
    let blurred = Arc::new(
        blur_image(&raw, blur_kernel).map_err(|e| format!("blur {}: {e}", icon_path.display()))?,
    );

    let mut guard = cache().write().expect("search cache lock");
    guard.templates.insert(
        key,
        TemplateEntry {
            blurred: Arc::clone(&blurred),
            mod_time,
            blur_kernel,
        },
    );
    Ok(blurred)
}

/// Load (or reuse) a file mask resized to `template_cols` × `template_rows` as CV_8U bytes.
pub fn get_cached_image_mask(
    mask_path: &Path,
    template_rows: usize,
    template_cols: usize,
) -> Option<Arc<Vec<u8>>> {
    let mod_time = file_mtime(mask_path)?;
    let key = mask_cache_key(mask_path, template_rows, template_cols);

    if let Ok(guard) = cache().read() {
        if let Some(entry) = guard.image_masks.get(&key) {
            if entry.mod_time == mod_time {
                return Some(Arc::clone(&entry.mask));
            }
        }
    }

    let loaded = load_rgb_image(mask_path).ok()?;
    let resized = resize_mask(&loaded, template_cols, template_rows);
    let mask = Arc::new(mask_as_u8(&resized));

    let mut guard = cache().write().expect("search cache lock");
    guard.image_masks.insert(
        key,
        MaskEntry {
            mask: Arc::clone(&mask),
            mod_time,
        },
    );
    Some(mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use sqyre_match::search_blur_kernel;

    fn write_rgb(path: &Path, w: u32, h: u32, fill: [u8; 3]) {
        let img = RgbImage::from_pixel(w, h, Rgb(fill));
        img.save(path).unwrap();
    }

    #[test]
    fn blurred_template_cache_hits_and_invalidates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icon.png");
        write_rgb(&path, 16, 16, [40, 80, 120]);
        let kernel = search_blur_kernel(5);

        let a = get_cached_blurred_template(&path, kernel).unwrap();
        let b = get_cached_blurred_template(&path, kernel).unwrap();
        assert!(Arc::ptr_eq(&a, &b));

        invalidate_search_templates_under(dir.path());
        let c = get_cached_blurred_template(&path, kernel).unwrap();
        assert!(!Arc::ptr_eq(&a, &c));
        let d = get_cached_blurred_template(&path, kernel).unwrap();
        assert!(Arc::ptr_eq(&c, &d));
    }

    #[test]
    fn mask_cache_resizes_and_hits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mask.png");
        write_rgb(&path, 8, 8, [255, 255, 255]);

        let a = get_cached_image_mask(&path, 4, 4).unwrap();
        assert_eq!(a.len(), 16);
        let b = get_cached_image_mask(&path, 4, 4).unwrap();
        assert!(Arc::ptr_eq(&a, &b));

        invalidate_search_masks_under(dir.path());
        let c = get_cached_image_mask(&path, 4, 4).unwrap();
        assert!(!Arc::ptr_eq(&a, &c));
    }

    #[test]
    fn mtime_change_refreshes_template() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icon.png");
        write_rgb(&path, 16, 16, [10, 20, 30]);
        let kernel = search_blur_kernel(5);

        let first = get_cached_blurred_template(&path, kernel).unwrap();
        // Ensure mtime can advance on filesystems with coarse resolution.
        std::thread::sleep(std::time::Duration::from_millis(20));
        write_rgb(&path, 16, 16, [200, 100, 50]);
        // Bump mtime explicitly when the FS truncates sub-second stamps.
        let t = file_mtime(&path).unwrap() + std::time::Duration::from_secs(1);
        filetime_set(&path, t);

        let second = get_cached_blurred_template(&path, kernel).unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        assert_ne!(first.data, second.data);
    }

    fn filetime_set(path: &Path, modified: SystemTime) {
        let file = std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap();
        file.set_modified(modified).unwrap();
    }
}
