use eframe::egui;

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

    /// Paint the current status line, if any.
    pub(crate) fn paint(&self, ui: &mut egui::Ui) {
        let Some(msg) = &self.status else {
            return;
        };
        let color = if self.status_error {
            crate::theme::error_fg()
        } else {
            crate::theme::ok_fg()
        };
        ui.colored_label(color, msg);
    }
}
