//! First-start popup: explains each permission this platform needs, then requests them
//! all once the user clicks Continue. Nothing prompts while it is showing.

use eframe::egui::{self, Key, Modifiers, RichText};

/// Popup body width on wide windows; narrower windows use the dialog bounds instead.
const MAX_BODY_WIDTH: f32 = 440.0;

/// One permission line in the popup.
struct Item {
    title: &'static str,
    why: &'static str,
    /// Shell command the user must run themselves (Sqyre cannot grant it).
    copy_command: Option<&'static str>,
}

pub(crate) struct PermissionsIntro {
    items: Vec<Item>,
    note: Option<&'static str>,
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Showing,
    #[cfg(target_os = "android")]
    Requesting(android::Step),
    Finished,
}

impl PermissionsIntro {
    pub(crate) fn new() -> Self {
        let (items, note) = platform::items();
        Self {
            items,
            note,
            state: State::Showing,
        }
    }

    /// True while the popup still waits for Continue; startup prompts stay deferred.
    pub(crate) fn showing(&self) -> bool {
        self.state == State::Showing
    }

    pub(crate) fn finished(&self) -> bool {
        self.state == State::Finished
    }

    /// Paint the popup and advance any OS requests. Returns `true` on the frame the user
    /// clicked Continue.
    pub(crate) fn show(&mut self, ctx: &egui::Context) -> bool {
        #[cfg(target_os = "android")]
        if let State::Requesting(step) = self.state {
            self.state = match android::advance(step) {
                Some(next) => State::Requesting(next),
                None => State::Finished,
            };
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        if self.finished() {
            return false;
        }

        let mut confirmed = false;
        let id = egui::Id::new("sqyre_permissions_intro");
        let bounds = crate::widgets::dialog_constrain_rect(ctx);
        let frame = egui::Frame::popup(&ctx.global_style());
        let margin = frame.total_margin().sum();
        let body_w = (bounds.width() - margin.x).clamp(0.0, MAX_BODY_WIDTH);
        let area = egui::Modal::default_area(id)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(bounds.center())
            .constrain_to(bounds);
        // Continue is the only way out: Esc and backdrop clicks (`should_close`) are ignored.
        egui::Modal::new(id).area(area).frame(frame).show(ctx, |ui| {
            ui.set_width(body_w);
            ui.label(RichText::new("Before you start").strong().heading());
            ui.label(
                RichText::new(
                    "Sqyre needs a few permissions to run macros. Nothing is asked until you click Continue.",
                )
                .weak(),
            );
            crate::widgets::section_separator(ui);
            // Footer height (button row) stays outside the scroll so Continue is always visible.
            let footer_h = ui.spacing().interact_size.y + crate::theme::SPACE_8 * 2.0;
            let scroll_h = (bounds.height() - margin.y - ui.min_rect().height() - footer_h)
                .max(ui.spacing().interact_size.y);
            egui::ScrollArea::vertical()
                .id_salt("sqyre_permissions_intro_body")
                .max_height(scroll_h)
                .show(ui, |ui| {
                    for item in &self.items {
                        paint_item(ui, ctx, item);
                        ui.add_space(crate::theme::SPACE_8);
                    }
                    if let Some(note) = self.note {
                        ui.label(RichText::new(note).weak().small());
                        ui.add_space(crate::theme::SPACE_8);
                    }
                    ui.label(
                        RichText::new("You can review these any time in Settings → Permissions.")
                            .weak()
                            .small(),
                    );
                });
            ui.add_space(crate::theme::SPACE_8);
            if self.showing() {
                let clicked = ui
                    .button("Continue")
                    .on_hover_text("Show the system permission prompts now.")
                    .clicked();
                let enter = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                confirmed = clicked || enter;
            } else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak("Waiting for the system prompts…");
                });
            }
        });

        if confirmed {
            self.state = platform::start();
        }
        confirmed
    }
}

fn paint_item(ui: &mut egui::Ui, ctx: &egui::Context, item: &Item) {
    ui.label(RichText::new(item.title).strong());
    ui.label(RichText::new(item.why).small());
    if let Some(cmd) = item.copy_command {
        ui.horizontal_wrapped(|ui| {
            ui.monospace(cmd);
            if ui.button("Copy").clicked() {
                ctx.copy_text(cmd.to_string());
            }
        });
    }
}

#[cfg(not(target_os = "android"))]
const HOTKEYS_WHY: &str =
    "Sqyre listens for your macro hotkeys and the Esc stop key while other apps are in front.";
#[cfg(any(target_os = "linux", target_os = "windows"))]
const CAPTURE_WHY: &str = "Image search, OCR, and Find Pixel look at the screen to find things.";

#[cfg(target_os = "linux")]
mod platform {
    use super::{Item, State, CAPTURE_WHY, HOTKEYS_WHY};

