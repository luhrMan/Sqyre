//! Blurred-template and resized-mask cache.
//!
//! Entries are keyed by path + mtime (+ blur kernel / size). Invalidation helpers
//! drop prefixes when icons or masks change on disk. The cache is process-global
//! for reuse within a macro; call [`clear_search_cache`] when a run finishes so
//! peak RSS can be released.

use crate::image_util::{load_rgb_image, mask_as_u8, resize_mask};
use parking_lot::{Mutex, RwLock};
use sqyre_match::{blur_image_owned, prepare_template, ImageBuf, MatchMethod, PreparedTemplate};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// Soft cap on cached template + mask + prepared-template bytes (evict oldest on insert).
const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
enum EntryKind {
    Template,
    Mask,
    Prepared,
}

/// Monotonic "clock" used to stamp recency without a write lock: a hit stores
/// the next tick into the entry's atomic field via a shared reference, so
/// touching an entry never needs to upgrade the cache's read lock to a write
/// lock. Recency order is only consulted (via an O(n) scan) when evicting on
/// insert, which is far rarer than lookups.
fn next_tick() -> u64 {
    static CLOCK: AtomicU64 = AtomicU64::new(0);
    CLOCK.fetch_add(1, Ordering::Relaxed)
}

struct TemplateEntry {
    blurred: Arc<ImageBuf>,
    mod_time: SystemTime,
    blur_kernel: i32,
    bytes: usize,
    last_used: AtomicU64,
}

struct MaskEntry {
    /// CV_8U-style bytes, length `width * height`.
    mask: Arc<Vec<u8>>,
    mod_time: SystemTime,
    bytes: usize,
    last_used: AtomicU64,
}

/// Packed/sparse template samples for a (template, mask, method) triple — the
/// per-match-attempt work `build_packed_template` + `SparseTemplate::from_packed`
/// would otherwise redo on every match attempt against the same icon.
struct PreparedEntry {
    prepared: Arc<PreparedTemplate>,
    icon_path: std::path::PathBuf,
    mask_path: Option<std::path::PathBuf>,
    tmpl_mod_time: SystemTime,
    mask_mod_time: Option<SystemTime>,
    blur_kernel: i32,
    bytes: usize,
    last_used: AtomicU64,
}

#[derive(Default)]
struct SearchCache {
    templates: HashMap<String, TemplateEntry>,
    image_masks: HashMap<String, MaskEntry>,
    prepared: HashMap<String, PreparedEntry>,
    bytes: usize,
}

impl SearchCache {
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
            EntryKind::Prepared => {
                if let Some(e) = self.prepared.remove(key) {
                    self.bytes = self.bytes.saturating_sub(e.bytes);
                }
            }
        }
    }

    /// Finds the globally least-recently-used entry across all maps.
    fn least_recently_used(&self) -> Option<(EntryKind, String)> {
        let mut best: Option<(u64, EntryKind, String)> = None;
        let consider =
            |best: &mut Option<(u64, EntryKind, String)>, tick: u64, kind: EntryKind, key: &str| {
                if best.as_ref().is_none_or(|(bt, _, _)| tick < *bt) {
                    *best = Some((tick, kind, key.to_string()));
                }
            };
        for (k, e) in &self.templates {
            consider(
                &mut best,
                e.last_used.load(Ordering::Relaxed),
                EntryKind::Template,
                k,
            );
        }
        for (k, e) in &self.image_masks {
            consider(
                &mut best,
                e.last_used.load(Ordering::Relaxed),
                EntryKind::Mask,
                k,
            );
        }
        for (k, e) in &self.prepared {
            consider(
                &mut best,
                e.last_used.load(Ordering::Relaxed),
                EntryKind::Prepared,
                k,
            );
        }
        best.map(|(_, kind, key)| (kind, key))
    }

    fn evict_until_fits(&mut self, extra: usize) {
        while self.bytes + extra > MAX_CACHE_BYTES {
            let Some((kind, key)) = self.least_recently_used() else {
                break;
            };
            self.remove_key(kind, &key);
        }
    }

    fn insert_template(&mut self, key: String, entry: TemplateEntry) {
        self.remove_key(EntryKind::Template, &key);
        self.evict_until_fits(entry.bytes);
        self.bytes += entry.bytes;
        self.templates.insert(key, entry);
    }

    fn insert_mask(&mut self, key: String, entry: MaskEntry) {
        self.remove_key(EntryKind::Mask, &key);
        self.evict_until_fits(entry.bytes);
        self.bytes += entry.bytes;
        self.image_masks.insert(key, entry);
    }

    fn insert_prepared(&mut self, key: String, entry: PreparedEntry) {
        self.remove_key(EntryKind::Prepared, &key);
        self.evict_until_fits(entry.bytes);
        self.bytes += entry.bytes;
        self.prepared.insert(key, entry);
    }

    fn clear(&mut self) {
        self.templates.clear();
        self.image_masks.clear();
        self.prepared.clear();
        self.bytes = 0;
    }
}

