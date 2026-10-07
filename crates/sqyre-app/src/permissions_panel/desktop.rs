//! User Settings → Permissions: background probe + status rows.

use eframe::egui::{self, Color32, RichText};
use sqyre_hotkeys::{
    open_system_shortcuts, system_shortcuts_configurable, system_shortcuts_status,
    SystemShortcutsStatus,
};
use sqyre_probe::{
    build_permission_items, run_probe, PermissionEligibility, PermissionItem, ProbeOptions,
};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

enum ProbeMsg {
    Done(Result<Vec<PermissionItem>, String>),
}

const IN_APP_PROBE_TIMEOUT: Duration = Duration::from_secs(6);
/// Live grant checks read portal tokens from disk; do not repeat them every frame.
const LIVE_STATUS_INTERVAL: Duration = Duration::from_secs(1);
/// Minimum gap between re-probes triggered by the window regaining focus.
const FOCUS_REPROBE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct PermissionsPanel {
    items: Vec<PermissionItem>,
    running: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<ProbeMsg>>,
    started_once: bool,
    /// Re-run once after the deferred portal capturer finishes opening.
    refresh_when_capture_ready: bool,
    live_status_at: Option<Instant>,
    probed_at: Option<Instant>,
    was_focused: bool,
    missing: bool,
}

impl PermissionsPanel {
    /// Background work that keeps [`Self::has_missing`] current while Settings is closed.
    pub fn tick(&mut self, ctx: &egui::Context) {
        self.poll(ctx);
        self.ensure_loaded(ctx);
        self.maybe_refresh_after_capture(ctx);
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let regained = focused && !self.was_focused;
        self.was_focused = focused;
        if regained
            && self
                .probed_at
                .is_none_or(|t| t.elapsed() >= FOCUS_REPROBE_INTERVAL)
        {
            self.refresh(ctx);
        }
        if self
            .live_status_at
            .is_none_or(|t| t.elapsed() >= LIVE_STATUS_INTERVAL)
        {
            self.update_live_status();
        }
    }

    /// At least one permission still needs the user's action.
    pub fn has_missing(&self) -> bool {
        self.missing
    }

    fn update_live_status(&mut self) {
        apply_live_capture_status(&mut self.items);
        self.missing = any_missing(&self.items);
        self.live_status_at = Some(Instant::now());
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(ProbeMsg::Done(result)) => {
                self.rx = None;
                self.running = false;
                match result {
                    Ok(items) => {
                        self.items = items;
                        self.error = None;
                        self.update_live_status();
                    }
                    Err(e) => self.error = Some(e),
                }
                ctx.request_repaint();
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                self.running = false;
                self.error = Some("permission probe exited unexpectedly".into());
                ctx.request_repaint();
            }
        }
    }

    pub fn ensure_loaded(&mut self, ctx: &egui::Context) {
        if !self.started_once && !self.running && self.rx.is_none() {
            self.started_once = true;
            self.refresh(ctx);
        }
    }

    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.running || self.rx.is_some() {
            return;
        }
        self.running = true;
        self.error = None;
        self.probed_at = Some(Instant::now());
        self.refresh_when_capture_ready = sqyre_capture::shared_capturer_open_may_block()
            && sqyre_capture::shared_capturer_if_ready().is_none();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let opts = ProbeOptions {
                skip_hotkeys_probe: true,
                skip_outline_grab: true,
                nonblocking_capture: true,
                ..ProbeOptions::default()
            };
            let (inner_tx, inner_rx) = mpsc::sync_channel(1);
            thread::spawn(move || {
                let msg = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_probe(&opts)
                })) {
                    Ok(report) => {
                        let items = build_permission_items(&report.session, &report.capabilities);
                        ProbeMsg::Done(Ok(items))
                    }
                    Err(_) => ProbeMsg::Done(Err("permission probe panicked".into())),
                };
                let _ = inner_tx.send(msg);
            });
            let msg = match inner_rx.recv_timeout(IN_APP_PROBE_TIMEOUT) {
                Ok(m) => m,
                Err(_) => ProbeMsg::Done(Err("permission probe timed out".into())),
            };
            let _ = tx.send(msg);
        });
        ctx.request_repaint();
    }

    fn maybe_refresh_after_capture(&mut self, ctx: &egui::Context) {
        if self.running {
            return;
        }
        if sqyre_capture::shared_capturer_is_opening() {
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        if !self.refresh_when_capture_ready {
            return;
        }
        if sqyre_capture::shared_capturer_if_ready().is_some()
            || sqyre_capture::portal_screencast_granted()
        {
            self.refresh_when_capture_ready = false;
            self.refresh(ctx);
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    pub fn paint(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.poll(ctx);
        self.ensure_loaded(ctx);
        self.maybe_refresh_after_capture(ctx);
        self.update_live_status();
        if system_shortcuts_status() == SystemShortcutsStatus::Waiting {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Desktop permissions")
                    .strong()
                    .color(crate::theme::PRIMARY),
            );
            if self.running
                || self.refresh_when_capture_ready
                || sqyre_capture::shared_capturer_is_opening()
            {
                ui.weak("Checking…");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.running, egui::Button::new("Refresh"))
                    .on_hover_text(
                        "Re-run capability checks (portal dialogs may appear on Wayland).",
                    )
                    .clicked()
                {
                    self.refresh(ctx);
                }
            });
        });
        ui.label(
            RichText::new(
                "Grant these so capture, recording, hotkeys, and macro playback work on your session.",
            )
            .weak()
            .small(),
        );
        ui.add_space(crate::theme::SPACE_8);

        if let Some(err) = &self.error {
            ui.colored_label(crate::theme::error_fg(), err);
            ui.add_space(crate::theme::SPACE_8);
        }

        if (self.running || self.refresh_when_capture_ready) && self.items.is_empty() {
            ui.spinner();
            ui.weak("Probing screen capture, portals, and input access…");
            return;
        }

        if self.items.is_empty() && !self.running {
            ui.weak("No permission data yet — click Refresh.");
            return;
        }

        for item in self.items.clone() {
            match paint_permission_row(ui, ctx, &item) {
                PermissionRowAction::ShareScreen => {
                    sqyre_capture::request_portal_screencast_picker();
                    self.refresh_when_capture_ready = true;
                    self.update_live_status();
                    ctx.request_repaint();
                }
                PermissionRowAction::Revoke => {
                    sqyre_capture::revoke_portal_grants();
                    mark_portal_permissions_revoked(&mut self.items);
                    self.update_live_status();
                    self.refresh(ctx);
                    // `refresh` waits for a capturer that we just dropped on purpose.
                    self.refresh_when_capture_ready = false;
                    ctx.request_repaint();
                }
                PermissionRowAction::SystemShortcuts => {
                    open_system_shortcuts();
                    ctx.request_repaint_after(Duration::from_millis(300));
                }
                PermissionRowAction::None => {}
            }
            ui.add_space(crate::theme::SPACE_12);
        }
    }
}

