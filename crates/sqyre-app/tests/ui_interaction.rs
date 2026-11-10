//! Interaction coverage beyond README screenshot goldens.
//!
//! Uses the same docs fixture + lavapipe path as `docs_screenshots`, but drives
//! AccessKit clicks and asserts app state.

mod common;

use common::build_docs_harness;
use egui_kittest::kittest::{NodeT, Queryable};

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
        .get_by_role_and_label(eframe::egui::accesskit::Role::Button, "Macros")
        .click();
    harness.run();
    harness
        .get_by_label("Highlight the currently executing action")
        .click();
    harness.run();

    assert!(
        harness.state().docs_settings().highlight_active_action,
        "clicking highlight checkbox should enable the setting"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn settings_checkbox_toggles_os_title_bar() {
    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
        app.open_settings_appearance_for_docs();
    });
    harness.run();

    assert!(
        harness.state().docs_settings().window_decorations,
        "docs fixture should start with OS title bar on"
    );

    harness.get_by_label("OS title bar").click();
    harness.run();

    assert!(
        !harness.state().docs_settings().window_decorations,
        "clicking OS title bar should hide decorations"
    );

    harness.get_by_label("OS title bar").click();
    harness.run();

    assert!(
        harness.state().docs_settings().window_decorations,
        "second click should restore decorations"
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

/// Top-bar chrome floats onto the lower name/tag band under the brand (list
/// toggle on the right) — not a third strip below the chips.
#[test]
fn top_bar_chrome_row_under_brand() {
    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
        app.open_macro_list_for_docs();
    });
    // Settle stack-height memory so the chip band exists before we measure.
    harness.run_steps(4);

    let brand = harness.get_by_label("Sqyre").rect();
    let data_editor = harness.get_by_label("Data Editor").rect();
    let settings = harness.get_by_label("Settings").rect();
    let list_toggle = harness.get_by_label("Hide macro list").rect();
    let bar_rect =
        egui::containers::panel::PanelState::load(&harness.ctx, egui::Id::new("app_top_bar"))
            .map(|s| s.outer_rect)
            .unwrap_or(egui::Rect::NOTHING);
    let bar_h = bar_rect.height();

    assert!(
        data_editor.top() > brand.center().y,
        "Data Editor should sit under the brand (brand={brand:?}, data={data_editor:?})"
    );
    let chrome_mid_y = (data_editor.center().y + settings.center().y) * 0.5;
    assert!(
        (list_toggle.center().y - chrome_mid_y).abs() < data_editor.height(),
        "macro-list toggle should share the tag-row plane (list={list_toggle:?}, chrome_y={chrome_mid_y})"
    );
    assert!(
        list_toggle.left() > settings.right(),
        "macro-list toggle should be on the right of the chrome row"
    );
    // Chrome must sit inside the bar's lower half (chip band), not hang below
    // the bar as a third strip.
    assert!(
        data_editor.bottom() <= bar_rect.bottom() + 1.0,
        "chrome should stay inside the top bar (data={data_editor:?}, bar={bar_rect:?})"
    );
    assert!(
        chrome_mid_y > bar_rect.top() + bar_h * 0.35,
        "chrome should sit on the lower tag band (chrome_y={chrome_mid_y}, bar={bar_rect:?})"
    );
    assert!(
        bar_h < brand.height() + data_editor.height() * 2.4,
        "chrome should not add a third strip (bar_h={bar_h}, brand={brand:?}, data={data_editor:?})"
    );

    // Click each chrome control from a clean top bar (open dialogs steal
    // labels / hit targets for later clicks).
    harness.get_by_label("Data Editor").click();
    harness.run();
    assert!(harness.state().docs_data_editor_open());

    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
        app.open_macro_list_for_docs();
    });
    harness.run();
    harness.get_by_label("Settings").click();
    harness.run();
    assert!(harness.state().docs_settings_open());

    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
        app.open_macro_list_for_docs();
    });
    harness.run();
    assert!(harness.state().docs_macro_list_open());
    harness.get_by_label("Hide macro list").click();
    harness.run();
    assert!(!harness.state().docs_macro_list_open());
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
fn add_action_edit_defaults_affordance_is_visible() {
    let mut harness = build_docs_harness([1100.0, 520.0], |app| {
        app.open_add_action_picker();
    });
    harness.run();
    // Defaults edit must not be right-click-only (icon chrome + tip).
    harness.get_by_label("Edit the selected action's default fields (also right-click a tile)");
}

