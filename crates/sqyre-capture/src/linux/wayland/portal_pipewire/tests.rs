//! Unit tests for portal PipeWire helpers.

use super::frame_slot::{
    cache_crop_geom, copy_cache_rgb, copy_cache_rgba, ensure_cache_contains,
    fresh_region_stuck_while_others_advance, idle_release_remaining, kick_damage_rect,
    note_region_copy, note_starved_kick, region_generation, release_slot_cache,
    shrink_cpu_frame_cache, wait_until_region_after, FrameSlot,
};
use super::spa_params::{
    cpu_buffer_type_mask, cursor_meta_size, pw_buffers_param_bytes, pw_cursor_meta_bytes,
    pw_header_meta_bytes,
};
use super::*;
use pw::spa::buffer::DataType;
use pw::spa::pod::Pod;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn empty_slot(generation: u64) -> FrameSlot {
    FrameSlot {
        cache: FrameCache {
            virtual_bounds: DesktopRect::default(),
            monitor_rects: Vec::new(),
            width: 0,
            height: 0,
            stride: 0,
            pixels: Vec::new(),
            ready: false,
        },
        generation,
        region_gen: Vec::new(),
        region_copy_at: Vec::new(),
        starved_kicks: Vec::new(),
        buffering: true,
        last_demand: Instant::now(),
    }
}

fn monitor_rect(x: i32, w: i32, h: i32) -> DesktopRect {
    DesktopRect { x, y: 0, w, h }
}

#[test]
fn screencast_stream_suspend_follows_the_setting() {
    let asleep = screencast_stream_props(Some("42"), true);
    assert_eq!(asleep.get("node.suspend-on-idle"), Some("true"));
    assert_eq!(asleep.get("target.object"), Some("42"));
    assert_eq!(asleep.get(*pw::keys::MEDIA_ROLE), Some("Screen"));
    let awake = screencast_stream_props(None, false);
    assert_eq!(awake.get("node.suspend-on-idle"), Some("false"));
    assert_eq!(awake.get("target.object"), None);
}

#[test]
fn wake_icon_is_one_sequence_until_the_stream_stays_up() {
    // Suspend off: never, even with a request in flight.
    assert!(!wake_visible(false, 1, true, false));
    // Idle: nothing asked, and no retry gap to bridge.
    assert!(!wake_visible(true, 0, false, false));
    // Capture that started asleep stays loading after a Streaming blip.
    assert!(wake_visible(true, 1, false, false));
    assert!(wake_visible(true, 1, false, true));
    // Gap before the next retry, stream still down: same sequence.
    assert!(wake_visible(true, 0, true, false));
    // Stream has stayed up and nothing is waiting: the sequence ends.
    assert!(!wake_visible(true, 0, true, true));
    assert!(!wake_visible(true, 0, false, true));
}

#[test]
fn wake_stream_is_stable_only_after_it_holds() {
    let stable_ms = WAKE_STABLE.as_millis() as u64;
    assert!(!wake_stream_stable(1_000, 0, stable_ms));
    assert!(!wake_stream_stable(1_000, 900, stable_ms));
    assert!(wake_stream_stable(1_000, 1_000 - stable_ms, stable_ms));
    assert!(wake_stream_stable(1_000, 1, stable_ms));
}

#[test]
fn cpu_buffer_types_exclude_dmabuf() {
    let mask = cpu_buffer_type_mask();
    assert_eq!(mask & (1 << DataType::DmaBuf.as_raw()), 0);
    assert_ne!(mask & (1 << DataType::MemFd.as_raw()), 0);
    assert_ne!(mask & (1 << DataType::MemPtr.as_raw()), 0);
}

#[test]
fn pipewire_cursor_and_buffer_pods_serialize() {
    let cursor = pw_cursor_meta_bytes().expect("cursor meta");
    let header = pw_header_meta_bytes().expect("header meta");
    let buffers = pw_buffers_param_bytes(8294400, 7680).expect("buffers");
    assert!(Pod::from_bytes(&cursor).is_some());
    assert!(Pod::from_bytes(&header).is_some());
    assert!(Pod::from_bytes(&buffers).is_some());
    assert!(cursor_meta_size(64, 64) > cursor_meta_size(1, 1));
}

#[test]
fn region_generation_uses_overlapping_stream_only() {
    let mut slot = empty_slot(10);
    slot.region_gen = vec![
        (monitor_rect(0, 1920, 1080), 10),
        (monitor_rect(1920, 1280, 1440), 7),
    ];
    assert_eq!(region_generation(&slot, monitor_rect(1920, 1280, 1440)), 7);
    assert_eq!(region_generation(&slot, monitor_rect(0, 1920, 1080)), 10);
}

