//! Unit tests for the X11 button host.

use super::visibility::{allow_map_raised, MAP_RAISED_SKIP_COUNT};
use super::paint::{circle_inset, rounded_rect_xrectangles};
use super::*;

#[test]
fn button_crossing_deferred_during_click_through() {
    assert!(button_crossing_deferred(true, false, false));
    assert!(button_crossing_deferred(false, true, false));
    assert!(button_crossing_deferred(false, false, true));
    assert!(!button_crossing_deferred(false, false, false));
}

#[test]
fn tip_flips_above_and_clamps_to_monitor() {
    let mon = crate::os_popup::MonitorRect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    };
    let (x, y) = crate::os_popup::place_tip_near_button(100, 1000, 40, 40, 120, 28, mon);
    assert!(y + 28 <= 1080, "below would clip → flip/clamp y={y}");
    assert!(x >= 0 && x + 120 <= 1920);
    let (x2, _) = crate::os_popup::place_tip_near_button(1900, 100, 40, 40, 120, 28, mon);
    assert!(x2 + 120 <= 1920, "right-edge tip clamped x={x2}");
}

#[test]
fn click_through_follows_foreign_x11_focus() {
    assert!(
        !next_click_through(false, true),
        "pure X11 never forces click-through"
    );
    assert!(!next_click_through(false, false));
    assert!(
        !next_click_through(true, false),
        "desktop / Wayland focus → interactive hits"
    );
    assert!(
        next_click_through(true, true),
        "foreign X11 focus → empty hits for the whole period"
    );
}

#[test]
fn map_raised_blocked_during_click_through() {
    assert!(
        allow_map_raised(false),
        "desktop focus may restack with MapRaised"
    );
    assert!(
        !allow_map_raised(true),
        "foreign X11 / click-through must not MapRaised"
    );
}

/// Live regression: with overlay buttons mapped and a foreign X11 client
/// focused (click-through), `raise_all_hits` must skip `XMapRaised`. Pre-fix
/// MapRaised during that period dropped held mouse buttons on Mutter/XWayland.
///
/// ```text
/// cargo test -p sqyre-overlay --lib live_click_through_hold -- --ignored --nocapture
/// # or: make overlay-hold-check
/// ```
#[test]
#[ignore = "requires graphical Wayland+XWayland session"]
fn live_click_through_hold_survives_stall_raise_window() {
    live_click_through_map_raised_skipped_inner();
}

