use crate::domain::kdbx::{format_database_version, OpenDatabase};
use crate::domain::secure::SecureString;
use crate::dto::database::DatabaseInfo;
use crate::dto::error::AppError;
use crate::services::kdbx::backups::{self, BackupInfo};
use crate::services::kdbx::key::build_database_key;
use keepass::error::{
    BlockStreamError, CompressionConfigError, CryptographyError, DatabaseFormatError,
    DatabaseKeyError, DatabaseOpenError, DatabaseVersionParseError, InnerCipherConfigError,
    Kdbx3OpenError, Kdbx3OuterHeaderError, Kdbx4InnerHeaderError, Kdbx4OpenError,
    Kdbx4OuterHeaderError, KdfConfigError, OuterCipherConfigError,
};
use keepass::{Database, DatabaseKey};
use std::fs::File;
use std::path::Path;

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

        let root_group_id = db.root().id().uuid().to_string();
        let name = db.root().name.clone();
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

        let root_group_id = db.root().id().uuid().to_string();
        let name = db.root().name.clone();
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

        let root_group_id = db.root().id().uuid().to_string();
        let name = db.root().name.clone();
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
    ///
    /// Returns `(info, did_transition)` where `did_transition` is `true`
    /// iff *this* call actually moved the DB from unlocked to locked. The
    /// flag is computed inside the same critical section that mutates the
    /// state, so concurrent callers cannot both observe themselves as the
    /// transitioning one (TOCTOU). Callers that need to record a single
    /// audit event per real transition (`lock_database`, auto-lock,
    /// app-quit) must gate on this flag rather than on a pre-check.
    pub fn lock(&self, db_id: &str) -> Result<(DatabaseInfo, bool), AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let did_transition = !open_db.is_locked();
        // Drop the decrypted database (frees all entry/group data from memory)
        open_db.db = None;
        // Zeroize and drop the password (SecureString implements ZeroizeOnDrop)
        open_db.password = None;

        Ok((
            DatabaseInfo {
                name: open_db.name.clone(),
                path: open_db.path.clone(),
                is_modified: open_db.is_modified,
                is_locked: true,
                root_group_id: open_db.root_group_id.clone(),
                version: open_db.version.clone(),
            },
            did_transition,
        ))
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

    /// Open-side backup hook (issue #193).
    ///
    /// Run by the command layer after a successful `open` / `unlock`. Honours
    /// the currently-loaded `BackupSettings` — silently a no-op when
    /// `enabled` or `on_open` is false, or when the latest existing snapshot
    /// already matches the source on disk.
    ///
    /// Resolves `db_id` through the open-database map so a caller passing an
    /// alias path (e.g., a symlink resolving to the same canonical file)
    /// snapshots against the canonical stored location. Without that
    /// resolution the snapshot would land next to the alias and dedup would
    /// look in the wrong directory on the next unlock.
    ///
    /// Errors are NOT propagated as `open` failures: callers should surface a
    /// `backup-warning` event on `Err(BackupError)` and otherwise carry on.
    pub fn snapshot_after_open(
        &self,
        db_id: &str,
    ) -> Result<Option<BackupInfo>, backups::BackupError> {
        let stored_path = self.resolve_open_db_path(db_id)?;
        let settings =
            self.current_backup_settings()
                .map_err(|_| backups::BackupError::BackupFailed {
                    path: std::path::PathBuf::from(db_id),
                    source: std::io::Error::other("backup settings lock poisoned"),
                })?;
        backups::snapshot_on_open(Path::new(&stored_path), &settings)
    }

    /// Looks up the canonical stored path for an open database. Returns a
    /// `BackupError::BackupFailed` shaped error if the database is not
    /// currently open — surfaced as a non-blocking warning by the command
    /// layer rather than as an `open` failure.
    fn resolve_open_db_path(&self, db_id: &str) -> Result<String, backups::BackupError> {
        let normalized = Self::normalize_path(db_id);
        let databases = self
            .lock_databases()
            .map_err(|_| backups::BackupError::BackupFailed {
                path: std::path::PathBuf::from(db_id),
                source: std::io::Error::other("databases lock poisoned"),
            })?;
        databases
            .get(&normalized)
            .map(|open_db| open_db.path.clone())
            .ok_or_else(|| backups::BackupError::BackupFailed {
                path: std::path::PathBuf::from(db_id),
                source: std::io::Error::other(format!("database not open: {db_id}")),
            })
    }

    /// Unlocks a locked database by re-opening it from disk with optional password.
    ///
    /// Returns `(info, did_transition)` where `did_transition` is `true`
    /// iff *this* call actually moved the DB from locked to unlocked. The
    /// flag is decided inside the same critical section that mutates the
    /// open-database state, so concurrent unlock calls cannot both
    /// observe themselves as the transitioning one. Callers that need to
    /// gate one-shot side effects on a real transition (audit
    /// `vault.opened`, open-side backup snapshot) must read this flag
    /// rather than pre-check `is_database_locked`.
    pub fn unlock(
        &self,
        db_id: &str,
        password: Option<&str>,
    ) -> Result<(DatabaseInfo, bool), AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        if !open_db.is_locked() {
            return Ok((
                DatabaseInfo {
                    name: open_db.name.clone(),
                    path: open_db.path.clone(),
                    is_modified: open_db.is_modified,
                    is_locked: false,
                    root_group_id: open_db.root_group_id.clone(),
                    version: open_db.version.clone(),
                },
                false,
            ));
        }

        let path = &open_db.path;
        let keyfile_path = open_db.keyfile_path.clone();

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let key = build_database_key(password, keyfile_path.as_deref())?;
        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        open_db.name.clone_from(&db.root().name);
        open_db.root_group_id = db.root().id().uuid().to_string();
        open_db.version = format_database_version(&db.config.version);
        open_db.db = Some(db);
        open_db.password = password.map(SecureString::from);
        open_db.is_modified = false;

        Ok((
            DatabaseInfo {
                name: open_db.name.clone(),
                path: open_db.path.clone(),
                is_modified: false,
                is_locked: false,
                root_group_id: open_db.root_group_id.clone(),
                version: open_db.version.clone(),
            },
            true,
        ))
    }
}

