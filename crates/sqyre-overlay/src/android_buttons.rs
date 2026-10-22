//! Accessibility-overlay windows for macro buttons.
//!
//! The activity stops drawing while another app is in front, so this thread
//! shows and hides windows itself from the focus report and the image-gate map.
//! A tap starts the macro through [`sqyre_android::request_overlay_run`] instead
//! of waiting for the next egui frame.

use crate::brand_s::SWEEP_PER_SEC;
use crate::button_visibility::{clamp_origin, shown_buttons, AndroidButtonSpec};
use crate::raster::{self, ButtonPaint};
use crate::OverlayButtonMove;
use parking_lot::Mutex;
use sqyre_android::{
    bridge, clear_overlay_input_handlers, overlay_input, request_overlay_run, AndroidError,
};
use sqyre_capture::{get_active_window, note};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const IDLE_TICK: Duration = Duration::from_millis(100);
const BUSY_TICK: Duration = Duration::from_millis(50);

#[derive(PartialEq)]
struct Applied {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    clickable: bool,
    relocate: bool,
    /// Sweep step while busy; `None` when the face is idle.
    phase_step: Option<u8>,
}

/// Owns the window thread. Drop removes every button.
pub struct AndroidButtonHost {
    specs: Arc<Mutex<Vec<AndroidButtonSpec>>>,
    relocate: Arc<AtomicBool>,
    visibility: Arc<Mutex<Arc<Mutex<HashMap<String, bool>>>>>,
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for AndroidButtonHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl AndroidButtonHost {
    pub fn start(
        pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
        running_macro: Arc<Mutex<Option<String>>>,
        visibility: Arc<Mutex<HashMap<String, bool>>>,
    ) -> Option<Self> {
        let specs = Arc::new(Mutex::new(Vec::new()));
        let relocate = Arc::new(AtomicBool::new(false));
        let visibility = Arc::new(Mutex::new(visibility));
        let stop = Arc::new(AtomicBool::new(false));
        install_input_handlers(
            Arc::clone(&specs),
            Arc::clone(&running_macro),
            pending_moves,
        );
        let stop_t = Arc::clone(&stop);
        let specs_t = Arc::clone(&specs);
        let relocate_t = Arc::clone(&relocate);
        let visibility_t = Arc::clone(&visibility);
        let join = match thread::Builder::new()
            .name("sqyre-android-overlay".into())
            .spawn(move || {
                host_loop(specs_t, relocate_t, running_macro, visibility_t, stop_t);
            }) {
            Ok(join) => join,
            Err(e) => {
                note(&format!("overlay: android host failed to start: {e}"));
                clear_overlay_input_handlers();
                return None;
            }
        };
        Some(Self {
            specs,
            relocate,
            visibility,
            stop,
            join: Mutex::new(Some(join)),
        })
    }

    pub fn set_buttons(&self, buttons: Vec<AndroidButtonSpec>) {
        *self.specs.lock() = buttons;
    }

    pub fn set_visibility_map(&self, map: Arc<Mutex<HashMap<String, bool>>>) {
        *self.visibility.lock() = map;
    }

    pub fn set_relocate(&self, enabled: bool) {
        self.relocate.store(enabled, Ordering::Relaxed);
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.lock().take() {
            let _ = join.join();
        }
        clear_overlay_input_handlers();
    }
}

fn install_input_handlers(
    specs: Arc<Mutex<Vec<AndroidButtonSpec>>>,
    running_macro: Arc<Mutex<Option<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
) {
    let specs_click = Arc::clone(&specs);
    let running_click = Arc::clone(&running_macro);
    overlay_input::set_overlay_click_handler(Arc::new(move |id| {
        let name = {
            let specs = specs_click.lock();
            specs
                .iter()
                .find(|b| b.id == id)
                .map(|b| b.macro_name.clone())
        };
        let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
            return;
        };
        let busy = running_click
            .lock()
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(&name));
        if busy {
            note(&format!("overlay-android: press-busy-ignore name={name}"));
            return;
        }
        note(&format!("overlay-android: click id={id} macro={name}"));
        sqyre_capture::event_log(
            "SQYRE_OVERLAY",
            &[("click", "press"), ("id", id), ("name", name.as_str())],
        );
        request_overlay_run(&name);
    }));
    overlay_input::set_overlay_move_handler(Arc::new(move |id, x, y| {
        let mut specs = specs.lock();
        if let Some(btn) = specs.iter_mut().find(|b| b.id == id) {
            btn.x = x;
            btn.y = y;
        }
        drop(specs);
        pending_moves.lock().push(OverlayButtonMove {
            id: id.to_string(),
            x,
            y,
        });
        note(&format!("overlay-android: move id={id} x={x} y={y}"));
    }));
}

