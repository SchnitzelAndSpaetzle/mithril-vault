// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::settings::SettingsService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDatabase {
    pub path: String,
    pub keyfile_path: Option<String>,
    pub last_opened: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_lock_timeout: u32,
    pub clipboard_clear_timeout: u32,
    pub show_password_by_default: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub theme: String,
    pub recent_databases: Vec<RecentDatabase>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_lock_timeout: 300,
            clipboard_clear_timeout: 30,
            show_password_by_default: false,
            minimize_to_tray: true,
            start_minimized: false,
            theme: "system".into(),
            recent_databases: Vec::new(),
        }
    }
}

/// Fetches application settings.
#[tauri::command]
pub async fn get_settings(
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<AppSettings, AppError> {
    settings_service.get_settings()
}

/// Updates application settings.
#[tauri::command]
pub async fn update_settings(
    new_settings: AppSettings,
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<(), AppError> {
    settings_service.update_settings(new_settings)
}

/// Adds a database to the recent list with optional keyfile association.
#[tauri::command]
pub async fn add_recent_database(
    path: String,
    keyfile_path: Option<String>,
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<(), AppError> {
    settings_service.add_recent_database(&path, keyfile_path.as_deref())
}

/// Gets the associated keyfile path for a database if one was saved.
#[tauri::command]
pub async fn get_keyfile_for_database(
    path: String,
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<Option<String>, AppError> {
    settings_service.get_keyfile_for_database(&path)
}

/// Removes a database from the recent list.
#[tauri::command]
pub async fn remove_recent_database(
    path: String,
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<(), AppError> {
    settings_service.remove_recent_database(&path)
}

/// Clears all recent database entries.
#[tauri::command]
pub async fn clear_recent_databases(
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<(), AppError> {
    settings_service.clear_recent_databases()
}