/// Only `Needed` counts: `Checking`, `NotRequired`, and `Unavailable` are not the user's to fix.
fn any_missing(items: &[PermissionItem]) -> bool {
    items
        .iter()
        .any(|item| item.eligibility == PermissionEligibility::Needed)
}

fn apply_live_capture_status(items: &mut [PermissionItem]) {
    let portal_session = sqyre_capture::shared_capturer_open_may_block();
    let capture_granted = sqyre_capture::portal_screencast_granted();
    let opening = sqyre_capture::shared_capturer_is_opening();
    let input_ready = sqyre_capture::portal_input_ready();

    for item in items {
        match item.id {
            "screen_recording" if portal_session => {
                if capture_granted {
                    item.eligibility = PermissionEligibility::Granted;
                    item.detail = None;
                    item.setup_steps.clear();
                } else if opening {
                    item.eligibility = PermissionEligibility::Checking;
                    item.detail = Some("Waiting for the screen sharing dialog.".into());
                    item.setup_steps.clear();
                } else if item.eligibility == PermissionEligibility::Granted {
                    item.eligibility = PermissionEligibility::Needed;
                }
            }
            "automation_input" if portal_session => {
                if input_ready {
                    item.eligibility = PermissionEligibility::Granted;
                    item.detail = None;
                    item.setup_steps.clear();
                } else if item.eligibility == PermissionEligibility::Granted {
                    item.eligibility = PermissionEligibility::Needed;
                }
            }
            "global_shortcuts" => apply_system_shortcuts_status(item, system_shortcuts_status()),
            _ => {}
        }
    }
}

fn apply_system_shortcuts_status(item: &mut PermissionItem, status: SystemShortcutsStatus) {
    let (eligibility, detail) = match status {
        SystemShortcutsStatus::Unavailable => return,
        SystemShortcutsStatus::Waiting => (
            PermissionEligibility::Checking,
            "Waiting for the desktop's shortcut dialog.".to_string(),
        ),
        SystemShortcutsStatus::Active { bound } => (
            PermissionEligibility::Granted,
            match bound {
                0 => "No macro hotkeys to register yet.".to_string(),
                1 => "1 macro hotkey is handled by your desktop.".to_string(),
                n => format!("{n} macro hotkeys are handled by your desktop."),
            },
        ),
        SystemShortcutsStatus::Declined => (
            PermissionEligibility::Needed,
            "The shortcut dialog was dismissed. Hotkeys still work when Sqyre can read input devices."
                .to_string(),
        ),
        SystemShortcutsStatus::Failed(e) => (
            PermissionEligibility::Needed,
            format!("Desktop shortcuts failed ({e}). Hotkeys fall back to input devices."),
        ),
    };
    item.eligibility = eligibility;
    item.detail = Some(detail);
    item.setup_steps.clear();
}

fn mark_portal_permissions_revoked(items: &mut [PermissionItem]) {
    for item in items {
        if matches!(item.id, "screen_recording" | "automation_input")
            && matches!(
                item.eligibility,
                PermissionEligibility::Granted | PermissionEligibility::Checking
            )
        {
            item.eligibility = PermissionEligibility::Needed;
            item.detail = Some("Portal grant revoked.".into());
            item.setup_steps.clear();
        }
    }
}

