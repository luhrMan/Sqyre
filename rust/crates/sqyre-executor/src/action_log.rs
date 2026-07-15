//! Per-action execution log sink (keyed by [`ActionId`]).
//!
//! Entries are chronological: text lines, shared pipeline images, and browseable
//! [`ActionLogEntry::ItemPipeline`] groups (image-search items with steps + finds).

use sqyre_domain::ActionId;
use sqyre_match::ImageBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

    fn log_image(&self, action_id: ActionId, label: String, image: &ImageBuf) {
        let _ = (action_id, label, image);
    }

    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &ImageBuf,
        steps: &[(String, ImageBuf)],
        details: Vec<String>,
    ) {
        let _ = (action_id, title, summary, thumbnail, steps, details);
    }
}

/// Thread-safe per-action entry buffer for the UI.
#[derive(Clone, Default)]
pub struct SharedActionLog {
    inner: Arc<Mutex<HashMap<ActionId, Vec<ActionLogEntry>>>>,
}

impl SharedActionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    pub fn entries_for(&self, action_id: ActionId) -> Vec<ActionLogEntry> {
        self.inner
            .lock()
            .unwrap()
            .get(&action_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Text-only lines (skips images) — convenient for tests and Copy.
    /// Item pipelines contribute their title, summary, and detail lines.
    pub fn lines_for(&self, action_id: ActionId) -> Vec<String> {
        let mut out = Vec::new();
        for e in self.entries_for(action_id) {
            match e {
                ActionLogEntry::Text(s) => out.push(s),
                ActionLogEntry::Image(img) => out.push(format!("[image] {}", img.label)),
                ActionLogEntry::ItemPipeline {
                    title,
                    summary,
                    details,
                    ..
                } => {
                    out.push(format!("[item] {title} — {summary}"));
                    out.extend(details);
                }
            }
        }
        out
    }
}

impl ActionLogger for SharedActionLog {
    fn log(&self, action_id: ActionId, message: String) {
        push_entry(
            &self.inner,
            action_id,
            ActionLogEntry::Text(message),
        );
    }

