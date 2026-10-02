//! Interaction coverage beyond README screenshot goldens.
//!
//! Uses the same docs fixture + lavapipe path as `docs_screenshots`, but drives
//! AccessKit clicks and asserts app state.

mod common;

use common::build_docs_harness;
use egui_kittest::kittest::Queryable;

#[test]
fn settings_checkbox_toggles_log_meta_images() {
    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
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
    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
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
    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
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

fn open_first_wait_row_menu(harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>) {
    harness
        .query_all_by_label("Wait")
        .next()
        .expect("demo macro should contain a Wait row")
        .click_secondary();
    harness.run();
}

#[test]
fn tree_row_menu_logs_entry_follows_log_meta_images_setting() {
    let mut harness = build_docs_harness([1000.0, 500.0], |_| {});
    harness.run();
    assert!(
        harness.query_all_by_label("Logs").next().is_none(),
        "no inline log buttons on tree rows"
    );
    open_first_wait_row_menu(&mut harness);
    harness.get_by_label("Edit");
    harness.get_by_label("Delete");
    assert!(
        harness.query_all_by_label("Logs").next().is_none(),
        "Logs menu entry should be hidden when Log Meta Images is off"
    );

    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
        app.docs_settings_mut().save_meta_images = true;
    });
    harness.run();
    open_first_wait_row_menu(&mut harness);
    harness.get_by_label("Logs");
}

#[test]
fn add_action_picker_lists_wait() {
    let mut harness = build_docs_harness([1100.0, 520.0], |app| {
        app.open_add_action_picker();
    });
    harness.run();
    harness.get_by_label("Add Wait");
}

#[test]
fn add_wait_from_picker_increases_tree() {
    let mut harness = build_docs_harness([1100.0, 520.0], |app| {
        app.open_add_action_picker();
    });
    harness.run();
    let before = harness.state().docs_selected_root_child_count();
    assert!(before >= 1, "demo macro should have root children");

    harness.get_by_label("Add Wait").click();
    // Provisional insert opens a pulsing Save; Harness::run never settles.
    harness.run_steps(4);

    assert_eq!(
        harness.state().docs_selected_root_child_count(),
        before + 1,
        "picking Wait should insert a child under the demo root"
    );
}

#[test]
fn run_toolbar_button_is_present() {
    let mut harness = build_docs_harness([1000.0, 500.0], |_| {});
    harness.run();
    harness.get_by_label("Run");
}
