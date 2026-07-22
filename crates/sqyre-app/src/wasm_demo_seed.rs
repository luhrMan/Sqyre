//! Demo macros and programs for the empty WASM editor (ported from the Go `wasm` seed).

use crate::demo_icons;
use sqyre_domain::{
    blank_action, root_loop, Action, ActionKind, ConditionBlock, ConditionClause,
    CoordinateOutputs, CoordinateRef, DetectionBranch, Macro, MaskShape, MatchMode, MouseButton,
    RepeatMode, ScalarValue, VariableAssignment, WaitTilFoundConfig, PROGRAM_DELIMITER,
};
use sqyre_persist::{
    Database, ProgramCatalog, ProgramCollection, ProgramItem, ProgramMask, ProgramPoint,
    ProgramSearchArea,
};

const DEMO_RESOLUTION: &str = "1920x1080";

struct DemoProgramTheme {
    name: &'static str,
    item_tag: &'static str,
    items: &'static [&'static str],
    points: &'static [&'static str],
    search_areas: &'static [&'static str],
    masks: &'static [&'static str],
}

const DEMO_PROGRAM_THEMES: &[DemoProgramTheme] = &[
    DemoProgramTheme {
        name: "Eldoria Online",
        item_tag: "mmorpg",
        items: &[
            "Health potion stack",
            "Iron ore bundle",
            "Quest log tab",
            "Raid frame slot",
            "Minimap expand",
            "Action bar 7",
        ],
        points: &[
            "NPC merchant interact",
            "Dungeon entrance ping",
            "Spellbook toggle",
            "Flight master icon",
            "Guild roster tab",
            "Accept resurrection",
        ],
        search_areas: &[
            "Bag inventory grid",
            "Party chat window",
            "Minimap frame",
            "Quest tracker panel",
            "Buff bar row",
            "Encounter journal",
        ],
        masks: &[
            "Target frame portrait",
            "Cast bar fill",
            "Combo point pips",
            "Proc glow flash",
            "Raid ready icon",
            "Tooltip anchor",
        ],
    },
    DemoProgramTheme {
        name: "Hex Dominion",
        item_tag: "strategy",
        items: &[
            "Worker chip",
            "Siege workshop slot",
            "Tech tier badge",
            "Radar blip",
            "Alliance banner",
            "Resource ticker",
        ],
        points: &[
            "Command center",
            "Build grid cell",
            "Menu — Multiplayer",
            "Pause overlay resume",
            "Voice chat push-to-talk",
            "End turn button",
        ],
        search_areas: &[
            "Build palette dock",
            "Unit selection ring",
            "Mission briefing panel",
            "Top resource bar",
            "Minimap corner",
            "Combat log strip",
        ],
        masks: &[
            "Selection halo",
            "Ping marker",
            "Health bar strip",
            "Upgrade chevron",
            "Rally flag",
            "Fog of war edge",
        ],
    },
    DemoProgramTheme {
        name: "PixelSmith Studio",
        item_tag: "creative",
        items: &[
            "Adjustment layer",
            "Brush preset small",
            "Swatch group cool",
            "Smart object thumb",
            "Export preset WebP",
            "History snapshot",
        ],
        points: &[
            "Eyedropper sample",
            "Transform handle SE",
            "Layer opacity slider",
            "New artboard",
            "Filter gallery OK",
            "Ruler origin",
        ],
        search_areas: &[
            "Layers panel stack",
            "Tool options bar",
            "Histogram panel",
            "Navigator preview",
            "Save for Web dialog",
            "Color spectrum strip",
        ],
        masks: &[
            "Marquee feather edge",
            "Vignette ellipse",
            "Watermark corner",
            "Lens flare core",
            "Gradient overlay bar",
            "Clipping mask thumb",
        ],
    },
    DemoProgramTheme {
        name: "Nimbus Mail",
        item_tag: "productivity",
        items: &[
            "Unread thread row",
            "Calendar invite chip",
            "Signature block",
            "Promotions label",
            "PDF attachment icon",
            "Snooze menu item",
        ],
        points: &[
            "Compose floating button",
            "Search field clear",
            "Archive toolbar",
            "Star thread toggle",
            "Reply all",
            "Sidebar collapse",
        ],
        search_areas: &[
            "Inbox message list",
            "Reading pane body",
            "Folder sidebar",
            "Meeting scheduler grid",
            "Quick settings drawer",
            "People picker popover",
        ],
        masks: &[
            "Unread dot badge",
            "Priority flag strip",
            "Avatar circle crop",
            "Thread count bubble",
            "Inline image thumb",
            "Meeting join pill",
        ],
    },
    DemoProgramTheme {
        name: "Gem Stack",
        item_tag: "puzzle",
        items: &[
            "Move counter gem",
            "Booster hammer charge",
            "Daily streak flame",
            "Leaderboard badge",
            "Reward video chest",
            "Level star row",
        ],
        points: &[
            "Play next level",
            "Shop cart FAB",
            "Lives hearts row",
            "Pause settings gear",
            "Continue after ad",
            "Home map node",
        ],
        search_areas: &[
            "Puzzle board grid",
            "Booster tray",
            "Score header bar",
            "Victory banner",
            "Settings scroll panel",
            "Daily challenge strip",
        ],
        masks: &[
            "Gem match template",
            "Explosion burst core",
            "Progress bar fill",
            "Coin fly-out trail",
            "Streak multiplier text",
            "Ad close button",
        ],
    },
];

