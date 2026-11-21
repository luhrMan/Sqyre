//! Frame cache / slot helpers for portal PipeWire capture.

use super::{KICK_RECT_MAX, LOGGED_FRAME_DROUGHT, STALE_REGION_AGE};
use crate::error::CaptureError;
use crate::linux::wayland::portal_session::{rects_overlap, union_rect};
use image::RgbaImage;
use parking_lot::{Condvar, Mutex};
use sqyre_ports::{cap_log, DesktopRect, RgbCapture};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub(super) struct FrameSlot {
    pub(super) cache: FrameCache,
    /// Incremented on every PipeWire frame copied into `cache`.
    pub(super) generation: u64,
    /// Last global generation at which each stream dest was copied. Search waits
    /// on the dests that overlap the crop so the other monitor cannot starve it.
    pub(super) region_gen: Vec<(DesktopRect, u64)>,
    /// Last copy time per stream dest (stale region → kick immediately).
    pub(super) region_copy_at: Vec<(DesktopRect, Instant)>,
    /// Consecutive armed waits that produced no copy, per stream dest.
    /// `STARVED_KICKS_DEAD` on any dest → capturer dead → session reopens.
    pub(super) starved_kicks: Vec<(DesktopRect, u32)>,
    /// When false, PipeWire callbacks dequeue but skip copying into `cache.pixels`.
    /// Cleared after a macro so the full-desktop RGBA buffer can be released;
    /// re-enabled when a capture waits for a frame.
    pub(super) buffering: bool,
    /// Last capture that needed the CPU mirror (buffering enable or cache crop).
    pub(super) last_demand: Instant,
}

pub(super) struct FrameCache {
    pub(super) virtual_bounds: DesktopRect,
    pub(super) monitor_rects: Vec<DesktopRect>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) stride: usize,
    pub(super) pixels: Vec<u8>,
    pub(super) ready: bool,
}

pub(super) fn shrink_cpu_frame_cache(cache: &mut FrameCache) {
    cache.pixels.clear();
    cache.pixels.shrink_to_fit();
    cache.width = 0;
    cache.height = 0;
    cache.stride = 0;
    cache.ready = false;
}

pub(super) fn release_slot_cache(slot: &mut FrameSlot) {
    slot.buffering = false;
    let before = slot.cache.pixels.len();
    let gen = slot.generation;
    shrink_cpu_frame_cache(&mut slot.cache);
    if before > 0 {
        cap_log(
            "PORTAL",
            "cache",
            &format!("release_kib={} gen={gen}", before / 1024),
        );
    }
}

/// Time left before an idle buffering mirror should be released (zero = now).
pub(super) fn idle_release_remaining(
    slot: &FrameSlot,
    idle: Duration,
    now: Instant,
) -> Option<Duration> {
    slot.buffering
        .then(|| idle.saturating_sub(now.saturating_duration_since(slot.last_demand)))
}

pub(super) fn note_region_copy(slot: &mut FrameSlot, dest: DesktopRect) {
    LOGGED_FRAME_DROUGHT.store(false, Ordering::Relaxed);
    slot.generation = slot.generation.saturating_add(1);
    let gen = slot.generation;
    let now = Instant::now();
    if let Some((_, g)) = slot.region_gen.iter_mut().find(|(r, _)| *r == dest) {
        *g = gen;
    } else {
        slot.region_gen.push((dest, gen));
    }
    if let Some((_, t)) = slot.region_copy_at.iter_mut().find(|(r, _)| *r == dest) {
        *t = now;
    } else {
        slot.region_copy_at.push((dest, now));
    }
    if let Some((_, n)) = slot.starved_kicks.iter_mut().find(|(r, _)| *r == dest) {
        *n = 0;
    }
}

