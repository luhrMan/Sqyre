//! Permission hints derived from probe results.

use crate::{CapStatus, CapabilityResult, SessionReport};
use std::collections::BTreeMap;

pub fn probe_permissions(_session: &SessionReport, caps: &mut BTreeMap<String, CapabilityResult>) {
    #[cfg(target_os = "linux")]
    {
        let in_input = in_input_group();
        caps.insert(
            "permissions.input_group".into(),
            if in_input {
                CapabilityResult {
                    status: CapStatus::Ok,
                    ..CapabilityResult::default()
                }
            } else {
                CapabilityResult {
                    status: CapStatus::Fail,
                    error: Some("user not in 'input' group".into()),
                    ..CapabilityResult::default()
                }
            },
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = session;
        caps.insert(
            "permissions.input_group".into(),
            CapabilityResult {
                status: CapStatus::Skip,
                reason: Some("Linux-only".into()),
                ..CapabilityResult::default()
            },
        );
    }
}

pub fn collect_hints(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> Vec<String> {
    let mut hints = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if session.session_type == "wayland" && session.display.is_none() {
            hints.push(
                "Enable XWayland or wait for Sqyre Wayland portal capture (ScreenCast + PipeWire)."
                    .into(),
            );
            hints.push(screen_recording_hint(session.desktop.as_deref()));
        }

        if session.session_type == "wayland"
            && session.display.is_some()
            && session.capture_backend.as_deref() == Some("xwayland")
        {
            hints.push(
                "GNOME/KDE Wayland + XWayland: rebuild sqyre-probe with portal-capture \
                 (make probe) — XGetImage root capture returns BadMatch."
                    .into(),
            );
            hints.push(screen_recording_hint(session.desktop.as_deref()));
        }

        if matches!(
            caps.get("permissions.input_group").map(|c| &c.status),
            Some(CapStatus::Fail)
        ) {
            hints.push(
                "Add user to 'input' group for synthetic input on Wayland: sudo usermod -aG input $USER (re-login)."
                    .into(),
            );
        }

        if matches!(
            caps.get("portal.screencast").map(|c| &c.status),
            Some(CapStatus::Fail)
        ) {
            hints.push(screen_recording_hint(session.desktop.as_deref()));
        }

        if matches!(
            caps.get("hotkeys.start").map(|c| &c.status),
            Some(CapStatus::Fail)
        ) && session.session_type == "wayland"
        {
            hints.push(
                "Grant Global Shortcuts via xdg-desktop-portal (Settings → Keyboard → Shortcuts, DE-specific)."
                    .into(),
            );
        }
    }

    if matches!(
        caps.get("capture.open").map(|c| &c.status),
        Some(CapStatus::Fail)
    ) && session.display.is_none()
    {
        hints.push(
            "No DISPLAY — start an X11 or XWayland session, or use a host with a graphical login."
                .into(),
        );
    }

    hints.sort();
    hints.dedup();
    hints
}

#[cfg(target_os = "linux")]
pub(crate) fn screen_recording_hint(desktop: Option<&str>) -> String {
    let d = desktop.unwrap_or("your desktop").to_lowercase();
    if d.contains("gnome") {
        "Grant Screen Recording: Settings → Privacy → Screen Recording → enable Sqyre.".into()
    } else if d.contains("kde") || d.contains("plasma") {
        "Grant Screen Recording: System Settings → Privacy & Security → Screen Recording.".into()
    } else if d.contains("cosmic") {
        "Grant Screen Recording in COSMIC Settings → Privacy → Screen Capture.".into()
    } else {
        "Grant screen recording / screencast permission when the portal dialog appears.".into()
    }
}

#[cfg(target_os = "linux")]
fn in_input_group() -> bool {
    let Ok(out) = std::process::Command::new("id").arg("-Gn").output() else {
        return false;
    };
    let groups = String::from_utf8_lossy(&out.stdout);
    groups.split_whitespace().any(|g| g == "input")
}

/// Whether the current user belongs to the `input` group (Wayland evdev access).
#[cfg(target_os = "linux")]
pub fn user_in_input_group() -> bool {
    in_input_group()
}

#[cfg(not(target_os = "linux"))]
pub fn user_in_input_group() -> bool {
    true
}

/// Fedora Atomic / Bazzite and other ostree-based desktops keep group membership in `/etc/group`.
#[cfg(target_os = "linux")]
pub fn is_immutable_linux() -> bool {
    if std::path::Path::new("/run/ostree-booted").exists() {
        return true;
    }
    let Ok(os_release) = std::fs::read_to_string("/etc/os-release") else {
        return false;
    };
    os_release.lines().any(|line| {
        line.starts_with("VARIANT_ID=")
            && (line.contains("silverblue")
                || line.contains("kinoite")
                || line.contains("bazzite")
                || line.contains("aurora")
                || line.contains("bluefin")
                || line.contains("mainframe"))
    })
}

#[cfg(not(target_os = "linux"))]
pub fn is_immutable_linux() -> bool {
    false
}

/// Setup steps for adding a user to a group on immutable Fedora systems.
#[cfg(target_os = "linux")]
pub fn atomic_group_setup_steps(group: &str) -> Vec<String> {
    let mut steps = vec![
        format!(
            "On Bazzite and other Fedora Atomic systems, copy the group into /etc/group first \
             (plain usermod alone may not survive reboot):"
        ),
        format!("grep '^{group}:' /usr/lib/group | sudo tee -a /etc/group"),
        format!("sudo usermod -aG {group} $USER"),
        format!(
            "Verify /etc/group contains `{group}:x:<gid>:$USER` before rebooting, then log out fully."
        ),
    ];
    if group == "input" {
        steps.insert(2, "Or on Bazzite: ujust add-user-to-input-group add".into());
    }
    steps
}

#[cfg(not(target_os = "linux"))]
pub fn atomic_group_setup_steps(_group: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "linux")]
pub fn atomic_group_tooltip(group: &str) -> String {
    atomic_group_setup_steps(group).join("\n")
}

#[cfg(not(target_os = "linux"))]
pub fn atomic_group_tooltip(_group: &str) -> String {
    String::new()
}
