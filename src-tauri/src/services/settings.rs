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
        // Reject snapshot paths so `Open Recent…` never lists a backup. A
        // user who unlocked a snapshot directly would have their next save
        // overwrite the backup itself — corrupting the very pre-image the
        // backup module exists to preserve.
        if Self::is_snapshot_path(path) {
            return Err(AppError::InvalidInput(format!(
                "refusing to add backup snapshot path to recent databases: {path}"
            )));
        }

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

    /// Detects whether a path's filename matches the auto- or manual-
    /// snapshot pattern owned by the backup module. Used to keep snapshot
    /// files out of `recent_databases` — opening one as a regular vault
    /// would clobber the backup on the next save.
    ///
    /// Both arms parse the canonical timestamped pattern via the backup
    /// module's filename parsers so a legitimate vault whose basename
    /// happens to contain `.backup.manual.` (e.g. `team.backup.manual.notes.kdbx`)
    /// is NOT incorrectly rejected. Substring matching here would lose
    /// `Open Recent…` history for valid files.
    fn is_snapshot_path(path: &str) -> bool {
        use crate::services::kdbx::backups::filename::{
            parse_backup_filename, parse_manual_backup_filename,
        };
        let Some(filename) = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
        else {
            return false;
        };
        parse_backup_filename(filename).is_some()
            || parse_manual_backup_filename(filename).is_some()
    }

    /// Rejects out-of-range values on the App Preferences boundary so a
    /// malformed IPC payload (or a future settings.json with corrupt values)
    /// cannot push the backup module into invariant violations.
    fn validate_preferences(prefs: &AppPreferences) -> Result<(), AppError> {
        const MAX_VERSIONS_RANGE: std::ops::RangeInclusive<u32> = 1..=500;
        const AUDIT_RETENTION_RANGE: std::ops::RangeInclusive<u32> = 1..=365;
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
        let r = prefs.audit.retention_days;
        if !AUDIT_RETENTION_RANGE.contains(&r) {
            return Err(AppError::InvalidInput(format!(
                "audit.retentionDays must be in 1..=365, got {r}"
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

/// Allowlisted App Preference leaves whose flips emit a
/// `preferences.security_changed` audit event. Adding a key here is a
/// one-line change; UI/preference changes (theme, language, layout) are
/// deliberately excluded.
///
/// The lookup is a `(setting_name, did_change)` pair so each entry pins
/// both the wire identifier (dot-pathed camelCase, matching the JSON
/// shape of `AppPreferences` over IPC) and the equality check against
/// the typed pref struct. Old/new values are never returned — the audit
/// log records THAT a flip happened, not what it flipped to.
type SecurityChangeProbe = fn(&AppPreferences, &AppPreferences) -> bool;
const AUDITED_SECURITY_LEAVES: &[(&str, SecurityChangeProbe)] = &[
    ("security.clipboardClearTimeout", |o, n| {
        o.security.clipboard_clear_timeout != n.security.clipboard_clear_timeout
    }),
    ("security.preventScreenCapture", |o, n| {
        o.security.prevent_screen_capture != n.security.prevent_screen_capture
    }),
    ("security.autoDownloadFavicons", |o, n| {
        o.security.auto_download_favicons != n.security.auto_download_favicons
    }),
    ("security.allowThirdPartyFaviconFallbacks", |o, n| {
        o.security.allow_third_party_favicon_fallbacks
            != n.security.allow_third_party_favicon_fallbacks
    }),
    ("security.autoLockTimeout", |o, n| {
        o.security.auto_lock_timeout != n.security.auto_lock_timeout
    }),
    ("audit.enabled", |o, n| o.audit.enabled != n.audit.enabled),
    ("audit.retentionDays", |o, n| {
        o.audit.retention_days != n.audit.retention_days
    }),
];

/// Returns the wire names of allowlisted App Preference leaves that
/// differ between `old` and `new`, in declaration order. Empty when
/// nothing audited changed — the caller (commands layer) fans the
/// result across every currently-open Vault, recording one
/// `preferences.security_changed` event per (vault, leaf) pair.
pub fn diff_security_changes(old: &AppPreferences, new: &AppPreferences) -> Vec<&'static str> {
    AUDITED_SECURITY_LEAVES
        .iter()
        .filter_map(|(name, changed)| if changed(old, new) { Some(*name) } else { None })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod validate_preferences_tests {
    use super::SettingsService;
    use crate::commands::settings::{AppPreferences, AuditSettings, BackupSettings};
    use crate::dto::error::AppError;

    fn prefs_with_directory(dir: Option<&str>) -> AppPreferences {
        AppPreferences {
            backups: BackupSettings {
                enabled: true,
                max_versions: 10,
                directory: dir.map(String::from),
                on_open: false,
            },
            ..AppPreferences::default()
        }
    }

    fn prefs_with_audit_retention(days: u32) -> AppPreferences {
        AppPreferences {
            audit: AuditSettings {
                enabled: true,
                retention_days: days,
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

    #[test]
    fn audit_retention_zero_is_rejected() {
        // Zero would mean "drop every event the moment it is written" once
        // the retention policy lands in #6 — silently neutralizing the
        // audit log. Reject at the boundary so a hand-edited settings.json
        // cannot push the retention task into that state.
        let prefs = prefs_with_audit_retention(0);
        match SettingsService::validate_preferences(&prefs) {
            Err(AppError::InvalidInput(msg)) => {
                assert!(
                    msg.contains("audit.retentionDays") && msg.contains("1..=365"),
                    "error should name the field and range, got: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn audit_retention_above_max_is_rejected() {
        let prefs = prefs_with_audit_retention(366);
        match SettingsService::validate_preferences(&prefs) {
            Err(AppError::InvalidInput(msg)) => {
                assert!(
                    msg.contains("audit.retentionDays"),
                    "error should mention 'audit.retentionDays', got: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn audit_retention_boundary_values_are_accepted() {
        for days in [1_u32, 90, 365] {
            let prefs = prefs_with_audit_retention(days);
            SettingsService::validate_preferences(&prefs)
                .unwrap_or_else(|_| panic!("retention_days={days} should validate"));
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod diff_security_changes_tests {
    use super::diff_security_changes;
    use crate::commands::settings::{
        AppPreferences, AppearanceSettings, AuditSettings, GeneralSettings, SecuritySettings,
    };
    use std::collections::HashSet;

    /// Tracer: flipping a single allowlisted leaf reports exactly that
    /// leaf's wire name. Nothing else changed, so nothing else fires.
    #[test]
    fn flipping_clipboard_clear_timeout_emits_one_leaf() {
        let old = AppPreferences::default();
        let new = AppPreferences {
            security: SecuritySettings {
                clipboard_clear_timeout: old.security.clipboard_clear_timeout + 10,
                ..old.security.clone()
            },
            ..old.clone()
        };
        let leaves = diff_security_changes(&old, &new);
        assert_eq!(leaves, vec!["security.clipboardClearTimeout"]);
    }

    /// PRD AC: UI/preference changes (theme, language, layout) are
    /// deliberately excluded. The audit log must stay tightly focused on
    /// security-relevant flips so the user can scan it without the signal
    /// drowning in cosmetic noise.
    #[test]
    fn changing_theme_emits_no_leaves() {
        let old = AppPreferences::default();
        let new = AppPreferences {
            appearance: AppearanceSettings {
                theme: "dark".into(),
                ..old.appearance.clone()
            },
            ..old.clone()
        };
        let leaves = diff_security_changes(&old, &new);
        assert!(leaves.is_empty(), "theme is not audited, got: {leaves:?}");
    }

    #[test]
    fn changing_language_emits_no_leaves() {
        let old = AppPreferences::default();
        let new = AppPreferences {
            general: GeneralSettings {
                language: "de".into(),
                ..old.general.clone()
            },
            ..old.clone()
        };
        let leaves = diff_security_changes(&old, &new);
        assert!(
            leaves.is_empty(),
            "language is not audited, got: {leaves:?}"
        );
    }

    /// AC: "Multiple changes in one call produce multiple events." The
    /// caller emits one audit record per leaf in the returned vec, so
    /// missing one here means a missing audit record in production.
    #[test]
    fn two_allowlisted_flips_in_one_call_emit_both_leaves() {
        let old = AppPreferences::default();
        let new = AppPreferences {
            security: SecuritySettings {
                prevent_screen_capture: !old.security.prevent_screen_capture,
                auto_download_favicons: !old.security.auto_download_favicons,
                ..old.security.clone()
            },
            ..old.clone()
        };
        let leaves: HashSet<&str> = diff_security_changes(&old, &new).into_iter().collect();
        let expected: HashSet<&str> = [
            "security.preventScreenCapture",
            "security.autoDownloadFavicons",
        ]
        .into_iter()
        .collect();
        assert_eq!(leaves, expected);
    }

    /// Each allowlisted leaf has its own test row: flipping that leaf in
    /// isolation must produce exactly its wire name. The table doubles as
    /// living documentation of which paths emit and which do not. Adding
    /// a new key to the allowlist requires adding a row here too — if a
    /// future contributor forgets, this test stays green only by accident.
    #[test]
    fn every_allowlisted_leaf_emits_its_own_wire_name() {
        struct Case {
            name: &'static str,
            mutate: fn(&mut AppPreferences),
        }
        let cases = [
            Case {
                name: "security.clipboardClearTimeout",
                mutate: |p| p.security.clipboard_clear_timeout += 1,
            },
            Case {
                name: "security.preventScreenCapture",
                mutate: |p| p.security.prevent_screen_capture = !p.security.prevent_screen_capture,
            },
            Case {
                name: "security.autoDownloadFavicons",
                mutate: |p| p.security.auto_download_favicons = !p.security.auto_download_favicons,
            },
            Case {
                name: "security.allowThirdPartyFaviconFallbacks",
                mutate: |p| {
                    p.security.allow_third_party_favicon_fallbacks =
                        !p.security.allow_third_party_favicon_fallbacks;
                },
            },
            Case {
                name: "security.autoLockTimeout",
                mutate: |p| p.security.auto_lock_timeout += 60,
            },
            Case {
                name: "audit.enabled",
                mutate: |p| p.audit.enabled = !p.audit.enabled,
            },
            Case {
                name: "audit.retentionDays",
                mutate: |p| p.audit.retention_days += 1,
            },
        ];
        for case in &cases {
            let old = AppPreferences::default();
            let mut new = old.clone();
            (case.mutate)(&mut new);
            let leaves = diff_security_changes(&old, &new);
            assert_eq!(
                leaves,
                vec![case.name],
                "flipping {} alone must emit only that leaf",
                case.name
            );
        }
    }

    /// Non-allowlisted security siblings (`showPasswordByDefault`,
    /// `minimizeToTray`, `startMinimized`, `clearClipboardOnLock`,
    /// `showClipboardCountdown`) must NOT emit — they are UX
    /// preferences that happen to live under `security.*`, not
    /// security-relevant flips. Pinning the carve-out here means a
    /// future "let's just audit everything under security.*" refactor
    /// fails loudly.
    #[test]
    fn non_allowlisted_security_siblings_emit_no_leaves() {
        let old = AppPreferences::default();
        let new = AppPreferences {
            security: SecuritySettings {
                show_password_by_default: !old.security.show_password_by_default,
                minimize_to_tray: !old.security.minimize_to_tray,
                start_minimized: !old.security.start_minimized,
                clear_clipboard_on_lock: !old.security.clear_clipboard_on_lock,
                show_clipboard_countdown: !old.security.show_clipboard_countdown,
                ..old.security.clone()
            },
            ..old.clone()
        };
        let leaves = diff_security_changes(&old, &new);
        assert!(
            leaves.is_empty(),
            "non-allowlisted security siblings must not emit, got: {leaves:?}"
        );
    }

    /// AC: "Adding a key to the allowlist is a one-line change." The
    /// table-driven definition above already makes the source-side
    /// one-liner; this test pins the runtime shape (declaration order,
    /// no duplicate wire names) so a future edit can't silently shadow a
    /// key or reorder it in a way that surprises log consumers.
    #[test]
    fn allowlist_wire_names_are_unique() {
        use super::AUDITED_SECURITY_LEAVES;
        let mut seen = HashSet::new();
        for (name, _) in AUDITED_SECURITY_LEAVES {
            assert!(
                seen.insert(*name),
                "duplicate wire name in allowlist: {name}"
            );
        }
    }

    /// The audit settings unit struct must round-trip through the diff:
    /// an unchanged `AuditSettings` (constructed via `..Default::default()`)
    /// across the call site must NOT cause a spurious diff. This pins the
    /// equality check against the typed `AuditSettings` struct so a future
    /// refactor of `AuditSettings` (adding a field, swapping the order)
    /// can't accidentally flip the eq invariant.
    #[test]
    fn unchanged_audit_struct_does_not_diff() {
        let old = AppPreferences::default();
        let new = AppPreferences {
            audit: AuditSettings {
                ..old.audit.clone()
            },
            ..old.clone()
        };
        assert!(diff_security_changes(&old, &new).is_empty());
    }
}
