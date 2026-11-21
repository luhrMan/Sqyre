//! Unit tests for navigate select/key (kept out of `navigate.rs` to avoid a run↔navigate cycle).

use super::*;
use crate::run::{execute_macro_with, ExecDeps};
use crate::test_support::{macro_of, ImmediateContinueWaiter, RecordingBackend};
use crate::test_support::{one_atlas, FixedResolver};
use sqyre_domain::{
    NavChords, NavInputs, NavOptions, NavOutputs, NavSelectAction, NavigateSelectData, PressState,
};

#[test]
fn select_exits_and_writes_outputs() {
    let mut backend = RecordingBackend::default();
    let waiter = ImmediateContinueWaiter::queued(&[0]); // select is only chord
    let resolver = one_bag_resolver();
    let mut macro_ = macro_of(vec![Action::new(ActionKind::NavigateSelect(Box::new(
        NavigateSelectData {
            program: "P".into(),
            atlas: "inventory".into(),
            chords: enter_chords(),
            options: NavOptions {
                move_cursor_with_nav: true,
                ..Default::default()
            },
            select: NavSelectAction::default(),
            inputs: NavInputs {
                row: "2".into(),
                col: "3".into(),
                ..Default::default()
            },
            outputs: NavOutputs {
                output_ref: "ref".into(),
                output_atlas: "a".into(),
                output_row: "r".into(),
                output_col: "c".into(),
                output_collection: "col".into(),
            },
            ..Default::default()
        },
    )))]);

    execute_macro_with(
        &mut macro_,
        ExecDeps::new(&mut backend)
            .resolver(&resolver)
            .continue_waiter(&waiter),
    )
    .unwrap();

    assert_eq!(
        macro_.variables.get("ref").map(|v| v.as_display()),
        Some("P~bag@2,3-2,3".into())
    );
    assert_eq!(
        macro_.variables.get("a").map(|v| v.as_display()),
        Some("inventory".into())
    );
    assert_eq!(
        macro_.variables.get("col").map(|v| v.as_display()),
        Some("bag".into())
    );
    assert_eq!(
        macro_.variables.get("r").map(|v| v.as_display()),
        Some("2".into())
    );
    assert_eq!(
        macro_.variables.get("c").map(|v| v.as_display()),
        Some("3".into())
    );
    assert!(backend.log.iter().any(|e| e.starts_with("move:")));
    assert!(backend.log.iter().any(|e| e.contains("click:left:down")));
    assert!(backend.log.iter().any(|e| e.contains("click:left:up")));
}

#[test]
fn navigate_key_branch_runs_children() {
    let mut backend = RecordingBackend::default();
    let waiter = ImmediateContinueWaiter::queued(&[0]); // index 0 = only Nav Key chord
    let resolver = one_atlas("bag", 2, 2, (0, 0, 100, 100));
    let mut macro_ = macro_of(vec![Action::new(ActionKind::NavigateSelect(Box::new(
        NavigateSelectData {
            program: "P".into(),
            atlas: "inventory".into(),
            chords: NavChords::default(),
            outputs: NavOutputs {
                output_row: "r".into(),
                output_col: "c".into(),
                ..Default::default()
            },
            subactions: vec![Action::new(ActionKind::NavigateKey {
                name: "Inspect".into(),
                chord: vec!["i".into()],
                exit: true,
                subactions: vec![Action::new(ActionKind::Click {
                    button: sqyre_domain::MouseButton::Right,
                    state: PressState::Down,
                })],
            })],
            ..Default::default()
        },
    )))]);

    execute_macro_with(
        &mut macro_,
        ExecDeps::new(&mut backend)
            .resolver(&resolver)
            .continue_waiter(&waiter),
    )
    .unwrap();

    backend.assert_logged("click:right:down");
    assert_eq!(
        macro_.variables.get("r").map(|v| v.as_display()),
        Some("1".into())
    );
}

