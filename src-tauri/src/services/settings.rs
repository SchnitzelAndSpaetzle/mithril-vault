// SPDX-License-Identifier: MIT

use crate::commands::settings::{AppSettings, RecentDatabase};
use crate::dto::error::AppError;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const SETTINGS_FILE: &str = "settings.json";
const MAX_RECENT_DATABASES: usize = 10;

pub struct SettingsService {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
}

impl SettingsService {
    /// Creates a new `SettingsService`, loading settings from the app data directory.
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
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
            serde_json::from_str(&content).map_err(|e| AppError::Io(e.to_string()))
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

    pub fn get_settings(&self) -> Result<AppSettings, AppError> {
        let settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        Ok(settings.clone())
    }

    pub fn update_settings(&self, new_settings: AppSettings) -> Result<(), AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::Lock)?;
        *settings = new_settings;
        self.save(&settings)
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
}
