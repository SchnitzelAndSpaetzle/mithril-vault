// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;
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
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
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
#[serde(default)]
pub struct GeneralSettings {
    pub language: String,
    pub startup_behavior: StartupBehavior,
    pub default_database_path: Option<String>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            startup_behavior: StartupBehavior::ShowUnlockScreen,
            default_database_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecuritySettings {
    pub auto_lock_timeout: u32,
    pub clipboard_clear_timeout: u32,
    pub clear_clipboard_on_lock: bool,
    pub show_clipboard_countdown: bool,
    pub show_password_by_default: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub prevent_screen_capture: bool,
    pub auto_download_favicons: bool,
    pub allow_third_party_favicon_fallbacks: bool,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            auto_lock_timeout: 300,
            clipboard_clear_timeout: 30,
            clear_clipboard_on_lock: true,
            show_clipboard_countdown: false,
            show_password_by_default: false,
            minimize_to_tray: true,
            start_minimized: false,
            prevent_screen_capture: true,
            auto_download_favicons: false,
            allow_third_party_favicon_fallbacks: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub color_preset: String,
    pub font_size: u8,
    pub entry_list_columns: EntryListColumns,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            color_preset: "default".into(),
            font_size: 14,
            entry_list_columns: EntryListColumns::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct BrowserIntegrationSettings {
    pub enabled: bool,
    pub allowed_sites: Vec<String>,
}

/// Advanced settings. `data_location` is derived at read time (it's the
/// filesystem path of the settings file itself) — any value persisted in
/// `settings.json` is ignored and overwritten on the next read.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AdvancedSettings {
    pub debug_mode: bool,
    pub data_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct BackupSettings {
    pub enabled: bool,
    #[serde(default = "default_max_versions")]
    pub max_versions: u32,
}

pub(crate) const DEFAULT_MAX_VERSIONS: u32 = 10;

fn default_max_versions() -> u32 {
    DEFAULT_MAX_VERSIONS
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_versions: DEFAULT_MAX_VERSIONS,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod backup_settings_tests {
    use super::BackupSettings;

    #[test]
    fn default_max_versions_is_ten() {
        let s = BackupSettings::default();
        assert_eq!(s.max_versions, 10);
    }

    #[test]
    fn absent_max_versions_deserializes_to_ten_not_zero() {
        // Existing settings.json files written before this slice have no
        // maxVersions field. Without a field-level serde default they would
        // deserialize as 0, which would break rotation. Cover both `{}`
        // (whole struct missing) and `{"enabled": ..}` (field missing).
        let from_empty: BackupSettings = serde_json::from_str("{}").expect("parse {}");
        assert_eq!(from_empty.max_versions, 10);

        let from_partial: BackupSettings =
            serde_json::from_str(r#"{"enabled": true}"#).expect("parse partial");
        assert_eq!(from_partial.max_versions, 10);
    }

    #[test]
    fn explicit_max_versions_round_trips() {
        let parsed: BackupSettings =
            serde_json::from_str(r#"{"enabled": true, "maxVersions": 25}"#)
                .expect("parse explicit");
        assert_eq!(parsed.max_versions, 25);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppPreferences {
    pub general: GeneralSettings,
    pub security: SecuritySettings,
    pub appearance: AppearanceSettings,
    pub browser_integration: BrowserIntegrationSettings,
    pub advanced: AdvancedSettings,
    pub backups: BackupSettings,
}

/// Persisted shape written to `settings.json`. Combines the editable
/// `AppPreferences` with the per-machine `recent_databases` list. The
/// `advanced.data_location` field is derived; it's not trusted on read.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppSettings {
    pub preferences: AppPreferences,
    pub recent_databases: Vec<RecentDatabase>,
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
    kdbx_service: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    settings_service.update_app_preferences(&new_preferences)?;
    kdbx_service.set_backup_settings(new_preferences.backups)?;
    Ok(())
}

#[tauri::command]
pub async fn reset_app_preferences(
    settings_service: State<'_, Arc<SettingsService>>,
    kdbx_service: State<'_, Arc<KdbxService>>,
) -> Result<AppPreferences, AppError> {
    let prefs = settings_service.reset_app_preferences()?;
    kdbx_service.set_backup_settings(prefs.backups.clone())?;
    Ok(prefs)
}

#[tauri::command]
pub async fn get_recent_databases(
    settings_service: State<'_, Arc<SettingsService>>,
) -> Result<Vec<RecentDatabase>, AppError> {
    settings_service.get_recent_databases()
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
