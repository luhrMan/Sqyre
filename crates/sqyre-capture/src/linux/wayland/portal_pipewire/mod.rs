//! PipeWire stream connect and frame wait for portal ScreenCast.

use super::portal_session::stage_mutation_recent;
use crate::error::CaptureError;
use crate::linux::session::{LinuxCaptureBackend, LinuxSessionInfo};
use image::RgbaImage;
use parking_lot::{Condvar, Mutex};
use pipewire as pw;
use pw::spa::param::video::VideoInfoRaw;
use sqyre_ports::{cap_log, mark_site};
use sqyre_ports::{DesktopRect, RgbCapture, APP_ID};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

mod frame_slot;
mod pw_thread;
mod spa_params;
use frame_slot::{FrameCache, FrameSlot};

/// Overall budget to obtain a PipeWire frame newer than the cache at call start.
/// Idle / emit-on-damage streams may need a kick + one frame interval after input.
const FRESH_CAPTURE_BUDGET: Duration = Duration::from_millis(200);
/// Brief no-kick wait only when a region frame arrived very recently (continuous /
/// max-framerate streams). Stale caches skip this and kick without waiting.
const SPONTANEOUS_WAIT: Duration = Duration::from_millis(16);
/// Overlay visibility polls: wait this long for a spontaneous frame, never kick.
const QUIET_CAPTURE_WAIT: Duration = Duration::from_millis(150);
/// Post-pulse wait when a cache crop had to induce damage itself — emit-on-damage
/// streams (COSMIC; Mutter on a fully idle output) send nothing spontaneously.
/// The layer-shell pulse roundtrips Wayland concurrently, so keep it short.
const QUIET_KICK_WAIT: Duration = Duration::from_millis(200);
/// Region copy older than this → treat as stale and kick without spontaneous wait.
pub(super) const STALE_REGION_AGE: Duration = Duration::from_millis(40);
/// Post-kick wait slices (short first; kick runs concurrently so we poll soon).
const POST_KICK_SLICES: &[Duration] = &[
    Duration::from_millis(20),
    Duration::from_millis(40),
    Duration::from_millis(80),
];
/// Log successful fresh waits that exceed this (stderr + diag when enabled).
const SLOW_FRESH_LOG: Duration = Duration::from_millis(80);
/// Minimum damage pulses before giving up when other monitors advance but the
/// search region stays stuck.
const EARLY_BAIL_MIN_PULSES: u32 = 1;
/// Damage pulse max edge — emit-on-damage portals send a whole-output frame for
/// any damage on the output, so a small rect suffices; a monitor-sized layer
/// surface allocates a huge shm per pulse and can stall mapping it.
pub(super) const KICK_RECT_MAX: i32 = 64;
/// Consecutive armed waits (damage pulsed, still no copy) on one stream dest
/// before the capturer counts as dead — PipeWire can keep reporting Streaming
/// while the producer silently stopped emitting.
pub(super) const STARVED_KICKS_DEAD: u32 = 3;

/// One "no frame" log per drought — reset whenever a frame copies into cache so
/// a stuck emit-on-damage stream does not spam stderr on every crop retry.
pub(super) static LOGGED_FRAME_DROUGHT: AtomicBool = AtomicBool::new(false);

/// `node.suspend-on-idle` for portal streams. On by default: an idle node
/// suspends, and the next capture waits out the driver restart.
static SUSPEND_ON_IDLE: AtomicBool = AtomicBool::new(true);
/// Streams currently in [`pw::stream::StreamState::Streaming`].
static STREAMING: AtomicU32 = AtomicU32::new(0);
/// Monotonic ms when `STREAMING` last rose from 0. `0` means none are up.
static STREAMING_SINCE: AtomicU64 = AtomicU64::new(0);
/// In-flight captures that started while the stream was not stably awake.
static WAKE_REQUESTS: AtomicU32 = AtomicU32::new(0);
/// Monotonic ms until which a finished wake still counts, so the gap before
/// the next retry does not blink the loader off. `0` means none.
static WAKE_HOLD_UNTIL: AtomicU64 = AtomicU64::new(0);

