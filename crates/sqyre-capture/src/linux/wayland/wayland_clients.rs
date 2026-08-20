//! GNOME (and other Mutter) window list without foreign-toplevel.
//!
//! Enumerate processes connected to the session's Wayland socket. Used when the
//! compositor does not advertise foreign-toplevel and AT-SPI has no applications.

use super::app_resolve::{desktop_app_id_for_pid, desktop_label_for_pid, process_from_pid};
use crate::WindowInfo;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn list_windows() -> Result<Vec<WindowInfo>, String> {
    let socket = wayland_socket_path();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let proc = fs::read_dir("/proc").map_err(|e| format!("/proc: {e}"))?;
    for ent in proc.flatten() {
        let pid = ent.file_name();
        let Some(pid) = pid.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        if !connects_to_wayland(pid, socket.as_deref()) {
            continue;
        }
        let (process_name, process_path) = process_from_pid(pid);
        if process_path.is_empty() || skip_helper(&process_name, &process_path) {
            continue;
        }
        if !seen.insert(process_path.clone()) {
            continue;
        }
        let title = desktop_label_for_pid(pid, &process_path)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| process_name.clone());
        if title.trim().is_empty() {
            continue;
        }
        out.push(WindowInfo {
            title,
            process_name,
            process_path,
            icon: None,
        });
    }
    Ok(out)
}

pub(crate) fn activate(process_path: &str, window_title: &str) -> Result<bool, String> {
    let want = process_path.trim();
    if want.is_empty() {
        return Ok(false);
    }
    let Some(pid) = pid_for_exe(want) else {
        return Ok(false);
    };
    let Some(app_id) = desktop_app_id_for_pid(pid, want) else {
        return Ok(false);
    };
    let _ = window_title;
    activate_freedesktop_application(&app_id)
}

fn pid_for_exe(process_path: &str) -> Option<u32> {
    let want = Path::new(process_path).file_name()?;
    let proc = fs::read_dir("/proc").ok()?;
    for ent in proc.flatten() {
        let pid = ent.file_name();
        let pid = pid.to_str()?.parse::<u32>().ok()?;
        let exe = fs::read_link(ent.path().join("exe")).ok()?;
        if exe.file_name() == Some(want) || exe.to_string_lossy() == process_path {
            return Some(pid);
        }
    }
    None
}

fn activate_freedesktop_application(app_id: &str) -> Result<bool, String> {
    let session = zbus::blocking::Connection::session().map_err(|e| format!("session bus: {e}"))?;
    let path = app_object_path(app_id);
    let proxy = zbus::blocking::Proxy::new(
        &session,
        app_id,
        path.as_str(),
        "org.freedesktop.Application",
    )
    .map_err(|e| format!("Application proxy: {e}"))?;
    let platform: std::collections::HashMap<String, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    match proxy.call::<_, _, ()>("Activate", &(platform,)) {
        Ok(()) => Ok(true),
        Err(e) => {
            crate::note(&format!("wayland clients: Activate {app_id}: {e}"));
            Ok(false)
        }
    }
}

fn app_object_path(app_id: &str) -> String {
    format!("/{}", app_id.replace('.', "/").replace('-', "_"))
}

fn wayland_socket_path() -> Option<PathBuf> {
    let display = std::env::var("WAYLAND_DISPLAY")
        .ok()
        .filter(|s| !s.is_empty())?;
    let path = PathBuf::from(&display);
    if path.is_absolute() {
        return Some(path);
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)?;
    Some(runtime.join(display))
}

fn connects_to_wayland(pid: u32, socket: Option<&Path>) -> bool {
    let fds = fs::read_dir(format!("/proc/{pid}/fd")).ok();
    let Some(fds) = fds else {
        return false;
    };
    for fd in fds.flatten() {
        let Ok(link) = fs::read_link(fd.path()) else {
            continue;
        };
        if let Some(socket) = socket {
            if link == socket {
                return true;
            }
            if link.file_name() == socket.file_name() {
                return true;
            }
        }
        if link
            .to_str()
            .is_some_and(|s| s.contains("wayland-") && !s.contains("wayland-socket"))
        {
            return true;
        }
    }
    false
}

fn skip_helper(name: &str, path: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let path = path.to_ascii_lowercase();
    if path.contains("/libexec/") {
        return true;
    }
    SKIP_NAMES.iter().any(|n| name == *n || name.starts_with(n))
        || SKIP_PATH_PARTS.iter().any(|p| path.contains(p))
}

const SKIP_NAMES: &[&str] = &[
    "at-spi-bus-launcher",
    "at-spi2-registryd",
    "dbus-broker",
    "dbus-daemon",
    "gjs",
    "gnome-session",
    "gnome-shell",
    "gsd-",
    "ibus-",
    "mutter",
    "pipewire",
    "sqyre",
    "wireplumber",
    "xwayland",
];

const SKIP_PATH_PARTS: &[&str] = &[
    "xdg-desktop-portal",
    "xdg-document-portal",
    "xdg-permission-store",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_session_helpers() {
        assert!(skip_helper("gnome-shell", "/usr/bin/gnome-shell"));
        assert!(skip_helper("gsd-media-keys", "/usr/libexec/gsd-media-keys"));
        assert!(skip_helper(
            "xdg-desktop-portal-gnome",
            "/usr/libexec/xdg-desktop-portal-gnome"
        ));
        assert!(!skip_helper("firefox", "/usr/lib/firefox/firefox"));
        assert!(!skip_helper("nautilus", "/usr/bin/nautilus"));
    }

    #[test]
    fn app_object_path_from_id() {
        assert_eq!(app_object_path("org.gnome.Nautilus"), "/org/gnome/Nautilus");
        assert_eq!(
            app_object_path("org.mozilla.firefox"),
            "/org/mozilla/firefox"
        );
    }
}
