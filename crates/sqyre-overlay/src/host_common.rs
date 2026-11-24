//! OS-neutral overlay host DTOs and helpers shared by X11, Windows, and Wayland hosts.

use crate::hub_fan::{hub_origin, is_hub_satellite_id, satellite_pos, HubFanState};
use crate::ButtonFace;
use std::collections::HashMap;

/// Desired on-screen button (physical pixels, host desktop coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayButtonSpec {
    pub id: String,
    pub macro_name: String,
    pub face: ButtonFace,
    /// Hover tip text (label / display name). Empty = no tip.
    pub tip: String,
    /// egui `pixels_per_point` — tip font/padding track Settings → UI scale.
    pub ui_scale: f32,
    pub busy: bool,
    /// When true, the host maps this button only while visibility says found.
    pub visibility_gated: bool,
    /// When false, hit-tests pass through and presses are ignored (hub fan travel).
    pub input_enabled: bool,
}

/// Desktop position committed after a relocate-mode drag (root / virtual-desktop coords).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayButtonMove {
    pub id: String,
    pub x: i32,
    pub y: i32,
}

/// What a spec update changed, for the minimal repaint/reshape path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecDelta {
    /// Desktop position changed (`x`/`y`).
    pub pos: bool,
    /// Face pixel size changed (`w`/`h`).
    pub size: bool,
    /// Corner radius changed (input shape needs rebuild even without geometry).
    pub radius: bool,
    /// Any paint-affecting field changed (radius counts — it shapes ink).
    pub style: bool,
    /// Hit-target arming changed (`input_enabled`).
    pub input: bool,
}

impl SpecDelta {
    /// Position and/or size changed — needs a MoveResize.
    pub fn geom(self) -> bool {
        self.pos || self.size
    }
}

/// Classify how `new` differs from `old` for host apply paths.
pub fn spec_delta(old: &OverlayButtonSpec, new: &OverlayButtonSpec) -> SpecDelta {
    let radius = sqyre_domain::f32_changed(old.face.corner_radius, new.face.corner_radius);
    SpecDelta {
        pos: old.face.x != new.face.x || old.face.y != new.face.y,
        size: old.face.w != new.face.w || old.face.h != new.face.h,
        radius,
        style: old.face.bg != new.face.bg
            || old.face.border != new.face.border
            || sqyre_domain::f32_changed(old.face.border_width, new.face.border_width)
            || radius
            || old.face.icon_glyph != new.face.icon_glyph
            || old.face.icon != new.face.icon
            || old.face.icon_hover != new.face.icon_hover
            || old.face.brand != new.face.brand
            || old.tip != new.tip
            || old.busy != new.busy,
        input: old.input_enabled != new.input_enabled,
    }
}

/// True when `(x, y)` is inside the spec's rect (saturating right/bottom edge).
/// A zero-dimension spec hits a 1×1 box on purpose: window creation clamps to
/// `max(1)` too, so the test covers the window that actually exists.
pub fn spec_contains(spec: &OverlayButtonSpec, x: i32, y: i32) -> bool {
    let w = spec.face.w.max(1) as i32;
    let h = spec.face.h.max(1) as i32;
    x >= spec.face.x
        && y >= spec.face.y
        && x < spec.face.x.saturating_add(w)
        && y < spec.face.y.saturating_add(h)
}

/// Local-space hit test against a rounded rectangle of size `w`×`h`.
///
/// `(lx, ly)` are coordinates relative to the face top-left. Corners outside
/// the radius miss. Zero-size faces are never hit.
pub fn point_in_rounded_rect(lx: i32, ly: i32, w: u32, h: u32, radius: f32) -> bool {
    let w = w.max(1) as i32;
    let h = h.max(1) as i32;
    if lx < 0 || ly < 0 || lx >= w || ly >= h {
        return false;
    }
    let r = (radius.round().max(0.0) as i32).min(w / 2).min(h / 2);
    if r <= 0 {
        return true;
    }
    let (cx, cy) = if lx < r && ly < r {
        (r, r)
    } else if lx >= w - r && ly < r {
        (w - r, r)
    } else if lx < r && ly >= h - r {
        (r, h - r)
    } else if lx >= w - r && ly >= h - r {
        (w - r, h - r)
    } else {
        return true;
    };
    let dx = lx - cx;
    let dy = ly - cy;
    dx * dx + dy * dy <= r * r
}