fn host_loop(
    specs: Arc<Mutex<Vec<AndroidButtonSpec>>>,
    relocate: Arc<AtomicBool>,
    running_macro: Arc<Mutex<Option<String>>>,
    visibility: Arc<Mutex<Arc<Mutex<HashMap<String, bool>>>>>,
    stop: Arc<AtomicBool>,
) {
    note("overlay: using android accessibility button host");
    let mut applied: HashMap<String, Applied> = HashMap::new();
    let mut phases: HashMap<String, f32> = HashMap::new();
    let mut logged_off = false;
    let mut last_tick = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        let dt = last_tick.elapsed().as_secs_f32().clamp(0.016, 0.5);
        last_tick = Instant::now();
        let any_busy = reconcile(
            &specs,
            relocate.load(Ordering::Relaxed),
            &running_macro,
            &visibility,
            &mut applied,
            &mut phases,
            &mut logged_off,
            dt,
        );
        let slice = if any_busy { BUSY_TICK } else { IDLE_TICK };
        let deadline = Instant::now() + slice;
        while Instant::now() < deadline {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
    applied.clear();
    if let Err(e) = bridge::clear_overlays() {
        note(&format!("overlay-android: clear on stop: {e}"));
    }
    note("overlay: android button host stopped");
}

fn reconcile(
    specs: &Mutex<Vec<AndroidButtonSpec>>,
    relocate: bool,
    running_macro: &Mutex<Option<String>>,
    visibility: &Mutex<Arc<Mutex<HashMap<String, bool>>>>,
    applied: &mut HashMap<String, Applied>,
    phases: &mut HashMap<String, f32>,
    logged_off: &mut bool,
    dt: f32,
) -> bool {
    let specs = specs.lock().clone();
    let running = running_macro.lock().clone();
    let focus = get_active_window().ok().flatten();
    let map = visibility.lock().clone();
    let found = map.lock().clone();
    let show = shown_buttons(&specs, focus.as_ref(), relocate, &found);
    let (screen_w, screen_h) = bridge::display_size().unwrap_or((0, 0));
    let mut live = Vec::new();
    let mut any_busy = false;
    for spec in show {
        live.push(spec.id.clone());
        let busy = running
            .as_deref()
            .is_some_and(|n| spec.macro_name.eq_ignore_ascii_case(n));
        if busy {
            any_busy = true;
        }
        let phase = if busy {
            let p = phases.entry(spec.id.clone()).or_insert(0.0);
            *p = (*p + SWEEP_PER_SEC * dt) % 1.0;
            *p
        } else {
            phases.remove(&spec.id);
            0.0
        };
        let (x, y) = clamp_origin(spec.x, spec.y, spec.w, spec.h, screen_w, screen_h);
        let clickable = !busy && !relocate && !spec.macro_name.trim().is_empty();
        let stamp = Applied {
            x,
            y,
            w: spec.w,
            h: spec.h,
            clickable,
            relocate,
            phase_step: busy.then_some((phase * 32.0) as u8),
        };
        if applied.get(&spec.id) == Some(&stamp) {
            continue;
        }
        let rgba = raster::rasterize_argb(&ButtonPaint {
            w: spec.w.max(1),
            h: spec.h.max(1),
            bg: spec.bg,
            border: spec.border,
            border_width: spec.border_width,
            corner_radius: spec.corner_radius,
            icon_glyph: spec.icon_glyph,
            icon: spec.icon,
            icon_hover: spec.icon_hover,
            hovered: false,
            busy,
            busy_phase: phase,
        });
        match bridge::upsert_overlay(
            &spec.id,
            x,
            y,
            spec.w as i32,
            spec.h as i32,
            &rgba,
            clickable,
            relocate,
            spec.macro_name.trim(),
        ) {
            Ok(()) => {
                applied.insert(spec.id.clone(), stamp);
                *logged_off = false;
            }
            Err(AndroidError::AccessibilityOff) => {
                if !*logged_off {
                    note("overlay-android: accessibility service is off; buttons hidden");
                    *logged_off = true;
                }
            }
            Err(e) => note(&format!("overlay-android: upsert {} failed: {e}", spec.id)),
        }
    }
    let stale: Vec<String> = applied
        .keys()
        .filter(|id| !live.iter().any(|live_id| live_id == *id))
        .cloned()
        .collect();
    for id in stale {
        applied.remove(&id);
        phases.remove(&id);
        if let Err(e) = bridge::remove_overlay(&id) {
            note(&format!("overlay-android: remove {id} failed: {e}"));
        }
    }
    any_busy
}