#[test]
fn delay_dialog_uses_icon_field_labels() {
    let mut harness = build_docs_harness([1000.0, 500.0], |app| {
        app.open_delay_dialog_for_docs();
    });
    harness.run();
    // C3: delay rows use pair_labeled icons, not Body text captions.
    harness.get_by_label("Global");
    harness.get_by_label("Keyboard");
    harness.get_by_label("Mouse");
}

#[test]
fn image_search_edit_shows_output_and_wait_without_advanced() {
    let mut harness = build_docs_harness([1100.0, 720.0], |app| {
        app.open_action_editor_for_docs("imagesearch");
    });
    harness.run_steps(6);
    // Common-path detection fields stay outside collapsed Advanced.
    harness.get_by_label("Output X");
    harness.get_by_label("Output Y");
    harness.get_by_label("Repeat mode");
}

#[test]
fn add_action_picker_focuses_search_on_open() {
    let mut harness = build_docs_harness([1100.0, 520.0], |app| {
        app.open_add_action_picker();
    });
    harness.run();
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("click".into()));
    harness.run_steps(2);
    harness.get_by_label("Add Click");
    assert!(
        harness.query_all_by_label("Add Wait").next().is_none(),
        "typed text should land in the Add Action search field"
    );

    harness.get_by_label("Add Click").click();
    harness.run_steps(4);

    harness.get_by_label("Add Action (Ctrl+A)").click();
    harness.run_steps(4);
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("wait".into()));
    harness.run_steps(2);
    harness.get_by_label("Add Wait");
    assert!(
        harness.query_all_by_label("Add Click").next().is_none(),
        "search focuses again after the picker is reopened"
    );
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
fn data_editor_program_name_tooltip_discloses_elided_text() {
    let mut harness = build_docs_harness([480.0, 360.0], |app| {
        // Elision needs a narrow list pane; pin the split to the minimum
        // instead of relying on the global default.
        app.docs_settings_mut().data_editor_left_split = sqyre_persist::MIN_DATA_EDITOR_LEFT_FRAC;
        app.open_data_editor_programs_for_docs();
    });
    harness.run();
    let (visible, rect) = {
        let node = harness
            .query_all_by(|node| {
                node.label()
                    .as_deref()
                    .is_some_and(|label| label.starts_with("Demo P"))
            })
            .next()
            .expect("program label in Data Editor");
        (
            node.accesskit_node().label().unwrap().to_string(),
            node.rect(),
        )
    };
    assert!(
        visible.ends_with('…'),
        "program label was not elided: {visible}"
    );
    harness.hover_at(rect.center());
    harness.run_steps(45);
    harness.get_by_label("Demo Program");
}

#[test]
fn long_macro_name_tooltip_discloses_elided_text() {
    let full_name = "Daily inventory restock and screenshot backup routine";
    let mut harness = build_docs_harness([480.0, 360.0], |app| {
        app.apply_stress_fixture_for_docs();
        app.open_macro_list_for_docs();
    });
    harness.run();
    // Expand the "inventory" tag group so its rows render. Locate the chevron
    // by node geometry — the panel width (and thus the click point) depends on
    // the persisted split, so a fixed coordinate is fragile.
    // The header label may elide ("invent…"), and the top bar's tag chips can
    // carry the same tag text — match the prefix and require the node to sit
    // below the app_top_bar strip, i.e. inside the macro list.
    let bar_rect =
        egui::containers::panel::PanelState::load(&harness.ctx, egui::Id::new("app_top_bar"))
            .map(|s| s.outer_rect)
            .unwrap_or(egui::Rect::NOTHING);
    let below_bar = |bb: egui::accesskit::Rect| {
        !bar_rect.contains(egui::pos2(
            ((bb.x0 + bb.x1) / 2.0) as f32,
            ((bb.y0 + bb.y1) / 2.0) as f32,
        ))
    };
    let inventory_header = harness
        .query_all_by(|node| {
            node.value()
                .as_deref()
                .is_some_and(|v| v.starts_with("invent"))
                && node.bounding_box().is_some_and(&below_bar)
        })
        .map(|node| node.rect())
        .max_by(|a, b| a.min.x.total_cmp(&b.min.x))
        .expect("inventory tag group header");
    harness
        .query_all_by(|node| {
            matches!(node.role(), egui::accesskit::Role::Unknown)
                && node.bounding_box().is_some_and(|bb| {
                    let row = inventory_header;
                    bb.x0 >= bar_rect.min.x as f64
                        && bb.x1 <= row.min.x as f64
                        && (bb.y0 + bb.y1) / 2.0 > row.min.y as f64
                        && (bb.y0 + bb.y1) / 2.0 < row.max.y as f64
                        && below_bar(bb)
                })
        })
        .next()
        .expect("inventory group chevron")
        .click();
    harness.run();

    let (visible, rect) = {
        let macro_node = harness
            .query_all_by(|node| {
                node.label()
                    .as_deref()
                    .is_some_and(|label| label.starts_with("Daily inventory"))
            })
            .max_by(|a, b| {
                (a.rect().width() * a.rect().height())
                    .total_cmp(&(b.rect().width() * b.rect().height()))
            })
            .expect("long macro name in list");
        (
            macro_node.accesskit_node().label().unwrap().to_string(),
            macro_node.rect(),
        )
    };
    assert!(
        visible
            .lines()
            .next()
            .is_some_and(|line| line.ends_with('…')),
        "macro label was not elided: {visible}"
    );

    harness.hover_at(rect.center());
    harness.run_steps(45);
    harness.get_by_label(&format!("{full_name}\nctrl + shift + f9"));
}

