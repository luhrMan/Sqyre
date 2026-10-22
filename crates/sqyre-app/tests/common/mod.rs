//! Shared docs / kittest harness helpers.

use egui::os::OperatingSystem;
use egui_kittest::Harness;
use sqyre_app::{theme, SettingsUi, SqyreApp};

pub fn build_docs_harness(
    size: [f32; 2],
    mut setup: impl FnMut(&mut SqyreApp),
) -> Harness<'static, SqyreApp> {
    // Isolate the data dir: app paths (create-macro → db.yaml, mark_site →
    // last_site.txt) would otherwise hit the real ~/.sqyre — both a real-file
    // leak and an ordering flake (db write ENOENTs when the dir isn't there
    // yet). `keep` leaks the unique dir for the rest of the process so the
    // process-global overrides stay valid through `Harness::run`.
    let dir = tempfile::tempdir().unwrap().keep();
    sqyre_persist::set_sqyre_dir_override(Some(dir.clone()));
    sqyre_persist::initialize_directories().unwrap();
    sqyre_capture::set_log_dir(Some(dir));
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
