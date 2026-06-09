use crate::domain::kdbx::{ensure_writable_version, format_database_version};
use crate::domain::secure::SecureString;
use crate::dto::error::AppError;
use crate::services::kdbx::backups::{self, BackupInfo};
use crate::services::kdbx::key::build_database_key;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use std::path::Path;

use super::KdbxService;

impl KdbxService {
    /// Saves a specific open database.
    ///
    /// Returns the snapshot info when a pre-image backup was taken, or
    /// `None` when the source did not yet exist (first save) or backups
    /// are disabled. The command layer uses this to decide whether to
    /// emit a `backup-created` event.
    pub fn save(&self, db_id: &str) -> Result<Option<BackupInfo>, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        if open_db.is_locked() {
            return Err(AppError::DatabaseLocked(open_db.path.clone()));
        }

        if open_db.password.is_none() && open_db.keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        let path = open_db.path.clone();
        let password = open_db.password.clone();
        let keyfile_path = open_db.keyfile_path.clone();

        // Fail-closed pre-image snapshot. Returns Ok(None) when the source
        // does not yet exist (first save of a brand-new vault or first save
        // after save_as to a fresh path) — those proceed without a backup.
        let backup_settings = self.current_backup_settings()?;
        let snapshot_info = backups::snapshot(Path::new(&path), &backup_settings)?;

        // Upgrade legacy KDBX 4.0 vaults to the writable 4.1 format and refresh
        // the cached display version to match what lands on disk.
        let db = open_db
            .db
            .as_mut()
            .ok_or_else(|| AppError::DatabaseLocked(path.clone()))?;
        ensure_writable_version(db);
        let version_label = format_database_version(&db.config.version);
        let db = &*db;

        atomic_write(
            &path,
            &AtomicWriteOptions {
                preserve_permissions: true,
            },
            |file| {
                let key = build_database_key(
                    password.as_ref().map(SecureString::as_str),
                    keyfile_path.as_deref(),
                )?;
                db.save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;

        open_db.version = version_label;
        open_db.is_modified = false;
        Ok(snapshot_info)
    }

    /// Saves a specific database to a new path.
    pub fn save_as(
        &self,
        db_id: &str,
        new_path: &str,
        new_password: Option<&str>,
    ) -> Result<(), AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let destination_normalized_path = Self::normalize_path(new_path);

        if destination_normalized_path != normalized_path
            && databases.contains_key(&destination_normalized_path)
        {
            return Err(AppError::DatabaseAlreadyOpen(new_path.to_string()));
        }

        {
            let open_db = databases
                .get_mut(&normalized_path)
                .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

            if open_db.is_locked() {
                return Err(AppError::DatabaseLocked(open_db.path.clone()));
            }

            let effective_password: Option<SecureString> = new_password
                .map(SecureString::from)
                .or_else(|| open_db.password.clone());

            if effective_password.is_none() && open_db.keyfile_path.is_none() {
                return Err(AppError::NoCredentials);
            }

            let keyfile_path = open_db.keyfile_path.clone();

            // Upgrade legacy KDBX 4.0 vaults to the writable 4.1 format and
            // refresh the cached display version to match what lands on disk.
            let db = open_db
                .db
                .as_mut()
                .ok_or_else(|| AppError::DatabaseLocked(open_db.path.clone()))?;
            ensure_writable_version(db);
            let version_label = format_database_version(&db.config.version);
            let db = &*db;

            atomic_write(
                new_path,
                &AtomicWriteOptions {
                    preserve_permissions: false,
                },
                |file| {
                    let key = build_database_key(
                        effective_password.as_ref().map(SecureString::as_str),
                        keyfile_path.as_deref(),
                    )?;
                    db.save(file, key)
                        .map_err(|e| AppError::Kdbx(e.to_string()))
                },
            )?;

            open_db.path = new_path.to_string();
            if new_password.is_some() {
                open_db.password = new_password.map(SecureString::from);
            }
            open_db.version = version_label;
            open_db.is_modified = false;
        }

        let new_normalized_path = Self::normalize_path(new_path);
        if new_normalized_path != normalized_path {
            if let Some(open_db) = databases.remove(&normalized_path) {
                databases.insert(new_normalized_path, open_db);
            }
        }

        Ok(())
    }
}
