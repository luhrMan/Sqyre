//! Seeded `General` program with common Points, Search Areas, and Collections.

use super::{MonitorRect, ProgramCatalog, ProgramCollection, ProgramPoint, ProgramSearchArea};
use crate::Result;
use sqyre_domain::ScalarValue;

pub const GENERAL_PROGRAM: &str = "General";

/// Scratch program for macro-recording temp points; overwritten each recording.
pub const TEMPORARY_PROGRAM: &str = "temporary";

/// Point that resolves to Image Search match coordinates (`foundX` / `foundY`).
pub const IMAGE_SEARCH_REFERENCE: &str = "Image Search Reference";

/// Create/update the `General` program with one search area + 2×2 Collection
/// per monitor (quadrants / halves via cell ranges), and per-monitor corner / center /
/// corner-mid Points. Coordinates are **monitor-relative** (origin at each slot's top-left).
/// Always ensures [`IMAGE_SEARCH_REFERENCE`] (`${foundX}` / `${foundY}`) is present.
///
/// Re-runs against the live (shared) monitor list so displays are added or
/// removed after capture changes, and generated coordinates stay aligned.
///
/// `monitors` are absolute virtual-desktop rects (`x`, `y`, `w`, `h`), typically from
/// the live capturer (used only for sizes / slot count). Returns `true` when modified.
pub fn ensure_general_program(
    catalog: &mut ProgramCatalog,
    monitors: &[MonitorRect],
) -> Result<bool> {
    let mut changed = false;
    if catalog.get(GENERAL_PROGRAM).is_none() {
        catalog.create_program(GENERAL_PROGRAM)?;
        changed = true;
    }
    let monitors = usable_monitors(monitors);
    if seed_search_areas(catalog, &monitors)? {
        changed = true;
    }
    if seed_collections(catalog, monitors.len())? {
        changed = true;
    }
    if seed_points(catalog, &monitors)? {
        changed = true;
    }
    if prune_generated_beyond(catalog, monitors.len())? {
        changed = true;
    }
    if ensure_image_search_reference(catalog)? {
        changed = true;
    }
    Ok(changed)
}

fn ensure_image_search_reference(catalog: &mut ProgramCatalog) -> Result<bool> {
    if program_has_point(catalog, IMAGE_SEARCH_REFERENCE) {
        return Ok(false);
    }
    catalog.upsert_point(
        GENERAL_PROGRAM,
        ProgramPoint {
            name: IMAGE_SEARCH_REFERENCE.into(),
            monitor: 1,
            x: ScalarValue::String("${foundX}".into()),
            y: ScalarValue::String("${foundY}".into()),
        },
    )?;
    Ok(true)
}

fn program_has_point(catalog: &ProgramCatalog, name: &str) -> bool {
    catalog
        .get(GENERAL_PROGRAM)
        .map(|p| p.points.values().any(|bucket| bucket.contains_key(name)))
        .unwrap_or(false)
}

fn usable_monitors(monitors: &[MonitorRect]) -> Vec<MonitorRect> {
    // Preserve caller order (preferred_monitor_rects is primary-first on Linux/Windows).
    let mut usable = Vec::with_capacity(monitors.len());
    for &r in monitors {
        let (_, _, w, h) = r;
        if w > 1 && h > 1 && !usable.contains(&r) {
            usable.push(r);
        }
    }
    if usable.is_empty() {
        vec![(0, 0, 1920, 1080)]
    } else {
        usable
    }
}

/// Current generated monitor entity: `Monitor N` (search area + collection).
const GENERATED_MONITOR_SUFFIXES: [&str; 1] = [""];
/// Previous named halves / quadrants / whole-screen areas — dropped in favor of collections.
const OBSOLETE_AREA_SUFFIXES: [&str; 9] = [
    " Whole Screen",
    " Left Half",
    " Right Half",
    " Top Half",
    " Bottom Half",
    " Top Left Quadrant",
    " Top Right Quadrant",
    " Bottom Left Quadrant",
    " Bottom Right Quadrant",
];
const GENERATED_POINT_SUFFIXES: [&str; 9] = [
    " Top Left",
    " Top Right",
    " Bottom Left",
    " Bottom Right",
    " Center",
    " Top Left Mid",
    " Top Right Mid",
    " Bottom Left Mid",
    " Bottom Right Mid",
];