#[test]
fn stress_tree_summary_tooltip_discloses_full_type_text_at_scale_two() {
    let full_text = "Please confirm the purchase after checking the selected inventory count. Save a screenshot of the updated shop dialog, verify the backup completed, and leave a clear note for the next maintenance run.";
    let mut harness = build_docs_harness([640.0, 360.0], |app| {
        app.docs_settings_mut().ui_scale = 2.0;
        app.apply_stress_fixture_for_docs();
        app.expand_all_branches_for_docs();
    });
    harness.run();

    let elided_pill_rect = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.job.text.contains("Please confirm the purchase")
                    && text.galley.elided =>
            {
                Some(text.visual_bounding_rect())
            }
            _ => None,
        })
        .expect("elided Type summary pill");
    harness.hover_at(elided_pill_rect.center());
    harness.run_steps(45);
    harness.get_by_label(&format!("\"{full_text}\""));
}

/// Same size as the `command-palette.png` golden.
const PALETTE_HARNESS_SIZE: [f32; 2] = [1000.0, 560.0];

fn press_release(harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>, at: egui::Pos2) {
    harness.hover_at(at);
    harness.run_steps(2);
    harness.input_mut().events.push(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(1);
    let at = at + egui::vec2(2.0, 1.0);
    harness.hover_at(at);
    harness.input_mut().events.push(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(2);
}

#[test]
fn command_palette_row_responds_to_mouse_click() {
    let mut harness = build_docs_harness([1000.0, 600.0], |_| {});
    harness.run_steps(4);
    harness.get_by_label("Sqyre").click();
    harness.run_steps(4);
    assert!(harness.state().docs_command_palette_open());
    let row_center = harness.get_by_label("Open Data Editor").rect().center();
    press_release(&mut harness, row_center);
    assert!(
        !harness.state().docs_command_palette_open(),
        "clicking a palette row should run it and close the palette"
    );
}

fn open_palette_with_shortcut(harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>) {
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(4);
    assert!(harness.state().docs_command_palette_open());
}

#[test]
fn command_palette_reopened_after_click_outside_responds_to_mouse_click() {
    let mut harness = build_docs_harness(PALETTE_HARNESS_SIZE, |_| {});
    harness.run_steps(4);
    open_palette_with_shortcut(&mut harness);
    press_release(&mut harness, egui::pos2(60.0, 560.0));
    assert!(
        !harness.state().docs_command_palette_open(),
        "clicking outside should dismiss the palette"
    );
    open_palette_with_shortcut(&mut harness);
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("settings".into()));
    harness.run_steps(2);
    let row_center = harness.get_by_label("Open Settings").rect().center();
    press_release(&mut harness, row_center);
    assert!(
        harness.state().docs_settings_open(),
        "reopened palette should run the clicked Open Settings row"
    );
}

#[test]
fn run_toolbar_button_is_present() {
    let mut harness = build_docs_harness([1000.0, 500.0], |_| {});
    harness.run();
    harness.get_by_label("Run");
}

#[test]
fn data_editor_new_program_writes_to_catalog() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_programs_for_docs()
    });
    harness.run();
    assert!(
        harness.state().docs_data_editor_open(),
        "open_data_editor_programs_for_docs should show the editor window"
    );

    let click_new = |harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>| {
        harness.get_by_label("New program").click();
        harness.run();
    };

    click_new(&mut harness);
    assert!(
        harness.state().docs_catalog().get("New Program").is_some(),
        "New should create a catalog program"
    );

    click_new(&mut harness);
    assert!(
        harness
            .state()
            .docs_catalog()
            .get("New Program 2")
            .is_some(),
        "second New should uniquify the name"
    );
}