fn cache() -> &'static RwLock<SearchCache> {
    static CACHE: OnceLock<RwLock<SearchCache>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(SearchCache::default()))
}

/// Per-key gates so parallel variant jobs do not stampede the same cold load.
fn template_inflight() -> &'static Mutex<HashMap<String, Arc<Mutex<()>>>> {
    static GATES: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn mask_inflight() -> &'static Mutex<HashMap<String, Arc<Mutex<()>>>> {
    static GATES: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn prepared_inflight() -> &'static Mutex<HashMap<String, Arc<Mutex<()>>>> {
    static GATES: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn inflight_gate(
    map: &'static Mutex<HashMap<String, Arc<Mutex<()>>>>,
    key: &str,
) -> Arc<Mutex<()>> {
    let mut gates = map.lock();
    Arc::clone(
        gates
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
}

fn drop_inflight_gate(map: &'static Mutex<HashMap<String, Arc<Mutex<()>>>>, key: &str) {
    map.lock().remove(key);
}

fn template_cache_key(path: &Path, blur_kernel: i32) -> String {
    format!("{}\0{blur_kernel}", path.display())
}

fn mask_cache_key(path: &Path, rows: usize, cols: usize) -> String {
    format!("{}\0{rows}\0{cols}", path.display())
}

/// Prepared-template cache key: template identity (path + blur) is the same as
/// [`template_cache_key`]; mask path and match method also select distinct packing.
fn prepared_cache_key(
    icon_path: &Path,
    blur_kernel: i32,
    mask_path: Option<&Path>,
    method: MatchMethod,
) -> String {
    let mask_part = mask_path.map(|p| p.display().to_string());
    format!(
        "{}\0{blur_kernel}\0{:?}\0{}",
        icon_path.display(),
        method,
        mask_part.as_deref().unwrap_or("")
    )
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Clears all cached templates, masks, and prepared templates (call after a macro finishes).
pub fn clear_search_cache() {
    cache().write().clear();
    template_inflight().lock().clear();
    mask_inflight().lock().clear();
    prepared_inflight().lock().clear();
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
    let prepared_keys: Vec<String> = guard
        .prepared
        .iter()
        .filter(|(_, e)| e.icon_path.starts_with(icon_prefix))
        .map(|(k, _)| k.clone())
        .collect();
    for key in prepared_keys {
        guard.remove_key(EntryKind::Prepared, &key);
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
    let prepared_keys: Vec<String> = guard
        .prepared
        .iter()
        .filter(|(_, e)| {
            e.mask_path
                .as_deref()
                .is_some_and(|p| p.starts_with(mask_prefix))
        })
        .map(|(k, _)| k.clone())
        .collect();
    for key in prepared_keys {
        guard.remove_key(EntryKind::Prepared, &key);
    }
}

fn template_cache_hit(key: &str, mod_time: SystemTime, blur_kernel: i32) -> Option<Arc<ImageBuf>> {
    let guard = cache().read();
    let entry = guard.templates.get(key)?;
    if entry.mod_time == mod_time && entry.blur_kernel == blur_kernel {
        // Stamp recency through the atomic field only: no write-lock upgrade needed.
        entry.last_used.store(next_tick(), Ordering::Relaxed);
        Some(Arc::clone(&entry.blurred))
    } else {
        None
    }
}

fn mask_cache_hit(key: &str, mod_time: SystemTime) -> Option<Arc<Vec<u8>>> {
    let guard = cache().read();
    let entry = guard.image_masks.get(key)?;
    if entry.mod_time == mod_time {
        entry.last_used.store(next_tick(), Ordering::Relaxed);
        Some(Arc::clone(&entry.mask))
    } else {
        None
    }
}

/// Load (or reuse) a blurred template for `icon_path` at `blur_kernel`.
pub fn get_cached_blurred_template(
    icon_path: &Path,
    blur_kernel: i32,
) -> Result<Arc<ImageBuf>, String> {
    let mod_time =
        file_mtime(icon_path).ok_or_else(|| format!("stat {}: missing", icon_path.display()))?;
    let key = template_cache_key(icon_path, blur_kernel);

    if let Some(hit) = template_cache_hit(&key, mod_time, blur_kernel) {
        return Ok(hit);
    }

    let gate = inflight_gate(template_inflight(), &key);
    let _busy = gate.lock();
    // Another variant may have filled the cache while we waited.
    if let Some(hit) = template_cache_hit(&key, mod_time, blur_kernel) {
        drop(_busy);
        drop_inflight_gate(template_inflight(), &key);
        return Ok(hit);
    }

    let raw = load_rgb_image(icon_path)?;
    let blurred = Arc::new(
        blur_image_owned(raw, blur_kernel)
            .map_err(|e| format!("blur {}: {e}", icon_path.display()))?,
    );
    let bytes = blurred.data.len();

    cache().write().insert_template(
        key.clone(),
        TemplateEntry {
            blurred: Arc::clone(&blurred),
            mod_time,
            blur_kernel,
            bytes,
            last_used: AtomicU64::new(next_tick()),
        },
    );
    drop(_busy);
    drop_inflight_gate(template_inflight(), &key);
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

    if let Some(hit) = mask_cache_hit(&key, mod_time) {
        return Some(hit);
    }

    let gate = inflight_gate(mask_inflight(), &key);
    let _busy = gate.lock();
    if let Some(hit) = mask_cache_hit(&key, mod_time) {
        drop(_busy);
        drop_inflight_gate(mask_inflight(), &key);
        return Some(hit);
    }

    let loaded = load_rgb_image(mask_path).ok()?;
    let resized = resize_mask(&loaded, template_cols, template_rows);
    let mask = Arc::new(mask_as_u8(&resized));
    let bytes = mask.len();

    cache().write().insert_mask(
        key.clone(),
        MaskEntry {
            mask: Arc::clone(&mask),
            mod_time,
            bytes,
            last_used: AtomicU64::new(next_tick()),
        },
    );
    drop(_busy);
    drop_inflight_gate(mask_inflight(), &key);
    Some(mask)
}

fn prepared_cache_hit(
    key: &str,
    tmpl_mod_time: SystemTime,
    mask_mod_time: Option<SystemTime>,
    blur_kernel: i32,
) -> Option<Arc<PreparedTemplate>> {
    let guard = cache().read();
    let entry = guard.prepared.get(key)?;
    if entry.tmpl_mod_time == tmpl_mod_time
        && entry.mask_mod_time == mask_mod_time
        && entry.blur_kernel == blur_kernel
    {
        entry.last_used.store(next_tick(), Ordering::Relaxed);
        Some(Arc::clone(&entry.prepared))
    } else {
        None
    }
}

/// Load (or reuse) the packed/sparse template for `icon_path` at `blur_kernel`, masked
/// by the (already resized) `mask` from `mask_path` if any, for `method`.
///
/// Caches `build_packed_template` + `SparseTemplate::from_packed` alongside the blurred
/// template cache, so repeated match attempts against the same icon (wait/repeat loops)
/// skip re-packing.
pub fn get_cached_prepared_template(
    icon_path: &Path,
    blur_kernel: i32,
    template: &ImageBuf,
    mask_path: Option<&Path>,
    mask: Option<&[u8]>,
    method: MatchMethod,
) -> Result<Arc<PreparedTemplate>, String> {
    let tmpl_mod_time =
        file_mtime(icon_path).ok_or_else(|| format!("stat {}: missing", icon_path.display()))?;
    let mask_mod_time = mask_path.and_then(file_mtime);
    let key = prepared_cache_key(icon_path, blur_kernel, mask_path, method);

    if let Some(hit) = prepared_cache_hit(&key, tmpl_mod_time, mask_mod_time, blur_kernel) {
        return Ok(hit);
    }

    let gate = inflight_gate(prepared_inflight(), &key);
    let _busy = gate.lock();
    if let Some(hit) = prepared_cache_hit(&key, tmpl_mod_time, mask_mod_time, blur_kernel) {
        drop(_busy);
        drop_inflight_gate(prepared_inflight(), &key);
        return Ok(hit);
    }

    let prepared = Arc::new(
        prepare_template(template, mask, method)
            .map_err(|e| format!("prepare template {}: {e}", icon_path.display()))?,
    );
    let bytes = prepared.approx_bytes();

    cache().write().insert_prepared(
        key.clone(),
        PreparedEntry {
            prepared: Arc::clone(&prepared),
            icon_path: icon_path.to_path_buf(),
            mask_path: mask_path.map(Path::to_path_buf),
            tmpl_mod_time,
            mask_mod_time,
            blur_kernel,
            bytes,
            last_used: AtomicU64::new(next_tick()),
        },
    );
    drop(_busy);
    drop_inflight_gate(prepared_inflight(), &key);
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use rayon::prelude::*;
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

    #[test]
    fn parallel_misses_single_flight() {
        with_search_cache_test_lock(|| {
            reset_search_cache_for_testing();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("icon.png");
            write_rgb(&path, 16, 16, [40, 50, 60]);
            let k = search_blur_kernel(1);
            let paths: Vec<_> = (0..8).map(|_| path.clone()).collect();
            let results: Vec<_> = paths
                .into_par_iter()
                .map(|p| get_cached_blurred_template(&p, k).unwrap())
                .collect();
            let first = &results[0];
            assert!(results.iter().all(|a| Arc::ptr_eq(a, first)));
        });
    }
}
