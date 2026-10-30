//! Click-effect feedback: a shrinking gold square on the desktop where each
//! automated mouse press lands (Settings → Macros → "Flash a marker where
//! clicks land").
//!
//! Draws through [`sqyre_capture::SelectionOutline`] — the same click-through
//! edge windows as the recording rubber-band — because deferred egui viewports
//! cannot be positioned on GNOME/Wayland. Presses inside a run are serialized,
//! so one outline chasing the newest press is enough.

use eframe::egui;
use sqyre_capture::{mark_site, SelectionOutline};
use sqyre_ports::SharedClickFx;
use std::time::{Duration, Instant};

/// Square edge at flash start / end (virtual-desktop pixels).
const FLASH_START_PX: i32 = 72;
const FLASH_END_PX: i32 = 20;
const FLASH_DURATION: Duration = Duration::from_millis(360);
/// Repaint pace while a flash animates.
const FRAME_MS: u64 = 16;

/// Owns the outline window and the running animation. UI thread only.
#[derive(Default)]
pub struct ClickFxOverlay {
    /// Created lazily on the first flash; never touched off the UI thread.
    outline: Option<SelectionOutline>,
    outline_failed: bool,
    /// `(x, y, started)` of the visible flash; the newest press replaces it.
    active: Option<(i32, i32, Instant)>,
}

impl ClickFxOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain queued press positions and advance the flash; call once per UI frame.
    pub fn sync(&mut self, ctx: &egui::Context, fx: &SharedClickFx) {
        for (x, y) in fx.drain() {
            self.active = Some((x, y, Instant::now()));
        }
        let Some((x, y, started)) = self.active else {
            return;
        };
        let t = started.elapsed().as_secs_f32() / FLASH_DURATION.as_secs_f32();
        if t >= 1.0 {
            self.active = None;
            if let Some(outline) = self.outline.as_mut() {
                if outline.is_active() {
                    outline.clear();
                }
            }
            return;
        }
        if self.outline.is_none() && !self.outline_failed {
            mark_site("clickfx:open");
            match SelectionOutline::open() {
                Ok(outline) => self.outline = Some(outline),
                Err(e) => {
                    self.outline_failed = true;
                    crate::log::warn(format_args!("click effect outline unavailable: {e}"));
                    return;
                }
            }
        }
        if let Some(outline) = self.outline.as_mut() {
            let size = FLASH_START_PX as f32 + (FLASH_END_PX - FLASH_START_PX) as f32 * t;
            let half = (size * 0.5).round().max(2.0) as i32;
            outline.set_rect(x - half, y - half, x + half, y + half);
        }
        ctx.request_repaint_after(Duration::from_millis(FRAME_MS));
    }
}
