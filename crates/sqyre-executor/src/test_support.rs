//! Shared test doubles for executor unit/integration tests.

use crate::backends::{
    AutomationBackend, AutomationError, CollectionArea, ContinueKeyWaiter, CoordinateResolver,
    DesktopRect, MacroLookup, MoveOptions, OcrEngine, PortError, ScreenCapturer, WindowFocuser,
};
use image::RgbaImage;
use sqyre_domain::{grid_cell_rect, CoordinateRef, Macro};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Collection grid metadata for [`FixedResolver`] atlas tests.
#[derive(Debug, Clone, Copy)]
pub struct FixedCollection {
    pub rows: i32,
    pub cols: i32,
    pub bounds: (i32, i32, i32, i32),
}

/// One atlas member passed to [`FixedResolver::with_atlas`].
#[derive(Debug, Clone)]
pub struct AtlasMemberSpec {
    pub name: String,
    pub collection: FixedCollection,
}

/// Fixed point + search-area resolver; optional collection grid / atlas.
#[derive(Debug, Clone)]
pub struct FixedResolver {
    pub point: (i32, i32),
    pub area: (i32, i32, i32, i32),
    pub grid: Option<(i32, i32)>,
    /// Collection name → grid; `None` for const/simple resolvers.
    pub collections: Option<HashMap<String, FixedCollection>>,
    pub atlas_members: Option<Vec<String>>,
}

impl FixedResolver {
    pub const fn point_area(point: (i32, i32), area: (i32, i32, i32, i32)) -> Self {
        Self {
            point,
            area,
            grid: None,
            collections: None,
            atlas_members: None,
        }
    }

    pub fn with_atlas(collections: Vec<AtlasMemberSpec>, members: Vec<String>) -> Self {
        let mut map = HashMap::new();
        for spec in collections {
            map.insert(spec.name, spec.collection);
        }
        Self {
            point: (0, 0),
            area: (0, 0, 100, 100),
            grid: None,
            collections: Some(map),
            atlas_members: Some(members),
        }
    }
}

/// Default used by most search tests: point (0,0), area (100,200)-(110,210).
pub const SEARCH_FIXED_AREA: FixedResolver =
    FixedResolver::point_area((0, 0), (100, 200, 110, 210));

impl CoordinateResolver for FixedResolver {
    fn resolve_point(&self, r: &CoordinateRef, _macro_: &Macro) -> Result<(i32, i32), PortError> {
        if let Some((r1, c1, _, _)) = r.cell_range() {
            if let Some(cols) = &self.collections {
                if let Some(FixedCollection {
                    rows,
                    cols: cols_n,
                    bounds: (lx, ty, rx, by),
                }) = cols.get(r.name())
                {
                    let width = rx - lx;
                    let height = by - ty;
                    let cx = lx + (c1 - 1) * width / cols_n + width / (cols_n * 2);
                    let cy = ty + (r1 - 1) * height / rows + height / (rows * 2);
                    return Ok((cx, cy));
                }
            }
            if self.grid.is_some() {
                return Ok((c1 * 10, r1 * 10));
            }
        }
        Ok(self.point)
    }

    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), PortError> {
        if let Some((r1, c1, r2, c2)) = r.cell_range() {
            if let Some(cols) = &self.collections {
                if let Some(FixedCollection {
                    rows,
                    cols: cols_n,
                    bounds,
                }) = cols.get(r.name())
                {
                    return grid_cell_rect(*bounds, *rows, *cols_n, r1, c1, r2, c2).ok_or_else(
                        || {
                            PortError::invalid(format!(
                                "cell range {r1},{c1}-{r2},{c2} out of bounds for {rows}x{cols_n}"
                            ))
                        },
                    );
                }
            }
        }
        Ok(self.area)
    }

    fn collection_grid(&self, _program: &str, collection: &str) -> Result<(i32, i32), PortError> {
        if let Some(cols) = &self.collections {
            if let Some(FixedCollection { rows, cols, .. }) = cols.get(collection) {
                return Ok((*rows, *cols));
            }
        }
        self.grid
            .ok_or_else(|| PortError::not_configured("collection grid lookup"))
    }

    fn collection_area(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<CollectionArea, PortError> {
        if let Some(cols) = &self.collections {
            if let Some(FixedCollection {
                rows,
                cols,
                bounds: (left, top, right, bottom),
            }) = cols.get(r.name())
            {
                return Ok(CollectionArea {
                    left: *left,
                    top: *top,
                    right: *right,
                    bottom: *bottom,
                    rows: *rows,
                    cols: *cols,
                });
            }
        }
        if let Some((rows, cols)) = self.grid {
            let (left, top, right, bottom) = self.area;
            return Ok(CollectionArea {
                left,
                top,
                right,
                bottom,
                rows,
                cols,
            });
        }
        Err(PortError::not_configured("collection area lookup"))
    }

    fn atlas_members(&self, _program: &str, _atlas: &str) -> Result<Vec<String>, PortError> {
        self.atlas_members
            .clone()
            .ok_or_else(|| PortError::not_configured("atlas lookup"))
    }
}

