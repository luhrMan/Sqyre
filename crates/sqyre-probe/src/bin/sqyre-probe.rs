//! `sqyre-probe` — desktop capability probe for GNOME / Plasma / Cosmic parity.

use sqyre_probe::{run_probe_with_wait, ProbeOptions};
use std::env;

fn main() {
    let opts = parse_args();
    let (report, code) = run_probe_with_wait(&opts);

    if opts.human {
        print_human(&report, code);
    }
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        println!("{json}");
    } else {
        eprintln!("sqyre-probe: failed to serialize report");
        std::process::exit(2);
    }
    std::process::exit(code);
}

fn parse_args() -> ProbeOptions {
    let mut opts = ProbeOptions::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--human" => opts.human = true,
            "--json" => opts.human = false,
            "--wait-permissions" => {
                let secs = args.next().and_then(|s| s.parse().ok()).unwrap_or(60);
                opts.wait_permissions_secs = secs;
            }
            "--require" => {
                let list = args.next().unwrap_or_default();
                opts.required = list
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("sqyre-probe: unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }
    opts
}

fn print_help() {
    eprintln!(
        "\
sqyre-probe — Sqyre desktop capability probe

Usage:
  sqyre-probe [--json|--human] [--require CAP,...] [--wait-permissions SECS]

Options:
  --json                 Print JSON report to stdout (default)
  --human                Also print human summary on stderr
  --require CAPS         Comma-separated required capabilities (default: capture, windows, input, hotkeys, outline, grab)
  --wait-permissions N   Poll every 2s until required caps pass or N seconds elapse

Exit codes:
  0  all required capabilities ok (or skipped)
  1  one or more required capabilities failed
  2  probe infrastructure error
"
    );
}

fn print_human(report: &sqyre_probe::ProbeReport, code: i32) {
    eprintln!(
        "Session: {} ({:?})",
        report.session.session_type, report.session.desktop
    );
    if let Some(ref b) = report.session.capture_backend {
        eprintln!("Capture backend: {b}");
    }
    eprintln!("Parity tier: {}", report.parity_tier);
    for (key, cap) in &report.capabilities {
        let status = match cap.status {
            sqyre_probe::CapStatus::Ok => "ok",
            sqyre_probe::CapStatus::Fail => "FAIL",
            sqyre_probe::CapStatus::Skip => "skip",
            sqyre_probe::CapStatus::Pending => "pending",
        };
        eprintln!("  {key}: {status}");
        if let Some(ref e) = cap.error {
            eprintln!("    error: {e}");
        }
        if let Some(ref r) = cap.reason {
            eprintln!("    reason: {r}");
        }
    }
    if !report.permissions_needed.is_empty() {
        eprintln!("Permissions needed:");
        for h in &report.permissions_needed {
            eprintln!("  - {h}");
        }
    }
    eprintln!("Exit code: {code}");
}
