//! Macro list panel width persistence: the saved settings fraction drives the
//! panel across restarts, user drags write back to settings, and a stale
//! eframe `PanelState` cannot override the saved split.

mod common;
use common::build_docs_harness;

const PANEL_ID: &str = "macro_list_tags";

fn panel_size(ctx: &egui::Context) -> Option<egui::Vec2> {
    egui::containers::panel::PanelState::load(ctx, egui::Id::new(PANEL_ID))
        .map(|s| s.outer_rect.size())
}

#[test]
fn macro_list_default_opens_at_thirty_percent() {
    let mut h = build_docs_harness([1280.0, 800.0], |_| {});
    h.run_ok();
    let size = panel_size(&h.ctx).expect("panel state after first frame");
    assert!(
        (size.x - 1280.0 * 0.30).abs() < 2.0,
        "default split should open at 30% of the window, got {size:?}"
    );
}

#[test]
fn macro_list_drag_persists_split_to_settings() {
    let mut h = build_docs_harness([1280.0, 800.0], |_| {});
    h.run_ok();
    let size = panel_size(&h.ctx).expect("panel state");
    let edge_x = h.ctx.content_rect().width() - size.x;

    // Drag the panel's left edge ~150px wider.
    h.hover_at(egui::pos2(edge_x + 2.0, 400.0));
    h.drag_at(egui::pos2(edge_x + 2.0, 400.0));
    h.hover_at(egui::pos2(edge_x - 150.0, 400.0));
    h.run_ok();
    h.drop_at(egui::pos2(edge_x - 150.0, 400.0));
    h.run_ok();

    let frac = h.state().docs_settings().macro_list_split;
    let size2 = panel_size(&h.ctx).expect("panel state after drag");
    assert!(
        (frac - size2.x / 1280.0).abs() < 0.02,
        "saved split {frac} should match panel width {size2:?}"
    );

    // The written file carries the same fraction (loaded via the harness's
    // isolated data-dir override).
    let reloaded = sqyre_persist::UserSettings::load_default().unwrap();
    assert!(
        (reloaded.macro_list_split - frac).abs() < 0.001,
        "settings file split {} != in-memory {frac}",
        reloaded.macro_list_split
    );

    // A "restart" (fresh ctx, no PanelState) restores the saved width.
    let mut h2 = build_docs_harness([1280.0, 800.0], |app| {
        app.docs_settings_mut().macro_list_split = frac;
    });
    h2.run_ok();
    let size3 = panel_size(&h2.ctx).expect("panel state after restart");
    assert!(
        (size3.x - size2.x).abs() < 2.0,
        "restart should restore {size2:?}, got {size3:?}"
    );
}

#[test]
fn macro_list_stale_panel_state_yields_to_settings() {
    // A stale eframe-persisted PanelState (e.g. saved at a different window
    // size) must be realigned to the settings fraction on the first frame.
    let mut h = build_docs_harness([1280.0, 800.0], |app| {
        app.docs_settings_mut().macro_list_split = 0.30;
    });
    h.run_ok();

    let right = h.ctx.content_rect().right();
    let stale = egui::containers::panel::PanelState {
        outer_rect: egui::Rect::from_min_max(
            egui::pos2(right - 900.0, 0.0),
            egui::pos2(right, 700.0),
        ),
    };
    h.ctx.data_mut(|d| {
        d.insert_persisted(egui::Id::new(PANEL_ID), stale);
        // Re-arm the one-shot restore the way a fresh app start sees it.
        d.insert_temp(egui::Id::new(PANEL_ID).with("split_restored"), false);
    });
    h.run_ok();

    let size = panel_size(&h.ctx).expect("panel state after realign");
    assert!(
        (size.x - 1280.0 * 0.30).abs() < 2.0,
        "stale 900px state should realign to 30%, got {size:?}"
    );
}
