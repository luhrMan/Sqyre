//! Per-action execution log sink (keyed by [`ActionId`]).
//!
//! Entries are chronological: text lines, shared pipeline images, and browseable
//! [`ActionLogEntry::ItemPipeline`] groups (image-search items with steps + finds).

use parking_lot::Mutex;
use sqyre_domain::ActionId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Max entries retained per action (oldest dropped).
pub const MAX_ENTRIES_PER_ACTION: usize = 200;

/// RGBA image stored in an action log.
#[derive(Clone, Debug)]
pub struct LogImage {
    pub label: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, length `width * height * 4`.
    pub pixels: Arc<Vec<u8>>,
}

impl LogImage {
    /// Build from packed RGBA8 (`pixels.len() == width * height * 4`).
    pub fn from_rgba(label: String, width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        if pixels.len() != width as usize * height as usize * 4 {
            return None;
        }
        Some(Self {
            label,
            width,
            height,
            pixels: Arc::new(pixels),
        })
    }
}

/// One chronologically ordered log item for an action.
#[derive(Clone, Debug)]
pub enum ActionLogEntry {
    Text(String),
    Image(LogImage),
    /// Clickable item card: thumbnail + detail steps / find locations.
    ItemPipeline {
        title: String,
        summary: String,
        thumbnail: LogImage,
        steps: Vec<LogImage>,
        details: Vec<String>,
    },
}

impl ActionLogEntry {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s.as_str()),
            Self::Image(_) | Self::ItemPipeline { .. } => None,
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    pub fn is_item_pipeline(&self) -> bool {
        matches!(self, Self::ItemPipeline { .. })
    }
}

/// Receives log lines / images tagged with the action that produced them.
pub trait ActionLogger: Send + Sync {
    fn log(&self, action_id: ActionId, message: String);

    /// When false, [`Self::log_image`] / [`Self::log_item_pipeline`] are no-ops
    /// and callers may skip building debug overlays.
    fn log_images_enabled(&self) -> bool {
        false
    }

    fn log_image(&self, action_id: ActionId, image: &LogImage) {
        let _ = (action_id, image);
    }

    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &LogImage,
        steps: &[LogImage],
        details: Vec<String>,
    ) {
        let _ = (action_id, title, summary, thumbnail, steps, details);
    }
}

/// Thread-safe per-action entry buffer for the UI.
///
/// Images are in-memory only (no `images/meta` dump). Enable via
/// [`Self::set_log_images`] — matches the user "Log Meta Images" preference.
#[derive(Clone)]
pub struct SharedActionLog {
    inner: Arc<Mutex<HashMap<ActionId, Vec<ActionLogEntry>>>>,
    log_images: Arc<AtomicBool>,
}

impl Default for SharedActionLog {
    fn default() -> Self {
        // Enabled by default so unit tests that assert images need no setup;
        // the app sets this from `UserSettings::save_meta_images` (default off).
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            log_images: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl SharedActionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_log_images(&self, enabled: bool) {
        self.log_images.store(enabled, Ordering::SeqCst);
    }

    pub fn log_images_enabled(&self) -> bool {
        self.log_images.load(Ordering::SeqCst)
    }

    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    /// Snapshot of the entries logged for `action_id`.
    pub fn entries_for(&self, action_id: ActionId) -> Vec<ActionLogEntry> {
        self.inner
            .lock()
            .get(&action_id)
            .cloned()
            .unwrap_or_default()
    }
}

/// Render a snapshot as text lines — convenient for tests and Copy.
pub fn lines_for(entries: &[ActionLogEntry]) -> Vec<String> {
    let mut out = Vec::new();
    for e in entries {
        match e {
            ActionLogEntry::Text(s) => out.push(s.clone()),
            ActionLogEntry::Image(img) => out.push(format!("[image] {}", img.label)),
            ActionLogEntry::ItemPipeline {
                title,
                summary,
                details,
                ..
            } => {
                out.push(format!("[item] {title} — {summary}"));
                out.extend(details.iter().cloned());
            }
        }
    }
    out
}

impl ActionLogger for SharedActionLog {
    fn log(&self, action_id: ActionId, message: String) {
        push_entry(&self.inner, action_id, ActionLogEntry::Text(message));
    }

