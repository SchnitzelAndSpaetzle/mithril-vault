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
            if let Ok(settings) = serde_json::from_str(&content) {
                Ok(settings)
            } else {
                let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
                let backup_name = format!("{SETTINGS_FILE}.bad-{timestamp}");
                let backup_path = path.with_file_name(backup_name);
                let _ = std::fs::rename(path, backup_path);
                Ok(AppSettings::default())
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
        Ok(())
    }

    fn data_location_display(&self) -> String {
        self.settings_path
            .parent()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