#[test]
fn early_bail_when_other_monitor_advances_after_min_pulses() {
    assert!(fresh_region_stuck_while_others_advance(
        1,
        EARLY_BAIL_MIN_PULSES,
        364,
        366,
        353,
        353
    ));
    // No pulse yet — allow a late region frame.
    assert!(!fresh_region_stuck_while_others_advance(
        0,
        EARLY_BAIL_MIN_PULSES,
        364,
        366,
        353,
        353
    ));
    // Nothing else advanced (single-monitor / fully idle): keep waiting so
    // nested searches can still land a post-click frame.
    assert!(!fresh_region_stuck_while_others_advance(
        5,
        EARLY_BAIL_MIN_PULSES,
        353,
        353,
        353,
        353
    ));
    // Region did advance — success path, not a bail.
    assert!(!fresh_region_stuck_while_others_advance(
        1,
        EARLY_BAIL_MIN_PULSES,
        353,
        354,
        354,
        353
    ));
}

#[test]
fn kick_damage_rect_clamps_to_overlapping_monitor() {
    let mut slot = empty_slot(1);
    let mon = monitor_rect(1920, 1280, 1440);
    slot.cache.monitor_rects = vec![monitor_rect(0, 1920, 1080), mon];
    let search = DesktopRect {
        x: 2000,
        y: 100,
        w: 400,
        h: 50,
    };
    let kick = kick_damage_rect(&slot, search);
    // Damage only needs to touch the output — the pulse stays small even
    // for a wide search area (large layer surfaces stall emit-on-damage).
    assert_eq!(kick.x, 2000);
    assert_eq!(kick.y, 100);
    assert_eq!(kick.w, KICK_RECT_MAX);
    assert_eq!(kick.h, 50);
}

#[test]
fn starved_kicks_reset_when_stream_copies() {
    let mut slot = empty_slot(1);
    let mon = monitor_rect(0, 1920, 1080);
    slot.cache.monitor_rects = vec![mon];
    for _ in 0..STARVED_KICKS_DEAD - 1 {
        note_starved_kick(&mut slot, mon);
    }
    assert!(slot
        .starved_kicks
        .iter()
        .all(|(_, n)| *n < STARVED_KICKS_DEAD));
    note_starved_kick(&mut slot, mon);
    assert!(slot
        .starved_kicks
        .iter()
        .any(|(_, n)| *n >= STARVED_KICKS_DEAD));
    // A copy on that dest proves the stream still emits — restart counting.
    note_region_copy(&mut slot, mon);
    assert!(slot.starved_kicks.iter().all(|(_, n)| *n == 0));
}

#[test]
fn wait_until_region_after_ignores_other_monitor_copies() {
    let lock = Arc::new(Mutex::new(empty_slot(1)));
    {
        let mut slot = lock.lock();
        slot.region_gen = vec![
            (monitor_rect(0, 1920, 1080), 1),
            (monitor_rect(1920, 1280, 1440), 1),
        ];
    }
    let cvar = Arc::new(Condvar::new());
    let wait_lock = Arc::clone(&lock);
    let wait_cvar = Arc::clone(&cvar);
    let search = monitor_rect(1920, 1280, 1440);
    let handle = thread::spawn(move || {
        wait_until_region_after(&wait_lock, &wait_cvar, search, 1, Duration::from_secs(2))
    });
    thread::sleep(Duration::from_millis(20));
    {
        let mut slot = lock.lock();
        note_region_copy(&mut slot, monitor_rect(0, 1920, 1080));
        cvar.notify_all();
    }
    thread::sleep(Duration::from_millis(30));
    {
        let mut slot = lock.lock();
        note_region_copy(&mut slot, monitor_rect(1920, 1280, 1440));
        cvar.notify_all();
    }
    assert_eq!(handle.join().unwrap(), 3);
}

#[test]
fn ensure_cache_grows_from_empty() {
    let mut cache = FrameCache {
        virtual_bounds: DesktopRect::default(),
        monitor_rects: Vec::new(),
        width: 0,
        height: 0,
        stride: 0,
        pixels: Vec::new(),
        ready: false,
    };
    ensure_cache_contains(
        &mut cache,
        DesktopRect {
            x: 0,
            y: 0,
            w: 10,
            h: 4,
        },
    );
    assert_eq!(cache.width, 10);
    assert_eq!(cache.height, 4);
    assert_eq!(cache.pixels.len(), 10 * 4 * 4);
    shrink_cpu_frame_cache(&mut cache);
    assert!(cache.pixels.is_empty());
    assert!(!cache.ready);
    assert_eq!(cache.width, 0);
}

