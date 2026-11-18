//! Data location, backup, restore, and confirm dialogs.

use super::common::*;
use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

impl SettingsUi {
    pub(super) fn draw_data(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, DATA_LOCATION) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (db, macros, catalog);
            ui.label(
                "Browser editor: macros live in memory. Use Import / Export on the toolbar for db.yaml. Full backups are not available.",
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let current = if self.settings.data.sqyre_dir.trim().is_empty() {
                sqyre_dir()
            } else {
                PathBuf::from(self.settings.data.sqyre_dir.trim())
            };
            ui.label(current.display().to_string());

            let flatpak = sqyre_update::is_flatpak_install();
            if flatpak {
                ui.label(
                    egui::RichText::new(
                        "Flatpak keeps a private data copy by default. To share macros \
                         with AppImage/native, grant access and point at host ~/.sqyre.",
                    )
                    .weak()
                    .small(),
                );
            }

            ui.horizontal_wrapped(|ui| {
                if settings_folder_btn(ui, "Open .sqyre folder").clicked() {
                    match open_sqyre_dir() {
                        Ok(()) => self.set_ok("Opened data folder."),
                        Err(e) => self.set_err(format!(
                            "Open folder failed: {}",
                            crate::status_banner::user_error_detail(&e)
                        )),
                    }
                }
                if crate::file_dialogs::supported(PickPurpose::SqyreLocation)
                    && settings_choose_btn(ui, "Choose location…").clicked()
                {
                    self.choose_sqyre_location();
                }
                if flatpak {
                    if settings_folder_btn(ui, ht::SET_HOST_FOLDER).clicked() {
                        self.choose_host_sqyre_location();
                    }
                    if !self.settings.data.sqyre_dir.trim().is_empty()
                        && settings_reset_btn(ui, ht::SET_FLATPAK_RESET).clicked()
                    {
                        self.reset_sqyre_to_sandbox(db, macros, catalog);
                    }
                }
            });
        }
    }

    pub(super) fn draw_backup(
        &mut self,
        ui: &mut egui::Ui,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        q: &str,
        section_hit: bool,
    ) {
        if !setting_visible(q, section_hit, DATA_BACKUP) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (ui, db, macros, catalog);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (db, macros, catalog);

            if dirty_checkbox(
                ui,
                &mut self.settings.data.backup_enabled,
                "Automatic backups",
                ht::SET_AUTO_BACKUP,
            ) {
                self.mark_dirty();
            }

            ui.horizontal(|ui| {
                ui.label("Interval (hours):");
                if dirty_i32(
                    ui,
                    &mut self.settings.data.backup_interval_hours,
                    ht::SET_BACKUP_HOURS,
                    |d| {
                        d.range(MIN_BACKUP_INTERVAL_HOURS..=MAX_BACKUP_INTERVAL_HOURS)
                            .speed(1)
                    },
                ) {
                    self.mark_dirty();
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.data.backup_interval_hours = DEFAULT_BACKUP_INTERVAL_HOURS;
                    self.mark_dirty();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Keep at most:");
                if dirty_i32(
                    ui,
                    &mut self.settings.data.backup_max_keep,
                    ht::SET_BACKUP_KEEP,
                    |d| d.range(MIN_BACKUP_MAX_KEEP..=MAX_BACKUP_MAX_KEEP).speed(1),
                ) {
                    self.mark_dirty();
                }
                if settings_reset_btn(ui, "Reset to default").clicked() {
                    self.settings.data.backup_max_keep = DEFAULT_BACKUP_MAX_KEEP;
                    self.mark_dirty();
                }
            });

            // The path is its own wrapping line. A horizontal row with the
            // buttons lets a long Windows path push Choose past the pane.
            ui.label(
                egui::RichText::new(format!("Backups folder: {}", backups_dir().display()))
                    .weak()
                    .small(),
            );
            ui.horizontal_wrapped(|ui| {
                if crate::file_dialogs::supported(PickPurpose::BackupDir)
                    && settings_choose_btn(ui, ht::SET_BACKUP_DIR).clicked()
                {
                    crate::file_dialogs::request(
                        PickPurpose::BackupDir,
                        "Choose backups folder",
                        Some(&backups_dir()),
                    );
                }
                if !self.settings.data.backup_dir.trim().is_empty()
                    && settings_reset_btn(ui, ht::SET_BACKUP_DIR).clicked()
                {
                    self.settings.data.backup_dir.clear();
                    self.settings.apply_backup_dir_override();
                    self.mark_dirty();
                }
            });
            ui.label(
                egui::RichText::new(format_last_backup(self.settings.data.last_backup_unix))
                    .weak()
                    .small(),
            );
            if let Ok(list) = list_backups() {
                if let Some(latest) = list.first() {
                    if let Some(name) = latest.file_name().and_then(|n| n.to_str()) {
                        ui.label(
                            egui::RichText::new(format!("Latest archive: {name}"))
                                .weak()
                                .small(),
                        );
                    }
                }
            }

            ui.horizontal(|ui| {
                if crate::widgets::icon_button(ui, egui_phosphor::regular::ARCHIVE, "Back up now")
                    .clicked()
                {
                    self.run_manual_backup();
                }
                if crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::UPLOAD_SIMPLE,
                    "Restore from backup…",
                )
                .clicked()
                {
                    self.request_restore_backup();
                }
            });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn run_manual_backup(&mut self) {
        match create_backup() {
            Ok(path) => {
                let keep = self.settings.data.backup_max_keep.max(1) as usize;
                if let Err(e) = prune_backups(keep) {
                    self.set_err(format!(
                        "Backup created but prune failed: {}",
                        crate::status_banner::user_error_detail(&e)
                    ));
                }
                self.note_backup_success(&path);
            }
            Err(e) => self.set_err(format!(
                "Backup failed: {}",
                crate::status_banner::user_error_detail(&e)
            )),
        }
    }

    /// Record a failed automatic backup in the Settings status line (the log
    /// keeps the same warning so diagnostics don't regress).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn note_backup_failure(&mut self, err: &str) {
        crate::log::warn(format_args!("automatic backup failed: {err}"));
        self.set_err(format!(
            "Automatic backup failed: {}",
            crate::status_banner::user_error_detail(err)
        ));
    }

    /// Record a successful backup timestamp and persist settings.
    pub fn note_backup_success(&mut self, path: &std::path::Path) {
        self.settings.data.last_backup_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.persist();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("backup");
        self.set_ok(format!("Backup saved: {name}"));
    }

    /// Pick a backup zip; [`Self::apply_pick`] opens Settings on the restore confirm.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_restore_backup(&mut self) {
        let start = backups_dir();
        let start = if start.exists() { start } else { sqyre_dir() };
        crate::file_dialogs::request(
            PickPurpose::RestoreBackup,
            "Restore from backup",
            Some(&start),
        );
    }

    /// Finish a Settings pick and open Settings on its confirm dialog.
    pub(crate) fn apply_pick(&mut self, purpose: PickPurpose, path: PathBuf) {
        match purpose {
            PickPurpose::RestoreBackup => {
                self.confirm = Some(PendingConfirm::RestoreBackup { path });
            }
            PickPurpose::SqyreLocation | PickPurpose::HostSqyreLocation => {
                self.confirm_sqyre_location(resolve_sqyre_dir_from_pick(path));
            }
            PickPurpose::FinishSound => {
                self.settings.sound.finish_sound_file = path.to_string_lossy().into_owned();
                self.mark_dirty();
            }
            PickPurpose::AddSound => {
                self.settings.sound.add_sound_file = path.to_string_lossy().into_owned();
                self.mark_dirty();
            }
            PickPurpose::DeleteSound => {
                self.settings.sound.delete_sound_file = path.to_string_lossy().into_owned();
                self.mark_dirty();
            }
            PickPurpose::BackupDir => {
                self.settings.data.backup_dir = path.to_string_lossy().into_owned();
                self.settings.apply_backup_dir_override();
                self.mark_dirty();
            }
            PickPurpose::IconVariant | PickPurpose::MaskImage => return,
        }
        if self.confirm.is_some() {
            self.open = true;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn choose_sqyre_location(&mut self) {
        let start = sqyre_dir()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(sqyre_dir);
        crate::file_dialogs::request(
            PickPurpose::SqyreLocation,
            "Choose .sqyre location",
            Some(&start),
        );
    }

    /// Flatpak: portal-pick host `~/.sqyre` (or its parent) so the sandbox can use it.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn choose_host_sqyre_location(&mut self) {
        let start = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(sqyre_dir);
        crate::file_dialogs::request(
            PickPurpose::HostSqyreLocation,
            "Select host ~/.sqyre (or your Home folder)",
            Some(&start),
        );
    }

    pub(super) fn confirm_sqyre_location(&mut self, new_dir: PathBuf) {
        let old_dir = sqyre_dir();
        if new_dir == old_dir {
            return;
        }
        self.confirm = Some(PendingConfirm::MoveData { old_dir, new_dir });
    }

    /// Clear a custom data path and reload from the default Flatpak/native location.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn reset_sqyre_to_sandbox(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if self.settings.data.sqyre_dir.trim().is_empty() {
            return;
        }
        self.settings.data.sqyre_dir.clear();
        set_sqyre_dir_override(None);
        self.persist();
        let path = sqyre_dir();
        self.reload_data_from_disk(db, macros, catalog, &path, "Using Flatpak sandbox data");
    }

    pub(super) fn draw_confirm(
        &mut self,
        ctx: &egui::Context,
        confirm: PendingConfirm,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
        data: &mut SettingsData<'_>,
    ) {
        match &confirm {
            PendingConfirm::MoveData { old_dir, new_dir } => {
                let open = crate::widgets::confirm_window(
                    ctx,
                    "Move existing data?",
                    pending_scale,
                    |ui| {
                        ui.label(format!(
                        "Move your current data from\n{}\nto\n{}?\n\nChoose No to start fresh at the new location (existing data is left in place).",
                        old_dir.display(),
                        new_dir.display()
                    ));
                        // Enter → Yes (index 1).
                        match crate::widgets::confirm_choice_row(
                            ui,
                            &[
                                ("No", crate::widgets::ConfirmKind::Primary),
                                ("Yes", crate::widgets::ConfirmKind::Primary),
                            ],
                            Some(1),
                        ) {
                            crate::widgets::ConfirmChoice::Cancel => {
                                self.confirm = None;
                            }
                            crate::widgets::ConfirmChoice::Choice(0) => {
                                let old = old_dir.clone();
                                let new = new_dir.clone();
                                self.confirm = None;
                                self.apply_sqyre_location(
                                    old,
                                    new,
                                    false,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(1) => {
                                let old = old_dir.clone();
                                let new = new_dir.clone();
                                self.confirm = None;
                                self.apply_sqyre_location(
                                    old,
                                    new,
                                    true,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(_)
                            | crate::widgets::ConfirmChoice::None => {}
                        }
                    },
                );
                if !open {
                    self.confirm = None;
                }
            }
            PendingConfirm::RestoreBackup { path } => {
                let open = crate::widgets::confirm_window(
                    ctx,
                    "Import backup?",
                    pending_scale,
                    |ui| {
                        ui.label(format!(
                        "Archive:\n{}\n\nOverwrite replaces all macros, settings, images, and variables with the archive.\n\nMerge keeps live-only items, prefers the archive on name conflicts, replaces settings, and merges other assets. Automatic backups in the backups folder are not removed.",
                        path.display()
                    ));
                        // Enter → Overwrite (index 0); both are intentional verbs.
                        match crate::widgets::confirm_choice_row(
                            ui,
                            &[
                                ("Overwrite", crate::widgets::ConfirmKind::Destructive),
                                ("Merge", crate::widgets::ConfirmKind::Primary),
                            ],
                            Some(0),
                        ) {
                            crate::widgets::ConfirmChoice::Cancel => {
                                self.confirm = None;
                            }
                            crate::widgets::ConfirmChoice::Choice(0) => {
                                let path = path.clone();
                                self.confirm = None;
                                self.apply_restore_backup(
                                    path,
                                    ImportMode::Overwrite,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(1) => {
                                let path = path.clone();
                                self.confirm = None;
                                self.apply_restore_backup(
                                    path,
                                    ImportMode::Merge,
                                    data.db,
                                    data.macros,
                                    data.catalog,
                                );
                            }
                            crate::widgets::ConfirmChoice::Choice(_)
                            | crate::widgets::ConfirmChoice::None => {}
                        }
                    },
                );
                if !open {
                    self.confirm = None;
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn apply_restore_backup(
        &mut self,
        path: PathBuf,
        mode: ImportMode,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if let Err(e) = import_backup(&path, mode) {
            self.set_err(format!(
                "Import failed: {}",
                crate::status_banner::user_error_detail(&e)
            ));
            return;
        }

        // Prefer restored settings.yaml; keep current if load fails.
        match UserSettings::load_default() {
            Ok(loaded) => {
                loaded.apply_sqyre_dir_override();
                Self::apply_action_colors(&loaded);
                self.settings = loaded;
            }
            Err(e) => {
                crate::log::warn(format_args!(
                    "import succeeded but settings reload failed: {e}"
                ));
            }
        }

        match Database::load_default() {
            Ok(mut loaded) => {
                let mut cat = Arc::unwrap_or_clone(loaded.program_catalog().unwrap_or_default());
                let _ = crate::catalog::prepare_catalog(&mut cat, &mut loaded);
                let mut list: Vec<_> = loaded.macros.values().cloned().collect();
                list.sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
                *db = loaded;
                *macros = list;
                *catalog = cat;
                self.reload_requested = true;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("backup");
                let mode_label = match mode {
                    ImportMode::Overwrite => "Overwrote from",
                    ImportMode::Merge => "Merged from",
                };
                self.set_ok(format!("{mode_label} {name}."));
            }
            Err(e) => {
                *db = Database::default();
                macros.clear();
                *catalog = ProgramCatalog::default();
                crate::catalog::apply_main_monitor_resolution(catalog);
                let _ = crate::catalog::ensure_general_program_seeded(catalog);
                self.reload_requested = true;
                self.set_err(format!(
                    "Imported archive but failed to load db.yaml: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn apply_restore_backup(
        &mut self,
        _path: PathBuf,
        _mode: ImportMode,
        _db: &mut Database,
        _macros: &mut Vec<Macro>,
        _catalog: &mut ProgramCatalog,
    ) {
    }

    pub(super) fn apply_sqyre_location(
        &mut self,
        old_dir: PathBuf,
        new_dir: PathBuf,
        move_data: bool,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
    ) {
        if move_data && old_dir.exists() {
            if let Err(e) = move_dir(&old_dir, &new_dir) {
                self.set_err(format!(
                    "Move failed: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
                return;
            }
        }

        self.settings.data.sqyre_dir = new_dir.display().to_string();
        set_sqyre_dir_override(Some(new_dir.clone()));
        self.persist();
        self.reload_data_from_disk(
            db,
            macros,
            catalog,
            &new_dir,
            &format!("Data location changed to {}.", new_dir.display()),
        );
    }

    pub(super) fn reload_data_from_disk(
        &mut self,
        db: &mut Database,
        macros: &mut Vec<Macro>,
        catalog: &mut ProgramCatalog,
        path: &std::path::Path,
        ok_msg: &str,
    ) {
        match Database::load_default() {
            Ok(mut loaded) => {
                let mut cat = Arc::unwrap_or_clone(loaded.program_catalog().unwrap_or_default());
                let _ = crate::catalog::prepare_catalog(&mut cat, &mut loaded);
                let mut list: Vec<_> = loaded.macros.values().cloned().collect();
                list.sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
                *db = loaded;
                *macros = list;
                *catalog = cat;
                self.reload_requested = true;
                self.set_ok(ok_msg.to_string());
            }
            Err(e) => {
                *db = Database::default();
                macros.clear();
                *catalog = ProgramCatalog::default();
                crate::catalog::apply_main_monitor_resolution(catalog);
                let _ = crate::catalog::ensure_general_program_seeded(catalog);
                self.reload_requested = true;
                self.set_err(format!(
                    "Switched to {} but failed to load db.yaml: {}",
                    path.display(),
                    crate::status_banner::user_error_detail(&e)
                ));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn format_last_backup(unix: i64) -> String {
    if unix <= 0 {
        return "Last backup: never".into();
    }
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return format!("Last backup: unix {unix}");
    };
    let age_secs = now.as_secs().saturating_sub(unix as u64);
    let label = if age_secs < 60 {
        format!("{age_secs}s ago")
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else if age_secs < 86_400 {
        format!("{}h ago", age_secs / 3600)
    } else {
        format!("{}d ago", age_secs / 86_400)
    };
    format!("Last backup: {label}")
}
