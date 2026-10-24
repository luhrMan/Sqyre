//! Per-action execution log sink (keyed by [`ActionId`]).
//!
//! Entries are chronological: text lines, shared pipeline images, and browseable
//! [`ActionLogEntry::ItemPipeline`] groups (image-search items with steps + finds).

use parking_lot::Mutex;
use sqyre_domain::ActionId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// Max entries retained per action (oldest dropped).
pub const MAX_ENTRIES_PER_ACTION: usize = 5000;

/// A labeled region inside a [`LogImage`] — e.g. every recognized OCR word
/// box, so the Logs zoom view can tooltip what was seen even outside target
/// finds.
#[derive(Clone, Debug)]
pub struct ImageAnnotation {
    /// Region in stored-image pixel coords `(left, top, right, bottom)`.
    pub rect: (i32, i32, i32, i32),
    /// What the region contains (the OCR'd word).
    pub label: String,
    /// Absolute desktop rect `(left, top, right, bottom)` — shown in the
    /// click-inspect tooltip.
    pub screen: (i32, i32, i32, i32),
}

/// RGBA image stored in an action log.
#[derive(Clone, Debug)]
pub struct LogImage {
    pub label: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, length `width * height * 4`.
    pub pixels: Arc<Vec<u8>>,
    /// Labeled regions the zoom view can click-inspect (OCR word boxes).
    pub annotations: Vec<ImageAnnotation>,
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
            annotations: Vec::new(),
        })
    }
}

/// One detected object's location for hover/click "where found" markers.
#[derive(Clone, Debug)]
pub struct LogFind {
    /// Short label shown on the interactive chip / row ("Find #1", word, "(x,y)").
    pub label: String,
    /// Optional longer description for the click-inspect tooltip — what was
    /// actually found (OCR text inside the box, matched pixel color, variant).
    pub detail: Option<String>,
    /// Absolute desktop rect `(left, top, right, bottom)` — pinned outline target.
    pub screen: (i32, i32, i32, i32),
    /// Composite image + rect for the in-window hover highlight; `None` when the
    /// find carries only screen coordinates.
    pub context: Option<FindContext>,
}

/// Where a [`LogFind`]'s in-window highlight lives, in image pixel coordinates
/// (post-log downscale).
#[derive(Clone, Debug)]
pub enum FindContext {
    /// Index into the parent [`ActionLogEntry::ItemPipeline`]'s `steps` — the
    /// image-search "Where found" composite. No extra pixels.
    Step {
        step: usize,
        rect: (i32, i32, i32, i32),
    },
    /// Standalone context image (OCR word boxes, pixel capture). Sharing the
    /// [`Arc`] with a displayed `Image` entry lets the UI stroke `rect` directly
    /// on the shown image via `pixels` pointer identity.
    Image {
        image: Arc<LogImage>,
        rect: (i32, i32, i32, i32),
    },
}

impl FindContext {
    /// Context image bytes when carried as a separate [`LogImage`].
    fn image(&self) -> Option<&Arc<LogImage>> {
        match self {
            Self::Image { image, .. } => Some(image),
            Self::Step { .. } => None,
        }
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
        finds: Vec<LogFind>,
    },
    /// Text line tied to one or more detected desktop regions (OCR hits, pixel
    /// finds) — hoverable / clickable "where found" markers.
    Finds {
        text: String,
        finds: Vec<LogFind>,
    },
    /// Section boundary (detection attempt header). Entries logged after it
    /// belong to the section until the next `Section`; the Logs window renders
    /// each section as a collapsible group.
    Section(String),
}

