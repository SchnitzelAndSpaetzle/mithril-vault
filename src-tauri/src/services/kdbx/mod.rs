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
        use crate::services::kdbx::backups::filename::{
            parse_backup_filename, parse_manual_backup_filename,
        };

        let settings = self.current_backup_settings()?;
        let target = Path::new(backup_path);
        let canonical_target =
            fs::canonicalize(target).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        // Parse the filename FIRST. Authorization checks must verify both
        // location AND shape — otherwise the command becomes "delete any
        // file in a backup directory", which the issue text explicitly
        // does not authorize. Refuse anything that doesn't decode as a
        // snapshot of *some* vault; later we'll match the encoded vault
        // basename against the open vault we authorize under.
        let target_filename = canonical_target
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "backup path has no filename component: {backup_path}"
                ))
            })?;
        let target_vault = parse_backup_filename(target_filename)
            .or_else(|| parse_manual_backup_filename(target_filename))
            .map(|(v, _)| v)
            .ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "path is not a backup snapshot filename: {backup_path}"
                ))
            })?;

        let databases = self.lock_databases()?;
        let mut authorized = false;
        for open_db in databases.values() {
            let source = Path::new(&open_db.path);
            // The vault basename embedded in the snapshot filename must
            // match the open vault's basename. Without this, an attacker
            // who plants a snapshot-shaped file for a *different* vault
            // inside our backup dir could trick us into deleting it.
            let Some(open_basename) = source.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if open_basename != target_vault {
                continue;
            }
            let Ok(backup_dir) = backups::resolved_backup_dir(source, &settings) else {
                continue;
            };
            // Refuse to canonicalize through a symlink at the backup dir.
            // Otherwise an attacker who plants a symlink could shift the
            // allowed delete boundary to the symlink target and use this
            // command to remove arbitrary files. Skip this vault's
            // authorization path instead of bubbling the error: another
            // open vault may legitimately own the target.
            if backups::assert_backup_dir_not_symlinked(&backup_dir).is_err() {
                continue;
            }
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
                "backup path is not an authorized snapshot for any open vault: {backup_path}"
            )));
        }

        fs::remove_file(&canonical_target).map_err(|e| AppError::Io(e.to_string()))?;
        Ok(())
    }

    /// Enumerates snapshots that belong to `db_path`, sorted newest-first.
    ///
    /// Requires `db_path` to map to a currently-open Vault. Exposed over IPC,
    /// this check prevents callers from enumerating snapshot metadata
    /// (filenames, timestamps, sizes) for arbitrary paths the user has not
    /// opened — a metadata-disclosure surface even when the bytes are
    /// encrypted.
    pub fn list_backups(&self, db_path: &str) -> Result<Vec<BackupListEntry>, AppError> {
        // Resolve via the open-database map so an alias path (e.g. symlink
        // to the canonical file) is accepted iff the canonical path is open.
        // Matches `snapshot_after_open`'s resolution behavior.
        let normalized = Self::normalize_path(db_path);
        let stored_path = {
            let databases = self.lock_databases()?;
            databases
                .get(&normalized)
                .map(|open_db| open_db.path.clone())
                .ok_or_else(|| AppError::DatabaseNotFound(db_path.to_string()))?
        };
        let settings = self.current_backup_settings()?;
        Ok(backups::list_for(Path::new(&stored_path), &settings)?)
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
