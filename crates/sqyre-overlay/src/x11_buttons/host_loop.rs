//! X11 overlay host thread loop and HostCmd dispatch.

use super::geometry::apply_hub_fan_cmd;
use super::visibility::{
    apply_gate_visibility, apply_running_busy, raise_all_hits, sync_press_stops,
};
use super::*;

pub(super) fn host_loop(
    cmd_rx: Receiver<HostCmd>,
    pending: Arc<Mutex<Vec<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    running_macro: Arc<Mutex<Option<String>>>,
    pending_chooser: Arc<Mutex<Option<HotkeyChooserResult>>>,
    pending_context: Arc<Mutex<Vec<crate::OverlayButtonContextAction>>>,
    visibility: Arc<Mutex<HashMap<String, bool>>>,
    stop: Arc<AtomicBool>,
    run_stop: Arc<Mutex<sqyre_hotkeys::StopFlag>>,
    press_stops_flag: Arc<AtomicBool>,
) {
    // SAFETY: connects to the default display; pointer is owned by this thread
    // until unregister + x_close_display below (or the early-error path).
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        note("overlay-x11: XOpenDisplay failed");
        return;
    }
    register_secondary_x_display(display.cast());

    // Separate Display for pointer samples — never issue ConfigureWindow on it.
    // QueryPointer on the configure connection stalls under fullscreen XWayland.
    // SAFETY: null name selects $DISPLAY; result is null-checked before every use.
    let ptr_display = unsafe { XOpenDisplay(std::ptr::null()) };
    if !ptr_display.is_null() {
        register_secondary_x_display(ptr_display.cast());
    }

    // SAFETY: `display` just opened and non-null.
    let screen = unsafe { XDefaultScreen(display) };
    // SAFETY: `display` is open and non-null.
    let root = unsafe { XDefaultRootWindow(display) };
    // SAFETY: `display` is open and `root` belongs to it; valuemask 0 means `values` is unread.
    let gc = unsafe { XCreateGC(display, root, 0, std::ptr::null_mut()) };
    if gc.is_null() {
        note("overlay-x11: XCreateGC failed");
        if !ptr_display.is_null() {
            unregister_secondary_x_display(ptr_display.cast());
            // SAFETY: non-null `ptr_display` is unregistered and not used again after close.
            unsafe { x_close_display(ptr_display) };
        }
        unregister_secondary_x_display(display.cast());
        // SAFETY: display still live; nothing else holds it.
        unsafe { x_close_display(display) };
        return;
    }
    // SAFETY: `display` non-null; fd is valid for poll until XCloseDisplay.
    let xfd = unsafe { XConnectionNumber(display) };
    // SAFETY: host-thread display; cursors freed in destroy_all before close.
    let move_cursor = unsafe { XCreateFontCursor(display, XC_FLEUR) };
    // SAFETY: same live host-thread `display`; freed in destroy_all before close.
    let arrow_cursor = unsafe { XCreateFontCursor(display, XC_LEFT_PTR) };

    // `_NET_WM_CM_S<n>` ownership is only an X11-compositor signal: on Wayland
    // sessions the Wayland compositor composites XWayland windows without
    // necessarily claiming the X selection, so checking the selection alone
    // false-negatives on e.g. COSMIC and sends hit covers down the InputOnly
    // path that swallows presses over native Wayland windows.
    // SAFETY: `display` is the host-thread open Display; `cm` is nul-terminated.
    let composited = sqyre_capture::LinuxSessionInfo::detect().has_wayland
        || unsafe {
            let cm = std::ffi::CString::new(format!("_NET_WM_CM_S{screen}")).unwrap_or_default();
            let atom = XInternAtom(display, cm.as_ptr(), True);
            atom != 0 && XGetSelectionOwner(display, atom) != 0
        };

    // SAFETY: `display` is the host-thread open Display; zeroed XVisualInfo is a
    // valid out-param for XMatchVisualInfo.
    let argb = unsafe {
        let mut vinfo: XVisualInfo = std::mem::zeroed();
        if composited
            && XMatchVisualInfo(display, screen, 32, TrueColor, &mut vinfo) != 0
            && !vinfo.visual.is_null()
        {
            Some(ArgbVisual {
                visual: vinfo.visual,
                depth: vinfo.depth,
                red_mask: (*vinfo.visual).red_mask,
                green_mask: (*vinfo.visual).green_mask,
                blue_mask: (*vinfo.visual).blue_mask,
            })
        } else {
            None
        }
    };

    // Wayland compositors confine an X11 game pointer to the surface under the
    // cursor. A hit window with ShapeInput is a different surface, so the cursor
    // stops at the button edge. `passthrough_hits` arms focus-scoped click-through
    // while a foreign X11 client is focused.
    let passthrough_hits =
        sqyre_capture::LinuxSessionInfo::detect().has_wayland && xfixes_supported(display);

    let mut state = HostState {
        display,
        ptr_display,
        screen,
        root,
        gc,
        buttons: HashMap::new(),
        tip: None,
        chooser: None,
        shield: None,
        xfd,
        pending,
        pending_moves,
        pending_chooser,
        pending_context,
        visibility,
        wake: None,
        relocate_mode: false,
        relocate_keep_above: false,
        drag: None,
        move_cursor,
        arrow_cursor,
        suppress_crossing: false,
        argb,
        argb_gc: std::ptr::null_mut(),
        run_stop,
        press_stops: press_stops_flag.load(Ordering::Relaxed),
        press_stops_flag,
        running_label: None,
        capture_omit: false,
        passthrough_hits,
        click_through: false,
        last_x11_pos: None,
        last_portal_pos: None,
        portal_aligned: false,
        passthrough_hover: None,
        hub_fan_dismiss: false,
        last_hub_dismiss_btn1: false,
        last_chooser_dismiss_btn1: false,
        hub_fan: crate::hub_fan::HubFanState::default(),
    };

    note(&format!(
        "overlay-x11: host thread started (override-redirect buttons; ptr_conn={}; hit_input=shape; click_through=off)",
        if ptr_display.is_null() {
            "shared"
        } else {
            "separate"
        },
    ));
    event_log(
        "SQYRE_OVERLAY",
        &[
            ("hit_input", "shape"),
            ("session", if passthrough_hits { "wayland" } else { "x11" }),
            ("click_through", "off"),
        ],
    );
    let mut last_busy_tick = Instant::now();
    let mut last_x_growth = Instant::now();
    let mut stall_raised = false;
    let mut last_relocate_raise = Instant::now();
    let mut last_btn1 = false;
    let mut last_btn3 = false;
    let mut last_running: Option<String> = None;

    while !stop.load(Ordering::Relaxed) {
        loop {
            match cmd_rx.try_recv() {
                Ok(HostCmd::SetButtons(specs)) => {
                    apply_specs(&mut state, specs);
                    apply_running_busy(&mut state, last_running.as_deref());
                    if state.relocate_mode && state.relocate_keep_above && state.drag.is_none() {
                        raise_all_hits(&state);
                        last_relocate_raise = Instant::now();
                    }
                }
                Ok(HostCmd::SetRelocateMode(enabled)) => set_relocate_mode(&mut state, enabled),
                Ok(HostCmd::SetRelocateKeepAbove(enabled)) => {
                    if state.relocate_keep_above != enabled {
                        state.relocate_keep_above = enabled;
                        note(&format!("overlay-x11: relocate_keep_above={enabled}"));
                        if enabled && state.relocate_mode && state.drag.is_none() {
                            raise_all_hits(&state);
                            last_relocate_raise = Instant::now();
                        }
                    }
                }
                Ok(HostCmd::SetVisibilityMap(map)) => {
                    state.visibility = map;
                    apply_gate_visibility(&mut state);
                    note("overlay-x11: visibility map replaced");
                }
                Ok(HostCmd::SetWake(ctx)) => {
                    state.wake = Some(ctx);
                }
                Ok(HostCmd::SetHubFanDismiss(enabled)) => {
                    if state.hub_fan_dismiss != enabled {
                        state.hub_fan_dismiss = enabled;
                        // When arming, ignore a held press (the hub-open click).
                        // Focused egui wakes fast enough that SetHubFan lands
                        // mid-click; a rising-edge reset would instant-dismiss.
                        state.last_hub_dismiss_btn1 = enabled;
                        note(&format!("overlay-x11: hub_fan_dismiss={enabled}"));
                    }
                }
                Ok(HostCmd::SetHubFan { expanded, snap }) => {
                    apply_hub_fan_cmd(&mut state, expanded, snap);
                }
                Ok(HostCmd::ShowChooser(spec)) => show_chooser(&mut state, spec, None),
                Ok(HostCmd::HideChooser) => hide_chooser(&mut state, None),
                Ok(HostCmd::SetCaptureOmit(omit, ack)) => {
                    state.capture_omit = omit;
                    apply_gate_visibility(&mut state);
                    let _ = ack.send(());
                }
                Ok(HostCmd::Shutdown) => {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }

        // Busy must not wait on egui sync — ROOT starves under fullscreen games.
        sync_press_stops(&mut state);
        let running_now = running_macro.lock().clone();
        state.running_label = running_now.clone();
        if running_now != last_running {
            apply_running_busy(&mut state, running_now.as_deref());
            last_running = running_now;
            stall_raised = false;
            last_x_growth = Instant::now();
        }

        // SAFETY: `state.display` stays open on this thread until the loop exits.
        let before_pending = unsafe { XPending(state.display) };
        let _got_pointer = drain_x_events(&mut state);
        // Poll-track relocate: portal / quiet X after press (X11 or Wayland poll-hit).
        // Over Wayland surfaces (Cursor, GNOME desktop) OR hit windows often get no
        // ButtonPress — detect rising Button1 over a button rect instead.
        let ptr = poll_relocate_pointer(&mut state);
        if state.drag.as_ref().is_some_and(|d| d.poll_track) {
            let mut release = false;
            if let Some((x, y, btn1)) = ptr {
                relocate_drag_to(&mut state, x, y);
                if !btn1 {
                    release = true;
                }
                last_btn1 = btn1;
            }
            flush_relocate_visual(&mut state);
            if release {
                note("overlay-x11: relocate-release (poll)");
                mark_site("overlay-x11:relocate-release-poll");
                commit_drag(&mut state);
            }
        } else {
            if state.relocate_mode && state.drag.is_none() {
                if let Some((x, y, btn1)) = ptr {
                    if btn1 && !last_btn1 {
                        if let Some(id) = button_id_at_root(&state, x, y) {
                            begin_relocate_drag(&mut state, &id, x, y, "poll-hit");
                        }
                    }
                    last_btn1 = btn1;
                }
            } else if state.passthrough_hits && state.chooser.as_ref().is_none_or(|c| !c.mapped) {
                let (b1, b3) = poll_click_through(&mut state, last_btn1, last_btn3);
                last_btn1 = b1;
                last_btn3 = b3;
            } else if let Some((_, _, btn1)) = ptr {
                last_btn1 = btn1;
                last_btn3 = sqyre_hotkeys::right_button_down();
            }
            if state.chooser.as_ref().is_some_and(|c| c.mapped) {
                poll_chooser_outside_dismiss(&mut state, ptr);
            } else if state.hub_fan_dismiss {
                poll_hub_fan_outside_dismiss(&mut state, ptr);
            }
            if state.drag.is_some() {
                flush_relocate_visual(&mut state);
            }
        }
        if before_pending > 0 {
            last_x_growth = Instant::now();
            stall_raised = false;
        }
        // Relocate drag uses the implicit ButtonPress grab (events keep flowing to
        // the hit window until release). Never XQueryPointer here — that round-trip
        // stalls for seconds under a fullscreen XWayland game and blocks the host
        // so ButtonRelease sits unread until a focus cycle (see x11_outline /
        // recording_overlay).
        apply_gate_visibility(&mut state);
        sync_relocate_shield(&mut state);

        // After macros, the game often restacks above OR hits. Re-raise when the
        // X queue goes quiet (no events at all — not only pointer). Never
        // MapRaised while click-through — that drops held buttons on XWayland.
        let any_busy = state.buttons.values().any(|b| b.spec.busy);
        let chooser_open = state.chooser.as_ref().is_some_and(|c| c.mapped);
        let relocating = state.drag.is_some();
        if chooser_open {
            if let Some(ch) = state.chooser.as_ref() {
                // SAFETY: HostState display invariant; chooser window still live.
                unsafe {
                    XMapRaised(state.display, ch.win);
                    if state.arrow_cursor != 0 {
                        XDefineCursor(state.display, ch.win, state.arrow_cursor);
                    }
                    x_flush(state.display);
                }
            }
        } else if state.relocate_mode
            && state.relocate_keep_above
            && !relocating
            && !state.buttons.is_empty()
            && !state.shield.as_ref().is_some_and(|s| s.mapped)
            && last_relocate_raise.elapsed() >= Duration::from_millis(350)
        {
            // Without the Wayland shield, re-raise OR buttons above the Sqyre
            // editor. Skip while the shield is mapped — periodic MapRaised on
            // every button (plus the old per-tick shield raise) flickered the
            // GNOME top bar.
            raise_all_hits(&state);
            last_relocate_raise = Instant::now();
        } else if !state.relocate_mode
            && !relocating
            && !state.click_through
            && !state.buttons.is_empty()
            && !any_busy
            && !stall_raised
            && last_x_growth.elapsed() >= Duration::from_millis(800)
        {
            raise_all_hits(&state);
            stall_raised = true;
        }

        // Overlay clicks enqueue here, but start_macro only runs on a ROOT frame.
        // A single request_repaint is often ignored while a fullscreen game has
        // focus (same reason hotkeys use hotkey_wake). Keep nudging until drain.
        if !state.pending.lock().is_empty() {
            if let Some(ctx) = &state.wake {
                ctx.request_repaint();
            }
        }

        let busy_now = state.buttons.values().any(|b| b.spec.busy);
        if busy_now && last_busy_tick.elapsed() >= Duration::from_millis(BUSY_TICK_MS) {
            last_busy_tick = Instant::now();
            let busy_ids: Vec<String> = state
                .buttons
                .iter()
                .filter(|(_, b)| b.spec.busy)
                .map(|(id, _)| id.clone())
                .collect();
            for id in busy_ids {
                if let Some(btn) = state.buttons.get_mut(&id) {
                    // 50 ms busy tick → advance the traversal fraction so the
                    // sweep rate matches egui's `time * SWEEP_PER_SEC`.
                    btn.busy_phase = (btn.busy_phase + crate::brand_s::SWEEP_PER_SEC * 0.05) % 1.0;
                }
                if let Some(btn) = state.buttons.get(&id) {
                    if !btn.mapped {
                        continue;
                    }
                    // Pixels only — reapplying Shape* every spinner frame made the
                    // face eat clicks (empty ShapeInput regresses on XWayland/Mutter).
                    paint_button(&state, btn, false);
                }
            }
            // SAFETY: HostState display invariant.
            unsafe { x_flush(state.display) };
        }

        // Hub fan travel must not wait on egui — ROOT starves when unfocused.
        let fan_animating = crate::host_common::tick_hub_fan(&mut state);

        wait_x_or_timeout(
            state.xfd,
            if (busy_now || fan_animating) && !state.relocate_mode && state.drag.is_none() {
                if fan_animating {
                    crate::hub_fan::HUB_FAN_TICK_MS
                } else {
                    BUSY_TICK_MS
                }
            } else {
                // Relocate needs a steady poll for Wayland poll-hit press + release.
                POLL_IDLE_MS
            },
        );
    }

    destroy_all(&mut state);
    // SAFETY: HostState display/gc still live; windows already destroyed.
    unsafe {
        x_free_gc(state.display, state.gc);
    }
    if !state.ptr_display.is_null() {
        unregister_secondary_x_display(state.ptr_display.cast());
        // SAFETY: quiet pointer display; no windows created on it.
        unsafe {
            x_close_display(state.ptr_display);
        }
        state.ptr_display = std::ptr::null_mut();
    }
    unregister_secondary_x_display(state.display.cast());
    // SAFETY: unregistered; no further Xlib use of this pointer.
    unsafe {
        x_close_display(state.display);
    }
    note("overlay-x11: host thread stopped");
}
