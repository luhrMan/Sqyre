//! Wayland layer-shell overlay host thread.

use super::*;
use crate::host_common::{
    apply_hosted_visibility, apply_hub_fan_cmd, button_should_show, prepare_hub_satellite_spec,
    spec_contains_rounded, GateVisibilityCtx, HubButtonView, HubFanHost,
};
use crate::host_press::{
    is_hub_face_token, overlay_press_kind, should_coalesce_hub_click, OverlayPressKind,
};
use crate::hub_fan::{is_hub_chrome_id, is_hub_satellite_id, HUB_FAN_TICK_MS};
use crate::os_popup::{place_cursor_popup, place_tip_near_button, TIP_GAP_PX};
use sqyre_capture::LayerShellConn;
use sqyre_ports::{event_log, note};
use std::time::Instant;

#[expect(
    clippy::too_many_arguments,
    reason = "host thread owns the queues, the worker stop flag, and press-to-stop"
)]
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
    press_stops: Arc<AtomicBool>,
) {
    let mut conn = match LayerShellConn::connect() {
        Ok(c) => c,
        Err(e) => {
            note(&format!("overlay-wl: connect failed: {e}"));
            return;
        }
    };
    let outputs = conn.outputs();
    if outputs.is_empty() {
        note("overlay-wl: no outputs");
        return;
    }
    let seat = match conn.state.seat.clone() {
        Some(s) => s,
        None => {
            note("overlay-wl: no seat");
            return;
        }
    };
    let qh = conn.queue.handle();
    let pointer = seat.get_pointer(&qh, ());
    let _ = conn.roundtrip();

    let mut state = HostState {
        conn,
        pointer: Some(pointer),
        buttons: HashMap::new(),
        surface_to_button: HashMap::new(),
        tip: None,
        chooser: None,
        visibility,
        pending,
        pending_moves,
        running_macro,
        pending_chooser,
        pending_context,
        run_stop,
        press_stops,
        wake: None,
        relocate_mode: false,
        relocate_keep_above: false,
        hub_fan: HubFanState::default(),
        hub_fan_dismiss: false,
        capture_omit: false,
        armed_id: None,
        drag_id: None,
        drag_off: (0, 0),
        last_ptr: (0, 0),
        outputs,
    };

    event_log("SQYRE_OVERLAY", &[("backend", "wayland-layer-shell")]);
    note("overlay-wl: layer-shell host running");

    let mut last_busy = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        while let Ok(cmd) = cmd_rx.try_recv() {
            if matches!(cmd, HostCmd::Shutdown) {
                destroy_all(&mut state);
                return;
            }
            handle_cmd(&mut state, cmd);
        }

        // Pump Wayland.
        let _ = state.conn.conn.flush();
        if let Some(guard) = state.conn.conn.prepare_read() {
            let _ = guard.read();
        }
        let _ = state.conn.queue.dispatch_pending(&mut state.conn.state);

        // Drain grab-style pointer samples from LayerShellState into host actions.
        // Overlay uses enter/button via surface focus tracked in HostState by polling
        // portal cursor + hit tests when grab accum moves, and also processes
        // LayerShellState.grab for global dismiss.
        drain_pointer(&mut state);

        if state.hub_fan.animating() {
            let _ = crate::host_common::tick_hub_fan(&mut state);
        }

        // Busy spinner refresh.
        if last_busy.elapsed() >= Duration::from_millis(50) {
            last_busy = Instant::now();
            refresh_busy(&mut state);
        }

        apply_gate_visibility(&mut state);
        std::thread::sleep(Duration::from_millis(HUB_FAN_TICK_MS.min(16)));
    }
    destroy_all(&mut state);
}

fn handle_cmd(state: &mut HostState, cmd: HostCmd) {
    match cmd {
        HostCmd::SetButtons(specs) => apply_specs(state, specs),
        HostCmd::SetRelocateMode(on) => {
            state.relocate_mode = on;
            if !on {
                state.drag_id = None;
            }
        }
        HostCmd::SetRelocateKeepAbove(on) => {
            state.relocate_keep_above = on;
        }
        HostCmd::SetVisibilityMap(map) => {
            state.visibility = map;
        }
        HostCmd::SetWake(ctx) => {
            state.wake = Some(ctx);
        }
        HostCmd::SetHubFanDismiss(on) => {
            state.hub_fan_dismiss = on;
        }
        HostCmd::SetHubFan { expanded, snap } => {
            apply_hub_fan_cmd(state, expanded, snap);
        }
        HostCmd::ShowChooser(spec) => show_chooser(state, spec, None),
        HostCmd::HideChooser => hide_chooser(state, Some(HotkeyChooserResult::Dismissed)),
        HostCmd::SetCaptureOmit(omit, ack) => {
            state.capture_omit = omit;
            apply_gate_visibility(state);
            let _ = ack.send(());
        }
        HostCmd::Shutdown => {}
    }
}