fn join_item(program: &str, item: &str) -> String {
    format!("{program}{PROGRAM_DELIMITER}{item}")
}

fn cref(program: &str, entity: &str) -> CoordinateRef {
    CoordinateRef(format!("{program}{PROGRAM_DELIMITER}{entity}"))
}

fn wait_ms(ms: i64) -> Action {
    let mut a = blank_action("wait").expect("wait");
    if let ActionKind::Wait { time } = &mut a.kind {
        *time = ScalarValue::Int(ms);
    }
    a
}

fn key(key: &str, state: sqyre_domain::PressState) -> Action {
    let mut a = blank_action("key").expect("key");
    if let ActionKind::Key {
        key: k, state: s, ..
    } = &mut a.kind
    {
        *k = key.into();
        *s = state;
    }
    a
}

fn key_tap(key_name: &str) -> Vec<Action> {
    vec![key(key_name, sqyre_domain::PressState::Tap)]
}

fn key_chord(modifiers: &[&str], key_name: &str) -> Vec<Action> {
    let mut out = Vec::new();
    for m in modifiers {
        out.push(key(m, sqyre_domain::PressState::Down));
    }
    out.push(key(key_name, sqyre_domain::PressState::Tap));
    for m in modifiers.iter().rev() {
        out.push(key(m, sqyre_domain::PressState::Up));
    }
    out
}

fn click_left(state: sqyre_domain::PressState) -> Action {
    let mut a = blank_action("click").expect("click");
    if let ActionKind::Click { button, state: s } = &mut a.kind {
        *button = MouseButton::Left;
        *s = state;
    }
    a
}

fn move_to_found(program: &str, smooth: bool) -> Action {
    let mut a = blank_action("move").expect("move");
    if let ActionKind::Move {
        point, smooth: sm, ..
    } = &mut a.kind
    {
        *point = cref(program, "Match center");
        *sm = smooth;
    }
    a
}

fn focus_window(title: &str) -> Action {
    let mut a = blank_action("focuswindow").expect("focuswindow");
    if let ActionKind::FocusWindow { window_title, .. } = &mut a.kind {
        *window_title = title.into();
    }
    a
}

fn set_var(name: &str, value: &str) -> Action {
    let mut a = blank_action("setvariable").expect("setvariable");
    if let ActionKind::SetVariable { assignments } = &mut a.kind {
        *assignments = vec![VariableAssignment::new(
            name,
            ScalarValue::String(value.into()),
        )];
    }
    a
}

fn wait_until(seconds: i32, interval_ms: i32) -> WaitTilFoundConfig {
    WaitTilFoundConfig {
        repeat_mode: RepeatMode::WaitUntilFound,
        wait_til_found_seconds: seconds,
        wait_til_found_interval_ms: interval_ms,
        max_iterations: 0,
    }
}