enum PermissionRowAction {
    None,
    ShareScreen,
    Revoke,
    SystemShortcuts,
}

/// Button label for the global-shortcuts row, when the desktop backend can act.
fn system_shortcuts_button() -> Option<&'static str> {
    match system_shortcuts_status() {
        SystemShortcutsStatus::Active { bound } if bound > 0 && system_shortcuts_configurable() => {
            Some("Change shortcuts")
        }
        SystemShortcutsStatus::Declined => Some("Set up shortcuts"),
        _ => None,
    }
}

fn paint_permission_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    item: &PermissionItem,
) -> PermissionRowAction {
    let mut action = PermissionRowAction::None;
    let portal_session = sqyre_capture::shared_capturer_open_may_block();
    super::row_frame(ui).show(ui, |ui| {
            super::row_header(
                ui,
                item.title,
                item.tooltip.as_deref().unwrap_or(item.summary),
                item.eligibility.label(),
                eligibility_color(item.eligibility),
            );
            ui.label(RichText::new(item.summary).weak().small());
            if let Some(detail) = &item.detail {
                ui.add_space(crate::theme::SPACE_4);
                ui.label(RichText::new(detail).small().color(crate::theme::warn_fg()));
            }
            for step in &item.setup_steps {
                ui.label(RichText::new(format!("• {step}")).small());
            }
            if let Some(cmd) = &item.copy_command {
                ui.add_space(crate::theme::SPACE_4);
                ui.horizontal(|ui| {
                    ui.monospace(cmd);
                    if ui.button("Copy").clicked() {
                        ctx.copy_text(cmd.clone());
                    }
                });
            }
            let share_screen = item.id == "screen_recording" && portal_session;
            let revoke = item.portal_grant_revocable() && portal_session;
            let shortcuts = (item.id == "global_shortcuts")
                .then(system_shortcuts_button)
                .flatten();
            if share_screen || revoke || shortcuts.is_some() {
                ui.add_space(crate::theme::SPACE_8);
                ui.horizontal(|ui| {
                    if let Some(label) = shortcuts {
                        if ui
                            .button(label)
                            .on_hover_text(
                                "Open your desktop's shortcut settings to choose the keys that run your macros.",
                            )
                            .clicked()
                        {
                            action = PermissionRowAction::SystemShortcuts;
                        }
                    }
                    if share_screen {
                        let label = if item.eligibility == PermissionEligibility::Granted {
                            "Change shared screen"
                        } else {
                            "Share screen"
                        };
                        let enabled = !sqyre_capture::shared_capturer_is_opening();
                        if ui
                            .add_enabled(enabled, egui::Button::new(label))
                            .on_hover_text(
                                "Open the desktop portal picker to choose which screens Sqyre can capture.",
                            )
                            .clicked()
                        {
                            action = PermissionRowAction::ShareScreen;
                        }
                    }
                    if revoke
                        && ui
                            .button("Revoke")
                            .on_hover_text(
                                "Stop capturing and forget the saved portal grant. Sqyre will ask again the next time it needs screen access.",
                            )
                            .clicked()
                    {
                        action = PermissionRowAction::Revoke;
                    }
                });
            }
        });
    action
}

fn eligibility_color(status: PermissionEligibility) -> Color32 {
    match status {
        PermissionEligibility::Granted => crate::theme::ok_fg(),
        PermissionEligibility::Needed => crate::theme::warn_fg(),
        PermissionEligibility::Checking => crate::theme::PRIMARY,
        PermissionEligibility::NotRequired => Color32::from_gray(140),
        PermissionEligibility::Unavailable => crate::theme::error_fg(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_not_running() {
        let panel = PermissionsPanel::default();
        assert!(!panel.running);
        assert!(panel.items.is_empty());
    }

    fn granted_item(id: &'static str) -> PermissionItem {
        PermissionItem {
            id,
            title: id,
            summary: "",
            eligibility: PermissionEligibility::Granted,
            detail: None,
            setup_steps: Vec::new(),
            copy_command: None,
            tooltip: None,
        }
    }

    #[test]
    fn only_needed_counts_as_missing() {
        let mut item = granted_item("screen_recording");
        for (eligibility, missing) in [
            (PermissionEligibility::Granted, false),
            (PermissionEligibility::NotRequired, false),
            (PermissionEligibility::Unavailable, false),
            (PermissionEligibility::Checking, false),
            (PermissionEligibility::Needed, true),
        ] {
            item.eligibility = eligibility;
            assert_eq!(any_missing(std::slice::from_ref(&item)), missing);
        }
        assert!(!any_missing(&[]));
    }

    #[test]
    fn revoke_marks_portal_rows_needed() {
        let mut items = vec![
            granted_item("screen_recording"),
            granted_item("automation_input"),
            granted_item("global_hotkeys"),
        ];
        mark_portal_permissions_revoked(&mut items);
        assert_eq!(items[0].eligibility, PermissionEligibility::Needed);
        assert_eq!(items[1].eligibility, PermissionEligibility::Needed);
        assert_eq!(items[2].eligibility, PermissionEligibility::Granted);
        assert!(items[0]
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("revoked")));
    }
}
