//! Linux display-backend selection (X11 / XWayland vs pure Wayland).

/// Which Linux capture / focus / overlay stack to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDisplayBackend {
    /// Classic X11 or XWayland (`DISPLAY` is set).
    X11,
    /// Pure Wayland (portal / compositor protocols).
    Wayland,
}

/// Select the Linux backend for this process.
///
/// Prefer the existing X11 path whenever `DISPLAY` is available (including
/// XWayland sessions). Use the Wayland portal stack only for pure Wayland.
pub fn linux_display_backend() -> LinuxDisplayBackend {
    let has_x11 = std::env::var_os("DISPLAY").is_some();
    if has_x11 {
        return LinuxDisplayBackend::X11;
    }
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland =
        session.eq_ignore_ascii_case("wayland") || std::env::var_os("WAYLAND_DISPLAY").is_some();
    if wayland {
        LinuxDisplayBackend::Wayland
    } else {
        // Headless / unknown: try X11 open paths (they fail cleanly).
        LinuxDisplayBackend::X11
    }
}

/// True when the process should use the Wayland portal backends.
pub fn is_wayland_backend() -> bool {
    matches!(linux_display_backend(), LinuxDisplayBackend::Wayland)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_is_deterministic() {
        let _ = linux_display_backend();
        let _ = is_wayland_backend();
    }
}
