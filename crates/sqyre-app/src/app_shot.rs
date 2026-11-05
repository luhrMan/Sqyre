//! Dropdown of launchable apps used when no previous screen has been saved.
//!
//! Android previews and point / search-area recording need a still of some other
//! app. When none has been seen yet, the user picks one here and Sqyre screenshots it.

use crate::help_text as ht;
use eframe::egui;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Sentence above the dropdown.
pub(crate) const CHOOSE_HEADING: &str = "Choose an app to take a screenshot of";

/// One app the user can open and screenshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShotApp {
    pub package: String,
    /// Launcher label; empty when Android has none.
    pub label: String,
}

impl ShotApp {
    /// Name to show and to speak in status text.
    pub(crate) fn shown(&self) -> &str {
        if self.label.is_empty() {
            &self.package
        } else {
            &self.label
        }
    }
}

/// What to do after a screenshot attempt fails.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ShotFollowUp {
    /// Show the dropdown and no error.
    Choose,
    /// Show the dropdown with this notice (the chosen app did not open).
    ChooseWithNotice(String),
    /// A different failure (permission, capture). Keep the error.
    ShowError(String),
}

/// `no_previous_app` is the "nothing was in front" miss. `chose_an_app` means the
/// attempt started from this dropdown, so a later miss stays on it.
pub(crate) fn shot_follow_up(
    no_previous_app: bool,
    chose_an_app: bool,
    message: String,
) -> ShotFollowUp {
    if no_previous_app {
        ShotFollowUp::Choose
    } else if chose_an_app {
        ShotFollowUp::ChooseWithNotice(message)
    } else {
        ShotFollowUp::ShowError(message)
    }
}

/// Status line while a still is being taken. `name` is [`ShotApp::shown`] or
/// "the previous app".
pub(crate) fn taking_picture_of(name: &str) -> String {
    format!("Taking a picture of {name}…")
}

/// Closed-combo and popup text. Two apps with the same label keep the package.
pub(crate) fn option_label(apps: &[ShotApp], app: &ShotApp) -> String {
    let name = app.shown();
    let dup = apps
        .iter()
        .filter(|other| other.shown().eq_ignore_ascii_case(name))
        .count()
        > 1;
    if dup {
        format!("{name} ({})", app.package)
    } else {
        name.to_string()
    }
}

pub(crate) fn option_labels(apps: &[ShotApp]) -> Vec<String> {
    apps.iter().map(|app| option_label(apps, app)).collect()
}

pub(crate) fn find_by_option<'a>(apps: &'a [ShotApp], option: &str) -> Option<&'a ShotApp> {
    apps.iter().find(|app| option_label(apps, app) == option)
}

/// Searchable app dropdown. `selected` stores the option label from [`option_label`].
/// Returns whether the selection changed.
pub(crate) fn paint_shot_combo(
    ui: &mut egui::Ui,
    id_salt: &str,
    options: &[String],
    selected: &mut String,
    width: Option<f32>,
) -> bool {
    use crate::widgets::{searchable_combo, SearchableComboOpts};
    searchable_combo(
        ui,
        id_salt,
        selected,
        SearchableComboOpts {
            options,
            empty_text: "Choose an app",
            none_label: None,
            width,
            on_option_hover: None,
            option_icon: None,
            closed_icon: false,
            option_label: None,
        },
    )
    .on_hover_text(ht::PICK_CHOOSE_APP)
    .changed()
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    /// Background `launchable_apps` query. Only started on Android.
    #[cfg_attr(
        not(target_os = "android"),
        expect(dead_code, reason = "only the Android loader constructs this")
    )]
    Loading(Receiver<Result<Vec<ShotApp>, String>>),
    Ready,
    Failed(String),
}

/// Cached launchable-app list and the dropdown selection.
#[derive(Default)]
pub(crate) struct AppList {
    apps: Vec<ShotApp>,
    selected: String,
    phase: Phase,
}

impl AppList {
    /// Start a background load the first time the dropdown is shown.
    #[cfg(target_os = "android")]
    pub(crate) fn ensure_loading(&mut self, ctx: &egui::Context) {
        if !matches!(self.phase, Phase::Idle) {
            return;
        }
        self.phase = Phase::Loading(start_load(ctx));
    }

    pub(crate) fn poll(&mut self, ctx: &egui::Context) {
        let result = match &self.phase {
            Phase::Loading(rx) => Some(rx.try_recv()),
            _ => None,
        };
        let Some(result) = result else {
            return;
        };
        match result {
            Ok(Ok(apps)) => {
                self.apps = apps;
                self.phase = Phase::Ready;
            }
            Ok(Err(message)) => {
                self.apps.clear();
                self.phase = Phase::Failed(message);
            }
            Err(TryRecvError::Empty) => ctx.request_repaint(),
            Err(TryRecvError::Disconnected) => {
                self.phase = Phase::Failed("Could not list apps.".into());
            }
        }
    }

