//! Portal session tokens helpers.

use super::*;

pub(super) fn restore_tokens_on_disk() -> Vec<String> {
    [restore_token_path(), legacy_screencast_token_path()]
        .iter()
        .filter_map(|p| read_restore_token_at(p))
        .collect()
}

pub(super) fn clear_restore_tokens() {
    write_restore_token_at(&restore_token_path(), None);
    write_restore_token_at(&legacy_screencast_token_path(), None);
}

/// Best-effort: drop persist entries keyed by restore token and `com.sqyre.app`.
pub(super) fn wipe_portal_permission_store(tokens: &[String]) {
    let Ok(conn) = zbus::blocking::Connection::session() else {
        cap_log("PORTAL", "revoke", "permission-store: no session bus");
        return;
    };
    let Ok(proxy) = zbus::blocking::Proxy::new(
        &conn,
        "org.freedesktop.impl.portal.PermissionStore",
        "/org/freedesktop/impl/portal/PermissionStore",
        "org.freedesktop.impl.portal.PermissionStore",
    ) else {
        cap_log("PORTAL", "revoke", "permission-store: proxy failed");
        return;
    };
    const TABLES: &[&str] = &["remote-desktop", "screencast"];
    for table in TABLES {
        for token in tokens {
            // Not-found is the usual case after a local token clear.
            let _ = proxy.call::<_, _, ()>("Delete", &(table, token.as_str()));
        }
        let Ok(ids) = proxy.call::<_, _, Vec<String>>("List", &(table,)) else {
            continue;
        };
        for id in ids {
            let _ =
                proxy.call::<_, _, ()>("DeletePermission", &(table, id.as_str(), PORTAL_APP_ID));
        }
    }
}

pub(super) fn restore_token_path() -> std::path::PathBuf {
    dirs_home().join(APP_DIR_NAME).join("wayland-remote.token")
}

pub(super) fn legacy_screencast_token_path() -> std::path::PathBuf {
    dirs_home()
        .join(APP_DIR_NAME)
        .join("wayland-screencast.token")
}

pub(super) fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub(super) fn read_restore_token_at(path: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let token = raw.trim();
    if token.is_empty() || token.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    Some(token.to_string())
}

pub(super) fn write_restore_token_at(path: &std::path::Path, token: Option<&str>) {
    match token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(token) => {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(path, token) {
                cap_log("PORTAL", "token", &format!("save failed: {e}"));
            } else {
                cap_log("PORTAL", "token", "saved");
            }
        }
        None => {
            if path.exists() {
                let _ = std::fs::remove_file(path);
                cap_log("PORTAL", "token", "cleared");
            }
        }
    }
}
