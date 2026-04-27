use crate::domain::kdbx::{format_database_version, OpenDatabase};
use crate::domain::secure::SecureString;
use crate::dto::database::DatabaseInfo;
use crate::dto::error::AppError;
use crate::services::kdbx::key::build_database_key;
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

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let key = DatabaseKey::new().with_password(password);
        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        databases.insert(
            normalized_path,
            OpenDatabase {
                db: Some(db),
                path: path.to_string(),
                is_modified: false,
                password: Some(SecureString::from(password)),
                keyfile_path: None,
                version: version.clone(),
                name: name.clone(),
                root_group_id: root_group_id.clone(),
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
                db: Some(db),
                path: path.to_string(),
                is_modified: false,
                password: Some(SecureString::from(password)),
                keyfile_path: Some(keyfile_path.to_string()),
                version: version.clone(),
                name: name.clone(),
                root_group_id: root_group_id.clone(),
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
                db: Some(db),
                path: path.to_string(),
                is_modified: false,
                password: None,
                keyfile_path: Some(keyfile_path.to_string()),
                version: version.clone(),
                name: name.clone(),
                root_group_id: root_group_id.clone(),
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
            name: open_db.name.clone(),
            path: open_db.path.clone(),
            is_modified: open_db.is_modified,
            is_locked: open_db.is_locked(),
            root_group_id: open_db.root_group_id.clone(),
            version: open_db.version.clone(),
        })
    }

    /// Locks a specific database by dropping the decrypted data and password.
    pub fn lock(&self, db_id: &str) -> Result<DatabaseInfo, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        // Drop the decrypted database (frees all entry/group data from memory)
        open_db.db = None;
        // Zeroize and drop the password (SecureString implements ZeroizeOnDrop)
        open_db.password = None;

        Ok(DatabaseInfo {
            name: open_db.name.clone(),
            path: open_db.path.clone(),
            is_modified: open_db.is_modified,
            is_locked: true,
            root_group_id: open_db.root_group_id.clone(),
            version: open_db.version.clone(),
        })
    }

    /// Locks all currently unlocked clean databases.
    /// Databases with unsaved changes remain unlocked to avoid silent data loss.
    /// Returns the list of locked database paths.
    pub fn lock_all(&self) -> Result<Vec<String>, AppError> {
        let mut databases = self.lock_databases()?;
        let mut locked_paths = Vec::new();

        for open_db in databases.values_mut() {
            if !open_db.is_locked() && !open_db.is_modified {
                open_db.db = None;
                open_db.password = None;
                locked_paths.push(open_db.path.clone());
            }
        }

        Ok(locked_paths)
    }

    /// Unlocks a locked database by re-opening it from disk with optional password.
    pub fn unlock(&self, db_id: &str, password: Option<&str>) -> Result<DatabaseInfo, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        if !open_db.is_locked() {
            return Ok(DatabaseInfo {
                name: open_db.name.clone(),
                path: open_db.path.clone(),
                is_modified: open_db.is_modified,
                is_locked: false,
                root_group_id: open_db.root_group_id.clone(),
                version: open_db.version.clone(),
            });
        }

        let path = &open_db.path;
        let keyfile_path = open_db.keyfile_path.clone();

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let key = build_database_key(password, keyfile_path.as_deref())?;
        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        open_db.name.clone_from(&db.root.name);
        open_db.root_group_id = db.root.uuid.to_string();
        open_db.version = format_database_version(&db.config.version);
        open_db.db = Some(db);
        open_db.password = password.map(SecureString::from);
        open_db.is_modified = false;

        Ok(DatabaseInfo {
            name: open_db.name.clone(),
            path: open_db.path.clone(),
            is_modified: false,
            is_locked: false,
            root_group_id: open_db.root_group_id.clone(),
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
