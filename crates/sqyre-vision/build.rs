//! Emit workspace root so runtime tessdata fallback does not hard-code `../` depth.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = find_workspace_root(&manifest);
    println!("cargo:rustc-env=SQYRE_WORKSPACE_ROOT={}", root.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("Cargo.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("assets/tessdata").display()
    );
}

fn find_workspace_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    loop {
        let cargo = dir.join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&cargo) {
            if text.lines().any(|l| l.trim() == "[workspace]") {
                return dir;
            }
        }
        if !dir.pop() {
            // Fallback: crate is expected at crates/<name>
            return start.join("../..");
        }
    }
}
