//! Raster blit and X shape helpers for the X11 overlay host.

use super::*;

pub(super) fn paint_button(state: &HostState, btn: &LiveButton, update_shape: bool) {
    let paint = ButtonPaint::from_face(&btn.spec.face, btn.hovered, btn.spec.busy, btn.busy_phase);
    let rgba = if btn.argb {
        raster::rasterize_argb(&paint)
    } else {
        raster::rasterize(&paint)
    };
    // ARGB faces carry no Bounding — the painted alpha is the shape, and an
    // empty ShapeInput already routes clicks to the hit cover.
    if update_shape && !btn.argb {
        if paint.busy {
            // The spinner animates with pixels-only repaints (reapplying
            // Shape* every frame made the face eat clicks on XWayland), so
            // the bounding must cover the whole orbit — a single frame's ink
            // would clip the strokes wherever they leave it. The union is
            // phase-invariant: apply once per busy stint, not on every
            // expose/hover repaint.
            if btn.face_bounding.get() != FaceBounding::BusyUnion {
                // Grow the bounding BEFORE the blit: PutImage is clipped by
                // the current bounding, so the ink must land inside the
                // union that will display it.
                let alpha = raster::busy_shape_alpha(&paint);
                apply_face_chrome_bounding(state.display, btn.win, paint.w, paint.h, &alpha);
                btn.face_bounding.set(FaceBounding::BusyUnion);
            }
        } else if btn.face_bounding.get() == FaceBounding::Chrome {
            // Idle face: derive the Bounding from the painted alpha so the
            // clip is pixel-exact with the blit. The old integer-circle
            // XRectangle shape disagreed with the SDF fill at corner rows —
            // inside-shape gaps showed unpainted flecks past the border ring.
            // Apply the shape first so the clipped blit below covers ink that
            // grew since the last frame.
            apply_face_chrome_bounding(state.display, btn.win, paint.w, paint.h, &rgba);
        } else {
            // Union→chrome shrink: overwrite the stale S ink while the union
            // bounding still covers it, then shrink — otherwise the clipped
            // PutImage leaves old S pixels in the backing store, which
            // composite through as a ghost when the shape lags.
            btn.face_bounding.set(FaceBounding::Chrome);
            blit_rgba(state, btn.win, paint.w, paint.h, &rgba, false);
            apply_face_chrome_bounding(state.display, btn.win, paint.w, paint.h, &rgba);
            return;
        }
    }
    blit_rgba(state, btn.win, paint.w, paint.h, &rgba, btn.argb);
}

pub(super) fn blit_rgba(state: &HostState, win: Window, w: u32, h: u32, rgba: &[u8], argb: bool) {
    // SAFETY: HostState display/gc invariant; `win` is a button/tip on this display.
    // `XCreateImage` borrows `packed` until we null `data` and destroy the image.
    unsafe {
        let (visual, depth, gc, alpha_mask) = if argb {
            let Some(av) = state.argb else {
                return;
            };
            if state.argb_gc.is_null() {
                return;
            }
            // In a 32-bit TrueColor pixel the alpha byte lives in whatever
            // bits the RGB masks do not claim (top byte in practice).
            (
                av.visual,
                av.depth,
                state.argb_gc,
                !(av.red_mask | av.green_mask | av.blue_mask) & 0xFF00_0000,
            )
        } else {
            (
                XDefaultVisual(state.display, state.screen),
                XDefaultDepth(state.display, state.screen),
                state.gc,
                0,
            )
        };
        let (red_mask, green_mask, blue_mask) = if visual.is_null() {
            (0x00FF_0000u64, 0x0000_FF00, 0x0000_00FF)
        } else {
            (
                (*visual).red_mask,
                (*visual).green_mask,
                (*visual).blue_mask,
            )
        };
        let mut packed = pack_truecolor(rgba, red_mask, green_mask, blue_mask, alpha_mask);
        let ximage = XCreateImage(
            state.display,
            visual,
            depth as c_uint,
            ZPixmap,
            0,
            packed.as_mut_ptr().cast::<c_char>(),
            w,
            h,
            32,
            (w * 4) as c_int,
        );
        if ximage.is_null() {
            return;
        }
        (*ximage).byte_order = LSBFirst;
        (*ximage).bits_per_pixel = 32;
        XPutImage(state.display, win, gc, ximage, 0, 0, 0, 0, w, h);
        (*ximage).data = std::ptr::null_mut();
        XDestroyImage(ximage);
    }
}

pub(super) fn pack_truecolor(
    rgba: &[u8],
    red_mask: u64,
    green_mask: u64,
    blue_mask: u64,
    alpha_mask: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.as_chunks::<4>().0 {
        let packed = place_channel(px[0], red_mask)
            | place_channel(px[1], green_mask)
            | place_channel(px[2], blue_mask)
            | place_channel(px[3], alpha_mask);
        out.extend_from_slice(&packed.to_le_bytes());
    }
    out
}

pub(super) fn place_channel(value: u8, mask: u64) -> u32 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    (u32::from(value) << shift) & mask as u32
}

