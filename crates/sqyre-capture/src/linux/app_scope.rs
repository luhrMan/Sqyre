//! Run inside an `app-<APP_ID>-<pid>.scope` systemd unit.
//!
//! xdg-desktop-portal < 1.19 has no host app registry; for unsandboxed apps it
//! derives the app id from the systemd scope name. Launchers (e.g.
//! `cosmic-launcher.scope`) or terminals leave Sqyre in their own scope, so
//! portal dialogs say "Unknown Application" and permissions are not persisted
//! per app.

use crate::cap_log;
use std::time::{Duration, Instant};
use zbus::zvariant::Value;

/// Move this process into its own app scope before any portal call.
pub fn enter_app_scope(app_id: &str) {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return;
    }
    let prefix = format!("app-{app_id}-");
    let current = current_unit();
    if current.as_deref().is_some_and(|u| u.starts_with(&prefix)) {
        return;
    }
    let pid = std::process::id();
    let unit = format!("{prefix}{pid}.scope");
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
}
