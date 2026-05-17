// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::audit::AuditService;
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
    /// Optional absolute path override for snapshot storage. When `None`, the
    /// per-Vault sibling subdir (`.kdbx-backups/`) is used. Validated at the
    /// settings boundary as "must be absolute"; no eager filesystem check so
    /// a temporarily-unmounted external volume does not produce a false
    /// rejection.
    #[serde(default)]
    pub directory: Option<String>,
    /// When true, also take a snapshot every time a Vault successfully
    /// unlocks. Opt-in (per #193) and silently skipped when the most recent
    /// existing snapshot's size+mtime already match the source.
    #[serde(default)]
    pub on_open: bool,
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
            directory: None,
            on_open: false,
        }
    }
}

impl BackupSettings {
    /// Normalizes `Some("")` to `None`. Callers that resolve the snapshot
    /// directory can then assume `Some(_)` always carries a non-empty path.
    pub(crate) fn normalize_directory(&mut self) {
        if matches!(self.directory.as_deref(), Some("")) {
            self.directory = None;
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

    #[test]
    fn absent_directory_deserializes_to_none() {
        // Settings files written before this slice have no `directory` field;
        // they must load as `None` so existing installs fall through to the
        // default sibling subdir.
        let from_empty: BackupSettings = serde_json::from_str("{}").expect("parse {}");
        assert!(from_empty.directory.is_none());

        let from_partial: BackupSettings =
            serde_json::from_str(r#"{"enabled": true, "maxVersions": 10}"#).expect("parse partial");
        assert!(from_partial.directory.is_none());
    }

    #[test]
    fn normalize_directory_treats_empty_string_as_none() {
        // The UI clears the override by emptying the field, which arrives as
        // `Some("")` over IPC. Normalize on the boundary so resolver code can
        // treat presence-of-Some as "user wants a custom path" without
        // re-checking emptiness on every snapshot.
        let mut s = BackupSettings {
            enabled: true,
            max_versions: 10,
            directory: Some(String::new()),
            on_open: false,
        };
        s.normalize_directory();
        assert!(s.directory.is_none());
    }

    #[test]
    fn normalize_directory_preserves_nonempty_paths() {
        let mut s = BackupSettings {
            enabled: true,
            max_versions: 10,
            directory: Some("/mnt/backups".into()),
            on_open: false,
        };
        s.normalize_directory();
        assert_eq!(s.directory.as_deref(), Some("/mnt/backups"));
    }

    #[test]
    fn explicit_directory_round_trips() {
        let json = r#"{"enabled": true, "maxVersions": 10, "directory": "/mnt/backups"}"#;
        let parsed: BackupSettings = serde_json::from_str(json).expect("parse explicit");
        assert_eq!(parsed.directory.as_deref(), Some("/mnt/backups"));

        let serialized = serde_json::to_string(&parsed).expect("serialize");
        assert!(
            serialized.contains(r#""directory":"/mnt/backups""#),
            "serialized json should carry the camelCase directory field: {serialized}"
        );
    }

    #[test]
    fn on_open_defaults_to_false_and_round_trips() {
        // Opt-in behavior per #193: existing installs must not start taking
        // open-side snapshots until the user flips the toggle.
        let from_empty: BackupSettings = serde_json::from_str("{}").expect("parse {}");
        assert!(!from_empty.on_open);

        let explicit: BackupSettings =
            serde_json::from_str(r#"{"enabled": true, "maxVersions": 10, "onOpen": true}"#)
                .expect("parse explicit");
        assert!(explicit.on_open);

        let serialized = serde_json::to_string(&explicit).expect("serialize");
        assert!(
            serialized.contains(r#""onOpen":true"#),
            "serialized json should carry camelCase onOpen field: {serialized}"
        );
    }
}

/// Audit log preferences. `enabled` controls whether `AuditService::record`
/// writes events; flipping it off does not delete the existing log file.
/// `retention_days` bounds how long records are kept once the retention
/// policy lands (#6); validated at the settings boundary to `1..=365`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AuditSettings {
    pub enabled: bool,
    #[serde(default = "default_audit_retention_days")]
    pub retention_days: u32,
}

pub(crate) const DEFAULT_AUDIT_RETENTION_DAYS: u32 = 90;

fn default_audit_retention_days() -> u32 {
    DEFAULT_AUDIT_RETENTION_DAYS
}

impl Default for AuditSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: DEFAULT_AUDIT_RETENTION_DAYS,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod audit_settings_tests {
    use super::AuditSettings;

    #[test]
    fn defaults_are_enabled_and_ninety_day_retention() {
        // PRD AC: `enabled` defaults to true (opt-out, not opt-in) and
        // retention defaults to 90 days. Hard-coding the assertion locks
        // the documented user contract — changing it would be a behavior
        // change that has to be considered, not a silent edit.
        let s = AuditSettings::default();
        assert!(s.enabled);
        assert_eq!(s.retention_days, 90);
    }

    #[test]
    fn absent_audit_section_deserializes_to_defaults() {
        // Users with a settings.json written before this slice have no
        // audit section. serde defaults must fill it in (AC: no migration
        // required) so audit recording starts immediately, with the
        // documented retention.
        let from_empty: AuditSettings = serde_json::from_str("{}").expect("parse {}");
        assert!(from_empty.enabled);
        assert_eq!(from_empty.retention_days, 90);
    }

    #[test]
    fn explicit_values_round_trip() {
        let json = r#"{"enabled": false, "retentionDays": 30}"#;
        let parsed: AuditSettings = serde_json::from_str(json).expect("parse explicit");
        assert!(!parsed.enabled);
        assert_eq!(parsed.retention_days, 30);

        let serialized = serde_json::to_string(&parsed).expect("serialize");
        assert!(
            serialized.contains(r#""retentionDays":30"#),
            "serialized json should carry the camelCase retentionDays field: {serialized}"
        );
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
    pub audit: AuditSettings,
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
    audit_service: State<'_, Arc<AuditService>>,
) -> Result<(), AppError> {
    settings_service.update_app_preferences(&new_preferences)?;
    kdbx_service.set_backup_settings(new_preferences.backups)?;
    audit_service.set_enabled(new_preferences.audit.enabled);
    Ok(())
}

#[tauri::command]
pub async fn reset_app_preferences(
    settings_service: State<'_, Arc<SettingsService>>,
    kdbx_service: State<'_, Arc<KdbxService>>,
    audit_service: State<'_, Arc<AuditService>>,
) -> Result<AppPreferences, AppError> {
    let prefs = settings_service.reset_app_preferences()?;
    kdbx_service.set_backup_settings(prefs.backups.clone())?;
    audit_service.set_enabled(prefs.audit.enabled);
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
