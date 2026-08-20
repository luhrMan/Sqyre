//! GNOME (and other ATK/AT-SPI) window list + focus.
//!
//! Mutter does not advertise `ext-foreign-toplevel-list` or wlr-foreign-toplevel to
//! normal clients. AT-SPI is the session API that still exposes native Wayland apps.

use super::app_resolve::process_from_pid;
use crate::window_match::{paths_equal, titles_equal};
use crate::{window_matches_process, WindowInfo};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

const A11Y_BUS: &str = "org.a11y.Bus";
/// at-spi-bus-launcher exports this path (lowercase `bus`). D-Bus paths are case-sensitive.
const A11Y_BUS_PATH: &str = "/org/a11y/bus";
const A11Y_BUS_IFACE: &str = "org.a11y.Bus";
const A11Y_STATUS_IFACE: &str = "org.a11y.Status";
const REGISTRY: &str = "org.a11y.atspi.Registry";
const ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
const REGISTRY_PATH: &str = "/org/a11y/atspi/registry";
const REGISTRY_IFACE: &str = "org.a11y.atspi.Registry";
const ACCESSIBLE: &str = "org.a11y.atspi.Accessible";
const COMPONENT: &str = "org.a11y.atspi.Component";
const DBUS: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";

const STATE_ACTIVE: u32 = 1;
const STATE_DEFUNCT: u32 = 6;
const STATE_FOCUSED: u32 = 12;

struct AtspiRef {
    dest: String,
    path: OwnedObjectPath,
}

pub(crate) fn list_windows() -> Result<Vec<WindowInfo>, String> {
    let conn = a11y_connection()?;
    register_event_listener(&conn);
    let root = root_ref()?;
    wait_for_children(&conn, &root);
    let kids = children(&conn, &root)?;
    crate::cap_log(
        "FOCUS",
        if kids.is_empty() { "atspi" } else { "ok" },
        &format!("atspi root_children={}", kids.len()),
    );
    let mut out = Vec::new();
    for app in walk_applications_from(&conn, kids)? {
        out.extend(app);
    }
    Ok(out)
}

pub(crate) fn active_window() -> Result<Option<WindowInfo>, String> {
    let conn = a11y_connection()?;
    let mut focused = None;
    let root = AtspiRef {
        dest: REGISTRY.into(),
        path: ROOT_PATH
            .try_into()
            .map_err(|e: zbus::zvariant::Error| e.to_string())?,
    };
    for app in children(&conn, &root)? {
        if is_application_role(&role_name(&conn, &app).unwrap_or_default()) {
            for frame in children(&conn, &app)? {
                if !is_window_role(&role_name(&conn, &frame).unwrap_or_default()) {
                    continue;
                }
                if has_state(&conn, &frame, STATE_DEFUNCT) {
                    continue;
                }
                if has_state(&conn, &frame, STATE_FOCUSED) || has_state(&conn, &frame, STATE_ACTIVE)
                {
                    focused = window_info(&conn, &app, &frame);
                    break;
                }
            }
        }
        if focused.is_some() {
            break;
        }
    }
    Ok(focused)
}