fn live_click_through_map_raised_skipped_inner() {
    use std::collections::HashMap;
    use std::ffi::CString;
    use std::ptr;
    use x11::xlib::{PropModeReplace, XChangeProperty, XRaiseWindow, XA_WINDOW as XA_WINDOW_ATOM};
    use x11::xtest::{XTestFakeButtonEvent, XTestQueryExtension};

    let session = sqyre_capture::LinuxSessionInfo::detect();
    assert!(
        session.has_wayland,
        "click-through hold guard only arms on Wayland (passthrough_hits)"
    );

    // SAFETY: null name → $DISPLAY; checked before use.
    let display = unsafe { XOpenDisplay(ptr::null()) };
    assert!(
        !display.is_null(),
        "XOpenDisplay failed — need X11/XWayland DISPLAY"
    );

    let cleanup = |dpy: *mut Display, game: Window| {
        // SAFETY: caller-opened display; game may be 0.
        unsafe {
            if game != 0 {
                XDestroyWindow(dpy, game);
            }
            XCloseDisplay(dpy);
        }
    };

    // SAFETY: display live; DefaultRootWindow / screen from that display.
    let (root, screen, game) = unsafe {
        let screen = XDefaultScreen(display);
        let root = XDefaultRootWindow(display);
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.background_pixel = 0;
        let game = XCreateWindow(
            display,
            root,
            80,
            80,
            640,
            480,
            0,
            XDefaultDepth(display, screen),
            InputOutput as c_uint,
            XDefaultVisual(display, screen),
            CWBackPixel,
            &mut attrs,
        );
        assert!(game != 0, "failed to create foreign X11 fixture window");
        let title = CString::new("sqyre-overlay-hold-fixture").expect("cstr");
        XStoreName(display, game, title.as_ptr());
        XMapRaised(display, game);
        XFlush(display);
        (root, screen, game)
    };
    let _ = screen;

    // SAFETY: display/root/game live; property buffer is a single Window.
    unsafe {
        let Ok(name) = CString::new("_NET_ACTIVE_WINDOW") else {
            cleanup(display, game);
            panic!("atom name");
        };
        let atom = XInternAtom(display, name.as_ptr(), False);
        let mut win = game;
        XChangeProperty(
            display,
            root,
            atom,
            XA_WINDOW_ATOM,
            32,
            PropModeReplace,
            (&raw mut win).cast::<u8>(),
            1,
        );
        XRaiseWindow(display, game);
        XSync(display, False);
    }

    let pending = Arc::new(Mutex::new(Vec::new()));
    let pending_moves = Arc::new(Mutex::new(Vec::new()));
    let running = Arc::new(Mutex::new(None));
    let pending_chooser = Arc::new(Mutex::new(None));
    let pending_context = Arc::new(Mutex::new(Vec::new()));
    let visibility = Arc::new(Mutex::new(HashMap::new()));
    let run_stop = Arc::new(Mutex::new(sqyre_hotkeys::StopFlag::new()));
    let press_stops = Arc::new(AtomicBool::new(false));

    let host = match X11ButtonHost::start(
        Arc::clone(&pending),
        pending_moves,
        Arc::clone(&running),
        pending_chooser,
        pending_context,
        visibility,
        run_stop,
        press_stops,
    ) {
        Ok(h) => h,
        Err(e) => {
            cleanup(display, game);
            panic!("overlay host start failed: {e:?}");
        }
    };

    host.set_buttons(vec![NativeButtonSpec {
        id: "hold-bench".into(),
        macro_name: "hold-bench".into(),
        face: crate::ButtonFace {
            x: 8,
            y: 8,
            w: 48,
            h: 48,
            bg: [20, 20, 20, 200],
            border: [200, 160, 40, 255],
            border_width: 2.0,
            corner_radius: 8.0,
            icon_glyph: '+',
            icon: [255, 255, 255, 255],
            icon_hover: [200, 200, 200, 255],
            brand: false,
        },
        tip: String::new(),
        ui_scale: 1.0,
        busy: false,
        visibility_gated: false,
        input_enabled: true,
    }]);

    // Map + observe foreign focus → click-through.
    std::thread::sleep(Duration::from_millis(250));

    // Busy→idle calls raise_all_hits; under click-through that must skip MapRaised.
    MAP_RAISED_SKIP_COUNT.store(0, Ordering::Relaxed);
    *running.lock() = Some("hold-bench".into());
    std::thread::sleep(Duration::from_millis(150));
    *running.lock() = None;
    std::thread::sleep(Duration::from_millis(200));

    let skips = MAP_RAISED_SKIP_COUNT.load(Ordering::Relaxed);
    assert!(
        skips >= 1,
        "expected raise_all_hits to skip MapRaised while click-through (count={skips})"
    );

    // Best-effort hold check: XTest often cannot latch Button1Mask on Mutter.
    let mut event_base = 0;
    let mut error_base = 0;
    let mut major = 0;
    let mut minor = 0;
    // SAFETY: display live; out-params are stack locals.
    let xtest_ok = unsafe {
        XTestQueryExtension(
            display,
            &mut event_base,
            &mut error_base,
            &mut major,
            &mut minor,
        )
    } != 0;
    if xtest_ok {
        // SAFETY: display live.
        unsafe {
            XTestFakeButtonEvent(display, 1, True, CurrentTime);
            XFlush(display);
        }
        std::thread::sleep(Duration::from_millis(50));
        // SAFETY: display still open; root from that display.
        if unsafe { button1_down(display, root) } {
            std::thread::sleep(Duration::from_millis(1200));
            // SAFETY: same display/root invariant as above.
            let still = unsafe { button1_down(display, root) };
            // SAFETY: release before teardown.
            unsafe {
                XTestFakeButtonEvent(display, 1, False, CurrentTime);
                XFlush(display);
            }
            assert!(
                still,
                "Button1 dropped across the stall-raise window despite MapRaised skip"
            );
        } else {
            eprintln!(
                "overlay-hold-check: XTest Button1 did not latch (common on Mutter); \
                 MapRaised-skip assertion already passed"
            );
            // SAFETY: clear any partial inject.
            unsafe {
                XTestFakeButtonEvent(display, 1, False, CurrentTime);
                XFlush(display);
            }
        }
    } else {
        eprintln!("overlay-hold-check: XTest unavailable; MapRaised-skip assertion passed");
    }

    host.shutdown();
    cleanup(display, game);
}

