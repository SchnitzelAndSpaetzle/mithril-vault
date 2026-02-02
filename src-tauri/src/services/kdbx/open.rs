use crate::domain::kdbx::{format_database_version, OpenDatabase};
use crate::domain::secure::SecureString;
use crate::dto::database::DatabaseInfo;
use crate::dto::error::AppError;
use crate::services::file_lock::FileLockService;
use keepass::error::{
    BlockStreamError, CompressionConfigError, CryptographyError, DatabaseIntegrityError,
    DatabaseKeyError, DatabaseOpenError, InnerCipherConfigError, KdfConfigError,
    OuterCipherConfigError,
};
use keepass::{Database, DatabaseKey};
use std::fs::File;

use super::KdbxService;

impl KdbxService {
    /// Opens a database with a password.
    /// Returns the database info. If the database is already open, returns an error.
    pub fn open(&self, path: &str, password: &str) -> Result<DatabaseInfo, AppError> {
        let normalized_path = Self::normalize_path(path);
        let mut databases = self.lock_databases()?;

        // Check if this specific database is already open
        if databases.contains_key(&normalized_path) {
            return Err(AppError::DatabaseAlreadyOpen(path.to_string()));
        }

        // Acquire file lock before opening database
        let file_lock = FileLockService::try_acquire_lock(path)?;

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let key = DatabaseKey::new().with_password(password);
        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        databases.insert(
            normalized_path,
            OpenDatabase {
                db,
                path: path.to_string(),
                is_modified: false,
                password: Some(SecureString::from(password)),
                keyfile_path: None,
                version: version.clone(),
                file_lock: Some(file_lock),
            },
        );

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
        let normalized_path = Self::normalize_path(path);
        let mut databases = self.lock_databases()?;

        // Check if this specific database is already open
        if databases.contains_key(&normalized_path) {
            return Err(AppError::DatabaseAlreadyOpen(path.to_string()));
        }

        // Acquire file lock before opening database
        let file_lock = FileLockService::try_acquire_lock(path)?;

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

        databases.insert(
            normalized_path,
            OpenDatabase {
                db,
                path: path.to_string(),
                is_modified: false,
                password: Some(SecureString::from(password)),
                keyfile_path: Some(keyfile_path.to_string()),
                version: version.clone(),
                file_lock: Some(file_lock),
            },
        );

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
        let normalized_path = Self::normalize_path(path);
        let mut databases = self.lock_databases()?;

        // Check if this specific database is already open
        if databases.contains_key(&normalized_path) {
            return Err(AppError::DatabaseAlreadyOpen(path.to_string()));
        }

        // Acquire file lock before opening database
        let file_lock = FileLockService::try_acquire_lock(path)?;

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let mut keyfile = File::open(keyfile_path).map_err(|_| AppError::KeyfileNotFound)?;

        let key = DatabaseKey::new()
            .with_keyfile(&mut keyfile)
            .map_err(|_| AppError::KeyfileInvalid)?;

        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        databases.insert(
            normalized_path,
            OpenDatabase {
                db,
                path: path.to_string(),
                is_modified: false,
                password: None,
                keyfile_path: Some(keyfile_path.to_string()),
                version: version.clone(),
                file_lock: Some(file_lock),
            },
        );

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }

    /// Closes a specific database by its path.
    pub fn close(&self, db_id: &str) -> Result<(), AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;

        if databases.remove(&normalized_path).is_none() {
            return Err(AppError::DatabaseNotFound(db_id.to_string()));
        }

        Ok(())
    }

    /// Returns metadata for a specific open database.
    pub fn get_info(&self, db_id: &str) -> Result<DatabaseInfo, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

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
        // Authentication errors - incorrect credentials
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey)
        | DatabaseOpenError::DatabaseIntegrity(
            DatabaseIntegrityError::BlockStream(BlockStreamError::BlockHashMismatch { .. })
            | DatabaseIntegrityError::Cryptography(
                CryptographyError::Unpadding(_) | CryptographyError::Padding(_),
            ),
        ) => AppError::InvalidPassword,

        // Header integrity errors
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::HeaderHashMismatch) => {
            AppError::HeaderIntegrityError
        }

        // Invalid KDBX file format
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::InvalidKDBXIdentifier) => {
            AppError::InvalidKdbxFile
        }

        // Unsupported KDBX version
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::InvalidKDBXVersion {
            file_major_version,
            file_minor_version,
            ..
        }) => AppError::UnsupportedKdbxVersion(format!(
            "KDBX {file_major_version}.{file_minor_version}"
        )),

        // Unsupported outer cipher
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::OuterCipher(
            OuterCipherConfigError::InvalidOuterCipherID { cid },
        )) => AppError::UnsupportedCipher(format!("Unknown outer cipher ID: {cid:?}")),

        // Unsupported inner cipher
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::InnerCipher(
            InnerCipherConfigError::InvalidInnerCipherID { cid },
        )) => AppError::UnsupportedCipher(format!("Unknown inner cipher ID: {cid}")),

        // Unsupported compression
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::Compression(
            CompressionConfigError::InvalidCompressionSuite { cid },
        )) => AppError::HeaderParseError(format!("Unknown compression ID: {cid}")),

        // Unsupported KDF
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::KdfSettings(
            KdfConfigError::InvalidKDFUUID { uuid },
        )) => AppError::UnsupportedKdf(format!("Unknown KDF UUID: {uuid:?}")),
        DatabaseOpenError::DatabaseIntegrity(DatabaseIntegrityError::KdfSettings(
            KdfConfigError::InvalidKDFVersion { version },
        )) => AppError::UnsupportedKdf(format!("Unsupported KDF version: {version}")),

        // All other errors
        other => AppError::Kdbx(other.to_string()),
    }
}