#[test]
fn walks_right_into_neighbor_collection() {
    let mut backend = RecordingBackend::default();
    // chords: up, down, left, right, select — index 3 = right, then select
    let waiter = ImmediateContinueWaiter::queued(&[3, 4]);
    let resolver = FixedResolver::with_atlas(
        vec![
            crate::test_support::atlas_member("A", 2, 2, (0, 0, 100, 100)),
            crate::test_support::atlas_member("B", 2, 2, (120, 0, 220, 100)),
        ],
        vec!["A".into(), "B".into()],
    );
    let mut macro_ = macro_of(vec![Action::new(ActionKind::NavigateSelect(Box::new(
        NavigateSelectData {
            program: "P".into(),
            atlas: "inv".into(),
            chords: NavChords {
                up: vec!["up".into()],
                down: vec!["down".into()],
                left: vec!["left".into()],
                right: vec!["right".into()],
                select: vec!["enter".into()],
                ..Default::default()
            },
            options: NavOptions {
                wrap_edges: false,
                move_cursor_with_nav: false,
                ..Default::default()
            },
            inputs: NavInputs {
                collection: "A".into(),
                row: "1".into(),
                col: "2".into(),
                ..Default::default()
            },
            outputs: NavOutputs {
                output_collection: "col".into(),
                output_row: "r".into(),
                output_col: "c".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )))]);

    execute_macro_with(
        &mut macro_,
        ExecDeps::new(&mut backend)
            .resolver(&resolver)
            .continue_waiter(&waiter),
    )
    .unwrap();

    assert_eq!(
        macro_.variables.get("col").map(|v| v.as_display()),
        Some("B".into())
    );
    assert_eq!(
        macro_.variables.get("c").map(|v| v.as_display()),
        Some("1".into())
    );
}

fn enter_chords() -> NavChords {
    NavChords {
        select: vec!["enter".into()],
        ..Default::default()
    }
}

/// Standard "P~inventory" nav select; pass chords (or `NavChords::default()`).
fn nav_select(chords: NavChords) -> NavigateSelectData {
    NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords,
        ..Default::default()
    }
}

fn nav_macro(data: NavigateSelectData) -> Macro {
    macro_of(vec![Action::new(ActionKind::NavigateSelect(Box::new(
        data,
    )))])
}

fn one_bag_resolver() -> FixedResolver {
    one_atlas("bag", 3, 4, (0, 0, 120, 90))
}

fn run_nav(
    macro_: &mut Macro,
    backend: &mut RecordingBackend,
    resolver: Option<&dyn crate::backends::CoordinateResolver>,
    waiter: Option<&dyn crate::backends::ContinueKeyWaiter>,
) -> crate::error::Result<()> {
    let mut deps = ExecDeps::new(backend);
    deps.resolver = resolver;
    deps.continue_waiter = waiter;
    execute_macro_with(macro_, deps)
}

fn err_text(result: crate::error::Result<()>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn errors_when_atlas_unset() {
    let mut backend = RecordingBackend::default();
    let resolver = one_bag_resolver();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: String::new(),
        chords: enter_chords(),
        ..Default::default()
    });
    assert_eq!(
        err_text(run_nav(&mut macro_, &mut backend, Some(&resolver), None)),
        "navigate select: atlas not set"
    );
}

#[test]
fn errors_without_coordinate_resolver() {
    let mut backend = RecordingBackend::default();
    let mut macro_ = nav_macro(nav_select(NavChords {
        select: vec!["enter".into()],
        ..Default::default()
    }));
    assert_eq!(
        err_text(run_nav(&mut macro_, &mut backend, None, None)),
        "navigate select: coordinate resolver not configured"
    );
}

#[test]
fn errors_when_atlas_has_no_collections() {
    let mut backend = RecordingBackend::default();
    let resolver = FixedResolver::with_atlas(vec![], vec![]);
    let mut macro_ = nav_macro(nav_select(NavChords {
        select: vec!["enter".into()],
        ..Default::default()
    }));
    let err = err_text(run_nav(&mut macro_, &mut backend, Some(&resolver), None));
    assert!(err.contains("has no collections"), "{err}");
}

#[test]
fn errors_on_invalid_collection_grid() {
    let mut backend = RecordingBackend::default();
    let resolver = one_atlas("flat", 0, 2, (0, 0, 10, 10));
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inv".into(),
        chords: enter_chords(),
        ..Default::default()
    });
    let err = err_text(run_nav(&mut macro_, &mut backend, Some(&resolver), None));
    assert!(err.contains("invalid grid 0x2"), "{err}");
}

