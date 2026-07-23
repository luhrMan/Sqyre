//! Search-area selection outline + recording coords HUD.
//!
//! Driven by [`sqyre_hotkeys::ScreenClickBridge`]:
//! - OS edge windows ([`sqyre_capture::SelectionOutline`]) for the live search-area
//!   rect — not a fullscreen desktop snapshot (X11 on Linux, Win32 popups on Windows).
//! - A small always-on-top egui viewport for live coords / status while recording
//!   (needed when the main window is hidden via `hide_app_during_recording`).
//!
//! A short poller owns the outline and keeps requesting egui repaints so the
//! HUD stays alive even when the root viewport is `Visible(false)`.

use crate::theme;
use eframe::egui::{self, ViewportBuilder, ViewportClass, ViewportId};
use sqyre_capture::SelectionOutline;
use sqyre_hotkeys::ScreenClickBridge;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const POLL_MS: u64 = 16;
const HUD_ID: &str = "sqyre_recording_coords_hud";

/// Owns the outline poller and syncs it to the armed search-area draft.
#[derive(Default)]
pub struct RecordingOverlay {
    stop: Option<Arc<AtomicBool>>,
    join: Option<JoinHandle<()>>,
}

impl RecordingOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call every frame while the app is running.
    ///
    /// Starts the outline/repaint poller once any screen-click recording arms, and
    /// shows the coords HUD viewport while armed.
    pub fn sync(&mut self, ctx: &egui::Context, screen_click: &ScreenClickBridge) {
        if screen_click.is_armed() {
            self.ensure_worker(ctx.clone(), screen_click.clone());
            show_coords_hud(ctx, screen_click);
        }
    }

    fn ensure_worker(&mut self, ctx: egui::Context, bridge: ScreenClickBridge) {
        if self.join.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        self.stop = Some(Arc::clone(&stop));
        self.join = Some(thread::spawn(move || {
            let mut outline = match SelectionOutline::open() {
                Ok(o) => Some(o),
                Err(e) => {
                    eprintln!("sqyre: selection outline unavailable: {e}");
                    None
                }
            };
            while !stop.load(Ordering::Relaxed) {
                if let Some(outline) = outline.as_mut() {
                    match bridge.peek_search_area_selection() {
                        Some((lx, ty, rx, by)) => outline.set_rect(lx, ty, rx, by),
                        None => outline.clear(),
                    }
                }
                // Wake the UI loop so the HUD viewport keeps updating while the
                // main window is hidden for recording.
                if bridge.is_armed() {
                    ctx.request_repaint();
                }
                thread::sleep(Duration::from_millis(POLL_MS));
            }
            if let Some(mut outline) = outline {
                outline.clear();
            }
        }));
    }
}

impl Drop for RecordingOverlay {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn show_coords_hud(ctx: &egui::Context, screen_click: &ScreenClickBridge) {
    if screen_click.status_label().is_none() {
        return;
    }
    let bridge = screen_click.clone();
    let id = ViewportId::from_hash_of(HUD_ID);
    let builder = ViewportBuilder::default()
        .with_title("Sqyre recording")
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_inner_size([560.0, 44.0])
        .with_min_inner_size([200.0, 36.0]);

    // Deferred: independent of the (possibly hidden) root viewport paint cycle,
    // as long as the parent keeps registering it each frame via request_repaint.
    ctx.show_viewport_deferred(id, builder, move |ui, class| {
        let Some(text) = bridge.status_label() else {
            return;
        };
        paint_hud_label(ui, class, &text);
        ui.ctx().request_repaint();
    });
}

fn paint_hud_label(ui: &mut egui::Ui, class: ViewportClass, text: &str) {
    let frame = egui::Frame::NONE
        .fill(crate::theme::overlay_panel_fill())
        .stroke(egui::Stroke::new(1.0, theme::PRIMARY))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 8));

    if class == ViewportClass::EmbeddedWindow {
        egui::Window::new("Recording")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 12.0])
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.label(egui::RichText::new(text).color(theme::PRIMARY).strong());
            });
        return;
    }

    frame.show(ui, |ui| {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new(text).color(theme::PRIMARY).strong());
        });
    });
}
