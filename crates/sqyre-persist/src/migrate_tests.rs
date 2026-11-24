//! Migrate tests (separate from `migrate.rs` so it need not import `Database`).

use crate::{migrate_db_yaml, migrate_db_yaml_value, Database};
use serde_yaml::Value;

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

#[test]
fn migrate_edge_branches_keep_coverage() {
    // Non-mapping root is a no-op.
    let mut null = Value::Null;
    migrate_db_yaml_value(&mut null);
    assert!(null.is_null());

    // Non-mapping `programs` still walks macros; missing macros key returns early.
    let mut no_macros: Value = serde_yaml::from_str("programs: []\n").unwrap();
    migrate_db_yaml_value(&mut no_macros);

    let yaml = r#"
programs:
  Game:
    coordinates:
      1x1:
        searchareas:
          area_key:
            name: Area Display
            leftx: 0
            topy: 0
            rightx: 1
            bottomy: 1
          "":
            name: ignored
        points:
          pt_key:
            name: Point Display
            x: 1
            y: 2
          "":
            name: ignored
      bad: not-a-mapping
  Skipped: {}
macros:
  Demo:
    name: Demo
    variables:
      foundx: 1
      "": 9
      empty: ""
      flag: true
      ratio: 1.5
      numstr: "42"
      text: hello
      nest: { a: 1 }
    root:
      type: loop
      name: root
      count: 1
      subactions:
        - type: imagesearch
          searcharea:
            name: Area Display
          waittilfound: false
          elseactions:
            - type: wait
              time: 1
          subactions:
            - type: click
              button: true
        - type: move
          point:
            name: Missing Point
        - type: move
          point:
            name: "  "
        - type: move
          point: 123
  EmptyVars:
    name: EmptyVars
    variables:
      variables: {}
    root:
      type: loop
      name: root
      count: 1
      subactions: []
  BuiltinOnly:
    name: BuiltinOnly
    variables:
      foundx: 1
      foundy: 2
    root:
      type: loop
      name: root
      count: 1
      subactions: []
  FlatVars:
    name: FlatVars
    variables:
      delay: 3
    root:
      type: loop
      name: root
      count: 1
      subactions: []
  NoRoot:
    name: NoRoot
  NotMap: []
"#;
    let mut root: Value = serde_yaml::from_str(yaml).unwrap();
    migrate_db_yaml_value(&mut root);

    let macros = root
        .as_mapping()
        .unwrap()
        .get(Value::String("macros".into()))
        .and_then(|v| v.as_mapping())
        .unwrap();
    let demo = macros
        .get(Value::String("Demo".into()))
        .and_then(|v| v.as_mapping())
        .unwrap();
    let vars = demo
        .get(Value::String("variables".into()))
        .and_then(|v| v.as_sequence())
        .unwrap();
    let names: Vec<&str> = vars
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"flag"));
    assert!(names.contains(&"ratio"));
    assert!(names.contains(&"numstr"));
    assert!(names.contains(&"text"));
    assert!(!names.contains(&"foundx"));
    assert!(!names.contains(&"empty"));
    assert!(!names.contains(&"nest"));

    let root_action = demo
        .get(Value::String("root".into()))
        .and_then(|v| v.as_mapping())
        .unwrap();
    let subs = root_action
        .get(Value::String("subactions".into()))
        .and_then(|v| v.as_sequence())
        .unwrap();
    let imagesearch = subs[0].as_mapping().unwrap();
    assert_eq!(
        imagesearch
            .get(Value::String("searcharea".into()))
            .and_then(|v| v.as_str()),
        Some("Game~area_key")
    );
    assert!(imagesearch
        .get(Value::String("elseactions".into()))
        .and_then(|v| v.as_sequence())
        .is_some());
    let click = imagesearch
        .get(Value::String("subactions".into()))
        .and_then(|v| v.as_sequence())
        .unwrap()[0]
        .as_mapping()
        .unwrap();
    assert_eq!(
        click
            .get(Value::String("button".into()))
            .and_then(|v| v.as_str()),
        Some("right")
    );
    assert_eq!(
        subs[1]
            .as_mapping()
            .unwrap()
            .get(Value::String("point".into()))
            .and_then(|v| v.as_str()),
        Some("Missing Point")
    );
    assert!(!subs[2]
        .as_mapping()
        .unwrap()
        .contains_key(Value::String("point".into())));

    let empty_vars = macros
        .get(Value::String("EmptyVars".into()))
        .and_then(|v| v.as_mapping())
        .unwrap();
    assert!(!empty_vars.contains_key(Value::String("variables".into())));

    let builtin_only = macros
        .get(Value::String("BuiltinOnly".into()))
        .and_then(|v| v.as_mapping())
        .unwrap();
    assert!(!builtin_only.contains_key(Value::String("variables".into())));

    let flat = macros
        .get(Value::String("FlatVars".into()))
        .and_then(|v| v.as_mapping())
        .unwrap();
    assert_eq!(
        flat.get(Value::String("variables".into()))
            .and_then(|v| v.as_sequence())
            .map(|s| s.len()),
        Some(1)
    );
}
