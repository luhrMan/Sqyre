//! Embed Windows PE icon / version resources for `sqyre.exe`, and stamp
//! `SQYRE_VERSION` for the auto-updater.
//!
//! Explorer and shortcuts use the RT_GROUP_ICON in the executable, not the
//! runtime egui window icon from the SVG.

use std::path::PathBuf;

fn main() {
    emit_sqyre_version();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    println!("cargo:rerun-if-changed=assets/icons/sqyre.ico");

    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icons/sqyre.ico");
    // Prefer the branded product name over the crate name in file properties.
    res.set("ProductName", "Sqyre");
    res.set("FileDescription", "Sqyre");
    if let Err(err) = res.compile() {
        panic!("embed Windows icon resource: {err}");
    }
}

/// Prefer `RELEASE_VERSION` env, then a repo-root `VERSION` file, else `0.0.0-dev`.
fn emit_sqyre_version() {
    println!("cargo:rerun-if-env-changed=RELEASE_VERSION");

    let mut version = std::env::var("RELEASE_VERSION")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    // crates/sqyre-app → repo root
    let version_file = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("VERSION"));
    if let Some(ref path) = version_file {
        println!("cargo:rerun-if-changed={}", path.display());
        if version.is_none() {
            if let Ok(contents) = std::fs::read_to_string(path) {
                let trimmed = contents.trim();
                if !trimmed.is_empty() {
                    version = Some(trimmed.to_string());
                }
            }
        }
    }

    let version = version.unwrap_or_else(|| "0.0.0-dev".to_string());
    println!("cargo:rustc-env=SQYRE_VERSION={version}");
}
