//! Periodic image-search gate for overlay buttons (show when found).

use eframe::egui;
use sqyre_capture::{
    get_active_window, shared_capturer, window_matches_binding, window_matches_program, WindowInfo,
};
use sqyre_domain::{CoordinateRef, Macro};
use sqyre_match::{
    blur_image_owned, find_template_matches_preblurred_with_prepared, prepare_search,
    search_blur_kernel,
};
use sqyre_persist::{OverlayButtonConfig, OverlayVisibilityGate, ProgramCatalog, GENERAL_PROGRAM};
use sqyre_ports::clamp_search_rect;
use sqyre_vision::{
    get_cached_blurred_template, get_cached_image_mask, get_cached_prepared_template,
    rgb_capture_to_image_buf,
};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use web_time::{Duration, Instant};

struct PollOutcome {
    button_id: String,
    found: bool,
}

/// Background one-shot image match results used to filter overlay buttons.
pub struct OverlayVisibilityPoller {
    found: HashMap<String, bool>,
    last_poll: HashMap<String, Instant>,
    in_flight: bool,
    rx: Option<Receiver<PollOutcome>>,
}

impl Default for OverlayVisibilityPoller {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayVisibilityPoller {
    pub fn new() -> Self {
        Self {
            found: HashMap::new(),
            last_poll: HashMap::new(),
            in_flight: false,
            rx: None,
        }
    }

    /// Drain finished polls; return true if any result changed visibility state.
    pub fn poll_results(&mut self) -> bool {
        let Some(rx) = &self.rx else {
            return false;
        };
        let mut changed = false;
        loop {
            match rx.try_recv() {
                Ok(out) => {
                    self.in_flight = false;
                    let prev = self.found.insert(out.button_id, out.found);
                    if prev != Some(out.found) {
                        changed = true;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.in_flight = false;
                    self.rx = None;
                    break;
                }
            }
        }
        changed
    }

    /// True when the button may be drawn (gate off, or last poll found a match).
    /// Active gates stay hidden until the first successful poll result arrives.
    pub fn allows_draw(&self, btn: &OverlayButtonConfig) -> bool {
        if !btn.visibility_gate.is_active() {
            return true;
        }
        self.found.get(&btn.id).copied().unwrap_or(false)
    }

    /// Schedule due polls and request a wake for the soonest interval.
    ///
    /// Only polls buttons that are enabled, have a macro, have an active gate,
    /// and pass the program focus gate (same rules as overlay draw).
    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        buttons: &[OverlayButtonConfig],
        catalog: &ProgramCatalog,
        close_matches_distance: i32,
    ) {
        if self.poll_results() {
            ctx.request_repaint();
        }

        let focus = get_active_window().ok().flatten();
        let now = Instant::now();
        let mut soonest: Option<Duration> = None;

        for btn in buttons {
            if !btn.enabled || btn.macro_name.trim().is_empty() {
                continue;
            }
            if !btn.visibility_gate.is_active() {
                continue;
            }
            if button_is_focus_gated(btn)
                && !program_owns_focus(catalog, &btn.program, focus.as_ref())
            {
                continue;
            }

            let interval = Duration::from_millis(btn.visibility_gate.interval_ms.max(1));
            let due = match self.last_poll.get(&btn.id) {
                Some(t) => now.saturating_duration_since(*t) >= interval,
                None => true,
            };
            if !due {
                if let Some(t) = self.last_poll.get(&btn.id) {
                    let elapsed = now.saturating_duration_since(*t);
                    let remain = interval.saturating_sub(elapsed);
                    soonest = Some(match soonest {
                        Some(s) => s.min(remain),
                        None => remain,
                    });
                }
                continue;
            }

            if self.in_flight {
                soonest = Some(Duration::from_millis(50));
                break;
            }

            self.spawn_poll(ctx, btn, catalog, close_matches_distance);
            self.last_poll.insert(btn.id.clone(), now);
            soonest = Some(match soonest {
                Some(s) => s.min(interval),
                None => interval,
            });
            break; // global concurrency = 1
        }

        if let Some(d) = soonest {
            ctx.request_repaint_after(d.max(Duration::from_millis(16)));
        }
    }