#[test]
fn errors_on_unknown_start_collection() {
    let mut backend = RecordingBackend::default();
    let resolver = one_bag_resolver();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords: enter_chords(),
        inputs: NavInputs {
            collection: "satchel".into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let err = err_text(run_nav(&mut macro_, &mut backend, Some(&resolver), None));
    assert!(err.contains("not in atlas"), "{err}");
}

#[test]
fn errors_without_any_chords() {
    let mut backend = RecordingBackend::default();
    let resolver = one_bag_resolver();
    let mut macro_ = nav_macro(nav_select(NavChords::default()));
    let err = err_text(run_nav(&mut macro_, &mut backend, Some(&resolver), None));
    assert!(err.contains("no chords configured"), "{err}");
}

#[test]
fn errors_without_continue_waiter() {
    let mut backend = RecordingBackend::default();
    let resolver = one_bag_resolver();
    let mut macro_ = nav_macro(nav_select(NavChords {
        select: vec!["enter".into()],
        ..Default::default()
    }));
    let err = err_text(run_nav(&mut macro_, &mut backend, Some(&resolver), None));
    assert!(err.contains("key wait is not available"), "{err}");
}

/// Waiter that fails every wait with a fixed error.
struct FailingWaiter {
    err: PortError,
}

impl crate::backends::ContinueKeyWaiter for FailingWaiter {
    fn wait_for_continue(
        &self,
        _keys: &[String],
        _pass_through: bool,
        _stop: &AtomicBool,
    ) -> std::result::Result<(), PortError> {
        Err(self.err.clone())
    }

    fn wait_for_any_chord(
        &self,
        _chords: &[Vec<String>],
        _hold_repeat: &[bool],
        _pass_through: bool,
        _stop: &AtomicBool,
    ) -> std::result::Result<usize, PortError> {
        Err(self.err.clone())
    }
}

#[test]
fn propagates_waiter_error() {
    let mut backend = RecordingBackend::default();
    let resolver = one_bag_resolver();
    let waiter = FailingWaiter {
        err: PortError::Message("waiter boom".into()),
    };
    let mut macro_ = nav_macro(nav_select(NavChords {
        select: vec!["enter".into()],
        ..Default::default()
    }));
    let err = err_text(run_nav(
        &mut macro_,
        &mut backend,
        Some(&resolver),
        Some(&waiter),
    ));
    assert!(err.contains("waiter boom"), "{err}");
}

#[test]
fn stopped_waiter_exits_cleanly() {
    // PortError::Stopped is a user stop, not a failure: the macro run
    // resolves without error (FlowSignal::Stopped → Ok at the top level).
    let mut backend = RecordingBackend::default();
    let resolver = one_bag_resolver();
    let waiter = FailingWaiter {
        err: PortError::Stopped,
    };
    let mut macro_ = nav_macro(nav_select(NavChords {
        select: vec!["enter".into()],
        ..Default::default()
    }));
    run_nav(&mut macro_, &mut backend, Some(&resolver), Some(&waiter)).unwrap();
}

#[test]
fn standalone_navigate_key_errors() {
    let mut backend = RecordingBackend::default();
    let mut macro_ = macro_of(vec![Action::new(ActionKind::NavigateKey {
        name: "Inspect".into(),
        chord: vec!["i".into()],
        exit: false,
        subactions: vec![],
    })]);
    let err = err_text(run_nav(&mut macro_, &mut backend, None, None));
    assert!(err.contains("only runs as a child"), "{err}");
}

#[test]
fn kind_mismatch_errors() {
    let mut backend = RecordingBackend::default();
    let mut exec = Executor::new(&mut backend);
    let action = Action::new(ActionKind::Wait {
        time: ScalarValue::Int(1),
    });
    let mut macro_ = Macro::new("t", 0, vec![]);
    let err = execute_navigate_select(&mut exec, &action, &mut macro_, crate::run::run_children)
        .unwrap_err();
    assert!(err.to_string().contains("kind mismatch"), "{err}");
}

#[test]
fn walks_up_down_left_then_back() {
    // chords pushed in order: up(0), down(1), left(2), right(3), select(4), back(5)
    let waiter = ImmediateContinueWaiter::queued(&[0, 1, 2, 5]);
    let resolver = one_bag_resolver();
    let mut backend = RecordingBackend::default();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords: NavChords {
            up: vec!["up".into()],
            down: vec!["down".into()],
            left: vec!["left".into()],
            right: vec!["right".into()],
            select: vec!["enter".into()],
            back: vec!["esc".into()],
        },
        options: NavOptions {
            wrap_edges: false,
            move_cursor_with_nav: false,
            ..Default::default()
        },
        inputs: NavInputs {
            row: "2".into(),
            col: "2".into(),
            ..Default::default()
        },
        outputs: NavOutputs {
            output_row: "r".into(),
            output_col: "c".into(),
            ..Default::default()
        },
        ..Default::default()
    });

    run_nav(&mut macro_, &mut backend, Some(&resolver), Some(&waiter)).unwrap();

    // up → (1,2); down → (2,2); left → (2,1); back exits writing outputs.
    assert_eq!(
        macro_.variables.get("r").map(|v| v.as_display()),
        Some("2".into())
    );
    assert_eq!(
        macro_.variables.get("c").map(|v| v.as_display()),
        Some("1".into())
    );
}

