use crate::dto::error::AppError;
use crate::services::kdbx::key::build_database_key;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};

use super::KdbxService;

impl KdbxService {
    /// Saves the open database.
    pub fn save(&self) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        if open_db.password.is_none() && open_db.keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        let path = open_db.path.clone();
        let password = open_db.password.clone();
        let keyfile_path = open_db.keyfile_path.clone();

        atomic_write(
            &path,
            &AtomicWriteOptions {
                preserve_permissions: true,
            },
            |file| {
                let key = build_database_key(password.as_deref(), keyfile_path.as_deref())?;
                open_db
                    .db
                    .save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;

        open_db.is_modified = false;
        Ok(())
    }

    /// Saves the database to a new path.
    pub fn save_as(&self, new_path: &str, new_password: Option<&str>) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let effective_password: Option<String> = new_password
            .map(String::from)
            .or_else(|| open_db.password.clone());

        if effective_password.is_none() && open_db.keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        let keyfile_path = open_db.keyfile_path.clone();

        atomic_write(
            new_path,
            &AtomicWriteOptions {
                preserve_permissions: false,
            },
            |file| {
                let key =
                    build_database_key(effective_password.as_deref(), keyfile_path.as_deref())?;
                open_db
                    .db
                    .save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;

        open_db.path = new_path.to_string();
        if new_password.is_some() {
            open_db.password = new_password.map(String::from);
        }
        open_db.is_modified = false;

        Ok(())
    }
}
