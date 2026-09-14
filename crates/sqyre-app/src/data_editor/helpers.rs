//! Shared parsing / naming helpers for the data editor.

use sqyre_persist::{ProgramCatalog, TEMPORARY_PROGRAM};
use web_time::{SystemTime, UNIX_EPOCH};

/// Recording scratch program — omitted from Data Editor lists and selectors.
pub(crate) fn is_editor_listed_program(name: &str) -> bool {
    name != TEMPORARY_PROGRAM
}

pub(crate) fn editor_program_names(catalog: &ProgramCatalog) -> impl Iterator<Item = &String> {
    catalog
        .program_names()
        .filter(|n| is_editor_listed_program(n))
}

pub(crate) fn new_overlay_button_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("btn-{ms}")
}

pub(crate) fn rgba_color(c: [u8; 4]) -> eframe::egui::Color32 {
    crate::theme::rgba(c)
}

/// `#rrggbb` for persist, or empty when it matches the theme default.
pub(crate) fn overlay_hex_or_empty(c: eframe::egui::Color32, default_hex: &str) -> String {
    let hex = sqyre_domain::format_hex_color([c.r(), c.g(), c.b(), 255]);
    if hex.eq_ignore_ascii_case(default_hex) {
        String::new()
    } else {
        hex
    }
}

pub(crate) fn parse_i32(s: &str) -> Option<i32> {
    s.trim().parse().ok()
}

pub(crate) fn unique_name(base: &str, exists: impl Fn(&str) -> bool) -> String {
    if !exists(base) {
        return base.to_string();
    }
    for i in 2..10_000 {
        let candidate = format!("{base} {i}");
        if !exists(&candidate) {
            return candidate;
        }
    }
    format!("{base} {}", uuid_simple())
}

