//! Background job receivers and one-shot flags owned by the Data Editor.

use std::path::PathBuf;
use std::sync::mpsc;

#[cfg(feature = "native-runtime")]
use super::running_programs::RunningProgramsRx;

/// Collection-image capture+save job polled each frame.
pub(super) struct CollectionCapturePending {
    pub path: PathBuf,
    /// When set, delete this collection if capture fails (new-collection flow).
    pub rollback_collection: Option<(String, String)>,
    pub rx: mpsc::Receiver<Result<(), String>>,
}

/// ScreenCap / collection / running-program jobs and related one-shot flags.
#[derive(Default)]
pub(crate) struct BackgroundJobs {
    /// After New Point/Search Area: auto-arm record and persist on capture.
    pub save_after_record: bool,
    /// Program rename changed macro or hotkey tags; refresh bindings after paint.
    pub hotkey_refresh: bool,
    /// Background ScreenCap capture+save; polled each frame.
    pub screen_cap_pending: Option<mpsc::Receiver<Result<String, String>>>,
    /// Create a catalog item from the current ScreenCap preview (processed after paint).
    pub screen_cap_new_item: bool,
    /// Background collection image capture+save; polled each frame.
    pub collection_capture_pending: Option<CollectionCapturePending>,
    /// Background open-window fetch for Programs "Add running".
    #[cfg(feature = "native-runtime")]
    pub running_programs_pending: Option<RunningProgramsRx>,
}

impl BackgroundJobs {
    /// Take the one-shot flag set when a program rename rewrote name tags.
    pub(crate) fn take_hotkey_refresh(&mut self) -> bool {
        std::mem::take(&mut self.hotkey_refresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_hotkey_refresh_is_one_shot() {
        let mut jobs = BackgroundJobs {
            hotkey_refresh: true,
            ..Default::default()
        };
        assert!(jobs.take_hotkey_refresh());
        assert!(!jobs.take_hotkey_refresh());
        assert!(!jobs.hotkey_refresh);
    }

    #[test]
    fn default_has_no_pending_jobs() {
        let jobs = BackgroundJobs::default();
        assert!(jobs.screen_cap_pending.is_none());
        assert!(jobs.collection_capture_pending.is_none());
        assert!(!jobs.screen_cap_new_item);
        assert!(!jobs.save_after_record);
        #[cfg(feature = "native-runtime")]
        assert!(jobs.running_programs_pending.is_none());
    }
}