    fn log_image(&self, action_id: ActionId, label: String, image: &ImageBuf) {
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
        steps: &[(String, ImageBuf)],
        details: Vec<String>,
    ) {
        let Some(thumbnail) = image_buf_to_log_image(format!("Item — {title}"), thumbnail) else {
            return;
        };
        let steps: Vec<LogImage> = steps
            .iter()
            .filter_map(|(label, img)| image_buf_to_log_image(label.clone(), img))
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
    let mut map = inner.lock().unwrap();
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

/// Draw axis-aligned rectangles on a 3-channel RGB buffer (clips to bounds).
pub fn draw_rect_rgb(img: &mut ImageBuf, x0: i32, y0: i32, x1: i32, y1: i32, rgb: [u8; 3]) {
    if img.channels != 3 {
        return;
    }
    let w = img.width as i32;
    let h = img.height as i32;
    let left = x0.min(x1).clamp(0, w - 1);
    let right = x0.max(x1).clamp(0, w - 1);
    let top = y0.min(y1).clamp(0, h - 1);
    let bottom = y0.max(y1).clamp(0, h - 1);
    for x in left..=right {
        put_rgb(img, x, top, rgb);
        put_rgb(img, x, bottom, rgb);
    }
    for y in top..=bottom {
        put_rgb(img, left, y, rgb);
        put_rgb(img, right, y, rgb);
    }
}

fn put_rgb(img: &mut ImageBuf, x: i32, y: i32, rgb: [u8; 3]) {
    if x < 0 || y < 0 || x as usize >= img.width || y as usize >= img.height {
        return;
    }
    let i = img.pixel_offset(x as usize, y as usize);
    img.data[i] = rgb[0];
    img.data[i + 1] = rgb[1];
    img.data[i + 2] = rgb[2];
}

/// Crop a padded region around a template match and draw the match box (for logs).
pub fn crop_match_preview(
    search: &ImageBuf,
    x: i32,
    y: i32,
    tw: i32,
    th: i32,
    pad: i32,
) -> Option<ImageBuf> {
    if search.channels != 3 || tw <= 0 || th <= 0 {
        return None;
    }
    let w = search.width as i32;
    let h = search.height as i32;
    let x0 = (x - pad).max(0);
    let y0 = (y - pad).max(0);
    let x1 = (x + tw + pad).min(w);
    let y1 = (y + th + pad).min(h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let cw = (x1 - x0) as usize;
    let ch = (y1 - y0) as usize;
    let mut out = ImageBuf::new(cw, ch, 3, 0);
    for py in 0..ch {
        for px in 0..cw {
            let si = search.pixel_offset((x0 as usize) + px, (y0 as usize) + py);
            let di = out.pixel_offset(px, py);
            out.data[di..di + 3].copy_from_slice(&search.data[si..si + 3]);
        }
    }
    draw_rect_rgb(
        &mut out,
        x - x0,
        y - y0,
        x - x0 + tw - 1,
        y - y0 + th - 1,
        [255, 40, 40],
    );
    Some(out)
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
        let lines = log.lines_for(id);
        assert_eq!(lines.len(), MAX_ENTRIES_PER_ACTION);
        assert_eq!(lines[0], format!("line-{}", 50));
        assert_eq!(
            lines.last().unwrap(),
            &format!("line-{}", MAX_ENTRIES_PER_ACTION + 49)
        );
    }

    #[test]
    fn isolates_actions_and_clear_wipes_all() {
        let log = SharedActionLog::new();
        let a = ActionId::new();
        let b = ActionId::new();
        log.log(a, "from-a".into());
        log.log(b, "from-b".into());
        assert_eq!(log.lines_for(a), vec!["from-a".to_string()]);
        assert_eq!(log.lines_for(b), vec!["from-b".to_string()]);
        log.clear();
        assert!(log.lines_for(a).is_empty());
        assert!(log.lines_for(b).is_empty());
    }

    #[test]
    fn images_interleave_with_text_chronologically() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        log.log(id, "start".into());
        let img = ImageBuf::new(4, 4, 3, 128);
        log.log_image(id, "capture".into(), &img);
        log.log(id, "done".into());
        let entries = log.entries_for(id);
        assert_eq!(entries.len(), 3);
        assert!(matches!(&entries[0], ActionLogEntry::Text(s) if s == "start"));
        assert!(matches!(
            &entries[1],
            ActionLogEntry::Image(LogImage { label, .. }) if label == "capture"
        ));
        assert!(matches!(&entries[2], ActionLogEntry::Text(s) if s == "done"));
    }

    #[test]
    fn item_pipeline_stores_steps_and_details() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        let thumb = ImageBuf::new(4, 4, 3, 200);
        let step = ImageBuf::new(8, 8, 3, 100);
        log.log_item_pipeline(
            id,
            "Sword".into(),
            "1 match".into(),
            &thumb,
            &[("Where found".into(), step)],
            vec!["Found at (1, 2)".into()],
        );
        let entries = log.entries_for(id);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            ActionLogEntry::ItemPipeline {
                title,
                summary,
                steps,
                details,
                ..
            } => {
                assert_eq!(title, "Sword");
                assert_eq!(summary, "1 match");
                assert_eq!(steps.len(), 1);
                assert_eq!(steps[0].label, "Where found");
                assert_eq!(details, &vec!["Found at (1, 2)".to_string()]);
            }
            other => panic!("expected ItemPipeline, got {other:?}"),
        }
        let lines = log.lines_for(id);
        assert!(lines[0].contains("Sword"));
        assert!(lines.iter().any(|l| l.contains("Found at")));
    }

    #[test]
    fn crop_match_preview_draws_box() {
        let search = ImageBuf::new(40, 30, 3, 80);
        let crop = crop_match_preview(&search, 10, 8, 6, 4, 4).unwrap();
        assert_eq!(crop.width, 14); // 6 + 2*4
        assert_eq!(crop.height, 12); // 4 + 2*4
    }
}
