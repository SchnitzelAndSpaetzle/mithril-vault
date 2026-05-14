pub mod backups;
pub mod conversions;
pub mod create;
pub mod custom_icons;
pub mod entries;
pub mod favicons;
pub mod groups;
pub mod header;
pub mod key;
pub mod keyfile;
pub mod open;
pub mod save;
pub mod vault;

use crate::commands::settings::BackupSettings;
use crate::domain::kdbx::OpenDatabase;
use crate::dto::database::DatabaseInfo;
use crate::dto::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use self::backups::BackupListEntry;
use self::favicons::FaviconCooldown;

pub struct KdbxService {
    /// Map of normalized database paths to open databases.
    /// The key is the canonical/normalized path to ensure consistent lookups.
    databases: Mutex<HashMap<String, OpenDatabase>>,
    pub(crate) favicons: FaviconCooldown,
    backup_settings: Mutex<BackupSettings>,
}

impl KdbxService {
    /// Creates a new KDBX service.
    pub fn new() -> Self {
        Self {
            databases: Mutex::new(HashMap::new()),
            favicons: FaviconCooldown::new(),
            backup_settings: Mutex::new(BackupSettings::default()),
        }
    }

    /// Replaces the backup settings the save hook reads on each save.
    ///
    /// Called from app setup with the persisted value and from the settings
    /// update command whenever the user changes the toggle.
    pub fn set_backup_settings(&self, settings: BackupSettings) -> Result<(), AppError> {
        let mut guard = self.backup_settings.lock().map_err(|_| AppError::Lock)?;
        *guard = settings;
        Ok(())
    }

    pub fn current_backup_settings(&self) -> Result<BackupSettings, AppError> {
        let guard = self.backup_settings.lock().map_err(|_| AppError::Lock)?;
        Ok(guard.clone())
    }

    /// Normalizes a database path for consistent `HashMap` keys.
    /// Uses canonical path when possible, falls back to the original path.
    pub fn normalize_path(path: &str) -> String {
        Path::new(path)
            .canonicalize()
            .map_or_else(|_| path.to_string(), |p| p.to_string_lossy().to_string())
    }

    /// Acquires a lock on the databases `HashMap`.
    pub(crate) fn lock_databases(
        &self,
    ) -> Result<MutexGuard<'_, HashMap<String, OpenDatabase>>, AppError> {
        self.databases.lock().map_err(|_| AppError::Lock)
    }

    /// Checks if a database at the given path is already open.
    pub fn is_database_open(&self, path: &str) -> Result<bool, AppError> {
        let normalized = Self::normalize_path(path);
        let databases = self.lock_databases()?;
        Ok(databases.contains_key(&normalized))
    }

    /// Reports whether an open database is currently locked. Returns
    /// `Ok(None)` when the database is not open at all — distinct from
    /// "open and unlocked" so callers can tell apart the two states.
    pub fn is_database_locked(&self, path: &str) -> Result<Option<bool>, AppError> {
        let normalized = Self::normalize_path(path);
        let databases = self.lock_databases()?;
        Ok(databases.get(&normalized).map(OpenDatabase::is_locked))
    }

    /// Deletes a backup snapshot from disk after verifying that the path
    /// resolves inside the backup directory of at least one currently-open
    /// Vault.
    ///
    /// Path-safety: the supplied path is canonicalized and compared against
    /// the canonicalized backup directory of every open Vault. A path that
    /// does not resolve inside any of those directories is rejected without
    /// touching the filesystem — protecting against accidental or malicious
    /// deletes of unrelated files (e.g. system files, the Vault itself).
    ///
    /// Returns `InvalidInput` when the path falls outside every open vault's
    /// backup dir.
    pub fn delete_backup(&self, backup_path: &str) -> Result<(), AppError> {
        let settings = self.current_backup_settings()?;
        let target = Path::new(backup_path);
        let canonical_target =
            fs::canonicalize(target).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let databases = self.lock_databases()?;
        let mut authorized = false;
        for open_db in databases.values() {
            let source = Path::new(&open_db.path);
            let Ok(backup_dir) = backups::resolved_backup_dir(source, &settings) else {
                continue;
            };
            let Ok(canonical_dir) = fs::canonicalize(&backup_dir) else {
                continue;
            };
            if canonical_target.starts_with(&canonical_dir) {
                authorized = true;
                break;
            }
        }
        drop(databases);

        if !authorized {
            return Err(AppError::InvalidInput(format!(
                "backup path is not inside any open vault's backup directory: {backup_path}"
            )));
        }

        fs::remove_file(&canonical_target).map_err(|e| AppError::Io(e.to_string()))?;
        Ok(())
    }

    /// Enumerates snapshots that belong to `db_path`, sorted newest-first.
    ///
    /// Does NOT require the path to be currently open — the read is a pure
    /// directory walk against the resolved backup directory, with filename
    /// filtering keyed on the vault's basename. The Settings UI calls this
    /// for the currently-active Vault, but the boundary is path-based so the
    /// command stays simple and predictable.
    pub fn list_backups(&self, db_path: &str) -> Result<Vec<BackupListEntry>, AppError> {
        let settings = self.current_backup_settings()?;
        Ok(backups::list_for(Path::new(db_path), &settings)?)
    }

    /// Returns a list of all currently open databases.
    pub fn list_open_databases(&self) -> Result<Vec<DatabaseInfo>, AppError> {
        let databases = self.lock_databases()?;
        let mut infos = Vec::with_capacity(databases.len());

        for open_db in databases.values() {
            infos.push(DatabaseInfo {
                name: open_db.name.clone(),
                path: open_db.path.clone(),
                is_modified: open_db.is_modified,
                is_locked: open_db.is_locked(),
                root_group_id: open_db.root_group_id.clone(),
                version: open_db.version.clone(),
            });
        }

        Ok(infos)
    }
}

impl Default for KdbxService {
    fn default() -> Self {
        Self::new()
    }
}