/// Bridge between capture retries (100ms poll, then backoff) while the node
/// is still restarting, so the loader does not drop and start again.
const WAKE_HOLDOVER: Duration = Duration::from_millis(500);
/// Driver restart reports Streaming, then Paused, then Streaming again.
/// The wake ends only after Streaming holds this long.
const WAKE_STABLE: Duration = Duration::from_millis(150);

/// True for one continuous wake: from the capture that found the stream
/// asleep until a stream has stayed up, including driver-restart blips and
/// the gap before the next retry.
pub(crate) fn pipewire_stream_waking() -> bool {
    wake_visible(
        SUSPEND_ON_IDLE.load(Ordering::SeqCst),
        WAKE_REQUESTS.load(Ordering::SeqCst),
        mono_ms() < WAKE_HOLD_UNTIL.load(Ordering::SeqCst),
        stream_is_stable(),
    )
}

/// `holdover` is the post-attempt bridge; `stable` means a stream has stayed
/// in Streaming long enough that the restart is over.
fn wake_visible(suspend_on_idle: bool, wake_requests: u32, holdover: bool, stable: bool) -> bool {
    suspend_on_idle && (wake_requests > 0 || (holdover && !stable))
}

fn wake_stream_stable(now_ms: u64, streaming_since_ms: u64, stable_ms: u64) -> bool {
    streaming_since_ms > 0 && now_ms.saturating_sub(streaming_since_ms) >= stable_ms
}

fn mono_ms() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

/// Apply the settings toggle. An open session is updated on the PipeWire thread.
pub(crate) fn set_pipewire_suspend_on_idle(suspend: bool) {
    SUSPEND_ON_IDLE.store(suspend, Ordering::SeqCst);
    if !suspend {
        WAKE_HOLD_UNTIL.store(0, Ordering::SeqCst);
    }
    if let Some(Ok(capturer)) = crate::shared_capturer_if_ready() {
        capturer.apply_pipewire_suspend_on_idle(suspend);
    }
}

fn account_stream_state(old: &pw::stream::StreamState, new: &pw::stream::StreamState) {
    let was = matches!(old, pw::stream::StreamState::Streaming);
    let now = matches!(new, pw::stream::StreamState::Streaming);
    if now && !was {
        let prev = STREAMING.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            // `0` is the asleep sentinel, so a clock reading of 0 stamps as 1.
            STREAMING_SINCE.store(mono_ms().max(1), Ordering::SeqCst);
        }
    } else if was && !now {
        let prev = STREAMING.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1));
        if prev == Ok(1) {
            STREAMING_SINCE.store(0, Ordering::SeqCst);
        }
    }
}

/// Held for one capture wait that is part of a wake.
struct WakeTicket(bool);

fn arm_wake_if_asleep() -> WakeTicket {
    if !SUSPEND_ON_IDLE.load(Ordering::SeqCst) || stream_is_stable() {
        return WakeTicket(false);
    }
    WAKE_REQUESTS.fetch_add(1, Ordering::SeqCst);
    WakeTicket(true)
}

fn stream_is_stable() -> bool {
    wake_stream_stable(
        mono_ms(),
        STREAMING_SINCE.load(Ordering::SeqCst),
        WAKE_STABLE.as_millis() as u64,
    )
}

impl Drop for WakeTicket {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        let prev = WAKE_REQUESTS.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            WAKE_HOLD_UNTIL.store(
                mono_ms().saturating_add(WAKE_HOLDOVER.as_millis() as u64),
                Ordering::SeqCst,
            );
        }
    }
}

