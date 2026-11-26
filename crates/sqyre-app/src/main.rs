// Release GUI: no console window on Windows. Debug keeps a console for stderr.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "native-runtime")]
fn main() -> eframe::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _dhat = dhat::Profiler::new_heap();

    if sqyre_app::handle_cli_args() {
        return Ok(());
    }
    sqyre_capture::enable_per_monitor_dpi_v2();
    sqyre_app::run()
}

#[cfg(not(feature = "native-runtime"))]
fn main() -> eframe::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _dhat = dhat::Profiler::new_heap();

    if sqyre_app::handle_cli_args() {
        return Ok(());
    }
    sqyre_app::run()
}
