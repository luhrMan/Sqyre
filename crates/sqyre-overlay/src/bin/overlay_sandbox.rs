//! Minimal eframe harness for native X11 overlay buttons.
//!
//! ```text
//! cargo run -p sqyre-overlay --features sandbox --bin overlay_sandbox
//! ```

use eframe::egui;
use parking_lot::Mutex;
use sqyre_overlay::MacroOverlay;
use sqyre_persist::{OverlayButtonConfig, ProgramCatalog};
use std::sync::Arc;

struct SandboxApp {
    overlay: MacroOverlay,
    buttons: Vec<OverlayButtonConfig>,
    catalog: ProgramCatalog,
    pending: Arc<Mutex<Vec<String>>>,
    last_click: Option<String>,
}

impl SandboxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        sqyre_overlay::register_phosphor_family(&mut fonts);
        cc.egui_ctx.set_fonts(fonts);

        let mut btn = OverlayButtonConfig::new("sandbox", "Sandbox");
        btn.enabled = true;
        btn.macro_name = "sandbox".into();
        btn.label = "Go".into();
        btn.icon = "play".into();
        btn.x = 120.0;
        btn.y = 120.0;
        btn.size = 48.0;
        btn.corner_radius = 10.0;
        btn.border_width = 2.0;
        btn.border_color = "#dc9d2e".into();
        btn.bg_color = "#14120e".into();
        btn.bg_alpha = 255;
        btn.icon_color = "#f5e6c0".into();
        btn.icon_hover_color = "#dc9d2e".into();

        Self {
            overlay: MacroOverlay::new(),
            buttons: vec![btn],
            catalog: ProgramCatalog::default(),
            pending: Arc::new(Mutex::new(Vec::new())),
            last_click: None,
        }
    }
}

impl eframe::App for SandboxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("sqyre-overlay sandbox (native X11)");
            ui.label("Override-redirect button is a separate X11 window (not egui viewport).");
            ui.label("Hover the button for a native tip; clicks stay on the X11 thread.");
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if let Some(name) = &self.last_click {
                ui.colored_label(egui::Color32::LIGHT_GREEN, format!("last click → {name}"));
            }
        });

        self.overlay.sync(
            &ctx,
            &self.buttons,
            None,
            &self.catalog,
            &self.pending,
            None,
            false,
        );
        let drained: Vec<String> = self.pending.lock().drain(..).collect();
        if let Some(name) = drained.last() {
            self.last_click = Some(name.clone());
            ctx.request_repaint();
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 280.0])
            .with_title("sqyre-overlay sandbox"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "sqyre-overlay sandbox",
        options,
        Box::new(|cc| Ok(Box::new(SandboxApp::new(cc)))),
    )
}
