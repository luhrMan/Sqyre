//! Desktop capability probe — structured JSON for agents and CI.

mod checksum;
#[cfg(target_os = "linux")]
mod linux_portal;
mod permissions;
mod permissions_panel;

use serde::Serialize;
use sqyre_capture::{cap_log, event_log};
use sqyre_hotkeys::HotkeyCallbacks;
use sqyre_input::OsAutomation;
use sqyre_ports::{DesktopRect, ScreenCapturer};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use web_time::Instant as WebInstant;

pub use checksum::fnv1a_hex;
pub use permissions::user_can_open_evdev;
pub use permissions_panel::{build_permission_items, PermissionEligibility, PermissionItem};

/// Result of one capability check.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapStatus {
    Ok,
    Fail,
    Skip,
    Pending,
}

/// Per-capability probe output (flat JSON object per key in the report map).
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityResult {
    pub status: CapStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

impl Default for CapabilityResult {
    fn default() -> Self {
        Self {
            status: CapStatus::Fail,
            backend: None,
            error: None,
            reason: None,
            ms: None,
            size: None,
            checksum: None,
            count: None,
        }
    }
}

impl CapabilityResult {
    fn ok() -> Self {
        Self {
            status: CapStatus::Ok,
            ..Self::default()
        }
    }

    fn fail(error: impl Into<String>) -> Self {
        Self {
            status: CapStatus::Fail,
            error: Some(error.into()),
            ..Self::default()
        }
    }

    fn skip(reason: impl Into<String>) -> Self {
        Self {
            status: CapStatus::Skip,
            reason: Some(reason.into()),
            ..Self::default()
        }
    }

    fn pending(reason: impl Into<String>) -> Self {
        Self {
            status: CapStatus::Pending,
            reason: Some(reason.into()),
            ..Self::default()
        }
    }
}

/// Session metadata included in every report.
#[derive(Debug, Clone, Serialize)]
pub struct SessionReport {
    #[serde(rename = "type")]
    pub session_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compositor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wayland_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_backend: Option<String>,
}

/// Full probe output (JSON document root).
#[derive(Debug, Clone, Serialize)]
pub struct ProbeReport {
    pub session: SessionReport,
    pub capabilities: BTreeMap<String, CapabilityResult>,
    pub permissions_needed: Vec<String>,
    pub parity_tier: String,
}

/// Options controlling probe behavior.
#[derive(Debug, Clone)]
pub struct ProbeOptions {
    /// Capability keys that must pass for exit code 0.
    pub required: Vec<String>,
    /// Poll until required caps pass (seconds); 0 = single run.
    pub wait_permissions_secs: u64,
    /// Print human text to stderr instead of JSON-only stdout.
    pub human: bool,
    /// Do not start a second global hook thread (in-app permissions refresh).
    pub skip_hotkeys_probe: bool,
    /// Skip `SelectionOutline` / `SelectionGrab` open (in-app — those hitch the pointer).
    pub skip_outline_grab: bool,
    /// Do not block on a portal ScreenCast picker (`shared_capturer` OnceLock).
    pub nonblocking_capture: bool,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            required: default_required_caps(),
            wait_permissions_secs: 0,
            human: false,
            skip_hotkeys_probe: false,
            skip_outline_grab: false,
            nonblocking_capture: false,
        }
    }
}

/// Default capabilities required for macro automation parity.
pub fn default_required_caps() -> Vec<String> {
    vec![
        "capture.open".into(),
        "capture.rect".into(),
        "windows.list".into(),
        "input.open".into(),
        "hotkeys.start".into(),
        "outline.open".into(),
        "grab.open".into(),
    ]
}