/// Portal + PipeWire capturer for Wayland sessions.
pub struct PortalCapturer {
    frame: Arc<(Mutex<FrameSlot>, Condvar)>,
    shutdown: Arc<AtomicBool>,
    /// PipeWire stream/loop ended (Error, Unconnected after Streaming, iterate
    /// failure, or the pw thread exited) — captures fail fast instead of
    /// starving on a stream that can never deliver again.
    stream_dead: Arc<AtomicBool>,
    quit_tx: Mutex<Option<Sender<PwThreadMsg>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    /// `None` = not tried yet; `Some(None)` = unavailable; `Some(Some(_))` = ready.
    kick: Mutex<Option<Option<super::compositor_kick::DamageKick>>>,
}

enum PwThreadMsg {
    Quit,
    SetSuspendOnIdle(bool),
}

impl PortalCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        let info = LinuxSessionInfo::detect();
        if info.capture_backend() != LinuxCaptureBackend::WaylandPortal {
            return Err(CaptureError::Message(
                "portal capture not selected for this session".into(),
            ));
        }
        if crate::linux::shared_capturer_open_superseded() {
            return Err(CaptureError::Message(
                "portal ScreenCast superseded by a newer picker request".into(),
            ));
        }

        let frame = Arc::new((
            Mutex::new(FrameSlot {
                cache: FrameCache {
                    virtual_bounds: DesktopRect::default(),
                    monitor_rects: Vec::new(),
                    width: 0,
                    height: 0,
                    stride: 0,
                    pixels: Vec::new(),
                    ready: false,
                },
                generation: 0,
                region_gen: Vec::new(),
                region_copy_at: Vec::new(),
                starved_kicks: Vec::new(),
                buffering: true,
                last_demand: Instant::now(),
            }),
            Condvar::new(),
        ));
        let shutdown = Arc::new(AtomicBool::new(false));
        let stream_dead = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), CaptureError>>(1);
        let (msg_tx, msg_rx) = mpsc::channel::<PwThreadMsg>();

        let frame_thread = Arc::clone(&frame);
        let shutdown_thread = Arc::clone(&shutdown);
        let dead_thread = Arc::clone(&stream_dead);
        let handle = thread::Builder::new()
            .name("sqyre-portal-pw".into())
            .spawn(move || {
                if let Err(e) = pw_thread::portal_pw_thread(
                    frame_thread,
                    shutdown_thread,
                    dead_thread.clone(),
                    ready_tx,
                    msg_rx,
                ) {
                    cap_log("PORTAL", "fail", &format!("thread={e}"));
                }
                dead_thread.store(true, Ordering::SeqCst);
            })
            .map_err(|e| CaptureError::Message(format!("portal thread spawn: {e}")))?;

        match ready_rx.recv_timeout(Duration::from_secs(120)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                shutdown.store(true, Ordering::SeqCst);
                let _ = msg_tx.send(PwThreadMsg::Quit);
                let _ = handle.join();
                return Err(e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                shutdown.store(true, Ordering::SeqCst);
                let _ = msg_tx.send(PwThreadMsg::Quit);
                let _ = handle.join();
                return Err(CaptureError::Message(
                    "portal ScreenCast timed out (grant permission in the picker)".into(),
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = handle.join();
                return Err(CaptureError::Message(
                    "portal PipeWire thread exited before first frame".into(),
                ));
            }
        }

        let slot = frame.0.lock();
        cap_log(
            "PORTAL",
            "ok",
            &format!(
                "backend=portal+pipewire size={}x{}",
                slot.cache.width, slot.cache.height
            ),
        );
        drop(slot);

        let capturer = Self {
            frame,
            shutdown,
            stream_dead,
            quit_tx: Mutex::new(Some(msg_tx)),
            thread: Mutex::new(Some(handle)),
            kick: Mutex::new(None),
        };
        // Every PipeWire frame otherwise runs a parallel full-desktop copy while idle.
        capturer.release_cpu_frame_cache();
        Ok(capturer)
    }

    pub(crate) fn apply_pipewire_suspend_on_idle(&self, suspend: bool) {
        if let Some(tx) = self.quit_tx.lock().as_ref() {
            // The thread may already have exited; the next open reads the flag.
            let _ = tx.send(PwThreadMsg::SetSuspendOnIdle(suspend));
        }
    }

    /// Wait until a PipeWire stream that overlaps `rect` copies a frame newer than
    /// the cache at the start of this call. Prefers a spontaneous frame (no kick);
    /// then pulses layer-shell or windowed-xdg damage when available.
    ///
    /// If other streams advance while the search region stays stuck, bail early
    /// (same cache as a full timeout).
    fn wait_for_overlapping_stream_frame(&self, rect: DesktopRect) {
        let _wake = arm_wake_if_asleep();
        // Unmap staged xdg kick and restore the window that had focus before the
        // pulse — GNOME activates the kick toplevel and EIS clicks otherwise miss.
        struct KickFocusRestore {
            pre: Mutex<Option<crate::WindowInfo>>,
        }
        impl KickFocusRestore {
            fn capture_before_pulse(&self) {
                let Ok(Some(w)) = super::windows::get_active_window() else {
                    return;
                };
                if is_sqyre_managed_window(&w) {
                    return;
                }
                if w.process_path.trim().is_empty() || w.title.trim().is_empty() {
                    return;
                }
                *self.pre.lock() = Some(w);
            }

            fn restore_after_unmap(&self) {
                let Some(w) = self.pre.lock().take() else {
                    return;
                };
                match super::windows::activate_window(&w.process_path, &w.title) {
                    Ok(()) => cap_log(
                        "PORTAL",
                        "kick",
                        &format!("refocus=ok title={}", w.title.replace(' ', "_")),
                    ),
                    Err(e) => cap_log("PORTAL", "kick", &format!("refocus=fail err={e}")),
                }
            }
        }
        struct ReleaseKickStage<'a> {
            capturer: &'a PortalCapturer,
            focus: &'a KickFocusRestore,
        }
        impl Drop for ReleaseKickStage<'_> {
            fn drop(&mut self) {
                self.capturer.release_compositor_kick_stage();
                self.focus.restore_after_unmap();
            }
        }
        if self.is_dead() {
            return;
        }
        let focus = KickFocusRestore {
            pre: Mutex::new(None),
        };
        let _release = ReleaseKickStage {
            capturer: self,
            focus: &focus,
        };

        self.enable_cpu_frame_buffering();

        let (min_gen, start_global, region_stale) = {
            let slot = self.frame.0.lock();
            (
                frame_slot::region_generation(&slot, rect),
                slot.generation,
                frame_slot::region_copy_is_stale(&slot, rect),
            )
        };
        let kick_rect = {
            let slot = self.frame.0.lock();
            frame_slot::kick_damage_rect(&slot, rect)
        };
        let started = Instant::now();
        let deadline = started + FRESH_CAPTURE_BUDGET;
        // After EIS input or an idle emit-on-damage cache, skip the spontaneous
        // wait — it almost never helps and just adds latency before the kick.
        let expect_kick = region_stale || stage_mutation_recent(Duration::from_millis(400));

        if !expect_kick {
            let spontaneous = frame_slot::wait_until_region_after(
                &self.frame.0,
                &self.frame.1,
                rect,
                min_gen,
                SPONTANEOUS_WAIT.min(deadline.saturating_duration_since(Instant::now())),
            );
            if spontaneous > min_gen {
                log_fresh_ok(started, 0, min_gen, spontaneous, "spontaneous");
                return;
            }
        }

        if !self.ensure_compositor_kick() {
            if stage_mutation_recent(Duration::from_millis(500)) {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if !remaining.is_zero() {
                    let region_after = frame_slot::wait_until_region_after(
                        &self.frame.0,
                        &self.frame.1,
                        rect,
                        min_gen,
                        remaining.min(Duration::from_millis(160)),
                    );
                    if region_after > min_gen {
                        log_fresh_ok(started, 0, min_gen, region_after, "post-input");
                        return;
                    }
                }
            }
            cap_log(
                "PORTAL",
                "wait",
                &format!(
                    "result=no-kick gen={min_gen} wait_ms={}",
                    started.elapsed().as_millis()
                ),
            );
            return;
        }

        // One kick only. Re-queueing while the first xdg pulse is still running
        // stacked map/unmap work and made release_stage wait on the backlog.
        focus.capture_before_pulse();
        self.pulse_compositor_kick(kick_rect);
        let pulses = 1u32;
        let mut slice_i = 0usize;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let slice = remaining.min(POST_KICK_SLICES[slice_i.min(POST_KICK_SLICES.len() - 1)]);
            slice_i = slice_i.saturating_add(1);
            let region_after = frame_slot::wait_until_region_after(
                &self.frame.0,
                &self.frame.1,
                rect,
                min_gen,
                slice,
            );
            if region_after > min_gen {
                log_fresh_ok(started, pulses, min_gen, region_after, "kick");
                return;
            }
            let now_global = self.frame.0.lock().generation;
            if frame_slot::fresh_region_stuck_while_others_advance(
                pulses,
                EARLY_BAIL_MIN_PULSES,
                start_global,
                now_global,
                region_after,
                min_gen,
            ) {
                {
                    let mut slot = self.frame.0.lock();
                    let dest = frame_slot::overlapping_stream_dest(&slot, rect);
                    frame_slot::note_starved_kick(&mut slot, dest);
                }
                cap_log(
                    "PORTAL",
                    "wait",
                    &format!(
                        "result=early-bail gen={min_gen} now={now_global} region={region_after} pulses={pulses} wait_ms={}",
                        started.elapsed().as_millis()
                    ),
                );
                return;
            }
        }

        let (now, region) = {
            let mut slot = self.frame.0.lock();
            let dest = frame_slot::overlapping_stream_dest(&slot, rect);
            frame_slot::note_starved_kick(&mut slot, dest);
            (slot.generation, frame_slot::region_generation(&slot, rect))
        };
        cap_log(
            "PORTAL",
            "wait",
            &format!(
                "result=timeout gen={min_gen} now={now} region={region} pulses={pulses} wait_ms={}",
                started.elapsed().as_millis()
            ),
        );
    }

    /// Wait for a newer overlapping PipeWire frame without mapping a kick surface.
    fn wait_for_spontaneous_stream_frame(&self, rect: DesktopRect) -> bool {
        let _wake = arm_wake_if_asleep();
        self.enable_cpu_frame_buffering();
        let min_gen = {
            let slot = self.frame.0.lock();
            frame_slot::region_generation(&slot, rect)
        };
        let after = frame_slot::wait_until_region_after(
            &self.frame.0,
            &self.frame.1,
            rect,
            min_gen,
            QUIET_CAPTURE_WAIT,
        );
        after > min_gen
    }

    /// Open damage kick once (layer-shell, else windowed-xdg). `false` if neither.
    fn ensure_compositor_kick(&self) -> bool {
        let mut slot = self.kick.lock();
        if slot.is_none() {
            *slot = Some(super::compositor_kick::DamageKick::open());
        }
        matches!(slot.as_ref(), Some(Some(_)))
    }

    fn pulse_compositor_kick(&self, rect: DesktopRect) {
        let mut slot = self.kick.lock();
        if let Some(Some(k)) = slot.as_mut() {
            k.pulse_rect(rect);
        }
    }

    fn release_compositor_kick_stage(&self) {
        let mut slot = self.kick.lock();
        if let Some(Some(k)) = slot.as_mut() {
            k.release_stage();
        }
    }

    /// The PipeWire side ended (stream Error/Unconnected, loop failure, or the
    /// pw thread exited) — or damage pulses repeatedly drew no copy on a stream
    /// dest, meaning the producer is wedged even though PipeWire still reports
    /// Streaming. No frame can arrive on this capturer → reopen the session.
    pub fn is_dead(&self) -> bool {
        if self.stream_dead.load(Ordering::SeqCst)
            || self.thread.lock().as_ref().is_some_and(|t| t.is_finished())
        {
            return true;
        }
        self.frame
            .0
            .lock()
            .starved_kicks
            .iter()
            .any(|(_, n)| *n >= STARVED_KICKS_DEAD)
    }

    fn dead_err(&self) -> Option<CaptureError> {
        self.is_dead()
            .then(|| sqyre_ports::NotReady::StreamRestarting.into())
    }

    /// Capture after waiting for a new PipeWire frame on the overlapping stream.
    pub fn capture_rect_fresh_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        if let Some(e) = self.dead_err() {
            return Err(e);
        }
        self.wait_for_overlapping_stream_frame(rect);
        self.crop_cached_rgba_ready(rect)
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        if let Some(e) = self.dead_err() {
            return Err(e);
        }
        self.crop_cached_rgba(rect)
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        if let Some(e) = self.dead_err() {
            return Err(e);
        }
        self.crop_cached_rgb(rect)
    }

    pub fn capture_rect_rgb_fresh_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<RgbCapture, CaptureError> {
        if let Some(e) = self.dead_err() {
            return Err(e);
        }
        self.wait_for_overlapping_stream_frame(rect);
        self.crop_cached_rgb_ready(rect)
    }

    /// Crop after waiting briefly for a spontaneous overlapping frame (no kick).
    /// Second value is true when the overlapping stream copied a newer dest.
    pub fn capture_rect_rgb_quiet_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<(RgbCapture, bool), CaptureError> {
        if let Some(e) = self.dead_err() {
            return Err(e);
        }
        let fresh = self.wait_for_spontaneous_stream_frame(rect);
        // Direct crop — do not re-enter quiet wait via [`Self::ensure_frame_for_crop`].
        Ok((self.crop_cached_rgb_ready(rect)?, fresh))
    }

    /// Ensure the CPU cache has a frame before a non-fresh crop.
    ///
    /// After [`Self::release_cpu_frame_cache`], OCR / Find Pixel / editor paths that
    /// crop without `*_fresh` would otherwise hit `NotReady`. Re-enable buffering and
    /// wait quietly (no compositor kick) so search keeps working. Hot path: if the
    /// cache is already `ready`, this is a single lock check.
    ///
    /// Emit-on-damage streams (COSMIC; Mutter on a fully idle output) emit no
    /// spontaneous frames, so an empty cache would starve forever: fall back to
    /// one layer-shell pulse at the crop region. The windowed-xdg kick can steal
    /// activation, so non-fresh crops only kick when
    /// [`super::compositor_kick::DamageKick::is_focus_safe`] — overlay quiet polls
    /// bypass this and stay silent on the cache.
    fn ensure_frame_for_crop(&self, rect: DesktopRect) -> Result<(), CaptureError> {
        if let Some(e) = self.dead_err() {
            return Err(e);
        }
        let min_gen = {
            let mut slot = self.frame.0.lock();
            slot.last_demand = Instant::now();
            if slot.cache.ready {
                return Ok(());
            }
            // Capture gen before enabling so the first post-release copy wakes us.
            let min = frame_slot::region_generation(&slot, rect);
            slot.buffering = true;
            min
        };
        let _wake = arm_wake_if_asleep();
        let _ = frame_slot::wait_until_region_after(
            &self.frame.0,
            &self.frame.1,
            rect,
            min_gen,
            QUIET_CAPTURE_WAIT,
        );
        if self.frame.0.lock().cache.ready {
            return Ok(());
        }

        if self.focus_safe_kick() {
            let kick_rect = {
                let slot = self.frame.0.lock();
                frame_slot::kick_damage_rect(&slot, rect)
            };
            self.pulse_compositor_kick(kick_rect);
            let after = frame_slot::wait_until_region_after(
                &self.frame.0,
                &self.frame.1,
                rect,
                min_gen,
                QUIET_KICK_WAIT,
            );
            cap_log(
                "PORTAL",
                "wait",
                &format!("result=quiet-kick gen={min_gen}->{after}"),
            );
            let mut slot = self.frame.0.lock();
            if slot.cache.ready {
                return Ok(());
            }
            // Damage was pulsed on a focus-safe surface and nothing copied —
            // the producer may be wedged while PipeWire still reports Streaming.
            let dest = frame_slot::overlapping_stream_dest(&slot, rect);
            frame_slot::note_starved_kick(&mut slot, dest);
        }
        if self.frame.0.lock().cache.ready {
            return Ok(());
        }
        if !LOGGED_FRAME_DROUGHT.swap(true, Ordering::SeqCst) {
            cap_log("PORTAL", "wait", &format!("result=no-frame gen={min_gen}"));
        }
        Err(sqyre_ports::NotReady::NoFrameYet.into())
    }

    /// Kick only when it cannot steal activation (layer-shell). The windowed-xdg
    /// kick maps a toplevel that grabs focus on Mutter, so non-fresh crops skip it.
    fn focus_safe_kick(&self) -> bool {
        if !self.ensure_compositor_kick() {
            return false;
        }
        matches!(self.kick.lock().as_ref(), Some(Some(k)) if k.is_focus_safe())
    }

    fn crop_cached_rgba(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        self.ensure_frame_for_crop(rect)?;
        self.crop_cached_rgba_ready(rect)
    }

    fn crop_cached_rgb(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        self.ensure_frame_for_crop(rect)?;
        self.crop_cached_rgb_ready(rect)
    }

    fn crop_cached_rgba_ready(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        let slot = self.frame.0.lock();
        let crop = frame_slot::cache_crop_geom(&slot.cache, rect)?;
        frame_slot::copy_cache_rgba(&slot.cache, crop)
    }

    fn crop_cached_rgb_ready(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        let slot = self.frame.0.lock();
        let crop = frame_slot::cache_crop_geom(&slot.cache, rect)?;
        frame_slot::copy_cache_rgb(&slot.cache, crop)
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        let slot = self.frame.0.lock();
        if slot.cache.virtual_bounds.w <= 0 || slot.cache.virtual_bounds.h <= 0 {
            return Err(CaptureError::Message(
                "portal capture: virtual bounds unavailable before stream metadata".into(),
            ));
        }
        Ok(slot.cache.virtual_bounds)
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        let slot = self.frame.0.lock();
        if slot.cache.monitor_rects.is_empty() {
            return Err(CaptureError::Message(
                "portal capture: monitor layout unavailable before stream metadata".into(),
            ));
        }
        Ok(slot.cache.monitor_rects.clone())
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        Ok(self
            .monitor_rects_ref()?
            .into_iter()
            .map(|r| (r.w, r.h))
            .collect())
    }

    /// Resume copying PipeWire frames into the CPU cache (after [`Self::release_cpu_frame_cache`]).
    fn enable_cpu_frame_buffering(&self) {
        let mut slot = self.frame.0.lock();
        slot.buffering = true;
        slot.last_demand = Instant::now();
    }

    /// Drop the full-desktop RGBA CPU cache and pause further copies until the next
    /// capture wait or non-fresh crop (which re-enables buffering via
    /// [`Self::ensure_frame_for_crop`] / wait helpers). PipeWire streams stay up;
    /// only the Rust mirror is released. Image Search (`*_fresh`) and OCR / Find
    /// Pixel cache crops both recover automatically.
    pub fn release_cpu_frame_cache(&self) {
        frame_slot::release_slot_cache(&mut self.frame.0.lock());
    }

    /// [`Self::release_cpu_frame_cache`] once no capture has needed the mirror for
    /// `idle`. Returns how long until the next check is due, or `None` when the
    /// mirror is not buffering.
    pub fn release_cpu_frame_cache_if_idle(&self, idle: Duration) -> Option<Duration> {
        let mut slot = self.frame.0.lock();
        let remaining = frame_slot::idle_release_remaining(&slot, idle, Instant::now())?;
        if remaining.is_zero() {
            frame_slot::release_slot_cache(&mut slot);
            return None;
        }
        Some(remaining)
    }

    /// Bytes currently held in the CPU frame cache (0 when released / not yet filled).
    pub fn cpu_frame_cache_bytes(&self) -> usize {
        self.frame.0.lock().cache.pixels.len()
    }
}