fn coords(ox: &str, oy: &str) -> CoordinateOutputs {
    CoordinateOutputs {
        output_x_variable: ox.into(),
        output_y_variable: oy.into(),
    }
}

fn detection_branch(
    wait: WaitTilFoundConfig,
    out: CoordinateOutputs,
    subactions: Vec<Action>,
) -> DetectionBranch {
    DetectionBranch {
        wait,
        coords: out,
        run_branch_on_no_find: false,
        order: Default::default(),
        subactions,
    }
}

fn image_search(
    name: &str,
    targets: Vec<String>,
    search_area: CoordinateRef,
    tolerance: f64,
    blur: i32,
    detection: DetectionBranch,
) -> Action {
    let mut a = blank_action("imagesearch").expect("imagesearch");
    if let ActionKind::ImageSearch {
        name: n,
        targets: t,
        search_area: sa,
        tolerance: tol,
        blur: b,
        detection: det,
    } = &mut a.kind
    {
        *n = name.into();
        *t = targets;
        *sa = search_area;
        *tol = tolerance;
        *b = blur;
        *det = detection;
    }
    a
}

fn ocr(
    name: &str,
    target: &str,
    search_area: CoordinateRef,
    output_variable: &str,
    out: CoordinateOutputs,
) -> Action {
    let mut a = blank_action("ocr").expect("ocr");
    if let ActionKind::Ocr {
        name: n,
        target: t,
        search_area: sa,
        output_variable: ov,
        grayscale,
        detection,
        ..
    } = &mut a.kind
    {
        *n = name.into();
        *t = target.into();
        *sa = search_area;
        *ov = output_variable.into();
        *grayscale = true;
        detection.coords = out;
        detection.subactions.clear();
    }
    a
}

fn find_pixel(
    name: &str,
    search_area: CoordinateRef,
    color: &str,
    color_tolerance: i32,
    wait: WaitTilFoundConfig,
    out: CoordinateOutputs,
) -> Action {
    let mut a = blank_action("findpixel").expect("findpixel");
    if let ActionKind::FindPixel {
        name: n,
        search_area: sa,
        target_color,
        color_tolerance: ct,
        detection,
    } = &mut a.kind
    {
        *n = name.into();
        *sa = search_area;
        *target_color = color.into();
        *ct = color_tolerance;
        *detection = DetectionBranch {
            wait,
            coords: out,
            run_branch_on_no_find: false,
            order: Default::default(),
            subactions: Vec::new(),
        };
    }
    a
}

fn loop_named(name: &str, count: i64, subactions: Vec<Action>) -> Action {
    let mut a = blank_action("loop").expect("loop");
    if let ActionKind::Loop {
        name: n,
        count: c,
        subactions: kids,
    } = &mut a.kind
    {
        *n = name.into();
        *c = ScalarValue::Int(count);
        *kids = subactions;
    }
    a
}

fn conditional_contains(name: &str, left: &str, right: &str, subactions: Vec<Action>) -> Action {
    let mut a = blank_action("conditional").expect("conditional");
    if let ActionKind::Conditional {
        condition,
        subactions: kids,
    } = &mut a.kind
    {
        *condition = ConditionBlock {
            name: name.into(),
            match_mode: MatchMode::All,
            clauses: vec![ConditionClause {
                left: ScalarValue::String(left.into()),
                operator: "contains".into(),
                right: ScalarValue::String(right.into()),
            }],
        };
        *kids = subactions;
    }
    a
}

fn macro_with(name: &str, delay: i32, hotkey: Vec<String>, actions: Vec<Action>) -> Macro {
    let mut m = Macro::new(name, delay, hotkey);
    m.root = root_loop(actions);
    m
}

fn theme_accent(tag: &str) -> [u8; 3] {
    match tag {
        "mmorpg" => [196, 140, 48],
        "strategy" => [64, 148, 96],
        "creative" => [72, 148, 210],
        "productivity" => [72, 112, 196],
        "puzzle" => [210, 72, 140],
        _ => [120, 120, 128],
    }
}

fn theme_collection_name(tag: &str) -> &'static str {
    match tag {
        "mmorpg" => "Bag slots",
        "strategy" => "Build grid",
        "creative" => "Layer stack",
        "productivity" => "Thread list",
        "puzzle" => "Board cells",
        _ => "Demo grid",
    }
}

