use keepass::config::DatabaseVersion;
use keepass::Database;

use super::secure::SecureString;
use crate::dto::error::AppError;

pub struct OpenDatabase {
    pub db: Option<Database>,
    pub path: String,
    pub is_modified: bool,
    pub password: Option<SecureString>,
    pub keyfile_path: Option<String>,
    pub version: String,
    pub name: String,
    pub root_group_id: String,
    /// Monotonically increasing per-unlock counter, bumped by
    /// `VaultMut::mark_modified()`. The Password Health service keys
    /// its `(db_id, generation)` cache on this counter so a fresh
    /// analysis runs after every Vault mutation — see ADR 0002. Reset
    /// to zero on `unlock()` so a re-unlocked Vault doesn't carry
    /// stale generation numbers from its previous session.
    pub generation: u64,
}

impl OpenDatabase {
    pub fn db_or_locked(&self) -> Result<&Database, AppError> {
        self.db
            .as_ref()
            .ok_or_else(|| AppError::DatabaseLocked(self.path.clone()))
    }

    pub fn db_mut_or_locked(&mut self) -> Result<&mut Database, AppError> {
        let path = self.path.clone();
        self.db.as_mut().ok_or(AppError::DatabaseLocked(path))
    }

    pub fn is_locked(&self) -> bool {
        self.db.is_none()
    }
}

/// Upgrades a database's in-memory format version to one the writer accepts.
///
/// keepass 0.13 serializes only KDBX 4.1 and rejects KDBX 4.0 output with
/// `Unsupported database version`. Vaults opened from legacy 4.0 files keep
/// `KDB4(0)` in their config, so we bump them to the current writable minor
/// version before every save. New vaults are already created as `KDB4(1)`.
pub fn ensure_writable_version(db: &mut Database) {
    if matches!(db.config.version, DatabaseVersion::KDB4(0)) {
        db.config.version = DatabaseVersion::KDB4(1);
    }
}

/// Formats a database version for display.
pub fn format_database_version(version: &DatabaseVersion) -> String {
    match version {
        DatabaseVersion::KDB(minor) => format!("KDB 1.{minor}"),
        DatabaseVersion::KDB2(minor) => format!("KDB 2.{minor}"),
        DatabaseVersion::KDB3(minor) => format!("KDBX 3.{minor}"),
        DatabaseVersion::KDB4(minor) => format!("KDBX 4.{minor}"),
    }
}
