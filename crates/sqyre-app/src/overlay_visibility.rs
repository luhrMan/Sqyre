//! Background image-search gate for overlay buttons (show when found).
//!
//! Polls run on a dedicated thread so they do not wait for starved egui ROOT
//! frames under fullscreen games. The X11 host reads [`Self::found_map`] and
//! maps/unmaps gated buttons without a UI frame.

use eframe::egui;
use parking_lot::Mutex;
use rayon::prelude::*;
use sqyre_capture::{
    event_log, get_active_window, mark_site, note, shared_capturer, window_is_our_process,
    window_is_transient_shell_focus, window_matches_binding, window_matches_program, WindowInfo,
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Retry quickly when the portal cache did not advance (do not burn a full interval).
const STALE_RETRY: Duration = Duration::from_millis(50);

#[derive(Clone, Copy)]
enum LastAttempt {
    Fresh(Instant),
    Stale(Instant),
}

/// Deadline for keeping a gated button visible without a newer portal frame.
struct VisHold {
    until: Instant,
}

/// Worker-local hold / stale-block state for the shared found-map.
struct GateVisTracker {
    holds: HashMap<String, VisHold>,
    /// After a stale-frame expiry, require a fresh hit before showing again (same
    /// frozen portal pixels must not restart the hold).
    needs_fresh: HashMap<String, ()>,
}

impl GateVisTracker {
    fn new() -> Self {
        Self {
            holds: HashMap::new(),
            needs_fresh: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.holds.clear();
        self.needs_fresh.clear();
    }

    /// Apply one image-gate poll to the found-map.
    ///
    /// Stale portal frames (`fresh=false`) must not keep buttons mapped indefinitely:
    /// a hold started by a stale hit expires after [`stale_hold`], and then the id is
    /// blocked until a fresh hit (same frozen pixels cannot re-arm the hold).
    ///
    /// Returns a diag tag when visibility flips: `hit`, `miss`, or `stale-expire`.
    fn apply_match(
        &mut self,
        found: &Mutex<HashMap<String, bool>>,
        id: &str,
        matched: bool,
        fresh: bool,
        interval_ms: u64,
        now: Instant,
    ) -> Option<&'static str> {
        if matched && fresh {
            self.needs_fresh.remove(id);
            self.holds.insert(
                id.to_string(),
                VisHold {
                    until: now + stale_hold(interval_ms),
                },
            );
            let prev = found.lock().insert(id.to_string(), true);
            return (prev != Some(true)).then_some("hit");
        }
        if !matched {
            self.needs_fresh.remove(id);
            self.holds.remove(id);
            let prev = found.lock().insert(id.to_string(), false);
            return (prev == Some(true)).then_some("miss");
        }
        // Stale hit — frozen portal cache still matches the gate template.
        if self.needs_fresh.contains_key(id) {
            let prev = found.lock().insert(id.to_string(), false);
            self.holds.remove(id);
            return (prev == Some(true)).then_some("stale-expire");
        }
        match self.holds.get(id) {
            Some(h) if now < h.until => {
                let prev = found.lock().insert(id.to_string(), true);
                (prev != Some(true)).then_some("hit")
            }
            Some(_) => {
                // Hold expired without a fresh renew.
                self.holds.remove(id);
                self.needs_fresh.insert(id.to_string(), ());
                let prev = found.lock().insert(id.to_string(), false);
                (prev == Some(true)).then_some("stale-expire")
            }
            None => {
                // Optimistic brief show from stale; cannot renew without fresh.
                self.holds.insert(
                    id.to_string(),
                    VisHold {
                        until: now + stale_hold(interval_ms),
                    },
                );
                let prev = found.lock().insert(id.to_string(), true);
                (prev != Some(true)).then_some("hit")
            }
        }
    }

    /// Capture/match failed — hide and require a later fresh hit.
    fn mark_unverified(&mut self, found: &Mutex<HashMap<String, bool>>, id: &str) -> bool {
        self.holds.remove(id);
        self.needs_fresh.insert(id.to_string(), ());
        found.lock().insert(id.to_string(), false) == Some(true)
    }

    /// Drop stale hits for buttons we are not allowed to poll (wrong focus).
    fn clear_ineligible(
        &mut self,
        found: &Mutex<HashMap<String, bool>>,
        buttons: &[OverlayButtonConfig],
        focus: Option<&WindowInfo>,
        catalog: &ProgramCatalog,
    ) -> usize {
        let mut map = found.lock();
        let mut n = 0usize;
        for btn in buttons {
            if poll_eligible(btn, focus, catalog) {
                continue;
            }
            self.holds.remove(&btn.id);
            self.needs_fresh.remove(&btn.id);
            if map.get(&btn.id).copied() == Some(true) {
                map.insert(btn.id.clone(), false);
                n += 1;
            }
        }
        n
    }
}

#[derive(Clone)]
struct PollSnapshot {
    buttons: Vec<OverlayButtonConfig>,
    catalog: ProgramCatalog,
    close_dist: i32,
    last_foreign: Option<WindowInfo>,
}

struct Inner {
    found: Arc<Mutex<HashMap<String, bool>>>,
    snapshot: Mutex<Option<PollSnapshot>>,
    wake: Mutex<Option<egui::Context>>,
    stop: AtomicBool,
    /// Set while a macro run owns capture/match so this worker does not contend
    /// for the portal cache or the global rayon pool.
    paused: AtomicBool,
}

/// What the poller was last configured with: `(button id + gate pairs,
/// close-match distance, catalog resolution key)`. Re-polling is skipped while
/// this is unchanged.
type PollSignature = (Vec<(String, OverlayVisibilityGate)>, i32, String);

/// Background image-match results used to filter overlay buttons.
pub struct OverlayVisibilityPoller {
    inner: Arc<Inner>,
    join: Mutex<Option<JoinHandle<()>>>,
    last_sig: Option<PollSignature>,
}

impl Default for OverlayVisibilityPoller {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for OverlayVisibilityPoller {
    fn drop(&mut self) {
        self.inner.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.lock().take() {
            let _ = join.join();
        }
    }
}

impl OverlayVisibilityPoller {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                found: Arc::new(Mutex::new(HashMap::new())),
                snapshot: Mutex::new(None),
                wake: Mutex::new(None),
                stop: AtomicBool::new(false),
                paused: AtomicBool::new(false),
            }),
            join: Mutex::new(None),
            last_sig: None,
        }
    }

    /// Shared found-map the X11 overlay host reads to map/unmap gated buttons.
    pub fn found_map(&self) -> Arc<Mutex<HashMap<String, bool>>> {
        Arc::clone(&self.inner.found)
    }

    /// Pause image-gate polling while a macro is running.
    ///
    /// On resume, clear prior hits so gated buttons hide until a fresh poll
    /// confirms the gate is still on screen (avoids stale `found=true` after
    /// the gate left during the macro).
    pub fn set_paused(&self, paused: bool) {
        let was = self.inner.paused.swap(paused, Ordering::Relaxed);
        if was && !paused {
            let cleared = clear_found_hits(&self.inner.found);
            if cleared > 0 {
                note(&format!("overlay-vis: resume clear-found n={cleared}"));
            }
        }
    }

    #[cfg(test)]
    fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::Relaxed)
    }

    /// True when the button may be drawn (gate off, or last poll found a match).
    /// Active gates stay hidden until the first successful poll result arrives.
    #[cfg(test)]
    fn allows_draw(&self, btn: &OverlayButtonConfig) -> bool {
        if !btn.visibility_gate.is_active() {
            return true;
        }
        self.inner
            .found
            .lock()
            .get(&btn.id)
            .copied()
            .unwrap_or(false)
    }

    /// Push the latest gated-button snapshot and ensure the worker is running.
    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        buttons: &[OverlayButtonConfig],
        catalog: &ProgramCatalog,
        close_matches_distance: i32,
        last_foreign: Option<WindowInfo>,
    ) {
        *self.inner.wake.lock() = Some(ctx.clone());
        self.ensure_worker();

        let gated: Vec<OverlayButtonConfig> = buttons
            .iter()
            .filter(|b| {
                b.enabled && !b.macro_name.trim().is_empty() && b.visibility_gate.is_active()
            })
            .cloned()
            .collect();
        let close_dist = close_matches_distance.clamp(0, 100);
        let sig = (
            gated
                .iter()
                .map(|b| (b.id.clone(), b.visibility_gate.clone()))
                .collect::<Vec<_>>(),
            close_dist,
            catalog.resolution_key().to_string(),
        );
        let mut slot = self.inner.snapshot.lock();
        if self.last_sig.as_ref() == Some(&sig) {
            if let Some(snap) = slot.as_mut() {
                snap.last_foreign = last_foreign;
            }
            return;
        }
        self.last_sig = Some(sig);
        *slot = Some(PollSnapshot {
            buttons: gated,
            catalog: catalog.clone(),
            close_dist,
            last_foreign,
        });
    }

    fn ensure_worker(&self) {
        let mut slot = self.join.lock();
        if slot.is_some() {
            return;
        }
        let inner = Arc::clone(&self.inner);
        let join = thread::Builder::new()
            .name("sqyre-overlay-vis".into())
            .spawn(move || vis_worker_loop(inner))
            .ok();
        *slot = join;
    }
}