    fn log_images_enabled(&self) -> bool {
        SharedActionLog::log_images_enabled(self)
    }

    fn log_image(&self, action_id: ActionId, image: &LogImage) {
        if !self.log_images_enabled() {
            return;
        }
        replace_or_push_image(&self.inner, action_id, image.clone());
    }

    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &LogImage,
        steps: &[LogImage],
        details: Vec<String>,
    ) {
        if !self.log_images_enabled() {
            return;
        }
        push_entry(
            &self.inner,
            action_id,
            ActionLogEntry::ItemPipeline {
                title,
                summary,
                thumbnail: thumbnail.clone(),
                steps: steps.to_vec(),
                details,
            },
        );
    }
}

fn push_entry(
    inner: &Mutex<HashMap<ActionId, Vec<ActionLogEntry>>>,
    action_id: ActionId,
    entry: ActionLogEntry,
) {
    let mut map = inner.lock();
    let entries = map.entry(action_id).or_default();
    push_capped(entries, entry);
}

fn push_capped(entries: &mut Vec<ActionLogEntry>, entry: ActionLogEntry) {
    entries.push(entry);
    if entries.len() > MAX_ENTRIES_PER_ACTION {
        let drop = entries.len() - MAX_ENTRIES_PER_ACTION;
        entries.drain(0..drop);
    }
}

/// Stable identity for wait-until-found capture shots so retries update one slot
/// instead of stacking a frozen first frame above newer copies.
fn live_capture_slot_key(label: &str) -> Option<&'static str> {
    label
        .starts_with("1. Capture (search area)")
        .then_some("1. Capture (search area)")
}

fn replace_or_push_image(
    inner: &Mutex<HashMap<ActionId, Vec<ActionLogEntry>>>,
    action_id: ActionId,
    img: LogImage,
) {
    let mut map = inner.lock();
    let entries = map.entry(action_id).or_default();
    if let Some(key) = live_capture_slot_key(&img.label) {
        if let Some(slot) = entries.iter_mut().rev().find(|e| {
            matches!(
                e,
                ActionLogEntry::Image(existing) if live_capture_slot_key(&existing.label) == Some(key)
            )
        }) {
            *slot = ActionLogEntry::Image(img);
            return;
        }
    }
    push_capped(entries, ActionLogEntry::Image(img));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(label: &str, fill: u8) -> LogImage {
        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            pixels.extend_from_slice(&[fill, fill, fill, 255]);
        }
        LogImage::from_rgba(label.to_string(), 4, 4, pixels).expect("solid log image")
    }

    #[test]
    fn caps_entries_per_action() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        for i in 0..(MAX_ENTRIES_PER_ACTION + 50) {
            log.log(id, format!("line-{i}"));
        }
        let lines = lines_for(&log.entries_for(id));
        assert_eq!(lines.len(), MAX_ENTRIES_PER_ACTION);
        assert_eq!(lines[0], format!("line-{}", 50));
    }

    #[test]
    fn isolates_actions_and_clear_wipes_all() {
        let log = SharedActionLog::new();
        let a = ActionId::new();
        let b = ActionId::new();
        log.log(a, "from-a".into());
        log.log(b, "from-b".into());
        assert_eq!(lines_for(&log.entries_for(a)), vec!["from-a".to_string()]);
        log.clear();
        assert!(log.entries_for(a).is_empty());
    }

    #[test]
    fn log_images_disabled_skips_image_entries() {
        let log = SharedActionLog::new();
        log.set_log_images(false);
        let id = ActionId::new();
        log.log(id, "start".into());
        log.log_image(id, &solid("capture", 128));
        let entries = log.entries_for(id);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn wait_until_found_replaces_capture_image_slot() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        let a = solid("1. Capture (search area)", 10);
        let b = solid("1. Capture (search area)", 200);
        log.log_image(id, &a);
        log.log_image(id, &b);
        let entries = log.entries_for(id);
        let images: Vec<&LogImage> = entries
            .iter()
            .filter_map(|e| match e {
                ActionLogEntry::Image(img) => Some(img),
                _ => None,
            })
            .collect();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].label, "1. Capture (search area)");
        assert_eq!(images[0].pixels[0], 200);
    }
}
