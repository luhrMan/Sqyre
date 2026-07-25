//! Seeded `General` program with common Points and Search Areas.

use super::{ProgramCatalog, ProgramPoint, ProgramSearchArea};
use sqyre_domain::ScalarValue;

pub const GENERAL_PROGRAM: &str = "General";

/// Point that resolves to Image Search match coordinates (`foundX` / `foundY`).
pub const IMAGE_SEARCH_REFERENCE: &str = "Image Search Reference";

/// One display in virtual-desktop coordinates (`x`, `y`, `w`, `h`).
pub type MonitorRect = (i32, i32, i32, i32);

/// Create the `General` program with Whole Screen + per-monitor half Search Areas
/// and per-monitor corner / center / corner-mid Points, if it does not already exist.
/// Always ensures [`IMAGE_SEARCH_REFERENCE`] (`${foundX}` / `${foundY}`) is present.
///
/// `monitors` are absolute virtual-desktop rects (`x`, `y`, `w`, `h`), typically from
/// the live capturer. Returns `true` when the catalog was modified.
pub fn ensure_general_program(
    catalog: &mut ProgramCatalog,
    monitors: &[MonitorRect],
) -> Result<bool, String> {
    let mut changed = false;
    if catalog.get(GENERAL_PROGRAM).is_none() {
        let monitors = usable_monitors(monitors);
        catalog
            .create_program(GENERAL_PROGRAM)
            .map_err(|e| e.to_string())?;
        seed_search_areas(catalog, &monitors)?;
        seed_points(catalog, &monitors)?;
        changed = true;
    }
    if ensure_image_search_reference(catalog)? {
        changed = true;
    }
    Ok(changed)
}