fn map_open_error(err: DatabaseOpenError) -> AppError {
    match err {
        // Authentication errors - incorrect credentials
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey)
        | DatabaseOpenError::Cryptography(CryptographyError::InvalidPadding(_))
        | DatabaseOpenError::Format(
            DatabaseFormatError::Kdbx3(Kdbx3OpenError::BlockHashMismatch(_))
            | DatabaseFormatError::Kdbx4(Kdbx4OpenError::BlockStream(
                BlockStreamError::BlockHashMismatch { .. },
            )),
        ) => AppError::InvalidPassword,

        // Header integrity errors (KDBX4 only - KDBX3 has no header hash)
        DatabaseOpenError::Format(DatabaseFormatError::Kdbx4(
            Kdbx4OpenError::HeaderHashMismatch,
        )) => AppError::HeaderIntegrityError,

        // Invalid KDBX file format
        DatabaseOpenError::VersionParse(DatabaseVersionParseError::InvalidKDBXIdentifier) => {
            AppError::InvalidKdbxFile
        }

        // Unsupported KDBX version
        DatabaseOpenError::VersionParse(DatabaseVersionParseError::InvalidKDBXVersion {
            file_major_version,
            file_minor_version,
            ..
        }) => AppError::UnsupportedKdbxVersion(format!(
            "KDBX {file_major_version}.{file_minor_version}"
        )),

        DatabaseOpenError::UnsupportedVersion => {
            AppError::UnsupportedKdbxVersion("unsupported".to_string())
        }

        // Unsupported outer cipher (KDBX3 or KDBX4)
        DatabaseOpenError::Format(
            DatabaseFormatError::Kdbx3(Kdbx3OpenError::OuterHeader(
                Kdbx3OuterHeaderError::OuterCipher(OuterCipherConfigError::InvalidOuterCipherID {
                    cid,
                }),
            ))
            | DatabaseFormatError::Kdbx4(Kdbx4OpenError::OuterHeader(
                Kdbx4OuterHeaderError::OuterCipherConfig(
                    OuterCipherConfigError::InvalidOuterCipherID { cid },
                ),
            )),
        ) => AppError::UnsupportedCipher(format!("Unknown outer cipher ID: {cid:?}")),

        // Unsupported inner cipher (KDBX3 outer header or KDBX4 inner header)
        DatabaseOpenError::Format(
            DatabaseFormatError::Kdbx3(Kdbx3OpenError::OuterHeader(
                Kdbx3OuterHeaderError::InnerCipher(InnerCipherConfigError::InvalidInnerCipherID {
                    cid,
                }),
            ))
            | DatabaseFormatError::Kdbx4(Kdbx4OpenError::InnerHeader(
                Kdbx4InnerHeaderError::InnerCipherConfig(
                    InnerCipherConfigError::InvalidInnerCipherID { cid },
                ),
            )),
        ) => AppError::UnsupportedCipher(format!("Unknown inner cipher ID: {cid}")),

        // Unsupported compression (KDBX3 or KDBX4)
        DatabaseOpenError::Format(
            DatabaseFormatError::Kdbx3(Kdbx3OpenError::OuterHeader(
                Kdbx3OuterHeaderError::Compression(
                    CompressionConfigError::InvalidCompressionSuite { cid },
                ),
            ))
            | DatabaseFormatError::Kdbx4(Kdbx4OpenError::OuterHeader(
                Kdbx4OuterHeaderError::CompressionConfig(
                    CompressionConfigError::InvalidCompressionSuite { cid },
                ),
            )),
        ) => AppError::HeaderParseError(format!("Unknown compression ID: {cid}")),

        // Unsupported KDF (KDBX4 only)
        DatabaseOpenError::Format(DatabaseFormatError::Kdbx4(Kdbx4OpenError::OuterHeader(
            Kdbx4OuterHeaderError::KdfConfig(KdfConfigError::InvalidKDFUUID { uuid }),
        ))) => AppError::UnsupportedKdf(format!("Unknown KDF UUID: {uuid:?}")),
        DatabaseOpenError::Format(DatabaseFormatError::Kdbx4(Kdbx4OpenError::OuterHeader(
            Kdbx4OuterHeaderError::KdfConfig(KdfConfigError::InvalidKDFVersion { version }),
        ))) => AppError::UnsupportedKdf(format!("Unsupported KDF version: {version}")),

        // All other errors
        other => AppError::Kdbx(other.to_string()),
    }
}