#[test]
fn wraps_around_edge_when_enabled() {
    // chords: left(0), select(1) — left at col 1 with wrap → last col (4)
    let waiter = ImmediateContinueWaiter::queued(&[0, 1]);
    let resolver = one_bag_resolver();
    let mut backend = RecordingBackend::default();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords: NavChords {
            left: vec!["left".into()],
            select: vec!["enter".into()],
            ..Default::default()
        },
        options: NavOptions {
            wrap_edges: true,
            move_cursor_with_nav: false,
            ..Default::default()
        },
        outputs: NavOutputs {
            output_row: "r".into(),
            output_col: "c".into(),
            ..Default::default()
        },
        ..Default::default()
    });

    run_nav(&mut macro_, &mut backend, Some(&resolver), Some(&waiter)).unwrap();

    assert_eq!(
        macro_.variables.get("c").map(|v| v.as_display()),
        Some("4".into())
    );
}

#[test]
fn start_position_resolves_from_variables() {
    let waiter = ImmediateContinueWaiter::queued(&[0]);
    let resolver = one_bag_resolver();
    let mut backend = RecordingBackend::default();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords: enter_chords(),
        inputs: NavInputs {
            row: "startRow".into(),
            col: "startCol".into(),
            ..Default::default()
        },
        outputs: NavOutputs {
            output_row: "r".into(),
            output_col: "c".into(),
            ..Default::default()
        },
        ..Default::default()
    });
    // Runtime variables re-init from declarations when the macro starts.
    macro_.upsert_variable(sqyre_domain::VariableDecl {
        name: "startRow".into(),
        type_: sqyre_domain::VariableType::Number,
        initial_value: "3".into(),
        ..Default::default()
    });
    macro_.upsert_variable(sqyre_domain::VariableDecl {
        name: "startCol".into(),
        type_: sqyre_domain::VariableType::Number,
        initial_value: "4".into(),
        ..Default::default()
    });

    run_nav(&mut macro_, &mut backend, Some(&resolver), Some(&waiter)).unwrap();

    assert_eq!(
        macro_.variables.get("r").map(|v| v.as_display()),
        Some("3".into())
    );
    assert_eq!(
        macro_.variables.get("c").map(|v| v.as_display()),
        Some("4".into())
    );
}

#[test]
fn keyboard_select_presses_key() {
    let waiter = ImmediateContinueWaiter::queued(&[0]);
    let resolver = one_bag_resolver();
    let mut backend = RecordingBackend::default();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords: enter_chords(),
        select: NavSelectAction {
            device: NavSelectDevice::Keyboard,
            key: "e".into(),
            ..Default::default()
        },
        ..Default::default()
    });

    run_nav(&mut macro_, &mut backend, Some(&resolver), Some(&waiter)).unwrap();

    backend.assert_logged("keydown:e");
    backend.assert_logged("keyup:e");
}

#[test]
fn keyboard_select_empty_key_errors() {
    let waiter = ImmediateContinueWaiter::queued(&[0]);
    let resolver = one_bag_resolver();
    let mut backend = RecordingBackend::default();
    let mut macro_ = nav_macro(NavigateSelectData {
        program: "P".into(),
        atlas: "inventory".into(),
        chords: enter_chords(),
        select: NavSelectAction {
            device: NavSelectDevice::Keyboard,
            key: String::new(),
            ..Default::default()
        },
        ..Default::default()
    });

    let err = err_text(run_nav(
        &mut macro_,
        &mut backend,
        Some(&resolver),
        Some(&waiter),
    ));
    assert!(err.contains("select key not set"), "{err}");
}