fn data_editor_rename_via_enter(harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>) {
    harness.get_by_label("Demo Program").click();
    harness.run();

    let name = harness
        .query_all_by_role(eframe::egui::accesskit::Role::TextInput)
        .find(|n| n.value().as_deref() == Some("Demo Program"))
        .expect("program name field");
    name.focus();
    name.type_text(" Renamed");
    harness.run();

    harness.key_press(egui::Key::Enter);
    harness.run();

    assert!(
        harness
            .state()
            .docs_catalog()
            .get("Demo Program Renamed")
            .is_some(),
        "Enter should save the dirty form like the Save button"
    );
}

#[test]
fn data_editor_enter_saves_dirty_program_form() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_programs_for_docs()
    });
    harness.run();
    assert!(
        harness.state().docs_catalog().get("Demo Program").is_some(),
        "docs fixture should ship with Demo Program"
    );

    data_editor_rename_via_enter(&mut harness);
}

#[test]
fn data_editor_enter_saves_after_foreground_dialog_closed() {
    let mut harness = build_docs_harness(PALETTE_HARNESS_SIZE, |app| {
        app.open_data_editor_programs_for_docs()
    });
    harness.run();

    // Foreground-order surfaces (palette, confirm, modal pickers) keep their
    // layer in egui's area order after closing; Enter must still reach Save.
    open_palette_with_shortcut(&mut harness);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(4);
    assert!(!harness.state().docs_command_palette_open());

    data_editor_rename_via_enter(&mut harness);
}

#[test]
fn data_editor_header_search_filters_and_preserves_query() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_tab_for_docs("points");
    });
    harness.run();

    // Selected-program list: the demo point row, no program headers, no pane search.
    harness.get_by_label("center");
    assert!(
        harness.query_by_label("Search Points…").is_none(),
        "search field is hidden until the magnifier is clicked"
    );

    harness.get_by_label("Search this tab's list").click();
    harness.run();
    harness.get_by_label("Search Points…").type_text("zzz");
    harness.run();
    harness.get_by_label("No matching points");

    // Hiding suspends the filter but keeps the query text.
    harness.get_by_label("Hide search").click();
    harness.run();
    assert!(harness.query_by_label("No matching points").is_none());
    harness.get_by_label("center");

    harness.get_by_label("Search this tab's list").click();
    harness.run();
    let field = harness.get_by_label("Search Points…");
    assert_eq!(
        field.value().as_deref(),
        Some("zzz"),
        "hidden search keeps its query"
    );
    harness.get_by_label("No matching points");
}

#[test]
fn data_editor_scope_all_programs_restores_group_headers() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_tab_for_docs("points");
    });
    harness.run();
    // Flat selected-program list: no program group header.
    harness.get_by_label("center");
    assert!(
        harness.query_by_label("Demo Program").is_none(),
        "program name only appears in the selector badge (with count)"
    );

    // The scope menu is part of the expanded search.
    harness.get_by_label("Search this tab's list").click();
    harness.run();
    harness.get_by_label("List scope").click();
    harness.run();
    harness.get_by_label("All programs").click();
    harness.run();

    // Grouped view: the point row sits under its program header again.
    harness.get_by_label("Demo Program");
    harness.get_by_label("center");
}

/// All-programs expand/collapse-all must open groups when collapsed and close
/// them when expanded (and flip its tip/glyph with that state).
#[test]
fn data_editor_all_programs_expand_collapse_all() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_tab_for_docs("points");
    });
    harness.run();
    harness.get_by_label("Search this tab's list").click();
    harness.run();
    harness.get_by_label("List scope").click();
    harness.run();
    harness.get_by_label("All programs").click();
    harness.run();

    // Selection scroll opens the demo group, so the toggle starts as Collapse.
    harness.get_by_label("Collapse all").click();
    harness.run();
    harness.get_by_label("Expand all");
    assert!(
        harness.query_by_label("center").is_none(),
        "collapse-all should hide entity rows under closed groups"
    );

    harness.get_by_label("Expand all").click();
    harness.run();
    harness.get_by_label("Collapse all");
    harness.get_by_label("center");
}