pub(crate) fn activate(process_path: &str, window_title: &str) -> Result<bool, String> {
    let conn = a11y_connection()?;
    let root = AtspiRef {
        dest: REGISTRY.into(),
        path: ROOT_PATH
            .try_into()
            .map_err(|e: zbus::zvariant::Error| e.to_string())?,
    };
    for app in children(&conn, &root)? {
        if !is_application_role(&role_name(&conn, &app).unwrap_or_default()) {
            continue;
        }
        let pid = unix_pid(&conn, &app).unwrap_or(0);
        let (name, path) = process_from_pid(pid);
        let frames = children(&conn, &app)?;
        let targets: Vec<AtspiRef> = if frames.is_empty() {
            vec![app]
        } else {
            frames
                .into_iter()
                .filter(|f| {
                    is_window_role(&role_name(&conn, f).unwrap_or_default())
                        && !has_state(&conn, f, STATE_DEFUNCT)
                })
                .collect()
        };
        for frame in targets {
            let title = name_of(&conn, &frame).unwrap_or_default();
            let info = WindowInfo {
                title,
                process_name: name.clone(),
                process_path: path.clone(),
                icon: None,
            };
            if !titles_equal(&info.title, window_title) {
                continue;
            }
            if !path.is_empty()
                && !paths_equal(&path, process_path)
                && !window_matches_process(&info, process_path)
            {
                continue;
            }
            if grab_focus(&conn, &frame).unwrap_or(false) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn root_ref() -> Result<AtspiRef, String> {
    Ok(AtspiRef {
        dest: REGISTRY.into(),
        path: ROOT_PATH
            .try_into()
            .map_err(|e: zbus::zvariant::Error| e.to_string())?,
    })
}

fn walk_applications_from(
    conn: &Connection,
    apps: Vec<AtspiRef>,
) -> Result<Vec<Vec<WindowInfo>>, String> {
    let mut groups = Vec::new();
    for app in apps {
        if !is_application_role(&role_name(conn, &app).unwrap_or_default()) {
            continue;
        }
        if has_state(conn, &app, STATE_DEFUNCT) {
            continue;
        }
        let pid = unix_pid(conn, &app).unwrap_or(0);
        let (process_name, process_path) = process_from_pid(pid);
        let mut windows = Vec::new();
        let frames = children(conn, &app)?;
        for frame in &frames {
            if !is_window_role(&role_name(conn, frame).unwrap_or_default()) {
                continue;
            }
            if has_state(conn, frame, STATE_DEFUNCT) {
                continue;
            }
            if let Some(info) =
                window_info_from(process_name.clone(), process_path.clone(), conn, frame)
            {
                windows.push(info);
            }
        }
        if windows.is_empty() {
            if let Some(info) = window_info_from(process_name, process_path, conn, &app) {
                windows.push(info);
            }
        }
        if !windows.is_empty() {
            groups.push(windows);
        }
    }
    Ok(groups)
}

fn window_info(conn: &Connection, app: &AtspiRef, frame: &AtspiRef) -> Option<WindowInfo> {
    let pid = unix_pid(conn, app).unwrap_or(0);
    let (process_name, process_path) = process_from_pid(pid);
    window_info_from(process_name, process_path, conn, frame)
}

fn window_info_from(
    process_name: String,
    process_path: String,
    conn: &Connection,
    node: &AtspiRef,
) -> Option<WindowInfo> {
    let title = name_of(conn, node).unwrap_or_default();
    if title.trim().is_empty() {
        return None;
    }
    Some(WindowInfo {
        title,
        process_name,
        process_path,
        icon: None,
    })
}

fn is_window_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "frame" | "window" | "terminal" | "dialog" | "file chooser" | "alert"
    )
}

fn is_application_role(role: &str) -> bool {
    role.is_empty() || role.eq_ignore_ascii_case("application")
}

fn a11y_connection() -> Result<Connection, String> {
    let session = Connection::session().map_err(|e| format!("session bus: {e}"))?;
    enable_toolkit_a11y(&session);
    let proxy = Proxy::new(&session, A11Y_BUS, A11Y_BUS_PATH, A11Y_BUS_IFACE)
        .map_err(|e| format!("a11y bus proxy: {e}"))?;
    let address: String = proxy
        .call("GetAddress", &())
        .map_err(|e| format!("GetAddress: {e}"))?;
    zbus::blocking::connection::Builder::address(address.as_str())
        .map_err(|e| format!("a11y address: {e}"))?
        .build()
        .map_err(|e| format!("a11y connect: {e}"))
}

/// GTK/Qt only export AT-SPI when this is true. Already-running apps may still need a restart.
fn enable_toolkit_a11y(session: &Connection) {
    let Ok(status) = Proxy::new(session, A11Y_BUS, A11Y_BUS_PATH, A11Y_STATUS_IFACE) else {
        return;
    };
    let enabled = status.get_property::<bool>("IsEnabled").unwrap_or(false);
    if enabled {
        return;
    }
    if let Err(e) = status.set_property("IsEnabled", true) {
        crate::note(&format!("atspi: enable IsEnabled: {e}"));
    }
}

/// Recent at-spi-bus-launcher treats event registration as "an AT is present".
fn register_event_listener(conn: &Connection) {
    let Ok(proxy) = Proxy::new(conn, REGISTRY, REGISTRY_PATH, REGISTRY_IFACE) else {
        return;
    };
    for ev in [
        "window:activate",
        "window:create",
        "object:children-changed",
    ] {
        if let Err(e) = proxy.call::<_, _, ()>("RegisterEvent", &ev) {
            crate::note(&format!("atspi: RegisterEvent {ev}: {e}"));
        }
    }
}

fn wait_for_children(conn: &Connection, root: &AtspiRef) {
    for i in 0..6 {
        match children(conn, root) {
            Ok(kids) if !kids.is_empty() => return,
            Ok(_) if i < 5 => std::thread::sleep(std::time::Duration::from_millis(50)),
            _ => return,
        }
    }
}

fn children(conn: &Connection, node: &AtspiRef) -> Result<Vec<AtspiRef>, String> {
    let proxy = accessible(conn, node)?;
    let kids: Vec<(String, OwnedObjectPath)> = proxy
        .call("GetChildren", &())
        .map_err(|e| format!("GetChildren: {e}"))?;
    Ok(kids
        .into_iter()
        .map(|(dest, path)| child_ref(node, dest, path))
        .collect())
}

/// Empty bus name in `(so)` means the child lives on the same unique name as `parent`.
fn child_ref(parent: &AtspiRef, dest: String, path: OwnedObjectPath) -> AtspiRef {
    AtspiRef {
        dest: if dest.is_empty() {
            parent.dest.clone()
        } else {
            dest
        },
        path,
    }
}

fn name_of(conn: &Connection, node: &AtspiRef) -> Result<String, String> {
    let proxy = accessible(conn, node)?;
    proxy
        .call("GetName", &())
        .map_err(|e| format!("GetName: {e}"))
}

fn role_name(conn: &Connection, node: &AtspiRef) -> Result<String, String> {
    let proxy = accessible(conn, node)?;
    proxy
        .call("GetRoleName", &())
        .map_err(|e| format!("GetRoleName: {e}"))
}

fn has_state(conn: &Connection, node: &AtspiRef, bit: u32) -> bool {
    let Ok(proxy) = accessible(conn, node) else {
        return false;
    };
    let Ok(states) = proxy.call::<&str, (), Vec<u32>>("GetState", &()) else {
        return false;
    };
    state_bit(&states, bit)
}

fn state_bit(states: &[u32], bit: u32) -> bool {
    let idx = (bit / 32) as usize;
    let mask = 1u32 << (bit % 32);
    states.get(idx).copied().unwrap_or(0) & mask != 0
}

fn unix_pid(conn: &Connection, node: &AtspiRef) -> Option<u32> {
    let proxy = Proxy::new(conn, DBUS, DBUS_PATH, DBUS).ok()?;
    proxy.call("GetConnectionUnixProcessID", &node.dest).ok()
}

fn grab_focus(conn: &Connection, node: &AtspiRef) -> Result<bool, String> {
    let proxy = Proxy::new(conn, node.dest.as_str(), node.path.as_str(), COMPONENT)
        .map_err(|e| format!("Component proxy: {e}"))?;
    proxy
        .call("GrabFocus", &())
        .map_err(|e| format!("GrabFocus: {e}"))
}

fn accessible<'a>(conn: &'a Connection, node: &'a AtspiRef) -> Result<Proxy<'a>, String> {
    Proxy::new(conn, node.dest.as_str(), node.path.as_str(), ACCESSIBLE)
        .map_err(|e| format!("Accessible proxy: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_bit_focused() {
        let mut states = vec![0u32, 0];
        states[0] |= 1 << STATE_FOCUSED;
        assert!(state_bit(&states, STATE_FOCUSED));
        assert!(!state_bit(&states, STATE_DEFUNCT));
    }

    #[test]
    fn a11y_bus_path_matches_at_spi_launcher() {
        assert_eq!(A11Y_BUS_PATH, "/org/a11y/bus");
    }

    #[test]
    fn empty_child_dest_inherits_parent() {
        let parent = AtspiRef {
            dest: ":1.42".into(),
            path: ROOT_PATH.try_into().unwrap(),
        };
        let path: OwnedObjectPath = "/org/a11y/atspi/accessible/1".try_into().unwrap();
        let child = child_ref(&parent, String::new(), path.clone());
        assert_eq!(child.dest, ":1.42");
        let other = child_ref(&parent, ":1.99".into(), path);
        assert_eq!(other.dest, ":1.99");
    }
}
