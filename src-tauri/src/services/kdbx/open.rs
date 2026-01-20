use crate::domain::kdbx::{format_database_version, OpenDatabase};
use crate::domain::secure::SecureString;
use crate::dto::database::DatabaseInfo;
use crate::dto::error::AppError;
use keepass::error::{
    BlockStreamError, CryptographyError, DatabaseIntegrityError, DatabaseKeyError,
    DatabaseOpenError,
};
use keepass::{Database, DatabaseKey};
use std::fs::File;

use super::KdbxService;

impl KdbxService {
    /// Opens a database with a password.
    pub fn open(&self, path: &str, password: &str) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let key = DatabaseKey::new().with_password(password);
        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: Some(SecureString::from(password)),
            keyfile_path: None,
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }

    /// Opens a database with a password and keyfile.
    pub fn open_with_keyfile(
        &self,
        path: &str,
        password: &str,
        keyfile_path: &str,
    ) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let mut keyfile =
            File::open(keyfile_path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let key = DatabaseKey::new()
            .with_password(password)
            .with_keyfile(&mut keyfile)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;

        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: Some(SecureString::from(password)),
            keyfile_path: Some(keyfile_path.to_string()),
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }

    /// Opens a database using only a keyfile.
    pub fn open_with_keyfile_only(
        &self,
        path: &str,
        keyfile_path: &str,
    ) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let mut keyfile = File::open(keyfile_path).map_err(|_| AppError::KeyfileNotFound)?;

        let key = DatabaseKey::new()
            .with_keyfile(&mut keyfile)
            .map_err(|_| AppError::KeyfileInvalid)?;

        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: None,
            keyfile_path: Some(keyfile_path.to_string()),
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }

    /// Closes the active database.
    pub fn close(&self) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_none() {
            return Err(AppError::DatabaseNotOpen);
        }

        *db_lock = None;
        Ok(())
    }

    /// Returns metadata for the open database.
    pub fn get_info(&self) -> Result<DatabaseInfo, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        Ok(DatabaseInfo {
            name: open_db.db.root.name.clone(),
            path: open_db.path.clone(),
            is_modified: open_db.is_modified,
            is_locked: false,
            root_group_id: open_db.db.root.uuid.to_string(),
            version: open_db.version.clone(),
        })
    }
}

fn map_open_error(err: DatabaseOpenError) -> AppError {
    match err {
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey)
        | DatabaseOpenError::DatabaseIntegrity(
            DatabaseIntegrityError::BlockStream(BlockStreamError::BlockHashMismatch { .. })
            | DatabaseIntegrityError::HeaderHashMismatch
            | DatabaseIntegrityError::Cryptography(
                CryptographyError::Unpadding(_) | CryptographyError::Padding(_),
            ),
        ) => AppError::InvalidPassword,
        other => AppError::Kdbx(other.to_string()),
    }
}
