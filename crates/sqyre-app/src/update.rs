//! Background update check / download / apply for native builds.

use sqyre_update::{ReleaseAsset, UpdateStatus};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

/// Embedded release stamp from `build.rs` (`RELEASE_VERSION` / `VERSION` / `0.0.0-dev`).
pub const SQYRE_VERSION: &str = env!("SQYRE_VERSION");

#[derive(Debug, Clone, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    /// Local/unstamped or unsupported install — checks are skipped.
    Unavailable {
        reason: String,
    },
    Checking,
    UpToDate,
    Available {
        version: String,
        asset: ReleaseAsset,
    },
    Downloading {
        version: String,
    },
    Ready {
        version: String,
    },
    Failed {
        message: String,
    },
}

enum WorkerMsg {
    Check(Result<UpdateStatus, String>),
    Apply(Result<String, String>),
}

pub struct UpdateManager {
    pub state: UpdateState,
    /// User dismissed the in-app update banner for the current Available version.
    pub banner_dismissed: bool,
    rx: Option<mpsc::Receiver<WorkerMsg>>,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            banner_dismissed: false,
            rx: None,
        }
    }
}

impl UpdateManager {
    pub fn is_busy(&self) -> bool {
        matches!(
            self.state,
            UpdateState::Checking | UpdateState::Downloading { .. }
        ) || self.rx.is_some()
    }

    /// Kick off a background `check_latest` if not already working.
    pub fn start_check(&mut self) {
        if matches!(
            self.state,
            UpdateState::Checking | UpdateState::Downloading { .. }
        ) || self.rx.is_some()
        {
            return;
        }
        // Local / unstamped builds never self-update.
        if is_dev_version(SQYRE_VERSION) {
            self.state = UpdateState::Unavailable {
                reason: format!(
                    "Dev build ({SQYRE_VERSION}) — set RELEASE_VERSION (or a VERSION file) when building to enable updates"
                ),
            };
            return;
        }
        self.banner_dismissed = false;
        self.state = UpdateState::Checking;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let current = SQYRE_VERSION.to_string();
        thread::Builder::new()
            .name("sqyre-update-check".into())
            .spawn(move || {
                let result = sqyre_update::check_latest(&current).map_err(|e| e.to_string());
                let _ = tx.send(WorkerMsg::Check(result));
            })
            .expect("spawn update check thread");
    }

    /// Download + apply the available update in a background thread.
    pub fn start_download(&mut self) {
        let (version, asset) = match &self.state {
            UpdateState::Available { version, asset } => (version.clone(), asset.clone()),
            _ => return,
        };
        if self.rx.is_some() {
            return;
        }
        self.state = UpdateState::Downloading {
            version: version.clone(),
        };
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::Builder::new()
            .name("sqyre-update-apply".into())
            .spawn(move || {
                let result = (|| {
                    let staged =
                        sqyre_update::download_and_stage(&asset).map_err(|e| e.to_string())?;
                    sqyre_update::apply(staged).map_err(|e| e.to_string())?;
                    Ok(version)
                })();
                let _ = tx.send(WorkerMsg::Apply(result));
            })
            .expect("spawn update apply thread");
    }

    /// Poll the worker channel; returns `true` if state changed.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = self.rx.take() else {
            return false;
        };
        match rx.try_recv() {
            Ok(WorkerMsg::Check(Ok(UpdateStatus::UpToDate))) => {
                self.state = UpdateState::UpToDate;
                true
            }
            Ok(WorkerMsg::Check(Ok(UpdateStatus::Available { version, asset }))) => {
                self.banner_dismissed = false;
                self.state = UpdateState::Available { version, asset };
                true
            }
            Ok(WorkerMsg::Check(Err(message))) => {
                if message.contains("do not auto-update") {
                    self.state = UpdateState::Unavailable {
                        reason: format!(
                            "Dev build ({SQYRE_VERSION}) — stamp RELEASE_VERSION when building to enable updates"
                        ),
                    };
                } else if message.contains("not supported on this platform") {
                    self.state = UpdateState::Unavailable {
                        reason: "Auto-update is not supported on this platform".into(),
                    };
                } else {
                    self.state = UpdateState::Failed { message };
                }
                true
            }
            Ok(WorkerMsg::Apply(Ok(version))) => {
                self.state = UpdateState::Ready { version };
                true
            }
            Ok(WorkerMsg::Apply(Err(message))) => {
                self.state = UpdateState::Failed { message };
                true
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.rx = Some(rx);
                false
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                if matches!(
                    self.state,
                    UpdateState::Checking | UpdateState::Downloading { .. }
                ) {
                    self.state = UpdateState::Failed {
                        message: "update worker exited unexpectedly".into(),
                    };
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn available_version(&self) -> Option<&str> {
        match &self.state {
            UpdateState::Available { version, .. }
            | UpdateState::Downloading { version }
            | UpdateState::Ready { version } => Some(version.as_str()),
            _ => None,
        }
    }

    pub fn show_banner(&self) -> bool {
        matches!(self.state, UpdateState::Available { .. }) && !self.banner_dismissed
    }

    pub fn dismiss_banner(&mut self) {
        self.banner_dismissed = true;
    }
}

pub fn note_check_time(settings: &mut sqyre_persist::UserSettings) {
    settings.last_update_check_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
}

fn is_dev_version(version: &str) -> bool {
    version == "0.0.0-dev" || version.ends_with("-dev")
}

/// Drop the single-instance lock and re-exec / spawn the (updated) binary.
pub fn restart_app(instance_lock: &mut Option<crate::single_instance::InstanceLock>) {
    // Release lock before the new process tries to acquire it.
    *instance_lock = None;
    if let Err(e) = sqyre_update::restart() {
        eprintln!("sqyre: restart after update failed: {e}");
    }
}