/// Extra `{item}~{variant}` placeholders for items used in demo image-searches.
const DEMO_ITEM_VARIANTS: &[(&str, &str, &[&str])] = &[
    ("Eldoria Online", "Health potion stack", &["glow", "empty"]),
    ("Eldoria Online", "Iron ore bundle", &["stack"]),
    ("Eldoria Online", "Minimap expand", &["hover"]),
    ("Hex Dominion", "Worker chip", &["selected", "damaged"]),
    ("Hex Dominion", "Siege workshop slot", &["busy"]),
    ("PixelSmith Studio", "Export preset WebP", &["hover"]),
    ("PixelSmith Studio", "Adjustment layer", &["mask-on"]),
    ("PixelSmith Studio", "Smart object thumb", &["linked"]),
    ("Nimbus Mail", "Unread thread row", &["selected", "compact"]),
    ("Nimbus Mail", "Calendar invite chip", &["accepted"]),
    ("Nimbus Mail", "PDF attachment icon", &["hover"]),
    ("Gem Stack", "Reward video chest", &["open", "claimed"]),
    ("Gem Stack", "Daily streak flame", &["lit"]),
    ("Gem Stack", "Booster hammer charge", &["ready"]),
];

fn seed_item_variants() {
    for (program, item, variants) in DEMO_ITEM_VARIANTS {
        let accent = DEMO_PROGRAM_THEMES
            .iter()
            .find(|t| t.name == *program)
            .map(|t| theme_accent(t.item_tag))
            .unwrap_or([120, 120, 128]);
        for (i, variant) in variants.iter().enumerate() {
            // Shift accent slightly so variants look distinct from the primary tile.
            let shifted = [
                accent[0].saturating_add((i as u8 + 1).saturating_mul(28)),
                accent[1].saturating_add((i as u8 + 1).saturating_mul(12)),
                accent[2].saturating_sub((i as u8 + 1).saturating_mul(18)),
            ];
            demo_icons::register_item_variant(program, item, variant, shifted, i + 3);
        }
    }
}

/// Build the full demo catalog + macros (same content as the Go WASM seed).
pub fn seed_demo_data() -> (Vec<Macro>, ProgramCatalog) {
    let mut catalog = ProgramCatalog::default();
    catalog.set_resolution_key(DEMO_RESOLUTION);
    demo_icons::clear();

    for (i, theme) in DEMO_PROGRAM_THEMES.iter().enumerate() {
        seed_program_from_theme(&mut catalog, i + 1, 6, theme);
    }
    seed_item_variants();

    let macros = vec![
        macro_with(
            "Eldoria — open bags",
            0,
            vec!["ctrl".into(), "shift".into(), "d".into()],
            demo_macro_eldoria_open_bags(),
        ),
        macro_with(
            "Hex — repeat last build",
            25,
            vec!["f5".into()],
            demo_macro_hex_repeat_build(),
        ),
        macro_with(
            "PixelSmith — export slice",
            50,
            vec!["ctrl".into(), "1".into()],
            demo_macro_pixelsmith_export(),
        ),
        macro_with(
            "Nimbus — focus inbox",
            100,
            Vec::new(),
            demo_macro_nimbus_inbox(),
        ),
        macro_with(
            "Gem — claim daily",
            150,
            vec!["alt".into(), "q".into()],
            demo_macro_gem_daily(),
        ),
        macro_with(
            "Studio — undo chain",
            200,
            Vec::new(),
            demo_macro_studio_undo(),
        ),
        macro_with(
            "Mail — quick archive",
            75,
            vec!["ctrl".into(), "space".into()],
            demo_macro_mail_archive(),
        ),
    ];

    (macros, catalog)
}

/// When both macros and programs are empty, populate with demo data and sync `db`.
/// Returns `true` if seeding ran.
pub fn ensure_demo_if_empty(
    macros: &mut Vec<Macro>,
    catalog: &mut ProgramCatalog,
    db: &mut Database,
) -> bool {
    if !macros.is_empty() || catalog.program_names().next().is_some() {
        return false;
    }
    let (seeded_macros, seeded_catalog) = seed_demo_data();
    *catalog = seeded_catalog;
    *macros = seeded_macros;
    macros.sort_by(|a, b| a.name.cmp(&b.name));
    db.replace_macros(macros.iter().cloned());
    db.set_programs_from_catalog(catalog);
    true
}