fn hub_face_geom(state: &HostState) -> (i32, i32, u32, u32) {
    state
        .buttons
        .get(sqyre_persist::QUICK_ACCESS_HUB_ID)
        .or_else(|| {
            state
                .buttons
                .values()
                .find(|b| is_hub_face_token(&b.spec.macro_name))
        })
        .map(|b| (b.spec.face.x, b.spec.face.y, b.spec.face.w, b.spec.face.h))
        .unwrap_or((0, 0, 48, 48))
}

/// Null-buffer commit — Wayland unmap (XUnmapWindow / SW_HIDE equivalent).
fn unmap_surface(surface: &wl_surface::WlSurface) {
    surface.attach(None, 0, 0);
    surface.commit();
}

fn sync_button_input(state: &mut HostState, id: &str) {
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    if btn.spec.input_enabled {
        let _ = state.conn.set_input_full(&btn.surface);
    } else {
        let _ = state.conn.set_input_empty(&btn.surface);
    }
    btn.surface.commit();
    state.conn.flush();
}

impl HubFanHost for HostState {
    fn hub_fan(&mut self) -> &mut HubFanState {
        &mut self.hub_fan
    }

    fn hub_face_geom(&self) -> (i32, i32, u32, u32) {
        hub_face_geom(self)
    }

    fn hosted_button_ids(&self) -> Vec<String> {
        self.buttons.keys().cloned().collect()
    }

    fn button_view(&self, id: &str) -> Option<HubButtonView> {
        self.buttons.get(id).map(|b| HubButtonView {
            face_x: b.spec.face.x,
            face_y: b.spec.face.y,
            face_w: b.spec.face.w,
            face_h: b.spec.face.h,
            mapped: b.mapped,
            visibility_gated: b.spec.visibility_gated,
            input_enabled: b.spec.input_enabled,
        })
    }

    fn set_satellite_xy(&mut self, id: &str, x: i32, y: i32) {
        if let Some(btn) = self.buttons.get_mut(id) {
            btn.spec.face.x = x;
            btn.spec.face.y = y;
        }
    }

    fn set_satellite_input_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(btn) = self.buttons.get_mut(id) {
            btn.spec.input_enabled = enabled;
        }
    }

    fn move_mapped_button(&mut self, id: &str) {
        reposition_button(self, id);
    }

    fn sync_mapped_input(&mut self, id: &str) {
        sync_button_input(self, id);
    }

    fn set_button_mapped(&mut self, id: &str, show: bool) {
        if show {
            if let Some(b) = self.buttons.get_mut(id) {
                b.mapped = true;
            }
            reposition_button(self, id);
            return;
        }
        if self.tip.as_ref().is_some_and(|t| t.for_id == id) {
            hide_tip(self);
        }
        if self.drag_id.as_deref() == Some(id) {
            self.drag_id = None;
        }
        if self.armed_id.as_deref() == Some(id) {
            self.armed_id = None;
        }
        if let Some(b) = self.buttons.get_mut(id) {
            b.mapped = false;
            b.hovered = false;
            let _ = self.conn.set_input_empty(&b.surface);
            unmap_surface(&b.surface);
        }
        self.conn.flush();
    }

    fn visibility_context(&self) -> GateVisibilityCtx {
        GateVisibilityCtx {
            capture_omit: self.capture_omit,
            relocate: self.relocate_mode,
            fan_open_or_traveling: self.hub_fan.open_or_traveling(),
            found: self.visibility.lock().clone(),
        }
    }

    fn sync_dismiss_arm(&mut self) {}

    fn flush_after_fan(&mut self) {
        self.conn.flush();
    }
}

