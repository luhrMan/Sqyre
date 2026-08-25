//! Linux X11 window list + activate.

use crate::window_match::{paths_equal, pick_matching_icon, titles_equal};
use crate::{CaptureError, ProcessIcon, WindowInfo, PROCESS_ICON_TARGET_PX};
use parking_lot::Mutex;
use sqyre_ports::AutomationError;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_ulong;
use std::path::{Path, PathBuf};
use std::ptr;
use x11::xlib::{
    Atom, ClientMessage, Display, False, PropModeReplace, Success, True, Window, XChangeProperty,
    XDefaultRootWindow, XEvent, XFlush, XFree, XGetWMName, XGetWindowProperty, XInternAtom,
    XOpenDisplay, XSendEvent, _XDisplay, XA_ATOM, XA_CARDINAL, XA_WINDOW,
};

/// Title used by floating macro-overlay viewports (`macro_overlay`).
pub const OVERLAY_WM_TITLE: &str = "sqyre-overlay";

/// Process-lifetime X11 display for focus / window-list APIs (serialized via Mutex).
struct SharedFocusDisplay {
    display: *mut _XDisplay,
}

// SAFETY: the raw display pointer is only ever touched while `SHARED_FOCUS`
// (a `Mutex`) is held, so concurrent access from another thread never overlaps.
unsafe impl Send for SharedFocusDisplay {}

static SHARED_FOCUS: Mutex<Option<SharedFocusDisplay>> = Mutex::new(None);

fn with_display<F, R>(f: F) -> Result<R, CaptureError>
where
    F: FnOnce(*mut _XDisplay) -> Result<R, CaptureError>,
{
    let mut guard = SHARED_FOCUS.lock();
    if guard.is_none() {
        // SAFETY: `XOpenDisplay(null)` connects to the default display; the
        // returned pointer is null-checked before being stored.
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(CaptureError::OpenDisplay);
            }
            crate::x11_secondary::register(display);
            *guard = Some(SharedFocusDisplay { display });
        }
    }
    let display = guard.as_ref().expect("just inserted").display;
    f(display)
}

/// List open top-level windows with title + executable path.
pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    // SAFETY: `display` comes from `with_display`, which guarantees a live,
    // non-null `XOpenDisplay` connection for the duration of this call.
    with_display(|display| unsafe { list_on_display(display) })
}

/// Currently focused top-level window (`_NET_ACTIVE_WINDOW`), if any.
pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    crate::diag::mark_site("x11:get_active_window:before_open");
    let result = with_display(|display| {
        crate::diag::mark_site("x11:get_active_window:on_display");
        // SAFETY: `display` comes from `with_display`, which guarantees a live,
        // non-null `XOpenDisplay` connection for the duration of this call.
        unsafe { active_on_display(display) }
    });
    crate::diag::mark_site("x11:get_active_window:done");
    if let Err(ref e) = result {
        crate::diag::note(&format!("x11:get_active_window err: {e}"));
    }
    result
}

/// Icon for a bound process: matching open window's `_NET_WM_ICON`, if any.
pub fn process_icon(process_path: &str, window_title: &str) -> Option<ProcessIcon> {
    let path = process_path.trim();
    if path.is_empty() {
        return None;
    }
    with_display(|display| -> Result<Option<ProcessIcon>, CaptureError> {
        // SAFETY: `display` comes from `with_display`, which guarantees a live,
        // non-null `XOpenDisplay` connection for the duration of this call.
        let infos = unsafe {
            let root = XDefaultRootWindow(display);
            let clients = client_list(display, root)?;
            clients
                .into_iter()
                .filter_map(|win| window_info_of(display, win))
                .collect::<Vec<_>>()
        };
        Ok(pick_matching_icon(
            infos,
            path,
            window_title,
            |info| Some((info.title.clone(), info.process_path.clone())),
            |info, _wtitle, _wpath| info.icon.clone(),
        ))
    })
    .ok()
    .flatten()
}