/// The header search and program selector share one toggle cluster: both
/// collapse to adjacent icons, expanding one replaces the other's pair, and
/// the expanded controls open to the right of both icons.
#[test]
fn data_editor_search_and_program_pairs_swap() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_tab_for_docs("points");
    });
    harness.run();
    use eframe::egui::accesskit::Role;
    let combo = |h: &egui_kittest::Harness<'_, sqyre_app::SqyreApp>| {
        h.query_all_by_role(Role::ComboBox).next().is_some()
    };

    // Collapsed: magnifier + program icon only — no field, no combo.
    harness.get_by_label("Search this tab's list");
    harness.get_by_label("Program: Demo Program");
    assert!(harness.query_by_label("Search Points…").is_none());
    assert!(
        !combo(&harness),
        "program combo is part of the expanded pair"
    );

    // Expanding the program pair shows the combo; the magnifier stays as the
    // collapsed form of the search pair.
    harness.get_by_label("Program: Demo Program").click();
    harness.run();
    assert!(combo(&harness));

    // Opening search replaces the program pair — but the program toggle stays
    // adjacent to the magnifier, left of the expanded field.
    harness.get_by_label("Search this tab's list").click();
    harness.run();
    harness.get_by_label("Search Points…");
    assert!(!combo(&harness), "search pair replaced the program pair");
    let program_x = harness.get_by_label("Program: Demo Program").rect().min.x;
    let field_x = harness.get_by_label("Search Points…").rect().min.x;
    assert!(
        program_x < field_x,
        "program toggle must stay adjacent to the magnifier ({program_x} !< {field_x})"
    );

    // Closing search restores both collapsed icons.
    harness.get_by_label("Hide search").click();
    harness.run();
    harness.get_by_label("Program: Demo Program");
    assert!(harness.query_by_label("Search Points…").is_none());
}

/// Program selector tip must attach once — IconButton already tips, so a
/// second `on_hover_text` stacked a duplicate popup.
#[test]
fn data_editor_program_selector_shows_single_hover_tip() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_tab_for_docs("points");
    });
    harness.run();
    harness
        .ctx
        .all_styles_mut(|s| s.interaction.tooltip_delay = 0.0);
    let tip = "Program: Demo Program";
    let rect = harness.get_by_label(tip).rect();
    harness.hover_at(rect.center());
    harness.run_steps(45);
    let tip_galleys = harness
        .output()
        .shapes
        .iter()
        .filter(|clipped| match &clipped.shape {
            egui::Shape::Text(t) => t.galley.text() == tip,
            _ => false,
        })
        .count();
    assert_eq!(
        tip_galleys, 1,
        "program selector must paint one hover tip, got {tip_galleys}"
    );
}

#[test]
fn data_editor_items_sort_menu_hosts_tag_priority() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_tab_for_docs("items");
    });
    harness.run();
    assert!(
        harness.query_by_label("Sort items").is_none(),
        "sort icon is part of the collapsed search cluster"
    );

    harness.get_by_label("Search this tab's list").click();
    harness.run();
    harness.get_by_label("Sort items").click();
    harness.run();
    // "Tags" as a label also exists on the item form — the radio is a RadioButton.
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::RadioButton, "Tags")
        .click();
    harness.run();

    // Picking Tags keeps the popup open so the priority chips can be edited.
    harness.get_by_label("Tag priority");
    assert_eq!(
        harness.state().docs_settings().data_editor_items_sort,
        sqyre_domain::CatalogItemSort::Tags,
        "choosing Tags should persist to settings"
    );

    let priority = harness.get_by_label("Priority tag…");
    priority.focus();
    priority.type_text("rare");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        harness
            .state()
            .docs_settings()
            .data_editor_items_tag_priority,
        vec!["rare".to_string()],
        "tag priority chips should persist to settings"
    );
    let reloaded = sqyre_persist::UserSettings::load_default().unwrap();
    assert_eq!(
        reloaded.data_editor_items_sort,
        sqyre_domain::CatalogItemSort::Tags
    );
    assert_eq!(
        reloaded.data_editor_items_tag_priority,
        vec!["rare".to_string()]
    );
}

