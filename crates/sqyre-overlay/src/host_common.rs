//! OS-neutral overlay host DTOs and helpers shared by X11 and Windows button hosts.

use crate::ButtonFace;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