fn generated_monitor_index(name: &str, suffixes: &[&str]) -> Option<usize> {
    let rest = name.strip_prefix("Monitor ")?;
    let digit_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if digit_end == 0 {
        return None;
    }
    let (n, suffix) = rest.split_at(digit_end);
    if !suffixes.contains(&suffix) {
        return None;
    }
    n.parse().ok().filter(|&i| i >= 1)
}

fn stale_monitor_area(name: &str, monitor_count: usize) -> bool {
    generated_monitor_index(name, &OBSOLETE_AREA_SUFFIXES).is_some()
        || generated_monitor_index(name, &GENERATED_MONITOR_SUFFIXES)
            .is_some_and(|n| n > monitor_count)
}

fn stale_monitor_entity(name: &str, suffixes: &[&str], monitor_count: usize) -> bool {
    generated_monitor_index(name, suffixes).is_some_and(|n| n > monitor_count)
}

fn prune_generated_beyond(catalog: &mut ProgramCatalog, monitor_count: usize) -> Result<bool> {
    let stale_in = |p: &super::ProgramData| {
        p.search_areas.values().any(|bucket| {
            bucket
                .keys()
                .any(|name| stale_monitor_area(name, monitor_count))
        }) || p.points.values().any(|bucket| {
            bucket
                .keys()
                .any(|name| stale_monitor_entity(name, &GENERATED_POINT_SUFFIXES, monitor_count))
        }) || p
            .collections
            .keys()
            .any(|name| stale_monitor_entity(name, &GENERATED_MONITOR_SUFFIXES, monitor_count))
    };
    if !catalog.get(GENERAL_PROGRAM).is_some_and(stale_in) {
        return Ok(false);
    }
    let p = catalog.program_mut(GENERAL_PROGRAM)?;
    let mut changed = false;
    for bucket in p.search_areas.values_mut() {
        let stale: Vec<String> = bucket
            .keys()
            .filter(|name| stale_monitor_area(name, monitor_count))
            .cloned()
            .collect();
        for key in stale {
            bucket.remove(&key);
            changed = true;
        }
    }
    for bucket in p.points.values_mut() {
        let stale: Vec<String> = bucket
            .keys()
            .filter(|name| stale_monitor_entity(name, &GENERATED_POINT_SUFFIXES, monitor_count))
            .cloned()
            .collect();
        for key in stale {
            bucket.remove(&key);
            changed = true;
        }
    }
    let stale_cols: Vec<String> = p
        .collections
        .keys()
        .filter(|name| stale_monitor_entity(name, &GENERATED_MONITOR_SUFFIXES, monitor_count))
        .cloned()
        .collect();
    for key in stale_cols {
        p.collections.remove(&key);
        changed = true;
    }
    Ok(changed)
}

fn seed_search_areas(catalog: &mut ProgramCatalog, monitors: &[MonitorRect]) -> Result<bool> {
    let mut changed = false;
    for (i, &(_, _, w, h)) in monitors.iter().enumerate() {
        let n = (i + 1) as u32;
        if upsert_area(catalog, &format!("Monitor {n}"), n, 0, 0, w, h)? {
            changed = true;
        }
    }
    // Drop legacy virtual-desktop union if still present.
    if prune_named_area(catalog, "Whole Screen")? {
        changed = true;
    }
    Ok(changed)
}

fn prune_named_area(catalog: &mut ProgramCatalog, name: &str) -> Result<bool> {
    let Some(p) = catalog.get(GENERAL_PROGRAM) else {
        return Ok(false);
    };
    let present = p
        .search_areas
        .values()
        .any(|bucket| bucket.contains_key(name));
    if !present {
        return Ok(false);
    }
    let p = catalog.program_mut(GENERAL_PROGRAM)?;
    let mut changed = false;
    for bucket in p.search_areas.values_mut() {
        if bucket.remove(name).is_some() {
            changed = true;
        }
    }
    Ok(changed)
}

fn seed_collections(catalog: &mut ProgramCatalog, monitor_count: usize) -> Result<bool> {
    let mut changed = false;
    for n in 1..=monitor_count {
        let name = format!("Monitor {n}");
        if upsert_collection(catalog, &name, &name, 2, 2)? {
            changed = true;
        }
    }
    Ok(changed)
}