#[test]
fn data_editor_items_sort_restores_from_settings() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.docs_settings_mut().data_editor_items_sort = sqyre_domain::CatalogItemSort::Tags;
        app.docs_settings_mut().data_editor_items_tag_priority = vec!["heal".into()];
        app.open_data_editor_tab_for_docs("items");
    });
    harness.run();
    harness.get_by_label("Search this tab's list").click();
    harness.run();
    harness.get_by_label("Sort items").click();
    harness.run();
    harness.get_by_label("Tag priority");
    assert_eq!(
        harness.state().docs_settings().data_editor_items_sort,
        sqyre_domain::CatalogItemSort::Tags
    );
    assert_eq!(
        harness
            .state()
            .docs_settings()
            .data_editor_items_tag_priority,
        vec!["heal".to_string()]
    );
    // Idle frames must not rewrite the saved priority.
    harness.run_steps(3);
    assert_eq!(
        harness
            .state()
            .docs_settings()
            .data_editor_items_tag_priority,
        vec!["heal".to_string()]
    );
}

fn ui_scale_appearance_harness() -> egui_kittest::Harness<'static, sqyre_app::SqyreApp> {
    let mut harness = build_docs_harness([1280.0, 720.0], |app| {
        app.docs_settings_mut().ui_scale = 1.4;
        app.open_settings_appearance_for_docs();
    });
    harness.run_steps(3);
    harness
}

fn ui_scale_field_rect(harness: &egui_kittest::Harness<'_, sqyre_app::SqyreApp>) -> egui::Rect {
    harness
        .query_all_by(|node| {
            node.label().as_deref() == Some("1.4") || node.value().as_deref() == Some("1.4")
        })
        .map(|node| node.rect())
        .max_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
        .expect("UI scale field")
}

fn ui_scale_stepper_up(harness: &egui_kittest::Harness<'_, sqyre_app::SqyreApp>) -> egui::Pos2 {
    let field = ui_scale_field_rect(harness);
    egui::Rect::from_min_max(
        egui::pos2(field.right() - 13.0, field.top()),
        egui::pos2(field.right() - 1.0, field.center().y),
    )
    .center()
}

fn run_at(harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>, time: f64) {
    harness.input_mut().time = Some(time);
    harness.run_steps(1);
}

fn primary_button(
    harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>,
    pos: egui::Pos2,
    pressed: bool,
) {
    harness.input_mut().events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

fn assert_ui_scale_and_zoom(
    harness: &egui_kittest::Harness<'_, sqyre_app::SqyreApp>,
    expected_scale: f32,
    expected_zoom: f32,
) {
    assert!(
        (harness.state().docs_settings().ui_scale - expected_scale).abs() < 0.001,
        "expected UI scale {expected_scale}, got {}",
        harness.state().docs_settings().ui_scale
    );
    assert!(
        (harness.ctx.zoom_factor() - expected_zoom).abs() < 0.001,
        "expected zoom {expected_zoom}, got {}",
        harness.ctx.zoom_factor()
    );
}

#[test]
fn ui_scale_zoom_waits_for_pointer_release_after_far_stepper_drag() {
    let mut harness = ui_scale_appearance_harness();
    assert_ui_scale_and_zoom(&harness, 1.4, 1.4);

    let dt = 1.0 / 60.0;
    let mut time = harness.ctx.input(|input| input.time);
    let field = ui_scale_field_rect(&harness);
    let up = egui::pos2(field.right() - 7.0, field.top() + 0.001);
    harness.hover_at(up);
    time += dt;
    run_at(&mut harness, time);
    primary_button(&mut harness, up, true);
    time += dt;
    run_at(&mut harness, time);
    assert_ui_scale_and_zoom(&harness, 1.5, 1.4);

    let outside = up + egui::vec2(500.0, 0.0);
    for offset in [100.0, 200.0, 300.0, 400.0, 500.0] {
        let pos = up + egui::vec2(offset, 0.0);
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(pos));
        time += dt;
        run_at(&mut harness, time);
        assert_ui_scale_and_zoom(&harness, 1.5, 1.4);
        assert!(harness.ctx.input(|input| input.pointer.primary_down()));
    }
    for _ in 0..55 {
        time += dt;
        run_at(&mut harness, time);
        assert_ui_scale_and_zoom(&harness, 1.5, 1.4);
        assert!(harness.ctx.input(|input| input.pointer.primary_down()));
    }

    primary_button(&mut harness, outside, false);
    time += dt;
    run_at(&mut harness, time);
    time += dt;
    run_at(&mut harness, time);
    assert_ui_scale_and_zoom(&harness, 1.5, 1.5);
    for _ in 0..3 {
        time += dt;
        run_at(&mut harness, time);
        assert_ui_scale_and_zoom(&harness, 1.5, 1.5);
    }
}

