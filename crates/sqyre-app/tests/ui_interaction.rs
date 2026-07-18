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

#[test]
fn settings_checkbox_toggles_highlight_active_action() {
    let mut harness = build_harness(|app| {
        app.open_settings_for_docs();
    });
    harness.run();

    assert!(
        !harness.state().docs_settings().highlight_active_action,
        "docs fixture should start with highlight off"
    );

    harness
        .get_by_label("Highlight the currently executing action")
        .click();
    harness.run();

    assert!(
        harness.state().docs_settings().highlight_active_action,
        "clicking highlight checkbox should enable the setting"
    );
}

#[test]
fn new_macro_button_adds_macro() {
    let mut harness = build_harness(|app| {
        app.open_macro_list_for_docs();
    });
    harness.run();

    let before = harness.state().docs_macro_count();
    assert!(before >= 1, "docs fixture should ship with a demo macro");

    harness.get_by_label("New macro").click();
    harness.run();

    assert_eq!(
        harness.state().docs_macro_count(),
        before + 1,
        "New macro (+) should append a macro"
    );
    let name = harness
        .state()
        .docs_selected_macro_name()
        .expect("selected macro after create");
    assert!(
        name.starts_with("new macro"),
        "created macro should be selected, got {name:?}"
    );
}
