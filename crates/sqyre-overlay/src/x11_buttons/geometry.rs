//! Hub-fan placement hooks for the X11 overlay host.

use super::*;
use crate::host_common::{GateVisibilityCtx, HubButtonView, HubFanHost};

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

impl HubFanHost for HostState {
    fn hub_fan(&mut self) -> &mut crate::hub_fan::HubFanState {
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
        if let Some(live) = self.buttons.get_mut(id) {
            live.spec.face.x = x;
            live.spec.face.y = y;
        }
    }

    fn set_satellite_input_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(live) = self.buttons.get_mut(id) {
            live.spec.input_enabled = enabled;
        }
    }

    fn move_mapped_button(&mut self, id: &str) {
        let Some(live) = self.buttons.get(id) else {
            return;
        };
        let (x, y, w, h, win, hit) = (
            live.spec.face.x,
            live.spec.face.y,
            live.spec.face.w,
            live.spec.face.h,
            live.win,
            live.hit,
        );
        // SAFETY: HostState display invariant; face+hit created on this display.
        unsafe {
            x_move_resize(self.display, hit, x, y, w, h);
            x_move_resize(self.display, win, x, y, w, h);
        }
    }

    fn sync_mapped_input(&mut self, id: &str) {
        let Some(live) = self.buttons.get(id) else {
            return;
        };
        let (hit, w, h, radius, enabled) = (
            live.hit,
            live.spec.face.w,
            live.spec.face.h,
            live.spec.face.corner_radius,
            live.spec.input_enabled,
        );
        sync_hit_input(self, hit, w, h, radius, enabled);
    }

    fn set_button_mapped(&mut self, id: &str, show: bool) {
        let Some(btn) = self.buttons.get(id) else {
            return;
        };
        let (hit, face) = (btn.hit, btn.win);
        if show {
            // SAFETY: HostState display invariant; face+hit created on this display.
            unsafe {
                if self.relocate_mode && self.relocate_keep_above {
                    XMapRaised(self.display, face);
                    XMapRaised(self.display, hit);
                } else {
                    XMapWindow(self.display, face);
                    XMapWindow(self.display, hit);
                }
            }
        } else {
            if self.drag.as_ref().is_some_and(|d| d.id == id) {
                cancel_drag(self);
            }
            if self.tip.as_ref().is_some_and(|t| t.for_id == id) {
                hide_tip(self);
            }
            // SAFETY: HostState display invariant; face+hit created on this display.
            unsafe {
                x_unmap(self.display, hit);
                x_unmap(self.display, face);
            }
        }
        if let Some(btn) = self.buttons.get_mut(id) {
            btn.mapped = show;
            if !show {
                btn.armed = false;
                btn.hovered = false;
            }
        }
    }

    fn visibility_context(&self) -> GateVisibilityCtx {
        GateVisibilityCtx {
            capture_omit: self.capture_omit,
            relocate: self.relocate_mode,
            fan_open_or_traveling: self.hub_fan.open_or_traveling(),
            found: self.visibility.lock().clone(),
        }
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
