//! Data Editor left-pane split persistence: the saved settings fraction drives
//! the list width across restarts, user divider drags write back only when the
//! gesture settles, and idle window resize must not rewrite the store.

mod common;
use common::build_docs_harness;
use egui_kittest::kittest::{NodeT, Queryable};

const DIVIDER_LABEL: &str = "Pane divider — drag, or press Left/Right arrows while focused, to resize the list and editor panes";

#[test]
fn data_editor_default_opens_at_thirty_percent_split() {
    let mut h = build_docs_harness([1280.0, 800.0], |app| {
        app.open_data_editor_programs_for_docs();
    });
    h.run_ok();
    let frac = h.state().docs_settings().ui.data_editor_left_split;
    assert!(
        (frac - sqyre_persist::DEFAULT_DATA_EDITOR_LEFT_FRAC).abs() < 0.001,
        "default data editor split should be 30%, got {frac}"
    );
}

#[test]
fn data_editor_drag_persists_split_to_settings() {
    let mut h = build_docs_harness([1280.0, 800.0], |app| {
        app.docs_settings_mut().ui.data_editor_left_split = 0.30;
        app.open_data_editor_programs_for_docs();
    });
    h.run_ok();

    let divider = h.get_by_label(DIVIDER_LABEL);
    let start = divider.rect().center();
    // Widen the left pane by ~120px.
    let end = egui::pos2(start.x + 120.0, start.y);
    h.hover_at(start);
    h.drag_at(start);
    h.hover_at(end);
    h.run_ok();
    // Mid-drag must not rewrite settings yet (settled-gesture write-back).
    let mid = h.state().docs_settings().ui.data_editor_left_split;
    assert!(
        (mid - 0.30).abs() < 0.001,
        "settings must stay at 0.30 until drag_stopped, got {mid}"
    );
    h.drop_at(end);
    h.run_ok();

    let frac = h.state().docs_settings().ui.data_editor_left_split;
    assert!(
        frac > 0.30 + 0.02,
        "drag should widen the saved split above 0.30, got {frac}"
    );

    let reloaded = sqyre_persist::UserSettings::load_default().unwrap();
    assert!(
        (reloaded.ui.data_editor_left_split - frac).abs() < 0.001,
        "settings file split {} != in-memory {frac}",
        reloaded.ui.data_editor_left_split
    );

    let mut h2 = build_docs_harness([1280.0, 800.0], |app| {
        app.docs_settings_mut().ui.data_editor_left_split = frac;
        app.open_data_editor_programs_for_docs();
    });
    h2.run_ok();
    let restored = h2.state().docs_settings().ui.data_editor_left_split;
    assert!(
        (restored - frac).abs() < 0.001,
        "restart should keep saved split {frac}, got {restored}"
    );
}

#[test]
fn data_editor_window_resize_keeps_saved_split() {
    let mut h = build_docs_harness([1280.0, 800.0], |app| {
        app.docs_settings_mut().ui.data_editor_left_split = 0.30;
        app.open_data_editor_programs_for_docs();
    });
    h.run_ok();

    h.set_size(egui::vec2(900.0, 800.0));
    h.run_ok();

    let frac = h.state().docs_settings().ui.data_editor_left_split;
    assert!(
        (frac - 0.30).abs() < 0.001,
        "window resize must not rewrite the saved split, got {frac}"
    );
}
