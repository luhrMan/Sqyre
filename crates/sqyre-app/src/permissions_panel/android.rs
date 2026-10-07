//! Settings → Permissions on Android: what the phone shell needs to run macros.

use eframe::egui::{self, Color32, RichText};
use sqyre_android::{bridge, frames, AndroidError, Projection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Granted,
    Needed,
    Unknown,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Granted => "Granted",
            Self::Needed => "Needed",
            Self::Unknown => "Unknown",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Granted => crate::theme::ok_fg(),
            Self::Needed => crate::theme::warn_fg(),
            Self::Unknown => crate::theme::error_fg(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fix {
    AllowRecording,
    AccessibilitySettings,
    NotificationSettings,
}

struct Row {
    title: &'static str,
    summary: &'static str,
    status: Status,
    detail: Option<String>,
    button: RowButton,
}

/// Every row has one button that opens its prompt or system settings page.
struct RowButton {
    label: &'static str,
    tip: &'static str,
    fix: Fix,
    enabled: bool,
}

/// System-settings rows are read over JNI only when the page opens or Sqyre returns to
/// the front (also while Settings is closed, for the missing-permission badge); screen
/// recording state is in memory and read every frame.
#[derive(Default)]
pub struct PermissionsPanel {
    system_rows: Vec<Row>,
    /// Pass this page last painted in; a gap means it was just opened.
    last_pass: Option<u64>,
    shell_shown: u64,
}

impl PermissionsPanel {
    /// Keeps [`Self::has_missing`] current while Settings is closed.
    pub fn tick(&mut self, _ctx: &egui::Context) {
        self.refresh_system_rows(false);
    }

    /// Accessibility or notifications are off, or screen recording was denied.
    /// A projection that has not started yet does not count: it is requested per run.
    pub fn has_missing(&self) -> bool {
        frames().projection() == Projection::Stopped
            || self.system_rows.iter().any(|r| r.status == Status::Needed)
    }

    fn refresh_system_rows(&mut self, force: bool) {
        let shown = frames().shell_shown_count();
        if force || shown != self.shell_shown || self.system_rows.is_empty() {
            self.system_rows = vec![
                accessibility(bridge::accessibility_enabled()),
                notifications(bridge::notifications_enabled()),
            ];
            self.shell_shown = shown;
        }
    }

    pub fn paint(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pass = ctx.cumulative_pass_nr();
        let opened = self.last_pass.is_none_or(|p| p + 1 < pass);
        self.last_pass = Some(pass);
        self.refresh_system_rows(opened);
        let recording = screen_recording(frames().projection());

        ui.label(
            RichText::new("Android permissions")
                .strong()
                .color(crate::theme::PRIMARY),
        );
        ui.label(
            RichText::new("Grant these so macros can see the screen, tap, type, and be stopped.")
                .weak()
                .small(),
        );
        ui.add_space(crate::theme::SPACE_8);

        let mut clicked = None;
        for row in std::iter::once(&recording).chain(&self.system_rows) {
            if let Some(fix) = paint_row(ui, row) {
                clicked = Some(fix);
            }
            ui.add_space(crate::theme::SPACE_12);
        }
        if let Some(fix) = clicked {
            let result = match fix {
                Fix::AllowRecording => {
                    frames().rearm();
                    bridge::request_projection()
                }
                Fix::AccessibilitySettings => bridge::open_accessibility_settings(),
                Fix::NotificationSettings => bridge::open_notification_settings(),
            };
            if let Err(e) = result {
                crate::log::warn(format_args!("permissions: {fix:?} failed: {e}"));
            }
        }
    }
}

fn paint_row(ui: &mut egui::Ui, row: &Row) -> Option<Fix> {
    let mut clicked = None;
    super::row_frame(ui).show(ui, |ui| {
        super::row_header(
            ui,
            row.title,
            row.summary,
            row.status.label(),
            row.status.color(),
        );
        ui.label(RichText::new(row.summary).weak().small());
        if let Some(detail) = &row.detail {
            ui.add_space(crate::theme::SPACE_4);
            ui.label(RichText::new(detail).small().color(crate::theme::warn_fg()));
        }
        ui.add_space(crate::theme::SPACE_8);
        let b = &row.button;
        if ui
            .add_enabled(b.enabled, egui::Button::new(b.label))
            .on_hover_text(b.tip)
            .on_disabled_hover_text(b.tip)
            .clicked()
        {
            clicked = Some(b.fix);
        }
    });
    clicked
}

fn screen_recording(projection: Projection) -> Row {
    let (status, detail) = match projection {
        Projection::Running => (Status::Granted, None),
        Projection::NotStarted => (
            Status::Needed,
            Some("Android asks again each time Sqyre starts.".to_string()),
        ),
        Projection::Stopped => (
            Status::Needed,
            Some("Screen recording was denied or stopped.".to_string()),
        ),
    };
    let running = projection == Projection::Running;
    Row {
        title: "Screen recording",
        summary: "Lets image search, OCR, Find Pixel, and previews see the screen.",
        status,
        detail,
        button: RowButton {
            label: "Allow screen recording",
            tip: if running {
                "Already allowed until Sqyre closes."
            } else {
                "Show Android's screen recording prompt."
            },
            fix: Fix::AllowRecording,
            enabled: !running,
        },
    }
}

fn accessibility(enabled: Result<bool, AndroidError>) -> Row {
    let (status, detail) = match enabled {
        Ok(true) => (Status::Granted, None),
        Ok(false) => (
            Status::Needed,
            Some("Turn on Sqyre under Accessibility → Installed apps.".to_string()),
        ),
        Err(e) => (Status::Unknown, Some(format!("Could not check: {e}"))),
    };
    Row {
        title: "Accessibility service",
        summary: "Lets macros tap, swipe, type text, and tell which app is in front.",
        status,
        detail,
        button: RowButton {
            label: "Open accessibility settings",
            tip: "Open Android's accessibility settings to turn Sqyre on or off.",
            fix: Fix::AccessibilitySettings,
            enabled: true,
        },
    }
}

fn notifications(enabled: Result<bool, AndroidError>) -> Row {
    let (status, detail) = match enabled {
        Ok(true) => (Status::Granted, None),
        Ok(false) => (
            Status::Needed,
            Some(
                "Without them, a running macro can only be stopped from inside Sqyre.".to_string(),
            ),
        ),
        Err(e) => (Status::Unknown, Some(format!("Could not check: {e}"))),
    };
    Row {
        title: "Notifications",
        summary:
            "Shows Stop and Continue for a running macro on the screen recording notification.",
        status,
        detail,
        button: RowButton {
            label: "Open notification settings",
            tip: "Open Sqyre's notification settings.",
            fix: Fix::NotificationSettings,
            enabled: true,
        },
    }
}
