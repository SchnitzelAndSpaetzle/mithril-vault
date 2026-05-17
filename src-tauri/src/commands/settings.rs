// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::audit::AuditService;
use crate::services::kdbx::KdbxService;
use crate::services::settings::{diff_security_changes, SettingsService};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

/// Fans changed allowlisted leaves out across every currently-open Vault,
/// emitting one `preferences.security_changed` audit record per
/// (vault, leaf) pair. Extracted from `update_app_preferences` so the
/// fan-out logic can be unit-tested without a Tauri runtime.
///
/// The audit log is per-Vault (one file per canonicalized vault path),
/// while preferences are global — a flip therefore has to land in each
/// open Vault's log to surface in the Audit Log panel. When no Vault is
/// open the call is a no-op: the preference flip still persists, but
/// there is no per-Vault log to write to.
pub(crate) fn fan_out_security_changes(
    audit_service: &AuditService,
    open_vault_paths: &[String],
    changed_leaves: &[&'static str],
) {
    for path in open_vault_paths {
        for leaf in changed_leaves {
            audit_service.record_preferences_security_changed(Path::new(path), leaf);
        }
    }
}

/// Diffs `old` vs `new` preferences, applies the new audit gate, and
/// force-writes one `preferences.security_changed` event per allowlisted
/// leaf that changed against every open Vault.
///
/// Ordering is deliberate. The master gate is set FIRST so any
/// concurrent producer sharing this `AuditService` (entry reveals,
/// failed unlocks, vault locks, …) immediately reflects the user's
/// persisted intent — a previous version of this helper transiently
/// flipped the gate on around the fan-out to capture the transition
/// itself, but that reopened logging process-wide and let unrelated
/// concurrent events slip into the disabled-by-the-user log.
///
/// The transition events themselves go through
/// [`AuditService::record_preferences_security_changed`], which
/// force-writes around the gate. That captures both the true→false
/// disable and false→true enable cases without ever widening the gate.
///
/// When logging is off at BOTH ends of the transition, the user has
/// consistently opted out — nothing is recorded, respecting that intent.
pub(crate) fn apply_preference_security_audit(
    audit_service: &AuditService,
    open_vault_paths: &[String],
    old: &AppPreferences,
    new: &AppPreferences,
) {
    audit_service.set_enabled(new.audit.enabled);
    let changed_leaves = diff_security_changes(old, new);
    let should_record = !changed_leaves.is_empty() && (old.audit.enabled || new.audit.enabled);
    if should_record {
        fan_out_security_changes(audit_service, open_vault_paths, &changed_leaves);
    }
}

fn open_vault_paths(kdbx_service: &KdbxService) -> Vec<String> {
    kdbx_service
        .list_open_databases()
        .map(|dbs| dbs.into_iter().map(|d| d.path).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn update_app_preferences(
    new_preferences: AppPreferences,
    settings_service: State<'_, Arc<SettingsService>>,
    kdbx_service: State<'_, Arc<KdbxService>>,
    audit_service: State<'_, Arc<AuditService>>,
) -> Result<(), AppError> {
    let old_preferences = settings_service.get_app_preferences()?;
    settings_service.update_app_preferences(&new_preferences)?;
    kdbx_service.set_backup_settings(new_preferences.backups.clone())?;

    let open_paths = open_vault_paths(&kdbx_service);
    apply_preference_security_audit(
        &audit_service,
        &open_paths,
        &old_preferences,
        &new_preferences,
    );
    Ok(())
}

#[tauri::command]
pub async fn reset_app_preferences(
    settings_service: State<'_, Arc<SettingsService>>,
    kdbx_service: State<'_, Arc<KdbxService>>,
    audit_service: State<'_, Arc<AuditService>>,
) -> Result<AppPreferences, AppError> {
    // Snapshot the pre-reset preferences so the diff/fan-out can emit
    // a `preferences.security_changed` event for every allowlisted
    // leaf that the reset is about to overwrite — otherwise resetting
    // from non-default values would silently change those settings.
    let old_preferences = settings_service.get_app_preferences()?;
    let prefs = settings_service.reset_app_preferences()?;
    kdbx_service.set_backup_settings(prefs.backups.clone())?;

    let open_paths = open_vault_paths(&kdbx_service);
    apply_preference_security_audit(&audit_service, &open_paths, &old_preferences, &prefs);
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod fan_out_security_changes_tests {
    use super::{apply_preference_security_audit, fan_out_security_changes, AppPreferences};
    use crate::commands::settings::AuditSettings;
    use crate::services::audit::format::AuditEvent;
    use crate::services::audit::key::InMemoryAuditKey;
    use crate::services::audit::{AuditFilter, AuditService};
    use std::sync::Arc;
    use tempfile::tempdir;

    /// AC: each (open vault × changed leaf) pair produces exactly one
    /// `preferences.security_changed` record. Two opens × two leaves
    /// must land four records (two per file), and each leaf must reach
    /// each vault — proves the fan-out is the cartesian product, not a
    /// per-call-once shortcut.
    #[test]
    fn each_vault_gets_one_event_per_leaf() {
        let dir = tempdir().expect("tempdir");
        let vault_a = dir.path().join("a.kdbx");
        let vault_b = dir.path().join("b.kdbx");
        std::fs::write(&vault_a, b"a").expect("write a");
        std::fs::write(&vault_b, b"b").expect("write b");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));

        let opens = vec![
            vault_a.to_string_lossy().into_owned(),
            vault_b.to_string_lossy().into_owned(),
        ];
        let leaves = ["security.preventScreenCapture", "audit.retentionDays"];
        fan_out_security_changes(&service, &opens, &leaves);

        for vault in [&vault_a, &vault_b] {
            let events = service.read(vault, &AuditFilter::default()).expect("read");
            assert_eq!(events.len(), 2, "two leaves => two events per vault");
            let names: Vec<&str> = events
                .iter()
                .map(|e| match e {
                    AuditEvent::PreferencesSecurityChanged { setting_name, .. } => {
                        setting_name.as_str()
                    }
                    other => panic!("unexpected variant: {other:?}"),
                })
                .collect();
            assert!(names.contains(&"security.preventScreenCapture"));
            assert!(names.contains(&"audit.retentionDays"));
        }
    }

    /// When no Vault is open, the preference flip still persists but
    /// nothing reaches the audit log. Pin the no-op explicitly so a
    /// future change can't quietly start writing to a global file (the
    /// ADR rejects a global single-stream log on privacy grounds).
    #[test]
    fn empty_open_vault_list_is_a_no_op() {
        let dir = tempdir().expect("tempdir");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        fan_out_security_changes(&service, &[], &["security.preventScreenCapture"]);
        assert!(!service.is_degraded());
        // No vault file should have been created under the audit dir.
        // tempdir() left it nonexistent, so any creation here would be loud.
        assert!(!dir.path().join("audit").exists());
    }

    /// Regression for the P1 review finding on #221: disabling audit
    /// logging in the same submit as another allowlisted flip must NOT
    /// suppress its own `audit.enabled` record. If we flipped the gate
    /// off before fanning out, every event in this submit (including
    /// the disable action itself) would short-circuit inside
    /// `AuditService::record` and the disable would happen silently.
    #[test]
    fn disabling_audit_in_one_submit_still_records_audit_enabled_event() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("v.kdbx");
        std::fs::write(&vault, b"v").expect("write vault");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        // Old prefs: audit enabled (the realistic starting state).
        let old = AppPreferences::default();
        // New prefs: audit disabled + screen-capture toggle flipped.
        let new = AppPreferences {
            audit: AuditSettings {
                enabled: false,
                ..old.audit.clone()
            },
            security: crate::commands::settings::SecuritySettings {
                prevent_screen_capture: !old.security.prevent_screen_capture,
                ..old.security.clone()
            },
            ..old.clone()
        };

        let opens = vec![vault.to_string_lossy().into_owned()];
        apply_preference_security_audit(&service, &opens, &old, &new);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let names: Vec<&str> = events
            .iter()
            .map(|e| match e {
                AuditEvent::PreferencesSecurityChanged { setting_name, .. } => {
                    setting_name.as_str()
                }
                other => panic!("unexpected variant: {other:?}"),
            })
            .collect();
        assert!(
            names.contains(&"audit.enabled"),
            "audit.enabled disable must be recorded, got: {names:?}"
        );
        assert!(
            names.contains(&"security.preventScreenCapture"),
            "co-submitted flip must also be recorded, got: {names:?}"
        );
        // Final gate state matches the new preferences.
        assert!(!service.is_enabled());
    }

    /// Follow-up review finding on #221: a false→true `audit.enabled`
    /// flip must also be recorded. The helper briefly forces the gate
    /// on around the fan-out so the enable event is captured under
    /// "logging on," even though logging was off at the start of the
    /// call.
    #[test]
    fn enabling_audit_from_disabled_records_the_enable_event() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("v.kdbx");
        std::fs::write(&vault, b"v").expect("write vault");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        service.set_enabled(false);

        let old = AppPreferences {
            audit: AuditSettings {
                enabled: false,
                ..AppPreferences::default().audit
            },
            ..AppPreferences::default()
        };
        let new = AppPreferences::default(); // audit.enabled = true

        let opens = vec![vault.to_string_lossy().into_owned()];
        apply_preference_security_audit(&service, &opens, &old, &new);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let names: Vec<&str> = events
            .iter()
            .map(|e| match e {
                AuditEvent::PreferencesSecurityChanged { setting_name, .. } => {
                    setting_name.as_str()
                }
                other => panic!("unexpected variant: {other:?}"),
            })
            .collect();
        assert!(
            names.contains(&"audit.enabled"),
            "audit.enabled enable must be recorded, got: {names:?}"
        );
        assert!(service.is_enabled(), "gate must end up matching new state");
    }

    /// Follow-up review finding on #221: the prior fix forced the
    /// master gate on around the fan-out, which let unrelated
    /// concurrent producers (sharing this `AuditService`) slip events
    /// through the transition window. The corrected design routes
    /// preference-transition events through a force-write path and
    /// leaves the gate strictly tracking `new.audit.enabled`. Pin
    /// that invariant: across a disable transition, the gate must be
    /// off the moment `apply_preference_security_audit` returns —
    /// AND `is_enabled()` reporting it being off mid-call must imply
    /// no transient flip happened. We assert the end-state here and
    /// rely on the service-level `record_preferences_security_changed_bypasses_the_master_gate`
    /// test to pin that the fan-out itself doesn't touch the gate.
    #[test]
    fn disable_transition_never_transiently_widens_the_gate() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("v.kdbx");
        std::fs::write(&vault, b"v").expect("write vault");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));

        let defaults = AppPreferences::default(); // audit enabled
        let new = AppPreferences {
            audit: AuditSettings {
                enabled: false,
                ..defaults.audit
            },
            ..defaults.clone()
        };

        let opens = vec![vault.to_string_lossy().into_owned()];
        apply_preference_security_audit(&service, &opens, &defaults, &new);

        // Final state: gate off (persisted user intent applied).
        assert!(
            !service.is_enabled(),
            "gate must reflect new.audit.enabled after the call"
        );
        // And the disable event itself was captured via force-write.
        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let names: Vec<&str> = events
            .iter()
            .map(|e| match e {
                AuditEvent::PreferencesSecurityChanged { setting_name, .. } => {
                    setting_name.as_str()
                }
                other => panic!("unexpected variant: {other:?}"),
            })
            .collect();
        assert!(
            names.contains(&"audit.enabled"),
            "disable event must be captured, got: {names:?}"
        );
    }

    /// When logging is off at BOTH ends of the transition, the user has
    /// consistently opted out — even an allowlisted change like
    /// `preventScreenCapture` must not write to disk. This pins the
    /// privacy contract: a disabled audit log stays disabled, and no
    /// transient gate-widening leaks into pure-disabled saves.
    #[test]
    fn disabled_at_both_ends_records_nothing_even_when_other_leaves_change() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("v.kdbx");
        std::fs::write(&vault, b"v").expect("write vault");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        service.set_enabled(false);

        let defaults = AppPreferences::default();
        let old = AppPreferences {
            audit: AuditSettings {
                enabled: false,
                ..defaults.audit
            },
            ..defaults.clone()
        };
        let new = AppPreferences {
            audit: AuditSettings {
                enabled: false,
                ..defaults.audit
            },
            security: crate::commands::settings::SecuritySettings {
                prevent_screen_capture: !defaults.security.prevent_screen_capture,
                ..defaults.security.clone()
            },
            ..defaults.clone()
        };

        let opens = vec![vault.to_string_lossy().into_owned()];
        apply_preference_security_audit(&service, &opens, &old, &new);

        // read() returns Ok([]) when no log file exists for this vault.
        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert!(
            events.is_empty(),
            "no events must be recorded when audit is off at both ends, got: {events:?}"
        );
        assert!(
            !service.is_enabled(),
            "gate must remain disabled after the call"
        );
    }

    /// Regression for the P2 review finding on #221: resetting from
    /// non-default values for any allowlisted leaf must emit one
    /// `preferences.security_changed` record per changed leaf, just
    /// like an in-place update would.
    #[test]
    fn reset_from_non_default_values_emits_security_change_records() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("v.kdbx");
        std::fs::write(&vault, b"v").expect("write vault");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));

        let defaults = AppPreferences::default();
        // Pre-reset preferences: flip several allowlisted leaves away
        // from defaults so the reset has something to record.
        let old = AppPreferences {
            security: crate::commands::settings::SecuritySettings {
                prevent_screen_capture: !defaults.security.prevent_screen_capture,
                auto_download_favicons: !defaults.security.auto_download_favicons,
                ..defaults.security.clone()
            },
            audit: AuditSettings {
                enabled: !defaults.audit.enabled,
                retention_days: defaults.audit.retention_days.saturating_add(7),
            },
            ..defaults.clone()
        };

        let opens = vec![vault.to_string_lossy().into_owned()];
        apply_preference_security_audit(&service, &opens, &old, &defaults);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let names: std::collections::HashSet<&str> = events
            .iter()
            .map(|e| match e {
                AuditEvent::PreferencesSecurityChanged { setting_name, .. } => {
                    setting_name.as_str()
                }
                other => panic!("unexpected variant: {other:?}"),
            })
            .collect();
        for want in [
            "security.preventScreenCapture",
            "security.autoDownloadFavicons",
            "audit.enabled",
            "audit.retentionDays",
        ] {
            assert!(names.contains(want), "missing {want} in {names:?}");
        }
    }

    /// Empty leaf set with N open vaults is also a no-op — happens on
    /// every preference save where nothing audited changed (e.g. font
    /// size edit). Must not touch the per-Vault log files.
    #[test]
    fn empty_leaf_list_is_a_no_op() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("v.kdbx");
        std::fs::write(&vault, b"v").expect("write");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        fan_out_security_changes(&service, &[vault.to_string_lossy().into_owned()], &[]);
        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert!(events.is_empty());
    }
}
