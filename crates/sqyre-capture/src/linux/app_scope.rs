//! Run inside an `app-<APP_ID>-<pid>.scope` systemd unit.
//!
//! xdg-desktop-portal derives an unsandboxed app id from the systemd unit when
//! the host registry was not used. The unit is `app[-<launcher>]-<id>-<pid>.scope`
//! (COSMIC uses `app-cosmic-com.sqyre.app-<pid>.scope`). A terminal or
//! `session-N.scope` does not match, so the portal id is empty and COSMIC
//! shows "unknown application".

use crate::cap_log;
use std::time::{Duration, Instant};
use zbus::zvariant::Value;

/// Move this process into its own app scope before any portal call.
pub fn enter_app_scope(app_id: &str) {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return;
    }
    let current = current_unit();
    // COSMIC's `app-cosmic-com.sqyre.app-<pid>.scope` already parses as our id.
    if current
        .as_deref()
        .is_some_and(|u| portal_app_id_from_unit(u) == Some(app_id))
    {
        return;
    }
    let pid = std::process::id();
    let unit = format!("app-{app_id}-{pid}.scope");
    if let Err(e) = start_scope(&unit, pid) {
        cap_log("PORTAL", "scope", &format!("fail unit={unit} error={e}"));
        return;
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    while current_unit().as_deref() != Some(unit.as_str()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let now = current_unit().unwrap_or_default();
    cap_log(
        "PORTAL",
        "scope",
        &format!(
            "from={} to={now} ok={}",
            current.unwrap_or_default(),
            now == unit
        ),
    );
}

fn start_scope(unit: &str, pid: u32) -> zbus::Result<()> {
    let conn = zbus::blocking::Connection::session()?;
    let properties: Vec<(&str, Value<'_>)> = vec![
        ("Description", Value::from(sqyre_ports::APP_NAME)),
        ("PIDs", Value::from(vec![pid])),
        ("CollectMode", Value::from("inactive-or-failed")),
    ];
    let aux: Vec<(&str, Vec<(&str, Value<'_>)>)> = Vec::new();
    conn.call_method(
        Some("org.freedesktop.systemd1"),
        "/org/freedesktop/systemd1",
        Some("org.freedesktop.systemd1.Manager"),
        "StartTransientUnit",
        &(unit, "fail", properties, aux),
    )?;
    Ok(())
}

/// Last component of the cgroup v2 path (`…/app.slice/cosmic-launcher.scope`).
fn current_unit() -> Option<String> {
    let cgroup = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    unit_from_cgroup(&cgroup).map(str::to_string)
}

/// App id xdg-desktop-portal parses from a systemd user unit.
///
/// Mirrors `_xdp_app_info_host_parse_app_id_from_unit_name`:
/// `^app-(?:[[:alnum:]]+\-)?(.+?)(?:\-[[:alnum:]]*)(?:\.scope|\.slice)$`.
/// The optional launcher token is one leading alphanumeric run (`cosmic`,
/// `glib`, `gnome`). `None` when the unit is not an app scope.
fn portal_app_id_from_unit(unit: &str) -> Option<&str> {
    let rest = unit.strip_prefix("app-")?;
    let rest = rest
        .strip_suffix(".scope")
        .or_else(|| rest.strip_suffix(".slice"))?;
    let (head, instance) = rest.rsplit_once('-')?;
    if instance.is_empty() || !instance.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let app_id = match head.split_once('-') {
        Some((launcher, app))
            if !launcher.is_empty()
                && launcher.bytes().all(|b| b.is_ascii_alphanumeric())
                && !app.is_empty() =>
        {
            app
        }
        _ => head,
    };
    if app_id.is_empty() {
        None
    } else {
        Some(app_id)
    }
}

fn unit_from_cgroup(cgroup: &str) -> Option<&str> {
    cgroup
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .and_then(|path| path.rsplit('/').next())
        .filter(|u| !u.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_is_last_cgroup_v2_component() {
        let cg =
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/cosmic-launcher.scope\n";
        assert_eq!(unit_from_cgroup(cg), Some("cosmic-launcher.scope"));
        assert_eq!(unit_from_cgroup("1:name=systemd:/x\n"), None);
    }

    #[test]
    fn portal_parses_launcher_prefixed_app_scopes() {
        assert_eq!(
            portal_app_id_from_unit("app-com.sqyre.app-12345.scope"),
            Some("com.sqyre.app")
        );
        assert_eq!(
            portal_app_id_from_unit("app-cosmic-com.sqyre.app-12345.scope"),
            Some("com.sqyre.app")
        );
        assert_eq!(
            portal_app_id_from_unit("app-glib-com.sqyre.app-12345.scope"),
            Some("com.sqyre.app")
        );
        assert_eq!(
            portal_app_id_from_unit("app-gnome-org.gnome.Evolution-2437.scope"),
            Some("org.gnome.Evolution")
        );
        assert_eq!(
            portal_app_id_from_unit("app-cosmic-sqyre-12345.scope"),
            Some("sqyre")
        );
        assert_eq!(
            portal_app_id_from_unit("app-sqyre-12345.slice"),
            Some("sqyre")
        );
        assert_eq!(
            portal_app_id_from_unit("app-cosmic-com.sqyre.app.scope"),
            None
        );
        assert_eq!(portal_app_id_from_unit("session-2.scope"), None);
        assert_eq!(portal_app_id_from_unit("cosmic-launcher.scope"), None);
    }
}
