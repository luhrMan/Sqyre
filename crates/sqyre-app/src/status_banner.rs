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
}
