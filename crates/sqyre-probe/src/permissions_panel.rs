//! User-facing permission rows for in-app settings (derived from probe results).

use crate::{CapStatus, CapabilityResult, SessionReport};
use std::collections::BTreeMap;

/// Whether a permission is satisfied on this session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionEligibility {
    Granted,
    Needed,
    Checking,
    NotRequired,
    Unavailable,
}

impl PermissionEligibility {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Granted => "Granted",
            Self::Needed => "Needed",
            Self::Checking => "Checking…",
            Self::NotRequired => "Not required",
            Self::Unavailable => "Unavailable",
        }
    }
}

/// One row in the User Settings → Permissions panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionItem {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub eligibility: PermissionEligibility,
    pub detail: Option<String>,
    pub setup_steps: Vec<String>,
    /// Shell command the user can copy (e.g. `sudo usermod …`).
    pub copy_command: Option<String>,
    /// Hover text for the permission row (setup detail beyond inline bullets).
    pub tooltip: Option<String>,
}

/// Build all permission rows for the current probe report.
pub fn build_permission_items(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> Vec<PermissionItem> {
    vec![
        screen_recording_item(session, caps),
        input_device_group_item(session, caps),
        global_shortcuts_item(session, caps),
        automation_input_item(session, caps),
        global_hotkeys_item(session, caps),
    ]
}

fn cap_ok(caps: &BTreeMap<String, CapabilityResult>, key: &str) -> bool {
    matches!(caps.get(key).map(|c| &c.status), Some(CapStatus::Ok))
}

fn cap_pending(caps: &BTreeMap<String, CapabilityResult>, key: &str) -> bool {
    matches!(caps.get(key).map(|c| &c.status), Some(CapStatus::Pending))
}

fn live_capture_granted() -> bool {
    sqyre_capture::portal_screencast_granted()
}

fn cap_fail_detail(caps: &BTreeMap<String, CapabilityResult>, key: &str) -> Option<String> {
    caps.get(key)
        .and_then(|c| c.error.clone().or_else(|| c.reason.clone()))
}

fn screen_recording_item(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> PermissionItem {
    let mut setup_steps = Vec::new();
    let (eligibility, detail) = if session.session_type != "wayland" {
        if cap_ok(caps, "capture.open") {
            (PermissionEligibility::Granted, None)
        } else {
            (
                PermissionEligibility::Needed,
                cap_fail_detail(caps, "capture.open"),
            )
        }
    } else if live_capture_granted()
        || cap_ok(caps, "capture.open")
        || cap_ok(caps, "portal.screencast")
    {
        (PermissionEligibility::Granted, None)
    } else if cap_pending(caps, "capture.open")
        || cap_pending(caps, "portal.screencast")
        || sqyre_capture::shared_capturer_is_opening()
    {
        (
            PermissionEligibility::Checking,
            Some("Waiting for the screen sharing dialog.".into()),
        )
    } else {
        setup_steps.push(screen_recording_hint(session.desktop.as_deref()));
        setup_steps.push(
            "When Sqyre asks, choose the screen or window to share in the portal picker.".into(),
        );
        (
            PermissionEligibility::Needed,
            cap_fail_detail(caps, "portal.screencast")
                .or_else(|| cap_fail_detail(caps, "capture.open")),
        )
    };

    PermissionItem {
        id: "screen_recording",
        title: "Screen recording",
        summary: "Capture the desktop for image search, OCR, and previews.",
        eligibility,
        detail,
        setup_steps,
        copy_command: None,
        tooltip: None,
    }
}

fn input_device_group_item(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> PermissionItem {
    let mut setup_steps = Vec::new();
    let copy_command;
    let mut tooltip = None;

    let (eligibility, detail) = if session.session_type != "wayland" {
        copy_command = None;
        (
            PermissionEligibility::NotRequired,
            Some("X11 sessions use display hooks; no /dev/input access is required.".into()),
        )
    } else if cap_ok(caps, "permissions.evdev_access") {
        copy_command = None;
        (PermissionEligibility::Granted, None)
    } else {
        setup_steps.push("If /dev/input is not readable, add your user to the input group, then log out and back in.".into());
        setup_steps
            .push("On some distros the group is named plugdev — add both if they exist.".into());
        #[cfg(target_os = "linux")]
        if crate::permissions::is_immutable_linux() {
            setup_steps.extend(crate::permissions::atomic_group_setup_steps("input"));
            tooltip = Some(crate::permissions::atomic_group_tooltip("input"));
        }
        copy_command = Some("sudo usermod -aG input $USER".into());
        (
            PermissionEligibility::Needed,
            cap_fail_detail(caps, "permissions.evdev_access"),
        )
    };

    PermissionItem {
        id: "input_device_group",
        title: "Input device access",
        summary: "Record global mouse clicks and keys on Wayland (evdev).",
        eligibility,
        detail,
        setup_steps,
        copy_command,
        tooltip,
    }
}

fn global_shortcuts_item(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> PermissionItem {
    let mut setup_steps = Vec::new();
    let (eligibility, detail) = if session.session_type != "wayland" {
        (
            PermissionEligibility::NotRequired,
            Some("X11 sessions register global hotkeys without a portal.".into()),
        )
    } else if cap_ok(caps, "portal.global_shortcuts") {
        (PermissionEligibility::Granted, None)
    } else {
        setup_steps.push(
            "Grant Global Shortcuts when the portal dialog appears, or enable them in your desktop settings.".into(),
        );
        setup_steps.push(
            "GNOME: Settings → Keyboard → Keyboard Shortcuts. KDE: System Settings → Shortcuts."
                .into(),
        );
        (
            PermissionEligibility::Needed,
            cap_fail_detail(caps, "portal.global_shortcuts"),
        )
    };

    PermissionItem {
        id: "global_shortcuts",
        title: "Global shortcuts",
        summary: "Run macros from system-wide hotkey chords on Wayland.",
        eligibility,
        detail,
        setup_steps,
        copy_command: None,
        tooltip: None,
    }
}

fn automation_input_item(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> PermissionItem {
    let mut setup_steps = Vec::new();
    let (eligibility, detail) = if cap_ok(caps, "input.open") {
        (PermissionEligibility::Granted, None)
    } else if session.session_type == "wayland" {
        setup_steps.push(
            "In the screen share dialog, enable Allow Remote Interaction (pointer and keyboard), then Share."
                .into(),
        );
        setup_steps.push(
            "GNOME: the toggle is on the same picker as the monitor list. Re-share if capture was granted without it.".into(),
        );
        (
            PermissionEligibility::Needed,
            cap_fail_detail(caps, "input.open"),
        )
    } else {
        (
            PermissionEligibility::Needed,
            cap_fail_detail(caps, "input.open"),
        )
    };

    PermissionItem {
        id: "automation_input",
        title: "Desktop automation",
        summary: "Move the mouse and type keys while running macros.",
        eligibility,
        detail,
        setup_steps,
        copy_command: None,
        tooltip: None,
    }
}

fn global_hotkeys_item(
    session: &SessionReport,
    caps: &BTreeMap<String, CapabilityResult>,
) -> PermissionItem {
    let mut setup_steps = Vec::new();
    let copy_command;

    let (eligibility, detail) = if cap_ok(caps, "hotkeys.start") {
        copy_command = None;
        (PermissionEligibility::Granted, None)
    } else {
        #[cfg(target_os = "linux")]
        if session.session_type == "wayland" {
            setup_steps
                .push("Recording and Esc-stop hotkeys need input device access on Wayland.".into());
            copy_command = Some("sudo usermod -aG input $USER".into());
        } else {
            copy_command = None;
        }
        #[cfg(not(target_os = "linux"))]
        {
            copy_command = None;
        }
        (
            PermissionEligibility::Needed,
            cap_fail_detail(caps, "hotkeys.start"),
        )
    };

    PermissionItem {
        id: "global_hotkeys",
        title: "Global hotkeys",
        summary: "Esc to stop macros, record actions, and the failsafe chord.",
        eligibility,
        detail,
        setup_steps,
        copy_command,
        tooltip: None,
    }
}

#[cfg(target_os = "linux")]
fn screen_recording_hint(desktop: Option<&str>) -> String {
    crate::permissions::screen_recording_hint(desktop)
}

#[cfg(not(target_os = "linux"))]
fn screen_recording_hint(_desktop: Option<&str>) -> String {
    "Grant screen recording when the portal dialog appears.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapStatus;

    fn session_wayland() -> SessionReport {
        SessionReport {
            session_type: "wayland".into(),
            desktop: Some("GNOME".into()),
            compositor: Some("mutter".into()),
            portal_version: None,
            display: Some(":0".into()),
            wayland_display: Some("wayland-0".into()),
            capture_backend: Some("portal+pipewire".into()),
        }
    }

    fn ok_cap() -> CapabilityResult {
        CapabilityResult {
            status: CapStatus::Ok,
            ..CapabilityResult::default()
        }
    }

    fn fail_cap(msg: &str) -> CapabilityResult {
        CapabilityResult {
            status: CapStatus::Fail,
            error: Some(msg.into()),
            ..CapabilityResult::default()
        }
    }

    #[test]
    fn evdev_access_needed_on_wayland_when_missing() {
        let mut caps = BTreeMap::new();
        caps.insert(
            "permissions.evdev_access".into(),
            fail_cap("cannot open /dev/input"),
        );
        caps.insert("capture.open".into(), ok_cap());
        caps.insert("hotkeys.start".into(), ok_cap());
        caps.insert("input.open".into(), ok_cap());
        caps.insert("portal.global_shortcuts".into(), ok_cap());
        caps.insert("portal.screencast".into(), ok_cap());

        let items = build_permission_items(&session_wayland(), &caps);
        let input = items
            .iter()
            .find(|i| i.id == "input_device_group")
            .expect("input row");
        assert_eq!(input.eligibility, PermissionEligibility::Needed);
        assert_eq!(
            input.copy_command.as_deref(),
            Some("sudo usermod -aG input $USER")
        );
        #[cfg(target_os = "linux")]
        if crate::permissions::is_immutable_linux() {
            assert!(
                input
                    .setup_steps
                    .iter()
                    .any(|s| s.contains("/usr/lib/group")),
                "expected atomic setup steps on immutable Linux"
            );
            assert!(input.tooltip.is_some());
        }
    }

    fn pending_cap() -> CapabilityResult {
        CapabilityResult {
            status: CapStatus::Pending,
            reason: Some("waiting for portal ScreenCast permission".into()),
            ..CapabilityResult::default()
        }
    }

    #[test]
    fn screen_recording_pending_is_checking() {
        let mut caps = BTreeMap::new();
        caps.insert("capture.open".into(), pending_cap());
        caps.insert("portal.screencast".into(), pending_cap());
        let items = build_permission_items(&session_wayland(), &caps);
        let row = items
            .iter()
            .find(|i| i.id == "screen_recording")
            .expect("screen recording row");
        assert_eq!(row.eligibility, PermissionEligibility::Checking);
        assert!(row.setup_steps.is_empty());
    }

    #[test]
    fn automation_needed_on_wayland_without_eis() {
        let mut caps = BTreeMap::new();
        caps.insert("capture.open".into(), ok_cap());
        caps.insert("portal.screencast".into(), ok_cap());
        caps.insert(
            "input.open".into(),
            fail_cap("enable Allow Remote Interaction in the screen share dialog"),
        );
        let items = build_permission_items(&session_wayland(), &caps);
        let row = items
            .iter()
            .find(|i| i.id == "automation_input")
            .expect("automation row");
        assert_eq!(row.eligibility, PermissionEligibility::Needed);
        assert!(row
            .setup_steps
            .iter()
            .any(|s| s.contains("Allow Remote Interaction")));
    }

    #[test]
    fn screen_recording_granted_when_capture_open_ok() {
        let mut caps = BTreeMap::new();
        caps.insert("capture.open".into(), ok_cap());
        caps.insert("portal.screencast".into(), pending_cap());
        let items = build_permission_items(&session_wayland(), &caps);
        let row = items
            .iter()
            .find(|i| i.id == "screen_recording")
            .expect("screen recording row");
        assert_eq!(row.eligibility, PermissionEligibility::Granted);
    }

    #[test]
    fn atomic_group_setup_steps_include_group_name() {
        #[cfg(target_os = "linux")]
        {
            let steps = crate::permissions::atomic_group_setup_steps("input");
            assert!(steps.iter().any(|s| s.contains("^input:")));
            assert!(steps
                .iter()
                .any(|s| s.contains("ujust add-user-to-input-group")));
        }
    }
}
