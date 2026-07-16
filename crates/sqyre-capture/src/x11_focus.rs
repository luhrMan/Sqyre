//! Linux X11 window list + activate.

use crate::WindowInfo;
use parking_lot::Mutex;
use sqyre_executor::WindowFocuser;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use x11::xlib::{
    Atom, ClientMessage, Display, False, Success, Window, XDefaultRootWindow, XEvent, XFlush,
    XFree, XGetWindowProperty, XGetWMName, XInternAtom, XOpenDisplay, XSendEvent, XA_CARDINAL,
    XA_WINDOW, _XDisplay,
};

/// Title used by floating macro-overlay viewports (`macro_overlay`).
pub const OVERLAY_WM_TITLE: &str = "sqyre-overlay";

/// Process-lifetime X11 display for focus / window-list APIs (serialized via Mutex).
struct SharedFocusDisplay {
    display: *mut _XDisplay,
}

// X11 display pointer: all access goes through SHARED_FOCUS Mutex.
unsafe impl Send for SharedFocusDisplay {}

static SHARED_FOCUS: Mutex<Option<SharedFocusDisplay>> = Mutex::new(None);

fn with_display<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(*mut _XDisplay) -> Result<R, String>,
{
    let mut guard = SHARED_FOCUS.lock();
    if guard.is_none() {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay failed".into());
            }
            *guard = Some(SharedFocusDisplay { display });
        }
    }
    let display = guard.as_ref().expect("just inserted").display;
    f(display)
}

/// Focus a top-level window by executable path + window title.
#[derive(Debug, Default, Clone, Copy)]
pub struct X11WindowFocuser;

impl WindowFocuser for X11WindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), String> {
        activate_window(process_path, window_title)
    }
}

/// List open top-level windows with title + executable path.
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    with_display(|display| unsafe { list_on_display(display) })
}

/// Currently focused top-level window (`_NET_ACTIVE_WINDOW`), if any.
pub fn get_active_window() -> Result<Option<WindowInfo>, String> {
    crate::diag::mark_site("x11:get_active_window:before_open");
    let result = with_display(|display| {
        crate::diag::mark_site("x11:get_active_window:on_display");
        unsafe { active_on_display(display) }
    });
    crate::diag::mark_site("x11:get_active_window:done");
    if let Err(ref e) = result {
        crate::diag::note(&format!("x11:get_active_window err: {e}"));
    }
    result
}

/// Ask the WM to omit this process's overlay tool windows from taskbar / pager / Alt-Tab.
///
/// egui-winit's `with_taskbar(false)` is Windows-only; on X11 Utility type alone is not
/// enough on many DEs (GNOME, Pop, etc.). We set `_NET_WM_STATE_SKIP_TASKBAR` and
/// `_NET_WM_STATE_SKIP_PAGER` on top-level windows we own whose title matches
/// [`OVERLAY_WM_TITLE`].
pub fn skip_taskbar_for_overlay_windows() -> Result<(), String> {
    crate::diag::mark_site("x11:skip_taskbar:before_open");
    let result = with_display(|display| {
        crate::diag::mark_site("x11:skip_taskbar:on_display");
        unsafe { skip_taskbar_on_display(display) }
    });
    crate::diag::mark_site("x11:skip_taskbar:done");
    if let Err(ref e) = result {
        crate::diag::note(&format!("x11:skip_taskbar err: {e}"));
    }
    result
}

fn activate_window(process_path: &str, window_title: &str) -> Result<(), String> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() || title.is_empty() {
        return Err("path and title required".into());
    }

    with_display(|display| unsafe { activate_on_display(display, path, title) })
}

unsafe fn list_on_display(display: *mut _XDisplay) -> Result<Vec<WindowInfo>, String> {
    let root = XDefaultRootWindow(display);
    let clients = client_list(display, root)?;
    let mut out = Vec::with_capacity(clients.len());
    let mut seen = HashSet::new();
    for win in clients {
        let Some(info) = window_info_of(display, win) else {
            continue;
        };
        let key = format!(
            "{}:{}:{}",
            info.process_path, info.process_name, info.title
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(info);
    }
    Ok(out)
}

unsafe fn active_on_display(display: *mut _XDisplay) -> Result<Option<WindowInfo>, String> {
    let root = XDefaultRootWindow(display);
    let Some(win) = active_window_id(display, root)? else {
        return Ok(None);
    };
    Ok(window_info_of(display, win))
}

unsafe fn active_window_id(
    display: *mut Display,
    root: Window,
) -> Result<Option<Window>, String> {
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
    Some(WindowInfo {
        title,
        process_name: name,
        process_path: path,
    })
}

unsafe fn activate_on_display(
    display: *mut _XDisplay,
    process_path: &str,
    window_title: &str,
) -> Result<(), String> {
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
        return set_active_window(display, root, win);
    }
    Err(format!(
        "no window with title {window_title:?} from {process_path:?}"
    ))
}

unsafe fn client_list(display: *mut Display, root: Window) -> Result<Vec<Window>, String> {
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
        return Err("failed to read _NET_CLIENT_LIST".into());
    }
    let slice = std::slice::from_raw_parts(prop as *const Window, nitems as usize);
    let out = slice.to_vec();
    XFree(prop as *mut _);
    Ok(out)
}

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
    let s = String::from_utf8_lossy(bytes).trim_end_matches('\0').to_string();
    XFree(prop as *mut _);
    Some(s)
}

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

unsafe fn set_active_window(
    display: *mut Display,
    root: Window,
    win: Window,
) -> Result<(), String> {
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
        return Err("XSendEvent _NET_ACTIVE_WINDOW failed".into());
    }
    XFlush(display);
    Ok(())
}

unsafe fn skip_taskbar_on_display(display: *mut _XDisplay) -> Result<(), String> {
    let our_pid = std::process::id();
    let root = XDefaultRootWindow(display);
    let clients = client_list(display, root)?;
    let state = intern(display, "_NET_WM_STATE")?;
    let skip_taskbar = intern(display, "_NET_WM_STATE_SKIP_TASKBAR")?;
    let skip_pager = intern(display, "_NET_WM_STATE_SKIP_PAGER")?;
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
        // EWMH: clients request state changes via ClientMessage to the root.
        send_net_wm_state_add(display, root, win, state, skip_taskbar, skip_pager);
    }
    XFlush(display);
    Ok(())
}

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
        send_event: False,
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

unsafe fn intern(display: *mut Display, name: &str) -> Result<Atom, String> {
    let c = CString::new(name).map_err(|e| e.to_string())?;
    let atom = XInternAtom(display, c.as_ptr(), False);
    if atom == 0 {
        Err(format!("XInternAtom {name} failed"))
    } else {
        Ok(atom)
    }
}

fn paths_equal(a: &str, b: &str) -> bool {
    let a = Path::new(a.trim());
    let b = Path::new(b.trim());
    if a.as_os_str().is_empty() || b.as_os_str().is_empty() {
        return false;
    }
    a == b
}

fn titles_equal(a: &str, b: &str) -> bool {
    a.trim() == b.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_titles() {
        assert!(paths_equal("/usr/bin/foo", "/usr/bin/foo"));
        assert!(!paths_equal("/usr/bin/foo", "/usr/bin/bar"));
        assert!(titles_equal(" Hi ", "Hi"));
    }
}
