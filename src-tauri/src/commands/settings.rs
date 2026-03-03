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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StartupBehavior {
    #[default]
    ShowUnlockScreen,
    OpenLastDatabase,
    OpenDefaultDatabase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryListColumns {
    pub username: bool,
    pub url: bool,
    pub modified_at: bool,
    pub tags: bool,
}

impl Default for EntryListColumns {
    fn default() -> Self {
        Self {
            username: true,
            url: true,
            modified_at: true,
            tags: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub language: String,
    pub startup_behavior: StartupBehavior,
    pub default_database_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySettings {
    pub auto_lock_timeout: u32,
    pub clipboard_clear_timeout: u32,
    pub show_password_by_default: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub theme: String,
    pub font_size: u8,
    pub entry_list_columns: EntryListColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserIntegrationSettings {
    pub enabled: bool,
    pub allowed_sites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    pub debug_mode: bool,
    pub data_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub general: GeneralSettings,
    pub security: SecuritySettings,
    pub appearance: AppearanceSettings,
    pub browser_integration: BrowserIntegrationSettings,
    pub advanced: AdvancedSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppSettings {
    pub language: String,
    pub startup_behavior: StartupBehavior,
    pub default_database_path: Option<String>,
    pub auto_lock_timeout: u32,
    pub clipboard_clear_timeout: u32,
    pub show_password_by_default: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub theme: String,
    pub font_size: u8,
    pub entry_list_show_username: bool,
    pub entry_list_show_url: bool,
    pub entry_list_show_modified_at: bool,
    pub entry_list_show_tags: bool,
    pub browser_integration_enabled: bool,
    pub browser_allowed_sites: Vec<String>,
    pub debug_mode: bool,
    pub recent_databases: Vec<RecentDatabase>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            startup_behavior: StartupBehavior::ShowUnlockScreen,
            default_database_path: None,
            auto_lock_timeout: 300,
            clipboard_clear_timeout: 30,
            show_password_by_default: false,
            minimize_to_tray: true,
            start_minimized: false,
            theme: "system".into(),
            font_size: 14,
            entry_list_show_username: true,
            entry_list_show_url: true,
            entry_list_show_modified_at: true,
            entry_list_show_tags: true,
            browser_integration_enabled: false,
            browser_allowed_sites: Vec::new(),
            debug_mode: false,
            recent_databases: Vec::new(),
        }
    }
}

impl AppPreferences {
    pub fn from_settings(settings: &AppSettings, data_location: String) -> Self {
        Self {
            general: GeneralSettings {
                language: settings.language.clone(),
                startup_behavior: settings.startup_behavior.clone(),
                default_database_path: settings.default_database_path.clone(),
            },
            security: SecuritySettings {
                auto_lock_timeout: settings.auto_lock_timeout,
                clipboard_clear_timeout: settings.clipboard_clear_timeout,
                show_password_by_default: settings.show_password_by_default,
                minimize_to_tray: settings.minimize_to_tray,
                start_minimized: settings.start_minimized,
            },
            appearance: AppearanceSettings {
                theme: settings.theme.clone(),
                font_size: settings.font_size,
                entry_list_columns: EntryListColumns {
                    username: settings.entry_list_show_username,
                    url: settings.entry_list_show_url,
                    modified_at: settings.entry_list_show_modified_at,
                    tags: settings.entry_list_show_tags,
                },
            },
            browser_integration: BrowserIntegrationSettings {
                enabled: settings.browser_integration_enabled,
                allowed_sites: settings.browser_allowed_sites.clone(),
            },
            advanced: AdvancedSettings {
                debug_mode: settings.debug_mode,
                data_location,
            },
        }
    }

    pub fn apply_to_settings(&self, settings: &mut AppSettings) {
        settings.language = self.general.language.clone();
        settings.startup_behavior = self.general.startup_behavior.clone();
        settings.default_database_path = self.general.default_database_path.clone();
        settings.auto_lock_timeout = self.security.auto_lock_timeout;
        settings.clipboard_clear_timeout = self.security.clipboard_clear_timeout;
        settings.show_password_by_default = self.security.show_password_by_default;
        settings.minimize_to_tray = self.security.minimize_to_tray;
        settings.start_minimized = self.security.start_minimized;
        settings.theme = self.appearance.theme.clone();
        settings.font_size = self.appearance.font_size;
        settings.entry_list_show_username = self.appearance.entry_list_columns.username;
        settings.entry_list_show_url = self.appearance.entry_list_columns.url;
        settings.entry_list_show_modified_at = self.appearance.entry_list_columns.modified_at;
        settings.entry_list_show_tags = self.appearance.entry_list_columns.tags;
        settings.browser_integration_enabled = self.browser_integration.enabled;
        settings.browser_allowed_sites = self.browser_integration.allowed_sites.clone();
        settings.debug_mode = self.advanced.debug_mode;
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

#[tauri::command]
pub async fn get_app_preferences(
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<AppPreferences, AppError> {
    settings_service.get_app_preferences()
}

#[tauri::command]
pub async fn update_app_preferences(
    new_preferences: AppPreferences,
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<(), AppError> {
    settings_service.update_app_preferences(new_preferences)
}

#[tauri::command]
pub async fn reset_app_preferences(
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<AppPreferences, AppError> {
    settings_service.reset_app_preferences()
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
