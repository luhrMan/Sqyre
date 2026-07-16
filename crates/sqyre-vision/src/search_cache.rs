//! Blurred-template and resized-mask cache.
//!
//! Entries are keyed by path + mtime (+ blur kernel / size). Invalidation helpers
//! drop prefixes when icons or masks change on disk. The cache is process-global
//! for reuse within a macro; call [`clear_search_cache`] when a run finishes so
//! peak RSS can be released.

use crate::image_util::{load_rgb_image, mask_as_u8, resize_mask};
use parking_lot::RwLock;
use sqyre_match::{blur_image_owned, ImageBuf};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// Soft cap on cached template + mask bytes (evict oldest on insert).
const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EntryKind {
    Template,
    Mask,
}

struct TemplateEntry {
    blurred: Arc<ImageBuf>,
    mod_time: SystemTime,
    blur_kernel: i32,
    bytes: usize,
}

struct MaskEntry {
    /// CV_8U-style bytes, length `width * height`.
    mask: Arc<Vec<u8>>,
    mod_time: SystemTime,
    bytes: usize,
}

#[derive(Default)]
struct SearchCache {
    templates: HashMap<String, TemplateEntry>,
    image_masks: HashMap<String, MaskEntry>,
    /// Oldest at front; newest / most recently used at back.
    lru: VecDeque<(EntryKind, String)>,
    /// Index into `lru` for O(1) touch.
    lru_index: HashMap<(EntryKind, String), usize>,
    bytes: usize,
}

impl SearchCache {
    fn rebuild_lru_index(&mut self) {
        self.lru_index.clear();
        for (i, (kind, key)) in self.lru.iter().enumerate() {
            self.lru_index.insert((*kind, key.clone()), i);
        }
    }

    fn touch(&mut self, kind: EntryKind, key: &str) {
        let map_key = (kind, key.to_string());
        let Some(&i) = self.lru_index.get(&map_key) else {
            return;
        };
        if i + 1 == self.lru.len() {
            return; // already most recent
        }
        if let Some(item) = self.lru.remove(i) {
            self.lru.push_back(item);
            self.rebuild_lru_index();
        }
    }

    fn remove_key(&mut self, kind: EntryKind, key: &str) {
        match kind {
            EntryKind::Template => {
                if let Some(e) = self.templates.remove(key) {
                    self.bytes = self.bytes.saturating_sub(e.bytes);
                }
            }
            EntryKind::Mask => {
                if let Some(e) = self.image_masks.remove(key) {
                    self.bytes = self.bytes.saturating_sub(e.bytes);
                }
            }
        }
        self.lru.retain(|(k, s)| !(*k == kind && s == key));
        self.rebuild_lru_index();
    }

    fn evict_until_fits(&mut self, extra: usize) {
        while self.bytes + extra > MAX_CACHE_BYTES {
            let Some((kind, key)) = self.lru.pop_front() else {
                break;
            };
            match kind {
                EntryKind::Template => {
                    if let Some(e) = self.templates.remove(&key) {
                        self.bytes = self.bytes.saturating_sub(e.bytes);
                    }
                }
                EntryKind::Mask => {
                    if let Some(e) = self.image_masks.remove(&key) {
                        self.bytes = self.bytes.saturating_sub(e.bytes);
                    }
                }
            }
        }
        self.rebuild_lru_index();
    }

    fn insert_template(&mut self, key: String, entry: TemplateEntry) {
        self.remove_key(EntryKind::Template, &key);
        self.evict_until_fits(entry.bytes);
        self.bytes += entry.bytes;
        self.templates.insert(key.clone(), entry);
        self.lru.push_back((EntryKind::Template, key.clone()));
        self.lru_index
            .insert((EntryKind::Template, key), self.lru.len() - 1);
    }

    fn insert_mask(&mut self, key: String, entry: MaskEntry) {
        self.remove_key(EntryKind::Mask, &key);
        self.evict_until_fits(entry.bytes);
        self.bytes += entry.bytes;
        self.image_masks.insert(key.clone(), entry);
        self.lru.push_back((EntryKind::Mask, key.clone()));
        self.lru_index
            .insert((EntryKind::Mask, key), self.lru.len() - 1);
    }