#[test]
fn ui_scale_stepper_diagonal_drag_does_not_drag_the_field() {
    let mut harness = ui_scale_appearance_harness();
    assert_ui_scale_and_zoom(&harness, 1.4, 1.4);

    let dt = 1.0 / 60.0;
    let mut time = harness.ctx.input(|input| input.time);
    let up = ui_scale_stepper_up(&harness);
    harness.hover_at(up);
    time += dt;
    run_at(&mut harness, time);
    primary_button(&mut harness, up, true);
    time += dt;
    run_at(&mut harness, time);
    assert_ui_scale_and_zoom(&harness, 1.5, 1.4);

    for offset in [10.0, 20.0, 30.0, 40.0] {
        let pos = up + egui::vec2(offset, offset);
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(pos));
        time += dt;
        run_at(&mut harness, time);
        assert_ui_scale_and_zoom(&harness, 1.5, 1.4);
    }
    for _ in 0..60 {
        time += dt;
        run_at(&mut harness, time);
        assert_ui_scale_and_zoom(&harness, 1.5, 1.4);
    }

    let outside = up + egui::vec2(40.0, 40.0);
    primary_button(&mut harness, outside, false);
    time += dt;
    run_at(&mut harness, time);
    time += dt;
    run_at(&mut harness, time);
    assert_ui_scale_and_zoom(&harness, 1.5, 1.5);
}

#[test]
fn ui_scale_zoom_waits_for_release_during_inside_repeat() {
    let mut harness = ui_scale_appearance_harness();
    assert_ui_scale_and_zoom(&harness, 1.4, 1.4);

    let dt = 1.0 / 60.0;
    let mut time = harness.ctx.input(|input| input.time);
    let up = ui_scale_stepper_up(&harness);
    harness.hover_at(up);
    time += dt;
    run_at(&mut harness, time);
    primary_button(&mut harness, up, true);
    time += dt;
    run_at(&mut harness, time);
    assert_ui_scale_and_zoom(&harness, 1.5, 1.4);

    for _ in 0..48 {
        time += dt;
        run_at(&mut harness, time);
        assert!(
            (harness.ctx.zoom_factor() - 1.4).abs() < 0.001,
            "zoom must not change while the stepper is held"
        );
    }
    let final_scale = harness.state().docs_settings().ui_scale;
    assert!(final_scale > 1.5);

    primary_button(&mut harness, up, false);
    time += dt;
    run_at(&mut harness, time);
    time += dt;
    run_at(&mut harness, time);
    assert_ui_scale_and_zoom(&harness, final_scale, final_scale);
}

/// Ctrl+`+`/`-`/`0` edit the persisted `ui_scale` (and apply it immediately),
/// not just egui's transient zoom factor.
#[test]
fn ctrl_zoom_keys_edit_ui_scale_setting() {
    let mut harness = build_docs_harness([1000.0, 500.0], |_| {});
    harness.run_steps(4);
    assert_ui_scale_and_zoom(&harness, 1.0, 1.0);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Plus);
    harness.run_steps(2);
    assert_ui_scale_and_zoom(&harness, 1.1, 1.1);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Minus);
    harness.run_steps(2);
    assert_ui_scale_and_zoom(&harness, 1.0, 1.0);

    // Ctrl+`=` is the no-shift alias for zoom in.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Equals);
    harness.run_steps(2);
    assert_ui_scale_and_zoom(&harness, 1.1, 1.1);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Num0);
    harness.run_steps(2);
    assert_ui_scale_and_zoom(&harness, 1.0, 1.0);
}

/// Zoom keys stop at the persisted UI scale bounds instead of egui's
/// built-in 0.2–5.0 range.
#[test]
fn ctrl_zoom_keys_clamp_to_ui_scale_range() {
    let mut harness = build_docs_harness([1000.0, 500.0], |_| {});
    harness.run_steps(4);

    for _ in 0..6 {
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Minus);
        harness.run_steps(2);
    }
    assert_ui_scale_and_zoom(&harness, 0.5, 0.5);

    for _ in 0..21 {
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Plus);
        harness.run_steps(2);
    }
    assert_ui_scale_and_zoom(&harness, 2.5, 2.5);
}