/// Ask the WM to omit this process's overlay tool windows from taskbar / pager / Alt-Tab.
///
/// egui-winit's `with_taskbar(false)` is Windows-only. Overlay buttons use Dock type
/// (Mutter skips docks from Alt-Tab), but we still set `_NET_WM_STATE_SKIP_TASKBAR` and
/// `_NET_WM_STATE_SKIP_PAGER` on top-level windows we own whose title matches
/// [`OVERLAY_WM_TITLE`], and re-assert `_NET_WM_WINDOW_TYPE_DOCK` in case the WM
/// remapped the type.
pub fn skip_taskbar_for_overlay_windows() -> Result<(), CaptureError> {
    crate::diag::mark_site("x11:skip_taskbar:before_open");
    let result = with_display(|display| {
        crate::diag::mark_site("x11:skip_taskbar:on_display");
        // SAFETY: `display` comes from `with_display`, which guarantees a live,
        // non-null `XOpenDisplay` connection for the duration of this call.
        unsafe { skip_taskbar_on_display(display) }
    });
    crate::diag::mark_site("x11:skip_taskbar:done");
    if let Err(ref e) = result {
        crate::diag::note(&format!("x11:skip_taskbar err: {e}"));
    }
    result
}

pub(crate) fn activate_window(
    process_path: &str,
    window_title: &str,
) -> Result<(), AutomationError> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() || title.is_empty() {
        return Err(AutomationError::InvalidArg(
            "focus window: path and title required".into(),
        ));
    }

    // SAFETY: `display` comes from `with_display`, which guarantees a live,
    // non-null `XOpenDisplay` connection for the duration of this call.
    let activated = with_display(|display| unsafe { activate_on_display(display, path, title) })
        .map_err(|e| AutomationError::Backend(e.to_string()))?;
    if activated {
        Ok(())
    } else {
        Err(AutomationError::WindowNotFound {
            process_path: path.to_string(),
            title: title.to_string(),
        })
    }
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection that
// outlives this call; all Xlib calls inside are otherwise self-contained
// (properties are null/status-checked and freed with `XFree`).
unsafe fn list_on_display(display: *mut _XDisplay) -> Result<Vec<WindowInfo>, CaptureError> {
    let root = XDefaultRootWindow(display);
    let clients = client_list(display, root)?;
    let mut out = Vec::with_capacity(clients.len());
    let mut seen = HashSet::new();
    for win in clients {
        let Some(info) = window_info_of(display, win) else {
            continue;
        };
        let key = format!("{}:{}:{}", info.process_path, info.process_name, info.title);
        if !seen.insert(key) {
            continue;
        }
        out.push(info);
    }
    Ok(out)
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection that
// outlives this call.
unsafe fn active_on_display(display: *mut _XDisplay) -> Result<Option<WindowInfo>, CaptureError> {
    let root = XDefaultRootWindow(display);
    let Some(win) = active_window_id(display, root)? else {
        return Ok(None);
    };
    Ok(window_info_of(display, win))
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `root` window; `prop`/`nitems` are status- and null-checked before the
// `Window` read, and `XFree` is called on every path that allocates `prop`.
unsafe fn active_window_id(
    display: *mut Display,
    root: Window,
) -> Result<Option<Window>, CaptureError> {
    let atom = intern(display, "_NET_ACTIVE_WINDOW")?;
    let mut actual_type: Atom = 0;
    let mut actual_format: i32 = 0;
    let mut nitems: u64 = 0;
    let mut bytes_after: u64 = 0;
    let mut prop: *mut u8 = ptr::null_mut();
    let status = XGetWindowProperty(
        display,
        root,
        atom,
        0,
        1,
        False,
        XA_WINDOW,
        &mut actual_type,
        &mut actual_format,
        &mut nitems,
        &mut bytes_after,
        &mut prop,
    );
    if status != Success as i32 || prop.is_null() || nitems == 0 {
        if !prop.is_null() {
            XFree(prop as *mut _);
        }
        return Ok(None);
    }
    let win = *(prop as *const Window);
    XFree(prop as *mut _);
    if win == 0 {
        Ok(None)
    } else {
        Ok(Some(win))
    }
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `win` window.
unsafe fn window_info_of(display: *mut Display, win: Window) -> Option<WindowInfo> {
    let title = window_title_of(display, win)?;
    if title.trim().is_empty() {
        return None;
    }
    let pid = window_pid(display, win)?;
    let path = process_exe_path(pid).unwrap_or_default();
    let name = process_comm(pid).unwrap_or_else(|| {
        Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let icon = window_icon(display, win);
    Some(WindowInfo {
        title,
        process_name: name,
        process_path: path,
        icon,
    })
}

/// Read `_NET_WM_ICON` and pick the size closest to [`PROCESS_ICON_TARGET_PX`].
// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `win` window; `prop`/`nitems`/`actual_format` are checked before the
// slice is built, and `XFree` is called on every path that allocates `prop`.
unsafe fn window_icon(display: *mut Display, win: Window) -> Option<ProcessIcon> {
    let atom = intern(display, "_NET_WM_ICON").ok()?;
    let mut actual_type: Atom = 0;
    let mut actual_format: i32 = 0;
    let mut nitems: u64 = 0;
    let mut bytes_after: u64 = 0;
    let mut prop: *mut u8 = ptr::null_mut();
    // Enough for several multi-resolution icons (CARDINALs as platform longs).
    let status = XGetWindowProperty(
        display,
        win,
        atom,
        0,
        1 << 18,
        False,
        XA_CARDINAL,
        &mut actual_type,
        &mut actual_format,
        &mut nitems,
        &mut bytes_after,
        &mut prop,
    );
    if status != Success as i32 || prop.is_null() || nitems == 0 || actual_format != 32 {
        if !prop.is_null() {
            XFree(prop as *mut _);
        }
        return None;
    }
    let slice = std::slice::from_raw_parts(prop as *const c_ulong, nitems as usize);
    let icon = pick_net_wm_icon(slice);
    XFree(prop as *mut _);
    icon
}

/// Parse EWMH `_NET_WM_ICON` cardinals (ARGB in the low 32 bits of each long).
fn pick_net_wm_icon(data: &[c_ulong]) -> Option<ProcessIcon> {
    let mut best: Option<(u32, u32, usize)> = None; // w, h, pixel_start
    let mut i = 0usize;
    while i + 2 <= data.len() {
        let w = data[i] as u32;
        let h = data[i + 1] as u32;
        i += 2;
        let pixels = (w as usize).checked_mul(h as usize)?;
        if w == 0 || h == 0 || i.checked_add(pixels)? > data.len() {
            break;
        }
        let replace = match best {
            None => true,
            Some((bw, bh, _)) => icon_size_prefer(w, h, bw, bh),
        };
        if replace {
            best = Some((w, h, i));
        }
        i += pixels;
    }
    let (w, h, start) = best?;
    let px = (w as usize).checked_mul(h as usize)?;
    let mut rgba = Vec::with_capacity(px * 4);
    for &card in &data[start..start + px] {
        let argb = card as u32;
        let a = ((argb >> 24) & 0xff) as u8;
        let r = ((argb >> 16) & 0xff) as u8;
        let g = ((argb >> 8) & 0xff) as u8;
        let b = (argb & 0xff) as u8;
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    Some(ProcessIcon {
        width: w,
        height: h,
        rgba,
    })
}

/// Prefer sizes at or just above the target; otherwise the largest below.
fn icon_size_prefer(w: u32, h: u32, best_w: u32, best_h: u32) -> bool {
    icon_size_score(w, h) < icon_size_score(best_w, best_h)
}

fn icon_size_score(w: u32, h: u32) -> u32 {
    let side = w.min(h);
    let target = PROCESS_ICON_TARGET_PX;
    if side >= target {
        side - target
    } else {
        // Penalize undersized icons so we prefer any >= target when available.
        (target - side) + 10_000
    }
}

/// `Ok(false)` when no window matched; `Err` only for X11 failures.
// SAFETY: callers must pass a live, non-null Xlib `display` connection that
// outlives this call.
unsafe fn activate_on_display(
    display: *mut _XDisplay,
    process_path: &str,
    window_title: &str,
) -> Result<bool, CaptureError> {
    let root = XDefaultRootWindow(display);
    let clients = client_list(display, root)?;
    for win in clients {
        let Some(wtitle) = window_title_of(display, win) else {
            continue;
        };
        if !titles_equal(&wtitle, window_title) {
            continue;
        }
        let Some(pid) = window_pid(display, win) else {
            continue;
        };
        let Some(exe) = process_exe_path(pid) else {
            continue;
        };
        if !paths_equal(&exe, process_path) {
            continue;
        }
        return set_active_window(display, root, win).map(|()| true);
    }
    Ok(false)
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `root` window; `prop`/`nitems` are status- and null-checked before the
// slice is built, and `XFree` is called on every path that allocates `prop`.
unsafe fn client_list(display: *mut Display, root: Window) -> Result<Vec<Window>, CaptureError> {
    let atom = intern(display, "_NET_CLIENT_LIST")?;
    let mut actual_type: Atom = 0;
    let mut actual_format: i32 = 0;
    let mut nitems: u64 = 0;
    let mut bytes_after: u64 = 0;
    let mut prop: *mut u8 = ptr::null_mut();
    let status = XGetWindowProperty(
        display,
        root,
        atom,
        0,
        4096,
        False,
        XA_WINDOW,
        &mut actual_type,
        &mut actual_format,
        &mut nitems,
        &mut bytes_after,
        &mut prop,
    );
    if status != Success as i32 || prop.is_null() || nitems == 0 {
        if !prop.is_null() {
            XFree(prop as *mut _);
        }
        return Err(CaptureError::Message(
            "failed to read _NET_CLIENT_LIST".into(),
        ));
    }
    let slice = std::slice::from_raw_parts(prop as *const Window, nitems as usize);
    let out = slice.to_vec();
    XFree(prop as *mut _);
    Ok(out)
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `win` window; `name.value` is null-checked before `CStr::from_ptr`,
// and `XFree` is called after the C string is copied into an owned `String`.
unsafe fn window_title_of(display: *mut Display, win: Window) -> Option<String> {
    if let Ok(atom) = intern(display, "_NET_WM_NAME") {
        if let Some(s) = get_string_prop(display, win, atom) {
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }
    let mut name: x11::xlib::XTextProperty = std::mem::zeroed();
    if XGetWMName(display, win, &mut name) != 0 && !name.value.is_null() {
        let c = CStr::from_ptr(name.value as *const _);
        let s = c.to_string_lossy().into_owned();
        XFree(name.value as *mut _);
        if !s.trim().is_empty() {
            return Some(s);
        }
    }
    None
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `win` window; `prop`/`nitems` are status- and null-checked before the
// slice is built, and `XFree` is called on every path that allocates `prop`.
unsafe fn get_string_prop(display: *mut Display, win: Window, atom: Atom) -> Option<String> {
    let utf8 = intern(display, "UTF8_STRING").ok()?;
    let mut actual_type: Atom = 0;
    let mut actual_format: i32 = 0;
    let mut nitems: u64 = 0;
    let mut bytes_after: u64 = 0;
    let mut prop: *mut u8 = ptr::null_mut();
    let status = XGetWindowProperty(
        display,
        win,
        atom,
        0,
        4096,
        False,
        utf8,
        &mut actual_type,
        &mut actual_format,
        &mut nitems,
        &mut bytes_after,
        &mut prop,
    );
    if status != Success as i32 || prop.is_null() || nitems == 0 {
        if !prop.is_null() {
            XFree(prop as *mut _);
        }
        return None;
    }
    let bytes = std::slice::from_raw_parts(prop, nitems as usize);
    let s = String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string();
    XFree(prop as *mut _);
    Some(s)
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// valid `win` window; `prop`/`nitems` are status- and null-checked before the
// `u32` read, and `XFree` is called on every path that allocates `prop`.
unsafe fn window_pid(display: *mut Display, win: Window) -> Option<u32> {
    let atom = intern(display, "_NET_WM_PID").ok()?;
    let mut actual_type: Atom = 0;
    let mut actual_format: i32 = 0;
    let mut nitems: u64 = 0;
    let mut bytes_after: u64 = 0;
    let mut prop: *mut u8 = ptr::null_mut();
    let status = XGetWindowProperty(
        display,
        win,
        atom,
        0,
        1,
        False,
        XA_CARDINAL,
        &mut actual_type,
        &mut actual_format,
        &mut nitems,
        &mut bytes_after,
        &mut prop,
    );
    if status != Success as i32 || prop.is_null() || nitems == 0 {
        if !prop.is_null() {
            XFree(prop as *mut _);
        }
        return None;
    }
    let pid = *(prop as *const u32);
    XFree(prop as *mut _);
    if pid == 0 {
        None
    } else {
        Some(pid)
    }
}

fn process_exe_path(pid: u32) -> Option<String> {
    let link = PathBuf::from(format!("/proc/{pid}/exe"));
    std::fs::read_link(link)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

fn process_comm(pid: u32) -> Option<String> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let name = raw.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection plus a
// valid `root` and `win`; the `XEvent` is zeroed before its `client_message`
// variant is written, and it outlives the `XSendEvent` call that borrows it.
unsafe fn set_active_window(
    display: *mut Display,
    root: Window,
    win: Window,
) -> Result<(), CaptureError> {
    let atom = intern(display, "_NET_ACTIVE_WINDOW")?;
    let mut data = x11::xlib::ClientMessageData::new();
    data.set_long(0, 2); // source indication: pager
    data.set_long(1, 0);
    data.set_long(2, 0);
    data.set_long(3, 0);
    data.set_long(4, 0);

    let mut event: XEvent = std::mem::zeroed();
    event.client_message = x11::xlib::XClientMessageEvent {
        type_: ClientMessage,
        serial: 0,
        send_event: False,
        display,
        window: win,
        message_type: atom,
        format: 32,
        data,
    };

    const SUBSTRUCTURE_REDIRECT: i64 = 1 << 20;
    const SUBSTRUCTURE_NOTIFY: i64 = 1 << 19;
    let mask = SUBSTRUCTURE_REDIRECT | SUBSTRUCTURE_NOTIFY;
    let status = XSendEvent(display, root, False, mask, &mut event);
    if status == 0 {
        return Err(CaptureError::Message(
            "XSendEvent _NET_ACTIVE_WINDOW failed".into(),
        ));
    }
    XFlush(display);
    Ok(())
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection that
// outlives this call; the windows passed on come from `_NET_CLIENT_LIST` on
// that same connection.
unsafe fn skip_taskbar_on_display(display: *mut _XDisplay) -> Result<(), CaptureError> {
    let our_pid = std::process::id();
    let root = XDefaultRootWindow(display);
    let clients = client_list(display, root)?;
    let state = intern(display, "_NET_WM_STATE")?;
    let skip_taskbar = intern(display, "_NET_WM_STATE_SKIP_TASKBAR")?;
    let skip_pager = intern(display, "_NET_WM_STATE_SKIP_PAGER")?;
    let win_type = intern(display, "_NET_WM_WINDOW_TYPE")?;
    let type_dock = intern(display, "_NET_WM_WINDOW_TYPE_DOCK")?;
    let mut hinted = 0u32;
    for win in clients {
        let Some(pid) = window_pid(display, win) else {
            continue;
        };
        if pid != our_pid {
            continue;
        }
        let Some(title) = window_title_of(display, win) else {
            continue;
        };
        if title.trim() != OVERLAY_WM_TITLE {
            continue;
        }
        // Dock type: Mutter/GNOME omit these from Alt-Tab even without skip hints.
        set_window_type_dock(display, win, win_type, type_dock);
        // EWMH: clients request state changes via ClientMessage to the root.
        send_net_wm_state_add(display, root, win, state, skip_taskbar, skip_pager);
        hinted += 1;
    }
    XFlush(display);
    if hinted > 0 {
        crate::diag::mark_site(&format!("x11:skip_taskbar:hinted={hinted}"));
    }
    Ok(())
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection, a valid
// `win`, and atoms interned on that connection; the single-element `atom` buffer
// matches the `format: 32` / `nelements: 1` passed to `XChangeProperty`.
unsafe fn set_window_type_dock(
    display: *mut Display,
    win: Window,
    win_type: Atom,
    type_dock: Atom,
) {
    let mut atom = type_dock;
    XChangeProperty(
        display,
        win,
        win_type,
        XA_ATOM,
        32,
        PropModeReplace,
        &mut atom as *mut Atom as *mut u8,
        1,
    );
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection, valid
// `root`/`win` windows, and atoms interned on that connection; the `XEvent` is
// zeroed before its `client_message` variant is written and outlives `XSendEvent`.
unsafe fn send_net_wm_state_add(
    display: *mut Display,
    root: Window,
    win: Window,
    state_atom: Atom,
    atom1: Atom,
    atom2: Atom,
) {
    const NET_WM_STATE_ADD: i64 = 1;
    let mut data = x11::xlib::ClientMessageData::new();
    data.set_long(0, NET_WM_STATE_ADD);
    data.set_long(1, atom1 as i64);
    data.set_long(2, atom2 as i64);
    data.set_long(3, 1); // source: application
    data.set_long(4, 0);

    let mut event: XEvent = std::mem::zeroed();
    event.client_message = x11::xlib::XClientMessageEvent {
        type_: ClientMessage,
        serial: 0,
        send_event: True,
        display,
        window: win,
        message_type: state_atom,
        format: 32,
        data,
    };

    const SUBSTRUCTURE_REDIRECT: i64 = 1 << 20;
    const SUBSTRUCTURE_NOTIFY: i64 = 1 << 19;
    let mask = SUBSTRUCTURE_REDIRECT | SUBSTRUCTURE_NOTIFY;
    let _ = XSendEvent(display, root, False, mask, &mut event);
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection; the
// `CString` outlives the `XInternAtom` call that reads its pointer.
unsafe fn intern(display: *mut Display, name: &str) -> Result<Atom, CaptureError> {
    let c = CString::new(name).map_err(|e| CaptureError::Message(e.to_string()))?;
    let atom = XInternAtom(display, c.as_ptr(), False);
    if atom == 0 {
        Err(CaptureError::Message(format!("XInternAtom {name} failed")))
    } else {
        Ok(atom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_net_wm_icon_prefers_near_target() {
        // Two icons: 16x16 solid red, 48x48 solid green (ARGB in low 32 bits).
        let mut data = Vec::new();
        data.push(16);
        data.push(16);
        data.extend(std::iter::repeat_n(0xFFFF0000u64, 16 * 16));
        data.push(48);
        data.push(48);
        data.extend(std::iter::repeat_n(0xFF00FF00u64, 48 * 48));
        let icon = pick_net_wm_icon(&data).expect("icon");
        assert_eq!((icon.width, icon.height), (48, 48));
        assert_eq!(icon.rgba.len(), 48 * 48 * 4);
        assert_eq!(&icon.rgba[..4], &[0, 255, 0, 255]);
    }

    #[test]
    fn pick_net_wm_icon_argb_to_rgba() {
        let data = [1u64, 1, 0x80AABBCC];
        let icon = pick_net_wm_icon(&data).expect("icon");
        assert_eq!(icon.rgba, vec![0xAA, 0xBB, 0xCC, 0x80]);
    }
}
