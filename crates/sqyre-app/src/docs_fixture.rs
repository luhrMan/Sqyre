//! Fixed demo data for README / docs screenshots.

use sqyre_domain::{
    blank_action, root_loop, ActionKind, CoordinateOutputs, CoordinateRef, Macro, PressState,
    ScalarValue, PROGRAM_DELIMITER,
};
use sqyre_persist::{Database, ProgramCatalog, ProgramItem, ProgramPoint, ProgramSearchArea};

const DEMO_PROGRAM: &str = "Demo Program";

fn prog_ref(name: &str) -> CoordinateRef {
    CoordinateRef(format!("{DEMO_PROGRAM}{PROGRAM_DELIMITER}{name}"))
}

fn target_ref(item: &str) -> String {
    format!("{DEMO_PROGRAM}{PROGRAM_DELIMITER}{item}")
}

/// Demo program with a point, search area, and image-search item.
pub fn demo_catalog() -> ProgramCatalog {
    let mut catalog = ProgramCatalog::default();
    catalog.set_resolution_key("1920x1080");
    catalog
        .create_program(DEMO_PROGRAM)
        .expect("create Demo Program");
    catalog
        .upsert_point(
            DEMO_PROGRAM,
            ProgramPoint {
                name: "center".into(),
                x: ScalarValue::Int(500),
                y: ScalarValue::Int(300),
            },
        )
        .expect("upsert center");
    catalog
        .upsert_search_area(
            DEMO_PROGRAM,
            ProgramSearchArea {
                name: "Main area".into(),
                left_x: ScalarValue::Int(100),
                top_y: ScalarValue::Int(100),
                right_x: ScalarValue::Int(900),
                bottom_y: ScalarValue::Int(600),
            },
        )
        .expect("upsert Main area");
    catalog
        .upsert_item(
            DEMO_PROGRAM,
            ProgramItem {
                name: "Find button".into(),
                mask: String::new(),
                stack_max: 0,
                grid_cols: 0,
                grid_rows: 0,
                tags: Vec::new(),
            },
        )
        .expect("upsert Find button");
    catalog
}

/// Demo macro with nested image-search and loop branches.
pub fn demo_macro() -> Macro {
    let mut focus = blank_action("focuswindow").expect("focuswindow");
    if let ActionKind::FocusWindow { window_title, .. } = &mut focus.kind {
        *window_title = "Notepad".into();
    }

    let mut move_act = blank_action("move").expect("move");
    if let ActionKind::Move { point, smooth, .. } = &mut move_act.kind {
        *point = prog_ref("center");
        *smooth = true;
    }

    let mut click = blank_action("click").expect("click");
    if let ActionKind::Click { button, state } = &mut click.kind {
        *button = sqyre_domain::MouseButton::Left;
        *state = PressState::Down;
    }

    let mut image_search = blank_action("imagesearch").expect("imagesearch");
    if let ActionKind::ImageSearch {
        name,
        targets,
        search_area,
        tolerance,
        blur,
        detection,
        ..
    } = &mut image_search.kind
    {
        *name = "Find button".into();
        *targets = vec![target_ref("Find button")];
        *search_area = prog_ref("Main area");
        *tolerance = 0.95;
        *blur = 5;
        detection.coords = CoordinateOutputs::defaults();
        detection.subactions = vec![click];
    }

    let mut type_act = blank_action("type").expect("type");
    if let ActionKind::Type { text, delay_ms } = &mut type_act.kind {
        *text = "Hello, Sqyre!".into();
        *delay_ms = 40;
    }

    let mut wait = blank_action("wait").expect("wait");
    if let ActionKind::Wait { time } = &mut wait.kind {
        *time = ScalarValue::Int(500);
    }

    let mut key = blank_action("key").expect("key");
    if let ActionKind::Key { key, state } = &mut key.kind {
        *key = "enter".into();
        *state = PressState::Down;
    }

    let mut loop_act = blank_action("loop").expect("loop");
    if let ActionKind::Loop {
        count, subactions, ..
    } = &mut loop_act.kind
    {
        *count = ScalarValue::Int(3);
        *subactions = vec![key];
    }

    let mut m = Macro::new("Demo Macro", 0, vec![]);
    m.root = root_loop(vec![
        focus,
        move_act,
        image_search,
        type_act,
        wait,
        loop_act,
    ]);
    m
}

/// In-memory database built from the demo macro list and catalog.
pub fn demo_database(macros: &[Macro], catalog: &ProgramCatalog) -> Database {
    let mut db = Database::default();
    db.replace_macros(macros.iter().cloned());
    db.set_programs_from_catalog(catalog);
    db
}