/// SAFETY: `display` open; `root` from that display.
unsafe fn button1_down(display: *mut Display, root: Window) -> bool {
    let mut root_ret = 0;
    let mut child = 0;
    let mut root_x = 0;
    let mut root_y = 0;
    let mut win_x = 0;
    let mut win_y = 0;
    let mut mask = 0u32;
    // SAFETY: caller guarantees open display + valid root.
    let ok = unsafe {
        XQueryPointer(
            display,
            root,
            &mut root_ret,
            &mut child,
            &mut root_x,
            &mut root_y,
            &mut win_x,
            &mut win_y,
            &mut mask,
        )
    };
    ok != 0 && (mask & Button1Mask) != 0
}

#[test]
fn passthrough_hit_prefers_aligned_portal_when_x_frozen() {
    assert_eq!(
        passthrough_hit_xy(10, 20, Some((100, 200)), false, true, true),
        (100, 200),
        "X frozen + portal moving + aligned"
    );
    assert_eq!(
        passthrough_hit_xy(10, 20, Some((100, 200)), false, false, true),
        (10, 20),
        "stopped cursor keeps X even when portal aligned"
    );
    assert_eq!(
        passthrough_hit_xy(10, 20, Some((100, 200)), false, true, false),
        (10, 20),
        "unaligned Cosmic portal must not drive hits"
    );
    assert_eq!(
        passthrough_hit_xy(10, 20, Some((100, 200)), true, true, true),
        (10, 20),
        "moving X wins over portal"
    );
}

#[test]
fn relocate_pointer_stays_in_x_root_space() {
    let x11 = (10, 20);
    let portal = Some((100, 200));
    assert_eq!(
        relocate_pointer_xy(x11, portal, false, true, true),
        (100, 200),
        "aligned portal while X frozen and portal moving"
    );
    assert_eq!(
        relocate_pointer_xy(x11, portal, false, false, true),
        x11,
        "stopped cursor must not jump to portal"
    );
    assert_eq!(
        relocate_pointer_xy(x11, portal, false, true, false),
        x11,
        "unaligned Cosmic portal must not drive relocate"
    );
    assert_eq!(
        relocate_pointer_xy(x11, portal, true, true, true),
        x11,
        "moving X wins over portal"
    );
}

#[test]
fn pointer_moved_ignores_the_first_sample() {
    assert!(!pointer_moved(None, Some((1, 2))));
    assert!(!pointer_moved(Some((1, 2)), Some((1, 2))));
    assert!(pointer_moved(Some((1, 2)), Some((2, 2))));
    assert!(!pointer_moved(Some((1, 2)), None));
}

#[test]
fn rounded_rect_strips_cover_full_height() {
    // Radius 0 → a full-width strip per row.
    let rects = rounded_rect_xrectangles(10, 4, 0);
    assert_eq!(rects.len(), 4);
    assert!(rects.iter().all(|r| r.x == 0 && r.width == 10));

    // Degenerate sizes produce no strips.
    assert!(rounded_rect_xrectangles(0, 4, 2).is_empty());
    assert!(rounded_rect_xrectangles(4, 0, 2).is_empty());

    // Radius clamps to half the smaller dimension.
    let clamped = rounded_rect_xrectangles(8, 6, 100);
    let r2 = rounded_rect_xrectangles(8, 6, 3);
    assert_eq!(clamped, r2, "radius clamps at min(w,h)/2");

    // Corner rows are inset; middle rows are full width.
    let rects = rounded_rect_xrectangles(20, 20, 5);
    assert_eq!(rects.len(), 20);
    let corner = rects[0];
    assert!(corner.x > 0 && corner.width < 20, "top row is inset");
    let mid = rects[10];
    assert_eq!((mid.x, mid.width), (0, 20), "middle row is full width");
    // Symmetric: last row mirrors the first.
    let last = *rects.last().unwrap();
    assert_eq!(last.x, corner.x);
    assert_eq!(last.width, corner.width);
}

#[test]
fn circle_inset_monotonic_and_edge_safe() {
    // `y_from_edge` is the row's distance inward from the circle boundary:
    // inset grows as the row approaches the far side of the arc.
    let r = 8;
    let mut prev = 0;
    for y in 0..r {
        let ins = circle_inset(r, y);
        assert!(ins >= 0);
        assert!(ins <= r, "inset never exceeds radius");
        assert!(ins >= prev, "inset non-decreasing away from the edge");
        prev = ins;
    }
    assert_eq!(circle_inset(r, 0), 0, "row on the boundary needs no inset");
    assert_eq!(circle_inset(r, r), r, "opposite edge is fully inset");
    // Negative distances clamp to the boundary.
    assert_eq!(circle_inset(r, -5), 0);
}
