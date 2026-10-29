use eframe::egui;

/// Prefix for workspace / list-scoped load failures (macro list).
pub(crate) const PREFIX_LOAD_ERROR: &str = "Load error";
/// Prefix for workspace / list-scoped save failures (macro list).
pub(crate) const PREFIX_SAVE_ERROR: &str = "Save error";

/// Trim an arbitrary error to one short line for status/banner text.
///
/// Internal dumps (multi-line serde/schema reports, backend debug strings)
/// still reach the diagnostic log in full; the UI gets the first non-empty
/// line and whitespace-collapsed, so banners stay readable.
pub(crate) fn user_error_detail(e: impl std::fmt::Display) -> String {
    let text = e.to_string();
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("unknown error");
    let collapsed = first.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StatusBanner {
    pub(crate) status: Option<String>,
    pub(crate) status_error: bool,
}

impl StatusBanner {
    pub(crate) fn set_ok(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
        self.status_error = false;
    }

    pub(crate) fn set_err(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
        self.status_error = true;
    }

    pub(crate) fn clear(&mut self) {
        self.status = None;
        self.status_error = false;
    }

    pub(crate) fn is_set(&self) -> bool {
        self.status.is_some()
    }

    /// Paint the current status line, if any.
    pub(crate) fn paint(&self, ui: &mut egui::Ui) {
        let Some(msg) = &self.status else {
            return;
        };
        Self::paint_line(ui, msg, self.status_error);
    }

    /// Paint an ok line, or an error icon whose hover tooltip carries `msg`.
    pub(crate) fn paint_line(ui: &mut egui::Ui, msg: impl AsRef<str>, error: bool) {
        if error {
            crate::widgets::error_icon(ui, msg.as_ref());
        } else {
            ui.add(
                egui::Label::new(egui::RichText::new(msg.as_ref()).color(crate::theme::ok_fg()))
                    .wrap(),
            );
        }
    }

    /// Paint a prefixed error (`"{prefix}: {detail}"`) as an icon + tooltip.
    pub(crate) fn paint_prefixed_error(ui: &mut egui::Ui, prefix: &str, detail: &str) {
        Self::paint_line(ui, format!("{prefix}: {detail}"), true);
    }

    /// Paint a warning advisory as an icon + tooltip.
    pub(crate) fn paint_warn(ui: &mut egui::Ui, msg: impl AsRef<str>) {
        crate::widgets::warn_icon(ui, msg.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_err_clear_rules() {
        let mut banner = StatusBanner::default();
        assert!(!banner.is_set());

        banner.set_ok("Saved x");
        assert_eq!(banner.status.as_deref(), Some("Saved x"));
        assert!(!banner.status_error);
        assert!(banner.is_set());

        banner.set_err("variable \"x\" already exists");
        assert_eq!(
            banner.status.as_deref(),
            Some("variable \"x\" already exists")
        );
        assert!(banner.status_error);

        banner.clear();
        assert!(!banner.is_set());
        assert!(!banner.status_error);
    }

    #[test]
    fn load_save_prefixes() {
        assert_eq!(
            format!("{PREFIX_LOAD_ERROR}: corrupt db"),
            "Load error: corrupt db"
        );
        assert_eq!(
            format!("{PREFIX_SAVE_ERROR}: disk full"),
            "Save error: disk full"
        );
    }

    #[test]
    fn user_error_detail_keeps_long_first_line() {
        let long = "x".repeat(200);
        assert_eq!(user_error_detail(format!("{long}\nsecond line")), long);
    }
}
