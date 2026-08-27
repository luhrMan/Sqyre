//! Host-only desktop parity integration test.
//!
//! Run on a machine with a graphical session:
//!   cargo test -p sqyre-probe --test linux_desktop_parity -- --ignored --nocapture

use sqyre_probe::{run_probe, CapStatus, ProbeOptions};

#[test]
fn probe_json_serializes() {
    let report = run_probe(&ProbeOptions::default());
    let json = serde_json::to_string(&report).expect("serialize");
    assert!(json.contains("parity_tier"));
    assert!(json.contains("capture.open"));
}

#[test]
#[ignore = "requires graphical Linux session (X11 or XWayland)"]
fn linux_desktop_capture_open() {
    let report = run_probe(&ProbeOptions::default());
    let cap = report
        .capabilities
        .get("capture.open")
        .expect("capture.open probed");
    assert!(
        cap.status == CapStatus::Ok || cap.status == CapStatus::Skip,
        "capture.open: {:?}",
        cap
    );
}

#[test]
#[ignore = "requires graphical Linux session"]
fn linux_full_parity_tier() {
    let report = run_probe(&ProbeOptions::default());
    assert_eq!(
        report.parity_tier, "full",
        "permissions_needed: {:?}",
        report.permissions_needed
    );
}