/// Run all capability probes once and return a structured report.
pub fn run_probe(opts: &ProbeOptions) -> ProbeReport {
    let started = Instant::now();
    let session = detect_session();
    session.log_native();

    let mut caps = BTreeMap::new();
    probe_capture(&session, &mut caps, opts);
    probe_windows(&mut caps);
    probe_input(&session, &mut caps);
    if opts.skip_hotkeys_probe {
        probe_hotkeys_inferred(&session, &mut caps);
    } else {
        probe_hotkeys(&session, &mut caps);
    }
    probe_outline_grab(&mut caps, opts);
    #[cfg(target_os = "linux")]
    linux_portal::probe_portal(&session, &mut caps, opts);
    #[cfg(not(target_os = "linux"))]
    {
        caps.insert(
            "portal.screencast".into(),
            CapabilityResult::skip("Linux-only"),
        );
        caps.insert(
            "portal.global_shortcuts".into(),
            CapabilityResult::skip("Linux-only"),
        );
    }
    permissions::probe_permissions(&session, &mut caps);

    let permissions_needed = permissions::collect_hints(&session, &caps);
    let parity_tier = compute_parity_tier(&caps, &opts.required);

    let elapsed = started.elapsed().as_millis();
    event_log(
        "SQYRE_PROBE",
        &[("parity_tier", &parity_tier), ("ms", &elapsed.to_string())],
    );

    ProbeReport {
        session,
        capabilities: caps,
        permissions_needed,
        parity_tier,
    }
}

/// Run probe, optionally waiting for permissions; returns (report, exit_code).
pub fn run_probe_with_wait(opts: &ProbeOptions) -> (ProbeReport, i32) {
    if opts.wait_permissions_secs == 0 {
        let report = run_probe(opts);
        return (report.clone(), exit_code(&report, opts));
    }

    let deadline = WebInstant::now() + Duration::from_secs(opts.wait_permissions_secs.max(1));
    let mut last = run_probe(opts);
    loop {
        let code = exit_code(&last, opts);
        if code == 0 || WebInstant::now() >= deadline {
            return (last, code);
        }
        if opts.human {
            eprintln!(
                "sqyre-probe: waiting for permissions ({}s left)…",
                (deadline - WebInstant::now()).as_secs()
            );
        }
        std::thread::sleep(Duration::from_secs(2));
        last = run_probe(opts);
    }
}

/// Exit code: 0 = all required ok, 1 = capability failure, 2 = internal error.
pub fn exit_code(report: &ProbeReport, opts: &ProbeOptions) -> i32 {
    for key in &opts.required {
        match report.capabilities.get(key) {
            Some(c) if c.status == CapStatus::Ok => {}
            Some(c) if c.status == CapStatus::Skip => {}
            Some(_) | None => return 1,
        }
    }
    0
}

fn compute_parity_tier(caps: &BTreeMap<String, CapabilityResult>, required: &[String]) -> String {
    let mut ok = 0u32;
    let mut need = 0u32;
    for key in required {
        need += 1;
        if matches!(
            caps.get(key).map(|c| &c.status),
            Some(CapStatus::Ok) | Some(CapStatus::Skip)
        ) {
            ok += 1;
        }
    }
    if ok == need {
        "full".into()
    } else if ok > 0 {
        "partial".into()
    } else {
        "none".into()
    }
}