fn clear_found_hits(found: &Mutex<HashMap<String, bool>>) -> usize {
    let mut map = found.lock();
    let mut n = 0usize;
    for v in map.values_mut() {
        if *v {
            *v = false;
            n += 1;
        }
    }
    n
}

/// Max time a button may stay up on stale portal pixels without a fresh frame.
fn stale_hold(interval_ms: u64) -> Duration {
    Duration::from_millis(interval_ms.clamp(1, 2_000))
}

fn focus_label(focus: Option<&WindowInfo>) -> String {
    let Some(w) = focus else {
        return "(none)".into();
    };
    let name = w.process_name.trim();
    let path = w.process_path.trim();
    let title = w.title.trim();
    if !name.is_empty() {
        name.to_string()
    } else if !path.is_empty() {
        path.to_string()
    } else if !title.is_empty() {
        format!("title:{title}")
    } else {
        "(empty)".into()
    }
}

fn vis_worker_loop(inner: Arc<Inner>) {
    note("overlay-vis: worker started");
    let mut last_poll: HashMap<String, LastAttempt> = HashMap::new();
    let mut tracker = GateVisTracker::new();
    let mut last_foreign: Option<WindowInfo> = None;
    let mut rr = 0usize;
    let mut paused_logged = false;
    while !inner.stop.load(Ordering::Relaxed) {
        if inner.paused.load(Ordering::Relaxed) {
            if !paused_logged {
                note("overlay-vis: pause macro-running");
                paused_logged = true;
            }
            sleep_while_running(&inner.stop, Duration::from_millis(50));
            continue;
        }
        if paused_logged {
            note("overlay-vis: resume");
            paused_logged = false;
            tracker.clear();
        }
        let snap = inner.snapshot.lock().clone();
        let Some(snap) = snap else {
            sleep_while_running(&inner.stop, Duration::from_millis(50));
            continue;
        };
        if snap.buttons.is_empty() {
            sleep_while_running(&inner.stop, Duration::from_millis(50));
            continue;
        }

        if last_foreign.is_none() {
            last_foreign = snap.last_foreign.clone();
        }
        let focus = resolve_poll_focus(&mut last_foreign);
        let cleared = tracker.clear_ineligible(
            &inner.found,
            &snap.buttons,
            focus.as_ref(),
            &snap.catalog,
        );
        if cleared > 0 {
            note(&format!(
                "overlay-vis: clear-ineligible n={cleared} focus={}",
                focus_label(focus.as_ref())
            ));
            if let Some(ctx) = inner.wake.lock().clone() {
                nudge_overlay_repaint(ctx);
            }
        }
        let now = Instant::now();
        if let Some((idx, btn)) = next_due(
            &snap.buttons,
            &last_poll,
            now,
            rr,
            focus.as_ref(),
            &snap.catalog,
        ) {
            rr = idx.wrapping_add(1);
            let t0 = Instant::now();
            let Some((matched, fresh)) =
                match_once(&snap.catalog, &btn.visibility_gate, snap.close_dist)
            else {
                last_poll.insert(btn.id.clone(), LastAttempt::Stale(Instant::now()));
                if tracker.mark_unverified(&inner.found, &btn.id) {
                    note(&format!("overlay-vis: unverified-clear id={}", btn.id));
                    if let Some(ctx) = inner.wake.lock().clone() {
                        nudge_overlay_repaint(ctx);
                    }
                }
                continue;
            };
            let ms = t0.elapsed().as_millis();
            last_poll.insert(
                btn.id.clone(),
                if fresh {
                    LastAttempt::Fresh(Instant::now())
                } else {
                    LastAttempt::Stale(Instant::now())
                },
            );
            let now = Instant::now();
            if let Some(gate) = tracker.apply_match(
                &inner.found,
                &btn.id,
                matched,
                fresh,
                btn.visibility_gate.interval_ms,
                now,
            ) {
                mark_site(&format!("overlay-vis:{gate}:{}", btn.id));
                note(&format!(
                    "overlay-vis: {gate} id={} ms={ms} interval={} fresh={fresh}",
                    btn.id, btn.visibility_gate.interval_ms
                ));
                event_log(
                    "SQYRE_OVERLAY",
                    &[
                        ("gate", gate),
                        ("id", btn.id.as_str()),
                        ("ms", &ms.to_string()),
                        ("fresh", if fresh { "1" } else { "0" }),
                    ],
                );
                if let Some(ctx) = inner.wake.lock().clone() {
                    nudge_overlay_repaint(ctx);
                }
            }
            continue;
        }

        let wait = next_wait(
            &snap.buttons,
            &last_poll,
            Instant::now(),
            focus.as_ref(),
            &snap.catalog,
        );
        sleep_while_running(&inner.stop, wait);
    }
    note("overlay-vis: worker stopped");
}

