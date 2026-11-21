//! Migrate tests (separate from `migrate.rs` so it need not import `Database`).

use crate::{migrate_db_yaml, Database};

#[test]
fn migrates_legacy_snippets() {
    let text = include_str!("../tests/fixtures/db/legacy_inline.yaml");
    let out = migrate_db_yaml(text).expect("migrate");
    let db = Database::from_yaml(&out).expect("load");
    let m = &db.macros["Demo"];
    assert_eq!(m.variable_decls.len(), 1);
    assert_eq!(m.variable_decls[0].name, "delay");
    match &m.root.children()[0].kind {
        sqyre_domain::ActionKind::Click { button, .. } => {
            assert_eq!(*button, sqyre_domain::MouseButton::Left);
        }
        other => panic!("expected click, got {other:?}"),
    }
    match &m.root.children()[1].kind {
        sqyre_domain::ActionKind::Move { point, .. } => {
            assert_eq!(point.as_str(), "windows 10~image search reference");
        }
        other => panic!("expected move, got {other:?}"),
    }
    match &m.root.children()[2].kind {
        sqyre_domain::ActionKind::ImageSearch(d) => {
            assert_eq!(d.search_area.as_str(), "Arena Breakout Infinite~Mail Area");
            assert_eq!(
                d.detection.wait.repeat_mode,
                sqyre_domain::RepeatMode::WaitUntilFound
            );
        }
        other => panic!("expected imagesearch, got {other:?}"),
    }
}