impl Drop for PortalCapturer {
    fn drop(&mut self) {
        mark_site("portal:drop:start");
        let t0 = Instant::now();
        self.shutdown.store(true, Ordering::SeqCst);
        mark_site("portal:drop:before_kick");
        let t_kick = Instant::now();
        drop(self.kick.lock().take());
        let kick_ms = t_kick.elapsed().as_millis();
        mark_site("portal:drop:after_kick");
        if let Some(tx) = self.quit_tx.lock().take() {
            let _ = tx.send(PwThreadMsg::Quit);
        }
        mark_site("portal:drop:before_pw_join");
        let t_pw = Instant::now();
        if let Some(handle) = self.thread.lock().take() {
            // On process quit, don't block the UI thread on PipeWire / portal
            // Session::Close — the kernel reaps the thread and D-Bus disconnect
            // ends the screencast. Mid-session reset still joins.
            if sqyre_ports::process_exiting() {
                cap_log("PORTAL", "drop", "pw_join=abandon");
                mark_site("portal:drop:abandon_pw");
                // Detach: dropping JoinHandle without join does not abort the thread.
                std::mem::forget(handle);
            } else {
                let _ = handle.join();
            }
        }
        let pw_ms = t_pw.elapsed().as_millis();
        mark_site("portal:drop:after_pw_join");
        cap_log(
            "PORTAL",
            "drop",
            &format!(
                "kick_ms={kick_ms} pw_ms={pw_ms} total_ms={}",
                t0.elapsed().as_millis()
            ),
        );
    }
}