/// A damage pulse on `dest`'s monitor produced no copy — bump its starved
/// count. At `STARVED_KICKS_DEAD` the capturer reports dead so the shared
/// portal session reopens instead of failing crops on a wedged stream forever.
pub(super) fn note_starved_kick(slot: &mut FrameSlot, dest: DesktopRect) {
    let n = if let Some((_, n)) = slot.starved_kicks.iter_mut().find(|(r, _)| *r == dest) {
        *n = n.saturating_add(1);
        *n
    } else {
        // `overlapping_stream_dest` falls back to the crop rect when no monitor
        // overlaps, so tracked dests can be unbounded — cap the set.
        if slot.starved_kicks.len() >= 16 {
            slot.starved_kicks.remove(0);
        }
        slot.starved_kicks.push((dest, 1));
        1
    };
    cap_log(
        "PORTAL",
        "wait",
        &format!(
            "result=starved-kick n={n} dest={}x{}+{}+{}",
            dest.w, dest.h, dest.x, dest.y
        ),
    );
}

pub(super) fn region_copy_is_stale(slot: &FrameSlot, rect: DesktopRect) -> bool {
    let age = slot
        .region_copy_at
        .iter()
        .filter(|(r, _)| rects_overlap(*r, rect))
        .map(|(_, t)| t.elapsed())
        .min();
    match age {
        None => true,
        Some(a) => a >= STALE_REGION_AGE,
    }
}

pub(super) fn region_generation(slot: &FrameSlot, rect: DesktopRect) -> u64 {
    slot.region_gen
        .iter()
        .filter(|(r, _)| rects_overlap(*r, rect))
        .map(|(_, g)| *g)
        .max()
        .unwrap_or(0)
}

pub(super) fn overlapping_stream_dest(slot: &FrameSlot, rect: DesktopRect) -> DesktopRect {
    slot.cache
        .monitor_rects
        .iter()
        .copied()
        .find(|d| rects_overlap(*d, rect))
        .unwrap_or(rect)
}

/// Damage region for a compositor kick: the search crop, clamped into the
/// overlapping monitor and capped small. Emit-on-damage portals emit a
/// whole-output frame for any damage on the output, so a big overlay only
/// makes the pulse expensive (large layer-shell shm, slow map) without
/// improving the chance a frame arrives.
pub(super) fn kick_damage_rect(slot: &FrameSlot, rect: DesktopRect) -> DesktopRect {
    let dest = overlapping_stream_dest(slot, rect);
    let left = rect.x.max(dest.x);
    let top = rect.y.max(dest.y);
    let right = (rect.x + rect.w).min(dest.x + dest.w);
    let bottom = (rect.y + rect.h).min(dest.y + dest.h);
    let (x, y, w, h) = if right - left < 2 || bottom - top < 2 {
        (dest.x, dest.y, dest.w, dest.h)
    } else {
        (left, top, right - left, bottom - top)
    };
    DesktopRect {
        x,
        y,
        w: w.min(KICK_RECT_MAX),
        h: h.min(KICK_RECT_MAX),
    }
}

#[derive(Clone, Copy)]
pub(super) struct CacheCrop {
    pub(super) src_x: u32,
    pub(super) src_y: u32,
    pub(super) out_w: u32,
    pub(super) out_h: u32,
}

pub(super) fn cache_crop_geom(
    cache: &FrameCache,
    rect: DesktopRect,
) -> Result<CacheCrop, CaptureError> {
    if rect.is_empty() {
        return Err(CaptureError::EmptyRect);
    }
    if !cache.ready {
        return Err(sqyre_ports::NotReady::NoFrameYet.into());
    }

    let vb = cache.virtual_bounds;
    let left = rect.x.max(vb.x);
    let top = rect.y.max(vb.y);
    let right = (rect.x + rect.w).min(vb.x + vb.w);
    let bottom = (rect.y + rect.h).min(vb.y + vb.h);
    if right <= left || bottom <= top {
        return Err(CaptureError::OutsideVirtualDesktop);
    }

    Ok(CacheCrop {
        src_x: (left - vb.x) as u32,
        src_y: (top - vb.y) as u32,
        out_w: (right - left) as u32,
        out_h: (bottom - top) as u32,
    })
}