fn nudge_overlay_repaint(ctx: egui::Context) {
    ctx.request_repaint();
    let _ = thread::Builder::new()
        .name("sqyre-overlay-vis-wake".into())
        .spawn(move || {
            for _ in 0..8 {
                thread::sleep(Duration::from_millis(25));
                ctx.request_repaint();
            }
        });
}

fn sleep_while_running(stop: &AtomicBool, dur: Duration) {
    let deadline = Instant::now() + dur;
    while Instant::now() < deadline {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let remain = deadline.saturating_duration_since(Instant::now());
        thread::sleep(remain.min(Duration::from_millis(16)));
    }
}

/// Keep polling through overlay-chrome / Sqyre / None blips so the found-map
/// is already current when the game is focused again.
fn resolve_poll_focus(last_foreign: &mut Option<WindowInfo>) -> Option<WindowInfo> {
    match get_active_window() {
        Ok(Some(w)) if window_is_transient_shell_focus(&w) => last_foreign.clone(),
        Ok(Some(w)) if window_is_our_process(&w) => last_foreign.clone(),
        Ok(Some(w)) => {
            *last_foreign = Some(w.clone());
            Some(w)
        }
        Ok(None) | Err(_) => last_foreign.clone(),
    }
}

fn next_due<'a>(
    buttons: &'a [OverlayButtonConfig],
    last_poll: &HashMap<String, LastAttempt>,
    now: Instant,
    start: usize,
    focus: Option<&WindowInfo>,
    catalog: &ProgramCatalog,
) -> Option<(usize, &'a OverlayButtonConfig)> {
    let n = buttons.len();
    if n == 0 {
        return None;
    }
    let start = start % n;
    for k in 0..n {
        let i = (start + k) % n;
        let btn = &buttons[i];
        if !poll_eligible(btn, focus, catalog) {
            continue;
        }
        if is_due(btn, last_poll, now) {
            return Some((i, btn));
        }
    }
    None
}