extern "C" {
    fn pw_stream_update_properties(
        stream: *mut pw::sys::pw_stream,
        dict: *const pw::spa::sys::spa_dict,
    ) -> i32;
}

fn pipewire_main_loop_err(e: pw::Error) -> CaptureError {
    CaptureError::Message(format!(
        "PipeWire main loop: {e} (host PipeWire with SPA plugins required; \
         do not bundle libpipewire in portable releases)"
    ))
}

fn propagate_ready<T>(
    ready: &Arc<SyncSender<Result<(), CaptureError>>>,
    result: Result<T, CaptureError>,
) -> Result<T, CaptureError> {
    if let Err(ref e) = result {
        let _ = ready.send(Err(e.clone()));
    }
    result
}

struct UserData {
    format: VideoInfoRaw,
    monitor_rect: DesktopRect,
}
fn log_fresh_ok(started: Instant, pulses: u32, min_gen: u64, region_after: u64, path: &str) {
    let elapsed = started.elapsed();
    // Always log slower waits so intermittent 500ms dispatches are diagnosable.
    if elapsed < SLOW_FRESH_LOG {
        return;
    }
    cap_log(
        "PORTAL",
        "fresh",
        &format!(
            "result=ok path={path} wait_ms={} pulses={pulses} gen={min_gen}->{region_after}",
            elapsed.as_millis()
        ),
    );
}

fn is_sqyre_managed_window(w: &crate::WindowInfo) -> bool {
    let title = w.title.trim();
    if title == crate::x11_focus::OVERLAY_WM_TITLE
        || title == crate::x11_focus::OVERLAY_TIP_WM_TITLE
    {
        return true;
    }
    // Kick toplevel uses empty title / app_id sqyre-kick; process name may be sqyre.
    let name = w.process_name.to_ascii_lowercase();
    name.contains(APP_ID)
}

#[cfg(test)]
mod tests;