    pub(super) fn items() -> (Vec<Item>, Option<&'static str>) {
        if !sqyre_capture::shared_capturer_open_may_block() {
            return (
                vec![
                    Item {
                        title: "Global hotkeys",
                        why: HOTKEYS_WHY,
                        copy_command: None,
                    },
                    Item {
                        title: "Screen capture",
                        why: CAPTURE_WHY,
                        copy_command: None,
                    },
                ],
                Some("Your desktop does not ask for these, so no prompts will appear."),
            );
        }
        let mut items = vec![
            Item {
                title: "Screen sharing",
                why: "Your desktop asks which screens Sqyre may see. Image search, OCR, and Find Pixel need every screen.",
                copy_command: None,
            },
            Item {
                title: "Remote interaction",
                why: "In the same dialog, turn on Allow Remote Interaction so macros can click and type.",
                copy_command: None,
            },
            Item {
                title: "Global shortcuts",
                why: "Your desktop may ask to register the keys that start macros.",
                copy_command: None,
            },
        ];
        if !sqyre_hotkeys::linux_can_open_evdev() {
            items.push(Item {
                title: "Input devices",
                why: "Hotkeys while other apps are focused need read access to keyboards. Run this, then log out and back in:",
                copy_command: Some("sudo usermod -aG input $USER"),
            });
        }
        (items, None)
    }

    /// The deferred portal probe and hotkey start resume once the popup is gone.
    pub(super) fn start() -> State {
        State::Finished
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{Item, State, CAPTURE_WHY, HOTKEYS_WHY};

    pub(super) fn items() -> (Vec<Item>, Option<&'static str>) {
        (
            vec![
                Item {
                    title: "Global hotkeys",
                    why: HOTKEYS_WHY,
                    copy_command: None,
                },
                Item {
                    title: "Screen capture",
                    why: CAPTURE_WHY,
                    copy_command: None,
                },
            ],
            Some(
                "Windows does not ask for these. Apps running as administrator only receive Sqyre's clicks and hotkeys when Sqyre runs as administrator too.",
            ),
        )
    }

    pub(super) fn start() -> State {
        State::Finished
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Item, State, HOTKEYS_WHY};

    pub(super) fn items() -> (Vec<Item>, Option<&'static str>) {
        (
            vec![Item {
                title: "Global hotkeys",
                why: HOTKEYS_WHY,
                copy_command: None,
            }],
            Some("Screen capture is not supported on macOS yet."),
        )
    }

    pub(super) fn start() -> State {
        State::Finished
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::{android, Item, State};

    pub(super) fn items() -> (Vec<Item>, Option<&'static str>) {
        (
            vec![
                Item {
                    title: "Notifications",
                    why: "Shows Stop and Continue for a running macro.",
                    copy_command: None,
                },
                Item {
                    title: "Screen recording",
                    why: "Lets image search, OCR, Find Pixel, and previews see the screen. Android asks again each time Sqyre starts.",
                    copy_command: None,
                },
                Item {
                    title: "Accessibility service",
                    why: "Lets macros tap, swipe, and type. Turn on Sqyre in the settings page that opens.",
                    copy_command: None,
                },
            ],
            None,
        )
    }

    pub(super) fn start() -> State {
        android::start()
    }
}

#[cfg(target_os = "android")]
mod android {
    use super::State;
    use sqyre_android::{bridge, frames, Projection};

    /// Prompts run one at a time; each waits for the previous dialog to close.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Step {
        Notifications,
        Projection,
    }

    pub(super) fn start() -> State {
        if let Err(e) = bridge::request_notifications() {
            crate::log::warn(format_args!("permissions intro: notifications: {e}"));
        }
        State::Requesting(Step::Notifications)
    }

    /// Next step once `step`'s dialog closed, `None` when every prompt has been shown.
    pub(super) fn advance(step: Step) -> Option<Step> {
        match step {
            Step::Notifications => {
                if bridge::notifications_prompt_open() {
                    return Some(step);
                }
                frames().rearm();
                match bridge::request_projection() {
                    Ok(()) => Some(Step::Projection),
                    Err(e) => {
                        crate::log::warn(format_args!("permissions intro: screen recording: {e}"));
                        open_accessibility();
                        None
                    }
                }
            }
            Step::Projection => {
                if frames().projection() == Projection::NotStarted {
                    return Some(step);
                }
                open_accessibility();
                None
            }
        }
    }

    fn open_accessibility() {
        let result = match bridge::accessibility_enabled() {
            Ok(true) => Ok(()),
            Ok(false) | Err(_) => bridge::open_accessibility_settings(),
        };
        if let Err(e) = result {
            crate::log::warn(format_args!("permissions intro: accessibility: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_showing_with_items() {
        let intro = PermissionsIntro::new();
        assert!(intro.showing());
        assert!(!intro.finished());
        assert!(!intro.items.is_empty());
    }

    #[test]
    fn fits_inside_narrow_window() {
        let ctx = egui::Context::default();
        let mut intro = PermissionsIntro::new();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 480.0));
        // Areas settle their size over a couple of passes.
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                intro.show(ui.ctx());
            })
            .drop_without_applying_deltas();
        }
        let rect = ctx
            .memory(|m| m.area_rect(egui::Id::new("sqyre_permissions_intro")))
            .expect("intro area painted");
        let bounds = crate::widgets::dialog_constrain_rect(&ctx);
        assert!(
            bounds.expand(0.5).contains_rect(rect),
            "intro {rect:?} must stay inside {bounds:?}"
        );
    }

    #[test]
    fn continue_leaves_showing_state() {
        let ctx = egui::Context::default();
        let mut intro = PermissionsIntro::new();
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        let mut confirmed = false;
        ctx.run_ui(input, |ui| confirmed = intro.show(ui.ctx()))
            .drop_without_applying_deltas();
        assert!(confirmed);
        assert!(!intro.showing());
        #[cfg(not(target_os = "android"))]
        assert!(intro.finished());
    }
}
