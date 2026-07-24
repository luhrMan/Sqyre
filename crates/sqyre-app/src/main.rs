// Release GUI: no console window on Windows. Debug keeps a console for stderr.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // Before egui/winit: physical pixels for capture, metrics, and input (Windows).
    sqyre_capture::enable_per_monitor_dpi_v2();
    sqyre_app::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    console_error_panic_hook::set_once();
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("no window")
            .document()
            .expect("no document");

        let canvas = document
            .get_element_by_id("sqyre_canvas")
            .expect("missing #sqyre_canvas in index.html")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#sqyre_canvas is not a canvas");

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(sqyre_app::SqyreApp::load_web(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