    /// Paint the dropdown (or the load / empty state). `Some` when the user picks an app.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: &str,
        width: Option<f32>,
    ) -> Option<ShotApp> {
        let failed = match &self.phase {
            Phase::Idle | Phase::Loading(_) => {
                ui.label("Loading apps…");
                return None;
            }
            Phase::Failed(message) => Some(message.clone()),
            Phase::Ready => None,
        };
        if let Some(message) = failed {
            ui.colored_label(crate::theme::error_fg(), message);
            if ui.small_button("Try again").clicked() {
                self.phase = Phase::Idle;
            }
            return None;
        }
        if self.apps.is_empty() {
            ui.label("No other apps to open");
            return None;
        }
        let labels = option_labels(&self.apps);
        let changed = paint_shot_combo(ui, id_salt, &labels, &mut self.selected, width);
        if !changed {
            return None;
        }
        find_by_option(&self.apps, &self.selected).cloned()
    }
}

#[cfg(target_os = "android")]
fn start_load(ctx: &egui::Context) -> Receiver<Result<Vec<ShotApp>, String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let result = load_shot_apps();
        // The editor may have moved on; the still is not required then.
        let _ = tx.send(result);
        ctx.request_repaint();
    });
    rx
}

#[cfg(target_os = "android")]
fn load_shot_apps() -> Result<Vec<ShotApp>, String> {
    let apps = sqyre_android::bridge::launchable_apps().map_err(|e| e.to_string())?;
    Ok(apps
        .into_iter()
        .filter(|app| app.package != sqyre_capture::ANDROID_APP_PACKAGE)
        .map(|app| ShotApp {
            package: app.package,
            label: app.label,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(package: &str, label: &str) -> ShotApp {
        ShotApp {
            package: package.into(),
            label: label.into(),
        }
    }

    fn sample() -> Vec<ShotApp> {
        vec![
            app("com.android.chrome", "Chrome"),
            app("com.example.chrome", "Chrome"),
            app("com.google.maps", "Maps"),
            app("com.noname", ""),
        ]
    }

    #[test]
    fn duplicate_labels_keep_the_package() {
        let apps = sample();
        assert_eq!(
            option_labels(&apps),
            vec![
                "Chrome (com.android.chrome)",
                "Chrome (com.example.chrome)",
                "Maps",
                "com.noname",
            ]
        );
        assert_eq!(
            find_by_option(&apps, "Chrome (com.example.chrome)").map(|a| a.package.as_str()),
            Some("com.example.chrome")
        );
        assert_eq!(
            find_by_option(&apps, "Maps").map(|a| a.package.as_str()),
            Some("com.google.maps")
        );
    }

    #[test]
    fn no_previous_app_opens_the_chooser_without_an_error() {
        assert_eq!(
            shot_follow_up(
                true,
                false,
                "The previous app did not come to the front.".into()
            ),
            ShotFollowUp::Choose
        );
        assert_eq!(
            shot_follow_up(false, true, "Could not open Maps.".into()),
            ShotFollowUp::ChooseWithNotice("Could not open Maps.".into())
        );
        assert_eq!(
            shot_follow_up(false, false, "Screen recording was not allowed.".into()),
            ShotFollowUp::ShowError("Screen recording was not allowed.".into())
        );
        assert_eq!(taking_picture_of("Maps"), "Taking a picture of Maps…");
        assert!(!CHOOSE_HEADING.is_empty());
    }

    #[test]
    fn choosing_an_app_stores_its_label() {
        use egui_kittest::kittest::Queryable;

        let mut harness = egui_kittest::Harness::builder()
            .with_size([420.0, 640.0])
            .build_ui_state(
                |ui, list: &mut AppList| {
                    list.poll(ui.ctx());
                    let _ = list.show(ui, "shot", Some(240.0));
                },
                AppList {
                    apps: sample(),
                    selected: String::new(),
                    phase: Phase::Ready,
                },
            );
        harness.run();
        harness.get_by_role(egui::accesskit::Role::ComboBox).click();
        harness.run();
        harness.get_by_label("Maps").click();
        harness.run();
        assert_eq!(harness.state().selected, "Maps");
        assert_eq!(
            find_by_option(&harness.state().apps, &harness.state().selected)
                .map(|a| a.package.as_str()),
            Some("com.google.maps")
        );
    }
}
