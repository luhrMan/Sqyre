//! Action executors formerly in `misc` (I/O, flow, window, submacro).

mod flow;
mod io;
mod submacro;
mod window;

pub(crate) use flow::{execute_for_each_row, execute_pause, execute_while};
pub(crate) use io::{execute_save_variable, execute_set_variable};
pub(crate) use submacro::execute_run_macro;
pub(crate) use window::execute_focus_window;

#[cfg(test)]
mod tests {
    use crate::backends::RecordingBackend;
    use crate::run::{execute_macro, execute_macro_with, ExecDeps};
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, ConditionClause, ListColumn, Macro, ScalarValue,
    };
    use std::fs;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn set_evaluates_arithmetic_expressions() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "n".into(),
            type_: sqyre_domain::VariableType::Number,
            initial_value: "10".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                variable_name: "out".into(),
                value: sqyre_domain::ScalarValue::String("${n}*2+1".into()),
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            macro_.variables.get("out").map(|v| v.as_display()),
            Some("21".into())
        );
    }

    #[test]
    fn save_variable_clipboard_and_file() {
        let mut backend = RecordingBackend::default();
        let dir = tempfile::tempdir().unwrap();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "msg".into(),
            type_: sqyre_domain::VariableType::Text,
            initial_value: "hello".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::SaveVariable {
                    variable_name: "msg".into(),
                    destination: "clipboard".into(),
                    append: false,
                    append_newline: false,
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::SaveVariable {
                    variable_name: "msg".into(),
                    destination: "out.txt".into(),
                    append: false,
                    append_newline: true,
                },
            },
        ]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: Some(dir.path()),
            },
        )
        .unwrap();
        assert!(backend.log.iter().any(|e| e == "clipboard:hello"));
        assert_eq!(
            fs::read_to_string(dir.path().join("out.txt")).unwrap(),
            "hello\n"
        );
    }

    #[test]
    fn while_runs_until_condition_false() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "i".into(),
            type_: sqyre_domain::VariableType::Number,
            initial_value: "0".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::While {
                condition: sqyre_domain::ConditionBlock {
                    name: "inc".into(),
                    match_mode: "all".into(),
                    clauses: vec![ConditionClause {
                        left: ScalarValue::String("${i}".into()),
                        operator: "!=".into(),
                        right: ScalarValue::String("3".into()),
                    }],
                },
                max_iterations: 10,
                subactions: vec![
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(1),
                        },
                    },
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::SetVariable {
                            variable_name: "i".into(),
                            value: sqyre_domain::ScalarValue::String("${i}+1".into()),
                        },
                    },
                ],
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count(),
            3
        );
        assert_eq!(
            macro_.variables.get("i").map(|v| v.as_display()),
            Some("3".into())
        );
    }

    #[test]
    fn for_each_row_sets_vars_and_respects_range() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ForEachRow {
                name: "rows".into(),
                sources: vec![
                    ListColumn {
                        source: "a\nb\nc\nd".into(),
                        output_var: "letter".into(),
                        is_file: false,
                        skip_blank_lines: false,
                    },
                    ListColumn {
                        source: "1\n2\n3\n4".into(),
                        output_var: "digit".into(),
                        is_file: false,
                        skip_blank_lines: false,
                    },
                ],
                start_row: ScalarValue::Int(2),
                end_row: ScalarValue::Int(3),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    },
                }],
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count(),
            2
        );
        assert_eq!(
            macro_.variables.get("letter").map(|v| v.as_display()),
            Some("c".into())
        );
        assert_eq!(
            macro_.variables.get("digit").map(|v| v.as_display()),
            Some("3".into())
        );
        assert_eq!(
            macro_.variables.get("Row").map(|v| v.as_display()),
            Some("3".into())
        );
        assert_eq!(
            macro_.variables.get("RowCount").map(|v| v.as_display()),
            Some("4".into())
        );
    }

    #[test]
    fn for_each_row_continue_skips_rest() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ForEachRow {
                name: "rows".into(),
                sources: vec![ListColumn {
                    source: "a\nb".into(),
                    output_var: "letter".into(),
                    is_file: false,
                    skip_blank_lines: false,
                }],
                start_row: ScalarValue::Null,
                end_row: ScalarValue::Null,
                subactions: vec![
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Continue,
                    },
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(50),
                        },
                    },
                ],
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert!(!backend.log.iter().any(|e| e == "sleep:50"));
    }

    #[test]
    fn while_respects_stop_flag() {
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(true);
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variables.set("i", ScalarValue::Int(0));
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::While {
                condition: sqyre_domain::ConditionBlock {
                    name: "forever".into(),
                    match_mode: "all".into(),
                    clauses: vec![],
                },
                max_iterations: 100,
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    },
                }],
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: Some(&stop),
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        assert!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count()
                < 5
        );
    }

    #[test]
    fn run_macro_executes_target_children() {
        use crate::backends::MapMacroLookup;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        let mut helper = Macro::new("helper", 0, vec![]);
        helper.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(7),
            },
        }]);
        let lookup = MapMacroLookup {
            macros: BTreeMap::from([("helper".into(), Arc::new(helper))]),
        };

        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("caller", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::RunMacro {
                macro_name: "helper".into(),
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: None,
                icons: None,
                macros: Some(&lookup),
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        assert!(backend.log.iter().any(|e| e == "sleep:7"));
    }

    #[test]
    fn run_macro_respects_root_loop_count() {
        use crate::backends::MapMacroLookup;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        let mut helper = Macro::new("helper", 0, vec![]);
        // Root count 3 — must run children three times (not unwrap once).
        helper.root = Action {
            id: ActionId::root(),
            kind: ActionKind::Loop {
                name: "root".into(),
                count: ScalarValue::Int(3),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    },
                }],
            },
        };
        let lookup = MapMacroLookup {
            macros: BTreeMap::from([("helper".into(), Arc::new(helper))]),
        };

        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("caller", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::RunMacro {
                macro_name: "helper".into(),
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: None,
                icons: None,
                macros: Some(&lookup),
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count(),
            3
        );
    }

    #[test]
    fn pause_uses_continue_waiter() {
        use crate::backends::ImmediateContinueWaiter;

        let waiter = ImmediateContinueWaiter::default();
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Pause {
                message: "hold".into(),
                continue_key: vec!["f9".into()],
                pass_through: false,
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: Some(&waiter),
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        let log = waiter.log.lock().unwrap();
        assert!(log.iter().any(|e| e.contains("f9")));
    }

    #[test]
    fn focus_window_uses_focuser() {
        use crate::backends::RecordingWindowFocuser;

        let focuser = RecordingWindowFocuser::default();
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::FocusWindow {
                process_path: "/usr/bin/app".into(),
                window_title: "Title".into(),
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: Some(&focuser),
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        let log = focuser.log.lock().unwrap();
        assert_eq!(log.as_slice(), ["focus:/usr/bin/app:Title"]);
    }
}