/// Sorted unique tags across items in a program.
pub(crate) fn collect_program_item_tags(catalog: &ProgramCatalog, program: &str) -> Vec<String> {
    crate::macro_meta::unique_sorted(
        catalog
            .get(program)
            .map(|p| {
                p.items
                    .values()
                    .flat_map(|it| it.tags.iter().cloned())
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// Sorted unique tags across all editor-listed programs.
pub(crate) fn collect_all_item_tags(catalog: &ProgramCatalog) -> Vec<String> {
    crate::macro_meta::unique_sorted(
        editor_program_names(catalog)
            .filter_map(|n| catalog.get(n))
            .flat_map(|p| p.items.values().flat_map(|it| it.tags.iter().cloned()))
            .collect(),
    )
}

pub(crate) fn uuid_simple() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{n}")
}

/// Parse a form coordinate as a literal number suitable for live preview capture.
/// Returns `None` for `${var}` refs and other non-numeric expressions.
pub(crate) fn form_coord_literal(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    if sqyre_varref::contains(s) {
        return None;
    }
    if let Ok(i) = s.parse::<i32>() {
        return Some(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Some(f as i32);
    }
    None
}

/// Live layout for record→relative conversion. Prefers capture/X11 over a stale
/// catalog snapshot so slot assignment matches the rubber-band cover.
pub(crate) fn live_monitor_rects(catalog: &ProgramCatalog) -> Vec<sqyre_persist::MonitorRect> {
    #[cfg(feature = "native-runtime")]
    {
        let live: Vec<sqyre_persist::MonitorRect> = sqyre_capture::preferred_monitor_rects()
            .into_iter()
            .map(|r| (r.x, r.y, r.w, r.h))
            .collect();
        if !live.is_empty() {
            let cached = catalog.monitor_rects();
            if cached.is_empty() || live.len() >= cached.len() {
                return live;
            }
        }
    }
    catalog.monitor_rects().to_vec()
}

/// Prefer live rects, then persist them on the catalog when they are at least as rich.
pub(crate) fn sync_live_monitor_rects(
    catalog: &mut ProgramCatalog,
) -> Vec<sqyre_persist::MonitorRect> {
    let live = live_monitor_rects(catalog);
    if !live.is_empty() {
        let cached = catalog.monitor_rects();
        if cached.is_empty() || live.len() >= cached.len() {
            catalog.set_monitor_rects(live.clone());
        }
    }
    live
}

/// Origin of a 1-based monitor slot. Catalog layout first so this matches
/// [`sqyre_persist::ProgramCatalog::resolve_search_area`]; live capture only
/// when the catalog snapshot is empty.
pub(crate) fn monitor_origin(catalog: &ProgramCatalog, monitor: u32) -> (i32, i32) {
    let slot = monitor.max(1) as usize;
    let cached = catalog.monitor_rects();
    let rects = if cached.is_empty() {
        live_monitor_rects(catalog)
    } else {
        cached.to_vec()
    };
    rects
        .get(slot - 1)
        .map(|&(ox, oy, _, _)| (ox, oy))
        .unwrap_or((0, 0))
}

/// Relative form literals → absolute desktop coords for capture preview.
pub(crate) fn form_absolute_xy(
    catalog: &ProgramCatalog,
    monitor: u32,
    x: Option<i32>,
    y: Option<i32>,
) -> (Option<i32>, Option<i32>) {
    let (ox, oy) = monitor_origin(catalog, monitor);
    (x.map(|v| ox + v), y.map(|v| oy + v))
}

pub(crate) fn form_absolute_area(
    catalog: &ProgramCatalog,
    monitor: u32,
    lx: Option<i32>,
    ty: Option<i32>,
    rx: Option<i32>,
    by: Option<i32>,
) -> (Option<i32>, Option<i32>, Option<i32>, Option<i32>) {
    let (ox, oy) = monitor_origin(catalog, monitor);
    (
        lx.map(|v| ox + v),
        ty.map(|v| oy + v),
        rx.map(|v| ox + v),
        by.map(|v| oy + v),
    )
}

/// Monitor-relative form strings → absolute desktop pixels for capture/preview.
pub(crate) fn form_desktop_area(
    catalog: &ProgramCatalog,
    monitor: u32,
    left: &str,
    top: &str,
    right: &str,
    bottom: &str,
) -> (Option<i32>, Option<i32>, Option<i32>, Option<i32>) {
    form_absolute_area(
        catalog,
        monitor,
        form_coord_literal(left),
        form_coord_literal(top),
        form_coord_literal(right),
        form_coord_literal(bottom),
    )
}

pub(crate) fn copy_image_as_png(
    src: &std::path::Path,
    dest: &std::path::Path,
) -> Result<(), String> {
    let bytes = std::fs::read(src).map_err(|e| format!("read: {e}"))?;
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        std::fs::write(dest, &bytes).map_err(|e| format!("write: {e}"))?;
        return Ok(());
    }
    let img = image::load_from_memory(&bytes).map_err(|e| format!("decode: {e}"))?;
    img.save(dest).map_err(|e| format!("save png: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::ScalarValue;
    use sqyre_persist::{ProgramCatalog, ProgramItem};

    #[test]
    fn editor_lists_omit_temporary_program() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("Game").unwrap();
        cat.create_program(sqyre_persist::TEMPORARY_PROGRAM)
            .unwrap();
        let names: Vec<_> = editor_program_names(&cat).map(|s| s.as_str()).collect();
        assert_eq!(names, ["Game"]);
        assert!(!is_editor_listed_program(sqyre_persist::TEMPORARY_PROGRAM));
        assert!(is_editor_listed_program("Game"));
    }

    #[test]
    fn unique_name_appends_suffix() {
        assert_eq!(unique_name("Item", |_| false), "Item");
        assert_eq!(unique_name("Item", |n| n == "Item"), "Item 2");
        assert_eq!(
            unique_name("Item", |n| n == "Item" || n == "Item 2"),
            "Item 3"
        );
    }

    #[test]
    fn parse_scalar_kinds() {
        assert_eq!(ScalarValue::parse_edit(""), ScalarValue::Null);
        assert_eq!(ScalarValue::parse_edit("42"), ScalarValue::Int(42));
        assert_eq!(ScalarValue::parse_edit("1.5"), ScalarValue::Float(1.5));
        assert_eq!(ScalarValue::parse_edit("true"), ScalarValue::Bool(true));
        assert_eq!(ScalarValue::parse_edit("FALSE"), ScalarValue::Bool(false));
        assert_eq!(
            ScalarValue::parse_edit("hello"),
            ScalarValue::String("hello".into())
        );
    }

    #[test]
    fn form_coord_literal_rejects_refs() {
        assert_eq!(form_coord_literal("100"), Some(100));
        assert_eq!(form_coord_literal(""), Some(0));
        assert_eq!(form_coord_literal("${x}"), None);
        assert_eq!(form_coord_literal("1+2"), None);
    }

    #[test]
    fn item_tag_completion_filters() {
        let opts = crate::widgets::tags::tag_completion_options(
            "hel",
            &["healing".into()],
            &[
                "healing".into(),
                "helm".into(),
                "herb".into(),
                "other".into(),
            ],
            10,
        );
        assert_eq!(opts, vec!["helm".to_string()]);
    }

    #[test]
    fn collect_program_item_tags_dedups() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("Game").unwrap();
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "A".into(),
                mask: String::new(),
                stack_max: 0,
                grid_cols: 1,
                grid_rows: 1,
                tags: vec!["alpha".into(), "beta".into()],
            },
        )
        .unwrap();
        cat.upsert_item(
            "Game",
            ProgramItem {
                name: "B".into(),
                mask: String::new(),
                stack_max: 0,
                grid_cols: 1,
                grid_rows: 1,
                tags: vec!["beta".into(), "gamma".into()],
            },
        )
        .unwrap();
        assert_eq!(
            collect_program_item_tags(&cat, "Game"),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn overlay_hex_or_empty_uses_default() {
        let c = eframe::egui::Color32::from_rgb(0x12, 0x34, 0x56);
        assert!(overlay_hex_or_empty(c, "#123456").is_empty());
        assert!(!overlay_hex_or_empty(c, "#000000").is_empty());
    }

    #[test]
    fn form_absolute_area_adds_monitor_origin() {
        let mut cat = ProgramCatalog::default();
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080), (1920, 100, 1920, 1080)]);
        let (lx, ty, rx, by) = form_absolute_area(&cat, 2, Some(10), Some(20), Some(110), Some(80));
        assert_eq!(
            (lx, ty, rx, by),
            (Some(1930), Some(120), Some(2030), Some(180))
        );
        let (x, y) = form_absolute_xy(&cat, 2, Some(10), Some(20));
        assert_eq!((x, y), (Some(1930), Some(120)));
    }

    #[test]
    fn form_desktop_area_roundtrips_recorded_relative() {
        let mut cat = ProgramCatalog::default();
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080), (2560, 0, 1920, 1080)]);
        let (monitor, lx, ty, rx, by) = sqyre_persist::absolute_area_to_relative(
            cat.monitor_rects(),
            2660,
            40,
            2660,
            40,
            2760,
            140,
        );
        assert_eq!(monitor, 2);
        let abs = form_desktop_area(
            &cat,
            monitor,
            &lx.to_string(),
            &ty.to_string(),
            &rx.to_string(),
            &by.to_string(),
        );
        assert_eq!(abs, (Some(2660), Some(40), Some(2760), Some(140)));
    }
}
