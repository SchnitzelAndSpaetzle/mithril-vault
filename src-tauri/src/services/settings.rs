// SPDX-License-Identifier: MIT

use crate::commands::settings::{AppPreferences, AppSettings, RecentDatabase};
use crate::dto::error::AppError;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE: &str = "settings.json";
const MAX_RECENT_DATABASES: usize = 10;

pub struct SettingsService {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
}

impl SettingsService {
    /// Creates a new `SettingsService`, loading settings from the app data directory.
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> Result<Self, AppError> {
        let data_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| AppError::Io(e.to_string()))?;
        std::fs::create_dir_all(&data_dir)?;

        let settings_path = data_dir.join(SETTINGS_FILE);
        let settings = Self::load_or_default(&settings_path)?;

        Ok(Self {
            settings: Mutex::new(settings),
            settings_path,
        })
    }

    fn load_or_default(path: &PathBuf) -> Result<AppSettings, AppError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let parsed: Result<AppSettings, _> = serde_json::from_str(&content);
            // Treat out-of-range values the same as a malformed file: back up
            // the bad copy and use defaults. Otherwise a hand-edited file
            // could push `max_versions = 0` straight to rotation and erase
            // every snapshot on the next save.
            match parsed {
                Ok(settings) if Self::validate_preferences(&settings.preferences).is_ok() => {
                    Ok(settings)
                }
                _ => {
                    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
                    let backup_name = format!("{SETTINGS_FILE}.bad-{timestamp}");
                    let backup_path = path.with_file_name(backup_name);
                    let _ = std::fs::rename(path, backup_path);
                    Ok(AppSettings::default())
                }
            }
        } else {
            Ok(AppSettings::default())
        }
    }

    fn save(&self, settings: &AppSettings) -> Result<(), AppError> {
        let content =
            serde_json::to_string_pretty(settings).map_err(|e| AppError::Io(e.to_string()))?;
        std::fs::write(&self.settings_path, content)?;
        Ok(())
    }

    /// Returns a snapshot of the persisted settings. Internal use only —
    /// the IPC surface goes through `get_app_preferences` / `get_recent_databases`.
    pub fn get_settings(&self) -> Result<AppSettings, AppError> {
        let settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        Ok(settings.clone())
    }

    pub fn get_app_preferences(&self) -> Result<AppPreferences, AppError> {
        let settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        let mut preferences = settings.preferences.clone();
        preferences.advanced.data_location = self.data_location_display();
        Ok(preferences)
    }

    pub fn update_app_preferences(&self, new_preferences: &AppPreferences) -> Result<(), AppError> {
        Self::validate_preferences(new_preferences)?;
        let mut settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        settings.preferences = new_preferences.clone();
        settings.preferences.backups.normalize_directory();
        // data_location is derived at read time; don't trust the value the
        // frontend echoed back. Persist as empty so the on-disk file doesn't
        // carry a stale path if the user moves their app data later.
        settings.preferences.advanced.data_location.clear();
        self.save(&settings)
    }

    pub fn reset_app_preferences(&self) -> Result<AppPreferences, AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        let recent_databases = settings.recent_databases.clone();
        *settings = AppSettings {
            preferences: AppPreferences::default(),
            recent_databases,
        };
        self.save(&settings)?;
        let mut preferences = settings.preferences.clone();
        preferences.advanced.data_location = self.data_location_display();
        Ok(preferences)
    }

    pub fn get_recent_databases(&self) -> Result<Vec<RecentDatabase>, AppError> {
        let settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        Ok(settings.recent_databases.clone())
    }

    pub fn add_recent_database(
        &self,
        path: &str,
        keyfile_path: Option<&str>,
    ) -> Result<(), AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::Lock)?;

        // Remove existing entry with same path
        settings.recent_databases.retain(|r| r.path != path);

        // Add to front
        settings.recent_databases.insert(
            0,
            RecentDatabase {
                path: path.to_string(),
                keyfile_path: keyfile_path.map(String::from),
                last_opened: chrono::Utc::now().to_rfc3339(),
            },
        );

        // Limit size
        settings.recent_databases.truncate(MAX_RECENT_DATABASES);

        self.save(&settings)
    }

    pub fn get_keyfile_for_database(&self, db_path: &str) -> Result<Option<String>, AppError> {
        let settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        Ok(settings
            .recent_databases
            .iter()
            .find(|r| r.path == db_path)
            .and_then(|r| r.keyfile_path.clone()))
    }

    pub fn remove_recent_database(&self, path: &str) -> Result<(), AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        settings.recent_databases.retain(|r| r.path != path);
        self.save(&settings)
    }

    pub fn clear_recent_databases(&self) -> Result<(), AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        settings.recent_databases.clear();
        self.save(&settings)
    }

    /// Rejects out-of-range values on the App Preferences boundary so a
    /// malformed IPC payload (or a future settings.json with corrupt values)
    /// cannot push the backup module into invariant violations.
    fn validate_preferences(prefs: &AppPreferences) -> Result<(), AppError> {
        const MAX_VERSIONS_RANGE: std::ops::RangeInclusive<u32> = 1..=500;
        let v = prefs.backups.max_versions;
        if !MAX_VERSIONS_RANGE.contains(&v) {
            return Err(AppError::InvalidInput(format!(
                "backups.maxVersions must be in 1..=500, got {v}"
            )));
        }
        if let Some(dir) = prefs.backups.directory.as_deref() {
            if !dir.is_empty() && !std::path::Path::new(dir).is_absolute() {
                return Err(AppError::InvalidInput(format!(
                    "backups.directory must be an absolute path, got {dir:?}"
                )));
            }
        }
        Ok(())
    }

    fn data_location_display(&self) -> String {
        self.settings_path
            .parent()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod validate_preferences_tests {
    use super::SettingsService;
    use crate::commands::settings::{AppPreferences, BackupSettings};
    use crate::dto::error::AppError;

    fn prefs_with_directory(dir: Option<&str>) -> AppPreferences {
        AppPreferences {
            backups: BackupSettings {
                enabled: true,
                max_versions: 10,
                directory: dir.map(String::from),
            },
            ..AppPreferences::default()
        }
    }

    #[test]
    fn relative_directory_path_is_rejected() {
        // A relative override would be resolved against whatever CWD the
        // process happens to have at save time — wildly unpredictable for a
        // safety-net feature. Reject it at the boundary so the backup
        // module's resolver can trust the path it sees.
        let prefs = prefs_with_directory(Some("relative/path"));
        match SettingsService::validate_preferences(&prefs) {
            Err(AppError::InvalidInput(msg)) => {
                assert!(
                    msg.contains("absolute"),
                    "error should mention 'absolute', got: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn absolute_directory_path_is_accepted() {
        // Use a platform-appropriate absolute path so the test runs on every
        // supported target. The validator does not touch the filesystem.
        let abs = if cfg!(windows) {
            "C:/backups"
        } else {
            "/mnt/backups"
        };
        let prefs = prefs_with_directory(Some(abs));
        SettingsService::validate_preferences(&prefs).expect("absolute path should validate");
    }

    #[test]
    fn no_directory_override_is_accepted() {
        let prefs = prefs_with_directory(None);
        SettingsService::validate_preferences(&prefs).expect("None should validate");
    }
}
