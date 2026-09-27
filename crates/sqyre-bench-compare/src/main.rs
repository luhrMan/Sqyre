//! Comparative / regression micro-harness for Sqyre hot paths.
//!
//! Emits a single JSON document (schema_version 1) with stable section names so
//! results can be compared Go↔Rust or Rust-baseline↔current.
//!
//! ```text
//! cargo run -p sqyre-bench-compare --release -- --json
//! cargo run -p sqyre-bench-compare --release -- --section match_direct --json
//! ```

mod metrics;
mod sections;

use metrics::SectionResult;
use sections::{run_section, ALL_SECTIONS};
use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    impl_name: &'static str,
    git_rev: String,
    git_describe: String,
    host: HostInfo,
    iterations_default: u64,
    fixture_db: String,
    sections: Vec<SectionResult>,
}

#[derive(Serialize)]
struct HostInfo {
    os: String,
    arch: String,
    cpus: usize,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut json = false;
    let mut list = false;
    let mut isolate = false;
    let mut iterations: u64 = 40;
    let mut sections: Vec<String> = Vec::new();
    let mut fixture = default_fixture_db();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--list" => list = true,
            "--isolate" => isolate = true,
            "--iterations" => {
                i += 1;
                iterations = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| die("--iterations needs a positive integer"));
            }
            "--section" => {
                i += 1;
                let name = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| die("--section needs a name"));
                sections.push(name);
            }
            "--fixture-db" => {
                i += 1;
                fixture = PathBuf::from(
                    args.get(i)
                        .cloned()
                        .unwrap_or_else(|| die("--fixture-db needs a path")),
                );
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            other => die(&format!("unknown argument: {other}")),
        }
        i += 1;
    }

    if list {
        for name in ALL_SECTIONS {
            println!("{name}");
        }
        return;
    }

    if sections.is_empty() {
        sections = ALL_SECTIONS.iter().map(|s| (*s).to_string()).collect();
    }

    let results = if isolate && sections.len() > 1 {
        run_isolated(&sections, iterations, &fixture)
    } else {
        sections
            .iter()
            .map(|name| run_section(name, iterations, &fixture))
            .collect()
    };

    let report = Report {
        schema_version: 1,
        impl_name: "rust",
        git_rev: git_output(&["rev-parse", "HEAD"]),
        git_describe: git_output(&["describe", "--always", "--dirty"]),
        host: HostInfo {
            os: env::consts::OS.to_string(),
            arch: env::consts::ARCH.to_string(),
            cpus: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        },
        iterations_default: iterations,
        fixture_db: fixture.display().to_string(),
        sections: results,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report).expect("json"));
    } else {
        print_human(&report);
    }
}

fn run_isolated(sections: &[String], iterations: u64, fixture: &Path) -> Vec<SectionResult> {
    let exe = env::current_exe().expect("current_exe");
    let mut out = Vec::with_capacity(sections.len());
    for name in sections {
        let output = Command::new(&exe)
            .args([
                "--json",
                "--section",
                name,
                "--iterations",
                &iterations.to_string(),
                "--fixture-db",
                &fixture.display().to_string(),
            ])
            .output()
            .unwrap_or_else(|e| die(&format!("isolate spawn failed: {e}")));
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            out.push(SectionResult::error(
                name,
                format!("isolate child failed: {stderr}"),
            ));
            continue;
        }
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
                die(&format!(
                    "isolate child JSON parse failed for {name}: {e}\n{}",
                    String::from_utf8_lossy(&output.stdout)
                ))
            });
        let section = report
            .get("sections")
            .and_then(|s| s.as_array())
            .and_then(|a| a.first())
            .cloned();
        match section.and_then(|v| serde_json::from_value::<SectionResult>(v).ok()) {
            Some(s) => out.push(s),
            None => out.push(SectionResult::error(name, "isolate child missing section")),
        }
    }
    out
}

fn print_human(report: &Report) {
    println!(
        "sqyre-bench-compare  impl={}  rev={}  cpus={}",
        report.impl_name, report.git_describe, report.host.cpus
    );
    println!(
        "{:<24} {:>10} {:>12} {:>12} {:>10} {:>12}",
        "section", "status", "wall/iter", "cpu_user", "rss_kb", "io_rw"
    );
    for s in &report.sections {
        let wall = format_ns(s.wall_ns_per_iter);
        let cpu = format_ns(s.cpu_user_ns);
        let io = format!("{}+{}", s.io_read_bytes, s.io_write_bytes);
        println!(
            "{:<24} {:>10} {:>12} {:>12} {:>10} {:>12}",
            s.name, s.status, wall, cpu, s.peak_rss_kb, io
        );
        if let Some(reason) = &s.skip_reason {
            println!("  ↳ {reason}");
        }
    }
}

fn format_ns(ns: u64) -> String {
    if ns >= 1_000_000_000 {
        format!("{:.2}s", ns as f64 / 1e9)
    } else if ns >= 1_000_000 {
        format!("{:.2}ms", ns as f64 / 1e6)
    } else if ns >= 1_000 {
        format!("{:.1}µs", ns as f64 / 1e3)
    } else {
        format!("{ns}ns")
    }
}

fn default_fixture_db() -> PathBuf {
    // Prefer the shared harness fixture; fall back to persist test fixture.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."));
    let harness = root.join("scripts/bench-compare/fixtures/db.yaml");
    if harness.is_file() {
        return harness;
    }
    root.join("crates/sqyre-persist/tests/fixtures/db/catalog_and_actions.yaml")
}

fn git_output(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into())
}

fn print_help() {
    eprintln!(
        "Usage: sqyre-bench-compare [options]\n\
         \n\
         Options:\n\
           --json                 Emit JSON report on stdout\n\
           --list                 List stable section names\n\
           --section NAME         Run only NAME (repeatable)\n\
           --iterations N         Loops per section (default 40)\n\
           --fixture-db PATH      db.yaml for persist sections\n\
           --isolate              Re-exec each section for cleaner peak RSS\n\
           --help                 This text\n"
    );
}

fn die(msg: &str) -> ! {
    eprintln!("sqyre-bench-compare: {msg}");
    std::process::exit(2);
}
