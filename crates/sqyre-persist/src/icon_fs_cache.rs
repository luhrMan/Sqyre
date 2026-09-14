//! Process-global caches for icon directory listings and mask file presence.
//!
//! Image search, overlay gates, and UI grids call [`ProgramCatalog::variant_paths`]
//! / [`ProgramCatalog::mask_path`] repeatedly (wait/repeat polls). Scanning the
//! icons dir or `stat`-ing the same mask every time is pure I/O waste; trust
//! listings until the directory mtime changes or an explicit invalidate.

use parking_lot::Mutex;
use sqyre_domain::PROGRAM_DELIMITER;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

struct DirListing {
    mtime: Option<SystemTime>,
    /// Item name → sorted icon paths (legacy `item.png` + `item~variant.png`).
    by_item: HashMap<String, Vec<PathBuf>>,
}

#[derive(Default)]
struct IconFsCache {
    dirs: HashMap<PathBuf, DirListing>,
    /// Mask path → exists on disk (trusted until invalidate).
    masks: HashMap<PathBuf, bool>,
}

fn cache() -> &'static Mutex<IconFsCache> {
    static CACHE: OnceLock<Mutex<IconFsCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(IconFsCache::default()))
}

fn dir_mtime(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir).ok()?.modified().ok()
}

fn item_key_from_icon_filename(name: &str) -> Option<&str> {
    let stem = name.strip_suffix(".png")?;
    Some(
        stem.split_once(PROGRAM_DELIMITER)
            .map(|(item, _)| item)
            .unwrap_or(stem),
    )
}

fn scan_icons_dir(dir: &Path) -> HashMap<String, Vec<PathBuf>> {
    let mut by_item: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return by_item;
    };
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(item) = item_key_from_icon_filename(name.as_ref()) else {
            continue;
        };
        by_item
            .entry(item.to_string())
            .or_default()
            .push(entry.path());
    }
    for paths in by_item.values_mut() {
        paths.sort();
    }
    by_item
}

/// Sorted icon variant paths for `item` under `icons_dir` (one `read_dir` per dir mtime).
pub fn cached_variant_paths(icons_dir: &Path, item: &str) -> Vec<PathBuf> {
    let mtime = dir_mtime(icons_dir);
    let mut guard = cache().lock();
    if let Some(listing) = guard.dirs.get(icons_dir) {
        if listing.mtime == mtime {
            return listing.by_item.get(item).cloned().unwrap_or_default();
        }
    }
    let by_item = scan_icons_dir(icons_dir);
    let paths = by_item.get(item).cloned().unwrap_or_default();
    guard.dirs.insert(
        icons_dir.to_path_buf(),
        DirListing { mtime, by_item },
    );
    paths
}

/// `Some(path)` when the mask file exists; presence is cached until invalidate.
pub fn cached_mask_if_exists(path: PathBuf) -> Option<PathBuf> {
    let mut guard = cache().lock();
    if let Some(&exists) = guard.masks.get(&path) {
        return exists.then_some(path);
    }
    let exists = path.is_file();
    guard.masks.insert(path.clone(), exists);
    exists.then_some(path)
}

/// Drop cached listings / mask presence affected by `prefix`.
///
/// `prefix` may be an icons/masks directory, an item path prefix
/// (`…/icons/Prog/Sword`), or a single file path.
pub fn invalidate_icon_fs_cache_under(prefix: &Path) {
    let mut guard = cache().lock();
    guard.dirs.retain(|dir, _| !dir.starts_with(prefix) && !prefix.starts_with(dir));
    guard.masks.retain(|path, _| !path.starts_with(prefix));
}

/// Clear every listing and mask-presence entry (tests / full reload).
pub fn clear_icon_fs_cache() {
    let mut guard = cache().lock();
    guard.dirs.clear();
    guard.masks.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn touch_png(path: &Path) {
        std::fs::write(path, b"not-a-real-png").unwrap();
    }

    #[test]
    fn variant_paths_reuse_dir_scan() {
        clear_icon_fs_cache();
        let dir = tempdir().unwrap();
        let icons = dir.path().join("icons");
        std::fs::create_dir_all(&icons).unwrap();
        touch_png(&icons.join("Sword.png"));
        touch_png(&icons.join("Sword~Alt.png"));
        touch_png(&icons.join("Potion.png"));

        let a = cached_variant_paths(&icons, "Sword");
        let b = cached_variant_paths(&icons, "Sword");
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
        assert_eq!(cached_variant_paths(&icons, "Potion").len(), 1);
    }

    #[test]
    fn invalidate_refreshes_after_add() {
        clear_icon_fs_cache();
        let dir = tempdir().unwrap();
        let icons = dir.path().join("icons");
        std::fs::create_dir_all(&icons).unwrap();
        touch_png(&icons.join("Sword.png"));
        assert_eq!(cached_variant_paths(&icons, "Sword").len(), 1);

        touch_png(&icons.join("Sword~New.png"));
        invalidate_icon_fs_cache_under(&icons);
        assert_eq!(cached_variant_paths(&icons, "Sword").len(), 2);
    }

    #[test]
    fn mask_presence_cached_until_invalidate() {
        clear_icon_fs_cache();
        let dir = tempdir().unwrap();
        let path = dir.path().join("mask.png");
        assert!(cached_mask_if_exists(path.clone()).is_none());
        touch_png(&path);
        // Negative was cached.
        assert!(cached_mask_if_exists(path.clone()).is_none());
        invalidate_icon_fs_cache_under(dir.path());
        assert!(cached_mask_if_exists(path).is_some());
    }

    #[test]
    fn item_prefix_invalidates_parent_dir() {
        clear_icon_fs_cache();
        let dir = tempdir().unwrap();
        let icons = dir.path().join("icons").join("Prog");
        std::fs::create_dir_all(&icons).unwrap();
        touch_png(&icons.join("Sword.png"));
        assert_eq!(cached_variant_paths(&icons, "Sword").len(), 1);
        touch_png(&icons.join("Sword~B.png"));
        invalidate_icon_fs_cache_under(&icons.join("Sword"));
        assert_eq!(cached_variant_paths(&icons, "Sword").len(), 2);
    }
}