fn seed_points(catalog: &mut ProgramCatalog, monitors: &[MonitorRect]) -> Result<bool> {
    let mut changed = false;
    for (i, &(_, _, w, h)) in monitors.iter().enumerate() {
        let n = (i + 1) as u32;
        let right = w - 1;
        let bottom = h - 1;
        let cx = w / 2;
        let cy = h / 2;
        let mid_left_x = w / 4;
        let mid_right_x = (3 * w) / 4;
        let mid_top_y = h / 4;
        let mid_bottom_y = (3 * h) / 4;

        if upsert_point(catalog, &format!("Monitor {n} Top Left"), n, 0, 0)? {
            changed = true;
        }
        if upsert_point(catalog, &format!("Monitor {n} Top Right"), n, right, 0)? {
            changed = true;
        }
        if upsert_point(catalog, &format!("Monitor {n} Bottom Left"), n, 0, bottom)? {
            changed = true;
        }
        if upsert_point(
            catalog,
            &format!("Monitor {n} Bottom Right"),
            n,
            right,
            bottom,
        )? {
            changed = true;
        }
        if upsert_point(catalog, &format!("Monitor {n} Center"), n, cx, cy)? {
            changed = true;
        }
        if upsert_point(
            catalog,
            &format!("Monitor {n} Top Left Mid"),
            n,
            mid_left_x,
            mid_top_y,
        )? {
            changed = true;
        }
        if upsert_point(
            catalog,
            &format!("Monitor {n} Top Right Mid"),
            n,
            mid_right_x,
            mid_top_y,
        )? {
            changed = true;
        }
        if upsert_point(
            catalog,
            &format!("Monitor {n} Bottom Left Mid"),
            n,
            mid_left_x,
            mid_bottom_y,
        )? {
            changed = true;
        }
        if upsert_point(
            catalog,
            &format!("Monitor {n} Bottom Right Mid"),
            n,
            mid_right_x,
            mid_bottom_y,
        )? {
            changed = true;
        }
    }
    Ok(changed)
}