impl ActionLogEntry {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) | Self::Finds { text: s, .. } | Self::Section(s) => Some(s.as_str()),
            Self::Image(_) | Self::ItemPipeline { .. } => None,
        }
    }

    pub fn is_section(&self) -> bool {
        matches!(self, Self::Section(_))
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    pub fn is_item_pipeline(&self) -> bool {
        matches!(self, Self::ItemPipeline { .. })
    }

    pub fn is_finds(&self) -> bool {
        matches!(self, Self::Finds { .. })
    }

    pub fn finds(&self) -> &[LogFind] {
        match self {
            Self::Finds { finds, .. } | Self::ItemPipeline { finds, .. } => finds,
            Self::Text(_) | Self::Image(_) | Self::Section(_) => &[],
        }
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

    /// When false, callers must skip diagnostics whose *cost is in producing the
    /// value*, not in logging it — frame checksums, full OCR text, variable
    /// values. Guard the computation, not just the [`Self::log`] call.
    fn log_verbose_enabled(&self) -> bool {
        false
    }

    fn log_image(&self, action_id: ActionId, image: &LogImage) {
        let _ = (action_id, image);
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "pipeline payload: title, summary, images, detail lines, and find markers"
    )]
    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &LogImage,
        steps: &[LogImage],
        details: Vec<String>,
        finds: Vec<LogFind>,
    ) {
        let _ = (action_id, title, summary, thumbnail, steps, details, finds);
    }

    /// Log a text line carrying per-detection find markers.
    fn log_finds(&self, action_id: ActionId, text: String, finds: Vec<LogFind>) {
        let _ = (action_id, text, finds);
    }

    /// Start a new collapsible log section (one detection attempt); entries
    /// logged after it belong to the section until the next [`Self::log_section`].
    fn log_section(&self, action_id: ActionId, label: String) {
        let _ = (action_id, label);
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
    log_verbose: Arc<AtomicBool>,
    max_entries: Arc<AtomicUsize>,
}

impl Default for SharedActionLog {
    fn default() -> Self {
        // Both enabled by default so unit tests that assert images / diagnostic
        // lines need no setup; the app sets them from
        // `UserSettings::save_meta_images` (default off).
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            log_images: Arc::new(AtomicBool::new(true)),
            log_verbose: Arc::new(AtomicBool::new(true)),
            max_entries: Arc::new(AtomicUsize::new(MAX_ENTRIES_PER_ACTION)),
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

    pub fn set_log_verbose(&self, enabled: bool) {
        self.log_verbose.store(enabled, Ordering::SeqCst);
    }

    pub fn log_verbose_enabled(&self) -> bool {
        self.log_verbose.load(Ordering::SeqCst)
    }

    /// Per-action entry cap (from `UserSettings::action_log_max_entries`).
    /// Shrinking drains the oldest over-cap entries immediately.
    pub fn set_max_entries(&self, max: usize) {
        let max = max.max(1);
        // Called every frame by the UI sync — skip the lock when unchanged.
        if self.max_entries.swap(max, Ordering::SeqCst) == max {
            return;
        }
        let mut map = self.inner.lock();
        for entries in map.values_mut() {
            if entries.len() > max {
                entries.drain(0..entries.len() - max);
            }
        }
    }

    fn entry_cap(&self) -> usize {
        self.max_entries.load(Ordering::SeqCst)
    }

    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    /// Remove one action's entries (per-action Clear in the Logs window).
    pub fn clear_for(&self, action_id: ActionId) {
        self.inner.lock().remove(&action_id);
    }

    /// Total RGBA bytes retained in log images (for `SQYRE_MEM` samples).
    ///
    /// Pixel buffers shared between entries (a find context `Arc` sharing the
    /// sibling `Image` / pipeline step it was built from) count once.
    pub fn image_bytes(&self) -> usize {
        let map = self.inner.lock();
        let mut seen = std::collections::HashSet::new();
        map.values()
            .map(|entries| entries_image_bytes(entries, &mut seen))
            .sum()
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

fn entries_image_bytes(
    entries: &[ActionLogEntry],
    seen: &mut std::collections::HashSet<*const Vec<u8>>,
) -> usize {
    entries.iter().map(|e| entry_image_bytes(e, seen)).sum()
}

fn counted_pixels(
    seen: &mut std::collections::HashSet<*const Vec<u8>>,
    pixels: &Arc<Vec<u8>>,
) -> usize {
    if seen.insert(Arc::as_ptr(pixels)) {
        pixels.len()
    } else {
        0
    }
}

fn entry_image_bytes(
    entry: &ActionLogEntry,
    seen: &mut std::collections::HashSet<*const Vec<u8>>,
) -> usize {
    match entry {
        ActionLogEntry::Text(_) | ActionLogEntry::Section(_) => 0,
        ActionLogEntry::Image(img) => counted_pixels(seen, &img.pixels),
        ActionLogEntry::ItemPipeline {
            thumbnail,
            steps,
            finds,
            ..
        } => {
            counted_pixels(seen, &thumbnail.pixels)
                + steps
                    .iter()
                    .map(|s| counted_pixels(seen, &s.pixels))
                    .sum::<usize>()
                + finds_image_bytes(finds, seen)
        }
        ActionLogEntry::Finds { finds, .. } => finds_image_bytes(finds, seen),
    }
}

fn finds_image_bytes(
    finds: &[LogFind],
    seen: &mut std::collections::HashSet<*const Vec<u8>>,
) -> usize {
    finds
        .iter()
        .filter_map(|f| f.context.as_ref())
        .filter_map(|c| c.image())
        .map(|img| counted_pixels(seen, &img.pixels))
        .sum()
}

/// Render a snapshot as text lines — convenient for tests and Copy.
pub fn lines_for(entries: &[ActionLogEntry]) -> Vec<String> {
    let mut out = Vec::new();
    for e in entries {
        match e {
            ActionLogEntry::Text(s) => out.push(s.clone()),
            ActionLogEntry::Section(label) => out.push(format!("─── {label} ───")),
            ActionLogEntry::Finds { text, finds } => {
                out.push(text.clone());
                out.extend(finds.iter().map(find_line));
            }
            ActionLogEntry::Image(img) => out.push(format!("[image] {}", img.label)),
            ActionLogEntry::ItemPipeline {
                title,
                summary,
                details,
                finds,
                ..
            } => {
                out.push(format!("[item] {title} — {summary}"));
                out.extend(details.iter().cloned());
                out.extend(finds.iter().map(find_line));
            }
        }
    }
    out
}

fn find_line(f: &LogFind) -> String {
    let (l, t, r, b) = f.screen;
    match &f.detail {
        Some(d) => format!("  [find] {} — {d} screen ({l},{t})-({r},{b})", f.label),
        None => format!("  [find] {} screen ({l},{t})-({r},{b})", f.label),
    }
}

impl ActionLogger for SharedActionLog {
    fn log(&self, action_id: ActionId, message: String) {
        push_entry(
            &self.inner,
            action_id,
            ActionLogEntry::Text(message),
            self.entry_cap(),
        );
    }

    fn log_images_enabled(&self) -> bool {
        SharedActionLog::log_images_enabled(self)
    }

    fn log_verbose_enabled(&self) -> bool {
        SharedActionLog::log_verbose_enabled(self)
    }

    fn log_image(&self, action_id: ActionId, image: &LogImage) {
        if !self.log_images_enabled() {
            return;
        }
        replace_or_push_image(&self.inner, action_id, image.clone(), self.entry_cap());
    }

    fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: String,
        summary: String,
        thumbnail: &LogImage,
        steps: &[LogImage],
        details: Vec<String>,
        finds: Vec<LogFind>,
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
                finds,
            },
            self.entry_cap(),
        );
    }

