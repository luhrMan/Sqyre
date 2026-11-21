//! Entity / coordinate-ref preview specs: cache keys, captions, literal checks.

use super::geometry::PreviewCoords;
use super::PreviewKind;
use sqyre_domain::{Action, ActionKind, CoordinateRef, Macro, ScalarValue, PROGRAM_DELIMITER};
use sqyre_persist::{CatalogView, ProgramPoint, ProgramSearchArea};

/// Coordinate ref + preview kind for actions that show point/search-area captures.
pub fn coordinate_ref_for_preview(action: &Action) -> Option<(CoordinateRef, PreviewKind)> {
    match &action.kind {
        ActionKind::Move { point, .. } if !point.is_empty() => {
            Some((point.clone(), PreviewKind::Point))
        }
        ActionKind::ImageSearch(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::Ocr(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::FindPixel(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::ForEachCell { cells, .. } if !cells.is_empty() => {
            Some((cells.clone(), PreviewKind::SearchArea))
        }
        _ => None,
    }
}

pub(super) fn ref_preview_spec(
    catalog: CatalogView<'_>,
    macro_: &Macro,
    coord_ref: &CoordinateRef,
    kind: PreviewKind,
) -> Result<(String, String, PreviewCoords), String> {
    match kind {
        PreviewKind::Point => {
            let (x, y) = catalog
                .resolve_point(coord_ref, macro_)
                .map_err(|e| e.to_string())?;
            let coords = PreviewCoords::Point { x, y };
            Ok((
                cache_key_ref(coord_ref, coords),
                format!("X: {x}, Y: {y}"),
                coords,
            ))
        }
        PreviewKind::SearchArea | PreviewKind::Collection => {
            let (left, top, right, bottom) = catalog
                .resolve_search_area(coord_ref, macro_)
                .map_err(|e| e.to_string())?;
            let coords = PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid: None,
            };
            Ok((
                cache_key_ref(coord_ref, coords),
                format!("Left: {left}, Top: {top}, Right: {right}, Bottom: {bottom}"),
                coords,
            ))
        }
    }
}

fn cache_key_ref(coord_ref: &CoordinateRef, coords: PreviewCoords) -> String {
    match coords {
        PreviewCoords::Point { x, y } => format!("ref:pt:{}:{x}:{y}", coord_ref.as_str()),
        PreviewCoords::SearchArea {
            left,
            top,
            right,
            bottom,
            grid: _,
        } => format!(
            "ref:sa:{}:{left}:{top}:{right}:{bottom}",
            coord_ref.as_str()
        ),
    }
}

#[derive(Debug)]
pub(super) enum EntityPreviewError {
    Missing,
    NonLiteral,
}

pub(super) fn entity_preview_spec(
    catalog: CatalogView<'_>,
    program: &str,
    name: &str,
    kind: PreviewKind,
) -> Result<(String, String, PreviewCoords), EntityPreviewError> {
    let pdata = catalog.get(program).ok_or(EntityPreviewError::Missing)?;
    let res = catalog.resolution_key();
    let empty = Macro::new("", 0, vec![]);
    match kind {
        PreviewKind::Point => {
            let pt = pdata
                .points
                .get(res)
                .or_else(|| pdata.points.values().next())
                .and_then(|m| m.get(name))
                .ok_or(EntityPreviewError::Missing)?;
            let _ = coord_to_literal(&pt.x).ok_or(EntityPreviewError::NonLiteral)?;
            let _ = coord_to_literal(&pt.y).ok_or(EntityPreviewError::NonLiteral)?;
            let coord = CoordinateRef(format!("{program}{PROGRAM_DELIMITER}{name}"));
            let (x, y) = catalog
                .resolve_point(&coord, &empty)
                .map_err(|_| EntityPreviewError::Missing)?;
            Ok((
                cache_key_point(pt),
                point_caption(pt),
                PreviewCoords::Point { x, y },
            ))
        }
        PreviewKind::SearchArea => {
            let sa = pdata
                .search_areas
                .get(res)
                .or_else(|| pdata.search_areas.values().next())
                .and_then(|m| m.get(name))
                .ok_or(EntityPreviewError::Missing)?;
            require_literal_area(sa)?;
            let coord = CoordinateRef(format!("{program}{PROGRAM_DELIMITER}{name}"));
            let (left, top, right, bottom) = catalog
                .resolve_search_area(&coord, &empty)
                .map_err(|_| EntityPreviewError::Missing)?;
            Ok((
                cache_key_search_area(sa),
                search_area_caption(sa),
                PreviewCoords::SearchArea {
                    left,
                    top,
                    right,
                    bottom,
                    grid: None,
                },
            ))
        }
        PreviewKind::Collection => {
            let col = pdata
                .collections
                .get(name)
                .ok_or(EntityPreviewError::Missing)?;
            if col.name.is_empty() {
                return Err(EntityPreviewError::Missing);
            }
            let sa = pdata
                .search_areas
                .get(res)
                .or_else(|| pdata.search_areas.values().next())
                .and_then(|m| m.get(&col.name))
                .ok_or(EntityPreviewError::Missing)?;
            require_literal_area(sa)?;
            let rows = col.rows.max(1);
            let cols = col.cols.max(1);
            let cell = CoordinateRef::collection(program, name, 1, 1, rows, cols);
            let (left, top, right, bottom) = catalog
                .resolve_search_area(&cell, &empty)
                .map_err(|_| EntityPreviewError::Missing)?;
            Ok((
                cache_key_collection(&col.name, sa, rows, cols),
                collection_caption(sa, rows, cols),
                PreviewCoords::SearchArea {
                    left,
                    top,
                    right,
                    bottom,
                    grid: Some((rows, cols)),
                },
            ))
        }
    }
}

fn require_literal_area(sa: &ProgramSearchArea) -> Result<(), EntityPreviewError> {
    let _ = coord_to_literal(&sa.left_x).ok_or(EntityPreviewError::NonLiteral)?;
    let _ = coord_to_literal(&sa.top_y).ok_or(EntityPreviewError::NonLiteral)?;
    let _ = coord_to_literal(&sa.right_x).ok_or(EntityPreviewError::NonLiteral)?;
    let _ = coord_to_literal(&sa.bottom_y).ok_or(EntityPreviewError::NonLiteral)?;
    Ok(())
}

fn cache_key_point(pt: &ProgramPoint) -> String {
    format!(
        "pt:{}:m{}:{}:{}",
        pt.name,
        pt.monitor,
        pt.x.as_display(),
        pt.y.as_display()
    )
}

fn cache_key_search_area(sa: &ProgramSearchArea) -> String {
    format!(
        "sa:{}:m{}:{}:{}:{}:{}",
        sa.name,
        sa.monitor,
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display()
    )
}

fn cache_key_collection(name: &str, sa: &ProgramSearchArea, rows: i32, cols: i32) -> String {
    format!(
        "col:{}:m{}:{}:{}:{}:{}:{}x{}",
        name,
        sa.monitor,
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display(),
        rows,
        cols
    )
}

fn point_caption(pt: &ProgramPoint) -> String {
    format!("X: {}, Y: {}", pt.x.as_display(), pt.y.as_display())
}

fn search_area_caption(sa: &ProgramSearchArea) -> String {
    let bounds = format!(
        "Left: {}, Top: {}, Right: {}, Bottom: {}",
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display()
    );
    match crate::data_editor_preview::search_area_pixel_size(sa) {
        Some((w, h)) => format!(
            "{bounds}\n· area {}",
            crate::data_editor_preview::pixel_size_text(w, h)
        ),
        None => bounds,
    }
}

fn collection_caption(sa: &ProgramSearchArea, rows: i32, cols: i32) -> String {
    let bounds = search_area_caption(sa);
    match crate::data_editor_preview::search_area_pixel_size(sa)
        .and_then(|(w, h)| crate::data_editor_preview::collection_cell_pixel_size(w, h, rows, cols))
    {
        Some((cw, ch)) => format!(
            "{bounds}\n· cell {}\n· {rows} rows × {cols} cols",
            crate::data_editor_preview::pixel_size_text(cw, ch)
        ),
        None => format!("{bounds}\n· {rows} rows × {cols} cols"),
    }
}

/// Literal numeric coordinate suitable for a live screen preview.
/// Returns `None` for variable refs and other non-numeric expressions.
fn coord_to_literal(v: &ScalarValue) -> Option<i32> {
    match v {
        ScalarValue::Int(i) => Some(*i as i32),
        ScalarValue::Float(f) => Some(*f as i32),
        ScalarValue::Bool(b) => Some(if *b { 1 } else { 0 }),
        ScalarValue::String(s) => {
            let s = s.trim();
            if s.is_empty() || sqyre_varref::contains(s) {
                return None;
            }
            s.parse::<i32>()
                .ok()
                .or_else(|| s.parse::<f64>().ok().map(|f| f as i32))
        }
        ScalarValue::Null => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::ProgramCatalog;

    #[test]
    fn coordinate_ref_for_preview_matches_action_kinds() {
        use sqyre_domain::{ActionKind, CoordinateRef};

        assert!(coordinate_ref_for_preview(&Action::new(ActionKind::Wait {
            time: ScalarValue::Int(1),
        }))
        .is_none());

        let mv = Action::new(ActionKind::Move {
            point: CoordinateRef("P~Spot".into()),
            smooth: false,
            smooth_low: 0.05,
            smooth_high: 0.2,
            smooth_delay_ms: 1,
        });
        assert_eq!(
            coordinate_ref_for_preview(&mv),
            Some((CoordinateRef("P~Spot".into()), PreviewKind::Point))
        );

        let search = Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                name: "find".into(),
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                ..Default::default()
            },
        )));
        assert_eq!(
            coordinate_ref_for_preview(&search),
            Some((CoordinateRef("P~Box".into()), PreviewKind::SearchArea))
        );
    }

    #[test]
    fn captions_use_expected_format() {
        let pt = ProgramPoint {
            name: "Spot".into(),
            monitor: 1,
            x: ScalarValue::Int(100),
            y: ScalarValue::Int(200),
        };
        assert_eq!(point_caption(&pt), "X: 100, Y: 200");
        let sa = ProgramSearchArea {
            name: "Box".into(),
            monitor: 1,
            left_x: ScalarValue::Int(10),
            top_y: ScalarValue::Int(20),
            right_x: ScalarValue::Int(110),
            bottom_y: ScalarValue::Int(80),
        };
        assert_eq!(
            search_area_caption(&sa),
            "Left: 10, Top: 20, Right: 110, Bottom: 80\n· area 100×60 px"
        );
        assert_eq!(
            collection_caption(&sa, 2, 2),
            "Left: 10, Top: 20, Right: 110, Bottom: 80\n· area 100×60 px\n· cell 50×30 px\n· 2 rows × 2 cols"
        );
    }

    #[test]
    fn coord_to_literal_rejects_variable_refs() {
        assert_eq!(coord_to_literal(&ScalarValue::Int(12)), Some(12));
        assert_eq!(
            coord_to_literal(&ScalarValue::String("40".into())),
            Some(40)
        );
        assert_eq!(coord_to_literal(&ScalarValue::String("${x}".into())), None);
        assert_eq!(coord_to_literal(&ScalarValue::Null), None);
    }

    fn catalog_two_monitors() -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080), (1920, 100, 1920, 1080)]);
        cat.create_program("Game").unwrap();
        cat
    }

    #[test]
    fn entity_preview_point_uses_monitor_origin() {
        let mut cat = catalog_two_monitors();
        cat.upsert_point(
            "Game",
            ProgramPoint {
                name: "Spot".into(),
                monitor: 2,
                x: ScalarValue::Int(10),
                y: ScalarValue::Int(20),
            },
        )
        .unwrap();
        let (_, _, coords) =
            entity_preview_spec(CatalogView::from(&cat), "Game", "Spot", PreviewKind::Point)
                .unwrap();
        match coords {
            PreviewCoords::Point { x, y } => assert_eq!((x, y), (1930, 120)),
            _ => panic!("expected point"),
        }
    }

    #[test]
    fn entity_preview_search_area_uses_monitor_origin() {
        let mut cat = catalog_two_monitors();
        cat.upsert_search_area(
            "Game",
            ProgramSearchArea {
                name: "Box".into(),
                monitor: 2,
                left_x: ScalarValue::Int(10),
                top_y: ScalarValue::Int(20),
                right_x: ScalarValue::Int(110),
                bottom_y: ScalarValue::Int(80),
            },
        )
        .unwrap();
        let (_, _, coords) = entity_preview_spec(
            CatalogView::from(&cat),
            "Game",
            "Box",
            PreviewKind::SearchArea,
        )
        .unwrap();
        match coords {
            PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid,
            } => {
                assert_eq!((left, top, right, bottom), (1930, 120, 2030, 180));
                assert_eq!(grid, None);
            }
            _ => panic!("expected search area"),
        }
    }

    #[test]
    fn entity_preview_collection_uses_linked_search_area_origin() {
        let mut cat = catalog_two_monitors();
        cat.upsert_search_area(
            "Game",
            ProgramSearchArea {
                name: "Box".into(),
                monitor: 2,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(100),
                bottom_y: ScalarValue::Int(100),
            },
        )
        .unwrap();
        cat.upsert_collection(
            "Game",
            sqyre_persist::ProgramCollection {
                name: "Box".into(),
                rows: 2,
                cols: 2,
            },
        )
        .unwrap();
        let (_, _, coords) = entity_preview_spec(
            CatalogView::from(&cat),
            "Game",
            "Box",
            PreviewKind::Collection,
        )
        .unwrap();
        match coords {
            PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid,
            } => {
                assert_eq!((left, top, right, bottom), (1920, 100, 2020, 200));
                assert_eq!(grid, Some((2, 2)));
            }
            _ => panic!("expected collection area"),
        }
    }
}
