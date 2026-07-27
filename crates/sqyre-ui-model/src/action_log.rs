//! Per-action execution log sink (keyed by [`ActionId`]).
//!
//! Entries are chronological: text lines, shared pipeline images, and browseable
//! [`ActionLogEntry::ItemPipeline`] groups (image-search items with steps + finds).

use parking_lot::Mutex;
use sqyre_domain::ActionId;
use sqyre_match::ImageBuf;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Max entries retained per action (oldest dropped).
pub const MAX_ENTRIES_PER_ACTION: usize = 200;

/// Cap long edge when storing log images (keeps UI memory bounded).
const LOG_IMAGE_MAX_EDGE: usize = 640;

/// RGBA image stored in an action log.
#[derive(Clone, Debug)]
pub struct LogImage {
    pub label: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, length `width * height * 4`.
    pub pixels: Arc<Vec<u8>>,
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

    fn log_image(&self, action_id: ActionId, label: String, image: &ImageBuf) {
        let _ = (action_id, label, image);
    }

    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &ImageBuf,
        steps: &[(&str, &ImageBuf)],
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

    fn log_image(&self, action_id: ActionId, label: String, image: &ImageBuf) {
        if !self.log_images_enabled() {
            return;
        }
        let Some(img) = image_buf_to_log_image(label, image) else {
            return;
        };
        push_entry(&self.inner, action_id, ActionLogEntry::Image(img));
    }

    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &ImageBuf,
        steps: &[(&str, &ImageBuf)],
        details: Vec<String>,
    ) {
        if !self.log_images_enabled() {
            return;
        }
        let Some(thumbnail) = image_buf_to_log_image(format!("Item — {title}"), thumbnail) else {
            return;
        };
        let steps: Vec<LogImage> = steps
            .iter()
            .filter_map(|(label, img)| image_buf_to_log_image((*label).to_string(), img))
            .collect();
        push_entry(
            &self.inner,
            action_id,
            ActionLogEntry::ItemPipeline {
                title,
                summary,
                thumbnail,
                steps,
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
    entries.push(entry);
    if entries.len() > MAX_ENTRIES_PER_ACTION {
        let drop = entries.len() - MAX_ENTRIES_PER_ACTION;
        entries.drain(0..drop);
    }
}

fn image_buf_to_log_image(label: String, image: &ImageBuf) -> Option<LogImage> {
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let scaled = downscale_for_log(image);
    let rgba = image_buf_to_rgba(&scaled);
    Some(LogImage {
        label,
        width: scaled.width as u32,
        height: scaled.height as u32,
        pixels: Arc::new(rgba),
    })
}

fn downscale_for_log(img: &ImageBuf) -> ImageBuf {
    let long = img.width.max(img.height);
    if long <= LOG_IMAGE_MAX_EDGE {
        return img.clone();
    }
    let scale = LOG_IMAGE_MAX_EDGE as f64 / long as f64;
    let nw = ((img.width as f64) * scale).round().max(1.0) as usize;
    let nh = ((img.height as f64) * scale).round().max(1.0) as usize;
    nearest_resize(img, nw, nh)
}

fn nearest_resize(img: &ImageBuf, nw: usize, nh: usize) -> ImageBuf {
    let ch = img.channels;
    let mut data = vec![0u8; nw * nh * ch];
    for y in 0..nh {
        let sy = (y * img.height / nh).min(img.height - 1);
        for x in 0..nw {
            let sx = (x * img.width / nw).min(img.width - 1);
            let si = img.pixel_offset(sx, sy);
            let di = (y * nw + x) * ch;
            data[di..di + ch].copy_from_slice(&img.data[si..si + ch]);
        }
    }
    ImageBuf::from_raw(nw, nh, ch, data)
}

fn image_buf_to_rgba(img: &ImageBuf) -> Vec<u8> {
    let n = img.width * img.height;
    let mut out = Vec::with_capacity(n * 4);
    match img.channels {
        1 => {
            for &v in &img.data {
                out.extend_from_slice(&[v, v, v, 255]);
            }
        }
        3 => {
            for i in 0..n {
                let o = i * 3;
                out.extend_from_slice(&[img.data[o], img.data[o + 1], img.data[o + 2], 255]);
            }
        }
        _ => {
            for _ in 0..n {
                out.extend_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let img = ImageBuf::new(4, 4, 3, 128);
        log.log_image(id, "capture".into(), &img);
        let entries = log.entries_for(id);
        assert_eq!(entries.len(), 1);
    }
}