/// Test OCR engine that returns a fixed result.
#[derive(Debug, Default)]
pub struct FixedOcrEngine {
    pub result: sqyre_vision::OcrRecognition,
    pub log: std::sync::Mutex<Vec<String>>,
}

impl OcrEngine for FixedOcrEngine {
    fn recognize(
        &self,
        image: &sqyre_match::ImageBuf,
    ) -> Result<sqyre_vision::OcrRecognition, PortError> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "ocr:{}x{}c{}",
                image.width, image.height, image.channels
            ));
        }
        Ok(self.result.clone())
    }
}

/// Test OCR engine that pops results from a FIFO queue (then repeats the last).
#[derive(Debug, Default)]
pub struct QueuedOcrEngine {
    pub queue: std::sync::Mutex<Vec<sqyre_vision::OcrRecognition>>,
    pub log: std::sync::Mutex<Vec<String>>,
}

impl OcrEngine for QueuedOcrEngine {
    fn recognize(
        &self,
        image: &sqyre_match::ImageBuf,
    ) -> Result<sqyre_vision::OcrRecognition, PortError> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "ocr:{}x{}c{}",
                image.width, image.height, image.channels
            ));
        }
        let mut q = self
            .queue
            .lock()
            .map_err(|_| PortError::Message("QueuedOcrEngine: lock poisoned".into()))?;
        if q.is_empty() {
            return Err(PortError::invalid("QueuedOcrEngine: empty queue"));
        }
        if q.len() == 1 {
            return Ok(q[0].clone());
        }
        Ok(q.remove(0))
    }
}

/// Recording backend for unit tests.
#[derive(Debug, Default)]
pub struct RecordingBackend {
    pub log: Vec<String>,
}

impl RecordingBackend {
    /// True when `event` appears in the recorded log.
    pub fn has(&self, event: &str) -> bool {
        self.log.iter().any(|e| e == event)
    }

    /// Number of occurrences of `event` in the recorded log.
    pub fn count(&self, event: &str) -> usize {
        self.log.iter().filter(|e| e.as_str() == event).count()
    }

    /// Assert `event` was recorded at least once (dumps the log on failure).
    pub fn assert_logged(&self, event: &str) {
        assert!(self.has(event), "expected {event} in {:?}", self.log);
    }

    /// Assert `event` was never recorded (dumps the log on failure).
    pub fn assert_not_logged(&self, event: &str) {
        assert!(!self.has(event), "did not expect {event} in {:?}", self.log);
    }
}

impl AutomationBackend for RecordingBackend {
    fn milli_sleep(&mut self, ms: i32) {
        self.log.push(format!("sleep:{ms}"));
    }
    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        self.log
            .push(format!("move:{x},{y},smooth={}", opts.smooth));
    }
    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        self.log.push(format!(
            "click:{button}:{}",
            if down { "down" } else { "up" }
        ));
        Ok(())
    }
    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        self.log
            .push(format!("scroll:{}", if up { "up" } else { "down" }));
        Ok(())
    }
    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        self.log.push(format!("keydown:{key}"));
        Ok(())
    }
    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        self.log.push(format!("keyup:{key}"));
        Ok(())
    }
    fn type_char(&mut self, ch: char) -> Result<(), AutomationError> {
        self.log.push(format!("type:{ch}"));
        Ok(())
    }
    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError> {
        self.log.push(format!("clipboard:{s}"));
        Ok(())
    }
}

/// Backend wrapper that trips `stop` after `after` `milli_sleep` calls so
/// wait/retry loops abort; the inner backend's log stays inspectable.
#[derive(Debug)]
pub struct StopAfterSleeps {
    pub inner: RecordingBackend,
    pub stop: Arc<AtomicBool>,
    pub sleeps: usize,
    pub after: usize,
}

impl StopAfterSleeps {
    pub fn new(stop: Arc<AtomicBool>, after: usize) -> Self {
        Self {
            inner: RecordingBackend::default(),
            stop,
            sleeps: 0,
            after,
        }
    }
}

impl AutomationBackend for StopAfterSleeps {
    fn milli_sleep(&mut self, ms: i32) {
        self.inner.milli_sleep(ms);
        self.sleeps += 1;
        if self.sleeps >= self.after {
            self.stop.store(true, Ordering::SeqCst);
        }
    }
    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        self.inner.move_to(x, y, opts);
    }
    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        self.inner.click(button, down)
    }
    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        self.inner.scroll(up)
    }
    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        self.inner.key_down(key)
    }
    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        self.inner.key_up(key)
    }
    fn type_char(&mut self, ch: char) -> Result<(), AutomationError> {
        self.inner.type_char(ch)
    }
    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError> {
        self.inner.write_clipboard(s)
    }
}

