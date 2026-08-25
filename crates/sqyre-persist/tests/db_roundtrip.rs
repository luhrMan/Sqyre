//! Golden `db.yaml` decode → validate → encode → decode (structural equality).

use sqyre_domain::Macro;
use sqyre_persist::Database;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/db")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn kind_keys(macro_: &Macro) -> Vec<&'static str> {
    let mut keys = Vec::new();
    macro_.root.walk(&mut |a| keys.push(a.type_key()));
    keys
}

fn assert_roundtrip(yaml: &str) {
    let (first, warnings) = Database::from_yaml_with_warnings(yaml).expect("decode fixture");
    assert!(
        warnings.is_empty(),
        "fixture should load without skipped macros: {warnings:?}"
    );
    assert!(!first.macros.is_empty(), "fixture should contain macros");

    for (key, macro_) in &first.macros {
        sqyre_validate::validate_macro(macro_)
            .unwrap_or_else(|e| panic!("validate macro {key:?}: {e}"));
    }

    let encoded = first.to_yaml().expect("encode");
    let second = Database::from_yaml(&encoded).expect("decode encoded yaml");

    let names_a: BTreeSet<_> = first.macros.keys().cloned().collect();
    let names_b: BTreeSet<_> = second.macros.keys().cloned().collect();
    assert_eq!(names_a, names_b, "macro keys");

    for (key, a) in &first.macros {
        let b = &second.macros[key];
        assert_eq!(a.name, b.name, "macro name for {key}");
        assert_eq!(kind_keys(a), kind_keys(b), "action kinds for {key}");
        assert_eq!(
            a.root.children().len(),
            b.root.children().len(),
            "root child count for {key}"
        );
    }

    let cat_a = first.program_catalog().expect("catalog a");
    let cat_b = second.program_catalog().expect("catalog b");
    let prog_a: BTreeSet<_> = cat_a.program_names().cloned().collect();
    let prog_b: BTreeSet<_> = cat_b.program_names().cloned().collect();
    assert_eq!(prog_a, prog_b, "program names");
    for name in &prog_a {
        let pa = cat_a.get(name).expect("program a");
        let pb = cat_b.get(name).expect("program b");
        assert_eq!(
            pa.items.keys().collect::<Vec<_>>(),
            pb.items.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            pa.points.keys().collect::<Vec<_>>(),
            pb.points.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            pa.search_areas.keys().collect::<Vec<_>>(),
            pb.search_areas.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn minimal_db_roundtrip_validates() {
    assert_roundtrip(&fixture("minimal.yaml"));
}

#[test]
fn catalog_and_actions_roundtrip_validates() {
    assert_roundtrip(&fixture("catalog_and_actions.yaml"));
}
