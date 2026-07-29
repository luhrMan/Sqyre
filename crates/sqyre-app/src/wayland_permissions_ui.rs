//! First-run Wayland desktop-permission wizard.

use eframe::egui;
use sqyre_persist::UserSettings;

#[derive(Debug, Default)]
pub struct WaylandPermissionsUi {
    /// Show the first-run / re-prompt modal.
    pub open: bool,
    busy: bool,
    last_message: Option<String>,
}

impl WaylandPermissionsUi {
    pub fn should_prompt_on_startup(settings: &UserSettings) -> bool {
        #[cfg(all(target_os = "linux", feature = "native-runtime"))]
        {
            sqyre_capture::is_wayland_backend() && !settings.wayland_permissions_prompted
        }
        #[cfg(not(all(target_os = "linux", feature = "native-runtime")))]
        {
            let _ = settings;
            false
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut UserSettings,
        on_changed: &mut dyn FnMut(&UserSettings),
    ) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        let mut grant = false;
        let mut skip = false;
        egui::Window::new("Desktop permissions")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    "Sqyre needs permission from your desktop to capture the screen, control the mouse and keyboard, and listen for global shortcuts on Wayland.",
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Your system will show permission dialogs. You can change these later in User Settings → Desktop permissions.",
                    )
                    .weak()
                    .small(),
                );
                if let Some(msg) = &self.last_message {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(msg).weak());
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Grant permissions"))
                        .clicked()
                    {
                        grant = true;
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Not now"))
                        .clicked()
                    {
                        skip = true;
                    }
                });
            });
        self.open = open;
        if skip {
            settings.wayland_permissions_prompted = true;
            let _ = settings.save_default();
            on_changed(settings);
            self.open = false;
        } else if grant {
            self.run_request(settings, on_changed);
        }
    }

    fn run_request(
        &mut self,
        settings: &mut UserSettings,
        on_changed: &mut dyn FnMut(&UserSettings),
    ) {
        self.busy = true;
        #[cfg(all(target_os = "linux", feature = "native-runtime"))]
        {
            let results = sqyre_capture::request_all_permissions();
            settings.wayland_screen_capture = results.screen_capture;
            settings.wayland_input_control = results.input_control;
            settings.wayland_global_shortcuts = results.global_shortcuts;
            settings.wayland_permissions_prompted = true;
            let mut parts = Vec::new();
            if results.screen_capture {
                parts.push("screen capture granted".to_string());
            } else if let Some(e) = results.screen_error {
                parts.push(e);
            }
            if results.input_control {
                parts.push("input control granted".into());
            } else if let Some(e) = results.input_error {
                parts.push(e);
            }
            if results.global_shortcuts {
                parts.push("shortcuts granted".into());
            } else if let Some(e) = results.shortcuts_error {
                parts.push(e);
            }
            self.last_message = Some(parts.join(" · "));
            let _ = settings.save_default();
            on_changed(settings);
            if results.screen_capture || results.input_control || results.global_shortcuts {
                self.open = false;
            }
        }
        #[cfg(not(all(target_os = "linux", feature = "native-runtime")))]
        {
            let _ = (settings, on_changed);
            self.last_message = Some("Wayland permissions are only used on Linux.".into());
        }
        self.busy = false;
    }
}