/// Point inside the button's rounded rect (corners outside the radius miss).
pub fn spec_contains_rounded(spec: &OverlayButtonSpec, x: i32, y: i32) -> bool {
    if !spec_contains(spec, x, y) {
        return false;
    }
    point_in_rounded_rect(
        x - spec.face.x,
        y - spec.face.y,
        spec.face.w,
        spec.face.h,
        spec.face.corner_radius,
    )
}

/// Mapped tip fields used to decide whether `apply_specs` must re-show/hide.
pub struct TipRefreshInput<'a> {
    pub for_id: &'a str,
    pub text: &'a str,
    pub ui_scale: f32,
    pub mapped: bool,
}

/// Live tip text / scale for the tip's button after a specs update.
pub struct TipButtonView<'a> {
    pub text: &'a str,
    pub ui_scale: f32,
}

/// Returns `Some((id, show))` when a currently mapped tip must be refreshed.
///
/// `btn` is `None` when the tip's button was removed (caller handles hide via drop).
pub fn tip_refresh_decision(
    tip: TipRefreshInput<'_>,
    btn: Option<TipButtonView<'_>>,
) -> Option<(String, bool)> {
    if !tip.mapped {
        return None;
    }
    let btn = btn?;
    let text = btn.text.trim();
    if text.is_empty() {
        return Some((tip.for_id.to_string(), false));
    }
    if tip.text != text || (tip.ui_scale - btn.ui_scale).abs() > f32::EPSILON {
        return Some((tip.for_id.to_string(), true));
    }
    None
}

/// Place a hub satellite face at the current fan `t` (rested slot → hub origin).
///
/// `spec.face` must hold the **rested** (fully open) top-left when called.
pub fn place_hub_satellite_face(
    fan: &HubFanState,
    hub_geom: (i32, i32, u32, u32),
    spec: &mut OverlayButtonSpec,
) {
    let rest = (spec.face.x, spec.face.y);
    let (hx, hy, hw, hh) = hub_geom;
    let origin = hub_origin(hx, hy, hw, hh, spec.face.w, spec.face.h);
    let (x, y) = satellite_pos(fan.t, rest, origin);
    spec.face.x = x;
    spec.face.y = y;
    spec.input_enabled = fan.fully_open();
}

/// App sends rested satellite slots; remember them and place at the host fan `t`.
pub fn prepare_hub_satellite_spec(
    fan: &mut HubFanState,
    hub_geom: (i32, i32, u32, u32),
    spec: &mut OverlayButtonSpec,
) {
    if !is_hub_satellite_id(&spec.id) {
        return;
    }
    fan.remember_rest(&spec.id, spec.face.x, spec.face.y);
    place_hub_satellite_face(fan, hub_geom, spec);
}

/// Live button fields the shared fan / gate loops need (no OS handles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HubButtonView {
    pub face_x: i32,
    pub face_y: i32,
    pub face_w: u32,
    pub face_h: u32,
    pub mapped: bool,
    pub visibility_gated: bool,
    pub input_enabled: bool,
}

/// Capture-omit / relocate / fan / found-map inputs for [`button_should_show`].
#[derive(Debug, Clone)]
pub struct GateVisibilityCtx {
    pub capture_omit: bool,
    pub relocate: bool,
    pub fan_open_or_traveling: bool,
    pub found: HashMap<String, bool>,
}

/// Whether a hosted button should be mapped right now.
pub fn button_should_show(id: &str, visibility_gated: bool, ctx: &GateVisibilityCtx) -> bool {
    let fan_show = !is_hub_satellite_id(id) || ctx.fan_open_or_traveling;
    !ctx.capture_omit
        && fan_show
        && (ctx.relocate || !visibility_gated || ctx.found.get(id).copied().unwrap_or(false))
}

