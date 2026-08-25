//! Linux window list / focus: Wayland (foreign-toplevel + AT-SPI) merged with X11.

use super::{atspi_windows, foreign_toplevel, wayland_clients};
use crate::window_match::{paths_equal, titles_equal};
use crate::x11_focus;
use crate::{CaptureError, WindowInfo};
use sqyre_ports::{AutomationError, WindowFocuser};

/// Bring a window to the front by executable path + window title.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsWindowFocuser;

impl WindowFocuser for OsWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        activate_window(process_path, window_title)
    }
}

pub(crate) fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    let wayland = wayland_list();
    let atspi = atspi_list();
    let clients = wayland_clients_list();
    let x11 = x11_focus::list_open_windows();
    let merged = merge_window_lists([
        wayland.as_deref().unwrap_or(&[]),
        atspi.as_deref().unwrap_or(&[]),
        clients.as_deref().unwrap_or(&[]),
        x11.as_deref().unwrap_or(&[]),
    ]);
    crate::cap_log(
        "FOCUS",
        if merged.is_empty() { "fail" } else { "ok" },
        &format!(
            "list wayland={} atspi={} clients={} x11={} merged={} wayland_err={} atspi_err={}",
            wayland.as_ref().map(Vec::len).unwrap_or(0),
            atspi.as_ref().map(Vec::len).unwrap_or(0),
            clients.as_ref().map(Vec::len).unwrap_or(0),
            x11.as_ref().map(Vec::len).unwrap_or(0),
            merged.len(),
            wayland
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".into()),
            atspi
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".into()),
        ),
    );
    if merged.is_empty() {
        return x11.map_err(|e| {
            let extra = [wayland.err(), atspi.err()]
                .into_iter()
                .flatten()
                .map(|err| err.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            if extra.is_empty() {
                e
            } else {
                CaptureError::Message(format!("{e}; {extra}"))
            }
        });
    }
    Ok(merged)
}

pub(crate) fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    if let Ok(Some(w)) = foreign_toplevel::active_window() {
        return Ok(Some(w));
    }
    if let Ok(Some(w)) = atspi_windows::active_window() {
        return Ok(Some(w));
    }
    x11_focus::get_active_window()
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
    if let Ok(true) = foreign_toplevel::activate(path, title) {
        return Ok(());
    }
    if let Ok(true) = atspi_windows::activate(path, title) {
        return Ok(());
    }
    if let Ok(true) = wayland_clients::activate(path, title) {
        return Ok(());
    }
    x11_focus::activate_window(path, title)
}

pub(crate) fn toplevel_focus_available() -> Result<(), CaptureError> {
    let wayland = foreign_toplevel::list_windows();
    let atspi = atspi_windows::list_windows();
    match (&wayland, &atspi) {
        (Ok(w), _) if !w.is_empty() => Ok(()),
        (_, Ok(a)) if !a.is_empty() => Ok(()),
        _ if wayland_clients_list().ok().is_some_and(|c| !c.is_empty()) => Ok(()),
        (Err(w), Err(a)) => Err(CaptureError::Message(format!(
            "Wayland window list unavailable ({w}); AT-SPI unavailable ({a})"
        ))),
        _ => Err(CaptureError::Message(format!(
            "Wayland compositor has no foreign-toplevel ({}); AT-SPI listed no windows ({})",
            wayland
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "empty".into()),
            atspi
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "empty".into()),
        ))),
    }
}

fn wayland_list() -> Result<Vec<WindowInfo>, CaptureError> {
    if !crate::linux::LinuxSessionInfo::detect().has_wayland {
        return Ok(Vec::new());
    }
    foreign_toplevel::list_windows()
}

fn atspi_list() -> Result<Vec<WindowInfo>, CaptureError> {
    if !crate::linux::LinuxSessionInfo::detect().has_wayland {
        return Ok(Vec::new());
    }
    atspi_windows::list_windows()
}

fn wayland_clients_list() -> Result<Vec<WindowInfo>, CaptureError> {
    if !crate::linux::LinuxSessionInfo::detect().has_wayland {
        return Ok(Vec::new());
    }
    wayland_clients::list_windows()
}

fn merge_window_lists(
    lists: impl IntoIterator<Item = impl AsRef<[WindowInfo]>>,
) -> Vec<WindowInfo> {
    let mut out: Vec<WindowInfo> = Vec::new();
    for list in lists {
        for w in list.as_ref() {
            if w.title.trim() == x11_focus::OVERLAY_WM_TITLE {
                continue;
            }
            if let Some(existing) = out.iter_mut().find(|e| same_window(e, w)) {
                enrich(existing, w);
            } else {
                out.push(w.clone());
            }
        }
    }
    out
}

fn same_window(a: &WindowInfo, b: &WindowInfo) -> bool {
    if !titles_equal(&a.title, &b.title) {
        return false;
    }
    let path_a = a.process_path.trim();
    let path_b = b.process_path.trim();
    if !path_a.is_empty() && !path_b.is_empty() {
        return paths_equal(&a.process_path, &b.process_path);
    }
    let name_a = a.process_name.trim();
    let name_b = b.process_name.trim();
    if !name_a.is_empty() && !name_b.is_empty() {
        return name_a.eq_ignore_ascii_case(name_b);
    }
    true
}

fn enrich(dst: &mut WindowInfo, src: &WindowInfo) {
    if dst.process_path.trim().is_empty() && !src.process_path.trim().is_empty() {
        dst.process_path = src.process_path.clone();
    }
    if dst.process_name.trim().is_empty() && !src.process_name.trim().is_empty() {
        dst.process_name = src.process_name.clone();
    }
    if dst.icon.is_none() {
        dst.icon = src.icon.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(title: &str, name: &str, path: &str) -> WindowInfo {
        WindowInfo {
            title: title.into(),
            process_name: name.into(),
            process_path: path.into(),
            icon: None,
        }
    }

    #[test]
    fn merge_dedupes_same_title_and_path() {
        let a = vec![win("Terminal", "gnome-terminal", "/usr/bin/gnome-terminal")];
        let b = vec![win(
            "Terminal",
            "gnome-terminal-server",
            "/usr/bin/gnome-terminal",
        )];
        let merged = merge_window_lists([&a, &b]);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn merge_keeps_distinct_apps_with_same_title() {
        let a = vec![win(
            "Settings",
            "gnome-control-center",
            "/usr/bin/gnome-control-center",
        )];
        let b = vec![win("Settings", "steam", "/usr/bin/steam")];
        let merged = merge_window_lists([&a, &b]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merge_fills_empty_path_from_other_list() {
        let a = vec![win("Firefox", "firefox", "")];
        let b = vec![win("Firefox", "firefox", "/usr/lib/firefox/firefox")];
        let merged = merge_window_lists([&a, &b]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].process_path, "/usr/lib/firefox/firefox");
    }
}