    fn clear(&mut self) {
        self.templates.clear();
        self.image_masks.clear();
        self.lru.clear();
        self.lru_index.clear();
        self.bytes = 0;
    }
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

/// Clears all cached templates and masks (call after a macro finishes).
pub fn clear_search_cache() {
    cache().write().clear();
}

/// Clears all cached templates and masks (tests).
pub fn reset_search_cache_for_testing() {
    clear_search_cache();
}

/// Serializes tests that share the process-global search cache.
pub fn with_search_cache_test_lock<R>(f: impl FnOnce() -> R) -> R {
    use parking_lot::Mutex;
    use std::sync::OnceLock;
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock();
    f()
}

/// Drop cached templates whose path starts with `icon_prefix` (item or program icons dir).
pub fn invalidate_search_templates_under(icon_prefix: &Path) {
    let prefix = icon_prefix.to_string_lossy();
    let mut guard = cache().write();
    let keys: Vec<String> = guard
        .templates
        .keys()
        .filter(|k| k.starts_with(prefix.as_ref()))
        .cloned()
        .collect();
    for key in keys {
        guard.remove_key(EntryKind::Template, &key);
    }
}

/// Drop cached masks whose path starts with `mask_prefix`.
pub fn invalidate_search_masks_under(mask_prefix: &Path) {
    let prefix = mask_prefix.to_string_lossy();
    let mut guard = cache().write();
    let keys: Vec<String> = guard
        .image_masks
        .keys()
        .filter(|k| k.starts_with(prefix.as_ref()))
        .cloned()
        .collect();
    for key in keys {
        guard.remove_key(EntryKind::Mask, &key);
    }
}

/// Load (or reuse) a blurred template for `icon_path` at `blur_kernel`.
pub fn get_cached_blurred_template(
    icon_path: &Path,
    blur_kernel: i32,
) -> Result<Arc<ImageBuf>, String> {
    let mod_time = file_mtime(icon_path)
        .ok_or_else(|| format!("stat {}: missing", icon_path.display()))?;
    let key = template_cache_key(icon_path, blur_kernel);

    {
        let guard = cache().read();
        if let Some(entry) = guard.templates.get(&key) {
            if entry.mod_time == mod_time && entry.blur_kernel == blur_kernel {
                let out = Arc::clone(&entry.blurred);
                drop(guard);
                cache().write().touch(EntryKind::Template, &key);
                return Ok(out);
            }
        }
    }

    let raw = load_rgb_image(icon_path)?;
    let blurred = Arc::new(
        blur_image_owned(raw, blur_kernel)
            .map_err(|e| format!("blur {}: {e}", icon_path.display()))?,
    );
    let bytes = blurred.data.len();

    let mut guard = cache().write();
    guard.insert_template(
        key,
        TemplateEntry {
            blurred: Arc::clone(&blurred),
            mod_time,
            blur_kernel,
            bytes,
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

    {
        let guard = cache().read();
        if let Some(entry) = guard.image_masks.get(&key) {
            if entry.mod_time == mod_time {
                let out = Arc::clone(&entry.mask);
                drop(guard);
                cache().write().touch(EntryKind::Mask, &key);
                return Some(out);
            }
        }
    }

    let loaded = load_rgb_image(mask_path).ok()?;
    let resized = resize_mask(&loaded, template_cols, template_rows);
    let mask = Arc::new(mask_as_u8(&resized));
    let bytes = mask.len();

    let mut guard = cache().write();
    guard.insert_mask(
        key,
        MaskEntry {
            mask: Arc::clone(&mask),
            mod_time,
            bytes,
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
    fn cache_hit_reuses_same_arc() {
        with_search_cache_test_lock(|| {
            reset_search_cache_for_testing();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("icon.png");
            write_rgb(&path, 8, 8, [10, 20, 30]);
            let k = search_blur_kernel(1);
            let a = get_cached_blurred_template(&path, k).unwrap();
            let b = get_cached_blurred_template(&path, k).unwrap();
            assert!(Arc::ptr_eq(&a, &b));
        });
    }

    #[test]
    fn invalidate_by_prefix() {
        with_search_cache_test_lock(|| {
            reset_search_cache_for_testing();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("icon.png");
            write_rgb(&path, 4, 4, [1, 2, 3]);
            let k = search_blur_kernel(0);
            let _ = get_cached_blurred_template(&path, k).unwrap();
            invalidate_search_templates_under(dir.path());
            let a = get_cached_blurred_template(&path, k).unwrap();
            let b = get_cached_blurred_template(&path, k).unwrap();
            assert!(Arc::ptr_eq(&a, &b));
        });
    }
}
