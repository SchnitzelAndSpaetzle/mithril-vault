// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_lock_timeout: u32,
    pub clipboard_clear_timeout: u32,
    pub show_password_by_default: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub theme: String,
    pub recent_databases: Vec<String>,
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

/// TODO: Fetches application settings (not yet implemented).
#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, AppError> {
    Err(AppError::NotImplemented("get_settings".into()))
}

/// TODO: Updates application settings (not yet implemented).
#[tauri::command]
pub async fn update_settings(settings: AppSettings) -> Result<(), AppError> {
    let _ = settings;
    Err(AppError::NotImplemented("update_settings".into()))
}

/// TODO: Adds a recent database entry (not yet implemented).
#[tauri::command]
pub async fn add_recent_database(path: String) -> Result<(), AppError> {
    let _ = path;
    Err(AppError::NotImplemented("add_recent_database".into()))
}

/// TODO: Removes a recent database entry (not yet implemented).
#[tauri::command]
pub async fn remove_recent_database(path: String) -> Result<(), AppError> {
    let _ = path;
    Err(AppError::NotImplemented("remove_recent_database".into()))
}

/// TODO: Clears recent database entries (not yet implemented).
#[tauri::command]
pub async fn clear_recent_databases() -> Result<(), AppError> {
    Err(AppError::NotImplemented("clear_recent_databases".into()))
}