/// In-memory capturer for tests.
#[derive(Debug, Default)]
pub struct RecordingCapturer {
    pub log: Vec<String>,
    /// Single image returned when [`Self::queue`] is empty.
    pub next: Option<RgbaImage>,
    /// FIFO images consumed one per capture (then falls back to [`Self::next`]).
    pub queue: Vec<RgbaImage>,
    pub bounds: DesktopRect,
}

impl RecordingCapturer {
    /// True when `event` appears in the recorded log.
    pub fn has(&self, event: &str) -> bool {
        self.log.iter().any(|e| e == event)
    }

    /// True when any recorded event starts with `prefix`.
    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.log.iter().any(|e| e.starts_with(prefix))
    }

    /// Assert `event` was recorded at least once (dumps the log on failure).
    pub fn assert_logged(&self, event: &str) {
        assert!(self.has(event), "expected {event} in {:?}", self.log);
    }

    fn take_image(&mut self) -> Result<RgbaImage, crate::CaptureError> {
        if !self.queue.is_empty() {
            return Ok(self.queue.remove(0));
        }
        self.next
            .clone()
            .ok_or_else(|| crate::CaptureError::Message("RecordingCapturer: no image".into()))
    }
}

impl ScreenCapturer for RecordingCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, crate::CaptureError> {
        self.log.push(format!("monitor:{display_index}"));
        self.take_image()
    }
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, crate::CaptureError> {
        self.log
            .push(format!("rect:{},{},{},{}", rect.x, rect.y, rect.w, rect.h));
        self.take_image()
    }
    fn capture_rect_rgb_fresh(
        &mut self,
        rect: DesktopRect,
    ) -> Result<crate::backends::RgbCapture, crate::CaptureError> {
        self.log.push(format!(
            "rgb_fresh:{},{},{},{}",
            rect.x, rect.y, rect.w, rect.h
        ));
        Ok(crate::backends::RgbCapture::from_rgba(&self.take_image()?))
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, crate::CaptureError> {
        Ok(self.bounds)
    }
}

/// In-memory macro catalog for tests.
#[derive(Debug, Default)]
pub struct MapMacroLookup {
    pub macros: std::collections::BTreeMap<String, Arc<Macro>>,
}

impl MacroLookup for MapMacroLookup {
    fn get(&self, name: &str) -> Option<Arc<Macro>> {
        self.macros.get(name).cloned()
    }
}

/// Test waiter that returns immediately (does not block).
#[derive(Debug, Default)]
pub struct ImmediateContinueWaiter {
    pub log: std::sync::Mutex<Vec<String>>,
    /// Indices returned by successive `wait_for_any_chord` calls (defaults to 0).
    pub any_queue: std::sync::Mutex<Vec<usize>>,
}

impl ContinueKeyWaiter for ImmediateContinueWaiter {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        _stop: &AtomicBool,
    ) -> Result<(), PortError> {
        if keys.is_empty() {
            return Err(PortError::invalid("pause: continue key not set"));
        }
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "continue:{}:passthrough={pass_through}",
                keys.join("+")
            ));
        }
        Ok(())
    }

    fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        pass_through: bool,
        _stop: &AtomicBool,
    ) -> Result<usize, PortError> {
        if chords.is_empty() || chords.iter().all(|c| c.is_empty()) {
            return Err(PortError::invalid("key wait: no chords configured"));
        }
        let idx = self
            .any_queue
            .lock()
            .ok()
            .and_then(|mut q| {
                if q.is_empty() {
                    None
                } else {
                    Some(q.remove(0))
                }
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.log.lock() {
            let labels: Vec<String> = chords
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let hold = if hold_repeat.get(i).copied().unwrap_or(false) {
                        "*"
                    } else {
                        ""
                    };
                    format!("{hold}{}", c.join("+"))
                })
                .collect();
            g.push(format!(
                "any:{}:pick={idx}:passthrough={pass_through}",
                labels.join("|")
            ));
        }
        Ok(idx.min(chords.len().saturating_sub(1)))
    }
}

/// Test focuser that records calls; `focus` makes that window the `focused` one.
#[derive(Debug, Default)]
pub struct RecordingWindowFocuser {
    pub log: std::sync::Mutex<Vec<String>>,
    pub current: std::sync::Mutex<Option<sqyre_ports::FocusedWindow>>,
}

impl RecordingWindowFocuser {
    pub fn focused_on(process_path: &str, window_title: &str) -> Self {
        Self {
            current: std::sync::Mutex::new(Some(sqyre_ports::FocusedWindow {
                process_path: process_path.into(),
                window_title: window_title.into(),
            })),
            ..Self::default()
        }
    }
}

impl WindowFocuser for RecordingWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!("focus:{process_path}:{window_title}"));
        }
        if let Ok(mut g) = self.current.lock() {
            *g = Some(sqyre_ports::FocusedWindow {
                process_path: process_path.into(),
                window_title: window_title.into(),
            });
        }
        Ok(())
    }

    fn focused(&self) -> Result<Option<sqyre_ports::FocusedWindow>, AutomationError> {
        Ok(self.current.lock().ok().and_then(|g| g.clone()))
    }
}
