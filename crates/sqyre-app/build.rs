//! Embed Windows PE icon / version resources for `sqyre.exe`.
//!
//! Explorer and shortcuts use the RT_GROUP_ICON in the executable, not the
//! runtime egui window icon from the SVG.

fn main() {
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