fn next_wait(
    buttons: &[OverlayButtonConfig],
    last_poll: &HashMap<String, LastAttempt>,
    now: Instant,
    focus: Option<&WindowInfo>,
    catalog: &ProgramCatalog,
) -> Duration {
    let mut soonest: Option<Duration> = None;
    for btn in buttons {
        if !poll_eligible(btn, focus, catalog) {
            continue;
        }
        if last_poll.get(&btn.id).is_none() {
            return Duration::ZERO;
        }
        if !is_due(btn, last_poll, now) {
            let remain = remain_until_due(btn, last_poll, now);
            soonest = Some(match soonest {
                Some(s) => s.min(remain),
                None => remain,
            });
        }
    }
    soonest
        .unwrap_or(Duration::from_millis(50))
        .max(Duration::from_millis(16))
}

fn remain_until_due(
    btn: &OverlayButtonConfig,
    last_poll: &HashMap<String, LastAttempt>,
    now: Instant,
) -> Duration {
    match last_poll.get(&btn.id) {
        None => Duration::ZERO,
        Some(LastAttempt::Stale(t)) => {
            STALE_RETRY.saturating_sub(now.saturating_duration_since(*t))
        }
        Some(LastAttempt::Fresh(t)) => {
            Duration::from_millis(btn.visibility_gate.interval_ms.max(1))
                .saturating_sub(now.saturating_duration_since(*t))
        }
    }
}