    fn log_finds(&self, action_id: ActionId, text: String, mut finds: Vec<LogFind>) {
        // Find contexts hold pixels; screen rects are free, so keep the markers
        // (pin-to-desktop still works) when image logging is off.
        if !self.log_images_enabled() {
            for f in &mut finds {
                f.context = None;
            }
        }
        push_entry(
            &self.inner,
            action_id,
            ActionLogEntry::Finds { text, finds },
            self.entry_cap(),
        );
    }

    fn log_section(&self, action_id: ActionId, label: String) {
        push_entry(
            &self.inner,
            action_id,
            ActionLogEntry::Section(label),
            self.entry_cap(),
        );
    }
}

fn push_entry(
    inner: &Mutex<HashMap<ActionId, Vec<ActionLogEntry>>>,
    action_id: ActionId,
    entry: ActionLogEntry,
    max: usize,
) {
    let mut map = inner.lock();
    let entries = map.entry(action_id).or_default();
    push_capped(entries, entry, max);
}

fn push_capped(entries: &mut Vec<ActionLogEntry>, entry: ActionLogEntry, max: usize) {
    entries.push(entry);
    if entries.len() > max {
        let drop = entries.len() - max;
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
    max: usize,
) {
    let mut map = inner.lock();
    let entries = map.entry(action_id).or_default();
    if let Some(key) = live_capture_slot_key(&img.label) {
        if let Some(pos) = entries.iter().rposition(|e| {
            matches!(
                e,
                ActionLogEntry::Image(existing) if live_capture_slot_key(&existing.label) == Some(key)
            )
        }) {
            // Re-push at the tail so the live capture lands inside the newest
            // attempt's section instead of staying trapped in an old one.
            entries.remove(pos);
        }
    }
    push_capped(entries, ActionLogEntry::Image(img), max);
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
        assert_eq!(
            log.image_bytes(),
            0,
            "disabled images must not retain pixels"
        );
    }

    #[test]
    fn image_bytes_tracks_retained_pixels_and_clear() {
        let log = SharedActionLog::new();
        log.set_log_images(true);
        let id = ActionId::new();
        log.log(id, "text-only".into());
        assert_eq!(log.image_bytes(), 0);

        let img = solid("shot", 64);
        let expected = img.pixels.len();
        log.log_image(id, &img);
        assert_eq!(log.image_bytes(), expected);

        let thumb = solid("thumb", 1);
        let step = solid("step", 2);
        let pipeline_bytes = thumb.pixels.len() + step.pixels.len();
        log.log_item_pipeline(
            id,
            "item".into(),
            "sum".into(),
            &thumb,
            &[step],
            vec!["d".into()],
            vec![],
        );
        assert_eq!(log.image_bytes(), expected + pipeline_bytes);

        log.clear();
        assert_eq!(log.image_bytes(), 0);
    }

    #[test]
    fn find_context_shared_pixels_count_once() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        let img = Arc::new(solid("ctx", 64));
        let bytes = img.pixels.len();
        // Same LogImage as an Image entry and a find context — one allocation.
        log.log_image(id, &img);
        log.log_finds(
            id,
            "hit".into(),
            vec![LogFind {
                label: "f".into(),
                detail: None,
                screen: (0, 0, 1, 1),
                context: Some(FindContext::Image {
                    image: Arc::clone(&img),
                    rect: (0, 0, 1, 1),
                }),
            }],
        );
        assert_eq!(log.image_bytes(), bytes);
    }

    #[test]
    fn clear_for_drops_one_action() {
        let log = SharedActionLog::new();
        let a = ActionId::new();
        let b = ActionId::new();
        log.log(a, "from-a".into());
        log.log(b, "from-b".into());
        log.clear_for(a);
        assert!(log.entries_for(a).is_empty());
        assert_eq!(lines_for(&log.entries_for(b)), vec!["from-b".to_string()]);
    }

    #[test]
    fn log_finds_strips_contexts_when_images_disabled() {
        let log = SharedActionLog::new();
        log.set_log_images(false);
        let id = ActionId::new();
        let img = Arc::new(solid("ctx", 64));
        log.log_finds(
            id,
            "found one".into(),
            vec![LogFind {
                label: "f".into(),
                detail: None,
                screen: (10, 20, 30, 40),
                context: Some(FindContext::Image {
                    image: Arc::clone(&img),
                    rect: (0, 0, 1, 1),
                }),
            }],
        );
        let entries = log.entries_for(id);
        let ActionLogEntry::Finds { text, finds } = &entries[0] else {
            panic!("expected Finds entry");
        };
        assert_eq!(text, "found one");
        assert_eq!(finds[0].screen, (10, 20, 30, 40));
        assert!(finds[0].context.is_none());
        assert_eq!(log.image_bytes(), 0);
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

    #[test]
    fn set_max_entries_shrinks_existing_buffers() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        for i in 0..100 {
            log.log(id, format!("line-{i}"));
        }
        log.set_max_entries(10);
        let entries = log.entries_for(id);
        assert_eq!(entries.len(), 10);
        assert_eq!(entries[0].as_text(), Some("line-90"));
        // New pushes still respect the lower cap.
        log.log(id, "tail".into());
        assert_eq!(log.entries_for(id).len(), 10);
    }

    #[test]
    fn live_capture_slot_moves_into_latest_section() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        log.log_image(id, &solid("1. Capture (search area)", 10));
        log.log_section(id, "Attempt 2".into());
        log.log(id, "a2 line".into());
        log.log_image(id, &solid("1. Capture (search area)", 20));
        let entries = log.entries_for(id);
        // One capture entry, re-pushed after the new section so the live slot
        // renders inside the latest attempt's group.
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_section());
        assert_eq!(entries[1].as_text(), Some("a2 line"));
        assert!(entries[2].is_image());
    }

    #[test]
    fn section_entries_render_as_lines() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        log.log_section(id, "Attempt 1".into());
        log.log(id, "inside".into());
        let lines = lines_for(&log.entries_for(id));
        assert_eq!(
            lines,
            vec!["─── Attempt 1 ───".to_string(), "inside".to_string()]
        );
    }
}