fn apply_specs(state: &mut HostState, specs: Vec<OverlayButtonSpec>) {
    let mut keep = std::collections::HashSet::new();
    let mut pending: Vec<OverlayButtonSpec> = Vec::with_capacity(specs.len());
    for spec in specs {
        keep.insert(spec.id.clone());
        pending.push(spec);
    }
    // Hub face first so satellite place sees live hub geom.
    pending.sort_by_key(|s| i32::from(is_hub_satellite_id(&s.id)));
    for mut spec in pending {
        let hub_geom = hub_face_geom(state);
        prepare_hub_satellite_spec(&mut state.hub_fan, hub_geom, &mut spec);
        if let Some(btn) = state.buttons.get_mut(&spec.id) {
            let pos_changed = btn.spec.face.x != spec.face.x || btn.spec.face.y != spec.face.y;
            let size_changed = btn.spec.face.w != spec.face.w || btn.spec.face.h != spec.face.h;
            let mapped = btn.mapped;
            btn.spec = spec;
            let id = btn.id.clone();
            if mapped && (pos_changed || size_changed) {
                reposition_button(state, &id);
            } else if mapped {
                repaint_button(state, &id);
            }
        } else if let Err(e) = create_button(state, spec) {
            note(&format!("overlay-wl: create_button: {e}"));
        }
    }
    let remove: Vec<String> = state
        .buttons
        .keys()
        .filter(|id| !keep.contains(*id))
        .cloned()
        .collect();
    for id in remove {
        destroy_button(state, &id);
    }
    state.hub_fan.forget_missing(&keep);
    apply_gate_visibility(state);
}

fn create_button(state: &mut HostState, spec: OverlayButtonSpec) -> Result<(), String> {
    let show = button_should_show(
        &spec.id,
        spec.visibility_gated,
        &GateVisibilityCtx {
            capture_omit: state.capture_omit,
            relocate: state.relocate_mode,
            fan_open_or_traveling: state.hub_fan.open_or_traveling(),
            found: state.visibility.lock().clone(),
        },
    );
    let geom = pick_output(state, spec.face.x, spec.face.y)?;
    let (surface, layer) = state
        .conn
        .create_overlay_surface(
            &geom.output,
            &format!("sqyre-btn-{}", spec.id),
            zwlr_layer_surface_v1::KeyboardInteractivity::None,
        )
        .map_err(|e| e.to_string())?;
    if show && spec.input_enabled {
        let _ = state.conn.set_input_full(&surface);
    } else {
        let _ = state.conn.set_input_empty(&surface);
    }
    let rect = sqyre_ports::DesktopRect {
        x: spec.face.x,
        y: spec.face.y,
        w: spec.face.w.max(1) as i32,
        h: spec.face.h.max(1) as i32,
    };
    let (lw, lh) = LayerShellConn::position_layer(&layer, rect, &geom);
    state.conn.state.configure_serial = None;
    surface.commit();
    if !state.conn.wait_configure(8) {
        layer.destroy();
        surface.destroy();
        return Err("configure timeout".into());
    }
    let serial = state
        .conn
        .state
        .configure_serial
        .take()
        .ok_or("no serial")?;
    layer.ack_configure(serial);
    // Closed hub satellites stay created-but-unmapped (warm for the next open).
    // Attaching pixels then hiding via input-only left ghosts on KWin.
    if show {
        let (_, _, pixels) = paint_button_pixels(&spec, false, false);
        attach_face(&mut state.conn, &surface, &geom, lw, lh, &pixels)?;
    }
    let sid = surface.id().protocol_id();
    state.surface_to_button.insert(sid, spec.id.clone());
    state.buttons.insert(
        spec.id.clone(),
        LiveButton {
            id: spec.id.clone(),
            spec,
            surface,
            layer,
            mapped: show,
            hovered: false,
        },
    );
    state.conn.flush();
    Ok(())
}