fn is_due(
    btn: &OverlayButtonConfig,
    last_poll: &HashMap<String, LastAttempt>,
    now: Instant,
) -> bool {
    match last_poll.get(&btn.id) {
        None => true,
        Some(LastAttempt::Stale(t)) => now.saturating_duration_since(*t) >= STALE_RETRY,
        Some(LastAttempt::Fresh(t)) => {
            now.saturating_duration_since(*t)
                >= Duration::from_millis(btn.visibility_gate.interval_ms.max(1))
        }
    }
}

fn poll_eligible(
    btn: &OverlayButtonConfig,
    focus: Option<&WindowInfo>,
    catalog: &ProgramCatalog,
) -> bool {
    if !btn.enabled || btn.macro_name.trim().is_empty() || !btn.visibility_gate.is_active() {
        return false;
    }
    if !button_is_focus_gated(btn) {
        return true;
    }
    // Fullscreen XWayland often reports no active window. Keep polling so the
    // found-map does not freeze until the user alt-tabs (which populates last_foreign).
    focus.is_none() || program_owns_focus(catalog, &btn.program, focus)
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

fn match_once(
    catalog: &ProgramCatalog,
    gate: &OverlayVisibilityGate,
    close_dist: i32,
) -> Option<(bool, bool)> {
    if gate.targets.is_empty() || gate.search_area.trim().is_empty() {
        return Some((false, true));
    }
    let macro_ = Macro::new("", 0, vec![]);
    let area = CoordinateRef(gate.search_area.clone());
    let (lx, ty, rx, by) = catalog.resolve_search_area(&area, &macro_).ok()?;
    let capturer = shared_capturer().ok()?;
    let vb = capturer.virtual_bounds_ref().ok();
    let bounds = clamp_search_rect(lx, ty, rx, by, vb).ok()?;
    // Never `*_fresh_ref` — compositor kick steals game focus. X11 is live;
    // portal waits briefly for a spontaneous frame, then reports whether it
    // advanced so the worker can retry instead of sleeping a full interval.
    let (rgb, fresh) = capturer.capture_rect_rgb_quiet_ref(bounds).ok()?;
    let search = rgb_capture_to_image_buf(rgb);
    let kernel = search_blur_kernel(gate.blur);
    let search_blurred = match blur_image_owned(search, kernel) {
        Ok(b) => b,
        Err(_) => return None,
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
        // Warm caches on this thread before parallel variant match (same deadlock
        // class as image search: inflight gates + nested FFT rayon).
        for path in &paths {
            let Ok(template_blurred) = get_cached_blurred_template(path, kernel) else {
                continue;
            };
            let mask_bytes = mask_path.as_ref().and_then(|p| {
                get_cached_image_mask(p, template_blurred.height, template_blurred.width)
            });
            let _ = get_cached_prepared_template(
                path,
                kernel,
                template_blurred.as_ref(),
                mask_path.as_deref(),
                mask_bytes.as_deref().map(|m| m.as_slice()),
                method,
            );
            if mask_path.is_none() {
                search_prep.warm_fft_for_template(
                    &search_blurred,
                    template_blurred.width,
                    template_blurred.height,
                );
            }
        }
        let hit = paths.par_iter().any(|path| {
            let Ok(template_blurred) = get_cached_blurred_template(path, kernel) else {
                return false;
            };
            let tmpl_w = template_blurred.width;
            let tmpl_h = template_blurred.height;
            if tmpl_w == 0 || tmpl_h == 0 {
                return false;
            }
            let mask_bytes = mask_path
                .as_ref()
                .and_then(|p| get_cached_image_mask(p, tmpl_h, tmpl_w));
            let Ok(prepared) = get_cached_prepared_template(
                path,
                kernel,
                template_blurred.as_ref(),
                mask_path.as_deref(),
                mask_bytes.as_deref().map(|m| m.as_slice()),
                method,
            ) else {
                return false;
            };
            find_template_matches_preblurred_with_prepared(
                &search_blurred,
                template_blurred.as_ref(),
                &prepared,
                threshold,
                close_dist,
                method,
                Some(&search_prep),
            )
            .ok()
            .is_some_and(|hits| !hits.is_empty())
        });
        if hit {
            return Some((true, fresh));
        }
    }
    Some((false, fresh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::OverlayVisibilityMode;

    fn gated_btn(id: &str, interval_ms: u64) -> OverlayButtonConfig {
        let mut btn = OverlayButtonConfig::new(id, GENERAL_PROGRAM);
        btn.enabled = true;
        btn.macro_name = "m".into();
        btn.visibility_gate.mode = OverlayVisibilityMode::ShowWhenFound;
        btn.visibility_gate.interval_ms = interval_ms;
        btn
    }

    #[test]
    fn allows_draw_hides_until_found() {
        let p = OverlayVisibilityPoller::new();
        let btn = gated_btn("a", 100);
        assert!(!p.allows_draw(&btn));
        p.found_map().lock().insert("a".into(), true);
        assert!(p.allows_draw(&btn));
        let mut off = btn.clone();
        off.visibility_gate.mode = OverlayVisibilityMode::Off;
        assert!(p.allows_draw(&off));
    }

    #[test]
    fn set_paused_roundtrip() {
        let p = OverlayVisibilityPoller::new();
        assert!(!p.is_paused());
        p.set_paused(true);
        assert!(p.is_paused());
        p.set_paused(false);
        assert!(!p.is_paused());
    }

    #[test]
    fn resume_clears_stale_found_hits() {
        let p = OverlayVisibilityPoller::new();
        p.found_map().lock().insert("a".into(), true);
        p.found_map().lock().insert("b".into(), false);
        p.set_paused(true);
        assert_eq!(p.found_map().lock().get("a").copied(), Some(true));
        p.set_paused(false);
        assert_eq!(p.found_map().lock().get("a").copied(), Some(false));
        assert_eq!(p.found_map().lock().get("b").copied(), Some(false));
    }

    #[test]
    fn clear_ineligible_drops_stale_hits() {
        let mut btn = OverlayButtonConfig::new("a", "Mistfall Hunter");
        btn.enabled = true;
        btn.macro_name = "m".into();
        btn.visibility_gate.mode = OverlayVisibilityMode::ShowWhenFound;
        let catalog = ProgramCatalog::default();
        let found = Mutex::new(HashMap::from([("a".into(), true)]));
        let mut tracker = GateVisTracker::new();
        let other = WindowInfo {
            title: "Firefox".into(),
            process_name: "firefox".into(),
            process_path: "/usr/bin/firefox".into(),
            icon: None,
        };
        let n = tracker.clear_ineligible(&found, &[btn], Some(&other), &catalog);
        assert_eq!(n, 1);
        assert_eq!(found.lock().get("a").copied(), Some(false));
    }

    #[test]
    fn stale_hit_expires_and_blocks_rearm_until_fresh() {
        let found = Mutex::new(HashMap::new());
        let mut tracker = GateVisTracker::new();
        let t0 = Instant::now();

        assert_eq!(
            tracker.apply_match(&found, "a", true, false, 1_000, t0),
            Some("hit")
        );
        assert_eq!(found.lock().get("a").copied(), Some(true));

        // Within hold — stay up, no flip.
        assert_eq!(
            tracker.apply_match(
                &found,
                "a",
                true,
                false,
                1_000,
                t0 + Duration::from_millis(500)
            ),
            None
        );
        assert_eq!(found.lock().get("a").copied(), Some(true));

        // Past hold — expire and require fresh.
        assert_eq!(
            tracker.apply_match(
                &found,
                "a",
                true,
                false,
                1_000,
                t0 + Duration::from_millis(1_001)
            ),
            Some("stale-expire")
        );
        assert_eq!(found.lock().get("a").copied(), Some(false));

        // Same frozen frame must not re-show.
        assert_eq!(
            tracker.apply_match(
                &found,
                "a",
                true,
                false,
                1_000,
                t0 + Duration::from_millis(1_050)
            ),
            None
        );
        assert_eq!(found.lock().get("a").copied(), Some(false));

        // Fresh hit re-arms.
        assert_eq!(
            tracker.apply_match(
                &found,
                "a",
                true,
                true,
                1_000,
                t0 + Duration::from_millis(1_100)
            ),
            Some("hit")
        );
        assert_eq!(found.lock().get("a").copied(), Some(true));
    }

    #[test]
    fn fresh_hit_renews_hold_across_stale_polls() {
        let found = Mutex::new(HashMap::new());
        let mut tracker = GateVisTracker::new();
        let t0 = Instant::now();

        assert_eq!(
            tracker.apply_match(&found, "a", true, true, 1_000, t0),
            Some("hit")
        );
        // Stale hit still inside renewed hold.
        assert_eq!(
            tracker.apply_match(
                &found,
                "a",
                true,
                false,
                1_000,
                t0 + Duration::from_millis(900)
            ),
            None
        );
        assert_eq!(found.lock().get("a").copied(), Some(true));
    }

    #[test]
    fn next_due_prefers_never_polled_in_round_robin() {
        let a = gated_btn("a", 1_000);
        let b = gated_btn("b", 1_000);
        let buttons = vec![a, b];
        let catalog = ProgramCatalog::default();
        let now = Instant::now();
        let last = HashMap::new();
        let (i, btn) = next_due(&buttons, &last, now, 0, None, &catalog).unwrap();
        assert_eq!(i, 0);
        assert_eq!(btn.id, "a");
        let (i, btn) = next_due(&buttons, &last, now, 1, None, &catalog).unwrap();
        assert_eq!(i, 1);
        assert_eq!(btn.id, "b");
    }

    #[test]
    fn next_due_skips_until_interval() {
        let a = gated_btn("a", 1_000);
        let buttons = vec![a];
        let catalog = ProgramCatalog::default();
        let now = Instant::now();
        let mut last = HashMap::new();
        last.insert("a".into(), LastAttempt::Fresh(now));
        assert!(next_due(&buttons, &last, now, 0, None, &catalog).is_none());
        let later = now + Duration::from_millis(1_000);
        assert!(next_due(&buttons, &last, later, 0, None, &catalog).is_some());
    }

    #[test]
    fn stale_attempt_is_due_before_interval() {
        let a = gated_btn("a", 1_000);
        let buttons = vec![a];
        let catalog = ProgramCatalog::default();
        let now = Instant::now();
        let mut last = HashMap::new();
        last.insert("a".into(), LastAttempt::Stale(now));
        assert!(next_due(&buttons, &last, now, 0, None, &catalog).is_none());
        let later = now + STALE_RETRY;
        assert!(next_due(&buttons, &last, later, 0, None, &catalog).is_some());
    }

    #[test]
    fn unknown_focus_keeps_image_gate_polling() {
        let mut btn = OverlayButtonConfig::new("a", "Mistfall Hunter");
        btn.enabled = true;
        btn.macro_name = "m".into();
        btn.visibility_gate.mode = OverlayVisibilityMode::ShowWhenFound;
        let catalog = ProgramCatalog::default();
        assert!(poll_eligible(&btn, None, &catalog));
        let other = WindowInfo {
            title: "Firefox".into(),
            process_name: "firefox".into(),
            process_path: "/usr/bin/firefox".into(),
            icon: None,
        };
        assert!(!poll_eligible(&btn, Some(&other), &catalog));
    }
}