/// Post-macro release contract: shrink empties pixels; ensure can refill.
#[test]
fn release_cpu_frame_cache_then_refill() {
    let mut slot = empty_slot(0);
    ensure_cache_contains(
        &mut slot.cache,
        DesktopRect {
            x: 0,
            y: 0,
            w: 64,
            h: 48,
        },
    );
    slot.cache.ready = true;
    assert_eq!(slot.cache.pixels.len(), 64 * 48 * 4);

    // Mirrors PortalCapturer::release_cpu_frame_cache.
    slot.buffering = false;
    shrink_cpu_frame_cache(&mut slot.cache);
    assert!(!slot.buffering);
    assert!(!slot.cache.ready);
    assert!(slot.cache.pixels.is_empty());

    // Next capture re-enables buffering and grows the cache again.
    slot.buffering = true;
    ensure_cache_contains(
        &mut slot.cache,
        DesktopRect {
            x: 0,
            y: 0,
            w: 64,
            h: 48,
        },
    );
    slot.cache.ready = true;
    assert_eq!(slot.cache.pixels.len(), 64 * 48 * 4);
    assert!(slot.buffering);
}

#[test]
fn idle_release_waits_for_quiet_period_while_buffering() {
    let mut slot = empty_slot(0);
    let idle = Duration::from_secs(10);
    let t0 = slot.last_demand;
    assert_eq!(
        idle_release_remaining(&slot, idle, t0 + Duration::from_secs(4)),
        Some(Duration::from_secs(6))
    );
    assert_eq!(
        idle_release_remaining(&slot, idle, t0 + Duration::from_secs(11)),
        Some(Duration::ZERO)
    );
    release_slot_cache(&mut slot);
    assert!(!slot.buffering);
    assert_eq!(
        idle_release_remaining(&slot, idle, t0 + Duration::from_secs(11)),
        None
    );
}

#[test]
fn buffering_false_skips_ensure_growth_in_release_state() {
    let mut slot = empty_slot(1);
    slot.buffering = false;
    // Callback path checks buffering before ensure_cache_contains — simulate that.
    if slot.buffering {
        ensure_cache_contains(
            &mut slot.cache,
            DesktopRect {
                x: 0,
                y: 0,
                w: 32,
                h: 32,
            },
        );
    }
    assert!(
        slot.cache.pixels.is_empty(),
        "idle after release must not grow the CPU mirror"
    );
}

fn ready_cache(w: u32, h: u32, pixels: Vec<u8>) -> FrameCache {
    FrameCache {
        virtual_bounds: DesktopRect {
            x: 0,
            y: 0,
            w: w as i32,
            h: h as i32,
        },
        monitor_rects: Vec::new(),
        width: w,
        height: h,
        stride: w as usize * 4,
        pixels,
        ready: true,
    }
}

#[test]
fn copy_cache_rgb_skips_alpha() {
    let cache = ready_cache(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 128]);
    let crop = cache_crop_geom(
        &cache,
        DesktopRect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        },
    )
    .unwrap();
    let rgb = copy_cache_rgb(&cache, crop).unwrap();
    assert_eq!(rgb.width, 2);
    assert_eq!(rgb.height, 1);
    assert_eq!(rgb.data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn copy_cache_rgb_crops_subrect() {
    // 2x2 RGBA: row0 (1,2,3,a)(4,5,6,a)  row1 (7,8,9,a)(10,11,12,a)
    let cache = ready_cache(
        2,
        2,
        vec![1, 2, 3, 9, 4, 5, 6, 9, 7, 8, 9, 9, 10, 11, 12, 9],
    );
    let crop = cache_crop_geom(
        &cache,
        DesktopRect {
            x: 1,
            y: 1,
            w: 1,
            h: 1,
        },
    )
    .unwrap();
    let rgb = copy_cache_rgb(&cache, crop).unwrap();
    assert_eq!(rgb.data, vec![10, 11, 12]);
    let rgba = copy_cache_rgba(&cache, crop).unwrap();
    assert_eq!(rgba.as_raw(), &[10, 11, 12, 9]);
}

#[test]
fn ensure_cache_does_not_invent_monitors() {
    let mut cache = FrameCache {
        virtual_bounds: DesktopRect::default(),
        monitor_rects: vec![DesktopRect {
            x: 0,
            y: 0,
            w: 10,
            h: 4,
        }],
        width: 0,
        height: 0,
        stride: 0,
        pixels: Vec::new(),
        ready: false,
    };
    ensure_cache_contains(
        &mut cache,
        DesktopRect {
            x: 0,
            y: 0,
            w: 20,
            h: 4,
        },
    );
    assert_eq!(cache.monitor_rects.len(), 1);
    assert_eq!(cache.width, 20);
}
