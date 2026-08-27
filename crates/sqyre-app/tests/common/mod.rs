//! Shared docs / kittest harness helpers.

use egui::os::OperatingSystem;
use egui_kittest::Harness;
use sqyre_app::{theme, SettingsUi, SqyreApp};

pub fn build_docs_harness(
    size: [f32; 2],
    mut setup: impl FnMut(&mut SqyreApp),
) -> Harness<'static, SqyreApp> {
    let mut app = SqyreApp::for_docs();
    setup(&mut app);
    let settings = app.docs_settings().clone();
    Harness::builder()
        .with_size(size)
        .with_os(OperatingSystem::Nix)
        .wgpu()
        .build_eframe(move |cc| {
            SettingsUi::install_fonts(&cc.egui_ctx);
            SettingsUi::apply_appearance(&cc.egui_ctx, &settings);
            theme::apply(&cc.egui_ctx);
            app
        })
}
