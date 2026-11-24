//! Hub-fan placement geometry for the X11 overlay host.

use super::*;

pub(super) fn hub_face_geom(state: &HostState) -> (i32, i32, u32, u32) {
    state
        .buttons
        .get(sqyre_persist::QUICK_ACCESS_HUB_ID)
        .map(|hub| {
            (
                hub.spec.face.x,
                hub.spec.face.y,
                hub.spec.face.w,
                hub.spec.face.h,
            )
        })
        .unwrap_or((0, 0, 48, 48))
}

pub(super) fn place_hub_satellite_spec(state: &HostState, spec: &mut NativeButtonSpec) {
    place_hub_satellite_face(&state.hub_fan, hub_face_geom(state), spec);
}

impl HubFanHost for HostState {
    fn hub_fan(&mut self) -> &mut crate::hub_fan::HubFanState {
        &mut self.hub_fan
    }

    fn place_all_satellites(&mut self) {
        place_all_hub_satellites(self);
    }

    fn rearm_satellite_input(&mut self) {
        rearm_hub_satellite_input(self);
    }

    fn apply_fan_visibility(&mut self) {
        // Map while traveling/open; unmap when fully closed (no off-desktop park).
        visibility::apply_gate_visibility(self);
    }

    fn sync_dismiss_arm(&mut self) {
        sync_hub_fan_dismiss_arm(self);
    }

    fn flush_after_fan(&mut self) {
        // SAFETY: HostState display invariant.
        unsafe {
            x_flush(self.display);
        }
    }
}

pub(super) fn apply_hub_fan_cmd(state: &mut HostState, expanded: bool, snap: bool) {
    crate::host_common::apply_hub_fan_cmd(state, expanded, snap);
}

pub(super) fn sync_hub_fan_dismiss_arm(state: &mut HostState) {
    let want = state.hub_fan.open_or_traveling();
    if crate::hub_dismiss::sync_hub_fan_dismiss_fields(
        &mut state.hub_fan_dismiss,
        &mut state.last_hub_dismiss_btn1,
        want,
    ) {
        note(&format!("overlay-x11: hub_fan_dismiss={want} (fan)"));
    }
}

pub(super) fn place_all_hub_satellites(state: &mut HostState) {
    let (hx, hy, hw, hh) = hub_face_geom(state);
    let ids = state.hub_fan.satellite_ids();
    for id in ids {
        let Some(rest) = state.hub_fan.rest_of(&id) else {
            continue;
        };
        let Some(live) = state.buttons.get(&id) else {
            continue;
        };
        let (w, h) = (live.spec.face.w, live.spec.face.h);
        let origin = crate::hub_fan::hub_origin(hx, hy, hw, hh, w, h);
        let (x, y) = crate::hub_fan::satellite_pos(state.hub_fan.t, rest, origin);
        if live.spec.face.x == x && live.spec.face.y == y {
            continue;
        }
        let win = live.win;
        let hit = live.hit;
        // SAFETY: HostState display invariant; face+hit created on this display.
        unsafe {
            x_move_resize(state.display, hit, x, y, w, h);
            x_move_resize(state.display, win, x, y, w, h);
        }
        if let Some(live) = state.buttons.get_mut(&id) {
            live.spec.face.x = x;
            live.spec.face.y = y;
        }
    }
}

pub(super) fn rearm_hub_satellite_input(state: &mut HostState) {
    let enabled = state.hub_fan.fully_open();
    let ids = state.hub_fan.satellite_ids();
    let mut rearm: Vec<(Window, u32, u32, f32)> = Vec::new();
    for id in ids {
        let Some(live) = state.buttons.get_mut(&id) else {
            continue;
        };
        if live.spec.input_enabled == enabled {
            continue;
        }
        live.spec.input_enabled = enabled;
        rearm.push((
            live.hit,
            live.spec.face.w,
            live.spec.face.h,
            live.spec.face.corner_radius,
        ));
    }
    for (hit, w, h, radius) in rearm {
        sync_hit_input(state, hit, w, h, radius, enabled);
    }
}
