//! XDG Desktop Portal probes (ScreenCast, GlobalShortcuts).

use crate::{CapStatus, CapabilityResult, SessionReport};
use sqyre_capture::cap_log;
use std::collections::BTreeMap;

/// Best-effort portal version string from `busctl` (no hard dependency on portal running).
pub fn portal_version() -> Option<String> {
    let out = std::process::Command::new("busctl")
        .args([
            "--user",
            "get-property",
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Introspectable",
            "version",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let ver = text.trim().trim_matches('"').trim();
    if ver.is_empty() {
        None
    } else {
        Some(ver.to_string())
    }
}

pub fn probe_portal(session: &SessionReport, caps: &mut BTreeMap<String, CapabilityResult>) {
    caps.insert(
        "portal.version".into(),
        match session.portal_version.clone() {
            Some(v) => CapabilityResult {
                status: CapStatus::Ok,
                size: Some(v),
                ..CapabilityResult::default()
            },
            None => CapabilityResult::skip("portal version unavailable (busctl)"),
        },
    );

    caps.insert("portal.screencast".into(), probe_screencast(session, caps));
    caps.insert(
        "portal.global_shortcuts".into(),
        probe_global_shortcuts(session),
    );
    caps.insert(
        "capture.wayland_portal".into(),
        match sqyre_capture::linux::wayland::portal_capture_available() {
            Ok(()) => CapabilityResult {
                status: CapStatus::Ok,
                ..CapabilityResult::default()
            },
            Err(e) => CapabilityResult::pending(e.to_string()),
        },
    );
}

fn probe_screencast(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> CapabilityResult {
    if session.session_type != "wayland" {
        return CapabilityResult::skip("X11/XWayland session — portal screencast optional");
    }

    // capture.open already negotiated ScreenCast + PipeWire — do not open a second session.
    if screencast_verified_via_capture(caps) {
        cap_log("PORTAL", "ok", "interface=ScreenCast via=capture.open");
        return CapabilityResult {
            status: CapStatus::Ok,
            backend: Some("portal".into()),
            ..CapabilityResult::default()
        };
    }

    use ashpd::desktop::screencast::Screencast;

    let result = pollster::block_on(async {
        let proxy = Screencast::new().await?;
        let _session = proxy.create_session().await?;
        Ok::<(), ashpd::Error>(())
    });

    match result {
        Ok(()) => {
            cap_log("PORTAL", "ok", "interface=ScreenCast");
            CapabilityResult {
                status: CapStatus::Ok,
                backend: Some("portal".into()),
                ..CapabilityResult::default()
            }
        }
        Err(ashpd::Error::Portal(_)) | Err(ashpd::Error::Response(_)) => {
            cap_log("PORTAL", "denied", "interface=ScreenCast");
            CapabilityResult {
                status: CapStatus::Fail,
                error: Some("portal ScreenCast denied or unavailable".into()),
                ..CapabilityResult::default()
            }
        }
        Err(e) => CapabilityResult {
            status: CapStatus::Fail,
            error: Some(e.to_string()),
            ..CapabilityResult::default()
        },
    }
}

/// True when [`capture.open`] already proved portal ScreenCast on this run.
pub fn screencast_verified_via_capture(caps: &BTreeMap<String, CapabilityResult>) -> bool {
    matches!(
        caps.get("capture.open")
            .map(|c| (&c.status, c.backend.as_deref())),
        Some((CapStatus::Ok, Some("portal+pipewire")))
    )
}

fn probe_global_shortcuts(session: &SessionReport) -> CapabilityResult {
    if session.session_type != "wayland" {
        return CapabilityResult::skip("X11 session — rdev hotkeys preferred");
    }

    use ashpd::desktop::global_shortcuts::GlobalShortcuts;

    let result = pollster::block_on(async { GlobalShortcuts::new().await });

    match result {
        Ok(_proxy) => {
            cap_log("PORTAL", "ok", "interface=GlobalShortcuts");
            CapabilityResult {
                status: CapStatus::Ok,
                backend: Some("portal".into()),
                ..CapabilityResult::default()
            }
        }
        Err(e) => CapabilityResult {
            status: CapStatus::Fail,
            error: Some(format!("GlobalShortcuts: {e}")),
            ..CapabilityResult::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screencast_dedupe_when_capture_open_ok() {
        let mut caps = BTreeMap::new();
        caps.insert(
            "capture.open".into(),
            CapabilityResult {
                status: CapStatus::Ok,
                backend: Some("portal+pipewire".into()),
                ..CapabilityResult::default()
            },
        );
        assert!(screencast_verified_via_capture(&caps));
    }

    #[test]
    fn screencast_no_dedupe_when_xwayland_capture() {
        let mut caps = BTreeMap::new();
        caps.insert(
            "capture.open".into(),
            CapabilityResult {
                status: CapStatus::Ok,
                backend: Some("xwayland".into()),
                ..CapabilityResult::default()
            },
        );
        assert!(!screencast_verified_via_capture(&caps));
    }
}
