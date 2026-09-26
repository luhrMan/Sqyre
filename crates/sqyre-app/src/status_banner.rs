use eframe::egui;

/// Prefix for workspace / list-scoped load failures (macro list).
pub(crate) const PREFIX_LOAD_ERROR: &str = "Load error";
/// Prefix for workspace / list-scoped save failures (macro list).
pub(crate) const PREFIX_SAVE_ERROR: &str = "Save error";

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

    /// Paint an ok/err line with the shared status colors (panel footer or list-scoped).
    pub(crate) fn paint_line(ui: &mut egui::Ui, msg: impl AsRef<str>, error: bool) {
        let color = if error {
            crate::theme::error_fg()
        } else {
            crate::theme::ok_fg()
        };
        ui.colored_label(color, msg.as_ref());
    }

    /// Paint a prefixed error (`"{prefix}: {detail}"`) with error color.
    pub(crate) fn paint_prefixed_error(ui: &mut egui::Ui, prefix: &str, detail: &str) {
        Self::paint_line(ui, format!("{prefix}: {detail}"), true);
    }

    /// Paint a warning advisory with theme warn color.
    pub(crate) fn paint_warn(ui: &mut egui::Ui, msg: impl AsRef<str>) {
        ui.colored_label(crate::theme::warn_fg(), msg.as_ref());
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
}