fn detect_session() -> SessionReport {
    #[cfg(target_os = "linux")]
    {
        let info = sqyre_capture::LinuxSessionInfo::detect();
        let portal_version = linux_portal::portal_version();
        SessionReport {
            session_type: info.session_kind.as_str().into(),
            desktop: info.desktop.clone(),
            compositor: info.compositor.clone(),
            portal_version,
            display: info.display.clone(),
            wayland_display: info.wayland_display.clone(),
            capture_backend: Some(info.capture_backend().as_str().into()),
        }
    }
    #[cfg(target_os = "windows")]
    {
        SessionReport {
            session_type: "windows".into(),
            desktop: None,
            compositor: None,
            portal_version: None,
            display: None,
            wayland_display: None,
            capture_backend: Some("gdi".into()),
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        SessionReport {
            session_type: "other".into(),
            desktop: None,
            compositor: None,
            portal_version: None,
            display: None,
            wayland_display: None,
            capture_backend: None,
        }
    }
}

impl SessionReport {
    fn log_native(&self) {
        #[cfg(target_os = "linux")]
        {
            sqyre_capture::LinuxSessionInfo::detect().log_session();
        }
        #[cfg(not(target_os = "linux"))]
        {
            event_log(
                "SQYRE_SESSION",
                &[
                    ("type", self.session_type.as_str()),
                    (
                        "backend",
                        self.capture_backend.as_deref().unwrap_or("unknown"),
                    ),
                ],
            );
        }
    }
}

fn timed<F: FnOnce() -> CapabilityResult>(f: F) -> CapabilityResult {
    let t0 = Instant::now();
    let mut r = f();
    r.ms = Some(t0.elapsed().as_millis() as u64);
    r
}

fn probe_capture(
    session: &SessionReport,
    caps: &mut BTreeMap<String, CapabilityResult>,
    opts: &ProbeOptions,
) {
    let backend = session
        .capture_backend
        .clone()
        .unwrap_or_else(|| "unknown".into());

    caps.insert(
        "capture.open".into(),
        timed(|| {
            if opts.nonblocking_capture && sqyre_capture::shared_capturer_open_may_block() {
                return match sqyre_capture::shared_capturer_if_ready() {
                    Some(Ok(_)) => {
                        cap_log("CAP", "ok", &format!("backend={backend} op=open"));
                        CapabilityResult {
                            status: CapStatus::Ok,
                            backend: Some(backend.clone()),
                            ..CapabilityResult::default()
                        }
                    }
                    Some(Err(e)) => {
                        cap_log("CAP", "fail", &format!("error={e} op=open"));
                        CapabilityResult {
                            status: CapStatus::Fail,
                            backend: Some(backend.clone()),
                            error: Some(e.to_string()),
                            ..CapabilityResult::default()
                        }
                    }
                    None => CapabilityResult::pending("waiting for portal ScreenCast permission"),
                };
            }
            match sqyre_capture::shared_capturer() {
                Ok(_) => {
                    cap_log("CAP", "ok", &format!("backend={backend} op=open"));
                    CapabilityResult {
                        status: CapStatus::Ok,
                        backend: Some(backend.clone()),
                        ..CapabilityResult::default()
                    }
                }
                Err(e) => {
                    cap_log("CAP", "fail", &format!("error={e} op=open"));
                    CapabilityResult {
                        status: CapStatus::Fail,
                        backend: Some(backend.clone()),
                        error: Some(e.to_string()),
                        ..CapabilityResult::default()
                    }
                }
            }
        }),
    );

    let capture_open_pending = matches!(
        caps.get("capture.open").map(|c| &c.status),
        Some(CapStatus::Pending)
    );

    caps.insert(
        "capture.rect".into(),
        timed(|| {
            if capture_open_pending {
                return CapabilityResult::pending("waiting for portal ScreenCast permission");
            }
            let Ok(capturer) = sqyre_capture::shared_capturer() else {
                return CapabilityResult::fail("capture.open failed");
            };
            let mut wrap = sqyre_capture::SharedRunCapturer(capturer);
            let vb = match wrap.virtual_bounds() {
                Ok(v) => v,
                Err(e) => return CapabilityResult::fail(e.to_string()),
            };
            let w = vb.w.clamp(1, 128);
            let h = vb.h.clamp(1, 128);
            let rect = DesktopRect {
                x: vb.x,
                y: vb.y,
                w,
                h,
            };
            match wrap.capture_rect(rect) {
                Ok(img) => {
                    let checksum = fnv1a_hex(img.as_raw());
                    let size = format!("{}x{}", img.width(), img.height());
                    cap_log(
                        "CAP",
                        "ok",
                        &format!("backend={backend} rect={size} checksum={checksum}"),
                    );
                    CapabilityResult {
                        status: CapStatus::Ok,
                        backend: Some(backend.clone()),
                        size: Some(size),
                        checksum: Some(checksum),
                        ..CapabilityResult::default()
                    }
                }
                Err(e) => {
                    cap_log("CAP", "fail", &format!("error={e} op=rect"));
                    CapabilityResult {
                        status: CapStatus::Fail,
                        error: Some(e.to_string()),
                        ..CapabilityResult::default()
                    }
                }
            }
        }),
    );

    caps.insert(
        "capture.multi_monitor".into(),
        timed(|| {
            if capture_open_pending {
                return CapabilityResult::pending("waiting for portal ScreenCast permission");
            }
            let Ok(capturer) = sqyre_capture::shared_capturer() else {
                return CapabilityResult::skip("capture.open failed");
            };
            let mut wrap = sqyre_capture::SharedRunCapturer(capturer);
            match wrap.monitor_sizes() {
                Ok(sizes) if sizes.len() > 1 => CapabilityResult {
                    status: CapStatus::Ok,
                    count: Some(sizes.len()),
                    ..CapabilityResult::default()
                },
                Ok(sizes) => CapabilityResult::skip(format!("single monitor ({})", sizes.len())),
                Err(e) => CapabilityResult::fail(e.to_string()),
            }
        }),
    );

    caps.insert(
        "capture.pointer".into(),
        timed(|| {
            #[cfg(target_os = "linux")]
            {
                if capture_open_pending {
                    return CapabilityResult::pending("waiting for portal ScreenCast permission");
                }
                if session.capture_backend.as_deref() == Some("portal+pipewire") {
                    return CapabilityResult::skip("portal capture has no pointer API");
                }
                let Ok(cap) = sqyre_capture::shared_capturer() else {
                    return CapabilityResult::fail("capture.open failed");
                };
                match cap.pointer_position() {
                    Ok((x, y)) => {
                        cap_log("CAP", "ok", &format!("pointer={x},{y}"));
                        CapabilityResult {
                            status: CapStatus::Ok,
                            size: Some(format!("{x},{y}")),
                            ..CapabilityResult::default()
                        }
                    }
                    Err(e) => CapabilityResult::fail(e.to_string()),
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                CapabilityResult::skip("Linux X11 only")
            }
        }),
    );

    // Wayland-native capture implementation status.
    #[cfg(target_os = "linux")]
    if session.capture_backend.as_deref() == Some("portal+pipewire") {
        caps.insert(
            "capture.wayland_impl".into(),
            if sqyre_capture::linux::wayland::portal_capture_implemented() {
                CapabilityResult::ok()
            } else {
                CapabilityResult::pending("portal ScreenCast backend not implemented")
            },
        );
    }
}

fn probe_windows(caps: &mut BTreeMap<String, CapabilityResult>) {
    caps.insert(
        "windows.list".into(),
        timed(|| match sqyre_capture::list_open_windows() {
            Ok(wins) => {
                cap_log("FOCUS", "ok", &format!("list count={}", wins.len()));
                CapabilityResult {
                    status: CapStatus::Ok,
                    count: Some(wins.len()),
                    ..CapabilityResult::default()
                }
            }
            Err(e) => CapabilityResult::fail(e.to_string()),
        }),
    );

    caps.insert(
        "windows.active".into(),
        timed(|| match sqyre_capture::get_active_window() {
            Ok(Some(w)) => CapabilityResult {
                status: CapStatus::Ok,
                size: Some(w.title),
                ..CapabilityResult::default()
            },
            Ok(None) => CapabilityResult::skip("no active window"),
            Err(e) => CapabilityResult::fail(e.to_string()),
        }),
    );

    #[cfg(target_os = "linux")]
    caps.insert(
        "windows.wayland_impl".into(),
        match sqyre_capture::linux::wayland::toplevel_focus_available() {
            Ok(()) => CapabilityResult::ok(),
            Err(e) => CapabilityResult::pending(e.to_string()),
        },
    );
}

fn probe_input(session: &SessionReport, caps: &mut BTreeMap<String, CapabilityResult>) {
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    if session.session_type == "wayland" {
        caps.insert(
            "input.open".into(),
            timed(|| {
                if sqyre_capture::portal_input_ready() {
                    cap_log("INPUT", "ok", "backend=eis");
                    CapabilityResult {
                        status: CapStatus::Ok,
                        backend: Some("eis".into()),
                        ..CapabilityResult::default()
                    }
                } else if sqyre_capture::portal_remote_desktop_granted() {
                    cap_log("INPUT", "fail", "reason=eis_not_ready");
                    CapabilityResult::fail(String::from(
                        "Remote Desktop granted but EIS pointer is not ready",
                    ))
                } else if sqyre_capture::portal_screencast_granted() {
                    cap_log("INPUT", "fail", "reason=no_remote_interaction");
                    CapabilityResult::fail(String::from(
                        "enable Allow Remote Interaction in the screen share dialog",
                    ))
                } else {
                    cap_log("INPUT", "fail", "reason=portal_input_not_granted");
                    CapabilityResult::fail(String::from(
                        "grant screen share with Allow Remote Interaction so mouse playback works",
                    ))
                }
            }),
        );
        caps.insert(
            "input.wayland_impl".into(),
            if sqyre_capture::portal_input_ready() {
                CapabilityResult {
                    status: CapStatus::Ok,
                    backend: Some("eis".into()),
                    ..CapabilityResult::default()
                }
            } else {
                CapabilityResult::pending("RemoteDesktop EIS (combined ScreenCast session)")
            },
        );
        return;
    }

    caps.insert(
        "input.open".into(),
        timed(|| match std::panic::catch_unwind(OsAutomation::new) {
            Ok(Ok(_)) => {
                cap_log("INPUT", "ok", "backend=rustautogui");
                CapabilityResult {
                    status: CapStatus::Ok,
                    backend: Some("rustautogui".into()),
                    ..CapabilityResult::default()
                }
            }
            Ok(Err(e)) => {
                cap_log("INPUT", "fail", &format!("error={e}"));
                CapabilityResult::fail(e.to_string())
            }
            Err(_) => {
                cap_log("INPUT", "fail", "reason=backend_panicked");
                CapabilityResult::fail(
                    "input backend panicked (no X display / headless environment?)",
                )
            }
        }),
    );

    #[cfg(target_os = "linux")]
    if session.session_type == "wayland" && session.display.is_none() {
        caps.insert(
            "input.wayland_impl".into(),
            CapabilityResult::pending("uinput backend not implemented"),
        );
    }
}

fn hotkeys_backend_label() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "win32-llhook"
    }
    #[cfg(target_os = "linux")]
    {
        if sqyre_hotkeys::linux_uses_evdev_grab() {
            "evdev"
        } else {
            "rdev-x11"
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        "null"
    }
}

fn probe_hotkeys_inferred(session: &SessionReport, caps: &mut BTreeMap<String, CapabilityResult>) {
    caps.insert(
        "hotkeys.start".into(),
        timed(|| {
            let backend = hotkeys_backend_label();
            #[cfg(target_os = "linux")]
            if session.session_type == "wayland" && !permissions::user_can_open_evdev() {
                cap_log("HOTKEY", "fail", "reason=evdev_unreadable");
                return CapabilityResult::fail("cannot open /dev/input (evdev unavailable)");
            }
            cap_log("HOTKEY", "ok", &format!("backend={backend} inferred"));
            CapabilityResult {
                status: CapStatus::Ok,
                backend: Some(backend.into()),
                reason: Some("inferred (global hooks already owned by Sqyre)".into()),
                ..CapabilityResult::default()
            }
        }),
    );
}

fn probe_hotkeys(session: &SessionReport, caps: &mut BTreeMap<String, CapabilityResult>) {
    caps.insert(
        "hotkeys.start".into(),
        timed(|| {
            let backend = hotkeys_backend_label();
            #[cfg(target_os = "linux")]
            if session.session_type == "wayland" && !permissions::user_can_open_evdev() {
                cap_log("HOTKEY", "fail", "reason=evdev_unreadable");
                return CapabilityResult::fail("cannot open /dev/input (evdev unavailable)");
            }
            let result = std::panic::catch_unwind(|| {
                let (mut hk, _, _, _, _) = sqyre_hotkeys::default_hotkeys();
                hk.start(HotkeyCallbacks::default()).map(|()| hk)
            });
            match result {
                Ok(Ok(mut hk)) => {
                    cap_log("HOTKEY", "ok", &format!("backend={backend}"));
                    hk.stop();
                    CapabilityResult {
                        status: CapStatus::Ok,
                        backend: Some(backend.into()),
                        ..CapabilityResult::default()
                    }
                }
                Ok(Err(e)) => {
                    cap_log("HOTKEY", "fail", &format!("error={e}"));
                    CapabilityResult::fail(e.to_string())
                }
                Err(_) => {
                    cap_log("HOTKEY", "fail", "reason=backend_panicked");
                    CapabilityResult::fail(
                        "hotkey backend panicked (no display / headless environment?)",
                    )
                }
            }
        }),
    );
}

fn probe_outline_grab(caps: &mut BTreeMap<String, CapabilityResult>, opts: &ProbeOptions) {
    if opts.skip_outline_grab {
        caps.insert(
            "outline.open".into(),
            CapabilityResult::skip("skipped (in-app probe)"),
        );
        caps.insert(
            "grab.open".into(),
            CapabilityResult::skip("skipped (in-app probe)"),
        );
        return;
    }
    caps.insert(
        "outline.open".into(),
        timed(|| match sqyre_capture::SelectionOutline::open() {
            Ok(o) => {
                cap_log("OUTLINE", "ok", "backend=x11");
                drop(o);
                CapabilityResult {
                    status: CapStatus::Ok,
                    backend: Some("x11".into()),
                    ..CapabilityResult::default()
                }
            }
            Err(e) => {
                cap_log("OUTLINE", "fail", &format!("error={e}"));
                CapabilityResult::fail(e.to_string())
            }
        }),
    );

    caps.insert(
        "grab.open".into(),
        timed(|| match sqyre_capture::SelectionGrab::open() {
            Ok(g) => {
                cap_log("GRAB", "ok", "backend=x11");
                drop(g);
                CapabilityResult {
                    status: CapStatus::Ok,
                    backend: Some("x11".into()),
                    ..CapabilityResult::default()
                }
            }
            Err(e) => {
                cap_log("GRAB", "fail", &format!("error={e}"));
                CapabilityResult::fail(e.to_string())
            }
        }),
    );

    #[cfg(target_os = "linux")]
    {
        caps.insert(
            "outline.wayland_impl".into(),
            match sqyre_capture::linux::wayland::layer_outline_available() {
                Ok(()) => CapabilityResult::ok(),
                Err(e) => CapabilityResult::pending(e.to_string()),
            },
        );
        caps.insert(
            "grab.wayland_impl".into(),
            match sqyre_capture::linux::wayland::layer_grab_available() {
                Ok(()) => CapabilityResult::ok(),
                Err(e) => CapabilityResult::pending(e.to_string()),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_required_non_empty() {
        assert!(default_required_caps().len() >= 5);
    }

    #[test]
    fn probe_runs_without_panic() {
        let report = run_probe(&ProbeOptions::default());
        assert!(report.capabilities.contains_key("capture.open"));
        assert!(!report.parity_tier.is_empty());
    }

    #[test]
    fn exit_code_ok_when_all_required_skip_or_ok() {
        let mut caps = BTreeMap::new();
        caps.insert("capture.open".into(), CapabilityResult::skip("headless"));
        let report = ProbeReport {
            session: SessionReport {
                session_type: "unknown".into(),
                desktop: None,
                compositor: None,
                portal_version: None,
                display: None,
                wayland_display: None,
                capture_backend: None,
            },
            capabilities: caps,
            permissions_needed: vec![],
            parity_tier: "partial".into(),
        };
        let opts = ProbeOptions {
            required: vec!["capture.open".into()],
            ..ProbeOptions::default()
        };
        assert_eq!(exit_code(&report, &opts), 0);
    }

    #[test]
    fn exit_code_fail_on_missing_required() {
        let report = run_probe(&ProbeOptions::default());
        let opts = ProbeOptions {
            required: vec!["nonexistent.cap".into()],
            ..ProbeOptions::default()
        };
        assert_eq!(exit_code(&report, &opts), 1);
    }

    #[test]
    fn skip_outline_grab_does_not_open_x11() {
        let mut caps = BTreeMap::new();
        let opts = ProbeOptions {
            skip_outline_grab: true,
            ..ProbeOptions::default()
        };
        probe_outline_grab(&mut caps, &opts);
        assert_eq!(
            caps.get("outline.open").map(|c| &c.status),
            Some(&CapStatus::Skip)
        );
        assert_eq!(
            caps.get("grab.open").map(|c| &c.status),
            Some(&CapStatus::Skip)
        );
    }

    #[test]
    fn skip_hotkeys_probe_infers_without_starting_hooks() {
        let mut caps = BTreeMap::new();
        let session = SessionReport {
            session_type: "wayland".into(),
            desktop: Some("GNOME".into()),
            compositor: None,
            portal_version: None,
            display: Some(":0".into()),
            wayland_display: Some("wayland-0".into()),
            capture_backend: Some("portal+pipewire".into()),
        };
        probe_hotkeys_inferred(&session, &mut caps);
        let cap = caps.get("hotkeys.start").expect("hotkeys cap");
        #[cfg(target_os = "linux")]
        if !permissions::user_can_open_evdev() {
            assert_eq!(cap.status, CapStatus::Fail);
        } else {
            assert_eq!(cap.status, CapStatus::Ok);
            assert!(cap.reason.as_deref().unwrap_or("").contains("inferred"));
        }
        #[cfg(not(target_os = "linux"))]
        assert_eq!(cap.status, CapStatus::Ok);
    }
}