#[test]
fn view_tip_hugs_narrow_content() {
    let mut harness = build_docs_harness([1200.0, 700.0], |app| {
        app.expand_all_branches_for_docs();
    });
    harness.run();
    harness
        .ctx
        .all_styles_mut(|s| s.interaction.tooltip_delay = 0.0);
    let row = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.job.text.contains("Find button") => {
                Some(t.visual_bounding_rect())
            }
            _ => None,
        })
        .expect("image search row text");
    harness.hover_at(row.center());
    harness.run_steps(45);
    // The tip frame is a bordered popup rect near the row; it must hug its
    // content (a single-target Image Search tip ~200px) rather than stretch
    // toward the 0.5*screen cap (~600px).
    let tip_w = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(r)
                if r.rect.width() > 150.0
                    && r.rect.height() > 60.0
                    && r.rect.top() > row.bottom() =>
            {
                Some(r.rect.width())
            }
            _ => None,
        })
        .fold(0.0_f32, f32::max);
    assert!(tip_w > 150.0, "no view tip rendered (widest rect {tip_w})");
    assert!(
        tip_w < 400.0,
        "view tip stretched to {tip_w} despite narrow content"
    );
}

fn delete_confirm_visible(harness: &egui_kittest::Harness<'_, sqyre_app::SqyreApp>) -> bool {
    // Confirm body copy is not an AccessKit label; Cancel exists only on that dialog.
    harness.query_all_by_label("Cancel").next().is_some()
}

#[test]
fn delete_key_removes_selected_macro_action() {
    let mut harness = build_docs_harness([1100.0, 640.0], |_| {});
    harness.run();
    let before = harness.state().docs_selected_root_child_count();
    assert!(before >= 2, "demo macro should have root actions");

    harness.get_by_label("Mouse Move").click();
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();

    assert_eq!(
        harness.state().docs_selected_root_child_count(),
        before - 1,
        "Delete should remove the selected action"
    );
    assert!(
        harness.query_all_by_label("Mouse Move").next().is_none(),
        "Mouse Move row should be gone"
    );
}

#[test]
fn delete_key_confirms_removal_of_selected_model() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_programs_for_docs()
    });
    harness.run();
    let actions_before = harness.state().docs_selected_root_child_count();
    assert!(
        harness.state().docs_catalog().get("Demo Program").is_some(),
        "docs fixture should ship with Demo Program"
    );

    harness.get_by_label("Demo Program").click();
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();

    assert!(
        delete_confirm_visible(&harness),
        "Delete should open the same remove confirm as the model remove button"
    );
    assert_eq!(
        harness.state().docs_selected_root_child_count(),
        actions_before,
        "model Delete should not also remove the selected macro action"
    );
    assert!(
        harness.state().docs_catalog().get("Demo Program").is_some(),
        "confirm should still be pending"
    );

    harness.key_press(egui::Key::Enter);
    harness.run();

    assert!(
        harness.state().docs_catalog().get("Demo Program").is_none(),
        "confirming should delete the selected program"
    );
    assert!(
        !delete_confirm_visible(&harness),
        "confirm should close after Delete"
    );
}

#[test]
fn delete_key_in_model_name_field_does_not_remove_model() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_programs_for_docs()
    });
    harness.run();
    harness.get_by_label("Demo Program").click();
    harness.run();

    let name = harness
        .query_all_by_role(eframe::egui::accesskit::Role::TextInput)
        .find(|n| n.value().as_deref() == Some("Demo Program"))
        .expect("program name field");
    name.focus();
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();

    assert!(
        !delete_confirm_visible(&harness),
        "Delete inside the name field should edit text, not remove the model"
    );
    assert!(
        harness.state().docs_catalog().get("Demo Program").is_some(),
        "unsaved name edit must not delete the program"
    );
}

#[test]
fn masks_tab_opens_for_selected_program() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_programs_for_docs()
    });
    harness.run();
    harness.get_by_label("Demo Program").click();
    harness.run();

    assert_ne!(
        harness.state().docs_data_editor_tab(),
        "Masks",
        "Masks tab should start inactive on Programs"
    );

    harness.get_by_label("Masks").click();
    harness.run();

    assert_eq!(
        harness.state().docs_data_editor_tab(),
        "Masks",
        "Masks should switch the Data Editor tab for the selected program"
    );
}

#[test]
fn open_masks_from_command_palette() {
    let mut harness = build_docs_harness([1100.0, 640.0], |app| {
        app.open_data_editor_programs_for_docs();
        app.open_command_palette_for_docs();
    });
    harness.run();
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("masks".into()));
    harness.run_steps(2);
    harness.get_by_label("Open Masks").click();
    harness.run_steps(4);

    assert!(
        harness.state().docs_data_editor_open(),
        "Open Masks should keep/open the Data Editor"
    );
    assert_eq!(
        harness.state().docs_data_editor_tab(),
        "Masks",
        "Open Masks should land on the Masks tab"
    );
}
