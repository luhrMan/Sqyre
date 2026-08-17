//! Linux session / desktop environment detection for backend selection and probes.

use crate::event_log;

/// High-level session protocol (from env vars; no live compositor query).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxSessionKind {
    X11,
    Wayland,
    Unknown,
}

impl LinuxSessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
            Self::Unknown => "unknown",
        }
    }
}

/// Which capture backend Sqyre should prefer on this session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxCaptureBackend {
    /// Native X11 (`DISPLAY` without Wayland-only constraint).
    X11Native,
    /// Hybrid: Wayland session with XWayland (`DISPLAY` + `WAYLAND_DISPLAY`).
    XWayland,
    /// Pure Wayland — needs portal ScreenCast + PipeWire (not implemented yet).
    WaylandPortal,
    /// No usable backend (pure Wayland, no XWayland).
    Unavailable,
}

impl LinuxCaptureBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X11Native => "x11",
            Self::XWayland => "xwayland",
            Self::WaylandPortal => "portal+pipewire",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Snapshot of the current Linux graphical session.
#[derive(Debug, Clone)]
pub struct LinuxSessionInfo {
    pub session_kind: LinuxSessionKind,
    pub desktop: Option<String>,
    pub compositor: Option<String>,
    pub display: Option<String>,
    pub wayland_display: Option<String>,
    pub has_x11: bool,
    pub has_wayland: bool,
}

impl LinuxSessionInfo {
    /// Detect session from environment (safe in CI / headless).
    pub fn detect() -> Self {
        let session_raw = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let display = std::env::var("DISPLAY").ok().filter(|s| !s.is_empty());
        let wayland_display = std::env::var("WAYLAND_DISPLAY")
            .ok()
            .filter(|s| !s.is_empty());
        let has_x11 = display.is_some();
        let has_wayland = session_raw.eq_ignore_ascii_case("wayland") || wayland_display.is_some();

        let session_kind = if session_raw.eq_ignore_ascii_case("x11") || (has_x11 && !has_wayland) {
            LinuxSessionKind::X11
        } else if has_wayland {
            LinuxSessionKind::Wayland
        } else if has_x11 {
            LinuxSessionKind::X11
        } else {
            LinuxSessionKind::Unknown
        };

        let desktop = detect_desktop_name();
        let compositor = detect_compositor_hint(&desktop);

        Self {
            session_kind,
            desktop,
            compositor,
            display,
            wayland_display,
            has_x11,
            has_wayland,
        }
    }

    /// Preferred capture backend for this session.
    pub fn capture_backend(&self) -> LinuxCaptureBackend {
        // Wayland compositors (including XWayland hybrids) cannot serve desktop pixels
        // via XGetImage on the root window — use portal ScreenCast when available.
        #[cfg(feature = "portal-capture")]
        if self.session_kind == LinuxSessionKind::Wayland && self.has_wayland {
            return LinuxCaptureBackend::WaylandPortal;
        }

        match (self.has_x11, self.has_wayland, self.session_kind) {
            (true, false, _) => LinuxCaptureBackend::X11Native,
            (true, true, _) => LinuxCaptureBackend::XWayland,
            (false, true, LinuxSessionKind::Wayland) => LinuxCaptureBackend::WaylandPortal,
            (false, true, _) => LinuxCaptureBackend::WaylandPortal,
            _ => LinuxCaptureBackend::Unavailable,
        }
    }

    /// True when portal / native Wayland capture backends are required.
    pub fn needs_wayland_backend(&self) -> bool {
        matches!(
            self.capture_backend(),
            LinuxCaptureBackend::WaylandPortal | LinuxCaptureBackend::Unavailable
        )
    }

