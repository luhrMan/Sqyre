mod common;
#[path = "common/layout_audit.rs"]
mod layout_audit;

macro_rules! scene_tests {
    ($( $(#[$meta:meta])* $name:ident => $scene:literal; )*) => {
        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                layout_audit::run_scene($scene);
            }
        )*
    };
}

scene_tests! {
    main => "main";
    main_no_list => "main_no_list";
    add_action_picker => "add_action_picker";
    command_palette => "command_palette";

    settings_general => "settings_general";
    settings_macros => "settings_macros";
    settings_sound => "settings_sound";
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    settings_permissions => "settings_permissions";
    settings_data => "settings_data";
    #[cfg(not(target_arch = "wasm32"))]
    settings_updates => "settings_updates";
    settings_appearance => "settings_appearance";

    data_editor_programs => "data_editor_programs";
    data_editor_items => "data_editor_items";
    data_editor_masks => "data_editor_masks";
    data_editor_screencap => "data_editor_screencap";
    data_editor_pixelcheck => "data_editor_pixelcheck";
    data_editor_points => "data_editor_points";
    data_editor_searchareas => "data_editor_searchareas";
    data_editor_collections => "data_editor_collections";
    data_editor_atlases => "data_editor_atlases";
    data_editor_overlay => "data_editor_overlay";
    data_editor_points_form => "data_editor_points_form";

    variables_runtime => "variables_runtime";
    variables_builtins => "variables_builtins";

    edit_loop => "edit_loop";
    edit_while => "edit_while";
    edit_conditional => "edit_conditional";
    edit_imagesearch => "edit_imagesearch";
    edit_ocr => "edit_ocr";
    edit_findpixel => "edit_findpixel";
    edit_foreachrow => "edit_foreachrow";
    edit_foreachcell => "edit_foreachcell";
    edit_wait => "edit_wait";
    edit_pause => "edit_pause";
    edit_move => "edit_move";
    edit_click => "edit_click";
    edit_key => "edit_key";
    edit_type => "edit_type";
    edit_setvariable => "edit_setvariable";
    edit_savevariable => "edit_savevariable";
    edit_focuswindow => "edit_focuswindow";
    edit_runmacro => "edit_runmacro";
    edit_navigatekey => "edit_navigatekey";
    edit_loopjump => "edit_loopjump";
    edit_navigateselect => "edit_navigateselect";

    logs_window => "logs_window";
    yaml_builder => "yaml_builder";
    ai_builder => "ai_builder";
    delay_dialog => "delay_dialog";

    main_stress => "main_stress";
    edit_imagesearch_stress => "edit_imagesearch_stress";
    edit_type_stress => "edit_type_stress";
}

#[test]
fn docs_scene_lists_cover_compiled_sections_tabs_and_action_types() {
    let mut settings = vec!["General", "Macros", "Sound"];
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    settings.push("Permissions");
    settings.push("Data");
    #[cfg(not(target_arch = "wasm32"))]
    settings.push("Updates");
    settings.push("Appearance");
    assert_eq!(
        sqyre_app::SqyreApp::settings_section_labels_for_docs(),
        settings
    );

    assert_eq!(
        sqyre_app::SqyreApp::data_editor_tab_names_for_docs(),
        vec![
            "programs",
            "items",
            "masks",
            "screencap",
            "pixelcheck",
            "points",
            "searchareas",
            "collections",
            "atlases",
            "overlay",
        ]
    );

    let scene_action_keys = [
        "loop",
        "while",
        "conditional",
        "imagesearch",
        "ocr",
        "findpixel",
        "foreachrow",
        "foreachcell",
        "wait",
        "pause",
        "move",
        "click",
        "key",
        "type",
        "setvariable",
        "savevariable",
        "focuswindow",
        "runmacro",
        "navigatekey",
        "loopjump",
        "navigateselect",
    ];
    assert_eq!(scene_action_keys.as_slice(), sqyre_domain::WIRE_TYPE_KEYS);
}
