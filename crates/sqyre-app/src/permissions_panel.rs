//! User Settings → Permissions: background probe + status rows.

use eframe::egui::{self, Color32, RichText};
use sqyre_probe::{
    build_permission_items, run_probe, PermissionEligibility, PermissionItem, ProbeOptions,
};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

enum ProbeMsg {
    Done(Result<Vec<PermissionItem>, String>),
}

const IN_APP_PROBE_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Default)]
pub struct PermissionsPanel {
    items: Vec<PermissionItem>,
    running: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<ProbeMsg>>,
    started_once: bool,
    /// Re-run once after the deferred portal capturer finishes opening.
    refresh_when_capture_ready: bool,
}

impl PermissionsPanel {
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(ProbeMsg::Done(result)) => {
                self.rx = None;
                self.running = false;
                match result {
                    Ok(mut items) => {
                        apply_live_capture_status(&mut items);
                        self.items = items;
                        self.error = None;
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
        apply_live_capture_status(&mut self.items);

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
        ui.add_space(8.0);

        if let Some(err) = &self.error {
            ui.colored_label(crate::theme::error_fg(), err);
            ui.add_space(6.0);
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
                    apply_live_capture_status(&mut self.items);
                    ctx.request_repaint();
                }
                PermissionRowAction::Revoke => {
                    sqyre_capture::revoke_portal_grants();
                    mark_portal_permissions_revoked(&mut self.items);
                    apply_live_capture_status(&mut self.items);
                    self.refresh(ctx);
                    // `refresh` waits for a capturer that we just dropped on purpose.
                    self.refresh_when_capture_ready = false;
                    ctx.request_repaint();
                }
                PermissionRowAction::None => {}
            }
            ui.add_space(10.0);
        }
    }
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
            _ => {}
        }
    }
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
}

fn paint_permission_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    item: &PermissionItem,
) -> PermissionRowAction {
    let mut action = PermissionRowAction::None;
    let portal_session = sqyre_capture::shared_capturer_open_may_block();
    egui::Frame::NONE
        .fill(crate::theme::overlay_panel_fill())
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let title = ui.label(RichText::new(item.title).strong());
                if let Some(tip) = &item.tooltip {
                    title.on_hover_text(tip);
                } else {
                    title.on_hover_text(item.summary);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(
                        eligibility_color(item.eligibility),
                        item.eligibility.label(),
                    );
                });
            });
            ui.label(RichText::new(item.summary).weak().small());
            if let Some(detail) = &item.detail {
                ui.add_space(4.0);
                ui.label(RichText::new(detail).small().color(crate::theme::warn_fg()));
            }
            for step in &item.setup_steps {
                ui.label(RichText::new(format!("• {step}")).small());
            }
            if let Some(cmd) = &item.copy_command {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.monospace(cmd);
                    if ui.button("Copy").clicked() {
                        ctx.copy_text(cmd.clone());
                    }
                });
            }
            let share_screen = item.id == "screen_recording" && portal_session;
            let revoke = item.portal_grant_revocable() && portal_session;
            if share_screen || revoke {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
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