fn upsert_area(
    catalog: &mut ProgramCatalog,
    name: &str,
    monitor: u32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Result<bool> {
    let res = catalog.resolution_key();
    if let Some(existing) = catalog
        .get(GENERAL_PROGRAM)
        .and_then(|p| p.search_areas.get(res))
        .and_then(|m| m.get(name))
    {
        if existing.monitor == monitor
            && existing.left_x == ScalarValue::Int(left as i64)
            && existing.top_y == ScalarValue::Int(top as i64)
            && existing.right_x == ScalarValue::Int(right as i64)
            && existing.bottom_y == ScalarValue::Int(bottom as i64)
        {
            return Ok(false);
        }
    }
    catalog.upsert_search_area(
        GENERAL_PROGRAM,
        ProgramSearchArea {
            name: name.into(),
            monitor,
            left_x: ScalarValue::Int(left as i64),
            top_y: ScalarValue::Int(top as i64),
            right_x: ScalarValue::Int(right as i64),
            bottom_y: ScalarValue::Int(bottom as i64),
        },
    )?;
    Ok(true)
}

fn upsert_collection(
    catalog: &mut ProgramCatalog,
    name: &str,
    search_area: &str,
    rows: i32,
    cols: i32,
) -> Result<bool> {
    if let Some(existing) = catalog
        .get(GENERAL_PROGRAM)
        .and_then(|p| p.collections.get(name))
    {
        if existing.search_area == search_area && existing.rows == rows && existing.cols == cols {
            return Ok(false);
        }
    }
    catalog.upsert_collection(
        GENERAL_PROGRAM,
        ProgramCollection {
            name: name.into(),
            search_area: search_area.into(),
            rows,
            cols,
        },
    )?;
    Ok(true)
}

fn upsert_point(
    catalog: &mut ProgramCatalog,
    name: &str,
    monitor: u32,
    x: i32,
    y: i32,
) -> Result<bool> {
    let res = catalog.resolution_key();
    if let Some(existing) = catalog
        .get(GENERAL_PROGRAM)
        .and_then(|p| p.points.get(res))
        .and_then(|m| m.get(name))
    {
        if existing.monitor == monitor
            && existing.x == ScalarValue::Int(x as i64)
            && existing.y == ScalarValue::Int(y as i64)
        {
            return Ok(false);
        }
    }
    catalog.upsert_point(
        GENERAL_PROGRAM,
        ProgramPoint {
            name: name.into(),
            monitor,
            x: ScalarValue::Int(x as i64),
            y: ScalarValue::Int(y as i64),
        },
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::{absolute_area_to_relative, absolute_point_to_relative};
    use super::*;

    #[test]
    fn seeds_single_monitor_geometry() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());

        let p = cat.get(GENERAL_PROGRAM).expect("General");
        let res = "1920x1080";
        let areas = p.search_areas.get(res).expect("areas");
        assert!(!areas.contains_key("Whole Screen"));
        assert!(areas.contains_key("Monitor 1"));
        assert!(!areas.contains_key("Monitor 1 Left Half"));
        assert_eq!(areas.len(), 1);

        let monitor = &areas["Monitor 1"];
        assert_eq!(monitor.monitor, 1);
        assert_eq!(monitor.left_x, ScalarValue::Int(0));
        assert_eq!(monitor.top_y, ScalarValue::Int(0));
        assert_eq!(monitor.right_x, ScalarValue::Int(1920));
        assert_eq!(monitor.bottom_y, ScalarValue::Int(1080));

        let col = p.collections.get("Monitor 1").expect("collection");
        assert_eq!(col.search_area, "Monitor 1");
        assert_eq!(col.rows, 2);
        assert_eq!(col.cols, 2);

        let points = p.points.get(res).expect("points");
        assert_eq!(points.len(), 10);
        assert_eq!(points["Monitor 1 Top Left"].monitor, 1);
        assert_eq!(points["Monitor 1 Top Left"].x, ScalarValue::Int(0));
        assert_eq!(points["Monitor 1 Top Left"].y, ScalarValue::Int(0));
        assert_eq!(points["Monitor 1 Top Right"].x, ScalarValue::Int(1919));
        assert_eq!(points["Monitor 1 Bottom Right"].y, ScalarValue::Int(1079));
        assert_eq!(points["Monitor 1 Center"].x, ScalarValue::Int(960));
        assert_eq!(points["Monitor 1 Center"].y, ScalarValue::Int(540));
        assert_eq!(points["Monitor 1 Top Left Mid"].x, ScalarValue::Int(480));
        assert_eq!(points["Monitor 1 Top Left Mid"].y, ScalarValue::Int(270));
        assert_eq!(
            points["Monitor 1 Bottom Right Mid"].x,
            ScalarValue::Int(1440)
        );
        assert_eq!(
            points["Monitor 1 Bottom Right Mid"].y,
            ScalarValue::Int(810)
        );
        let found = &points[IMAGE_SEARCH_REFERENCE];
        assert_eq!(found.x, ScalarValue::String("${foundX}".into()));
        assert_eq!(found.y, ScalarValue::String("${foundY}".into()));
    }

    #[test]
    fn seeds_multi_monitor_relative_geometry() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("2560x1440");
        let monitors = [(0, 0, 2560, 1440), (2560, 0, 1920, 1080)];
        assert!(ensure_general_program(&mut cat, &monitors).unwrap());

        let p = cat.get(GENERAL_PROGRAM).expect("General");
        let areas = p.search_areas.get("2560x1440").expect("areas");
        assert_eq!(areas.len(), 2);
        assert!(!areas.contains_key("Whole Screen"));

        let m2 = &areas["Monitor 2"];
        assert_eq!(m2.monitor, 2);
        assert_eq!(m2.left_x, ScalarValue::Int(0));
        assert_eq!(m2.top_y, ScalarValue::Int(0));
        assert_eq!(m2.right_x, ScalarValue::Int(1920));
        assert_eq!(m2.bottom_y, ScalarValue::Int(1080));

        assert_eq!(p.collections.len(), 2);
        let m2_col = p
            .collections
            .get("Monitor 2")
            .expect("Monitor 2 collection");
        assert_eq!(m2_col.search_area, "Monitor 2");
        assert_eq!(m2_col.rows, 2);
        assert_eq!(m2_col.cols, 2);

        let points = p.points.get("2560x1440").expect("points");
        assert_eq!(points.len(), 19);
        assert_eq!(points["Monitor 2 Center"].monitor, 2);
        assert_eq!(points["Monitor 2 Center"].x, ScalarValue::Int(960));
        assert_eq!(points["Monitor 2 Center"].y, ScalarValue::Int(540));
        assert!(points.contains_key(IMAGE_SEARCH_REFERENCE));
    }

    #[test]
    fn fills_geometry_when_general_already_exists() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program(GENERAL_PROGRAM).unwrap();
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
        let p = cat.get(GENERAL_PROGRAM).unwrap();
        assert!(p.points["1920x1080"].contains_key("Monitor 1 Center"));
        let found = &p.points["1920x1080"][IMAGE_SEARCH_REFERENCE];
        assert_eq!(found.x, ScalarValue::String("${foundX}".into()));
        assert!(!ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
    }

    #[test]
    fn adds_second_monitor_when_layout_grows() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
        let monitors = [(0, 360, 1920, 1080), (1920, 0, 2560, 1440)];
        assert!(ensure_general_program(&mut cat, &monitors).unwrap());
        let points = &cat.get(GENERAL_PROGRAM).unwrap().points["1920x1080"];
        // Relative to each slot — Monitor 1 top-left stays (0,0) even if absolute origin shifts.
        assert_eq!(points["Monitor 1 Top Left"].y, ScalarValue::Int(0));
        assert_eq!(points["Monitor 1 Top Left"].monitor, 1);
        assert_eq!(points["Monitor 2 Center"].monitor, 2);
        assert_eq!(points["Monitor 2 Center"].x, ScalarValue::Int(1280));
        assert_eq!(points["Monitor 2 Center"].y, ScalarValue::Int(720));
        let p = cat.get(GENERAL_PROGRAM).unwrap();
        assert!(p.collections.contains_key("Monitor 2"));
        assert_eq!(
            p.search_areas["1920x1080"]["Monitor 2"].left_x,
            ScalarValue::Int(0)
        );
    }

    #[test]
    fn falls_back_when_monitors_unusable() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1, 1)]).unwrap());
        let areas = &cat.get(GENERAL_PROGRAM).unwrap().search_areas["1920x1080"];
        assert!(!areas.contains_key("Whole Screen"));
        assert_eq!(areas["Monitor 1"].right_x, ScalarValue::Int(1920));
    }

    #[test]
    fn drops_generated_geometry_when_share_shrinks() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        let two = [(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
        assert!(ensure_general_program(&mut cat, &two).unwrap());
        let points = &cat.get(GENERAL_PROGRAM).unwrap().points["1920x1080"];
        assert!(points.contains_key("Monitor 2 Center"));
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
        let p = cat.get(GENERAL_PROGRAM).unwrap();
        let points = &p.points["1920x1080"];
        let areas = &p.search_areas["1920x1080"];
        assert!(points.contains_key("Monitor 1 Center"));
        assert!(!points.contains_key("Monitor 2 Center"));
        assert!(!areas.contains_key("Monitor 2"));
        assert!(!p.collections.contains_key("Monitor 2"));
        assert!(areas.contains_key("Monitor 1"));
        assert!(p.collections.contains_key("Monitor 1"));
    }

    #[test]
    fn drops_obsolete_named_halves_for_collections() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program(GENERAL_PROGRAM).unwrap();
        cat.upsert_search_area(
            GENERAL_PROGRAM,
            ProgramSearchArea {
                name: "Monitor 1 Left Half".into(),
                monitor: 1,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(960),
                bottom_y: ScalarValue::Int(1080),
            },
        )
        .unwrap();
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
        let p = cat.get(GENERAL_PROGRAM).unwrap();
        let areas = &p.search_areas["1920x1080"];
        assert!(!areas.contains_key("Monitor 1 Left Half"));
        assert!(areas.contains_key("Monitor 1"));
        assert!(p.collections.contains_key("Monitor 1"));
    }

    #[test]
    fn drops_legacy_whole_screen_area() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program(GENERAL_PROGRAM).unwrap();
        cat.upsert_search_area(
            GENERAL_PROGRAM,
            ProgramSearchArea {
                name: "Whole Screen".into(),
                monitor: 1,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(3840),
                bottom_y: ScalarValue::Int(1080),
            },
        )
        .unwrap();
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
        let areas = &cat.get(GENERAL_PROGRAM).unwrap().search_areas["1920x1080"];
        assert!(!areas.contains_key("Whole Screen"));
        assert!(areas.contains_key("Monitor 1"));
    }

    #[test]
    fn resolve_applies_monitor_origin() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080), (2560, 100, 1920, 1080)]);
        ensure_general_program(&mut cat, &[(0, 0, 1920, 1080), (2560, 100, 1920, 1080)]).unwrap();
        let macro_ = sqyre_domain::Macro::new("t", 0, Vec::new());
        let (x, y) = cat
            .resolve_point(
                &sqyre_domain::CoordinateRef(format!("{GENERAL_PROGRAM}~Monitor 2 Center")),
                &macro_,
            )
            .unwrap();
        assert_eq!((x, y), (2560 + 960, 100 + 540));
        let area = cat
            .resolve_search_area(
                &sqyre_domain::CoordinateRef(format!("{GENERAL_PROGRAM}~Monitor 2")),
                &macro_,
            )
            .unwrap();
        assert_eq!(area, (2560, 100, 2560 + 1920, 100 + 1080));
    }

    #[test]
    fn resolve_errors_when_monitor_slot_missing() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080)]);
        ensure_general_program(&mut cat, &[(0, 0, 1920, 1080), (1920, 0, 1920, 1080)]).unwrap();
        // Keep Monitor 2 entity but shrink live layout to one slot.
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080)]);
        let macro_ = sqyre_domain::Macro::new("t", 0, Vec::new());
        let err = cat
            .resolve_point(
                &sqyre_domain::CoordinateRef(format!("{GENERAL_PROGRAM}~Monitor 2 Center")),
                &macro_,
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("monitor slot 2"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn monitor_collection_cells_cover_quadrants_and_halves() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.set_monitor_rects(vec![(100, 50, 1920, 1080)]);
        ensure_general_program(&mut cat, &[(100, 50, 1920, 1080)]).unwrap();
        let macro_ = sqyre_domain::Macro::new("t", 0, Vec::new());
        let tl = cat
            .resolve_search_area(
                &sqyre_domain::CoordinateRef::collection(GENERAL_PROGRAM, "Monitor 1", 1, 1, 1, 1),
                &macro_,
            )
            .unwrap();
        assert_eq!(tl, (100, 50, 100 + 960, 50 + 540));
        let left = cat
            .resolve_search_area(
                &sqyre_domain::CoordinateRef::collection(GENERAL_PROGRAM, "Monitor 1", 1, 1, 2, 1),
                &macro_,
            )
            .unwrap();
        assert_eq!(left, (100, 50, 100 + 960, 50 + 1080));
        let whole = cat
            .resolve_search_area(
                &sqyre_domain::CoordinateRef::collection(GENERAL_PROGRAM, "Monitor 1", 1, 1, 2, 2),
                &macro_,
            )
            .unwrap();
        assert_eq!(whole, (100, 50, 100 + 1920, 50 + 1080));
    }

    #[test]
    fn absolute_point_to_relative_uses_containing_monitor() {
        let rects = [(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
        assert_eq!(absolute_point_to_relative(&rects, 100, 200), (1, 100, 200));
        assert_eq!(
            absolute_point_to_relative(&rects, 1920 + 10, 20),
            (2, 10, 20)
        );
    }

    #[test]
    fn absolute_area_clamps_to_press_monitor() {
        let rects = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        // Drag from monitor 1 into monitor 2 — clamp stays on monitor 1.
        let (mon, lx, ty, rx, by) =
            absolute_area_to_relative(&rects, 100, 100, 100, 100, 2000, 500);
        assert_eq!(mon, 1);
        assert_eq!((lx, ty, rx, by), (100, 100, 1919, 500));
        // Press on monitor 2 even when normalized top-left would fall on monitor 1.
        let (mon2, _, _, _, _) = absolute_area_to_relative(&rects, 2000, 500, 100, 100, 2000, 500);
        assert_eq!(mon2, 2);
    }

    #[test]
    fn parse_defaults_missing_monitor_to_one() {
        let yaml = r#"
Demo:
  name: Demo
  coordinates:
    1920x1080:
      scale: 1.0
      points:
        Spot:
          name: Spot
          x: 10
          y: 20
      searchareas:
        Box:
          name: Box
          leftx: 0
          topy: 0
          rightx: 50
          bottomy: 50
"#;
        let v: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let cat = ProgramCatalog::from_yaml_value(&v).unwrap();
        let p = cat.get("Demo").unwrap();
        assert_eq!(p.points["1920x1080"]["Spot"].monitor, 1);
        assert_eq!(p.search_areas["1920x1080"]["Box"].monitor, 1);
        let encoded = cat.to_yaml_value(&serde_yaml::Value::Null);
        let reparsed = ProgramCatalog::from_yaml_value(&encoded).unwrap();
        let spot = &reparsed.get("Demo").unwrap().points["1920x1080"]["Spot"];
        assert_eq!(spot.monitor, 1);
        assert_eq!(spot.x, ScalarValue::Int(10));
    }
}