fn reposition_button(state: &mut HostState, id: &str) {
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    if !btn.mapped {
        return;
    }
    let (x, y, w, h) = (
        btn.spec.face.x,
        btn.spec.face.y,
        btn.spec.face.w.max(1) as i32,
        btn.spec.face.h.max(1) as i32,
    );
    let hovered = btn.hovered;
    let spec = btn.spec.clone();
    let Ok(geom) = pick_output(state, x, y) else {
        return;
    };
    let Some(btn) = state.buttons.get_mut(id) else {
        return;
    };
    if spec.input_enabled {
        let _ = state.conn.set_input_full(&btn.surface);
    } else {
        let _ = state.conn.set_input_empty(&btn.surface);
    }
    let rect = sqyre_ports::DesktopRect { x, y, w, h };
    let (lw, lh) = LayerShellConn::position_layer(&btn.layer, rect, &geom);
    state.conn.state.configure_serial = None;
    btn.surface.commit();
    if !state.conn.wait_configure(8) {
        return;
    }
    if let Some(serial) = state.conn.state.configure_serial.take() {
        btn.layer.ack_configure(serial);
    }
    let busy = state
        .running_macro
        .lock()
        .as_ref()
        .is_some_and(|n| n == &spec.macro_name);
    let (_, _, pixels) = paint_button_pixels(&spec, hovered, busy);
    let _ = attach_face(&mut state.conn, &btn.surface, &geom, lw, lh, &pixels);
    state.conn.flush();
}

fn repaint_button(state: &mut HostState, id: &str) {
    reposition_button(state, id);
}

fn destroy_button(state: &mut HostState, id: &str) {
    if let Some(btn) = state.buttons.remove(id) {
        state.surface_to_button.retain(|_, bid| bid != id);
        btn.layer.destroy();
        btn.surface.destroy();
    }
    if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
        hide_tip(state);
    }
}

fn destroy_all(state: &mut HostState) {
    hide_chooser(state, None);
    hide_tip(state);
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    for id in ids {
        destroy_button(state, &id);
    }
    if let Some(p) = state.pointer.take() {
        p.release();
    }
    state.conn.flush();
}

fn pick_output(state: &HostState, x: i32, y: i32) -> Result<OutputGeom, String> {
    let cosmic = sqyre_ports::cosmic_desktop_session();
    let pt = sqyre_ports::DesktopRect { x, y, w: 1, h: 1 };
    for o in &state.outputs {
        let dest = output_desktop_rect(o.x, o.y, o.width, o.height, o.scale, cosmic);
        if sqyre_capture::rects_overlap(dest, pt) {
            return Ok(o.clone());
        }
    }
    state
        .outputs
        .first()
        .cloned()
        .ok_or_else(|| "no output".into())
}

fn apply_gate_visibility(state: &mut HostState) {
    apply_hosted_visibility(state);
}

fn refresh_busy(state: &mut HostState) {
    let running = state.running_macro.lock().clone();
    let Some(name) = running else {
        return;
    };
    let ids: Vec<String> = state
        .buttons
        .iter()
        .filter(|(_, b)| b.spec.macro_name == name)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        repaint_button(state, &id);
    }
}

fn drain_pointer(state: &mut HostState) {
    // Prefer portal/desktop cursor; fall back to LayerShellState.grab samples.
    let ptr = sqyre_capture::desktop_cursor_position().or_else(|| {
        let g = state.conn.state.grab;
        if g.moved {
            Some((g.x, g.y))
        } else {
            None
        }
    });
    let Some((px, py)) = ptr else {
        // Still check escape / clicks from grab accum when exclusive grab isn't ours —
        // for overlay we synthesize clicks via seat only when over our surfaces.
        process_grab_clicks(state);
        return;
    };
    state.last_ptr = (px, py);

    if let Some(id) = state.drag_id.clone() {
        if state.relocate_mode {
            let (ox, oy) = state.drag_off;
            if let Some(btn) = state.buttons.get_mut(&id) {
                btn.spec.face.x = px - ox;
                btn.spec.face.y = py - oy;
            }
            reposition_button(state, &id);
        }
    }

    // Hover tip
    let over = button_at(state, px, py);
    update_hover(state, over.as_deref());

    process_grab_clicks(state);

    // Hub dismiss: outside left-click while dismiss armed.
    let g = state.conn.state.grab;
    if state.hub_fan_dismiss && g.left_clicks > 0 {
        let over_chrome = over.as_ref().is_some_and(|id| is_hub_chrome_id(id));
        if !over_chrome {
            let mut q = state.pending.lock();
            let dismiss = sqyre_persist::QUICK_ACCESS_HUB_DISMISS.to_string();
            if !q.iter().any(|n| n == &dismiss) {
                q.push(dismiss);
            }
            wake(state);
        }
    }
    // Clear accum
    state.conn.state.grab = sqyre_capture::GrabAccum {
        x: px,
        y: py,
        ..Default::default()
    };
}