fn seed_program_from_theme(
    catalog: &mut ProgramCatalog,
    program_idx: usize,
    n: usize,
    theme: &DemoProgramTheme,
) {
    catalog
        .create_program(theme.name)
        .unwrap_or_else(|e| panic!("create {}: {e}", theme.name));

    catalog
        .upsert_point(
            theme.name,
            ProgramPoint {
                name: "Match center".into(),
                x: ScalarValue::String("${foundX}".into()),
                y: ScalarValue::String("${foundY}".into()),
            },
        )
        .expect("Match center");

    let accent = theme_accent(theme.item_tag);

    for i in 0..n.min(theme.items.len()) {
        catalog
            .upsert_item(
                theme.name,
                ProgramItem {
                    name: theme.items[i].into(),
                    mask: String::new(),
                    stack_max: 1 + (i % 5) as i32,
                    grid_cols: 1 + ((i + program_idx) % 3) as i32,
                    grid_rows: 1 + (i % 2) as i32,
                    tags: vec![theme.item_tag.into(), format!("group-{}", i % 3 + 1)],
                },
            )
            .expect("item");
        demo_icons::register_item(theme.name, theme.items[i], accent, i + program_idx);
    }

    let collection = theme_collection_name(theme.item_tag);
    let search_area = theme.search_areas.first().copied().unwrap_or("Main");
    catalog
        .upsert_collection(
            theme.name,
            ProgramCollection {
                name: collection.into(),
                search_area: search_area.into(),
                rows: 4,
                cols: 6,
            },
        )
        .expect("collection");
    demo_icons::register_collection(theme.name, collection, accent, program_idx);

    for i in 0..n.min(theme.points.len()) {
        catalog
            .upsert_point(
                theme.name,
                ProgramPoint {
                    name: theme.points[i].into(),
                    x: ScalarValue::Int((100 + (i + 1) * 80 + program_idx * 10) as i64),
                    y: ScalarValue::Int((200 + (i + 1) * 60) as i64),
                },
            )
            .expect("point");
    }

    for i in 0..n.min(theme.search_areas.len()) {
        let j = (i + 1) as i64;
        catalog
            .upsert_search_area(
                theme.name,
                ProgramSearchArea {
                    name: theme.search_areas[i].into(),
                    left_x: ScalarValue::Int(10 + j * 5),
                    top_y: ScalarValue::Int(20 + j * 5),
                    right_x: ScalarValue::Int(800 - j * 10),
                    bottom_y: ScalarValue::Int(600 - j * 10),
                },
            )
            .expect("search area");
    }

    for i in 0..n.min(theme.masks.len()) {
        let j = i + 1;
        let shape = if j % 2 == 0 {
            MaskShape::Circle
        } else {
            MaskShape::Rectangle
        };
        catalog
            .upsert_mask(
                theme.name,
                ProgramMask {
                    name: theme.masks[i].into(),
                    shape,
                    center_x: "50".into(),
                    center_y: "50".into(),
                    base: format!("{}", 20 + j * 5),
                    height: format!("{}", 20 + j * 5),
                    radius: format!("{}", 15 + j * 3),
                    inverse: j % 3 == 0,
                },
            )
            .expect("mask");
    }
}