    /// Human-readable capture warning (replaces simpler check in `error.rs`).
    pub fn capture_warning(&self) -> Option<String> {
        match self.capture_backend() {
            LinuxCaptureBackend::Unavailable => Some(format!(
                "Pure Wayland session detected (no DISPLAY). Sqyre needs X11, XWayland, or a \
                 Wayland portal capture backend (backend={}).",
                LinuxCaptureBackend::WaylandPortal.as_str()
            )),
            LinuxCaptureBackend::WaylandPortal if !self.has_x11 => Some(format!(
                "Wayland session without XWayland. Screen capture requires portal ScreenCast \
                 (backend={}).",
                LinuxCaptureBackend::WaylandPortal.as_str()
            )),
            LinuxCaptureBackend::XWayland if self.session_kind == LinuxSessionKind::Wayland => {
                Some(
                    "Wayland + XWayland: root XGetImage capture fails on GNOME/KDE (BadMatch). \
                     Rebuild with the portal-capture feature for ScreenCast + PipeWire."
                        .into(),
                )
            }
            _ => None,
        }
    }

    /// Emit a stable log line for agents (`SQYRE_SESSION=…`).
    pub fn log_session(&self) {
        event_log(
            "SQYRE_SESSION",
            &[
                ("type", self.session_kind.as_str()),
                ("desktop", self.desktop.as_deref().unwrap_or("unknown")),
                (
                    "compositor",
                    self.compositor.as_deref().unwrap_or("unknown"),
                ),
                ("backend", self.capture_backend().as_str()),
                ("display", if self.has_x11 { "yes" } else { "no" }),
                ("wayland", if self.has_wayland { "yes" } else { "no" }),
            ],
        );
    }
}

fn detect_desktop_name() -> Option<String> {
    for key in [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ] {
        if let Ok(v) = std::env::var(key) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn detect_compositor_hint(desktop: &Option<String>) -> Option<String> {
    if let Ok(v) = std::env::var("XDG_SESSION_DESKTOP") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    desktop.as_ref().map(|d| {
        let lower = d.to_lowercase();
        if lower.contains("gnome") {
            "mutter".into()
        } else if lower.contains("kde") || lower.contains("plasma") {
            "kwin".into()
        } else if lower.contains("cosmic") {
            "cosmic-comp".into()
        } else {
            d.clone()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_backend_x11_native() {
        let info = LinuxSessionInfo {
            session_kind: LinuxSessionKind::X11,
            desktop: Some("GNOME".into()),
            compositor: Some("mutter".into()),
            display: Some(":0".into()),
            wayland_display: None,
            has_x11: true,
            has_wayland: false,
        };
        assert_eq!(info.capture_backend(), LinuxCaptureBackend::X11Native);
        assert!(info.capture_warning().is_none());
    }

    #[test]
    fn capture_backend_pure_wayland() {
        let info = LinuxSessionInfo {
            session_kind: LinuxSessionKind::Wayland,
            desktop: Some("KDE".into()),
            compositor: Some("kwin".into()),
            display: None,
            wayland_display: Some("wayland-0".into()),
            has_x11: false,
            has_wayland: true,
        };
        assert_eq!(info.capture_backend(), LinuxCaptureBackend::WaylandPortal);
        assert!(info.needs_wayland_backend());
        assert!(info.capture_warning().is_some());
    }

    #[test]
    fn capture_backend_xwayland() {
        let info = LinuxSessionInfo {
            session_kind: LinuxSessionKind::Wayland,
            desktop: Some("GNOME".into()),
            compositor: Some("mutter".into()),
            display: Some(":0".into()),
            wayland_display: Some("wayland-0".into()),
            has_x11: true,
            has_wayland: true,
        };
        #[cfg(feature = "portal-capture")]
        assert_eq!(info.capture_backend(), LinuxCaptureBackend::WaylandPortal);
        #[cfg(not(feature = "portal-capture"))]
        {
            assert_eq!(info.capture_backend(), LinuxCaptureBackend::XWayland);
            assert!(info.capture_warning().is_some());
        }
        #[cfg(feature = "portal-capture")]
        assert!(info.capture_warning().is_none());
    }
}