pub(super) fn copy_cache_rgba(
    cache: &FrameCache,
    crop: CacheCrop,
) -> Result<RgbaImage, CaptureError> {
    let out_w = crop.out_w as usize;
    let out_h = crop.out_h as usize;
    let mut out = vec![0u8; out_w * out_h * 4];
    let src_stride = cache.stride;
    let pixels = cache.pixels.as_slice();
    for row in 0..out_h {
        let dst_off = row * out_w * 4;
        let src_off = (crop.src_y as usize + row) * src_stride + crop.src_x as usize * 4;
        let end = src_off + out_w * 4;
        if end > pixels.len() {
            return Err(CaptureError::Message(
                "portal capture: frame buffer shorter than expected".into(),
            ));
        }
        out[dst_off..dst_off + out_w * 4].copy_from_slice(&pixels[src_off..end]);
    }
    RgbaImage::from_raw(crop.out_w, crop.out_h, out)
        .ok_or_else(|| CaptureError::Message("portal capture: RGBA size mismatch".into()))
}

pub(super) fn copy_cache_rgb(
    cache: &FrameCache,
    crop: CacheCrop,
) -> Result<RgbCapture, CaptureError> {
    use crate::pixel_convert::{strip_rgba_row_to_rgb, PARALLEL_ROW_GATE};
    use rayon::prelude::*;

    let out_w = crop.out_w as usize;
    let out_h = crop.out_h as usize;
    let mut out = vec![0u8; out_w * out_h * 3];
    let src_stride = cache.stride;
    let pixels = cache.pixels.as_slice();

    // Validate all rows before parallel strip (and fail fast on short buffers).
    for row in 0..out_h {
        let src_off = (crop.src_y as usize + row) * src_stride + crop.src_x as usize * 4;
        if src_off + out_w * 4 > pixels.len() {
            return Err(CaptureError::Message(
                "portal capture: frame buffer shorter than expected".into(),
            ));
        }
    }

    let out_addr = out.as_mut_ptr() as usize;
    let strip_row = |row: usize| {
        let src_off = (crop.src_y as usize + row) * src_stride + crop.src_x as usize * 4;
        let src = &pixels[src_off..src_off + out_w * 4];
        let dst_off = row * out_w * 3;
        // SAFETY: each crop row writes a disjoint `out` span.
        let dst = unsafe {
            std::slice::from_raw_parts_mut((out_addr as *mut u8).add(dst_off), out_w * 3)
        };
        strip_rgba_row_to_rgb(src, out_w, dst);
    };

    if out_h >= PARALLEL_ROW_GATE {
        (0..out_h).into_par_iter().for_each(strip_row);
    } else {
        for row in 0..out_h {
            strip_row(row);
        }
    }
    Ok(RgbCapture {
        width: crop.out_w,
        height: crop.out_h,
        data: out,
    })
}

/// True when the search-region stream stayed idle while another stream advanced.
///
/// Single-monitor (or fully idle) sessions keep `now_global == start_global` until
/// the overlapping dest copies, so this stays false and pulse retries continue —
/// required for nested image search after a click when the kick needs a few tries.
pub(super) fn fresh_region_stuck_while_others_advance(
    pulses: u32,
    min_pulses: u32,
    start_global: u64,
    now_global: u64,
    region_after: u64,
    min_gen: u64,
) -> bool {
    pulses >= min_pulses && region_after <= min_gen && now_global > start_global
}

/// Wait until a stream dest overlapping `rect` has generation `> min_gen`.
pub(super) fn wait_until_region_after(
    lock: &Mutex<FrameSlot>,
    cvar: &Condvar,
    rect: DesktopRect,
    min_gen: u64,
    timeout: Duration,
) -> u64 {
    let deadline = Instant::now() + timeout;
    let mut slot = lock.lock();
    while region_generation(&slot, rect) <= min_gen {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        cvar.wait_for(&mut slot, remaining);
    }
    region_generation(&slot, rect)
}

pub(super) fn ensure_cache_contains(cache: &mut FrameCache, rect: DesktopRect) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let vb = if cache.virtual_bounds.w <= 0 || cache.virtual_bounds.h <= 0 {
        rect
    } else {
        union_rect(cache.virtual_bounds, rect)
    };
    if vb == cache.virtual_bounds && !cache.pixels.is_empty() {
        return;
    }
    cache.virtual_bounds = vb;
    cache.width = vb.w.max(0) as u32;
    cache.height = vb.h.max(0) as u32;
    cache.stride = cache.width as usize * 4;
    let len = cache.stride.saturating_mul(cache.height as usize);
    cache.pixels.resize(len, 0);
}