fn ensure_image_search_reference(catalog: &mut ProgramCatalog) -> Result<bool, String> {
    if program_has_point(catalog, IMAGE_SEARCH_REFERENCE) {
        return Ok(false);
    }
    catalog
        .upsert_point(
            GENERAL_PROGRAM,
            ProgramPoint {
                name: IMAGE_SEARCH_REFERENCE.into(),
                x: ScalarValue::String("${foundX}".into()),
                y: ScalarValue::String("${foundY}".into()),
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(true)
}

fn program_has_point(catalog: &ProgramCatalog, name: &str) -> bool {
    catalog
        .get(GENERAL_PROGRAM)
        .map(|p| p.points.values().any(|bucket| bucket.contains_key(name)))
        .unwrap_or(false)
}

fn usable_monitors(monitors: &[MonitorRect]) -> Vec<MonitorRect> {
    let usable: Vec<_> = monitors
        .iter()
        .copied()
        .filter(|&(_, _, w, h)| w > 1 && h > 1)
        .collect();
    if usable.is_empty() {
        vec![(0, 0, 1920, 1080)]
    } else {
        usable
    }
}

fn seed_search_areas(catalog: &mut ProgramCatalog, monitors: &[MonitorRect]) -> Result<(), String> {
    let (left, top, right, bottom) = virtual_bounds(monitors);
    upsert_area(catalog, "Whole Screen", left, top, right, bottom)?;

    for (i, &(ox, oy, w, h)) in monitors.iter().enumerate() {
        let n = i + 1;
        let mid_x = ox + w / 2;
        let mid_y = oy + h / 2;
        let right = ox + w;
        let bottom = oy + h;
        upsert_area(
            catalog,
            &format!("Monitor {n} Left Half"),
            ox,
            oy,
            mid_x,
            bottom,
        )?;
        upsert_area(
            catalog,
            &format!("Monitor {n} Right Half"),
            mid_x,
            oy,
            right,
            bottom,
        )?;
        upsert_area(
            catalog,
            &format!("Monitor {n} Top Half"),
            ox,
            oy,
            right,
            mid_y,
        )?;
        upsert_area(
            catalog,
            &format!("Monitor {n} Bottom Half"),
            ox,
            mid_y,
            right,
            bottom,
        )?;
    }
    Ok(())
}

fn seed_points(catalog: &mut ProgramCatalog, monitors: &[MonitorRect]) -> Result<(), String> {
    for (i, &(ox, oy, w, h)) in monitors.iter().enumerate() {
        let n = i + 1;
        let right = ox + w - 1;
        let bottom = oy + h - 1;
        let cx = ox + w / 2;
        let cy = oy + h / 2;
        let mid_left_x = ox + w / 4;
        let mid_right_x = ox + (3 * w) / 4;
        let mid_top_y = oy + h / 4;
        let mid_bottom_y = oy + (3 * h) / 4;

        // Corners
        upsert_point(catalog, &format!("Monitor {n} Top Left"), ox, oy)?;
        upsert_point(catalog, &format!("Monitor {n} Top Right"), right, oy)?;
        upsert_point(catalog, &format!("Monitor {n} Bottom Left"), ox, bottom)?;
        upsert_point(catalog, &format!("Monitor {n} Bottom Right"), right, bottom)?;
        // Center
        upsert_point(catalog, &format!("Monitor {n} Center"), cx, cy)?;
        // Midpoints between each corner and the center
        upsert_point(
            catalog,
            &format!("Monitor {n} Top Left Mid"),
            mid_left_x,
            mid_top_y,
        )?;
        upsert_point(
            catalog,
            &format!("Monitor {n} Top Right Mid"),
            mid_right_x,
            mid_top_y,
        )?;
        upsert_point(
            catalog,
            &format!("Monitor {n} Bottom Left Mid"),
            mid_left_x,
            mid_bottom_y,
        )?;
        upsert_point(
            catalog,
            &format!("Monitor {n} Bottom Right Mid"),
            mid_right_x,
            mid_bottom_y,
        )?;
    }
    Ok(())
}

fn virtual_bounds(monitors: &[MonitorRect]) -> (i32, i32, i32, i32) {
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;
    for &(x, y, w, h) in monitors {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x + w);
        bottom = bottom.max(y + h);
    }
    (left, top, right, bottom)
}

fn upsert_area(
    catalog: &mut ProgramCatalog,
    name: &str,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Result<(), String> {
    catalog
        .upsert_search_area(
            GENERAL_PROGRAM,
            ProgramSearchArea {
                name: name.into(),
                left_x: ScalarValue::Int(left as i64),
                top_y: ScalarValue::Int(top as i64),
                right_x: ScalarValue::Int(right as i64),
                bottom_y: ScalarValue::Int(bottom as i64),
            },
        )
        .map_err(|e| e.to_string())
}

fn upsert_point(catalog: &mut ProgramCatalog, name: &str, x: i32, y: i32) -> Result<(), String> {
    catalog
        .upsert_point(
            GENERAL_PROGRAM,
            ProgramPoint {
                name: name.into(),
                x: ScalarValue::Int(x as i64),
                y: ScalarValue::Int(y as i64),
            },
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_single_monitor_geometry() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());

        let p = cat.get(GENERAL_PROGRAM).expect("General");
        let res = "1920x1080";
        let areas = p.search_areas.get(res).expect("areas");
        assert!(areas.contains_key("Whole Screen"));
        assert!(areas.contains_key("Monitor 1 Left Half"));
        assert!(areas.contains_key("Monitor 1 Right Half"));
        assert!(areas.contains_key("Monitor 1 Top Half"));
        assert!(areas.contains_key("Monitor 1 Bottom Half"));
        assert_eq!(areas.len(), 5);

        let whole = &areas["Whole Screen"];
        assert_eq!(whole.left_x, ScalarValue::Int(0));
        assert_eq!(whole.top_y, ScalarValue::Int(0));
        assert_eq!(whole.right_x, ScalarValue::Int(1920));
        assert_eq!(whole.bottom_y, ScalarValue::Int(1080));

        let left = &areas["Monitor 1 Left Half"];
        assert_eq!(left.right_x, ScalarValue::Int(960));
        let top = &areas["Monitor 1 Top Half"];
        assert_eq!(top.bottom_y, ScalarValue::Int(540));

        let points = p.points.get(res).expect("points");
        assert_eq!(points.len(), 10);
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
    fn seeds_multi_monitor_geometry() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("2560x1440");
        let monitors = [(0, 0, 2560, 1440), (2560, 0, 1920, 1080)];
        assert!(ensure_general_program(&mut cat, &monitors).unwrap());

        let p = cat.get(GENERAL_PROGRAM).expect("General");
        let areas = p.search_areas.get("2560x1440").expect("areas");
        assert_eq!(areas.len(), 1 + 4 * 2);
        let whole = &areas["Whole Screen"];
        assert_eq!(whole.left_x, ScalarValue::Int(0));
        assert_eq!(whole.right_x, ScalarValue::Int(4480));
        assert_eq!(whole.bottom_y, ScalarValue::Int(1440));

        let m2_left = &areas["Monitor 2 Left Half"];
        assert_eq!(m2_left.left_x, ScalarValue::Int(2560));
        assert_eq!(m2_left.right_x, ScalarValue::Int(2560 + 960));

        let points = p.points.get("2560x1440").expect("points");
        assert_eq!(points.len(), 19);
        assert_eq!(points["Monitor 2 Center"].x, ScalarValue::Int(2560 + 960));
        assert_eq!(points["Monitor 2 Center"].y, ScalarValue::Int(540));
        assert!(points.contains_key(IMAGE_SEARCH_REFERENCE));
    }

    #[test]
    fn skips_geometry_when_general_already_exists() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.create_program(GENERAL_PROGRAM).unwrap();
        // Still adds Image Search Reference when missing.
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
        let p = cat.get(GENERAL_PROGRAM).unwrap();
        assert!(p
            .search_areas
            .get("1920x1080")
            .map(|m| m.is_empty())
            .unwrap_or(true));
        let found = &p.points["1920x1080"][IMAGE_SEARCH_REFERENCE];
        assert_eq!(found.x, ScalarValue::String("${foundX}".into()));
        assert_eq!(found.y, ScalarValue::String("${foundY}".into()));
        // Second call is a no-op.
        assert!(!ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]).unwrap());
    }

    #[test]
    fn falls_back_when_monitors_unusable() {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        assert!(ensure_general_program(&mut cat, &[(0, 0, 1, 1)]).unwrap());
        let areas = &cat.get(GENERAL_PROGRAM).unwrap().search_areas["1920x1080"];
        assert_eq!(areas["Whole Screen"].right_x, ScalarValue::Int(1920));
    }
}
