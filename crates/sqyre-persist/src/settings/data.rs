//! Data directory, backup, update, and session lifecycle preferences.

use serde::{Deserialize, Serialize};

pub const DEFAULT_BACKUP_INTERVAL_HOURS: i32 = 24;
pub const MIN_BACKUP_INTERVAL_HOURS: i32 = 1;
pub const MAX_BACKUP_INTERVAL_HOURS: i32 = 720;
pub const DEFAULT_BACKUP_MAX_KEEP: i32 = 10;
pub const MIN_BACKUP_MAX_KEEP: i32 = 1;
pub const MAX_BACKUP_MAX_KEEP: i32 = 100;
pub const DEFAULT_AUTO_UPDATE_CHECK: bool = true;

pub const DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: i32 = 0;
pub const MIN_UPDATE_CHECK_INTERVAL_HOURS: i32 = 0;
pub const MAX_UPDATE_CHECK_INTERVAL_HOURS: i32 = 720;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataLifecycle {
    /// Absolute path override for the `.sqyre` data directory (empty = `~/.sqyre`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sqyre_dir: String,

    /// Automatically zip the data directory on a schedule.
    #[serde(default)]
    pub backup_enabled: bool,

    /// Hours between automatic backups.
    #[serde(default = "default_backup_interval")]
    pub backup_interval_hours: i32,

    /// How many managed `sqyre-backup-*.zip` files to keep.
    #[serde(default = "default_backup_max_keep")]
    pub backup_max_keep: i32,

    /// Unix seconds of the last successful backup (0 = never).
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_backup_unix: i64,

    /// Check GitHub Releases for a newer Sqyre build on startup.
    #[serde(default = "default_auto_update_check")]
    pub auto_update_check: bool,

    /// Unix seconds of the last successful update check (0 = never).
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_update_check_unix: i64,

    /// Hide to the system tray instead of quitting on window close.
    #[serde(default)]
    pub close_to_tray: bool,

    /// Start Sqyre automatically on login (Linux XDG autostart).
    #[serde(default)]
    pub launch_on_login: bool,

    /// Launch with the main window hidden in the system tray.
    #[serde(default)]
    pub start_minimized: bool,

    /// Custom backups folder (empty = `{sqyre_dir}/backups`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub backup_dir: String,

    /// Hours between update checks (0 = every launch).
    #[serde(default)]
    pub update_check_interval_hours: i32,

    /// Offer prerelease builds in update checks.
    #[serde(default)]
    pub update_include_prerelease: bool,
}

fn default_backup_interval() -> i32 {
    DEFAULT_BACKUP_INTERVAL_HOURS
}
fn default_backup_max_keep() -> i32 {
    DEFAULT_BACKUP_MAX_KEEP
}
fn default_auto_update_check() -> bool {
    DEFAULT_AUTO_UPDATE_CHECK
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

impl Default for DataLifecycle {
    fn default() -> Self {
        Self {
            sqyre_dir: String::new(),
            backup_enabled: false,
            backup_interval_hours: DEFAULT_BACKUP_INTERVAL_HOURS,
            backup_max_keep: DEFAULT_BACKUP_MAX_KEEP,
            last_backup_unix: 0,
            auto_update_check: DEFAULT_AUTO_UPDATE_CHECK,
            last_update_check_unix: 0,
            close_to_tray: false,
            launch_on_login: false,
            start_minimized: false,
            backup_dir: String::new(),
            update_check_interval_hours: DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
            update_include_prerelease: false,
        }
    }
}

impl DataLifecycle {
    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        if self.backup_interval_hours < MIN_BACKUP_INTERVAL_HOURS {
            self.backup_interval_hours = DEFAULT_BACKUP_INTERVAL_HOURS;
        }
        self.backup_interval_hours = self
            .backup_interval_hours
            .clamp(MIN_BACKUP_INTERVAL_HOURS, MAX_BACKUP_INTERVAL_HOURS);
        if self.backup_max_keep < MIN_BACKUP_MAX_KEEP {
            self.backup_max_keep = DEFAULT_BACKUP_MAX_KEEP;
        }
        self.backup_max_keep = self
            .backup_max_keep
            .clamp(MIN_BACKUP_MAX_KEEP, MAX_BACKUP_MAX_KEEP);
        if self.last_backup_unix < 0 {
            self.last_backup_unix = 0;
        }
        if self.last_update_check_unix < 0 {
            self.last_update_check_unix = 0;
        }
        self.update_check_interval_hours = self.update_check_interval_hours.clamp(
            MIN_UPDATE_CHECK_INTERVAL_HOURS,
            MAX_UPDATE_CHECK_INTERVAL_HOURS,
        );
    }
}
