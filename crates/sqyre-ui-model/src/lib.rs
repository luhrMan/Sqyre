//! Sqyre UI chrome: display params, tree summary pills, action colors and glyphs.
//!
//! Semantic variable bindings ([`sqyre_domain::BindingRole`],
//! [`sqyre_domain::VariableBinding`]) stay in `sqyre-domain`; this crate only
//! adds presentation (pill/tree/color) on top.

mod colors;
mod display;
mod icons;

pub use colors::*;
pub use display::*;
pub use icons::*;

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, ConditionBlock, CoordinateOutputs, CoordinateRef,
        DetectionBranch, MatchMode, MouseButton, RepeatMode, ScalarValue, VariableAssignment,
        WaitTilFoundConfig,
    };

    #[test]
    fn loop_iterations_come_before_name() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "batch".into(),
                count: ScalarValue::Int(3),
                subactions: vec![],
            },
        };
        let params = a.display_params();
        let (summary, _) = split_display_params(&params);
        assert_eq!(summary[0].label, "Iterations");
        assert_eq!(summary[0].value, "3");
        assert_eq!(summary[1].label, "Name");
    }

    #[test]
    fn set_variable_multi_assignments_in_pills() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![
                    VariableAssignment::new("a", ScalarValue::Int(1)),
                    VariableAssignment::new("b", ScalarValue::String("x".into())),
                ],
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills
            .iter()
            .any(|p| p.prefix.as_deref() == Some("Variable") && p.text == "a"));
        assert!(pills
            .iter()
            .any(|p| p.prefix.as_deref() == Some("Variable") && p.text == "b"));
        assert!(pills.iter().any(|p| p.text == "1"));
        assert!(pills.iter().any(|p| p.text == "x"));
    }

    #[test]
    fn wait_summary_includes_time_ms() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(250),
            },
        };
        let pills = a.tree_summary_pills();
        assert_eq!(pills.len(), 1);
        assert_eq!(pills[0].text, "250 ms");
    }

    #[test]
    fn image_search_tree_omits_items() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["a".into(), "b".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.9,
                blur: 0,
                detection: DetectionBranch::default(),
            },
        };
        let tree = a.display_params_for_tree();
        assert!(!tree.iter().any(|p| p.label == "Items"));
        let full = a.display_params();
        assert!(full.iter().any(|p| p.label == "Items" && p.value == "2"));
    }

    #[test]
    fn pastel_wait_differs_from_mouse() {
        clear_all_custom_action_colors();
        let wait = action_pastel_color("wait", false);
        let click = action_pastel_color("click", false);
        assert_ne!(wait, click);
    }

    #[test]
    fn custom_action_color_overrides_builtin() {
        clear_all_custom_action_colors();
        let custom = [0x11, 0x22, 0x33, 0xFF];
        set_custom_action_color(sqyre_domain::ACTION_COLOR_KEY_MOUSE_KEYBOARD, custom);
        assert_eq!(action_pastel_color("click", false), custom);
        assert_ne!(
            action_pastel_color("click", false),
            default_action_pastel_color("click", false)
        );
        clear_custom_action_color(sqyre_domain::ACTION_COLOR_KEY_MOUSE_KEYBOARD);
        assert_eq!(
            action_pastel_color("click", false),
            default_action_pastel_color("click", false)
        );
        clear_all_custom_action_colors();
    }

    #[test]
    fn set_variable_uses_binding_prefix() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![VariableAssignment::new("count", ScalarValue::Int(1))],
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills
            .iter()
            .any(|p| p.prefix.as_deref() == Some("Variable") && p.text == "count"));
    }

    #[test]
    fn looks_like_var_ref_detects_dollar_and_braces() {
        assert!(looks_like_var_ref("${foo}"));
        assert!(looks_like_var_ref("  {bar}  "));
        assert!(!looks_like_var_ref("plain"));
        assert!(!looks_like_var_ref("${unclosed"));
        assert!(!looks_like_var_ref("{"));
    }

    #[test]
    fn split_filters_type_empty_and_splits_extra() {
        let params = vec![
            DisplayParam::new("Type", "move"),
            DisplayParam::new("Point", "Prog~A"),
            DisplayParam::new("Empty", "  "),
            DisplayParam::extra("Smooth", "true"),
        ];
        let (summary, extra) = split_display_params(&params);
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].label, "Point");
        assert_eq!(extra.len(), 1);
        assert_eq!(extra[0].label, "Smooth");
    }

    #[test]
    fn move_smooth_extras_are_tooltip_only() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef("Prog~Spot".into()),
                smooth: true,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            },
        };
        let params = a.display_params();
        let (summary, extra) = split_display_params(&params);
        assert!(summary.iter().any(|p| p.label == "Point"));
        assert!(extra.iter().any(|p| p.label == "Smooth"));
        assert!(extra.iter().any(|p| p.label == "Smooth low"));
    }

    #[test]
    fn conditional_summary_omits_clauses_from_pills() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Conditional {
                condition: ConditionBlock {
                    name: "gate".into(),
                    match_mode: MatchMode::Any,
                    clauses: vec![
                        sqyre_domain::ConditionClause {
                            left: ScalarValue::String("${a}".into()),
                            operator: "==".into(),
                            right: ScalarValue::String("1".into()),
                        },
                        sqyre_domain::ConditionClause {
                            left: ScalarValue::String("${b}".into()),
                            operator: "is set".into(),
                            right: ScalarValue::Null,
                        },
                    ],
                },
                subactions: vec![],
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills.iter().any(|p| p.text == "gate"));
        assert!(pills.iter().any(|p| p.text.contains("any (OR)")));
        assert!(!pills
            .iter()
            .any(|p| p.text.contains("==") || p.text.contains("is set")));
        let params = a.display_params();
        let (_, extra) = split_display_params(&params);
        assert!(extra
            .iter()
            .any(|p| p.label == "If" && p.value.contains("${a} == 1")));
    }

    #[test]
    fn while_summary_omits_clauses_from_pills() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::While {
                condition: ConditionBlock {
                    name: "spin".into(),
                    match_mode: MatchMode::All,
                    clauses: vec![sqyre_domain::ConditionClause {
                        left: ScalarValue::String("${n}".into()),
                        operator: "<".into(),
                        right: ScalarValue::Int(10),
                    }],
                },
                max_iterations: 5,
                subactions: vec![],
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills.iter().any(|p| p.text == "spin"));
        assert!(pills.iter().any(|p| p.text.contains("all (AND)")));
        assert!(!pills
            .iter()
            .any(|p| p.text.contains("${n}") || p.text.contains("<")));
        let params = a.display_params();
        let (_, extra) = split_display_params(&params);
        assert!(extra
            .iter()
            .any(|p| p.label == "While" && p.value.contains("${n} < 10")));
        assert!(extra
            .iter()
            .any(|p| p.label == "Max iterations" && p.value == "5"));
    }

    #[test]
    fn find_pixel_shows_color_not_outputs_or_search_area() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::FindPixel {
                name: "red".into(),
                search_area: CoordinateRef("Prog~Box".into()),
                target_color: "#ff0000".into(),
                color_tolerance: 10,
                detection: DetectionBranch {
                    coords: CoordinateOutputs {
                        output_x_variable: "px".into(),
                        output_y_variable: "py".into(),
                    },
                    ..DetectionBranch::default()
                },
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills.iter().any(|p| p.text == "red"));
        assert!(pills.iter().any(|p| p.text == "#ff0000"));
        assert!(!pills.iter().any(|p| p.text == "px" || p.text == "py"));
        assert!(!pills.iter().any(|p| p.text.contains("Box")));
        assert_eq!(action_icon_glyph(&a), "🎨");
    }

    #[test]
    fn image_search_tree_omits_search_area_and_outputs() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["a".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.9,
                blur: 0,
                detection: DetectionBranch {
                    coords: CoordinateOutputs {
                        output_x_variable: "x".into(),
                        output_y_variable: "y".into(),
                    },
                    ..DetectionBranch::default()
                },
            },
        };
        let pills = a.tree_summary_pills();
        assert_eq!(pills.len(), 1);
        assert_eq!(pills[0].text, "find");
        let params = a.display_params();
        let (_, extra) = split_display_params(&params);
        assert!(extra.iter().any(|p| p.label == "Search Area"));
    }

    #[test]
    fn set_binding_uses_value_role() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![VariableAssignment::new(
                    "sum",
                    ScalarValue::String("1+2".into()),
                )],
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills
            .iter()
            .any(|p| { p.prefix.as_deref() == Some("Variable") && p.text == "sum" }));
    }

    #[test]
    fn pastel_dark_differs_from_light_and_glyphs_flip_with_state() {
        assert_ne!(
            action_pastel_color("click", false),
            action_pastel_color("click", true)
        );
        assert_ne!(nested_var_ref_color(false), nested_var_ref_color(true));
        let down = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: MouseButton::Left,
                state: true,
            },
        };
        let up = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: MouseButton::Left,
                state: false,
            },
        };
        assert_eq!(action_icon_glyph(&down), "⬇");
        assert_eq!(action_icon_glyph(&up), "⬆");
    }

    #[test]
    fn action_icon_glyphs_avoid_tofu_codepoints() {
        // These previously used Mathematical Alphanumeric / obscure symbols
        // missing from egui's proportional fonts (empty boxes).
        let cases = [
            (
                ActionKind::Type {
                    text: String::new(),
                    delay_ms: 0,
                },
                "⌨",
            ),
            (
                ActionKind::FocusWindow {
                    process_path: String::new(),
                    window_title: String::new(),
                },
                "👁",
            ),
            (
                ActionKind::SetVariable {
                    assignments: vec![VariableAssignment::new(
                        "a",
                        ScalarValue::String("1+2".into()),
                    )],
                },
                "x",
            ),
            (
                ActionKind::ForEachRow {
                    name: String::new(),
                    sources: vec![],
                    start_row: ScalarValue::Null,
                    end_row: ScalarValue::Null,
                    subactions: vec![],
                },
                "☰",
            ),
            (
                ActionKind::Ocr {
                    name: String::new(),
                    target: String::new(),
                    search_area: CoordinateRef(String::new()),
                    output_variable: String::new(),
                    blur: 0,
                    min_threshold: 0,
                    resize: 1.0,
                    grayscale: false,
                    threshold_otsu: false,
                    threshold_invert: false,
                    detection: DetectionBranch::default(),
                },
                "🔤",
            ),
            (
                ActionKind::ImageSearch {
                    name: String::new(),
                    targets: vec![],
                    search_area: CoordinateRef(String::new()),
                    tolerance: 0.0,
                    blur: 0,
                    detection: DetectionBranch::default(),
                },
                "🔍",
            ),
        ];
        for (kind, want) in cases {
            let a = Action {
                id: ActionId::new(),
                kind,
            };
            assert_eq!(action_icon_glyph(&a), want, "{}", a.type_key());
        }
    }

    #[test]
    fn parse_hex_short_form_and_display_text() {
        assert_eq!(
            sqyre_domain::parse_hex_color("#abc"),
            Some([0xaa, 0xbb, 0xcc, 255])
        );
        assert_eq!(sqyre_domain::parse_hex_color("not-hex"), None);
        let pill = SummaryPill {
            text: "x".into(),
            prefix: Some("X".into()),
        };
        assert_eq!(pill.display_text(), "X: x");
    }

    #[test]
    fn wait_display_mode_and_clause_summary() {
        let mut wait = WaitTilFoundConfig::default();
        assert_eq!(wait.display_wait_mode("instant"), "instant");
        wait.repeat_mode = RepeatMode::WaitUntilFound;
        wait.wait_til_found_seconds = 5;
        assert_eq!(
            wait.display_wait_mode("instant"),
            "5 seconds or until found"
        );
        let clause = sqyre_domain::ConditionClause {
            left: ScalarValue::String("name".into()),
            operator: "contains".into(),
            right: ScalarValue::String("foo".into()),
        };
        assert_eq!(clause.summary(), "name contains foo");
    }

    #[test]
    fn root_loop_still_type_keys_as_loop() {
        // Sanity: ui-model tests can use domain constructors freely.
        let root = root_loop(vec![]);
        assert_eq!(root.type_key(), "loop");
    }
}