fn demo_macro_eldoria_open_bags() -> Vec<Action> {
    let prog = "Eldoria Online";

    let low_hp = find_pixel(
        "Low HP frame tint",
        cref(prog, "Target frame portrait"),
        "b71c1c",
        42,
        wait_until(2, 120),
        coords("lowHpX", "lowHpY"),
    );

    let sort_ocr = ocr(
        "Bag chrome (sort / stacks)",
        "Sort",
        cref(prog, "Bag inventory grid"),
        "bagPanelOcr",
        coords("sortOcrX", "sortOcrY"),
    );

    let bag_icon = image_search(
        "Hotbar / tray icons",
        vec![
            join_item(prog, "Health potion stack"),
            join_item(prog, "Iron ore bundle"),
            join_item(prog, "Minimap expand"),
        ],
        cref(prog, "Minimap frame"),
        0.15,
        3,
        detection_branch(
            wait_until(5, 0),
            coords("bagIconX", "bagIconY"),
            vec![
                wait_ms(55),
                move_to_found(prog, true),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(100),
                sort_ocr,
                wait_ms(50),
                move_to_found(prog, false),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(80),
            ],
        ),
    );

    let quest_hud = ocr(
        "Quest HUD visible",
        "Quest",
        cref(prog, "Quest tracker panel"),
        "questHudText",
        coords("questOcrX", "questOcrY"),
    );

    let mut out = vec![
        wait_ms(160),
        set_var("lastMacroHint", "eldoria_bags"),
        low_hp,
        wait_ms(40),
    ];
    out.extend(key_tap("1"));
    out.extend([wait_ms(80), bag_icon, wait_ms(70), quest_hud, wait_ms(45)]);
    out.extend(key_tap("b"));
    out.push(wait_ms(50));
    out.extend(key_tap("i"));
    out
}

fn demo_macro_hex_repeat_build() -> Vec<Action> {
    let prog = "Hex Dominion";

    let mut place_body = key_tap("r");
    place_body.push(wait_ms(130));
    let place_loop = loop_named("Stamp repeat builds", 2, place_body);

    let build_hit = image_search(
        "Match build palette icon",
        vec![
            join_item(prog, "Worker chip"),
            join_item(prog, "Siege workshop slot"),
            join_item(prog, "Tech tier badge"),
        ],
        cref(prog, "Build palette dock"),
        0.14,
        2,
        detection_branch(
            wait_until(6, 0),
            coords("buildIconX", "buildIconY"),
            vec![
                wait_ms(50),
                move_to_found(prog, false),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(90),
                place_loop,
            ],
        ),
    );

    let minimap_threat = find_pixel(
        "Red blip on minimap",
        cref(prog, "Minimap corner"),
        "d32f2f",
        48,
        WaitTilFoundConfig::default(),
        coords("minimapPingX", "minimapPingY"),
    );

    let briefing = ocr(
        "Briefing text visible",
        "Mission",
        cref(prog, "Mission briefing panel"),
        "briefingSnippet",
        CoordinateOutputs::defaults(),
    );

    let mut out = vec![
        wait_ms(90),
        build_hit,
        wait_ms(100),
        minimap_threat,
        wait_ms(35),
    ];
    out.extend(key_tap("p"));
    out.extend([wait_ms(80), briefing, wait_ms(60), wait_ms(40)]);
    out
}

fn demo_macro_pixelsmith_export() -> Vec<Action> {
    let prog = "PixelSmith Studio";

    let menu_ocr = ocr(
        "File menu shows Export",
        "Export",
        cref(prog, "Tool options bar"),
        "menuOcrHit",
        coords("menuOcrX", "menuOcrY"),
    );

    let layer_thumb = image_search(
        "Layer / preset thumbnail",
        vec![
            join_item(prog, "Export preset WebP"),
            join_item(prog, "Adjustment layer"),
            join_item(prog, "Smart object thumb"),
        ],
        cref(prog, "Layers panel stack"),
        0.13,
        2,
        detection_branch(
            wait_until(5, 0),
            coords("layerHitX", "layerHitY"),
            vec![
                wait_ms(50),
                move_to_found(prog, true),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(120),
                menu_ocr,
            ],
        ),
    );

    let histogram = find_pixel(
        "Histogram clip warning",
        cref(prog, "Histogram panel"),
        "ff9800",
        40,
        WaitTilFoundConfig::default(),
        coords("histX", "histY"),
    );

    let mut out = vec![
        wait_ms(120),
        layer_thumb,
        wait_ms(90),
        histogram,
        wait_ms(40),
    ];
    out.extend(key_chord(&["ctrl", "shift"], "e"));
    out.extend(key_chord(&["ctrl", "shift"], "l"));
    out
}