/// OS-local hooks for shared hub-fan / visibility orchestration.
///
/// Place, rearm, and gate-map loops live in this module; hosts only move,
/// map/unmap, and sync input on real surfaces/windows.
pub trait HubFanHost {
    fn hub_fan(&mut self) -> &mut HubFanState;
    fn hub_face_geom(&self) -> (i32, i32, u32, u32);
    fn hosted_button_ids(&self) -> Vec<String>;
    fn button_view(&self, id: &str) -> Option<HubButtonView>;
    /// Write satellite face top-left into the live spec (no OS move).
    fn set_satellite_xy(&mut self, id: &str, x: i32, y: i32);
    fn set_satellite_input_enabled(&mut self, id: &str, enabled: bool);
    /// OS move/repaint for a button that is already mapped.
    fn move_mapped_button(&mut self, id: &str);
    /// Apply `input_enabled` to the OS hit target for a mapped button.
    fn sync_mapped_input(&mut self, id: &str);
    /// Map or unmap. Host updates its `mapped` flag and performs OS show/hide.
    fn set_button_mapped(&mut self, id: &str, show: bool);
    fn visibility_context(&self) -> GateVisibilityCtx;
    fn sync_dismiss_arm(&mut self);
    fn flush_after_fan(&mut self);
}

/// Place every remembered satellite at the current fan `t` (mapped ones move on OS).
pub fn place_all_satellites<H: HubFanHost>(host: &mut H) {
    let hub = host.hub_face_geom();
    let (fan_t, ids) = {
        let fan = host.hub_fan();
        (fan.t, fan.satellite_ids())
    };
    let mut moves: Vec<(String, i32, i32)> = Vec::new();
    for id in ids {
        let Some(rest) = host.hub_fan().rest_of(&id) else {
            continue;
        };
        let Some(view) = host.button_view(&id) else {
            continue;
        };
        let origin = hub_origin(hub.0, hub.1, hub.2, hub.3, view.face_w, view.face_h);
        let (x, y) = satellite_pos(fan_t, rest, origin);
        if view.face_x == x && view.face_y == y {
            continue;
        }
        moves.push((id, x, y));
    }
    for (id, x, y) in moves {
        let mapped = host.button_view(&id).is_some_and(|v| v.mapped);
        host.set_satellite_xy(&id, x, y);
        if mapped {
            host.move_mapped_button(&id);
        }
    }
}

/// Arm/disarm satellite hit input from fan fully-open state.
pub fn rearm_satellite_input<H: HubFanHost>(host: &mut H) {
    let enabled = host.hub_fan().fully_open();
    let ids = host.hub_fan().satellite_ids();
    let mut sync: Vec<(String, bool)> = Vec::new();
    for id in ids {
        let Some(view) = host.button_view(&id) else {
            continue;
        };
        if view.input_enabled == enabled {
            continue;
        }
        sync.push((id, view.mapped));
    }
    for (id, mapped) in sync {
        host.set_satellite_input_enabled(&id, enabled);
        if mapped {
            host.sync_mapped_input(&id);
        }
    }
}

/// Map/unmap all hosted buttons from capture-omit, fan, relocate, and found-map.
pub fn apply_hosted_visibility<H: HubFanHost>(host: &mut H) {
    let ctx = host.visibility_context();
    let ids = host.hosted_button_ids();
    for id in ids {
        let Some(view) = host.button_view(&id) else {
            continue;
        };
        let show = button_should_show(&id, view.visibility_gated, &ctx);
        if show == view.mapped {
            continue;
        }
        host.set_button_mapped(&id, show);
    }
}

/// Fan open/close visibility (same gate rules as image-search map/unmap).
pub fn apply_fan_visibility<H: HubFanHost>(host: &mut H) {
    apply_hosted_visibility(host);
}

/// Apply an expand/collapse command, placing immediately when snapped or settled.
pub fn apply_hub_fan_cmd<H: HubFanHost>(host: &mut H, expanded: bool, snap: bool) {
    let was_open = host.hub_fan().fully_open();
    host.hub_fan().set_expanded(expanded, snap);
    if snap || !host.hub_fan().animating() {
        place_all_satellites(host);
    }
    if was_open != host.hub_fan().fully_open() {
        rearm_satellite_input(host);
    }
    apply_fan_visibility(host);
    host.sync_dismiss_arm();
    host.flush_after_fan();
}

