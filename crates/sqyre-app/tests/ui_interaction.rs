//! Interaction coverage beyond README screenshot goldens.
//!
//! Uses the same docs fixture + lavapipe path as `docs_screenshots`, but drives
//! AccessKit clicks and asserts app state.

use egui::os::OperatingSystem;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use sqyre_app::{theme, SettingsUi, SqyreApp};

fn build_harness(mut setup: impl FnMut(&mut SqyreApp)) -> Harness<'static, SqyreApp> {
    let mut app = SqyreApp::for_docs();
    setup(&mut app);
    let settings = app.docs_settings().clone();
    Harness::builder()
        .with_size([1000.0, 500.0])
        .with_os(OperatingSystem::Nix)
        .wgpu()
        .build_eframe(move |cc| {
            SettingsUi::install_fonts(&cc.egui_ctx);
            SettingsUi::apply_appearance(&cc.egui_ctx, &settings);
            theme::apply(&cc.egui_ctx);
            app
        })
}

#[test]
fn settings_checkbox_toggles_log_meta_images() {
    let mut harness = build_harness(|app| {
        app.open_settings_for_docs();
    });
    harness.run();

    assert!(
        !harness.state().docs_settings().save_meta_images,
        "docs fixture should start with log meta images off"
    );

    harness.get_by_label("Log Meta Images").click();
    harness.run();

    assert!(
        harness.state().docs_settings().save_meta_images,
        "clicking Log Meta Images should enable the setting"
    );

    harness.get_by_label("Log Meta Images").click();
    harness.run();

    assert!(
        !harness.state().docs_settings().save_meta_images,
        "second click should disable the setting again"
    );
}
