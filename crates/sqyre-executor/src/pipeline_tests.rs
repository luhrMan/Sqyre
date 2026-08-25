//! YAML fixture → `execute_macro_with` mocks (no live capture/input).

use crate::test_support::RecordingBackend;
use crate::{execute_macro_with, lines_for, ExecDeps, SharedActionLog};
use sqyre_serialize::decode_macro_from_yaml;

const PIPELINE_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pipeline_wait_click.yaml"
));

const LOOP_IF_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pipeline_loop_if.yaml"
));

#[test]
fn wait_then_click_records_backend_and_logs() {
    let mut macro_ = decode_macro_from_yaml(PIPELINE_YAML).expect("decode pipeline fixture");
    let wait_id = macro_.root.children()[0].id;
    let click_id = macro_.root.children()[1].id;

    let mut backend = RecordingBackend::default();
    let logger = SharedActionLog::new();
    execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).logger(&logger))
        .expect("execute pipeline");

    assert_eq!(
        backend.log,
        ["sleep:5", "click:left:down", "click:left:up"],
        "Wait then tap click"
    );

    let wait_lines = lines_for(&logger.entries_for(wait_id));
    let click_lines = lines_for(&logger.entries_for(click_id));
    assert!(
        wait_lines.iter().any(|l| l.starts_with("Wait:")),
        "wait log: {wait_lines:?}"
    );
    assert!(
        click_lines.iter().any(|l| l.starts_with("Click:")),
        "click log: {click_lines:?}"
    );
}

#[test]
fn loop_then_true_if_records_repeated_clicks_and_waits() {
    let mut macro_ = decode_macro_from_yaml(LOOP_IF_YAML).expect("decode loop/if fixture");
    let mut backend = RecordingBackend::default();
    let logger = SharedActionLog::new();
    execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).logger(&logger))
        .expect("execute loop/if pipeline");

    assert_eq!(
        backend.log,
        [
            "sleep:1",
            "click:left:down",
            "click:left:up",
            "sleep:1",
            "click:left:down",
            "click:left:up",
            "sleep:1",
        ],
        "loop 2× (wait+click) then true conditional wait"
    );
}
