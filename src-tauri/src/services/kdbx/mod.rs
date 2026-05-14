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
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

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