/// Advance hub fan travel; returns whether travel is still in progress.
pub fn tick_hub_fan<H: HubFanHost>(host: &mut H) -> bool {
    let was_open = host.hub_fan().fully_open();
    let was_traveling = host.hub_fan().open_or_traveling();
    if !host.hub_fan().tick() {
        if was_traveling && !host.hub_fan().open_or_traveling() {
            apply_fan_visibility(host);
        }
        host.sync_dismiss_arm();
        return false;
    }
    if !was_traveling {
        apply_fan_visibility(host);
    }
    place_all_satellites(host);
    if was_open != host.hub_fan().fully_open() {
        rearm_satellite_input(host);
    }
    host.sync_dismiss_arm();
    host.flush_after_fan();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub_fan::is_hub_satellite_id;

    fn spec() -> OverlayButtonSpec {
        OverlayButtonSpec {
            id: "b1".into(),
            macro_name: "m".into(),
            face: ButtonFace {
                x: 100,
                y: 200,
                w: 48,
                h: 48,
                bg: [10, 20, 30, 200],
                border: [255, 255, 255, 255],
                border_width: 2.0,
                corner_radius: 8.0,
                icon_glyph: '+',
                icon: [255, 255, 255, 255],
                icon_hover: [200, 200, 200, 255],
                brand: false,
            },
            tip: "hello".into(),
            ui_scale: 1.0,
            busy: false,
            visibility_gated: false,
            input_enabled: true,
        }
    }

    #[test]
    fn spec_delta_classifies_changes() {
        let base = spec();
        assert_eq!(
            spec_delta(&base, &base),
            SpecDelta {
                pos: false,
                size: false,
                radius: false,
                style: false,
                input: false,
            }
        );

        let moved = OverlayButtonSpec {
            face: ButtonFace {
                x: base.face.x + 10,
                ..base.face
            },
            ..base.clone()
        };
        let d = spec_delta(&base, &moved);
        assert!(d.pos && !d.size && d.geom() && !d.radius && !d.style);

        let resized = OverlayButtonSpec {
            face: ButtonFace {
                w: base.face.w + 8,
                h: base.face.h + 8,
                ..base.face
            },
            ..base.clone()
        };
        let d = spec_delta(&base, &resized);
        assert!(d.size && !d.pos && d.geom() && !d.radius && !d.style);

        let rounded = OverlayButtonSpec {
            face: ButtonFace {
                corner_radius: base.face.corner_radius + 4.0,
                ..base.face
            },
            ..base.clone()
        };
        let d = spec_delta(&base, &rounded);
        assert!(d.radius && d.style && !d.geom());

        let near = OverlayButtonSpec {
            face: ButtonFace {
                corner_radius: base.face.corner_radius + f32::EPSILON / 2.0,
                ..base.face
            },
            ..base.clone()
        };
        assert_eq!(spec_delta(&base, &near), spec_delta(&base, &base));

        let thin = OverlayButtonSpec {
            face: ButtonFace {
                border_width: 0.25,
                ..base.face
            },
            ..base.clone()
        };
        let thinner = OverlayButtonSpec {
            face: ButtonFace {
                border_width: 0.25 + f32::EPSILON / 2.0,
                ..base.face
            },
            ..base.clone()
        };
        assert!(spec_delta(&thin, &thinner).style);

        let painted = OverlayButtonSpec {
            busy: true,
            ..base.clone()
        };
        let d = spec_delta(&base, &painted);
        assert!(d.style && !d.geom() && !d.radius && !d.input);

        let tinted = OverlayButtonSpec {
            face: ButtonFace {
                icon_hover: [1, 2, 3, 4],
                ..base.face
            },
            ..base.clone()
        };
        assert!(spec_delta(&base, &tinted).style);

        let muted = OverlayButtonSpec {
            input_enabled: false,
            ..base.clone()
        };
        let d = spec_delta(&base, &muted);
        assert!(d.input && !d.geom() && !d.radius && !d.style);
    }

    #[test]
    fn spec_contains_rect_edges() {
        let s = spec();
        assert!(spec_contains(&s, 100, 200));
        assert!(spec_contains(&s, 147, 247));
        assert!(!spec_contains(&s, 148, 200));
        assert!(!spec_contains(&s, 100, 248));
        assert!(!spec_contains(&s, 99, 200));
        assert!(!spec_contains(&s, 100, 199));
        let zero = OverlayButtonSpec {
            face: ButtonFace {
                w: 0,
                h: 0,
                ..s.face
            },
            ..s
        };
        assert!(spec_contains(&zero, 100, 200));
        assert!(!spec_contains(&zero, 101, 200));
    }

    #[test]
    fn spec_contains_rounded_skips_corners() {
        let mut s = spec();
        s.face.corner_radius = 24.0; // disk: radius is half of 48
        assert!(spec_contains_rounded(&s, 124, 224), "center");
        assert!(!spec_contains_rounded(&s, 100, 200), "top-left corner");
        assert!(!spec_contains_rounded(&s, 147, 200), "top-right corner");
        assert!(
            spec_contains_rounded(&s, 124, 200),
            "top middle of the disk"
        );
        s.face.corner_radius = 0.0;
        assert!(spec_contains_rounded(&s, 100, 200));
        assert!(!spec_contains_rounded(&s, 99, 200));
    }

    #[test]
    fn tip_refresh_tracks_text_and_scale() {
        let tip = TipRefreshInput {
            for_id: "b1",
            text: "hello",
            ui_scale: 1.0,
            mapped: true,
        };
        assert!(tip_refresh_decision(
            tip,
            Some(TipButtonView {
                text: "hello",
                ui_scale: 1.0,
            })
        )
        .is_none());
        assert_eq!(
            tip_refresh_decision(
                TipRefreshInput {
                    for_id: "b1",
                    text: "hello",
                    ui_scale: 1.0,
                    mapped: true,
                },
                Some(TipButtonView {
                    text: "world",
                    ui_scale: 1.0,
                })
            ),
            Some(("b1".into(), true))
        );
        assert_eq!(
            tip_refresh_decision(
                TipRefreshInput {
                    for_id: "b1",
                    text: "hello",
                    ui_scale: 1.0,
                    mapped: true,
                },
                Some(TipButtonView {
                    text: "  ",
                    ui_scale: 1.0,
                })
            ),
            Some(("b1".into(), false))
        );
        assert!(tip_refresh_decision(
            TipRefreshInput {
                for_id: "b1",
                text: "hello",
                ui_scale: 1.0,
                mapped: false,
            },
            Some(TipButtonView {
                text: "world",
                ui_scale: 1.0,
            })
        )
        .is_none());
    }

    #[test]
    fn button_should_show_respects_fan_omit_and_gate() {
        let mut ctx = GateVisibilityCtx {
            capture_omit: false,
            relocate: false,
            fan_open_or_traveling: false,
            found: HashMap::new(),
        };
        assert!(button_should_show("macro_a", false, &ctx));
        assert!(!button_should_show("__sqyre_hub_new_point", false, &ctx));
        ctx.fan_open_or_traveling = true;
        assert!(button_should_show("__sqyre_hub_new_point", false, &ctx));
        assert!(!button_should_show("gated", true, &ctx));
        ctx.found.insert("gated".into(), true);
        assert!(button_should_show("gated", true, &ctx));
        ctx.capture_omit = true;
        assert!(!button_should_show("gated", true, &ctx));
        ctx.capture_omit = false;
        ctx.found.insert("gated".into(), false);
        ctx.relocate = true;
        assert!(button_should_show("gated", true, &ctx));
    }

    #[test]
    fn prepare_hub_satellite_remembers_rest_and_places_closed() {
        let mut fan = HubFanState::default();
        let mut sat = OverlayButtonSpec {
            id: "__sqyre_hub_new_point".into(),
            face: ButtonFace {
                x: 200,
                y: 300,
                w: 40,
                h: 40,
                ..spec().face
            },
            ..spec()
        };
        assert!(is_hub_satellite_id(&sat.id));
        prepare_hub_satellite_spec(&mut fan, (100, 100, 48, 48), &mut sat);
        assert_eq!(fan.rest_of("__sqyre_hub_new_point"), Some((200, 300)));
        // Closed fan parks on hub origin.
        assert_eq!(sat.face.x, 100 + (48 - 40) / 2);
        assert_eq!(sat.face.y, 100 + (48 - 40) / 2);
        assert!(!sat.input_enabled);
    }

    #[derive(Default)]
    struct FakeHost {
        fan: HubFanState,
        hub: (i32, i32, u32, u32),
        buttons: HashMap<String, HubButtonView>,
        moved: Vec<String>,
        mapped_ops: Vec<(String, bool)>,
        input_sync: Vec<String>,
        relocate: bool,
        omit: bool,
        found: HashMap<String, bool>,
    }

    impl HubFanHost for FakeHost {
        fn hub_fan(&mut self) -> &mut HubFanState {
            &mut self.fan
        }
        fn hub_face_geom(&self) -> (i32, i32, u32, u32) {
            self.hub
        }
        fn hosted_button_ids(&self) -> Vec<String> {
            self.buttons.keys().cloned().collect()
        }
        fn button_view(&self, id: &str) -> Option<HubButtonView> {
            self.buttons.get(id).copied()
        }
        fn set_satellite_xy(&mut self, id: &str, x: i32, y: i32) {
            if let Some(b) = self.buttons.get_mut(id) {
                b.face_x = x;
                b.face_y = y;
            }
        }
        fn set_satellite_input_enabled(&mut self, id: &str, enabled: bool) {
            if let Some(b) = self.buttons.get_mut(id) {
                b.input_enabled = enabled;
            }
        }
        fn move_mapped_button(&mut self, id: &str) {
            self.moved.push(id.to_string());
        }
        fn sync_mapped_input(&mut self, id: &str) {
            self.input_sync.push(id.to_string());
        }
        fn set_button_mapped(&mut self, id: &str, show: bool) {
            if let Some(b) = self.buttons.get_mut(id) {
                b.mapped = show;
            }
            self.mapped_ops.push((id.to_string(), show));
        }
        fn visibility_context(&self) -> GateVisibilityCtx {
            GateVisibilityCtx {
                capture_omit: self.omit,
                relocate: self.relocate,
                fan_open_or_traveling: self.fan.open_or_traveling(),
                found: self.found.clone(),
            }
        }
        fn sync_dismiss_arm(&mut self) {}
        fn flush_after_fan(&mut self) {}
    }

    #[test]
    fn shared_place_and_visibility_drive_host_hooks() {
        let sat = "__sqyre_hub_new_point";
        let mut host = FakeHost {
            hub: (10, 20, 48, 48),
            ..FakeHost::default()
        };
        host.fan.remember_rest(sat, 100, 200);
        host.buttons.insert(
            sat.into(),
            HubButtonView {
                face_x: 10 + (48 - 40) / 2,
                face_y: 20 + (48 - 40) / 2,
                face_w: 40,
                face_h: 40,
                mapped: false,
                visibility_gated: false,
                input_enabled: false,
            },
        );
        host.buttons.insert(
            sqyre_persist::QUICK_ACCESS_HUB_ID.into(),
            HubButtonView {
                face_x: 10,
                face_y: 20,
                face_w: 48,
                face_h: 48,
                mapped: true,
                visibility_gated: false,
                input_enabled: true,
            },
        );

        // Closed: satellite stays unmapped; hub stays up.
        apply_hosted_visibility(&mut host);
        assert!(host.mapped_ops.is_empty());
        assert!(!host.buttons[sat].mapped);

        apply_hub_fan_cmd(&mut host, true, true);
        assert!(host.buttons[sat].mapped);
        assert!(host.mapped_ops.iter().any(|(id, show)| id == sat && *show));
        assert_eq!(host.buttons[sat].face_x, 100);
        assert_eq!(host.buttons[sat].face_y, 200);
        assert!(host.buttons[sat].input_enabled);

        // Travel with mapped satellite should move OS face (may take a few
        // ticks before smoothstep changes a pixel).
        host.fan.set_expanded(false, false);
        host.moved.clear();
        for _ in 0..20 {
            if !tick_hub_fan(&mut host) {
                break;
            }
            if host.moved.iter().any(|id| id == sat) {
                break;
            }
        }
        assert!(
            host.moved.iter().any(|id| id == sat),
            "expected move_mapped_button during collapse"
        );
    }
}