fn demo_macro_nimbus_inbox() -> Vec<Action> {
    let prog = "Nimbus Mail";

    let inbox_ocr = ocr(
        "Inbox label in sidebar",
        "Inbox",
        cref(prog, "Folder sidebar"),
        "sidebarOcr",
        coords("inboxOcrX", "inboxOcrY"),
    );

    let read_header_ocr = ocr(
        "Reading pane shows Subject",
        "Subject",
        cref(prog, "Reading pane body"),
        "subjectOcr",
        CoordinateOutputs::defaults(),
    );

    let unread_row = image_search(
        "Unread thread template",
        vec![
            join_item(prog, "Unread thread row"),
            join_item(prog, "Calendar invite chip"),
        ],
        cref(prog, "Inbox message list"),
        0.14,
        2,
        detection_branch(
            wait_until(5, 0),
            coords("rowHitX", "rowHitY"),
            vec![
                wait_ms(45),
                move_to_found(prog, true),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(150),
                read_header_ocr,
                wait_ms(50),
            ],
        ),
    );

    let promo = ocr(
        "Promotions tab visible",
        "Promotions",
        cref(prog, "Folder sidebar"),
        "",
        CoordinateOutputs::defaults(),
    );

    let mut out = vec![
        wait_ms(100),
        focus_window("Nimbus Mail"),
        wait_ms(260),
        inbox_ocr,
        wait_ms(40),
        move_to_found(prog, false),
        click_left(sqyre_domain::PressState::Tap),
        wait_ms(200),
    ];
    out.extend(key_tap("/"));
    out.extend([wait_ms(120), unread_row]);
    out.extend(key_tap("enter"));
    out.extend([wait_ms(100), promo, wait_ms(40)]);
    out.extend(key_tap("g"));
    out.extend(key_tap("p"));
    out.push(wait_ms(50));
    out.extend(key_tap("g"));
    out.extend(key_tap("i"));
    out
}

fn demo_macro_gem_daily() -> Vec<Action> {
    let prog = "Gem Stack";

    let glow = find_pixel(
        "Daily reward glow",
        cref(prog, "Daily challenge strip"),
        "ffc107",
        45,
        wait_until(3, 0),
        coords("glowX", "glowY"),
    );

    let claim_ocr = ocr(
        "Claim / free label",
        "Claim",
        cref(prog, "Victory banner"),
        "claimOcr",
        CoordinateOutputs::defaults(),
    );

    let chest = image_search(
        "Reward chest tile",
        vec![
            join_item(prog, "Reward video chest"),
            join_item(prog, "Daily streak flame"),
            join_item(prog, "Booster hammer charge"),
        ],
        cref(prog, "Puzzle board grid"),
        0.15,
        3,
        detection_branch(
            wait_until(5, 0),
            coords("chestX", "chestY"),
            vec![
                wait_ms(55),
                move_to_found(prog, true),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(120),
                claim_ocr,
                wait_ms(50),
                move_to_found(prog, false),
                click_left(sqyre_domain::PressState::Tap),
            ],
        ),
    );

    let lives = find_pixel(
        "Heart / life icon pink",
        cref(prog, "Score header bar"),
        "e91e63",
        50,
        WaitTilFoundConfig::default(),
        CoordinateOutputs::defaults(),
    );

    let mut out = vec![
        wait_ms(140),
        glow,
        wait_ms(45),
        move_to_found(prog, true),
        click_left(sqyre_domain::PressState::Tap),
        wait_ms(90),
        wait_ms(80),
        chest,
        wait_ms(70),
        lives,
        wait_ms(35),
    ];
    out.extend(key_tap("space"));
    out.extend(key_tap("esc"));
    out
}

fn demo_macro_studio_undo() -> Vec<Action> {
    let prog = "PixelSmith Studio";

    let mut undo_once = key_chord(&["ctrl"], "z");
    undo_once.push(wait_ms(70));

    let layer_title = ocr(
        "Layers panel title",
        "Layer",
        cref(prog, "Layers panel stack"),
        "layerTitleOcr",
        CoordinateOutputs::defaults(),
    );

    let layer_branch = conditional_contains(
        "Layers panel focused",
        "${layerTitleOcr}",
        "Layer",
        vec![wait_ms(55), loop_named("Undo burst", 3, undo_once)],
    );

    let marquee = find_pixel(
        "Selection outline (blue)",
        cref(prog, "Navigator preview"),
        "2196f3",
        38,
        WaitTilFoundConfig::default(),
        coords("selectionX", "selectionY"),
    );

    let mut out = vec![
        wait_ms(100),
        layer_title,
        wait_ms(55),
        layer_branch,
        wait_ms(90),
        marquee,
        wait_ms(40),
    ];
    out.extend(key_chord(&["ctrl"], "d"));
    out
}