    fn spawn_poll(
        &mut self,
        ctx: &egui::Context,
        btn: &OverlayButtonConfig,
        catalog: &ProgramCatalog,
        close_matches_distance: i32,
    ) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.in_flight = true;

        let button_id = btn.id.clone();
        let gate = btn.visibility_gate.clone();
        let catalog = catalog.clone();
        let close_dist = close_matches_distance.clamp(0, 100);
        let ctx = ctx.clone();

        thread::Builder::new()
            .name("sqyre-overlay-vis".into())
            .spawn(move || {
                let found = match_once(&catalog, &gate, close_dist);
                let _ = tx.send(PollOutcome { button_id, found });
                ctx.request_repaint();
            })
            .ok();
    }
}

fn button_is_focus_gated(btn: &OverlayButtonConfig) -> bool {
    let p = btn.program.trim();
    !p.is_empty() && p != GENERAL_PROGRAM
}

fn program_owns_focus(catalog: &ProgramCatalog, program: &str, focus: Option<&WindowInfo>) -> bool {
    let Some(win) = focus else {
        return false;
    };
    if let Some(data) = catalog.get(program) {
        let path = data.process_path.trim();
        if !path.is_empty() {
            return window_matches_binding(win, path, &data.window_title);
        }
    }
    window_matches_program(win, program)
}

fn match_once(catalog: &ProgramCatalog, gate: &OverlayVisibilityGate, close_dist: i32) -> bool {
    if gate.targets.is_empty() || gate.search_area.trim().is_empty() {
        return false;
    }
    let macro_ = Macro::new("", 0, vec![]);
    let area = CoordinateRef(gate.search_area.clone());
    let Ok((lx, ty, rx, by)) = catalog.resolve_search_area(&area, &macro_) else {
        return false;
    };
    let Ok(capturer) = shared_capturer() else {
        return false;
    };
    let vb = capturer.virtual_bounds_ref().ok();
    let Ok(bounds) = clamp_search_rect(lx, ty, rx, by, vb) else {
        return false;
    };
    // Cached crop only — never `*_fresh_ref`. Fresh portal capture pulses a
    // compositor damage kick (xdg/layer-shell toplevel) that steals focus from
    // the game on every interval. Portal/X11 already stream continuous frames;
    // the latest cache is enough for a visibility gate.
    let Ok(rgb) = capturer.capture_rect_rgb_ref(bounds) else {
        return false;
    };
    let search = rgb_capture_to_image_buf(rgb);
    let kernel = search_blur_kernel(gate.blur);
    let search_blurred = match blur_image_owned(search, kernel) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let threshold = gate.tolerance as f32;
    let method = gate.match_method;
    let search_prep = prepare_search(&search_blurred);

    for target in &gate.targets {
        let paths = catalog.variant_paths(target);
        if paths.is_empty() {
            continue;
        }
        let mask_path = catalog.mask_path(target);
        for path in paths {
            let Ok(template_blurred) = get_cached_blurred_template(&path, kernel) else {
                continue;
            };
            let tmpl_w = template_blurred.width;
            let tmpl_h = template_blurred.height;
            if tmpl_w == 0 || tmpl_h == 0 {
                continue;
            }
            let mask_bytes = mask_path
                .as_ref()
                .and_then(|p| get_cached_image_mask(p, tmpl_h, tmpl_w));
            let Ok(prepared) = get_cached_prepared_template(
                &path,
                kernel,
                template_blurred.as_ref(),
                mask_path.as_deref(),
                mask_bytes.as_deref().map(|m| m.as_slice()),
                method,
            ) else {
                continue;
            };
            let Ok(hits) = find_template_matches_preblurred_with_prepared(
                &search_blurred,
                template_blurred.as_ref(),
                &prepared,
                threshold,
                close_dist,
                method,
                Some(&search_prep),
            ) else {
                continue;
            };
            if !hits.is_empty() {
                return true;
            }
        }
    }
    false
}
