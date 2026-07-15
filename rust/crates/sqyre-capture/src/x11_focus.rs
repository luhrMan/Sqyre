//! Linux X11 window list + activate (Go `ActiveWindows` / `activateWindow`).

use crate::WindowInfo;
use sqyre_executor::WindowFocuser;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use x11::xlib::{
    Atom, ClientMessage, Display, False, Success, Window, XCloseDisplay, XDefaultRootWindow,
    XEvent, XFlush, XFree, XGetWindowProperty, XGetWMName, XInternAtom, XOpenDisplay, XSendEvent,
    XA_CARDINAL, XA_WINDOW, _XDisplay,
};

/// Focus a top-level window by executable path + window title.
#[derive(Debug, Default, Clone, Copy)]
pub struct X11WindowFocuser;

impl WindowFocuser for X11WindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), String> {
        activate_window(process_path, window_title)
    }
}

/// List open top-level windows with title + executable path (Go `listOpenWindows`).
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        let result = list_on_display(display);
        XCloseDisplay(display);
        result
    }
}

fn activate_window(process_path: &str, window_title: &str) -> Result<(), String> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() || title.is_empty() {
        return Err("path and title required".into());
    }

    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        let result = activate_on_display(display, path, title);
        XCloseDisplay(display);
        result
    }
}

unsafe fn list_on_display(display: *mut _XDisplay) -> Result<Vec<WindowInfo>, String> {
    let root = XDefaultRootWindow(display);
    let clients = client_list(display, root)?;
    let mut out = Vec::with_capacity(clients.len());
    let mut seen = HashSet::new();
    for win in clients {
        let Some(title) = window_title_of(display, win) else {
            continue;
        };
        if title.trim().is_empty() {
            continue;
        }
        let Some(pid) = window_pid(display, win) else {
            continue;
        };
        let path = process_exe_path(pid).unwrap_or_default();
        let name = process_comm(pid).unwrap_or_else(|| {
            Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        let key = format!("{pid}:{path}:{title}");
        if !seen.insert(key) {
            continue;
        }
        out.push(WindowInfo {
            title,
            process_name: name,
            process_path: path,
        });
    }
    Ok(out)
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