fn demo_macro_mail_archive() -> Vec<Action> {
    let prog = "Nimbus Mail";

    let archive_ocr = ocr(
        "Archive control visible",
        "Archive",
        cref(prog, "Reading pane body"),
        "archiveOcr",
        coords("archiveBtnX", "archiveBtnY"),
    );

    let pdf_ocr = ocr(
        "PDF mentioned",
        "PDF",
        cref(prog, "Reading pane body"),
        "",
        CoordinateOutputs::defaults(),
    );

    let clip_hit = image_search(
        "Attachment clip",
        vec![
            join_item(prog, "PDF attachment icon"),
            join_item(prog, "Signature block"),
        ],
        cref(prog, "Reading pane body"),
        0.13,
        2,
        detection_branch(
            wait_until(4, 0),
            coords("clipX", "clipY"),
            vec![
                wait_ms(40),
                move_to_found(prog, true),
                click_left(sqyre_domain::PressState::Tap),
                wait_ms(100),
                pdf_ocr,
                wait_ms(40),
            ],
        ),
    );

    let mut out = vec![
        wait_ms(80),
        focus_window("Nimbus Mail"),
        wait_ms(220),
        archive_ocr,
        wait_ms(45),
        move_to_found(prog, false),
        click_left(sqyre_domain::PressState::Tap),
        wait_ms(120),
    ];
    out.extend(key_tap("#"));
    out.extend([wait_ms(80), clip_hit]);
    out.extend(key_tap("o"));
    out.extend([wait_ms(60), wait_ms(40)]);
    out.extend(key_tap("e"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_demo_data_fills_macros_and_programs() {
        demo_icons::with_exclusive(|| {
            let (macros, catalog) = seed_demo_data();
            assert_eq!(macros.len(), 7);
            assert_eq!(catalog.program_names().count(), DEMO_PROGRAM_THEMES.len());

            let eldoria = catalog.get("Eldoria Online").expect("Eldoria Online");
            assert_eq!(eldoria.items.len(), 6);
            assert!(eldoria.items.contains_key("Health potion stack"));
            assert!(eldoria.collections.contains_key("Bag slots"));
            assert!(
                demo_icons::path_for_item_target("Eldoria Online~Health potion stack").is_some()
            );
            assert_eq!(
                demo_icons::variant_paths_for_target("Eldoria Online~Health potion stack").len(),
                3,
                "primary + glow + empty"
            );
            assert!(demo_icons::contains(&demo_icons::collection_icon_path(
                "Eldoria Online",
                "Bag slots",
            )));

            let m = macros
                .iter()
                .find(|m| m.name == "Eldoria — open bags")
                .expect("Eldoria macro");
            assert!(m.root.children().len() >= 3);
        });
    }

    #[test]
    fn ensure_demo_if_empty_skips_when_populated() {
        demo_icons::with_exclusive(|| {
            let (macros, catalog) = seed_demo_data();
            let mut db = Database::default();
            db.replace_macros(macros.iter().cloned());
            db.set_programs_from_catalog(&catalog);

            let mut macros = macros;
            let mut catalog = catalog;
            let before = macros.len();
            assert!(!ensure_demo_if_empty(&mut macros, &mut catalog, &mut db));
            assert_eq!(macros.len(), before);
        });
    }

    #[test]
    fn ensure_demo_if_empty_seeds_blank_db() {
        demo_icons::with_exclusive(|| {
            let mut macros = Vec::new();
            let mut catalog = ProgramCatalog::default();
            let mut db = Database::default();
            assert!(ensure_demo_if_empty(&mut macros, &mut catalog, &mut db));
            assert_eq!(macros.len(), 7);
            assert_eq!(catalog.program_names().count(), 5);
            assert_eq!(db.macros.len(), 7);
        });
    }
}