fn process_grab_clicks(state: &mut HostState) {
    let g = state.conn.state.grab;
    let (px, py) = state.last_ptr;
    if g.left_clicks > 0 {
        if state.relocate_mode {
            if let Some(id) = button_at(state, px, py) {
                if let Some(btn) = state.buttons.get(&id) {
                    state.drag_off = (px - btn.spec.face.x, py - btn.spec.face.y);
                    state.drag_id = Some(id);
                }
            }
        } else if let Some(id) = button_at(state, px, py) {
            on_press(state, &id);
        } else if state.chooser.is_some() {
            // Outside chooser → dismiss
            hide_chooser(state, Some(HotkeyChooserResult::Dismissed));
        }
    }
    if g.left_releases > 0 {
        if let Some(id) = state.drag_id.take() {
            if let Some(btn) = state.buttons.get(&id) {
                state.pending_moves.lock().push(OverlayButtonMove {
                    id: id.clone(),
                    x: btn.spec.face.x,
                    y: btn.spec.face.y,
                });
                wake(state);
            }
        }
        if let Some(id) = state.armed_id.take() {
            fire_button(state, &id);
        }
        // Chooser row pick
        if let Some(row) = chooser_row_at(state, px, py) {
            if let Some(ch) = &state.chooser {
                if let Some(name) = ch.spec.names.get(row).cloned() {
                    hide_chooser(state, Some(HotkeyChooserResult::Picked(name)));
                }
            }
        }
    }
    // Right click → context (BTN_RIGHT = 0x111) — not in GrabAccum; skip for now /
    // use pending from third button if we extend GrabAccum later.
}

fn button_at(state: &HostState, x: i32, y: i32) -> Option<String> {
    for (id, btn) in &state.buttons {
        if !btn.mapped || !btn.spec.input_enabled {
            continue;
        }
        if spec_contains_rounded(&btn.spec, x, y) {
            return Some(id.clone());
        }
    }
    None
}

fn update_hover(state: &mut HostState, over: Option<&str>) {
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    for id in ids {
        let want = over == Some(id.as_str());
        let Some(btn) = state.buttons.get_mut(&id) else {
            continue;
        };
        if btn.hovered == want {
            continue;
        }
        btn.hovered = want;
        repaint_button(state, &id);
        if want {
            show_tip_for(state, &id);
        } else if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
            hide_tip(state);
        }
    }
}

fn on_press(state: &mut HostState, id: &str) {
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    let name = btn.spec.macro_name.clone();
    let busy = state
        .running_macro
        .lock()
        .as_ref()
        .is_some_and(|n| n == &name);
    let running = state.running_macro.lock().is_some();
    let press_stops = state.press_stops.load(Ordering::Relaxed);
    match overlay_press_kind(press_stops, running, busy, state.armed_id.is_some()) {
        OverlayPressKind::Stop => {
            state.run_stop.lock().request_stop();
            wake(state);
        }
        OverlayPressKind::IgnoreBusy | OverlayPressKind::IgnoreArmed => {}
        OverlayPressKind::Fire => {
            state.armed_id = Some(id.to_string());
        }
    }
}

fn fire_button(state: &mut HostState, id: &str) {
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    let name = btn.spec.macro_name.clone();
    if is_hub_face_token(&name) {
        // Toggle handled by app via enqueue of hub id.
    }
    {
        let mut q = state.pending.lock();
        if should_coalesce_hub_click(&q, &name) {
            return;
        }
        q.push(name);
    }
    wake(state);
}

fn wake(state: &HostState) {
    if let Some(ctx) = &state.wake {
        ctx.request_repaint();
    }
}

