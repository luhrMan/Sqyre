use super::icon_grid::item_tooltip_parts;
use super::items_grid::{sort_by_display_name, toggle_select_all_filtered};
use super::query::{query_matches_name_or_tags, query_matches_window};
use crate::window_types::WindowInfo;
use sqyre_persist::{ProgramCatalog, ProgramData, ProgramItem};
use std::collections::BTreeMap;

#[test]
fn sort_by_display_name_orders_case_insensitive() {
    let mut rows = vec![
        ("b".into(), "Zebra".into()),
        ("a".into(), "apple".into()),
        ("c".into(), "Banana".into()),
    ];
    sort_by_display_name(&mut rows);
    assert_eq!(
        rows.iter().map(|(_, d)| d.as_str()).collect::<Vec<_>>(),
        vec!["apple", "Banana", "Zebra"]
    );
}

#[test]
fn toggle_select_all_adds_then_clears_filtered() {
    let filtered = vec!["A~1".into(), "A~2".into()];
    let mut selected = vec!["B~9".into()];
    toggle_select_all_filtered(&mut selected, &filtered);
    assert!(selected.contains(&"A~1".into()));
    assert!(selected.contains(&"A~2".into()));
    assert!(selected.contains(&"B~9".into()));
    toggle_select_all_filtered(&mut selected, &filtered);
    assert_eq!(selected, vec!["B~9".to_string()]);
}

#[test]
fn empty_query_matches_anything() {
    assert!(query_matches_name_or_tags("", "Potion", &[]));
    assert!(query_matches_name_or_tags("", "x", &["healing".into()]));
}

#[test]
fn matches_name_substring() {
    assert!(query_matches_name_or_tags("pot", "HealthPotion", &[]));
    assert!(!query_matches_name_or_tags("sword", "HealthPotion", &[]));
}

#[test]
fn matches_name_fuzzy_subsequence() {
    assert!(query_matches_name_or_tags("hlt", "HealthPotion", &[]));
    assert!(query_matches_name_or_tags("HPT", "HealthPotion", &[]));
    assert!(!query_matches_name_or_tags("thl", "HealthPotion", &[])); // wrong order
}

#[test]
fn matches_tag_substring() {
    let tags = vec!["consumable".into(), "healing".into()];
    assert!(query_matches_name_or_tags("heal", "Minor Flask", &tags));
    assert!(query_matches_name_or_tags("CONSUM", "Minor Flask", &tags));
    assert!(!query_matches_name_or_tags("weapon", "Minor Flask", &tags));
}

#[test]
fn item_tooltip_parts_resolves_name_and_tags() {
    let mut cat = ProgramCatalog::default();
    cat.programs_mut().insert(
        "Game".into(),
        ProgramData {
            name: "Game".into(),
            items: BTreeMap::from([(
                "Flask".into(),
                ProgramItem {
                    name: "Health Flask".into(),
                    tags: vec!["healing".into(), "consumable".into()],
                    ..Default::default()
                },
            )]),
            ..Default::default()
        },
    );

    let (name, tags) = item_tooltip_parts(&cat, "Game~Flask");
    assert_eq!(name, "Health Flask");
    assert_eq!(tags, vec!["healing", "consumable"]);

    let (name, tags) = item_tooltip_parts(&cat, "Game~Flask~v2");
    assert_eq!(name, "Health Flask");
    assert_eq!(tags, vec!["healing", "consumable"]);

    let (name, tags) = item_tooltip_parts(&cat, "Missing~Item");
    assert_eq!(name, "Item");
    assert!(tags.is_empty());
}

#[test]
fn window_query_matches_title_name_or_path() {
    let w = WindowInfo {
        title: "Inbox — Mail".into(),
        process_name: "thunderbird".into(),
        process_path: "/usr/lib/thunderbird/thunderbird".into(),
        icon: None,
    };
    assert!(query_matches_window("", &w));
    assert!(query_matches_window("inbox", &w));
    assert!(query_matches_window("THUNDER", &w));
    assert!(query_matches_window("/usr/lib/thunder", &w));
    assert!(!query_matches_window("firefox", &w));
}