/// Transparent face: chrome-only Bounding + empty Input (hollow stays clickable via hit).
pub(super) fn apply_face_chrome_bounding(
    display: *mut Display,
    win: Window,
    w: u32,
    h: u32,
    rgba: &[u8],
) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let runs = raster::shape_rects_from_alpha(rgba, w, h);
    let mut bound: Vec<XRectangle> = runs
        .into_iter()
        .map(|(x, y, width, height)| XRectangle {
            x,
            y,
            width,
            height,
        })
        .collect();
    if bound.is_empty() {
        bound.push(XRectangle {
            x: (w / 2) as i16,
            y: (h / 2) as i16,
            width: 1,
            height: 1,
        });
    }
    apply_shape_region(display, win, SHAPE_BOUNDING, &mut bound);
    apply_empty_input_shape(display, win);
}

pub(super) fn apply_chooser_input_shape(
    display: *mut Display,
    hit: Window,
    w: u32,
    h: u32,
    radius_px: f32,
) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let radius = radius_px.round().max(0.0) as i32;
    let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
    if rects.is_empty() {
        rects.push(XRectangle {
            x: 0,
            y: 0,
            width: w as u16,
            height: h as u16,
        });
    }
    apply_shape_region(display, hit, SHAPE_INPUT, &mut rects);
}

/// Rounded bounding + empty input so the tip never steals hover from the button.
pub(super) fn apply_tip_shape(display: *mut Display, win: Window, w: u32, h: u32, ui_scale: f32) {
    apply_rounded_shape(display, win, w, h, tip_corner_px(ui_scale));
}

/// Hit cover: rounded ShapeInput matching the button disk.
pub(super) fn apply_hit_rounded_input(
    display: *mut Display,
    hit: Window,
    w: u32,
    h: u32,
    radius_px: f32,
) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let radius = radius_px.round().max(0.0) as i32;
    let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
    if rects.is_empty() {
        rects.push(XRectangle {
            x: 0,
            y: 0,
            width: w as u16,
            height: h as u16,
        });
    }
    apply_shape_region(display, hit, SHAPE_INPUT, &mut rects);
}

/// Tip / face: no pointer events (visual-only).
pub(super) fn apply_empty_input_shape(display: *mut Display, win: Window) {
    if !xfixes_supported(display) {
        return;
    }
    // SAFETY: callers pass the host-thread display and a window created on it.
    unsafe {
        let empty = XFixesCreateRegion(display, std::ptr::null_mut(), 0);
        if empty == 0 {
            return;
        }
        XFixesSetWindowShapeRegion(display, win, SHAPE_INPUT, 0, 0, empty);
        XFixesDestroyRegion(display, empty);
    }
}

/// Opaque tip panel: rounded Bounding + empty Input.
pub(super) fn apply_rounded_shape(
    display: *mut Display,
    win: Window,
    w: u32,
    h: u32,
    radius_px: f32,
) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let radius = radius_px.round().max(0.0) as i32;
    let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
    apply_shape_region(display, win, SHAPE_BOUNDING, &mut rects);
    apply_empty_input_shape(display, win);
}

/// Cached XFixes probe — every shape update used to repeat the
/// `XFixesQueryExtension` + `XFixesQueryVersion` round-trips. Host-thread only.
pub(super) fn xfixes_supported(display: *mut Display) -> bool {
    thread_local! {
        static CACHED: Cell<Option<bool>> = const { Cell::new(None) };
    }
    CACHED.with(|c| {
        if let Some(ok) = c.get() {
            return ok;
        }
        // SAFETY: callers pass the host-thread display; version out-params are
        // stack locals. The `x11` binding declares `minor_version` as
        // `*const c_int` even though the C API writes through it — pass a real
        // slot (NULL would crash the server call that writes to it).
        let ok = unsafe {
            let mut event_base = 0;
            let mut error_base = 0;
            if XFixesQueryExtension(display, &mut event_base, &mut error_base) == 0 {
                false
            } else {
                let mut major: c_int = 4;
                let mut minor: c_int = 0;
                XFixesQueryVersion(
                    display,
                    &mut major,
                    &mut minor as *mut c_int as *const c_int,
                ) != 0
            }
        };
        c.set(Some(ok));
        ok
    })
}

pub(super) fn apply_shape_region(
    display: *mut Display,
    win: Window,
    kind: c_int,
    rects: &mut [XRectangle],
) {
    if rects.is_empty() || !xfixes_supported(display) {
        return;
    }
    // SAFETY: callers pass the host-thread display and a window created on it.
    unsafe {
        let region = XFixesCreateRegion(display, rects.as_mut_ptr(), rects.len() as c_int);
        if region == 0 {
            return;
        }
        XFixesSetWindowShapeRegion(display, win, kind, 0, 0, region);
        XFixesDestroyRegion(display, region);
    }
}

pub(super) fn rounded_rect_xrectangles(w: i32, h: i32, radius: i32) -> Vec<XRectangle> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let r = radius.max(0).min(w / 2).min(h / 2);
    let mut out = Vec::with_capacity(h as usize);
    for y in 0..h {
        let inset = if r == 0 {
            0
        } else if y < r {
            circle_inset(r, r - 1 - y)
        } else if y >= h - r {
            circle_inset(r, y - (h - r))
        } else {
            0
        };
        let width = (w - 2 * inset).max(0);
        if width > 0 {
            out.push(XRectangle {
                x: inset as i16,
                y: y as i16,
                width: width as u16,
                height: 1,
            });
        }
    }
    out
}

pub(super) fn circle_inset(r: i32, y_from_edge: i32) -> i32 {
    let r = r as f64;
    let y = y_from_edge.clamp(0, r as i32) as f64;
    let inside = (r * r - y * y).max(0.0).sqrt();
    (r - inside).ceil().max(0.0) as i32
}