fn show_tip_for(state: &mut HostState, id: &str) {
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    let text = btn.spec.tip.clone();
    if text.trim().is_empty() {
        return;
    }
    let ui_scale = btn.spec.ui_scale;
    let (tw, th, rgba) = raster::rasterize_tip(&text, ui_scale);
    let argb = rgba_to_wayland_argb(&rgba);
    let (bx, by, bw, bh) = (
        btn.spec.face.x,
        btn.spec.face.y,
        btn.spec.face.w as i32,
        btn.spec.face.h as i32,
    );
    let mon = crate::os_popup::monitor_for_point(bx, by);
    let (tx, ty) = place_tip_near_button(bx, by, bw, bh, tw as i32, th as i32, mon);
    hide_tip(state);
    let Ok(geom) = pick_output(state, tx, ty) else {
        return;
    };
    let Ok((surface, layer)) = state.conn.create_overlay_surface(
        &geom.output,
        "sqyre-tip",
        zwlr_layer_surface_v1::KeyboardInteractivity::None,
    ) else {
        return;
    };
    let _ = state.conn.set_input_empty(&surface);
    let rect = sqyre_ports::DesktopRect {
        x: tx,
        y: ty,
        w: tw as i32,
        h: th as i32,
    };
    let (lw, lh) = LayerShellConn::position_layer(&layer, rect, &geom);
    state.conn.state.configure_serial = None;
    surface.commit();
    if !state.conn.wait_configure(8) {
        layer.destroy();
        surface.destroy();
        return;
    }
    if let Some(serial) = state.conn.state.configure_serial.take() {
        layer.ack_configure(serial);
    }
    let _ = attach_face(&mut state.conn, &surface, &geom, lw, lh, &argb);
    let _ = TIP_GAP_PX;
    let _ = (text, ui_scale);
    state.tip = Some(TipSurface {
        for_id: id.to_string(),
        surface,
        layer,
    });
    state.conn.flush();
}

fn hide_tip(state: &mut HostState) {
    if let Some(tip) = state.tip.take() {
        tip.layer.destroy();
        tip.surface.destroy();
        state.conn.flush();
    }
}

fn show_chooser(state: &mut HostState, spec: HotkeyChooserSpec, context_button_id: Option<String>) {
    hide_chooser(state, None);
    let (cw, ch, rgba, _rows) = raster::rasterize_chooser(&spec.title, &spec.names, None, None);
    let argb = rgba_to_wayland_argb(&rgba);
    let (x, y) = place_cursor_popup(spec.x, spec.y, cw as i32, ch as i32);
    let Ok(geom) = pick_output(state, x, y) else {
        return;
    };
    let Ok((surface, layer)) = state.conn.create_overlay_surface(
        &geom.output,
        "sqyre-chooser",
        zwlr_layer_surface_v1::KeyboardInteractivity::None,
    ) else {
        return;
    };
    let _ = state.conn.set_input_full(&surface);
    let rect = sqyre_ports::DesktopRect {
        x,
        y,
        w: cw as i32,
        h: ch as i32,
    };
    let (lw, lh) = LayerShellConn::position_layer(&layer, rect, &geom);
    state.conn.state.configure_serial = None;
    surface.commit();
    if !state.conn.wait_configure(8) {
        layer.destroy();
        surface.destroy();
        return;
    }
    if let Some(serial) = state.conn.state.configure_serial.take() {
        layer.ack_configure(serial);
    }
    let _ = attach_face(&mut state.conn, &surface, &geom, lw, lh, &argb);
    let _ = context_button_id;
    state.chooser = Some(ChooserSurface {
        spec,
        surface,
        layer,
        row_h: 28,
        title_h: 22,
        w: cw as i32,
        h: ch as i32,
    });
    // Reposition stored at layer margins — stash desktop origin on layer via margins already.
    // Store x/y by reusing tip gap: update spec coords
    if let Some(ch) = state.chooser.as_mut() {
        ch.spec.x = x;
        ch.spec.y = y;
    }
    state.conn.flush();
}

fn hide_chooser(state: &mut HostState, result: Option<HotkeyChooserResult>) {
    if let Some(ch) = state.chooser.take() {
        ch.layer.destroy();
        ch.surface.destroy();
        state.conn.flush();
    }
    if let Some(r) = result {
        *state.pending_chooser.lock() = Some(r);
        wake(state);
    }
}

fn chooser_row_at(state: &HostState, x: i32, y: i32) -> Option<usize> {
    let ch = state.chooser.as_ref()?;
    if x < ch.spec.x || y < ch.spec.y || x >= ch.spec.x + ch.w || y >= ch.spec.y + ch.h {
        return None;
    }
    let local_y = y - ch.spec.y;
    if local_y < ch.title_h {
        return None;
    }
    let row = ((local_y - ch.title_h) / ch.row_h) as usize;
    if row < ch.spec.names.len() {
        Some(row)
    } else {
        None
    }
}
