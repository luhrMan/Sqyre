//! Permissions settings section.

use super::common::*;
use super::*;

impl SettingsUi {
    #[cfg(feature = "native-runtime")]
    pub(super) fn draw_permissions(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, PERMISSIONS_PANEL) {
            return;
        }
        self.permissions.paint(ui, ctx);
    }
}
